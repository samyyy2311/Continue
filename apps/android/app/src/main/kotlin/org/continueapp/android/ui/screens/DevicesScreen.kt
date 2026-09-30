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
import androidx.compose.material.icons.outlined.ContentPaste
import androidx.compose.material.icons.outlined.Folder
import androidx.compose.material.icons.outlined.Laptop
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
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
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.continueapp.android.AppState
import org.continueapp.android.ui.components.DeviceGlyph
import org.continueapp.android.ui.components.PageTitle
import org.continueapp.android.ui.components.ScreenPadding
import org.continueapp.android.ui.components.SectionLabel
import org.continueapp.android.ui.components.SettingsRow
import org.continueapp.android.ui.components.StatusLabel
import org.continueapp.bridge.CAPABILITY_CLIPBOARD_ID
import org.continueapp.bridge.CAPABILITY_FILE_TRANSFER_ID
import org.continueapp.bridge.CAPABILITY_NOTIFICATIONS_ID
import org.continueapp.bridge.PermissionGrant
import org.continueapp.bridge.TrustedPeer
import java.text.DateFormat
import java.util.Date

private const val MILLIS_PER_SECOND = 1000L

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
                "No paired devices yet. Tap Pair to add your computer.",
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

    Column(modifier = modifier.verticalScroll(rememberScrollState()).padding(horizontal = ScreenPadding)) {
        IconButton(onClick = onBack, modifier = Modifier.padding(top = 8.dp)) {
            Icon(Icons.AutoMirrored.Outlined.ArrowBack, contentDescription = "Back")
        }
        Column(
            modifier = Modifier.fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            DeviceGlyph(Icons.Outlined.Laptop, active = connected, size = 96.dp)
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
        }

        SectionLabel("What ${peer.displayName} can do on this phone")
        PermissionRow(state, peer, CAPABILITY_FILE_TRANSFER_ID, "Send files", Icons.Outlined.Folder, onMessage)
        PermissionRow(state, peer, CAPABILITY_CLIPBOARD_ID, "Send text", Icons.Outlined.ContentPaste, onMessage)
        PermissionRow(
            state,
            peer,
            CAPABILITY_NOTIFICATIONS_ID,
            "Send notifications",
            Icons.Outlined.Notifications,
            onMessage,
        )

        SectionLabel("About this device")
        SettingsRow(
            title = "Paired",
            subtitle = DateFormat.getDateInstance().format(Date(peer.pairedAt * MILLIS_PER_SECOND)),
        )
        SettingsRow(title = "Device key", subtitle = peer.fingerprint)

        TextButton(onClick = { confirmingForget = true }, modifier = Modifier.padding(vertical = 16.dp)) {
            Text("Forget this device", color = MaterialTheme.colorScheme.error)
        }
    }

    if (confirmingForget) {
        AlertDialog(
            onDismissRequest = { confirmingForget = false },
            title = { Text("Forget ${peer.displayName}?") },
            text = { Text("It will need to be paired again before it can connect.") },
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
    var grant by remember { mutableStateOf<PermissionGrant?>(null) }
    LaunchedEffect(peer.fingerprint, capability) { grant = state.permission(peer.fingerprint, capability) }

    SettingsRow(title = title, icon = icon)
    Row(
        modifier = Modifier.padding(start = 40.dp, bottom = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        GRANT_CHOICES.forEach { (choice, label) ->
            FilterChip(
                selected = grant == choice,
                onClick = {
                    scope.launch {
                        val error = state.setPermission(peer.fingerprint, capability, choice)
                        if (error == null) grant = choice else onMessage(error)
                    }
                },
                label = { Text(label) },
            )
        }
    }
}

private val GRANT_CHOICES =
    listOf(
        PermissionGrant.ALLOW to "Allow",
        PermissionGrant.ASK to "Ask",
        PermissionGrant.DENY to "Block",
    )
