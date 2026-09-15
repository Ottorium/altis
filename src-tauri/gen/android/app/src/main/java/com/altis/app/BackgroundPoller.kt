package com.altis.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequest
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

/**
 * The Kotlin half of the background poller.
 *
 * The poll itself is Rust (`background` in the Tauri backend, sharing its code with the frontend),
 * running on a thread this object starts. It lives out here rather than in the webview because the
 * webview is gone the moment the app is closed, which is exactly when the polling still has to
 * happen. Everything Rust needs from Android — the app context, the notification channels, the
 * service that keeps the process alive — it gets through here.
 *
 * There are two ways to keep checking once the app is closed, picked in Settings:
 * [MODE_PERIODIC] hands the schedule to WorkManager, which needs no permanent notification but
 * only runs when Android feels like it; [MODE_CONTINUOUS] runs [PollService], which keeps to the
 * interval the user chose and pays for it with an ongoing notification.
 *
 * Rust calls [postNotification] and [setBackgroundMode] as instance methods on this singleton.
 */
object BackgroundPoller {
    /** The channel the actual "your lesson was cancelled" notifications go to */
    const val CHANGES_CHANNEL = "altis_timetable_changes"

    /** The channel for the ongoing notification Android demands in return for a live process */
    const val SERVICE_CHANNEL = "altis_background"

    const val SERVICE_NOTIFICATION_ID = 1

    const val MODE_OFF = "off"
    const val MODE_PERIODIC = "periodic"
    const val MODE_CONTINUOUS = "continuous"

    /** WorkManager's hard floor. Kept in step with `PERIODIC_INTERVAL_MINUTES` in the Rust side. */
    const val PERIODIC_INTERVAL_MINUTES = 15L

    private const val PREFS = "altis_background"
    private const val KEY_MODE = "background_mode"
    private const val WORK_NAME = "altis_periodic_poll"

    @Volatile
    private var appContext: Context? = null

    /** Ids for the change notifications, so a new one never replaces an unread one */
    private val nextNotificationId = AtomicInteger(SERVICE_NOTIFICATION_ID + 1)

    /**
     * Loads the native library, creates the notification channels and tells Rust where to keep its
     * state. Both the activity and the service call this, because either can be what starts the
     * process: after a reboot, or after Android has killed and restarted the service, there is no
     * activity to do it.
     */
    @Synchronized
    fun prepare(context: Context) {
        val app = context.applicationContext
        createChannels(app)
        System.loadLibrary("altis_lib")
        appContext = app
        nativeInit(app.filesDir.absolutePath)
    }

    fun mode(context: Context): String =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getString(KEY_MODE, MODE_OFF) ?: MODE_OFF

    /**
     * Switches between the two background modes, or neither. Called from Rust when the
     * notification settings change, and the choice is remembered so [BootReceiver] can pick it up
     * again after a reboot.
     */
    fun setBackgroundMode(mode: String) {
        val context = appContext ?: return
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit()
            .putString(KEY_MODE, mode)
            .apply()

        val service = Intent(context, PollService::class.java)
        val work = WorkManager.getInstance(context)

        when (mode) {
            MODE_CONTINUOUS -> {
                work.cancelUniqueWork(WORK_NAME)
                ContextCompat.startForegroundService(context, service)
            }
            MODE_PERIODIC -> {
                context.stopService(service)
                // KEEP, so that the settings screen saving on every keystroke doesn't push the
                // next run further away each time
                work.enqueueUniquePeriodicWork(WORK_NAME, ExistingPeriodicWorkPolicy.KEEP, periodicRequest())
            }
            else -> {
                context.stopService(service)
                work.cancelUniqueWork(WORK_NAME)
            }
        }
    }

    private fun periodicRequest(): PeriodicWorkRequest =
        PeriodicWorkRequestBuilder<PollWorker>(PERIODIC_INTERVAL_MINUTES, TimeUnit.MINUTES)
            .setConstraints(
                Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()
            )
            .build()

    /** Called from the Rust poller thread for every change worth telling the user about */
    fun postNotification(title: String, body: String) {
        val context = appContext ?: return
        val manager = NotificationManagerCompat.from(context)
        if (!manager.areNotificationsEnabled()) return

        val notification = NotificationCompat.Builder(context, CHANGES_CHANNEL)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(title)
            .setContentText(body)
            .setStyle(NotificationCompat.BigTextStyle().bigText(body))
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .setAutoCancel(true)
            .setContentIntent(openAppIntent(context))
            .build()

        try {
            manager.notify(nextNotificationId.getAndIncrement(), notification)
        } catch (_: SecurityException) {
            // the notification permission was revoked between the check above and here
        }
    }

    /** The ongoing notification that buys the process the right to keep running */
    fun serviceNotification(context: Context): Notification =
        NotificationCompat.Builder(context, SERVICE_CHANNEL)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle("Altis")
            .setContentText("Watching your timetable for changes")
            .setPriority(NotificationCompat.PRIORITY_MIN)
            .setOngoing(true)
            .setShowWhen(false)
            .setContentIntent(openAppIntent(context))
            .build()

    private fun openAppIntent(context: Context): PendingIntent? {
        val intent = context.packageManager.getLaunchIntentForPackage(context.packageName) ?: return null
        return PendingIntent.getActivity(
            context,
            0,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
    }

    private fun createChannels(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = context.getSystemService(NotificationManager::class.java) ?: return

        manager.createNotificationChannel(
            NotificationChannel(CHANGES_CHANNEL, "Timetable changes", NotificationManager.IMPORTANCE_DEFAULT).apply {
                description = "Cancelled lessons, substitutions, exams and new messages"
            }
        )
        // IMPORTANCE_MIN keeps the permanent notification collapsed and silent, which is as close
        // to invisible as Android lets a foreground service get
        manager.createNotificationChannel(
            NotificationChannel(SERVICE_CHANNEL, "Background checking", NotificationManager.IMPORTANCE_MIN).apply {
                description = "Shown while Altis is checking your timetable in the background"
            }
        )
    }

    external fun nativeInit(filesDir: String)

    /** One poll on the calling thread, for [PollWorker] */
    external fun nativeRunOnce()

    external fun nativeStartPolling()

    external fun nativeStopPolling()
}
