package org.continueapp.android.ui.screens

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.text.format.DateUtils
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.ContentPaste
import androidx.compose.material.icons.outlined.Description
import androidx.compose.material.icons.outlined.Done
import androidx.compose.material.icons.outlined.ErrorOutline
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.continueapp.android.RecentTransfers
import org.continueapp.android.Transfer
import org.continueapp.android.TransferKind
import org.continueapp.android.TransferStatus
import org.continueapp.android.copyToClipboard
import org.continueapp.android.ui.components.SectionLabel
import org.continueapp.android.ui.components.SettingsRow
import org.continueapp.android.ui.theme.success
import org.continueapp.android.viewIntent

private const val RECENT_ON_HOME = 5

/**
 * Displays recent transfers with an action to clear them.
 *
 * Shows at most the configured number of recent transfers and calls [onMessage] with any message
 * produced when clearing them.
 *
 * @param onMessage Receives a message produced by the clear operation.
 */
@Composable
fun RecentSection(
    recent: RecentTransfers,
    onMessage: (String) -> Unit,
) {
    if (recent.items.isEmpty()) return
    val scope = rememberCoroutineScope()
    Row(verticalAlignment = Alignment.Bottom) {
        SectionLabel("Recent", modifier = Modifier.weight(1f))
        TextButton(onClick = { scope.launch { recent.clear()?.let(onMessage) } }) { Text("Clear") }
    }
    recent.items.take(RECENT_ON_HOME).forEach { RecentRow(it, onMessage) }
}

/**
 * Displays a transfer with its label, status, and available actions.
 *
 * Clicking the row opens an associated file or copies its text. A standalone web link in the text
 * is shown with an Open button.
 *
 * @param transfer The transfer to display.
 * @param onMessage Receives messages from copy and open actions.
 */
@Composable
private fun RecentRow(
    transfer: Transfer,
    onMessage: (String) -> Unit,
) {
    val context = LocalContext.current
    val name = transfer.peerName
    val link = linkIn(transfer.text)
    SettingsRow(
        title = transfer.label,
        icon = if (transfer.kind == TransferKind.File) Icons.Outlined.Description else Icons.Outlined.ContentPaste,
        subtitle =
            when (transfer.status) {
                TransferStatus.Sending -> "Sending to $name"
                TransferStatus.Sent -> "Sent to $name · ${whenText(transfer.at)}"
                TransferStatus.Failed -> "Couldn't send to $name"
                TransferStatus.Received ->
                    (if (transfer.kind == TransferKind.Text) "Copied from $name" else "From $name") +
                        " · ${whenText(transfer.at)}"
            },
        onClick =
            when {
                transfer.uri != null -> { -> openFile(context, transfer.uri, transfer.label, onMessage) }
                transfer.text != null -> { -> copyAgain(context, transfer.text, onMessage) }
                else -> null
            },
        trailing =
            when {
                link != null -> {
                    { TextButton(onClick = { openLink(context, link, onMessage) }) { Text("Open") } }
                }
                transfer.status == TransferStatus.Received -> null
                else -> {
                    { StatusIcon(transfer.status) }
                }
            },
    )
}

/**
 * Displays an icon indicating the transfer status.
 */
@Composable
private fun StatusIcon(status: TransferStatus) {
    val colors = MaterialTheme.colorScheme
    when (status) {
        TransferStatus.Sending -> CircularProgressIndicator(Modifier.size(20.dp), strokeWidth = 2.dp)
        TransferStatus.Sent -> Icon(Icons.Outlined.Done, null, tint = colors.success)
        TransferStatus.Failed -> Icon(Icons.Outlined.ErrorOutline, null, tint = colors.error)
        TransferStatus.Received -> Unit
    }
}

/**
 * Formats a timestamp as "Just now" if it is less than a minute old, or as a relative time otherwise.
 */
private fun whenText(at: Long): String {
    val now = System.currentTimeMillis()
    return if (now - at < DateUtils.MINUTE_IN_MILLIS) {
        "Just now"
    } else {
        DateUtils.getRelativeTimeSpanString(at, now, DateUtils.MINUTE_IN_MILLIS, DateUtils.FORMAT_ABBREV_ALL).toString()
    }
}

/**
 * Copies text to the clipboard and reports copy feedback on Android versions older than Android 13.
 *
 * @param onMessage Receives the copy confirmation message on older Android versions.
 */
private fun copyAgain(
    context: Context,
    text: String,
    onMessage: (String) -> Unit,
) {
    copyToClipboard(context, text)
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) onMessage("Copied")
}

/**
 * Opens a received file in an app that can handle it.
 *
 * If no app can open the file, reports a message through [onMessage].
 */
private fun openFile(
    context: Context,
    uri: Uri,
    name: String,
    onMessage: (String) -> Unit,
) {
    try {
        context.startActivity(viewIntent(uri, name))
    } catch (_: ActivityNotFoundException) {
        onMessage("No app on this phone can open $name.")
    }
}

/**
 * Opens a link and reports a message if no app can handle it.
 *
 * @param onMessage Receives the message when no app can open the link.
 */
private fun openLink(
    context: Context,
    link: Uri,
    onMessage: (String) -> Unit,
) {
    try {
        context.startActivity(Intent(Intent.ACTION_VIEW, link))
    } catch (_: ActivityNotFoundException) {
        onMessage("No app on this phone can open links.")
    }
}

/**
 * Finds an HTTP or HTTPS link in text that contains no other non-whitespace content.
 */
private fun linkIn(text: String?): Uri? {
    val trimmed = text?.trim().orEmpty()
    val uri = Uri.parse(trimmed)
    val web = uri.scheme == "https" || uri.scheme == "http"
    return uri.takeIf { web && !uri.host.isNullOrEmpty() && trimmed.none(Char::isWhitespace) }
}
