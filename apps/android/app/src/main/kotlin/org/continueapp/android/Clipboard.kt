package org.continueapp.android

import android.content.ClipData
import android.content.ClipDescription
import android.content.ClipboardManager
import android.content.Context
import android.os.Build

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

/** What's copied, read only while Continue is on screen, as Android requires. */
fun readCopy(
    context: Context,
    app: ContinueApplication,
): String? {
    val clipboard = context.getSystemService(ClipboardManager::class.java) ?: return null
    clipboard.primaryClipDescription?.let { app.lastCopySeen = it.timestamp }
    return clipboard.primaryClip
        ?.takeIf { it.itemCount > 0 }
        ?.getItemAt(0)
        ?.coerceToText(context)
        ?.toString()
}

/**
 * Something copied since the app last looked, or null. Skips copies marked sensitive, as
 * password managers do. Only the copy's time is checked until there's something new, so
 * Android's "pasted from your clipboard" notice only shows when there is.
 */
fun newCopy(
    context: Context,
    app: ContinueApplication,
): String? {
    val description = context.getSystemService(ClipboardManager::class.java)?.primaryClipDescription
    val seen = app.lastCopySeen
    val fresh = description != null && description.timestamp > seen && !description.isSensitive()
    // The first time, whatever is already copied counts as old.
    if (seen == 0L || !fresh) {
        description?.let { app.lastCopySeen = it.timestamp }
        return null
    }
    return readCopy(context, app)?.takeIf { it.isNotBlank() }
}

private fun ClipDescription.isSensitive(): Boolean =
    Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
        extras?.getBoolean(ClipDescription.EXTRA_IS_SENSITIVE) == true

/** Sends text to every connected computer, and says how that went. */
suspend fun AppState.sendToConnected(text: String): String {
    refresh()
    val targets = peers.filter { it.fingerprint in connected }
    val failed = targets.map { sendText(it, text) }.firstOrNull { it != null }
    return when {
        targets.isEmpty() -> "Your computer isn't connected."
        failed != null -> failed
        else -> "Sent to ${targets.joinToString { it.displayName }}"
    }
}
