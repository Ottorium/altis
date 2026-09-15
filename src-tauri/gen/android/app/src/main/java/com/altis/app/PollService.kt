package com.altis.app

import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder

/**
 * Keeps the app's process alive so the Rust poller can keep checking after the app is closed.
 *
 * The service does no work of its own: Android simply won't leave a process running without one,
 * and swiping the app away takes the activity (and the webview the poller used to live in) with
 * it. START_STICKY asks for the service back if the system kills it anyway.
 */
class PollService : Service() {
    override fun onCreate() {
        super.onCreate()
        // the process may have been started by this service alone, with no activity to set up
        BackgroundPoller.prepare(this)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val notification = BackgroundPoller.serviceNotification(this)

        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                startForeground(
                    BackgroundPoller.SERVICE_NOTIFICATION_ID,
                    notification,
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE,
                )
            } else {
                startForeground(BackgroundPoller.SERVICE_NOTIFICATION_ID, notification)
            }
        } catch (_: Exception) {
            // Android 12+ refuses foreground services started from the background. Nothing to be
            // done about it here; the app will start it again the next time it is opened.
            stopSelf()
            return START_NOT_STICKY
        }

        BackgroundPoller.nativeStartPolling()
        return START_STICKY
    }

    override fun onDestroy() {
        BackgroundPoller.nativeStopPolling()
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
