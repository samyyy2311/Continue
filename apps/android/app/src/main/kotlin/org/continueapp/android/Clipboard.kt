package org.continueapp.android

import android.content.ClipData
import android.content.ClipDescription
import android.content.ClipboardManager
import android.content.Context

private const val CLIP_LABEL = "Continue"

/**
 * Puts text from a computer on the clipboard, and notes the copy as seen so it isn't sent
 * straight back the next time the app opens.
 */
fun copyToClipboard(
    context: Context,
    text: String,
) {
    val clipboard = context.getSystemService(ClipboardManager::class.java) ?: return
    clipboard.setPrimaryClip(ClipData.newPlainText(CLIP_LABEL, text))
    val app = context.applicationContext as ContinueApplication
    clipboard.primaryClipDescription?.let { app.lastCopySeen = it.timestamp }
}

/** Text that was copied, and when, so it can be marked as dealt with once it's sent. */
class Copy(
    val text: String,
    val at: Long,
)

/**
 * The copied text, read only while Continue is on screen, as Android requires. Images and
 * other copies that aren't text count as nothing to send.
 */
fun readCopy(context: Context): Copy? {
    val clipboard = context.getSystemService(ClipboardManager::class.java)
    val at = clipboard?.primaryClipDescription?.timestamp ?: return null
    val text = clipboard.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0)?.text?.toString()
    return text?.takeIf { it.isNotBlank() }?.let { Copy(it, at) }
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

/** Sends text to every connected computer. Says whether it all went, and how to put it. */
suspend fun AppState.sendToConnected(text: String): Pair<Boolean, String> {
    refresh()
    val targets = peers.filter { it.fingerprint in connected }
    val failed = targets.map { sendText(it, text) }.firstOrNull { it != null }
    return when {
        targets.isEmpty() -> false to "Your computer isn't connected."
        failed != null -> false to failed
        else -> true to "Sent to ${targets.joinToString { it.displayName }}"
    }
}
