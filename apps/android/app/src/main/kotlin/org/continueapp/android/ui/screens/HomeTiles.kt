package org.continueapp.android.ui.screens

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Message
import androidx.compose.material.icons.automirrored.outlined.Send
import androidx.compose.material.icons.outlined.Bedtime
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.ContentPaste
import androidx.compose.material.icons.outlined.DocumentScanner
import androidx.compose.material.icons.outlined.Keyboard
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material.icons.outlined.NotificationsActive
import androidx.compose.material.icons.outlined.PushPin
import androidx.compose.material.icons.outlined.TouchApp
import androidx.compose.material.icons.outlined.UploadFile
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.continueapp.android.AppState
import org.continueapp.android.ContinueApplication
import org.continueapp.android.copyToClipboard
import org.continueapp.android.hasNotificationAccess
import org.continueapp.android.markSent
import org.continueapp.android.readCopy
import org.continueapp.android.scanPdf
import org.continueapp.android.scanPhotoUri
import org.continueapp.android.sendToConnected
import org.continueapp.bridge.ComputerAction
import org.continueapp.bridge.Snippet
import org.continueapp.bridge.TrustedPeer

private const val DISABLED_ALPHA = 0.45f

@Composable
internal fun Tiles(
    peer: TrustedPeer,
    connected: Boolean,
    state: AppState,
    onMessage: (String) -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var sending by remember { mutableStateOf(false) }
    var ringing by remember { mutableStateOf(false) }
    var dialog by remember { mutableStateOf<TileDialog?>(null) }

    var notifications by remember { mutableStateOf(hasNotificationAccess(context)) }
    val pickFiles =
        rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
            if (uris.isEmpty()) return@rememberLauncherForActivityResult
            sending = true
            scope.launch {
                state.sendFiles(context, peer, uris)?.let(onMessage)
                sending = false
            }
        }
    val scanned =
        rememberLauncherForActivityResult(ActivityResultContracts.TakePicture()) { taken ->
            if (!taken) return@rememberLauncherForActivityResult
            sending = true
            scope.launch {
                val pdf = withContext(Dispatchers.IO) { scanPdf(context) }
                if (pdf == null) {
                    onMessage("Couldn't read the photo. Try again.")
                } else {
                    state.sendFiles(context, peer, listOf(pdf))?.let(onMessage)
                }
                sending = false
            }
        }
    val scan = { scanned.launch(scanPhotoUri(context)) }
    // The camera app needs the camera permission too, since this app declares it for the webcam.
    val allowCamera =
        rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            if (granted) scan() else onMessage("Scanning needs the camera.")
        }
    val openNotificationAccess =
        rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) {
            notifications = hasNotificationAccess(context)
        }

    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Tile(
                icon = Icons.Outlined.UploadFile,
                label = "Send files",
                state = if (sending) "Sending…" else "Photos, documents, anything",
                enabled = connected && !sending,
                onClick = { pickFiles.launch(arrayOf("*/*")) },
                modifier = Modifier.weight(1f),
            )
            Tile(
                icon = Icons.AutoMirrored.Outlined.Message,
                label = "Send text",
                state = "A note or a link",
                enabled = connected,
                onClick = { dialog = TileDialog.Text },
                modifier = Modifier.weight(1f),
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Tile(
                icon = Icons.Outlined.ContentPaste,
                label = "Clipboard",
                state = "Send what you copied",
                enabled = connected,
                onClick = { sendClipboard(context, state, scope, onMessage) },
                modifier = Modifier.weight(1f),
            )
            Tile(
                icon = Icons.Outlined.Notifications,
                label = "Notifications",
                state = if (notifications) "Showing on computer" else "Off",
                enabled = true,
                onClick = { openNotificationAccess.launch(Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS)) },
                modifier = Modifier.weight(1f),
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Tile(
                icon = Icons.Outlined.Lock,
                label = "Lock computer",
                state = "When you walk away",
                enabled = connected,
                onClick = { scope.launch { state.act(peer, ComputerAction.Lock)?.let(onMessage) } },
                modifier = Modifier.weight(1f),
            )
            Tile(
                icon = Icons.Outlined.Keyboard,
                label = "Type on computer",
                state = "Into whatever's open",
                enabled = connected,
                onClick = { dialog = TileDialog.Typing },
                modifier = Modifier.weight(1f),
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Tile(
                icon = Icons.Outlined.PushPin,
                label = "Snippets",
                state = "Pinned on every device",
                enabled = true,
                onClick = { dialog = TileDialog.Snippets },
                modifier = Modifier.weight(1f),
            )
            Tile(
                icon = Icons.Outlined.DocumentScanner,
                label = "Scan to computer",
                state = "A page, as a PDF",
                enabled = connected && !sending,
                onClick = {
                    val granted = ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA)
                    if (granted == PackageManager.PERMISSION_GRANTED) {
                        scan()
                    } else {
                        allowCamera.launch(
                            Manifest.permission.CAMERA,
                        )
                    }
                },
                modifier = Modifier.weight(1f),
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Tile(
                icon = Icons.Outlined.Bedtime,
                label = "Sleep computer",
                state = "Saves its battery",
                enabled = connected,
                onClick = { scope.launch { state.act(peer, ComputerAction.Sleep)?.let(onMessage) } },
                modifier = Modifier.weight(1f),
            )
            Tile(
                icon = Icons.Outlined.NotificationsActive,
                label = if (ringing) "Stop ringing" else "Find computer",
                state = "Rings it, even if muted",
                enabled = connected,
                onClick = {
                    val ring = !ringing
                    scope.launch { state.ringComputer(peer, ring)?.let(onMessage) ?: run { ringing = ring } }
                },
                modifier = Modifier.weight(1f),
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Tile(
                icon = Icons.Outlined.TouchApp,
                label = "Touchpad",
                state = "Move its pointer and type",
                enabled = connected,
                onClick = { dialog = TileDialog.Touchpad },
                modifier = Modifier.weight(1f),
            )
            Spacer(Modifier.weight(1f))
        }
    }
    dialog?.let { open ->
        OpenTileDialog(open, peer, state, scope, onMessage, onClose = { dialog = null })
    }
}

private enum class TileDialog { Text, Typing, Snippets, Touchpad }

/** Sends from [scope], which outlives the dialog, so closing it doesn't cancel a send. */
@Composable
private fun OpenTileDialog(
    dialog: TileDialog,
    peer: TrustedPeer,
    state: AppState,
    scope: CoroutineScope,
    onMessage: (String) -> Unit,
    onClose: () -> Unit,
) {
    when (dialog) {
        TileDialog.Touchpad -> TouchpadDialog(peer = peer, onMessage = onMessage, onDismiss = onClose)
        TileDialog.Snippets -> SnippetsDialog(state = state, onMessage = onMessage, onDismiss = onClose)
        TileDialog.Text ->
            TextDialog(
                title = "Send text",
                action = "Send",
                initial = LocalClipboardManager.current.getText()?.text.orEmpty(),
                onDismiss = onClose,
                onDone = { text ->
                    onClose()
                    scope.launch { state.sendText(peer, text)?.let(onMessage) }
                },
            )
        TileDialog.Typing ->
            TextDialog(
                title = "Type on computer",
                action = "Type",
                initial = "",
                onDismiss = onClose,
                onDone = { text ->
                    onClose()
                    scope.launch { state.act(peer, ComputerAction.TypeText(text))?.let(onMessage) }
                },
            )
    }
}

private fun sendClipboard(
    context: Context,
    state: AppState,
    scope: CoroutineScope,
    onMessage: (String) -> Unit,
) {
    val copy = readCopy(context)
    if (copy == null) {
        onMessage("There's nothing copied to send.")
        return
    }
    scope.launch {
        val (sent, message) = state.sendToConnected(context, copy)
        if (sent) markSent(context.applicationContext as ContinueApplication, copy)
        onMessage(message)
    }
}

@Composable
private fun Tile(
    icon: ImageVector,
    label: String,
    state: String,
    enabled: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier =
            modifier
                .clip(RoundedCornerShape(24.dp))
                .background(MaterialTheme.colorScheme.surfaceContainerHigh)
                .clickable(enabled = enabled, onClick = onClick)
                .alpha(if (enabled) 1f else DISABLED_ALPHA)
                .padding(18.dp),
    ) {
        Icon(icon, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
        Text(label, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(top = 20.dp))
        Text(state, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun SnippetsDialog(
    state: AppState,
    onMessage: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    val context = LocalContext.current
    val copied = LocalClipboardManager.current.getText()?.text?.takeIf { it.isNotBlank() }
    val scope = rememberCoroutineScope()
    var snippets by remember { mutableStateOf(emptyList<Snippet>()) }
    LaunchedEffect(Unit) { snippets = state.snippets() }
    AlertDialog(
        onDismissRequest = onDismiss,
        icon = { Icon(Icons.Outlined.PushPin, contentDescription = null) },
        title = { Text("Snippets") },
        text = {
            if (snippets.isEmpty()) {
                Text("Copy something, then pin it here to have it on your computer too.")
            } else {
                LazyColumn(modifier = Modifier.heightIn(max = 360.dp)) {
                    items(snippets, key = { it.id }) { snippet ->
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier =
                                Modifier.fillMaxWidth().clickable {
                                    copyToClipboard(context, snippet.text)
                                    onMessage("Copied")
                                },
                        ) {
                            Text(snippet.text, maxLines = 2, modifier = Modifier.weight(1f).padding(vertical = 12.dp))
                            IconButton(
                                onClick = {
                                    scope.launch {
                                        state.unpin(snippet.id)?.let(onMessage)
                                        snippets = state.snippets()
                                    }
                                },
                            ) { Icon(Icons.Outlined.Close, contentDescription = "Unpin") }
                        }
                    }
                }
            }
        },
        confirmButton = {
            TextButton(
                enabled = copied != null,
                onClick = {
                    scope.launch {
                        copied?.let { state.pin(it) }?.let(onMessage)
                        snippets = state.snippets()
                    }
                },
            ) { Text("Pin what you copied") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Close") } },
    )
}

@Composable
private fun TextDialog(
    title: String,
    action: String,
    initial: String,
    onDismiss: () -> Unit,
    onDone: (String) -> Unit,
) {
    var text by remember { mutableStateOf(initial) }
    AlertDialog(
        onDismissRequest = onDismiss,
        icon = { Icon(Icons.AutoMirrored.Outlined.Send, contentDescription = null) },
        title = { Text(title) },
        text = {
            OutlinedTextField(
                value = text,
                onValueChange = { text = it },
                placeholder = { Text("Type or paste text") },
                modifier = Modifier.fillMaxWidth(),
                minLines = 3,
            )
        },
        confirmButton = { TextButton(onClick = { onDone(text) }, enabled = text.isNotBlank()) { Text(action) } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
