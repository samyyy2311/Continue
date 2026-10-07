package org.continueapp.android.ui.screens

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalWindowInfo
import org.continueapp.android.AppState
import org.continueapp.android.ContinueApplication
import org.continueapp.android.markSent
import org.continueapp.android.newCopy
import org.continueapp.android.sendToConnected

/**
 * Sends anything copied since the app was last open to the connected computer, each time
 * Continue comes to the front. Android only lets the app on screen read the clipboard.
 */
@Composable
fun SendNewCopies(
    state: AppState,
    onMessage: (String) -> Unit,
) {
    val context = LocalContext.current
    val focused = LocalWindowInfo.current.isWindowFocused
    // Connections are a key too, so a computer that connects after the app opens still gets it.
    LaunchedEffect(focused, state.loaded, state.connected) {
        val app = context.applicationContext as ContinueApplication
        val ready = focused && state.loaded && state.connected.isNotEmpty()
        val copy = if (ready && app.sendNewCopies) newCopy(context, app) else null
        if (copy != null) {
            val (sent, message) = state.sendToConnected(context, copy)
            // Only a copy that went counts as sent; otherwise the next opening tries again.
            if (sent) markSent(app, copy)
            onMessage(message.replace("Sent to", "Sent what you copied to"))
        }
    }
}
