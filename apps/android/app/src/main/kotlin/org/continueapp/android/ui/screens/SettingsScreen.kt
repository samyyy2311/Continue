package org.continueapp.android.ui.screens

import android.Manifest
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.DocumentsContract
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Code
import androidx.compose.material.icons.outlined.ContentPaste
import androidx.compose.material.icons.outlined.DarkMode
import androidx.compose.material.icons.outlined.Folder
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Palette
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import org.continueapp.android.ContinueApplication
import org.continueapp.android.needsNotificationPermission
import org.continueapp.android.ui.components.ChoiceRow
import org.continueapp.android.ui.components.PageTitle
import org.continueapp.android.ui.components.ScreenPadding
import org.continueapp.android.ui.components.SectionLabel
import org.continueapp.android.ui.components.SettingsRow
import org.continueapp.android.ui.theme.ThemeMode

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
