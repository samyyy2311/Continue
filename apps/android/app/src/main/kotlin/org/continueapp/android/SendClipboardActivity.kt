package org.continueapp.android

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
        val app = application as ContinueApplication
        val text = readCopy(this, app).orEmpty()
        lifecycleScope.launch {
            val message = if (text.isBlank()) "There's nothing copied to send." else app.state.sendToConnected(text)
            Toast.makeText(applicationContext, message, Toast.LENGTH_SHORT).show()
            finish()
        }
    }
}
