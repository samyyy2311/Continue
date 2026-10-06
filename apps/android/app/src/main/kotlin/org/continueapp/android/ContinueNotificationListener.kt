package org.continueapp.android

import android.app.Notification
import android.app.PendingIntent
import android.app.RemoteInput
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import org.continueapp.bridge.NotificationButton
import org.continueapp.bridge.NotificationEvent
import org.continueapp.bridge.PhoneNotification
import java.util.concurrent.ConcurrentHashMap

/**
 * Shows the phone's notifications on connected computers, and carries out what the computer
 * does with them: a button pressed, a reply typed, or the notification dismissed.
 */
class ContinueNotificationListener : NotificationListenerService() {
    /** The buttons of each forwarded notification, so a computer can press them later. */
    private val buttons = ConcurrentHashMap<String, List<Notification.Action>>()

    private val core get() = (application as ContinueApplication).coreBridge

    override fun onListenerConnected() {
        active = this
    }

    override fun onListenerDisconnected() {
        active = null
        buttons.clear()
    }

    override fun onNotificationPosted(sbn: StatusBarNotification) {
        val notification = sbn.notification
        // Ongoing ones are music, calls and the like; group summaries repeat their children.
        val skipped =
            sbn.packageName == packageName ||
                sbn.isOngoing ||
                notification.flags and Notification.FLAG_GROUP_SUMMARY != 0
        if (skipped) return
        val extras = notification.extras
        val title = extras.getCharSequence(Notification.EXTRA_TITLE)?.toString().orEmpty()
        val text = extras.getCharSequence(Notification.EXTRA_TEXT)?.toString().orEmpty()
        if (title.isBlank() && text.isBlank()) return

        val actions = notification.actions.orEmpty().toList()
        buttons[sbn.key] = actions
        core.forwardNotification(
            PhoneNotification(
                id = sbn.key,
                packageName = sbn.packageName,
                appName = appName(sbn.packageName),
                title = title,
                text = text,
                postedAt = sbn.postTime,
                buttons =
                    actions.mapIndexed { index, action ->
                        val isReply = !action.remoteInputs.isNullOrEmpty()
                        NotificationButton(index.toString(), action.title.toString(), isReply)
                    },
            ),
        )
    }

    override fun onNotificationRemoved(sbn: StatusBarNotification) {
        if (buttons.remove(sbn.key) != null) core.forwardNotificationRemoved(sbn.key)
    }

    private fun carryOut(event: NotificationEvent) {
        when (event) {
            is NotificationEvent.Dismissed -> cancelNotification(event.notificationId)
            is NotificationEvent.Pressed -> {
                val action = buttons[event.notificationId]?.getOrNull(event.buttonId.toIntOrNull() ?: -1) ?: return
                val inputs = action.remoteInputs
                val fillIn = Intent()
                if (!inputs.isNullOrEmpty() && event.replyText.isNotBlank()) {
                    val results = Bundle().apply { inputs.forEach { putCharSequence(it.resultKey, event.replyText) } }
                    RemoteInput.addResultsToIntent(inputs, fillIn, results)
                }
                try {
                    action.actionIntent.send(this, 0, fillIn)
                } catch (_: PendingIntent.CanceledException) {
                    // The app withdrew the button since; there's nothing left to press.
                }
            }
        }
    }

    private fun appName(packageName: String): String =
        try {
            packageManager.getApplicationLabel(packageManager.getApplicationInfo(packageName, 0)).toString()
        } catch (_: PackageManager.NameNotFoundException) {
            packageName
        }

    companion object {
        @Volatile
        private var active: ContinueNotificationListener? = null

        /** Carries out what a computer did, if Continue can still see notifications. */
        fun handle(event: NotificationEvent) {
            active?.carryOut(event)
        }
    }
}
