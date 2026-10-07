package org.continueapp.android.ui.screens

import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.QrCodeScanner
import androidx.compose.material.icons.outlined.WifiOff
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.continueapp.android.AppState
import org.continueapp.android.ui.components.ActionButton
import org.continueapp.android.ui.components.ScreenPadding
import org.continueapp.android.ui.components.StatusLabel
import org.continueapp.bridge.TrustedPeer

private const val LAPTOP_SCREEN_RATIO = 16f / 10f

// Hardware stays dark in both themes.
private val LaptopBody = Color(0xFF1C1D21)
private val LaptopDeck = Color(0xFF303238)
private val ScreenOff = Color(0xFF0B0C0E)

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
