package org.continueapp.android.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.withContext
import org.continueapp.android.ContinueApplication
import org.continueapp.bridge.PointerInput
import org.continueapp.bridge.ScreenInput
import org.continueapp.bridge.TrustedPeer

/** Computer pixels per phone pixel. */
private const val SPEED = 1.6f

/** How far two fingers move for one notch of a mouse wheel. */
private const val SCROLL_STEP_PX = 48f

/**
 * The phone as the computer's touchpad: one finger moves the pointer and taps to click, a long
 * press or a two-finger tap opens a menu, two fingers scroll, and the keyboard types there.
 */
@Composable
fun TouchpadDialog(
    peer: TrustedPeer,
    onMessage: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    val bridge = (LocalContext.current.applicationContext as ContinueApplication).coreBridge
    val inputs = remember { Channel<PointerInput>(Channel.UNLIMITED) }
    var typed by remember { mutableStateOf("") }

    // One sender, so presses and moves reach the computer in order without holding up the screen.
    LaunchedEffect(peer.fingerprint) {
        val started =
            withContext(Dispatchers.IO) { runCatching { bridge.touchpadStart(peer.fingerprint) }.getOrDefault(false) }
        if (!started) {
            onMessage(
                "${peer.displayName} isn't taking touchpad input. Check that Touchpad is on for this phone there.",
            )
            onDismiss()
            return@LaunchedEffect
        }
        try {
            withContext(Dispatchers.IO) {
                for (input in inputs) if (!bridge.touchpadInput(input)) break
            }
            onMessage("${peer.displayName} went away.")
            onDismiss()
        } finally {
            bridge.touchpadStop()
        }
    }

    Dialog(onDismissRequest = onDismiss, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.surface) {
            Column(
                modifier = Modifier.safeDrawingPadding().padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        "Touchpad for ${peer.displayName}",
                        style = MaterialTheme.typography.titleLarge,
                        modifier = Modifier.weight(1f),
                    )
                    IconButton(onClick = onDismiss) { Icon(Icons.Outlined.Close, contentDescription = "Close") }
                }
                Box(
                    contentAlignment = Alignment.Center,
                    modifier =
                        Modifier
                            .weight(1f)
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(28.dp))
                            .background(MaterialTheme.colorScheme.surfaceContainerHigh)
                            .pointerInput(Unit) { touchpadGestures { inputs.trySend(it) } },
                ) {
                    Text(
                        "Move, tap, or scroll with two fingers",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                OutlinedTextField(
                    value = typed,
                    onValueChange = { next ->
                        keysFor(typed, next).forEach { inputs.trySend(it) }
                        typed = next
                    },
                    placeholder = { Text("Type on ${peer.displayName}") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                    keyboardActions =
                        KeyboardActions(
                            onSend = {
                                inputs.trySend(PointerInput.Screen(ScreenInput.Enter))
                                typed = ""
                            },
                        ),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }
    }
}

/** What turning [before] into [after] takes on the computer: backspaces, then new text. */
private fun keysFor(
    before: String,
    after: String,
): List<PointerInput> {
    val kept = before.commonPrefixWith(after).length
    val erased = List(before.length - kept) { PointerInput.Screen(ScreenInput.Backspace) }
    val added = after.substring(kept)
    return if (added.isEmpty()) erased else erased + PointerInput.Screen(ScreenInput.Text(added))
}

private suspend fun androidx.compose.ui.input.pointer.PointerInputScope.touchpadGestures(send: (PointerInput) -> Unit) {
    val slop = viewConfiguration.touchSlop
    val longPress = viewConfiguration.longPressTimeoutMillis
    awaitEachGesture {
        val first = awaitFirstDown()
        var fingers = 1
        var travelled = 0f
        var scrolled = 0f
        var lastTime = first.uptimeMillis
        do {
            val event = awaitPointerEvent()
            val pressed = event.changes.filter { it.pressed }
            fingers = maxOf(fingers, pressed.size)
            lastTime = event.changes.maxOf { it.uptimeMillis }
            if (pressed.size >= 2) {
                val dy = pressed.map { it.position.y - it.previousPosition.y }.average().toFloat()
                travelled += kotlin.math.abs(dy)
                scrolled += dy
                // Fingers moving down pull the page down, as on the phone itself.
                val notches = (scrolled / SCROLL_STEP_PX).toInt()
                if (notches != 0) {
                    send(PointerInput.Scroll(notches.toFloat()))
                    scrolled -= notches * SCROLL_STEP_PX
                }
            } else if (pressed.size == 1 && fingers == 1) {
                val change = pressed.first()
                val delta = change.position - change.previousPosition
                travelled += delta.getDistance()
                if (travelled > slop) send(PointerInput.Move(delta.x * SPEED, delta.y * SPEED))
            }
            event.changes.forEach { it.consume() }
        } while (event.changes.any { it.pressed })

        if (travelled <= slop) {
            val menu = fingers >= 2 || lastTime - first.uptimeMillis >= longPress
            val (down, up) =
                if (menu) {
                    PointerInput.PressSecondary(
                        true,
                    ) to PointerInput.PressSecondary(false)
                } else {
                    PointerInput.Press(true) to PointerInput.Press(false)
                }
            send(down)
            send(up)
        }
    }
}
