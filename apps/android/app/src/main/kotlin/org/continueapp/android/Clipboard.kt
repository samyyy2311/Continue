package org.continueapp.android

import android.content.ClipData
import android.content.ClipDescription
import android.content.ClipboardManager
import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import androidx.core.content.FileProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream
import java.io.File

private const val CLIP_LABEL = "Continue"

fun copyToClipboard(
    context: Context,
    text: String,
) = setClip(context, ClipData.newPlainText(CLIP_LABEL, text))

/** Shared through [FileProvider] from the app's cache, which other apps can't read directly. */
fun copyImageToClipboard(
    context: Context,
    png: ByteArray,
) {
    val file = File(context.cacheDir, "clipboard").apply { mkdirs() }.resolve("copied.png")
    file.writeBytes(png)
    val uri = FileProvider.getUriForFile(context, "${context.packageName}.files", file)
    setClip(context, ClipData.newUri(context.contentResolver, CLIP_LABEL, uri))
}

/** Notes the copy as seen so it isn't sent straight back the next time the app opens. */
private fun setClip(
    context: Context,
    clip: ClipData,
) {
    val clipboard = context.getSystemService(ClipboardManager::class.java) ?: return
    clipboard.setPrimaryClip(clip)
    val app = context.applicationContext as ContinueApplication
    // In the background Android won't describe the clip, but it was copied just now.
    val copiedAt = clipboard.primaryClipDescription?.timestamp ?: System.currentTimeMillis()
    app.lastCopySeen = maxOf(app.lastCopySeen, copiedAt)
}

/** Text or an image that was copied, and when, so it can be marked as dealt with once it's sent. */
class Copy(
    val text: String?,
    val image: Uri?,
    val at: Long,
)

/** Read only while Continue is on screen, as Android requires. */
fun readCopy(context: Context): Copy? {
    val clipboard = context.getSystemService(ClipboardManager::class.java)
    val description = clipboard?.primaryClipDescription ?: return null
    val item = clipboard.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0) ?: return null
    val text = item.text?.toString()?.takeIf { it.isNotBlank() }
    val image = item.uri?.takeIf { description.hasMimeType("image/*") }
    return if (text != null || image != null) Copy(text, image, description.timestamp) else null
}

/**
 * Something copied since the app last sent a copy, or null. Skips copies marked sensitive, as
 * password managers do. Only the copy's time is checked until there's something new, so
 * Android's "pasted from your clipboard" notice only shows when there is.
 */
fun newCopy(
    context: Context,
    app: ContinueApplication,
): Copy? {
    val description = context.getSystemService(ClipboardManager::class.java)?.primaryClipDescription
    val seen = app.lastCopySeen
    if (description == null || description.isSensitive()) return null
    // The first time, whatever is already copied counts as old.
    if (seen == 0L) app.lastCopySeen = description.timestamp
    return if (seen != 0L && description.timestamp > seen) readCopy(context) else null
}

/** Marks a copy as dealt with, so it isn't sent again. */
fun markSent(
    app: ContinueApplication,
    copy: Copy,
) {
    app.lastCopySeen = maxOf(app.lastCopySeen, copy.at)
}

/** The extra password managers set on what they copy. Read by name, as apps set it on every version. */
private const val IS_SENSITIVE = "android.content.extra.IS_SENSITIVE"

private fun ClipDescription.isSensitive(): Boolean = extras?.getBoolean(IS_SENSITIVE) == true

/** Sends a copy to every connected computer. Says whether it all went, and how to put it. */
suspend fun AppState.sendToConnected(
    context: Context,
    copy: Copy,
): Pair<Boolean, String> {
    refresh()
    val targets = peers.filter { it.fingerprint in connected }
    // Text copied while the computer is away goes once it's back; images aren't kept.
    if (targets.isEmpty() && copy.text != null && peers.isNotEmpty()) {
        val failed = peers.map { sendLater(context, it, copy.text, emptyList()) }.firstOrNull { it != null }
        val names = peers.joinToString { it.displayName }
        return if (failed != null) false to failed else true to "Sends when $names connects"
    }
    val png = copy.image?.let { withContext(Dispatchers.IO) { readPng(context, it) } }
    val failed =
        targets
            .map { peer -> if (png != null) sendImage(peer, png) else sendText(peer, copy.text.orEmpty()) }
            .firstOrNull { it != null }
    return when {
        targets.isEmpty() -> false to "Your computer isn't connected."
        failed != null -> false to failed
        else -> true to "Sent to ${targets.joinToString { it.displayName }}"
    }
}

/** Computers expect PNG, so other formats are converted. */
private fun readPng(
    context: Context,
    uri: Uri,
): ByteArray? {
    val bytes = context.contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: return null
    if (context.contentResolver.getType(uri) == "image/png") return bytes
    val bitmap = BitmapFactory.decodeByteArray(bytes, 0, bytes.size) ?: return null
    return ByteArrayOutputStream().use {
        bitmap.compress(Bitmap.CompressFormat.PNG, 0, it)
        it.toByteArray()
    }
}
