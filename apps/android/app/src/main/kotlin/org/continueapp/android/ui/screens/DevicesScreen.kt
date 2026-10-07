package org.continueapp.android.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.ArrowBack
import androidx.compose.material.icons.outlined.Call
import androidx.compose.material.icons.outlined.ContentPaste
import androidx.compose.material.icons.outlined.Folder
import androidx.compose.material.icons.outlined.FolderOpen
import androidx.compose.material.icons.outlined.Laptop
import androidx.compose.material.icons.outlined.Mouse
import androidx.compose.material.icons.outlined.MusicNote
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material.icons.outlined.NotificationsActive
import androidx.compose.material.icons.outlined.PhotoLibrary
import androidx.compose.material.icons.outlined.ScreenShare
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.outlined.Sms
import androidx.compose.material.icons.outlined.Videocam
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.continueapp.android.AppState
import org.continueapp.android.ui.components.DeviceGlyph
import org.continueapp.android.ui.components.PageTitle
import org.continueapp.android.ui.components.ScreenPadding
import org.continueapp.android.ui.components.SectionLabel
import org.continueapp.android.ui.components.SettingsRow
import org.continueapp.android.ui.components.StatusLabel
import org.continueapp.bridge.CAPABILITY_CALLS_ID
import org.continueapp.bridge.CAPABILITY_CAMERA_ID
import org.continueapp.bridge.CAPABILITY_CLIPBOARD_ID
import org.continueapp.bridge.CAPABILITY_FILES_ID
import org.continueapp.bridge.CAPABILITY_FILE_TRANSFER_ID
import org.continueapp.bridge.CAPABILITY_FIND_ID
import org.continueapp.bridge.CAPABILITY_MEDIA_ID
import org.continueapp.bridge.CAPABILITY_MESSAGES_ID
import org.continueapp.bridge.CAPABILITY_NOTIFICATIONS_ID
import org.continueapp.bridge.CAPABILITY_PHOTOS_ID
import org.continueapp.bridge.CAPABILITY_POINTER_ID
import org.continueapp.bridge.CAPABILITY_SCREEN_ID
import org.continueapp.bridge.CAPABILITY_SEARCH_ID
import org.continueapp.bridge.TrustedPeer

@Composable
fun DevicesScreen(
    state: AppState,
    onOpenDevice: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier.verticalScroll(rememberScrollState()).padding(horizontal = ScreenPadding)) {
        PageTitle("Devices")
        if (state.peers.isEmpty()) {
            Text(
                "Nothing paired yet. Tap Pair to add your computer.",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        state.peers.forEach { peer ->
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                DeviceGlyph(Icons.Outlined.Laptop, active = peer.fingerprint in state.connected, size = 48.dp)
                SettingsRow(
                    title = peer.displayName,
                    subtitle = if (peer.fingerprint in state.connected) "Connected" else "Not connected",
                    onClick = { onOpenDevice(peer.fingerprint) },
                    modifier = Modifier.weight(1f),
                )
            }
        }
    }
}

@Composable
fun DeviceScreen(
    state: AppState,
    peer: TrustedPeer,
    onBack: () -> Unit,
    onMessage: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val scope = rememberCoroutineScope()
    val connected = peer.fingerprint in state.connected
    var confirmingForget by remember { mutableStateOf(false) }
    var words by remember { mutableStateOf("") }
    LaunchedEffect(peer.fingerprint) { words = state.pairingWords(peer) }

    Column(modifier = modifier.verticalScroll(rememberScrollState()).padding(horizontal = ScreenPadding)) {
        IconButton(onClick = onBack, modifier = Modifier.padding(top = 8.dp)) {
            Icon(Icons.AutoMirrored.Outlined.ArrowBack, contentDescription = "Back")
        }
        Column(
            modifier = Modifier.fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            DeviceGlyph(Icons.Outlined.Laptop, active = connected, size = 72.dp)
            Text(peer.displayName, style = MaterialTheme.typography.headlineSmall)
            StatusLabel(connected = connected)
            OutlinedButton(
                onClick = {
                    scope.launch {
                        val error =
                            if (connected) state.disconnect(peer.fingerprint) else state.reconnect(peer.fingerprint)
                        error?.let(onMessage)
                    }
                },
            ) {
                Text(if (connected) "Disconnect" else "Connect now")
            }
            if (words.isNotEmpty()) {
                Text(
                    "Pairing words: $words. ${peer.displayName} shows the same four for this phone; " +
                        "if they differ, forget it and pair again.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
            }
        }

        SectionLabel("What ${peer.displayName} can do on this phone")
        PermissionRow(state, peer, CAPABILITY_FILE_TRANSFER_ID, "Send files", Icons.Outlined.Folder, onMessage)
        PermissionRow(state, peer, CAPABILITY_CLIPBOARD_ID, "Send text", Icons.Outlined.ContentPaste, onMessage)
        PermissionRow(state, peer, CAPABILITY_PHOTOS_ID, "See photos", Icons.Outlined.PhotoLibrary, onMessage)
        PermissionRow(state, peer, CAPABILITY_MESSAGES_ID, "Read and send texts", Icons.Outlined.Sms, onMessage)
        PermissionRow(state, peer, CAPABILITY_FILES_ID, "Browse files", Icons.Outlined.FolderOpen, onMessage)
        PermissionRow(state, peer, CAPABILITY_CALLS_ID, "Answer and decline calls", Icons.Outlined.Call, onMessage)
        PermissionRow(
            state,
            peer,
            CAPABILITY_SCREEN_ID,
            "See and control the screen",
            Icons.Outlined.ScreenShare,
            onMessage,
        )
        PermissionRow(
            state,
            peer,
            CAPABILITY_CAMERA_ID,
            "Use the camera as a webcam",
            Icons.Outlined.Videocam,
            onMessage,
        )
        PermissionRow(state, peer, CAPABILITY_MEDIA_ID, "Control what's playing", Icons.Outlined.MusicNote, onMessage)
        PermissionRow(state, peer, CAPABILITY_FIND_ID, "Ring this phone", Icons.Outlined.NotificationsActive, onMessage)
        PermissionRow(state, peer, CAPABILITY_SEARCH_ID, "Search this phone", Icons.Outlined.Search, onMessage)
        PermissionRow(
            state,
            peer,
            CAPABILITY_POINTER_ID,
            "Use its mouse and keyboard here",
            Icons.Outlined.Mouse,
            onMessage,
        )
        PermissionRow(
            state,
            peer,
            CAPABILITY_NOTIFICATIONS_ID,
            "Act on notifications",
            Icons.Outlined.Notifications,
            onMessage,
        )

        TextButton(onClick = { confirmingForget = true }, modifier = Modifier.padding(vertical = 16.dp)) {
            Text("Forget this computer", color = MaterialTheme.colorScheme.error)
        }
    }

    if (confirmingForget) {
        AlertDialog(
            onDismissRequest = { confirmingForget = false },
            title = { Text("Forget ${peer.displayName}?") },
            text = { Text("You'll need to pair it again to send anything between them.") },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmingForget = false
                        scope.launch {
                            val error = state.forget(peer.fingerprint)
                            if (error == null) onBack() else onMessage(error)
                        }
                    },
                ) {
                    Text("Forget", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = { TextButton(onClick = { confirmingForget = false }) { Text("Cancel") } },
        )
    }
}

@Composable
private fun PermissionRow(
    state: AppState,
    peer: TrustedPeer,
    capability: Int,
    title: String,
    icon: ImageVector,
    onMessage: (String) -> Unit,
) {
    val scope = rememberCoroutineScope()
    var allowed by remember { mutableStateOf(true) }
    LaunchedEffect(peer.fingerprint, capability) { allowed = state.isAllowed(peer.fingerprint, capability) }
    val toggle = {
        val turnOn = !allowed
        scope.launch {
            val error = state.setAllowed(peer.fingerprint, capability, turnOn)
            if (error == null) allowed = turnOn else onMessage(error)
        }
        Unit
    }

    SettingsRow(
        title = title,
        icon = icon,
        onClick = toggle,
        trailing = { Switch(checked = allowed, onCheckedChange = { toggle() }) },
    )
}
