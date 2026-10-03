package org.continueapp.android

import android.widget.Toast
import androidx.activity.ComponentActivity
import kotlinx.coroutines.launch

/**
 * Opened from the background notification and the Quick Settings tile. Android only lets the
 * app on screen read the clipboard, so this shows nothing, reads it once it has focus, and
 * closes. Sending carries on in the app, so turning the phone can't interrupt it.
 */
class SendClipboardActivity : ComponentActivity() {
    private var read = false

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (!hasFocus || read) return
        read = true
        val app = application as ContinueApplication
        val copy = readCopy(this)
        finish()
        app.scope.launch {
            val message =
                if (copy == null) {
                    "There's nothing copied to send."
                } else {
                    val (sent, message) = app.state.sendToConnected(copy.text)
                    if (sent) markSent(app, copy)
                    message
                }
            Toast.makeText(app, message, Toast.LENGTH_SHORT).show()
        }
    }
}
