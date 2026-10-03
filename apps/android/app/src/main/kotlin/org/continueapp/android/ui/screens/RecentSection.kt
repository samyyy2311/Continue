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

/** "Just now", "5 minutes ago", "Yesterday" or a short date. */
private fun whenText(at: Long): String {
    val now = System.currentTimeMillis()
    return if (now - at < DateUtils.MINUTE_IN_MILLIS) {
        "Just now"
    } else {
        DateUtils.getRelativeTimeSpanString(at, now, DateUtils.MINUTE_IN_MILLIS, DateUtils.FORMAT_ABBREV_ALL).toString()
    }
}

/** Android 13 and newer confirm a copy themselves, so only older versions get a message. */
private fun copyAgain(
    context: Context,
    text: String,
    onMessage: (String) -> Unit,
) {
    copyToClipboard(context, text)
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) onMessage("Copied")
}

/** Opens a received file in whatever app handles it, if there is one. */
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

/** The web link, if the text is nothing but one. */
private fun linkIn(text: String?): Uri? {
    val trimmed = text?.trim().orEmpty()
    val uri = Uri.parse(trimmed)
    val web = uri.scheme == "https" || uri.scheme == "http"
    return uri.takeIf { web && !uri.host.isNullOrEmpty() && trimmed.none(Char::isWhitespace) }
}
