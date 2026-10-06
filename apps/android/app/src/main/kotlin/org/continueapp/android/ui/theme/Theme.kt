@file:Suppress("MagicNumber")

package org.continueapp.android.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialExpressiveTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.ExperimentalTextApi
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.continueapp.android.R

enum class ThemeMode { System, Light, Dark }

// The same palette as the desktop app: pure black or white behind everything, one accent, and
// greys for the few filled controls. Every text colour meets WCAG AA on what it sits on.
private val DarkColors =
    darkColorScheme(
        primary = Color(0xFFA8BFFF),
        onPrimary = Color(0xFF0B1B45),
        primaryContainer = Color(0xFF1E2638),
        onPrimaryContainer = Color(0xFFCBD9FF),
        secondaryContainer = Color(0xFF1F2024),
        onSecondaryContainer = Color(0xFFF1F2F4),
        background = Color.Black,
        onBackground = Color(0xFFF1F2F4),
        surface = Color.Black,
        onSurface = Color(0xFFF1F2F4),
        onSurfaceVariant = Color(0xFF9CA1A9),
        surfaceContainerLowest = Color.Black,
        surfaceContainerLow = Color.Black,
        surfaceContainer = Color.Black,
        surfaceContainerHigh = Color(0xFF16171A),
        surfaceContainerHighest = Color(0xFF1F2024),
        outline = Color(0xFF4A5057),
        outlineVariant = Color(0xFF222428),
        error = Color(0xFFF2877E),
    )

private val LightColors =
    lightColorScheme(
        primary = Color(0xFF2457D6),
        onPrimary = Color.White,
        primaryContainer = Color(0xFFE3EAFB),
        onPrimaryContainer = Color(0xFF17398B),
        secondaryContainer = Color(0xFFF1F2F4),
        onSecondaryContainer = Color(0xFF111418),
        background = Color.White,
        onBackground = Color(0xFF111418),
        surface = Color.White,
        onSurface = Color(0xFF111418),
        onSurfaceVariant = Color(0xFF5A616B),
        surfaceContainerLowest = Color.White,
        surfaceContainerLow = Color.White,
        surfaceContainer = Color.White,
        surfaceContainerHigh = Color(0xFFF1F2F4),
        surfaceContainerHighest = Color(0xFFE7E9EC),
        outline = Color(0xFF8A94A4),
        outlineVariant = Color(0xFFE4E6EA),
        error = Color(0xFFB3261E),
    )

/** Green for "connected", which Material's colour scheme has no slot for. Same values as desktop. */
val ColorScheme.success: Color
    get() = if (background.luminance() > 0.5f) Color(0xFF1E7A45) else Color(0xFF6CC795)

@OptIn(ExperimentalTextApi::class)
private fun instrumentSans(weight: Int) =
    Font(
        R.font.instrument_sans,
        FontWeight(weight),
        variationSettings = FontVariation.Settings(FontVariation.weight(weight)),
    )

private val InstrumentSans = FontFamily(instrumentSans(400), instrumentSans(500), instrumentSans(600))

private fun style(
    size: Int,
    lineHeight: Int,
    weight: FontWeight = FontWeight.Normal,
    tracking: Double = 0.0,
) = TextStyle(
    fontFamily = InstrumentSans,
    fontSize = size.sp,
    lineHeight = lineHeight.sp,
    fontWeight = weight,
    letterSpacing = (size * tracking).sp,
)

private val AppTypography =
    Typography(
        headlineLarge = style(32, 38, FontWeight.SemiBold, -0.02),
        headlineSmall = style(22, 28, FontWeight.SemiBold, -0.01),
        titleLarge = style(20, 26, FontWeight.SemiBold),
        titleMedium = style(17, 22, FontWeight.Medium),
        bodyLarge = style(16, 23),
        bodyMedium = style(14, 20),
        labelLarge = style(15, 20, FontWeight.Medium),
        labelMedium = style(13, 18, FontWeight.Medium),
        labelSmall = style(12, 16, FontWeight.Medium),
    )

private val AppShapes =
    Shapes(
        small = RoundedCornerShape(10.dp),
        medium = RoundedCornerShape(14.dp),
        large = RoundedCornerShape(20.dp),
        extraLarge = RoundedCornerShape(24.dp),
    )

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ContinueTheme(
    mode: ThemeMode = ThemeMode.System,
    wallpaperColors: Boolean = false,
    content: @Composable () -> Unit,
) {
    val dark =
        when (mode) {
            ThemeMode.System -> isSystemInDarkTheme()
            ThemeMode.Light -> false
            ThemeMode.Dark -> true
        }
    val context = LocalContext.current
    val colors =
        when {
            wallpaperColors && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S ->
                if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
            dark -> DarkColors
            else -> LightColors
        }
    MaterialExpressiveTheme(
        colorScheme = colors,
        shapes = AppShapes,
        typography = AppTypography,
        content = content,
    )
}
