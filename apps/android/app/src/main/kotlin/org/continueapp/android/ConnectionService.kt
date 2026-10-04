package org.continueapp.android

import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.ServiceCompat

/**
 * Keeps Continue running while the app is closed, so files and text keep arriving. The
 * receiving itself happens in [ContinueApplication]; this only holds the process open.
 */
class ConnectionService : Service() {
    /**
     * Indicates that this service does not support binding.
     *
     * @return `null` because the service is unbound.
     */
    override fun onBind(intent: Intent?): IBinder? = null

    /**
     * Starts the service in the foreground with the background notification.
     *
     * @return The start mode requesting that the system recreate the service after its process is killed.
     */
    override fun onStartCommand(
        intent: Intent?,
        flags: Int,
        startId: Int,
    ): Int {
        val type =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE
            } else {
                0
            }
        ServiceCompat.startForeground(this, BACKGROUND_NOTIFICATION_ID, backgroundNotification(this), type)
        return START_STICKY
    }
}
