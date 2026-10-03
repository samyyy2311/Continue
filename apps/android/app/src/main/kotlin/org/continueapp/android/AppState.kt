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
import org.continueapp.bridge.PermissionAnswer
import org.continueapp.bridge.PermissionGrant
import org.continueapp.bridge.PermissionQuestion
import org.continueapp.bridge.TrustedPeer
import java.io.File
import java.io.IOException
import java.util.UUID

private const val RECENT_LIMIT = 20
private const val QUESTION_WAIT_MS = 1_000L

enum class TransferKind { File, Text }

enum class TransferStatus { Sending, Sent, Failed, Received }

/** Something sent from or to this phone during this session. */
data class Transfer(
    val id: Long,
    val kind: TransferKind,
    val label: String,
    val peerName: String,
    val status: TransferStatus,
    /** Where a received file was saved, when it can be opened. */
    val uri: Uri? = null,
)

/** What this phone has sent and received while the app runs, newest first. */
class RecentTransfers {
    var items by mutableStateOf<List<Transfer>>(emptyList())
        private set
    private var nextId = 0L

    /** Lists the transfer as sending, runs [send], then marks it sent or failed by its result. */
    suspend fun track(
        kind: TransferKind,
        label: String,
        peerName: String,
        send: suspend () -> String?,
    ): String? {
        val id = nextId++
        items = (listOf(Transfer(id, kind, label, peerName, TransferStatus.Sending)) + items).take(RECENT_LIMIT)
        val error = send()
        val status = if (error == null) TransferStatus.Sent else TransferStatus.Failed
        items = items.map { if (it.id == id) it.copy(status = status) else it }
        return error
    }

    fun received(
        kind: TransferKind,
        label: String,
        peerName: String,
        uri: Uri?,
    ) {
        val transfer = Transfer(nextId++, kind, label, peerName, TransferStatus.Received, uri)
        items = (listOf(transfer) + items).take(RECENT_LIMIT)
    }
}

/** Questions from devices set to Ask. The core asks one at a time. */
class PermissionQuestions(private val bridge: ContinueCoreBridge) {
    var current by mutableStateOf<PermissionQuestion?>(null)
        private set

    /** Picks up questions from the core for as long as the caller keeps it running. */
    suspend fun listen() {
        while (true) {
            withContext(Dispatchers.IO) { bridge.nextPermissionQuestion(QUESTION_WAIT_MS) }?.let { current = it }
        }
    }

    suspend fun answer(answer: PermissionAnswer) {
        val asked = current ?: return
        current = null
        withContext(Dispatchers.IO) { bridge.answerPermissionQuestion(asked.id, answer) }
    }

    /** Hides a question the core has stopped waiting on. */
    fun dismiss(id: Long) {
        if (current?.id == id) current = null
    }
}

/**
 * What the screens show, read from the core. Every call into the core runs off the main
 * thread, and failures come back as a message to show rather than an exception.
 */
class AppState(private val bridge: ContinueCoreBridge) {
    var peers by mutableStateOf<List<TrustedPeer>>(emptyList())
        private set
    var connected by mutableStateOf<Set<String>>(emptySet())
        private set
    val recent = RecentTransfers()
    val questions = PermissionQuestions(bridge)
    val incoming = Incoming(bridge, recent)

    /** Keeps what's on screen if the core can't be read this time; the next refresh tries again. */
    suspend fun refresh() {
        val (latestPeers, latestConnected) =
            withContext(Dispatchers.IO) {
                try {
                    val list = bridge.listTrustedPeers()
                    list to list.filter { bridge.isPeerConnected(it.fingerprint) }.map { it.fingerprint }.toSet()
                } catch (_: ContinueException) {
                    null
                }
            } ?: return
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
        peer: TrustedPeer,
        text: String,
    ): String? =
        recent.track(TransferKind.Text, text.trim().lineSequence().first(), peer.displayName) {
            run("Couldn't send the text.") { bridge.sendClipboardText(peer.fingerprint, text) }
        }

    /**
     * Sends picked files one by one, carrying on past a failure so each gets its own result.
     * The core reads from a path, so each file is copied into the app's cache first.
     * Returns the last error, or null if everything was sent.
     */
    suspend fun sendFiles(
        context: Context,
        peer: TrustedPeer,
        uris: List<Uri>,
    ): String? {
        var error: String? = null
        for (uri in uris) {
            val name = withContext(Dispatchers.IO) { displayName(context, uri) }
            recent.track(TransferKind.File, name, peer.displayName) {
                run("Couldn't send $name.") {
                    val outbox = File(context.cacheDir, "outgoing/${UUID.randomUUID()}").apply { mkdirs() }
                    try {
                        bridge.sendFile(peer.fingerprint, copyInto(File(outbox, name), context, uri).absolutePath)
                    } finally {
                        outbox.deleteRecursively()
                    }
                }
            }?.let { error = it }
        }
        return error
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
    file: File,
    context: Context,
    uri: Uri,
): File {
    val input =
        context.contentResolver.openInputStream(uri)
            ?: throw ContinueException.InternalErrorException("Can't read the file")
    input.use { source -> file.outputStream().use { source.copyTo(it) } }
    return file
}

/**
 * The file's own name, reduced to a single safe path segment. Falls back to "file" when the
 * provider won't say; reading the file itself is where a real permission problem surfaces.
 */
private fun displayName(
    context: Context,
    uri: Uri,
): String {
    val name =
        try {
            context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
                if (cursor.moveToFirst()) cursor.getString(0) else null
            }
        } catch (_: SecurityException) {
            null
        }
    return name?.substringAfterLast('/')?.takeIf { it.isNotBlank() && it != "." && it != ".." } ?: "file"
}
