package org.continueapp.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Devices
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.outlined.Devices
import androidx.compose.material.icons.outlined.Home
import androidx.compose.material.icons.outlined.QrCodeScanner
import androidx.compose.material.icons.outlined.Settings
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.ShortNavigationBar
import androidx.compose.material3.ShortNavigationBarItem
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.continueapp.android.ui.screens.AppearanceSettings
import org.continueapp.android.ui.screens.DeviceScreen
import org.continueapp.android.ui.screens.DevicesScreen
import org.continueapp.android.ui.screens.HomeScreen
import org.continueapp.android.ui.screens.PairScreen
import org.continueapp.android.ui.screens.PermissionPrompts
import org.continueapp.android.ui.screens.SettingsScreen
import org.continueapp.android.ui.theme.ContinueTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val app = application as ContinueApplication
        val state = AppState(app.coreBridge)

        setContent {
            var themeMode by remember { mutableStateOf(app.themeMode) }
            var wallpaperColors by remember { mutableStateOf(app.wallpaperColors) }
            var visible by remember { mutableStateOf(app.visible) }
            val appearance =
                AppearanceSettings(
                    themeMode = themeMode,
                    wallpaperColors = wallpaperColors,
                    onThemeModeChange = {
                        themeMode = it
                        app.themeMode = it
                    },
                    onWallpaperColorsChange = {
                        wallpaperColors = it
                        app.wallpaperColors = it
                    },
                )
            ContinueTheme(mode = themeMode, wallpaperColors = wallpaperColors) {
                ContinueApp(
                    state = state,
                    visible = visible,
                    onVisibleChange = {
                        visible = it
                        app.visible = it
                    },
                    appearance = appearance,
                )
            }
        }
    }
}

private enum class Tab(val label: String, val selectedIcon: ImageVector, val icon: ImageVector) {
    Home("Home", Icons.Filled.Home, Icons.Outlined.Home),
    Devices("Devices", Icons.Filled.Devices, Icons.Outlined.Devices),
    Settings("Settings", Icons.Filled.Settings, Icons.Outlined.Settings),
}

/** A screen shown over the tabs, closed with back. */
private sealed interface Overlay {
    data object Pair : Overlay

    data class Device(val fingerprint: String) : Overlay
}

private const val REFRESH_INTERVAL_MS = 2_000L

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun ContinueApp(
    state: AppState,
    visible: Boolean,
    onVisibleChange: (Boolean) -> Unit,
    appearance: AppearanceSettings,
) {
    var tab by remember { mutableStateOf(Tab.Home) }
    var overlay by remember { mutableStateOf<Overlay?>(null) }
    val snackbar = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()
    val showMessage: (String) -> Unit = { message -> scope.launch { snackbar.showSnackbar(message) } }

    // The core has no connection events yet, so connection state is read on a short interval.
    LaunchedEffect(Unit) {
        while (true) {
            state.refresh()
            delay(REFRESH_INTERVAL_MS)
        }
    }
    PermissionPrompts(state.questions)
    val context = LocalContext.current
    LaunchedEffect(Unit) {
        state.recent.load()
        state.incoming.listen(context.applicationContext)
    }
    BackHandler(enabled = overlay != null) { overlay = null }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        snackbarHost = { SnackbarHost(snackbar) },
        floatingActionButton = {
            if (overlay == null && tab != Tab.Settings) {
                ExtendedFloatingActionButton(
                    onClick = { overlay = Overlay.Pair },
                    icon = { Icon(Icons.Outlined.QrCodeScanner, contentDescription = null) },
                    text = { Text("Pair") },
                )
            }
        },
        bottomBar = {
            if (overlay == null) {
                ShortNavigationBar {
                    Tab.entries.forEach { item ->
                        ShortNavigationBarItem(
                            selected = tab == item,
                            onClick = { tab = item },
                            icon = {
                                Icon(if (tab == item) item.selectedIcon else item.icon, contentDescription = null)
                            },
                            label = { Text(item.label) },
                        )
                    }
                }
            }
        },
    ) { padding ->
        Box(modifier = Modifier.fillMaxSize().padding(padding), contentAlignment = Alignment.TopCenter) {
            val content = Modifier.fillMaxSize().widthIn(max = 840.dp)
            when (val current = overlay) {
                Overlay.Pair ->
                    PairScreen(onPair = state::pair, onDone = { overlay = null }, modifier = content)
                is Overlay.Device -> {
                    val peer = state.peers.firstOrNull { it.fingerprint == current.fingerprint }
                    if (peer == null) {
                        // The device was forgotten, possibly from the other side.
                        LaunchedEffect(Unit) { overlay = null }
                    } else {
                        DeviceScreen(
                            state = state,
                            peer = peer,
                            onBack = { overlay = null },
                            onMessage = showMessage,
                            modifier = content,
                        )
                    }
                }
                null ->
                    when (tab) {
                        Tab.Home ->
                            HomeScreen(
                                state = state,
                                visible = visible,
                                onPair = { overlay = Overlay.Pair },
                                onOpenSettings = { tab = Tab.Settings },
                                onMessage = showMessage,
                                modifier = content,
                            )
                        Tab.Devices ->
                            DevicesScreen(
                                state = state,
                                onOpenDevice = { overlay = Overlay.Device(it) },
                                modifier = content,
                            )
                        Tab.Settings ->
                            SettingsScreen(
                                visible = visible,
                                onVisibleChange = onVisibleChange,
                                appearance = appearance,
                                modifier = content,
                            )
                    }
            }
        }
    }
}
