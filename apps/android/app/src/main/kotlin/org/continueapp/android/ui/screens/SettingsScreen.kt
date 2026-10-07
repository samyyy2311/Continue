package org.continueapp.android.ui.screens

import android.Manifest
import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.DocumentsContract
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Bluetooth
import androidx.compose.material.icons.outlined.Call
import androidx.compose.material.icons.outlined.Code
import androidx.compose.material.icons.outlined.ContentPaste
import androidx.compose.material.icons.outlined.DarkMode
import androidx.compose.material.icons.outlined.Folder
import androidx.compose.material.icons.outlined.FolderOpen
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material.icons.outlined.NotificationsOff
import androidx.compose.material.icons.outlined.Palette
import androidx.compose.material.icons.outlined.PauseCircle
import androidx.compose.material.icons.outlined.PhotoLibrary
import androidx.compose.material.icons.outlined.ScreenShare
import androidx.compose.material.icons.outlined.Sms
import androidx.compose.material.icons.outlined.Sync
import androidx.compose.material.icons.outlined.Wifi
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import org.continueapp.android.ContinueApplication
import org.continueapp.android.ControlService
import org.continueapp.android.appLabel
import org.continueapp.android.canAdvertise
import org.continueapp.android.hasNotificationAccess
import org.continueapp.android.needsNotificationPermission
import org.continueapp.android.ui.components.ChoiceRow
import org.continueapp.android.ui.components.PageTitle
import org.continueapp.android.ui.components.ScreenPadding
import org.continueapp.android.ui.components.SectionLabel
import org.continueapp.android.ui.components.SettingsRow
import org.continueapp.android.ui.theme.ThemeMode
import org.continueapp.bridge.CALLS_PERMISSIONS
import org.continueapp.bridge.MESSAGES_PERMISSIONS
import org.continueapp.bridge.PHOTOS_PERMISSION
import org.continueapp.bridge.canReadFiles
import org.continueapp.bridge.canReadMessages
import org.continueapp.bridge.canReadPhotos
import org.continueapp.bridge.canSeeCalls

private const val SOURCE_URL = "https://github.com/samyyy2311/Continue"

class AppearanceSettings(
    val themeMode: ThemeMode,
    val wallpaperColors: Boolean,
    val onThemeModeChange: (ThemeMode) -> Unit,
    val onWallpaperColorsChange: (Boolean) -> Unit,
)

/** Where received files go: a folder picked by the user, or Downloads/Continue. */
@Composable
private fun SaveFolderRow() {
    val context = LocalContext.current
    val app = context.applicationContext as ContinueApplication
    var folder by remember { mutableStateOf(app.saveFolder) }
    val access = Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
    val change = { picked: Uri? ->
        // Keeps access to the new folder across restarts, and lets go of the old one. A folder
        // that won't allow lasting access isn't used, so files keep going to Downloads.
        val kept =
            picked?.takeIf {
                runCatching { context.contentResolver.takePersistableUriPermission(it, access) }.isSuccess
            }
        folder?.takeIf { it != kept }?.let { old ->
            runCatching { context.contentResolver.releasePersistableUriPermission(old, access) }
        }
        app.saveFolder = kept
        folder = kept
    }
    val pick = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { it?.let(change) }
    SettingsRow(
        title = "Save files to",
        subtitle = folder?.let(::folderName) ?: "Downloads/Continue",
        icon = Icons.Outlined.Folder,
        onClick = { pick.launch(folder) },
        trailing =
            folder?.let {
                { TextButton(onClick = { change(null) }) { Text("Use Downloads") } }
            },
    )
}

/** "Documents/Phone" from a picked folder, as people know it rather than as a link. */
private fun folderName(tree: Uri): String {
    val path = DocumentsContract.getTreeDocumentId(tree).substringAfter(':')
    return path.ifBlank { "Picked folder" }
}

@Composable
private fun PauseRow() {
    val app = LocalContext.current.applicationContext as ContinueApplication
    var on by remember { mutableStateOf(app.paused) }
    SettingsRow(
        title = "Pause connections",
        subtitle = "Computers are disconnected and can't connect until you turn this off.",
        icon = Icons.Outlined.PauseCircle,
        trailing = {
            Switch(
                checked = on,
                onCheckedChange = {
                    on = it
                    app.paused = it
                },
            )
        },
    )
}

/** Lets a computer lock itself when the phone moves away. */
@Composable
private fun PresenceRow() {
    val context = LocalContext.current
    val app = context.applicationContext as ContinueApplication
    var on by remember { mutableStateOf(app.announcesPresence && canAdvertise(context)) }
    val turnOn = {
        on = true
        app.announcesPresence = true
    }
    val allow =
        rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            if (granted) turnOn()
        }
    SettingsRow(
        title = "Let your computer see you're nearby",
        subtitle =
            "Sends a signal over Bluetooth that only your paired computers recognise, " +
                "so one can lock itself when you walk away.",
        icon = Icons.Outlined.Bluetooth,
        trailing = {
            Switch(
                checked = on,
                onCheckedChange = { wanted ->
                    when {
                        !wanted -> {
                            on = false
                            app.announcesPresence = false
                        }
                        canAdvertise(context) -> turnOn()
                        Build.VERSION.SDK_INT >= Build.VERSION_CODES.S ->
                            allow.launch(Manifest.permission.BLUETOOTH_ADVERTISE)
                    }
                },
            )
        },
    )
}

/** Sends anything newly copied when the app opens. */
@Composable
private fun SendCopiesRow() {
    val app = LocalContext.current.applicationContext as ContinueApplication
    var on by remember { mutableStateOf(app.sendNewCopies) }
    SettingsRow(
        title = "Send what you copy",
        subtitle =
            "When you open Continue, anything you copied since goes to your computer. " +
                "For one tap from anywhere, add the Send clipboard tile to Quick Settings.",
        icon = Icons.Outlined.ContentPaste,
        trailing = {
            Switch(
                checked = on,
                onCheckedChange = {
                    on = it
                    app.sendNewCopies = it
                },
            )
        },
    )
}

/** Keeps receiving with the app closed. Turning it on asks to show notifications if needed. */
@Composable
private fun BackgroundRow() {
    val context = LocalContext.current
    val app = context.applicationContext as ContinueApplication
    var on by remember { mutableStateOf(app.receiveInBackground) }
    val askForNotifications = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {}
    SettingsRow(
        title = "Keep receiving when closed",
        subtitle =
            "Files and text from your computer arrive even with Continue closed. " +
                "A quiet notification stays while this is on.",
        icon = Icons.Outlined.Sync,
        trailing = {
            Switch(
                checked = on,
                onCheckedChange = {
                    on = it
                    app.receiveInBackground = it
                    if (it && needsNotificationPermission(context)) {
                        askForNotifications.launch(Manifest.permission.POST_NOTIFICATIONS)
                    }
                },
            )
        },
    )
}

/** Toggle for access Android grants either through a prompt or on its own settings page. */
@Composable
private fun AccessRow(
    title: String,
    subtitle: String,
    icon: ImageVector,
    granted: (Context) -> Boolean,
    permissions: Array<String> = emptyArray(),
    settings: Intent? = null,
) {
    val context = LocalContext.current
    var on by remember { mutableStateOf(granted(context)) }
    val update = {
        on = granted(context)
        (context.applicationContext as ContinueApplication).watchPhone()
    }
    val ask = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { update() }
    val openSettings = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { update() }
    val toggle = {
        when {
            settings != null -> openSettings.launch(settings)
            on -> {
                val details = Uri.fromParts("package", context.packageName, null)
                openSettings.launch(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, details))
            }
            else -> ask.launch(permissions)
        }
    }
    SettingsRow(
        title = title,
        subtitle = subtitle,
        icon = icon,
        onClick = toggle,
        trailing = { Switch(checked = on, onCheckedChange = { toggle() }) },
    )
}

/** Apps muted from a computer, each with a way to show it again. */
@Composable
private fun MutedAppsRows() {
    val context = LocalContext.current
    val app = context.applicationContext as ContinueApplication
    var muted by remember { mutableStateOf(app.mutedApps.sorted()) }
    muted.forEach { packageName ->
        SettingsRow(
            title = appLabel(context, packageName),
            subtitle = "Not shown on your computer",
            icon = Icons.Outlined.NotificationsOff,
            trailing = {
                TextButton(
                    onClick = {
                        app.mutedApps -= packageName
                        muted = muted - packageName
                    },
                ) { Text("Unmute") }
            },
        )
    }
}

@Composable
fun SettingsScreen(
    visible: Boolean,
    onVisibleChange: (Boolean) -> Unit,
    appearance: AppearanceSettings,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val version = context.packageManager.getPackageInfo(context.packageName, 0).versionName.orEmpty()

    Column(modifier = modifier.verticalScroll(rememberScrollState()).padding(horizontal = ScreenPadding)) {
        PageTitle("Settings")

        SectionLabel("Connectivity")
        SettingsRow(
            title = "Visible on this network",
            subtitle = "Lets your paired computer find this phone on the same Wi-Fi or hotspot.",
            icon = Icons.Outlined.Wifi,
            trailing = { Switch(checked = visible, onCheckedChange = onVisibleChange) },
        )
        BackgroundRow()
        AccessRow(
            title = "Show notifications on your computer",
            subtitle = "Read and reply to them from your computer. Turn on Continue in the list that opens.",
            icon = Icons.Outlined.Notifications,
            granted = ::hasNotificationAccess,
            settings = Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS),
        )
        MutedAppsRows()
        AccessRow(
            title = "Show photos on your computer",
            subtitle = "Browse your photos from your computer, and see new ones as you take them.",
            icon = Icons.Outlined.PhotoLibrary,
            permissions = arrayOf(PHOTOS_PERMISSION),
            granted = ::canReadPhotos,
        )
        AccessRow(
            title = "Text from your computer",
            subtitle = "Read your texts and reply from your computer.",
            icon = Icons.Outlined.Sms,
            permissions = MESSAGES_PERMISSIONS,
            granted = ::canReadMessages,
        )
        AccessRow(
            title = "Calls on your computer",
            subtitle = "See who's calling and answer or decline from your computer.",
            icon = Icons.Outlined.Call,
            permissions = CALLS_PERMISSIONS,
            granted = ::canSeeCalls,
        )
        AccessRow(
            title = "Control this phone from your computer",
            subtitle =
                "Tap, swipe and go back or home from the screen's window on your computer. " +
                    "Turn on Continue in the list that opens.",
            icon = Icons.Outlined.ScreenShare,
            granted = ControlService::isOn,
            settings = Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS),
        )
        AccessRow(
            title = "Browse phone files from your computer",
            subtitle = "See your phone's folders on your computer and copy files from them.",
            icon = Icons.Outlined.FolderOpen,
            granted = ::canReadFiles,
            permissions = arrayOf(Manifest.permission.READ_EXTERNAL_STORAGE),
            settings =
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    val app = Uri.fromParts("package", context.packageName, null)
                    Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION, app)
                } else {
                    null
                },
        )
        PauseRow()
        PresenceRow()
        SendCopiesRow()

        SectionLabel("Received files")
        SaveFolderRow()

        SectionLabel("Appearance")
        SettingsRow(title = "Theme", icon = Icons.Outlined.DarkMode)
        ChoiceRow(
            choices = ThemeMode.entries.map { it to it.name },
            selected = appearance.themeMode,
            onSelect = appearance.onThemeModeChange,
            modifier = Modifier.padding(start = 40.dp, bottom = 8.dp),
        )
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            SettingsRow(
                title = "Match wallpaper colours",
                subtitle = "Use colours from your wallpaper instead of Continue's own.",
                icon = Icons.Outlined.Palette,
                trailing = {
                    Switch(checked = appearance.wallpaperColors, onCheckedChange = appearance.onWallpaperColorsChange)
                },
            )
        }

        SectionLabel("About")
        SettingsRow(title = "Version", subtitle = version, icon = Icons.Outlined.Info)
        SettingsRow(
            title = "Open source",
            subtitle = "No accounts, no cloud. Read the code on GitHub.",
            icon = Icons.Outlined.Code,
            onClick = {
                // Nothing to open it with, e.g. no browser in a work profile.
                try {
                    context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(SOURCE_URL)))
                } catch (_: ActivityNotFoundException) {
                }
            },
        )
    }
}
