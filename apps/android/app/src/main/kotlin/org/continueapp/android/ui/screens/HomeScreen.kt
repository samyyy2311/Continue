package org.continueapp.android.ui.screens

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.BitmapFactory
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.material.icons.outlined.QrCodeScanner
import androidx.compose.material.icons.outlined.TouchApp
import androidx.compose.material.icons.outlined.UploadFile
import androidx.compose.material.icons.outlined.WifiOff
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.FilterChip
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
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
import org.continueapp.android.ui.components.ActionButton
import org.continueapp.android.ui.components.ScreenPadding
import org.continueapp.android.ui.components.StatusLabel
import org.continueapp.bridge.ComputerAction
import org.continueapp.bridge.Snippet
import org.continueapp.bridge.TrustedPeer

private const val LAPTOP_SCREEN_RATIO = 16f / 10f

// Hardware stays dark in both themes.
private val LaptopBody = Color(0xFF1C1D21)
private val LaptopDeck = Color(0xFF303238)
private val ScreenOff = Color(0xFF0B0C0E)
private const val DISABLED_ALPHA = 0.45f

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

    Column(
        modifier = modifier.verticalScroll(rememberScrollState()).padding(horizontal = ScreenPadding),
        verticalArrangement = Arrangement.spacedBy(24.dp),
    ) {
        if (!visible) HiddenLine(onClick = onOpenSettings)
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
        val connected = peer.fingerprint in state.connected
        ComputerHero(peer = peer, connected = connected, wallpaper = state.wallpapers[peer.fingerprint])
        Tiles(peer = peer, connected = connected, state = state, onMessage = onMessage)
        ConnectionAction(peer = peer, connected = connected, state = state, onMessage = onMessage)
        RecentSection(state.recent, state.incoming, onMessage)
    }
}

@Composable
private fun HiddenLine(onClick: () -> Unit) {
    Row(
        modifier = Modifier.padding(top = 16.dp).clip(RoundedCornerShape(12.dp)).clickable(onClick = onClick),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Icon(
            Icons.Outlined.WifiOff,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.size(18.dp),
        )
        Text(
            "Hidden from other devices",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun NoDevices(onPair: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(top = 48.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        LaptopFrame(wallpaper = null, active = false)
        Text("Pair with your computer", style = MaterialTheme.typography.headlineSmall)
        Text(
            "Open Continue on your computer, click Pair a device, and scan the code it shows.",
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
private fun ComputerHero(
    peer: TrustedPeer,
    connected: Boolean,
    wallpaper: ByteArray?,
) {
    val picture =
        remember(wallpaper) {
            wallpaper?.let { BitmapFactory.decodeByteArray(it, 0, it.size)?.asImageBitmap() }
        }
    Column(
        modifier = Modifier.fillMaxWidth().padding(top = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        LaptopFrame(wallpaper = picture, active = connected)
        Text(peer.displayName, style = MaterialTheme.typography.headlineSmall, modifier = Modifier.padding(top = 12.dp))
        StatusLabel(connected = connected)
    }
}

/** Shows the computer's wallpaper on screen while connected. */
@Composable
private fun LaptopFrame(
    wallpaper: ImageBitmap?,
    active: Boolean,
) {
    val screen = if (active) MaterialTheme.colorScheme.primary.copy(alpha = 0.35f) else ScreenOff
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Box(
            modifier =
                Modifier
                    .width(232.dp)
                    .clip(RoundedCornerShape(topStart = 14.dp, topEnd = 14.dp, bottomStart = 4.dp, bottomEnd = 4.dp))
                    .background(LaptopBody)
                    .padding(start = 7.dp, end = 7.dp, top = 12.dp, bottom = 7.dp),
            contentAlignment = Alignment.TopCenter,
        ) {
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .aspectRatio(LAPTOP_SCREEN_RATIO)
                        .clip(RoundedCornerShape(4.dp))
                        .background(screen),
            ) {
                if (active && wallpaper != null) {
                    Image(
                        wallpaper,
                        contentDescription = null,
                        contentScale = ContentScale.Crop,
                        modifier = Modifier.fillMaxSize(),
                    )
                }
            }
            Box(modifier = Modifier.offset(y = (-8).dp).size(4.dp).clip(CircleShape).background(ScreenOff))
        }
        Box(
            modifier =
                Modifier
                    .width(268.dp)
                    .height(10.dp)
                    .clip(RoundedCornerShape(topStart = 2.dp, topEnd = 2.dp, bottomStart = 10.dp, bottomEnd = 10.dp))
                    .background(LaptopDeck),
            contentAlignment = Alignment.TopCenter,
        ) {
            Box(
                modifier =
                    Modifier
                        .width(48.dp)
                        .height(4.dp)
                        .clip(RoundedCornerShape(bottomStart = 4.dp, bottomEnd = 4.dp))
                        .background(LaptopBody),
            )
        }
    }
}

@Composable
private fun Tiles(
    peer: TrustedPeer,
    connected: Boolean,
    state: AppState,
    onMessage: (String) -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var sending by remember { mutableStateOf(false) }
    var composingText by remember { mutableStateOf(false) }
    var typing by remember { mutableStateOf(false) }
    var pinned by remember { mutableStateOf(false) }
    var ringing by remember { mutableStateOf(false) }
    var touchpad by remember { mutableStateOf(false) }

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
    val sendClipboard: () -> Unit = {
        val copy = readCopy(context)
        if (copy == null) {
            onMessage("There's nothing copied to send.")
        } else {
            scope.launch {
                val (sent, message) = state.sendToConnected(context, copy)
                if (sent) markSent(context.applicationContext as ContinueApplication, copy)
                onMessage(message)
            }
        }
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
                onClick = { composingText = true },
                modifier = Modifier.weight(1f),
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Tile(
                icon = Icons.Outlined.ContentPaste,
                label = "Clipboard",
                state = "Send what you copied",
                enabled = connected,
                onClick = sendClipboard,
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
                onClick = { typing = true },
                modifier = Modifier.weight(1f),
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Tile(
                icon = Icons.Outlined.PushPin,
                label = "Snippets",
                state = "Pinned on every device",
                enabled = true,
                onClick = { pinned = true },
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
                onClick = { touchpad = true },
                modifier = Modifier.weight(1f),
            )
            Spacer(Modifier.weight(1f))
        }
    }
    if (touchpad) {
        TouchpadDialog(peer = peer, onMessage = onMessage, onDismiss = { touchpad = false })
    }
    if (pinned) {
        SnippetsDialog(state = state, onMessage = onMessage, onDismiss = { pinned = false })
    }
    if (composingText) {
        TextDialog(
            title = "Send text",
            action = "Send",
            initial = LocalClipboardManager.current.getText()?.text.orEmpty(),
            onDismiss = { composingText = false },
            onDone = { text ->
                composingText = false
                scope.launch { state.sendText(peer, text)?.let(onMessage) }
            },
        )
    }
    if (typing) {
        TextDialog(
            title = "Type on computer",
            action = "Type",
            initial = "",
            onDismiss = { typing = false },
            onDone = { text ->
                typing = false
                scope.launch { state.act(peer, ComputerAction.TypeText(text))?.let(onMessage) }
            },
        )
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
private fun ConnectionAction(
    peer: TrustedPeer,
    connected: Boolean,
    state: AppState,
    onMessage: (String) -> Unit,
) {
    val scope = rememberCoroutineScope()
    Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
        if (connected) {
            TextButton(onClick = { scope.launch { state.disconnect(peer.fingerprint)?.let(onMessage) } }) {
                Text("Disconnect", color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        } else {
            ActionButton(onClick = { scope.launch { state.reconnect(peer.fingerprint)?.let(onMessage) } }) {
                Text("Connect")
            }
        }
    }
}

/** Tap one to copy it; pin what's copied to add it on every device. */
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
