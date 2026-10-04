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

/**
 * Extracts shared files, nonblank text, and an optional peer fingerprint from a send intent.
 *
 * @return The extracted share, or `null` if the intent has an unsupported action or contains no files or nonblank text.
 */
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
 * Publishes trusted computers as dynamic share-sheet targets, replacing the existing shortcuts.
 *
 * @param peers Trusted computers to publish; entries beyond the system's per-activity limit are omitted.
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

/**
 * Sends shared files or text to a trusted peer, reconnecting first if needed.
 *
 * Reports the peer's share-sheet shortcut as used. If reconnection fails, returns that error without sending the content.
 *
 * @return The result of sending the shared content, or the reconnection error.
 */
suspend fun AppState.sendShared(
    context: Context,
    peer: TrustedPeer,
    shared: Shared,
): String? {
    // Puts the computers shared to most often first in the share sheet.
    ShortcutManagerCompat.reportShortcutUsed(context, peer.fingerprint)
    if (peer.fingerprint !in connected) reconnect(peer.fingerprint)?.let { return it }
    return if (shared.uris.isNotEmpty()) {
        sendFiles(context, peer, shared.uris)
    } else {
        sendText(peer, shared.text.orEmpty())
    }
}
