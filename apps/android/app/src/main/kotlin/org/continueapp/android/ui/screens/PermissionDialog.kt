package org.continueapp.android.ui.screens

import androidx.compose.foundation.layout.Row
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.window.DialogProperties
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.continueapp.android.PermissionQuestions
import org.continueapp.bridge.Capability
import org.continueapp.bridge.PermissionAnswer
import org.continueapp.bridge.PermissionQuestion

/** The core declines a question after waiting this long, so the dialog goes away with it. */
private const val QUESTION_TIMEOUT_MS = 30_000L

/** Shows questions from devices set to Ask while the app is open. */
@Composable
fun PermissionPrompts(questions: PermissionQuestions) {
    val scope = rememberCoroutineScope()
    LaunchedEffect(Unit) { questions.listen() }
    val question = questions.current ?: return
    PermissionDialog(question, onAnswer = { answer -> scope.launch { questions.answer(answer) } })
    LaunchedEffect(question.id) {
        delay(QUESTION_TIMEOUT_MS)
        questions.dismiss(question.id)
    }
}

/** Asks whether a device set to Ask may send something. Leaving it unanswered declines. */
@Composable
private fun PermissionDialog(
    question: PermissionQuestion,
    onAnswer: (PermissionAnswer) -> Unit,
) {
    val name = question.peerName.ifBlank { "A paired device" }
    val (title, kind) =
        when (question.capability) {
            Capability.FILE_TRANSFER -> "$name wants to send ${question.detail ?: "a file"}" to "files"
            Capability.CLIPBOARD -> "$name wants to send text" to "text"
            else -> "$name wants to show a notification" to "notifications"
        }
    AlertDialog(
        onDismissRequest = {},
        properties = DialogProperties(dismissOnBackPress = false, dismissOnClickOutside = false),
        title = { Text(title) },
        text = { Text("Allow it this once, or always allow $kind from $name.") },
        confirmButton = { TextButton(onClick = { onAnswer(PermissionAnswer.ALLOW) }) { Text("Allow") } },
        dismissButton = {
            Row {
                TextButton(onClick = { onAnswer(PermissionAnswer.DECLINE) }) { Text("Decline") }
                TextButton(onClick = { onAnswer(PermissionAnswer.ALWAYS_ALLOW) }) { Text("Always allow") }
            }
        },
    )
}
