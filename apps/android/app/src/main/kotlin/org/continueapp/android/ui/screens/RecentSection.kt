package org.continueapp.android.ui.screens

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri
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
import org.continueapp.android.mimeType
import org.continueapp.android.ui.components.SectionLabel
import org.continueapp.android.ui.components.SettingsRow
import org.continueapp.android.ui.theme.success

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
    recent.items.take(RECENT_ON_HOME).forEach { RecentRow(it) }
}

@Composable
private fun RecentRow(transfer: Transfer) {
    val context = LocalContext.current
    val name = transfer.peerName
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
        onClick = transfer.uri?.let { uri -> { openFile(context, uri, transfer.label) } },
        trailing =
            if (transfer.status == TransferStatus.Received) {
                null
            } else {
                { StatusIcon(transfer.status) }
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

/** Opens a received file in whatever app handles it, if there is one. */
private fun openFile(
    context: Context,
    uri: Uri,
    name: String,
) {
    val intent =
        Intent(Intent.ACTION_VIEW)
            .setDataAndType(uri, mimeType(name))
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    try {
        context.startActivity(intent)
    } catch (_: ActivityNotFoundException) {
    }
}
