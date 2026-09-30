package org.continueapp.android.ui.screens

import android.content.Intent
import android.net.Uri
import android.os.Build
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Code
import androidx.compose.material.icons.outlined.DarkMode
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Key
import androidx.compose.material.icons.outlined.Palette
import androidx.compose.material.icons.outlined.PhoneAndroid
import androidx.compose.material.icons.outlined.Wifi
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
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

@Composable
fun SettingsScreen(
    deviceKey: String,
    visible: Boolean,
    onVisibleChange: (Boolean) -> Unit,
    appearance: AppearanceSettings,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val version = context.packageManager.getPackageInfo(context.packageName, 0).versionName.orEmpty()

    Column(modifier = modifier.verticalScroll(rememberScrollState()).padding(horizontal = ScreenPadding)) {
        PageTitle("Settings")

        SectionLabel("This phone")
        SettingsRow(
            title = "Model",
            subtitle = "${Build.MANUFACTURER} ${Build.MODEL}",
            icon = Icons.Outlined.PhoneAndroid,
        )
        SettingsRow(title = "Device key", subtitle = deviceKey, icon = Icons.Outlined.Key)

        SectionLabel("Connectivity")
        SettingsRow(
            title = "Visible on this network",
            subtitle = "Lets your paired computer find this phone on the same Wi-Fi or hotspot.",
            icon = Icons.Outlined.Wifi,
            trailing = { Switch(checked = visible, onCheckedChange = onVisibleChange) },
        )

        SectionLabel("Appearance")
        SettingsRow(title = "Theme", icon = Icons.Outlined.DarkMode)
        Row(
            modifier = Modifier.padding(start = 40.dp, bottom = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            ThemeMode.entries.forEach { mode ->
                FilterChip(
                    selected = appearance.themeMode == mode,
                    onClick = { appearance.onThemeModeChange(mode) },
                    label = { Text(mode.name) },
                )
            }
        }
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
            onClick = { context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(SOURCE_URL))) },
        )
    }
}
