package org.continueapp.android.ui.screens

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
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
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import kotlinx.coroutines.launch
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
    onPair: suspend (String) -> String?,
    onRemovePeer: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
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

        PairingCard(onPair = onPair)

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

private enum class PairMode { Closed, Scan, Type }

/** [onPair] returns an error message, or null once paired. */
@Composable
private fun PairingCard(onPair: suspend (String) -> String?) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var mode by remember { mutableStateOf(PairMode.Closed) }
    var typedCode by remember { mutableStateOf("") }
    var pairing by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf<String?>(null) }
    // Changing this restarts the scanner after a failed attempt.
    var scanAttempt by remember { mutableIntStateOf(0) }

    val requestCamera =
        rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            mode = if (granted) PairMode.Scan else PairMode.Type
            if (!granted) {
                message = "Camera access is off. Type the code shown on your computer instead."
            }
        }

    fun startScanning() {
        message = null
        val granted =
            ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) ==
                PackageManager.PERMISSION_GRANTED
        if (granted) mode = PairMode.Scan else requestCamera.launch(Manifest.permission.CAMERA)
    }

    fun pair(code: String) {
        pairing = true
        message = null
        scope.launch {
            val error = onPair(code)
            pairing = false
            if (error == null) {
                mode = PairMode.Closed
                typedCode = ""
            } else {
                message = error
                scanAttempt++
            }
        }
    }

    if (mode == PairMode.Closed) {
        Button(
            onClick = ::startScanning,
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(8.dp),
        ) {
            Text("Pair a device")
        }
        return
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = Slate800),
        shape = RoundedCornerShape(8.dp),
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(
                text = "Pair a device",
                fontSize = 16.sp,
                fontWeight = FontWeight.SemiBold,
                color = Color.White,
            )
            Text(
                text =
                    if (mode == PairMode.Scan) {
                        "Point the camera at the code shown in Continue on your computer."
                    } else {
                        "Type or paste the code shown in Continue on your computer."
                    },
                fontSize = 14.sp,
                color = Slate400,
            )

            when {
                pairing -> Text(text = "Pairing…", fontSize = 14.sp, color = Color.White)
                mode == PairMode.Scan ->
                    key(scanAttempt) {
                        QrScanner(
                            onScanned = ::pair,
                            modifier =
                                Modifier
                                    .fillMaxWidth()
                                    .aspectRatio(1f)
                                    .clip(RoundedCornerShape(8.dp)),
                        )
                    }
                else ->
                    OutlinedTextField(
                        value = typedCode,
                        onValueChange = { typedCode = it },
                        label = { Text("Pairing code") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
            }

            message?.let { Text(text = it, fontSize = 14.sp, color = Rose500) }

            PairingButtons(
                scanning = mode == PairMode.Scan,
                enabled = !pairing,
                canPairTypedCode = typedCode.isNotBlank(),
                onSwitchMode = { if (mode == PairMode.Scan) mode = PairMode.Type else startScanning() },
                onCancel = {
                    mode = PairMode.Closed
                    message = null
                },
                onPairTypedCode = { pair(typedCode.trim()) },
            )
        }
    }
}

@Composable
private fun PairingButtons(
    scanning: Boolean,
    enabled: Boolean,
    canPairTypedCode: Boolean,
    onSwitchMode: () -> Unit,
    onCancel: () -> Unit,
    onPairTypedCode: () -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TextButton(onClick = onSwitchMode, enabled = enabled) {
            Text(if (scanning) "Type the code" else "Scan instead")
        }
        Button(
            onClick = onCancel,
            enabled = enabled,
            colors = ButtonDefaults.buttonColors(containerColor = Slate700),
        ) {
            Text("Cancel")
        }
        if (!scanning) {
            Button(onClick = onPairTypedCode, enabled = enabled && canPairTypedCode) {
                Text("Pair")
            }
        }
    }
}
