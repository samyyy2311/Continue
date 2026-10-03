package org.continueapp.android.ui.screens

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Send
import androidx.compose.material.icons.outlined.ContentPaste
import androidx.compose.material.icons.outlined.Description
import androidx.compose.material.icons.outlined.Done
import androidx.compose.material.icons.outlined.ErrorOutline
import androidx.compose.material.icons.outlined.Laptop
import androidx.compose.material.icons.outlined.QrCodeScanner
import androidx.compose.material.icons.outlined.UploadFile
import androidx.compose.material.icons.outlined.Wifi
import androidx.compose.material.icons.outlined.WifiOff
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.continueapp.android.AppState
import org.continueapp.android.Transfer
import org.continueapp.android.TransferKind
import org.continueapp.android.TransferStatus
import org.continueapp.android.ui.components.ActionButton
import org.continueapp.android.ui.components.DeviceGlyph
import org.continueapp.android.ui.components.PageTitle
import org.continueapp.android.ui.components.ScreenPadding
import org.continueapp.android.ui.components.SectionLabel
import org.continueapp.android.ui.components.SettingsRow
import org.continueapp.android.ui.components.StatusLabel
import org.continueapp.android.ui.theme.success
import org.continueapp.bridge.TrustedPeer

private const val RECENT_ON_HOME = 5

@Composable
fun HomeScreen(
    state: AppState,
    visible: Boolean,
    onPair: () -> Unit,
    onOpenSettings: () -> Unit,
    onMessage: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    var selected by remember { mutableStateOf<String?>(null) }
    val peer = state.peers.firstOrNull { it.fingerprint == selected } ?: state.peers.firstOrNull()

    Column(modifier = modifier.verticalScroll(rememberScrollState()).padding(horizontal = ScreenPadding)) {
        PageTitle("Continue")
        VisibilityLine(visible = visible, onClick = onOpenSettings)
        if (peer == null) {
            NoDevices(onPair = onPair)
            return@Column
        }
        if (state.peers.size > 1) {
            Row(
                modifier = Modifier.horizontalScroll(rememberScrollState()).padding(top = 16.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                state.peers.forEach {
                    FilterChip(
                        selected = it.fingerprint == peer.fingerprint,
                        onClick = { selected = it.fingerprint },
                        label = { Text(it.displayName) },
                    )
                }
            }
        }
        LinkPanel(peer = peer, connected = peer.fingerprint in state.connected, state = state, onMessage = onMessage)
        SectionLabel("Send to ${peer.displayName}")
        SendActions(peer = peer, enabled = peer.fingerprint in state.connected, state = state, onMessage = onMessage)
        if (state.recent.items.isNotEmpty()) {
            SectionLabel("Recent")
            state.recent.items.take(RECENT_ON_HOME).forEach { RecentRow(it) }
        }
    }
}

@Composable
private fun VisibilityLine(
    visible: Boolean,
    onClick: () -> Unit,
) {
    Row(
        modifier = Modifier.clip(RoundedCornerShape(12.dp)).clickable(onClick = onClick).padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Icon(
            if (visible) Icons.Outlined.Wifi else Icons.Outlined.WifiOff,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.size(18.dp),
        )
        Text(
            if (visible) "Visible on this network" else "Hidden from other devices",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun NoDevices(onPair: () -> Unit) {
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(top = 24.dp)
                .clip(MaterialTheme.shapes.extraLarge)
                .background(MaterialTheme.colorScheme.surfaceContainer)
                .padding(28.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        DeviceGlyph(Icons.Outlined.Laptop, active = false)
        Text("Pair with your computer", style = MaterialTheme.typography.headlineSmall)
        Text(
            "Open Continue on your computer, choose Pair Device, and scan the code it shows.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
        ActionButton(onClick = onPair) {
            Icon(Icons.Outlined.QrCodeScanner, contentDescription = null)
            Text("Scan code", modifier = Modifier.padding(start = 8.dp))
        }
    }
}

@Composable
private fun LinkPanel(
    peer: TrustedPeer,
    connected: Boolean,
    state: AppState,
    onMessage: (String) -> Unit,
) {
    val scope = rememberCoroutineScope()
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(top = 20.dp)
                .clip(MaterialTheme.shapes.extraLarge)
                .background(MaterialTheme.colorScheme.surfaceContainer)
                .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
            DeviceGlyph(Icons.Outlined.Laptop, active = connected, size = 56.dp)
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(peer.displayName, style = MaterialTheme.typography.titleLarge)
                StatusLabel(connected = connected)
            }
            OutlinedButton(
                onClick = {
                    scope.launch {
                        val error =
                            if (connected) state.disconnect(peer.fingerprint) else state.reconnect(peer.fingerprint)
                        error?.let(onMessage)
                    }
                },
            ) {
                Text(if (connected) "Disconnect" else "Connect")
            }
        }
        if (!connected) {
            Text(
                "Connects on its own when both devices are on the same network.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun SendActions(
    peer: TrustedPeer,
    enabled: Boolean,
    state: AppState,
    onMessage: (String) -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var sending by remember { mutableStateOf(false) }
    var composingText by remember { mutableStateOf(false) }
    val pickFiles =
        rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
            if (uris.isEmpty()) return@rememberLauncherForActivityResult
            sending = true
            scope.launch {
                state.sendFiles(context, peer, uris)?.let(onMessage)
                sending = false
            }
        }

    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        SendButton(
            icon = Icons.Outlined.UploadFile,
            label = if (sending) "Sending…" else "Send files",
            enabled = enabled && !sending,
            onClick = { pickFiles.launch(arrayOf("*/*")) },
            modifier = Modifier.weight(1f),
        )
        SendButton(
            icon = Icons.Outlined.ContentPaste,
            label = "Send text",
            enabled = enabled,
            onClick = { composingText = true },
            modifier = Modifier.weight(1f),
        )
    }
    if (!enabled) {
        Text(
            "You can send once ${peer.displayName} is connected.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = 12.dp),
        )
    }
    if (composingText) {
        SendTextDialog(
            onDismiss = { composingText = false },
            onSend = { text ->
                composingText = false
                scope.launch { state.sendText(peer, text)?.let(onMessage) }
            },
        )
    }
}

@Composable
private fun SendButton(
    icon: ImageVector,
    label: String,
    enabled: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    ActionButton(onClick = onClick, enabled = enabled, tonal = true, modifier = modifier) {
        Icon(icon, contentDescription = null)
        Text(label, modifier = Modifier.padding(start = 8.dp))
    }
}

@Composable
private fun RecentRow(transfer: Transfer) {
    val colors = MaterialTheme.colorScheme
    SettingsRow(
        title = transfer.label,
        icon = if (transfer.kind == TransferKind.File) Icons.Outlined.Description else Icons.Outlined.ContentPaste,
        subtitle =
            when (transfer.status) {
                TransferStatus.Sending -> "Sending to ${transfer.peerName}"
                TransferStatus.Sent -> "Sent to ${transfer.peerName}"
                TransferStatus.Failed -> "Couldn't send to ${transfer.peerName}"
            },
        trailing = {
            when (transfer.status) {
                TransferStatus.Sending -> CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp)
                TransferStatus.Sent -> Icon(Icons.Outlined.Done, null, tint = colors.success)
                TransferStatus.Failed -> Icon(Icons.Outlined.ErrorOutline, null, tint = colors.error)
            }
        },
    )
}

@Composable
private fun SendTextDialog(
    onDismiss: () -> Unit,
    onSend: (String) -> Unit,
) {
    val clipboard = LocalClipboardManager.current
    var text by remember { mutableStateOf(clipboard.getText()?.text.orEmpty()) }
    AlertDialog(
        onDismissRequest = onDismiss,
        icon = { Icon(Icons.AutoMirrored.Outlined.Send, contentDescription = null) },
        title = { Text("Send text") },
        text = {
            OutlinedTextField(
                value = text,
                onValueChange = { text = it },
                placeholder = { Text("Type or paste text") },
                modifier = Modifier.fillMaxWidth(),
                minLines = 3,
            )
        },
        confirmButton = { TextButton(onClick = { onSend(text) }, enabled = text.isNotBlank()) { Text("Send") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
