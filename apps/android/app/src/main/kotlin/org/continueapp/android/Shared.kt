package org.continueapp.android

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.core.content.IntentCompat
import androidx.core.content.pm.ShortcutInfoCompat
import androidx.core.content.pm.ShortcutManagerCompat
import androidx.core.graphics.drawable.IconCompat
import org.continueapp.bridge.TrustedPeer

private const val SHARE_CATEGORY = "org.continueapp.android.SEND_TO_COMPUTER"

/** Files or text another app shared to Continue. When both come, the files are sent. */
data class Shared(
    val uris: List<Uri>,
    val text: String?,
    /** The computer picked straight from the share sheet, if one was. */
    val peerFingerprint: String? = null,
) {
    /** What is being sent, for "Send … to". */
    val summary: String
        get() =
            when (uris.size) {
                0 -> "text"
                1 -> "1 file"
                else -> "${uris.size} files"
            }
}

/** Reads a share from another app, or null if [intent] isn't one. */
fun sharedFrom(intent: Intent?): Shared? {
    val uris =
        when (intent?.action) {
            Intent.ACTION_SEND ->
                listOfNotNull(IntentCompat.getParcelableExtra(intent, Intent.EXTRA_STREAM, Uri::class.java))
            Intent.ACTION_SEND_MULTIPLE ->
                IntentCompat.getParcelableArrayListExtra(intent, Intent.EXTRA_STREAM, Uri::class.java).orEmpty()
            else -> return null
        }
    val text = intent.getStringExtra(Intent.EXTRA_TEXT)?.takeIf { it.isNotBlank() }
    val peer = intent.getStringExtra(ShortcutManagerCompat.EXTRA_SHORTCUT_ID)
    return Shared(uris, text, peer).takeIf { uris.isNotEmpty() || text != null }
}

/**
 * Lists each paired computer in the system share sheet, so sharing to it takes one tap.
 * Replaces the whole set, which also drops computers that were forgotten.
 */
fun publishShareTargets(
    context: Context,
    peers: List<TrustedPeer>,
) {
    val icon = IconCompat.createWithResource(context, R.drawable.ic_shortcut_computer)
    val open = Intent(context, MainActivity::class.java).setAction(Intent.ACTION_VIEW)
    val shortcuts =
        peers.take(ShortcutManagerCompat.getMaxShortcutCountPerActivity(context)).map { peer ->
            ShortcutInfoCompat
                .Builder(context, peer.fingerprint)
                .setShortLabel(peer.displayName)
                .setIcon(icon)
                .setIntent(open)
                .setLongLived(true)
                .setCategories(setOf(SHARE_CATEGORY))
                .build()
        }
    ShortcutManagerCompat.setDynamicShortcuts(context, shortcuts)
}

/** Sends what another app shared, connecting to [peer] first if it isn't already. */
suspend fun AppState.sendShared(
    context: Context,
    peer: TrustedPeer,
    shared: Shared,
): String? {
    if (peer.fingerprint !in connected) reconnect(peer.fingerprint)?.let { return it }
    return if (shared.uris.isNotEmpty()) {
        sendFiles(context, peer, shared.uris)
    } else {
        sendText(peer, shared.text.orEmpty())
    }
}
