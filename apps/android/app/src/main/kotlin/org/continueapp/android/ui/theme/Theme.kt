@file:Suppress("MagicNumber")

package org.continueapp.android.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import org.continueapp.android.R

enum class ThemeMode { System, Light, Dark }

// Every foreground/background pair here meets WCAG AA contrast.
private val DarkColors =
    darkColorScheme(
        primary = Color(0xFF4CC2BD),
        onPrimary = Color(0xFF0E2A2A),
        primaryContainer = Color(0xFF1D4A4A),
        onPrimaryContainer = Color(0xFFBFEDEA),
        secondaryContainer = Color(0xFF2B3A4A),
        onSecondaryContainer = Color(0xFFD6E2F0),
        tertiary = Color(0xFFF2B544),
        background = Color(0xFF1B2130),
        onBackground = Color(0xFFE8ECF3),
        surface = Color(0xFF1B2130),
        onSurface = Color(0xFFE8ECF3),
        onSurfaceVariant = Color(0xFF8C97A8),
        surfaceContainer = Color(0xFF222A3B),
        surfaceContainerHigh = Color(0xFF2A3345),
        outline = Color(0xFF4A5568),
        outlineVariant = Color(0xFF2E3748),
        error = Color(0xFFFF8A80),
    )

private val LightColors =
    lightColorScheme(
        primary = Color(0xFF156E6D),
        onPrimary = Color.White,
        primaryContainer = Color(0xFFCDEDEA),
        onPrimaryContainer = Color(0xFF0B3534),
        secondaryContainer = Color(0xFFD8E3EE),
        onSecondaryContainer = Color(0xFF16202B),
        tertiary = Color(0xFF9A6200),
        background = Color(0xFFEEF2F6),
        onBackground = Color(0xFF16202B),
        surface = Color(0xFFEEF2F6),
        onSurface = Color(0xFF16202B),
        onSurfaceVariant = Color(0xFF5B6576),
        surfaceContainer = Color.White,
        surfaceContainerHigh = Color(0xFFE4E9EF),
        outline = Color(0xFF8A94A4),
        outlineVariant = Color(0xFFD3D9E1),
        error = Color(0xFFB3261E),
    )

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
    letterSpacing = tracking.em,
)

private val AppTypography =
    Typography(
        headlineLarge = style(34, 40, FontWeight.SemiBold, -0.02),
        headlineSmall = style(24, 30, FontWeight.SemiBold, -0.01),
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
        small = RoundedCornerShape(12.dp),
        medium = RoundedCornerShape(20.dp),
        large = RoundedCornerShape(28.dp),
        extraLarge = RoundedCornerShape(36.dp),
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
