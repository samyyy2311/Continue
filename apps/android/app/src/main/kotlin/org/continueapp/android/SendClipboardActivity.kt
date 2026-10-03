package org.continueapp.android

import android.content.ClipboardManager
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch

/**
 * Opened from the background notification. Android only lets the app on screen read the
 * clipboard, so this shows nothing, reads it once it has focus, sends it to every connected
 * computer, and closes.
 */
class SendClipboardActivity : ComponentActivity() {
    private var sending = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Nothing to do after a rotation; the first one is already sending.
        sending = savedInstanceState != null
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (!hasFocus || sending) return
        sending = true
        val text =
            getSystemService(ClipboardManager::class.java)
                ?.primaryClip
                ?.takeIf { it.itemCount > 0 }
                ?.getItemAt(0)
                ?.coerceToText(this)
                ?.toString()
                .orEmpty()
        val state = (application as ContinueApplication).state
        lifecycleScope.launch {
            val message = if (text.isBlank()) "There's nothing copied to send." else state.sendToConnected(text)
            Toast.makeText(applicationContext, message, Toast.LENGTH_SHORT).show()
            finish()
        }
    }
}

/** Sends text to every connected computer, and says how that went. */
private suspend fun AppState.sendToConnected(text: String): String {
    refresh()
    val targets = peers.filter { it.fingerprint in connected }
    val failed = targets.map { sendText(it, text) }.firstOrNull { it != null }
    return when {
        targets.isEmpty() -> "Your computer isn't connected."
        failed != null -> failed
        else -> "Sent to ${targets.joinToString { it.displayName }}"
    }
}
