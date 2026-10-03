package org.continueapp.android.ui.screens

import androidx.compose.foundation.layout.Row
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.window.DialogProperties
import kotlinx.coroutines.launch
import org.continueapp.android.PermissionQuestions
import org.continueapp.bridge.Capability
import org.continueapp.bridge.PermissionAnswer
import org.continueapp.bridge.PermissionQuestion

/** How a question reads, in the app and in its notification: a headline and the choice. */
fun describe(question: PermissionQuestion): Pair<String, String> {
    val name = question.peerName.ifBlank { "Your computer" }
    val (what, kind) =
        when (question.capability) {
            Capability.FILE_TRANSFER -> (question.detail ?: "a file") to "files"
            Capability.CLIPBOARD -> "some text" to "text"
            else -> "notifications" to "notifications"
        }
    return "$name wants to send $what" to "Allow it this once, or always allow $kind from $name."
}

/** Shows questions from devices set to Ask while the app is open. */
@Composable
fun PermissionPrompts(questions: PermissionQuestions) {
    val scope = rememberCoroutineScope()
    val question = questions.current ?: return
    PermissionDialog(question, onAnswer = { answer -> scope.launch { questions.answer(question.id, answer) } })
}

/** Asks whether a device set to Ask may send something. Leaving it unanswered declines. */
@Composable
private fun PermissionDialog(
    question: PermissionQuestion,
    onAnswer: (PermissionAnswer) -> Unit,
) {
    val (title, body) = describe(question)
    AlertDialog(
        onDismissRequest = {},
        properties = DialogProperties(dismissOnBackPress = false, dismissOnClickOutside = false),
        title = { Text(title) },
        text = { Text(body) },
        confirmButton = { TextButton(onClick = { onAnswer(PermissionAnswer.ALLOW) }) { Text("Allow") } },
        dismissButton = {
            Row {
                TextButton(onClick = { onAnswer(PermissionAnswer.DECLINE) }) { Text("Decline") }
                TextButton(onClick = { onAnswer(PermissionAnswer.ALWAYS_ALLOW) }) { Text("Always allow") }
            }
        },
    )
}
