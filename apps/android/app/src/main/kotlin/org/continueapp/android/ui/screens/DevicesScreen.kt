package org.continueapp.android.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.continueapp.android.ui.theme.Emerald500
import org.continueapp.android.ui.theme.Rose500
import org.continueapp.android.ui.theme.Slate400
import org.continueapp.android.ui.theme.Slate700
import org.continueapp.android.ui.theme.Slate800
import org.continueapp.bridge.TrustedPeer

private const val SUBSTRING_LENGTH = 16

@Composable
fun DevicesScreen(
    deviceFingerprint: String,
    deviceSpkiHash: String,
    peers: List<TrustedPeer>,
    onPairQr: (String) -> Unit,
    onRemovePeer: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    var qrInput by remember { mutableStateOf("") }
    var showPairInput by remember { mutableStateOf(false) }

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
                    text = "This Device",
                    fontSize = 16.sp,
                    fontWeight = FontWeight.SemiBold,
                    color = Color.White,
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "Fingerprint: ${deviceFingerprint.take(SUBSTRING_LENGTH)}...",
                    fontSize = 13.sp,
                    fontFamily = FontFamily.Monospace,
                    color = Slate400,
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = "SPKI Hash: ${deviceSpkiHash.take(SUBSTRING_LENGTH)}...",
                    fontSize = 13.sp,
                    fontFamily = FontFamily.Monospace,
                    color = Slate400,
                )
            }
        }

        if (showPairInput) {
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = Slate800),
                shape = RoundedCornerShape(8.dp),
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text(
                        text = "Pair New Device",
                        fontSize = 16.sp,
                        fontWeight = FontWeight.SemiBold,
                        color = Color.White,
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    OutlinedTextField(
                        value = qrInput,
                        onValueChange = { qrInput = it },
                        label = { Text("QR Payload (continue://pair/...)") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.End,
                    ) {
                        Button(
                            onClick = { showPairInput = false },
                            colors = ButtonDefaults.buttonColors(containerColor = Slate700),
                        ) {
                            Text("Cancel")
                        }
                        Spacer(modifier = Modifier.width(8.dp))
                        Button(
                            onClick = {
                                if (qrInput.isNotBlank()) {
                                    onPairQr(qrInput.trim())
                                    qrInput = ""
                                    showPairInput = false
                                }
                            },
                        ) {
                            Text("Confirm")
                        }
                    }
                }
            }
        } else {
            Button(
                onClick = { showPairInput = true },
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(8.dp),
            ) {
                Text("Pair Remote Device")
            }
        }

        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Slate800),
            shape = RoundedCornerShape(8.dp),
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = "Trusted Devices (${peers.size})",
                    fontSize = 16.sp,
                    fontWeight = FontWeight.SemiBold,
                    color = Color.White,
                )
                Spacer(modifier = Modifier.height(12.dp))
                if (peers.isEmpty()) {
                    Text(
                        text = "No devices paired yet.",
                        fontSize = 14.sp,
                        color = Slate400,
                    )
                } else {
                    peers.forEach { peer ->
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
                                    text = peer.displayName,
                                    fontSize = 14.sp,
                                    fontWeight = FontWeight.Medium,
                                    color = Color.White,
                                )
                                Text(
                                    text = "${peer.fingerprint.take(SUBSTRING_LENGTH)}...",
                                    fontSize = 12.sp,
                                    fontFamily = FontFamily.Monospace,
                                    color = Slate400,
                                )
                            }
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Box(
                                    modifier =
                                        Modifier
                                            .size(8.dp)
                                            .clip(CircleShape)
                                            .background(Emerald500),
                                )
                                Spacer(modifier = Modifier.width(6.dp))
                                Text(
                                    text = "Paired",
                                    fontSize = 12.sp,
                                    color = Emerald500,
                                )
                                Spacer(modifier = Modifier.width(12.dp))
                                Button(
                                    onClick = { onRemovePeer(peer.fingerprint) },
                                    colors = ButtonDefaults.buttonColors(containerColor = Rose500),
                                    shape = RoundedCornerShape(4.dp),
                                ) {
                                    Text("Remove", fontSize = 12.sp)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
