package org.continueapp.android

import android.Manifest
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.runtime.MutableState
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
import org.continueapp.android.ui.screens.AppearanceSettings
import org.continueapp.android.ui.screens.DeviceScreen
import org.continueapp.android.ui.screens.DevicesScreen
import org.continueapp.android.ui.screens.HomeScreen
import org.continueapp.android.ui.screens.PairScreen
import org.continueapp.android.ui.screens.PermissionPrompts
import org.continueapp.android.ui.screens.SendNewCopies
import org.continueapp.android.ui.screens.SettingsScreen
import org.continueapp.android.ui.screens.SharePrompt
import org.continueapp.android.ui.theme.ContinueTheme

class MainActivity : ComponentActivity() {
    private val askForNotifications = registerForActivityResult(ActivityResultContracts.RequestPermission()) {}

    /**
     * Initializes the activity, restores eligible incoming share data, and sets up the app UI.
     */
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val app = application as ContinueApplication
        // A share still waiting after the phone is turned is kept by the app; reopening from
        // recent apps shouldn't ask again about one already answered.
        val fromRecents = intent.flags and Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY != 0
        if (savedInstanceState == null && !fromRecents) sharedFrom(intent)?.let { app.pendingShare.value = it }
        val state = app.state
        app.applyBackground()
        // Asked once here; after that, only when background receiving is turned on in Settings.
        if (app.receiveInBackground && !app.askedForNotifications && needsNotificationPermission(this)) {
            app.askedForNotifications = true
            askForNotifications.launch(Manifest.permission.POST_NOTIFICATIONS)
        }

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
                    shared = app.pendingShare,
                )
            }
        }
    }

    /**
     * Updates the pending share when the incoming intent contains shared content.
     */
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        sharedFrom(intent)?.let { (application as ContinueApplication).pendingShare.value = it }
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

/**
 * Displays the app's selected tab or active overlay and handles navigation between them.
 *
 * @param shared State containing a pending incoming share, if any.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun ContinueApp(
    state: AppState,
    visible: Boolean,
    onVisibleChange: (Boolean) -> Unit,
    appearance: AppearanceSettings,
    shared: MutableState<Shared?>,
) {
    var tab by remember { mutableStateOf(Tab.Home) }
    var overlay by remember { mutableStateOf<Overlay?>(null) }
    val snackbar = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()
    val showMessage: (String) -> Unit = { message -> scope.launch { snackbar.showSnackbar(message) } }

    // The core has no connection events yet, so connection state is read on a short interval.
    PermissionPrompts(state.questions)
    SendNewCopies(state, onMessage = showMessage)
    BackHandler(enabled = overlay != null) { overlay = null }
    SharePrompt(shared, state, onMessage = showMessage) {
        overlay = null
        tab = Tab.Home
    }

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
