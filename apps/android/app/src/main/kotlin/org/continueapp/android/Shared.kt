package org.continueapp.android

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.core.content.IntentCompat
import org.continueapp.bridge.TrustedPeer

/** Files or text another app shared to Continue. When both come, the files are sent. */
data class Shared(
    val uris: List<Uri>,
    val text: String?,
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
    return Shared(uris, text).takeIf { uris.isNotEmpty() || text != null }
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
