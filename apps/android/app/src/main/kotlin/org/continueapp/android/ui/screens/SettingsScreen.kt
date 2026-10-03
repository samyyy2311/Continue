package org.continueapp.android.ui.screens

import android.Manifest
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Code
import androidx.compose.material.icons.outlined.DarkMode
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Palette
import androidx.compose.material.icons.outlined.Sync
import androidx.compose.material.icons.outlined.Wifi
import androidx.compose.material3.Switch
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
