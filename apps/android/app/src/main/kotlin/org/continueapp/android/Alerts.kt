package org.continueapp.android

import android.Manifest
import android.annotation.SuppressLint
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import kotlinx.coroutines.launch
import org.continueapp.android.ui.screens.describe
import org.continueapp.bridge.PermissionAnswer
import org.continueapp.bridge.PermissionQuestion

private const val CHANNEL_BACKGROUND = "background"
private const val CHANNEL_RECEIVED = "received"
private const val CHANNEL_QUESTIONS = "questions"
const val BACKGROUND_NOTIFICATION_ID = 1
private const val QUESTION_NOTIFICATION_ID = 2
private const val FIRST_ARRIVAL_ID = 100
private const val EXTRA_QUESTION = "question"
private const val EXTRA_ANSWER = "answer"
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
            NotificationChannel(CHANNEL_QUESTIONS, "Requests to send", NotificationManager.IMPORTANCE_HIGH),
        )
    context.getSystemService(NotificationManager::class.java)?.createNotificationChannels(channels)
}

/**
 * Determines whether notification permission has not been granted on Android 13 or newer.
 *
 * @return `true` if the device runs Android 13 or newer and notification permission is not granted, `false` otherwise.
 */
fun needsNotificationPermission(context: Context): Boolean =
    Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
        ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) !=
        PackageManager.PERMISSION_GRANTED

/**
 * Builds an ongoing, low-priority notification showing the connection status and a clipboard-send action.
 * Tapping the notification opens the app.
 *
 * @return The background notification.
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

/** Shows the latest connection state in the background notification. */
fun updateBackgroundNotification(context: Context) {
    post(context, BACKGROUND_NOTIFICATION_ID, backgroundNotification(context))
}

/**
 * Shows an auto-canceling notification for an arrival.
 *
 * @param open The intent to open when the notification is tapped, or `null` to open the app.
 */
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

/**
 * Presents a permission question as a notification with Decline, Always allow, and Allow actions.
 *
 * @param question The permission question to display; its expiry time determines how long the notification remains visible.
 */
fun notifyQuestion(
    context: Context,
    question: PermissionQuestion,
) {
    val (title, body) = describe(question)
    val notification =
        NotificationCompat
            .Builder(context, CHANNEL_QUESTIONS)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(title)
            .setContentText(body)
            .setContentIntent(openApp(context))
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setTimeoutAfter(question.expiresAt - System.currentTimeMillis())
            .addAction(0, "Decline", answerIntent(context, question.id, PermissionAnswer.DECLINE))
            .addAction(0, "Always allow", answerIntent(context, question.id, PermissionAnswer.ALWAYS_ALLOW))
            .addAction(0, "Allow", answerIntent(context, question.id, PermissionAnswer.ALLOW))
            .build()
    post(context, QUESTION_NOTIFICATION_ID, notification)
}

/**
 * Cancels the permission-question notification.
 */
fun cancelQuestion(context: Context) = NotificationManagerCompat.from(context).cancel(QUESTION_NOTIFICATION_ID)

/** Answers a question from its notification's buttons. */
class QuestionAnswerReceiver : BroadcastReceiver() {
    /**
     * Submits a valid permission-question answer from a broadcast.
     *
     * Broadcasts with an answer that does not match a [PermissionAnswer] are ignored.
     *
     * @param intent The broadcast containing the permission-question ID and answer.
     */
    override fun onReceive(
        context: Context,
        intent: Intent,
    ) {
        val answer = PermissionAnswer.entries.firstOrNull { it.name == intent.getStringExtra(EXTRA_ANSWER) } ?: return
        val app = context.applicationContext as ContinueApplication
        val done = goAsync()
        app.scope.launch {
            try {
                app.state.questions.answer(intent.getLongExtra(EXTRA_QUESTION, -1), answer)
            } finally {
                done.finish()
            }
        }
    }
}

/**
 * Creates a broadcast pending intent for a permission question answer.
 *
 * @return A pending intent carrying the question ID and selected answer.
 */
private fun answerIntent(
    context: Context,
    question: Long,
    answer: PermissionAnswer,
): PendingIntent {
    val intent =
        Intent(context, QuestionAnswerReceiver::class.java)
            .putExtra(EXTRA_QUESTION, question)
            .putExtra(EXTRA_ANSWER, answer.name)
    // Each question gets its own buttons, so an old notification can never answer a newer one.
    val code = (question * PermissionAnswer.entries.size + answer.ordinal).toInt()
    return PendingIntent.getBroadcast(context, code, intent, IMMUTABLE)
}

/**
 * Creates a pending intent that opens the app's main activity.
 *
 * @return A pending intent for opening `MainActivity`.
 */
private fun openApp(context: Context): PendingIntent =
    PendingIntent.getActivity(context, 0, Intent(context, MainActivity::class.java), IMMUTABLE)

/**
 * Posts a notification when notification permission is not required or has been granted.
 */
@SuppressLint("MissingPermission") // Checked first.
private fun post(
    context: Context,
    id: Int,
    notification: Notification,
) {
    if (!needsNotificationPermission(context)) NotificationManagerCompat.from(context).notify(id, notification)
}
