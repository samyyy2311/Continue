package org.continueapp.android

import android.content.ClipData
import android.content.ClipDescription
import android.content.ClipboardManager
import android.content.Context

private const val CLIP_LABEL = "Continue"

/**
 * Copies text to the clipboard and records its timestamp as already seen.
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
 * Reads the first nonblank text item from the clipboard.
 *
 * @return The copied text and clipboard timestamp, or `null` if the clipboard has no timestamp or its
 * first item contains no nonblank text.
 */
fun readCopy(context: Context): Copy? {
    val clipboard = context.getSystemService(ClipboardManager::class.java)
    val at = clipboard?.primaryClipDescription?.timestamp ?: return null
    val text = clipboard.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0)?.text?.toString()
    return text?.takeIf { it.isNotBlank() }?.let { Copy(it, at) }
}

/**
 * Detects a new, non-sensitive clipboard copy.
 *
 * The first observed clipboard timestamp is recorded as already seen, so it is not returned.
 * Subsequent copies are considered new only when their timestamp is later than the recorded
 * timestamp. Android's clipboard notice is triggered only when a new copy is found.
 *
 * @return The new copy, or `null` if the clipboard has no description, the copy is sensitive, or
 * its timestamp is not newer than the last seen timestamp.
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

/**
 * Checks whether the clipboard description is marked as sensitive.
 *
 * @return `true` if the sensitive flag is set, `false` otherwise.
 */
private fun ClipDescription.isSensitive(): Boolean = extras?.getBoolean(IS_SENSITIVE) == true

/**
 * Sends text to connected computers.
 *
 * @return A pair containing whether the send succeeded and a message. The send succeeds only when at least one computer is connected and all sends succeed; otherwise, the message describes the lack of connections or the first send failure.
 */
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
