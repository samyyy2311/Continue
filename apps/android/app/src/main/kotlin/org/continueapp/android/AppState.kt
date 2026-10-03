package org.continueapp.android

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.continueapp.bridge.ContinueCoreBridge
import org.continueapp.bridge.ContinueException
import org.continueapp.bridge.PermissionGrant
import org.continueapp.bridge.TrustedPeer
import java.io.File
import java.io.IOException
import java.util.UUID

/**
 * What the screens show, read from the core. Every call into the core runs off the main
 * thread, and failures come back as a message to show rather than an exception.
 */
class AppState(private val bridge: ContinueCoreBridge) {
    var peers by mutableStateOf<List<TrustedPeer>>(emptyList())
        private set
    var connected by mutableStateOf<Set<String>>(emptySet())
        private set

    val deviceKey: String = bridge.getDeviceFingerprint()

    suspend fun refresh() {
        val (latestPeers, latestConnected) =
            withContext(Dispatchers.IO) {
                val list = bridge.listTrustedPeers()
                list to list.filter { bridge.isPeerConnected(it.fingerprint) }.map { it.fingerprint }.toSet()
            }
        peers = latestPeers
        connected = latestConnected
    }

    /** Returns what to tell the user if pairing failed, or null once paired. */
    suspend fun pair(code: String): String? =
        run("Something went wrong while pairing. Try again.") {
            bridge.pairFromQr(code)
        }

    suspend fun disconnect(peer: String): String? = run("Couldn't disconnect.") { bridge.disconnect(peer) }

    suspend fun reconnect(peer: String): String? = run("Couldn't reconnect.") { bridge.reconnect(peer) }

    suspend fun forget(peer: String): String? = run("Couldn't forget this device.") { bridge.removeTrustedPeer(peer) }

    suspend fun sendText(
        peer: String,
        text: String,
    ): String? = run("Couldn't send the text.") { bridge.sendClipboardText(peer, text) }

    /**
     * Sends picked files one by one. The core reads from a path, so each file is copied into
     * the app's cache first and removed once sent.
     */
    suspend fun sendFiles(
        context: Context,
        peer: String,
        uris: List<Uri>,
    ): String? =
        run("Couldn't send the file.") {
            val outbox = File(context.cacheDir, "outgoing/${UUID.randomUUID()}").apply { mkdirs() }
            try {
                for (uri in uris) {
                    val file = copyInto(outbox, context, uri)
                    bridge.sendFile(peer, file.absolutePath)
                    file.delete()
                }
            } finally {
                outbox.deleteRecursively()
            }
        }

    suspend fun permission(
        peer: String,
        capability: Int,
    ): PermissionGrant =
        withContext(Dispatchers.IO) {
            PermissionGrant.fromRaw(bridge.queryPermission(peer, capability))
        }

    suspend fun setPermission(
        peer: String,
        capability: Int,
        grant: PermissionGrant,
    ): String? = run("Couldn't change the permission.") { bridge.setPermission(peer, capability, grant.rawValue) }

    private suspend fun run(
        fallback: String,
        action: () -> Unit,
    ): String? {
        val error =
            withContext(Dispatchers.IO) {
                try {
                    action()
                    null
                } catch (e: ContinueException) {
                    messageFor(e, fallback)
                } catch (_: IOException) {
                    fallback
                } catch (_: SecurityException) {
                    fallback
                }
            }
        refresh()
        return error
    }
}

private fun messageFor(
    error: ContinueException,
    fallback: String,
): String =
    when (error) {
        is ContinueException.InvalidQrException -> "That isn't a Continue pairing code."
        is ContinueException.PairingTimeoutException -> "Pairing took too long. Try again."
        is ContinueException.PairingFailedException ->
            "Couldn't pair. Check that both devices are on the same network and try again."
        else -> fallback
    }

private fun copyInto(
    dir: File,
    context: Context,
    uri: Uri,
): File {
    val file = File(dir, displayName(context, uri))
    val input =
        context.contentResolver.openInputStream(uri)
            ?: throw ContinueException.InternalErrorException("Can't read the file")
    input.use { source -> file.outputStream().use { source.copyTo(it) } }
    return file
}

/** The file's own name, reduced to a single safe path segment. */
private fun displayName(
    context: Context,
    uri: Uri,
): String {
    val name =
        context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) cursor.getString(0) else null
        }
    return name?.substringAfterLast('/')?.takeIf { it.isNotBlank() && it != "." && it != ".." } ?: "file"
}
