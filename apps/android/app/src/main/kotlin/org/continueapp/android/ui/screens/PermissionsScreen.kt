package org.continueapp.android.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.continueapp.android.ui.theme.Sky400
import org.continueapp.android.ui.theme.Slate400
import org.continueapp.android.ui.theme.Slate800

@Composable
fun PermissionsScreen(modifier: Modifier = Modifier) {
    var fileTransferEnabled by remember { mutableStateOf(true) }
    var clipboardSyncEnabled by remember { mutableStateOf(true) }
    var notificationRelayEnabled by remember { mutableStateOf(true) }

    Column(
        modifier = modifier.padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Slate800),
            shape = RoundedCornerShape(8.dp),
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = "Four-Layer Security Model",
                    fontSize = 16.sp,
                    fontWeight = FontWeight.SemiBold,
                    color = Color.White,
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text =
                        "Capabilities are guarded by platform capability, application settings, " +
                            "peer trust status, and per-session capability negotiation.",
                    fontSize = 13.sp,
                    color = Slate400,
                )
            }
        }

        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Slate800),
            shape = RoundedCornerShape(8.dp),
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = "Capability Grants",
                    fontSize = 16.sp,
                    fontWeight = FontWeight.SemiBold,
                    color = Color.White,
                )
                Spacer(modifier = Modifier.height(12.dp))

                PermissionRow(
                    title = "File Transfer",
                    description = "Stream and receive encrypted files",
                    checked = fileTransferEnabled,
                    onCheckedChange = { fileTransferEnabled = it },
                )

                Spacer(modifier = Modifier.height(8.dp))

                PermissionRow(
                    title = "Clipboard Synchronization",
                    description = "Bidirectional clipboard sync with loop prevention",
                    checked = clipboardSyncEnabled,
                    onCheckedChange = { clipboardSyncEnabled = it },
                )

                Spacer(modifier = Modifier.height(8.dp))

                PermissionRow(
                    title = "Notification Relay",
                    description = "Forward and dismiss alerts across devices",
                    checked = notificationRelayEnabled,
                    onCheckedChange = { notificationRelayEnabled = it },
                )
            }
        }
    }
}

@Composable
private fun PermissionRow(
    title: String,
    description: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(vertical = 8.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                fontSize = 14.sp,
                fontWeight = FontWeight.Medium,
                color = Color.White,
            )
            Text(
                text = description,
                fontSize = 12.sp,
                color = Slate400,
            )
        }
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
            colors =
                SwitchDefaults.colors(
                    checkedThumbColor = Color.White,
                    checkedTrackColor = Sky400,
                ),
        )
    }
}
