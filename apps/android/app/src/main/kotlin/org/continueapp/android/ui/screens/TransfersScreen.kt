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
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.continueapp.android.ui.theme.Emerald500
import org.continueapp.android.ui.theme.Slate400
import org.continueapp.android.ui.theme.Slate800

private const val BYTES_PER_MB = 1048576.0
private const val SAMPLE_PDF_SIZE = 2097152L
private const val SAMPLE_JPG_SIZE = 4194304L

@Composable
fun TransfersScreen(modifier: Modifier = Modifier) {
    val sampleTransfers =
        listOf(
            TransferRecord("1", "document.pdf", SAMPLE_PDF_SIZE, false, "Completed"),
            TransferRecord("2", "photo.jpg", SAMPLE_JPG_SIZE, true, "Completed"),
        )

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
                    text = "Send File",
                    fontSize = 16.sp,
                    fontWeight = FontWeight.SemiBold,
                    color = Color.White,
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "Stream files directly over authenticated QUIC with SHA-256 validation.",
                    fontSize = 13.sp,
                    color = Slate400,
                )
                Spacer(modifier = Modifier.height(12.dp))
                Button(
                    onClick = { /* File picker trigger */ },
                    shape = RoundedCornerShape(8.dp),
                ) {
                    Text("Select File to Send")
                }
            }
        }

        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Slate800),
            shape = RoundedCornerShape(8.dp),
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = "Recent Transfers",
                    fontSize = 16.sp,
                    fontWeight = FontWeight.SemiBold,
                    color = Color.White,
                )
                Spacer(modifier = Modifier.height(12.dp))
                sampleTransfers.forEach { tx ->
                    Row(
                        modifier =
                            Modifier
                                .fillMaxWidth()
                                .padding(vertical = 8.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column {
                            Text(
                                text = tx.fileName,
                                fontSize = 14.sp,
                                fontWeight = FontWeight.Medium,
                                color = Color.White,
                            )
                            val mb = tx.sizeBytes / BYTES_PER_MB
                            val dir = if (tx.isOutgoing) "Outgoing" else "Incoming"
                            Text(
                                text = "%.2f MB • %s".format(mb, dir),
                                fontSize = 12.sp,
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
                                text = tx.status,
                                fontSize = 12.sp,
                                color = Emerald500,
                            )
                        }
                    }
                }
            }
        }
    }
}

data class TransferRecord(
    val id: String,
    val fileName: String,
    val sizeBytes: Long,
    val isOutgoing: Boolean,
    val status: String,
)
