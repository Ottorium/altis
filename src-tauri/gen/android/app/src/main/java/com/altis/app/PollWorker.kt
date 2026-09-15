package com.altis.app

import android.content.Context
import androidx.work.Worker
import androidx.work.WorkerParameters

/**
 * One poll, run by WorkManager on its own schedule.
 *
 * This is the no-notification option: WorkManager starts the process when it is time, so nothing
 * has to be kept alive in between, and Android asks for nothing in return. The trade is that the
 * schedule is a request rather than a promise - see [BackgroundPoller.PERIODIC_INTERVAL_MINUTES].
 */
class PollWorker(context: Context, params: WorkerParameters) : Worker(context, params) {
    override fun doWork(): Result = try {
        // the process may have been started for this job alone, with no activity to set anything up
        BackgroundPoller.prepare(applicationContext)
        BackgroundPoller.nativeRunOnce()
        Result.success()
    } catch (_: Throwable) {
        Result.retry()
    }
}
