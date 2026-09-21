package org.continueapp.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import org.continueapp.android.ui.screens.DevicesScreen
import org.continueapp.android.ui.screens.PermissionsScreen
import org.continueapp.android.ui.screens.TransfersScreen
import org.continueapp.android.ui.theme.ContinueTheme
import org.continueapp.android.ui.theme.Slate800
import org.continueapp.android.ui.theme.Slate900
import org.continueapp.bridge.ContinueCoreBridge

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val bridge = (application as ContinueApplication).coreBridge

        setContent {
            ContinueTheme {
                MainAppContent(bridge = bridge)
            }
        }
    }
}

private const val TAB_DEVICES = 0
private const val TAB_TRANSFERS = 1
private const val TAB_PERMISSIONS = 2

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MainAppContent(bridge: ContinueCoreBridge) {
    var selectedTab by remember { mutableIntStateOf(TAB_DEVICES) }
    var peers by remember { mutableStateOf(bridge.listTrustedPeers()) }

    val fingerprint =
        remember {
            try {
                bridge.getDeviceFingerprint()
            } catch (_: Exception) {
                "device-fingerprint-unknown"
            }
        }

    val spkiHash =
        remember {
            try {
                bridge.getDeviceSpkiHash()
            } catch (_: Exception) {
                "device-spki-unknown"
            }
        }

    val handlePairQr: (String) -> Unit = { payload ->
        try {
            bridge.pairFromQr(payload)
            peers = bridge.listTrustedPeers()
        } catch (_: Exception) {
            // Handled gracefully in UI
        }
    }

    val handleRemovePeer: (String) -> Unit = { fp ->
        bridge.removeTrustedPeer(fp)
        peers = bridge.listTrustedPeers()
    }

    Scaffold(
        modifier = Modifier.fillMaxSize(),
        containerColor = Slate900,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = "Continue",
                        fontWeight = FontWeight.Bold,
                        fontSize = 18.sp,
                        color = Color.White,
                    )
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = Slate800),
            )
        },
        bottomBar = {
            NavigationBar(containerColor = Slate800) {
                NavigationBarItem(
                    selected = selectedTab == TAB_DEVICES,
                    onClick = { selectedTab = TAB_DEVICES },
                    label = { Text("Devices") },
                    icon = { Text("📱") },
                )
                NavigationBarItem(
                    selected = selectedTab == TAB_TRANSFERS,
                    onClick = { selectedTab = TAB_TRANSFERS },
                    label = { Text("Transfers") },
                    icon = { Text("📁") },
                )
                NavigationBarItem(
                    selected = selectedTab == TAB_PERMISSIONS,
                    onClick = { selectedTab = TAB_PERMISSIONS },
                    label = { Text("Permissions") },
                    icon = { Text("🔒") },
                )
            }
        },
    ) { innerPadding ->
        val screenModifier = Modifier.padding(innerPadding)
        when (selectedTab) {
            TAB_DEVICES ->
                DevicesScreen(
                    deviceFingerprint = fingerprint,
                    deviceSpkiHash = spkiHash,
                    peers = peers,
                    onPairQr = handlePairQr,
                    onRemovePeer = handleRemovePeer,
                    modifier = screenModifier,
                )
            TAB_TRANSFERS -> TransfersScreen(modifier = screenModifier)
            TAB_PERMISSIONS -> PermissionsScreen(modifier = screenModifier)
        }
    }
}
