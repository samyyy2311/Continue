package org.continueapp.android

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/** Starts receiving in the background again after the phone restarts or the app updates. */
class StartReceiver : BroadcastReceiver() {
    override fun onReceive(
        context: Context,
        intent: Intent,
    ) {
        val restarted =
            intent.action == Intent.ACTION_BOOT_COMPLETED || intent.action == Intent.ACTION_MY_PACKAGE_REPLACED
        val app = context.applicationContext as ContinueApplication
        if (!restarted || !app.receiveInBackground) return
        try {
            app.applyBackground()
        } catch (_: IllegalStateException) {
            // Android refused to start it from here; it starts the next time the app opens.
        }
    }
}
