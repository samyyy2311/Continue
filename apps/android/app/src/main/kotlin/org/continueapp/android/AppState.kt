package org.continueapp.android

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.continueapp.bridge.ContinueCoreBridge
import org.continueapp.bridge.ContinueException
import org.continueapp.bridge.HistoryEntry
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

/** Something sent from or to this phone. */
data class Transfer(
    val id: String,
    val kind: TransferKind,
    val label: String,
    val peerName: String,
    val status: TransferStatus,
    /** Where a received file was saved, when it can be opened. */
    val uri: Uri? = null,
    /** Unix time in milliseconds. */
    val at: Long = System.currentTimeMillis(),
    /** The whole text, for text that was sent or received. */
    val text: String? = null,
)

/** What this phone has sent and received, newest first. The core saves it across restarts. */
class RecentTransfers(private val bridge: ContinueCoreBridge) {
    var items by mutableStateOf<List<Transfer>>(emptyList())
        private set
    private var nextId = 0L

    /** Adds what was saved before the app started, below anything already listed. */
    suspend fun load() {
        val saved =
            withContext(Dispatchers.IO) {
                try {
                    bridge.listHistory(RECENT_LIMIT)
                } catch (_: ContinueException) {
                    emptyList()
                }
            }
        items = (items + saved.map { it.toTransfer() }).take(RECENT_LIMIT)
    }

    /** Clears the saved history; anything still sending stays. */
    suspend fun clear(): String? {
        items = items.filter { it.status == TransferStatus.Sending }
        return withContext(Dispatchers.IO) {
            try {
                bridge.clearHistory()
                null
            } catch (_: ContinueException) {
                "Couldn't clear your history."
            }
        }
    }

    /**
     * Tracks a transfer while sending it and updates its status based on the send result.
     *
     * @param text Optional text content to store with the transfer.
     * @param send Performs the send and returns an error message, or `null` on success.
     * @return The error message from [send], or `null` if sending succeeds.
     */
    suspend fun track(
        kind: TransferKind,
        label: String,
        peerName: String,
        text: String? = null,
        send: suspend () -> String?,
    ): String? {
        val id = "live-${nextId++}"
        val transfer = Transfer(id, kind, label, peerName, TransferStatus.Sending, text = text)
        items = (listOf(transfer) + items).take(RECENT_LIMIT)
        val error = send()
        val status = if (error == null) TransferStatus.Sent else TransferStatus.Failed
        items = items.map { if (it.id == id) it.copy(status = status) else it }
        return error
    }

    /**
     * Adds a received transfer to recent history.
     *
     * The transfer is added before existing entries, and the history is limited to 20 items.
     *
     * @param uri The received file's content URI, if applicable.
     * @param text The received text content, if applicable.
     */
    fun received(
        kind: TransferKind,
        label: String,
        peerName: String,
        uri: Uri? = null,
        text: String? = null,
    ) {
        val transfer = Transfer("live-${nextId++}", kind, label, peerName, TransferStatus.Received, uri, text = text)
        items = (listOf(transfer) + items).take(RECENT_LIMIT)
    }
}

/** Text shows as its first line, like it does while it's being sent. */
fun firstLine(text: String): String = text.trim().lineSequence().first()

/**
 * Converts a saved history entry into a transfer.
 *
 * Text entries use their first line as the label and retain the full label as text. File entries include a URI only when the saved location starts with `content://`.
 *
 * @return The transfer with its saved timestamp and status.
 */
private fun HistoryEntry.toTransfer() =
    Transfer(
        id = "saved-$id",
        kind = if (isText) TransferKind.Text else TransferKind.File,
        label = if (isText) firstLine(label) else label,
        peerName = peerName,
        status =
            when {
                received -> TransferStatus.Received
                failed -> TransferStatus.Failed
                else -> TransferStatus.Sent
            },
        uri = location?.takeIf { it.startsWith("content://") }?.let(Uri::parse),
        at = at,
        text = label.takeIf { isText },
    )

/** Questions from devices set to Ask. The core asks one at a time. */
class PermissionQuestions(private val bridge: ContinueCoreBridge) {
    private var shown by mutableStateOf<PermissionQuestion?>(null)

    /** The question waiting for an answer, if there is one. */
    val current: PermissionQuestion? get() = shown

    /** Hears each new question, and null once it's answered or has run out of time. */
    var onChange: (PermissionQuestion?) -> Unit = {}

    /**
     * Listens for permission questions and displays each one until it expires.
     */
    suspend fun listen() =
        coroutineScope {
            while (true) {
                val next = withContext(Dispatchers.IO) { bridge.nextPermissionQuestion(QUESTION_WAIT_MS) }
                if (next != null && next.expiresAt > System.currentTimeMillis()) {
                    show(next)
                    launch {
                        delay(next.expiresAt - System.currentTimeMillis())
                        if (shown?.id == next.id) show(null)
                    }
                }
            }
        }

    /**
     * Submits an answer to a permission question.
     *
     * Clears the displayed question if its ID matches [id]. Answers are submitted even if the question is no longer displayed.
     */
    suspend fun answer(
        id: Long,
        answer: PermissionAnswer,
    ) {
        if (shown?.id == id) show(null)
        withContext(Dispatchers.IO) { bridge.answerPermissionQuestion(id, answer) }
    }

    /**
     * Updates the displayed permission question and notifies the change listener.
     *
     * @param question The question to display, or `null` to clear it.
     */
    private fun show(question: PermissionQuestion?) {
        shown = question
        onChange(question)
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

    /** False until the core has been read once, so screens don't flash "nothing paired". */
    var loaded by mutableStateOf(false)
        private set
    val recent = RecentTransfers(bridge)
    val questions = PermissionQuestions(bridge)
    val incoming = Incoming(bridge, recent)

    /**
     * Refreshes the peer and connection state from the core.
     *
     * If reading the core fails, the current state is retained and `loaded` is unchanged.
     * On success, the refreshed state is stored and `loaded` is set to `true`.
     */
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
        loaded = true
    }

    /**
     * Pairs with a peer using a QR code.
     *
     * @return An error message if pairing fails, or `null` if it succeeds.
     */
    suspend fun pair(code: String): String? =
        run("Pairing didn't work. Try again.") {
            bridge.pairFromQr(code)
        }

    /**
     * Disconnects from a peer.
     *
     * @return An error message if disconnection fails, or `null` if it succeeds.
     */
    suspend fun disconnect(peer: String): String? = run("Couldn't disconnect.") { bridge.disconnect(peer) }

    /**
     * Reconnects to a peer.
     *
     * @param peer The peer identifier.
     * @return An error message if reconnection fails, or `null` if it succeeds.
     */
    suspend fun reconnect(peer: String): String? =
        run("Couldn't connect. Check that both are on the same Wi-Fi.") { bridge.reconnect(peer) }

    /**
     * Removes a trusted peer.
     *
     * @param peer The peer to forget.
     * @return An error message if the peer could not be forgotten, or `null` on success.
     */
    suspend fun forget(peer: String): String? {
        val failed = "Couldn't forget this computer. Try again."
        return run(failed) { bridge.removeTrustedPeer(peer) }
    }

    /**
     * Sends text to a peer and records the transfer in recent history.
     *
     * @return An error message if sending fails, or `null` if it succeeds.
     */
    suspend fun sendText(
        peer: TrustedPeer,
        text: String,
    ): String? =
        recent.track(TransferKind.Text, firstLine(text), peer.displayName, text) {
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

    /**
     * Updates a peer's permission for a capability.
     *
     * @param peer The peer whose permission to update.
     * @param capability The capability whose permission to update.
     * @param grant The permission decision to apply.
     * @return An error message if the update fails, or `null` if it succeeds.
     */
    suspend fun setPermission(
        peer: String,
        capability: Int,
        grant: PermissionGrant,
    ): String? = run("Couldn't save that change. Try again.") { bridge.setPermission(peer, capability, grant.rawValue) }

    /**
     * Runs an action and refreshes the application state, converting handled failures to an error message.
     *
     * @param fallback The error message used for I/O, security, and unrecognized core failures.
     * @param action The operation to run.
     * @return An error message if the action fails with a handled exception, or `null` if it succeeds.
     */
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
