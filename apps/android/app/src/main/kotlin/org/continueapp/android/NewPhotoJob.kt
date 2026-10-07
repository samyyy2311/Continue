package org.continueapp.android

import android.app.job.JobInfo
import android.app.job.JobParameters
import android.app.job.JobScheduler
import android.app.job.JobService
import android.content.ComponentName
import android.content.Context
import android.provider.MediaStore
import kotlin.concurrent.thread

private const val JOB_ID = 1
private const val SETTLE_MS = 1000L

class NewPhotoJob : JobService() {
    override fun onStartJob(params: JobParameters): Boolean {
        thread {
            (application as ContinueApplication).coreBridge.announceNewPhoto()
            jobFinished(params, false)
            // A content trigger fires once, so it's set again for the next photo.
            schedule(this)
        }
        return true
    }

    override fun onStopJob(params: JobParameters) = true

    companion object {
        fun schedule(context: Context) {
            val photos =
                JobInfo.TriggerContentUri(
                    MediaStore.Images.Media.EXTERNAL_CONTENT_URI,
                    JobInfo.TriggerContentUri.FLAG_NOTIFY_FOR_DESCENDANTS,
                )
            val job =
                JobInfo
                    .Builder(JOB_ID, ComponentName(context, NewPhotoJob::class.java))
                    .addTriggerContentUri(photos)
                    .setTriggerContentUpdateDelay(SETTLE_MS)
                    .build()
            context.getSystemService(JobScheduler::class.java).schedule(job)
        }
    }
}
