package org.continueapp.android.ui.screens

import androidx.compose.foundation.layout.Column
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Laptop
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.platform.LocalContext
import kotlinx.coroutines.launch
import org.continueapp.android.AppState
import org.continueapp.android.Shared
import org.continueapp.android.publishShareTargets
import org.continueapp.android.sendShared
import org.continueapp.android.ui.components.SettingsRow
import org.continueapp.bridge.TrustedPeer

/**
 * Keeps paired computers in the system share sheet, and sends what another app shared: to
 * the computer picked there, or else to one picked here. [onSending] runs once one is chosen,
 * so the caller can show Home, where the transfer appears.
 */
@Composable
fun SharePrompt(
    shared: MutableState<Shared?>,
    state: AppState,
    onMessage: (String) -> Unit,
    onSending: () -> Unit,
) {
    val context = LocalContext.current
    // Taken before the early return so it stays in the composition and the send carries on
    // after the dialog closes.
    val scope = rememberCoroutineScope()
    // Keyed on what the share sheet shows, so the 2-second refresh doesn't republish it.
    val shareTargets = state.peers.map { it.fingerprint to it.displayName }
    LaunchedEffect(shareTargets, state.loaded) {
        if (state.loaded) publishShareTargets(context.applicationContext, state.peers)
    }
    val pending = shared.value
    if (pending == null || !state.loaded) return
    val send = { peer: TrustedPeer ->
        shared.value = null
        onSending()
        scope.launch { onMessage(state.sendShared(context, peer, pending) ?: "Sent to ${peer.displayName}") }
    }
    // Picked straight from the share sheet: nothing left to ask.
    val chosen = state.peers.firstOrNull { it.fingerprint == pending.peerFingerprint }
    if (chosen != null) {
        LaunchedEffect(pending) { send(chosen) }
    } else {
        ShareDialog(shared = pending, state = state, onSend = { send(it) }, onCancel = { shared.value = null })
    }
}

@Composable
private fun ShareDialog(
    shared: Shared,
    state: AppState,
    onSend: (TrustedPeer) -> Unit,
    onCancel: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onCancel,
        title = { Text(if (state.peers.isEmpty()) "No computer paired yet" else "Send ${shared.summary} to") },
        text = {
            if (state.peers.isEmpty()) {
                Text("Pair a computer first, then share again.")
            } else {
                Column {
                    state.peers.forEach { peer ->
                        SettingsRow(
                            title = peer.displayName,
                            icon = Icons.Outlined.Laptop,
                            subtitle = if (peer.fingerprint in state.connected) "Connected" else "Not connected",
                            onClick = { onSend(peer) },
                        )
                    }
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onClick = onCancel) { Text("Cancel") } },
    )
}
