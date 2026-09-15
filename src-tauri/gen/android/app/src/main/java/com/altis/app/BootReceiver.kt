package com.altis.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.core.content.ContextCompat

/**
 * Starts the foreground service again after a reboot, if that was the mode when the device went
 * down. Without this, "check on the interval above" would quietly stop meaning anything until the
 * user next opened the app.
 *
 * Only that mode needs handling here: WorkManager persists its own schedule and restores it after
 * boot by itself.
 */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        if (BackgroundPoller.mode(context) != BackgroundPoller.MODE_CONTINUOUS) return

        ContextCompat.startForegroundService(context, Intent(context, PollService::class.java))
    }
}
