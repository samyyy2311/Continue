package org.continueapp.android

import android.app.Notification
import android.app.RemoteInput
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import android.util.Log
import org.continueapp.bridge.NotificationActionInvokeModel
import org.continueapp.bridge.NotificationActionModel
import org.continueapp.bridge.NotificationPostModel
import java.util.concurrent.ConcurrentHashMap

class ContinueNotificationListener : NotificationListenerService() {
    companion object {
        private const val TAG = "ContinueNotifListener"

        private val cachedActions = ConcurrentHashMap<String, MutableMap<String, Notification.Action>>()

        fun handleRemoteAction(
            context: Context,
            action: NotificationActionInvokeModel,
        ) {
            val cachedAction = cachedActions[action.notificationId]?.get(action.actionId) ?: return

            if (action.replyText.isNotEmpty() && !cachedAction.remoteInputs.isNullOrEmpty()) {
                val intent = Intent()
                val bundle = Bundle()
                for (remoteInput in cachedAction.remoteInputs) {
                    bundle.putCharSequence(remoteInput.resultKey, action.replyText)
                }
                RemoteInput.addResultsToIntent(cachedAction.remoteInputs, intent, bundle)
                runCatching {
                    cachedAction.actionIntent.send(context, 0, intent)
                }.onFailure { e ->
                    Log.e(TAG, "Failed to send RemoteInput reply", e)
                }
            } else {
                runCatching {
                    cachedAction.actionIntent.send()
                }.onFailure { e ->
                    Log.e(TAG, "Failed to send action intent", e)
                }
            }
        }
    }

    private fun isIgnored(sbn: StatusBarNotification?): Boolean {
        return sbn == null || sbn.packageName == packageName || sbn.isOngoing
    }

    private fun extractText(extras: Bundle): Pair<String, String> {
        val title =
            extras.getCharSequence(Notification.EXTRA_TITLE)
                ?: extras.getCharSequence(Notification.EXTRA_TITLE_BIG)
        val body =
            extras.getCharSequence(Notification.EXTRA_TEXT)
                ?: extras.getCharSequence(Notification.EXTRA_BIG_TEXT)
        return Pair(title?.toString().orEmpty(), body?.toString().orEmpty())
    }

    private data class ExtractedActions(
        val models: List<NotificationActionModel>,
        val map: Map<String, Notification.Action>,
    )

    private fun extractActions(notification: Notification): ExtractedActions {
        val actions = notification.actions ?: return ExtractedActions(emptyList(), emptyMap())
        val models = mutableListOf<NotificationActionModel>()
        val map = mutableMapOf<String, Notification.Action>()
        actions.forEachIndexed { index, action ->
            if (action != null) {
                val actionId = "act_${index}_${action.title?.hashCode() ?: index}"
                val isReply = !action.remoteInputs.isNullOrEmpty()
                models.add(
                    NotificationActionModel(
                        actionId = actionId,
                        label = action.title?.toString() ?: if (isReply) "Reply" else "Open",
                        isReply = isReply,
                    ),
                )
                map[actionId] = action
            }
        }
        return ExtractedActions(models, map)
    }

    private fun resolveAppName(packageName: String): String =
        runCatching {
            val pm = packageManager
            pm.getApplicationLabel(pm.getApplicationInfo(packageName, 0)).toString()
        }.getOrDefault(packageName)

    private fun dispatchNotification(
        sbn: StatusBarNotification,
        title: String,
        body: String,
    ) {
        val (actionModels, actionMap) = extractActions(sbn.notification)
        cachedActions[sbn.key] = actionMap.toMutableMap()

        val post =
            NotificationPostModel(
                notificationId = sbn.key,
                packageName = sbn.packageName,
                appName = resolveAppName(sbn.packageName),
                title = title,
                body = body,
                timestamp = sbn.postTime,
                actions = actionModels,
            )

        val app = applicationContext as? ContinueApplication
        app?.let { application ->
            for (peer in application.state.peers) {
                if (peer.fingerprint in application.state.connected) {
                    runCatching {
                        application.coreBridge.sendNotification(peer.fingerprint, post)
                    }.onFailure { e ->
                        Log.e(TAG, "Failed to send notification to ${peer.fingerprint}", e)
                    }
                }
            }
        }
    }

    override fun onNotificationPosted(sbn: StatusBarNotification?) {
        if (isIgnored(sbn)) return
        val currentSbn = sbn ?: return

        val extras = currentSbn.notification?.extras
        if (extras != null) {
            val (title, body) = extractText(extras)
            if (title.isNotBlank() || body.isNotBlank()) {
                dispatchNotification(currentSbn, title, body)
            }
        }
    }

    override fun onNotificationRemoved(sbn: StatusBarNotification?) {
        if (sbn == null || sbn.packageName == packageName) return

        cachedActions.remove(sbn.key)

        val app = applicationContext as? ContinueApplication
        app?.let { application ->
            for (peer in application.state.peers) {
                if (peer.fingerprint in application.state.connected) {
                    runCatching {
                        application.coreBridge.sendNotificationDismiss(
                            peer.fingerprint,
                            sbn.key,
                            sbn.packageName,
                        )
                    }.onFailure { e ->
                        Log.e(TAG, "Failed to send notification dismiss to ${peer.fingerprint}", e)
                    }
                }
            }
        }
    }
}
