package org.continueapp.android

import android.Manifest
import android.annotation.SuppressLint
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat

private const val CHANNEL_BACKGROUND = "background"
private const val CHANNEL_RECEIVED = "received"
const val BACKGROUND_NOTIFICATION_ID = 1
const val SCREEN_NOTIFICATION_ID = 3
const val CAMERA_NOTIFICATION_ID = 4
private const val RINGING_NOTIFICATION_ID = 5
private const val FIRST_ARRIVAL_ID = 100
private const val IMMUTABLE = PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT

private var nextArrivalId = FIRST_ARRIVAL_ID

/** Sets up the kinds of notification Continue posts, so each can be turned off on its own. */
fun createNotificationChannels(context: Context) {
    val channels =
        listOf(
            // Low rather than min: Android adds its own "running in the background" notice for min.
            NotificationChannel(CHANNEL_BACKGROUND, "Receiving in the background", NotificationManager.IMPORTANCE_LOW)
                .apply { description = "Shown while Continue keeps receiving with the app closed." },
            NotificationChannel(CHANNEL_RECEIVED, "Files and text you receive", NotificationManager.IMPORTANCE_DEFAULT),
        )
    val manager = context.getSystemService(NotificationManager::class.java) ?: return
    manager.createNotificationChannels(channels)
    // Leftover from older versions.
    manager.deleteNotificationChannel("questions")
}

/** Whether Android 13 or newer still needs to be asked before notifications can show. */
fun needsNotificationPermission(context: Context): Boolean =
    Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
        ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) !=
        PackageManager.PERMISSION_GRANTED

/**
 * The quiet notification that keeps receiving going. It says who's connected and has a button
 * to send the clipboard.
 */
fun backgroundNotification(context: Context): Notification {
    val app = context.applicationContext as ContinueApplication
    val sendClipboard =
        PendingIntent.getActivity(context, 0, Intent(context, SendClipboardActivity::class.java), IMMUTABLE)
    return NotificationCompat
        .Builder(context, CHANNEL_BACKGROUND)
        .setSmallIcon(R.drawable.ic_notification)
        .setContentTitle(app.connectionStatus)
        .setContentText("Files and text from your computer arrive even with Continue closed.")
        .setContentIntent(openApp(context))
        .addAction(0, "Send clipboard", sendClipboard)
        .setOngoing(true)
        .setPriority(NotificationCompat.PRIORITY_LOW)
        .build()
}

/** Ongoing while the screen or camera is shared, with a Stop action. */
fun sharingNotification(
    context: Context,
    title: String,
    stop: Intent,
): Notification =
    NotificationCompat
        .Builder(context, CHANNEL_BACKGROUND)
        .setSmallIcon(R.drawable.ic_notification)
        .setContentTitle(title)
        .setContentIntent(openApp(context))
        .addAction(0, "Stop", PendingIntent.getService(context, 0, stop, IMMUTABLE))
        .setOngoing(true)
        .build()

/** Stopping or swiping it away stops the ringing. */
fun notifyRinging(
    context: Context,
    stop: Intent,
) {
    val stopping = PendingIntent.getBroadcast(context, 0, stop, IMMUTABLE)
    val notification =
        NotificationCompat
            .Builder(context, CHANNEL_RECEIVED)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle("Your computer is ringing this phone")
            .setContentIntent(stopping)
            .setDeleteIntent(stopping)
            .addAction(0, "Stop", stopping)
            .build()
    post(context, RINGING_NOTIFICATION_ID, notification)
}

fun cancelRinging(context: Context) = NotificationManagerCompat.from(context).cancel(RINGING_NOTIFICATION_ID)

/** Shows the latest connection state in the background notification. */
fun updateBackgroundNotification(context: Context) {
    post(context, BACKGROUND_NOTIFICATION_ID, backgroundNotification(context))
}

/** Says something arrived. Tapping it opens [open], or the app when there's nothing to open. */
fun notifyArrival(
    context: Context,
    title: String,
    detail: String,
    open: Intent?,
) {
    val id = nextArrivalId++
    val tap = open?.let { PendingIntent.getActivity(context, id, it, IMMUTABLE) } ?: openApp(context)
    val notification =
        NotificationCompat
            .Builder(context, CHANNEL_RECEIVED)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(title)
            .setContentText(detail)
            .setContentIntent(tap)
            .setAutoCancel(true)
            .build()
    post(context, id, notification)
}

private fun openApp(context: Context): PendingIntent =
    PendingIntent.getActivity(context, 0, Intent(context, MainActivity::class.java), IMMUTABLE)

@SuppressLint("MissingPermission") // Checked first.
private fun post(
    context: Context,
    id: Int,
    notification: Notification,
) {
    if (!needsNotificationPermission(context)) NotificationManagerCompat.from(context).notify(id, notification)
}
