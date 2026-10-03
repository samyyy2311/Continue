package org.continueapp.android

import android.content.ContentValues
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import androidx.annotation.RequiresApi
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import org.continueapp.bridge.ContinueCoreBridge
import org.continueapp.bridge.ContinueException
import org.continueapp.bridge.ReceivedFile
import org.continueapp.bridge.ReceivedText
import java.io.File
import java.io.IOException
import java.net.URLConnection

private const val RECEIVE_WAIT_MS = 1_000L
private const val SAVE_FOLDER = "Continue"

/** Takes what paired devices send while the app is open: files go to Downloads, text to the clipboard. */
class Incoming(
    private val bridge: ContinueCoreBridge,
    private val recent: RecentTransfers,
) {
    /** Hears about each arrival: a title, a line of detail, and what tapping it should open. */
    var onArrival: (title: String, detail: String, open: Intent?) -> Unit = { _, _, _ -> }

    /**
     * Takes what arrives while [active] says to. Otherwise it waits, so nothing is saved or
     * copied to the clipboard until it is.
     */
    suspend fun listen(
        context: Context,
        active: () -> Boolean,
    ) {
        while (true) {
            if (!active()) {
                delay(RECEIVE_WAIT_MS)
                continue
            }
            when (val item = withContext(Dispatchers.IO) { bridge.nextReceived(RECEIVE_WAIT_MS) }) {
                is ReceivedFile -> {
                    val uri = withContext(Dispatchers.IO) { saveToDownloads(context, item) }
                    recent.received(TransferKind.File, item.name, item.peerName, uri)
                    val open = uri?.let { viewIntent(it, item.name) }
                    onArrival("${item.peerName.ifBlank { "Your computer" }} sent a file", item.name, open)
                    val id = item.historyId
                    if (uri != null && id != null) remember(id, uri)
                }
                is ReceivedText -> {
                    copyToClipboard(context, item.text)
                    recent.received(TransferKind.Text, firstLine(item.text), item.peerName, text = item.text)
                    val from = item.peerName.ifBlank { "your computer" }
                    onArrival("Copied text from $from", firstLine(item.text), null)
                }
                null -> Unit
            }
        }
    }

    /** Saves where the file went, so it can still be opened after a restart. */
    private suspend fun remember(
        historyId: Long,
        uri: Uri,
    ) = withContext(Dispatchers.IO) {
        try {
            bridge.setHistoryLocation(historyId, uri.toString())
        } catch (_: ContinueException) {
        }
    }
}

/** Opens a received file in whatever app handles its type. */
fun viewIntent(
    uri: Uri,
    name: String,
): Intent =
    Intent(Intent.ACTION_VIEW)
        .setDataAndType(uri, mimeType(name))
        .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)

fun mimeType(name: String): String = URLConnection.guessContentTypeFromName(name) ?: "application/octet-stream"

/**
 * Moves a received file into Downloads/Continue. Returns its content URI on Android 10 and
 * later; older versions keep it in the app's own Downloads folder, which file managers can
 * see, and return null.
 */
private fun saveToDownloads(
    context: Context,
    file: ReceivedFile,
): Uri? {
    val source = File(file.path)
    return try {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            saveToMediaStore(context, source, file.name)
        } else {
            context.getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS)?.let { dir ->
                source.copyTo(freeName(dir, file.name))
                source.delete()
            }
            null
        }
    } catch (_: IOException) {
        null
    }
}

@RequiresApi(Build.VERSION_CODES.Q)
private fun saveToMediaStore(
    context: Context,
    source: File,
    name: String,
): Uri? {
    val resolver = context.contentResolver
    val values =
        ContentValues().apply {
            put(MediaStore.Downloads.DISPLAY_NAME, name)
            put(MediaStore.Downloads.MIME_TYPE, mimeType(name))
            put(MediaStore.Downloads.RELATIVE_PATH, "${Environment.DIRECTORY_DOWNLOADS}/$SAVE_FOLDER")
        }
    val uri = resolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values) ?: return null
    try {
        val output = resolver.openOutputStream(uri) ?: throw IOException("Downloads isn't writable")
        output.use { out -> source.inputStream().use { it.copyTo(out) } }
    } catch (e: IOException) {
        resolver.delete(uri, null, null)
        throw e
    }
    source.delete()
    return uri
}

/** [name] in [dir], or "name (2)", "name (3)" and so on if it's taken. */
private fun freeName(
    dir: File,
    name: String,
): File {
    val base = name.substringBeforeLast('.')
    val extension = name.substringAfterLast('.', "").let { if (it.isEmpty()) "" else ".$it" }
    return generateSequence(1) { it + 1 }
        .map { n -> File(dir, if (n == 1) name else "$base ($n)$extension") }
        .first { !it.exists() }
}
