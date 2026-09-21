@file:Suppress("MagicNumber")

package org.continueapp.android.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

val Slate900 = Color(0xFF0F172A)
val Slate800 = Color(0xFF1E293B)
val Slate700 = Color(0xFF334155)
val Slate400 = Color(0xFF94A3B8)
val Sky400 = Color(0xFF38BDF8)
val Emerald500 = Color(0xFF10B981)
val Rose500 = Color(0xFFEF4444)

private val DarkColorScheme =
    darkColorScheme(
        primary = Sky400,
        background = Slate900,
        surface = Slate800,
        onPrimary = Color.Black,
        onBackground = Color(0xFFF8FAFC),
        onSurface = Color(0xFFF8FAFC),
    )

private val LightColorScheme =
    lightColorScheme(
        primary = Sky400,
        background = Color(0xFFF8FAFC),
        surface = Color.White,
        onPrimary = Color.White,
        onBackground = Slate900,
        onSurface = Slate900,
    )

@Composable
fun ContinueTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val colorScheme = if (darkTheme) DarkColorScheme else LightColorScheme
    MaterialTheme(
        colorScheme = colorScheme,
        content = content,
    )
}
