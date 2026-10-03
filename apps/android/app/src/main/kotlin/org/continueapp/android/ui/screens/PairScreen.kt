package org.continueapp.android.ui.screens

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.ArrowBack
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import kotlinx.coroutines.launch
import org.continueapp.android.ui.components.ActionButton
import org.continueapp.android.ui.components.PageTitle
import org.continueapp.android.ui.components.ScreenPadding

/** [onPair] returns an error message, or null once paired. */
@Composable
fun PairScreen(
    onPair: suspend (String) -> String?,
    onDone: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var scanning by remember { mutableStateOf(false) }
    var typedCode by remember { mutableStateOf("") }
    var pairing by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf<String?>(null) }
    // Changing this restarts the scanner after a failed attempt.
    var scanAttempt by remember { mutableIntStateOf(0) }

    val requestCamera =
        rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            scanning = granted
            if (!granted) message = "Camera access is off. Type the code shown on your computer instead."
        }

    fun startScanning() {
        message = null
        val granted =
            ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) ==
                PackageManager.PERMISSION_GRANTED
        if (granted) scanning = true else requestCamera.launch(Manifest.permission.CAMERA)
    }

    fun pair(code: String) {
        pairing = true
        message = null
        scope.launch {
            val error = onPair(code)
            pairing = false
            if (error == null) {
                onDone()
            } else {
                message = error
                scanAttempt++
            }
        }
    }

    LaunchedEffect(Unit) { startScanning() }

    Column(modifier = modifier.verticalScroll(rememberScrollState()).padding(horizontal = ScreenPadding)) {
        IconButton(onClick = onDone, modifier = Modifier.padding(top = 8.dp)) {
            Icon(Icons.AutoMirrored.Outlined.ArrowBack, contentDescription = "Back")
        }
        PageTitle("Pair a device")
        Text(
            if (scanning) {
                "Point the camera at the code in Continue on your computer. The camera is only used to read it."
            } else {
                "Type or paste the code shown in Continue on your computer."
            },
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        Column(
            modifier = Modifier.padding(vertical = 24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            when {
                pairing -> Text("Pairing…", style = MaterialTheme.typography.titleMedium)
                scanning ->
                    key(scanAttempt) {
                        QrScanner(
                            onScanned = ::pair,
                            modifier =
                                Modifier
                                    .fillMaxWidth()
                                    .aspectRatio(1f)
                                    .clip(MaterialTheme.shapes.large)
                                    .border(2.dp, MaterialTheme.colorScheme.primary, MaterialTheme.shapes.large),
                        )
                    }
                else -> {
                    OutlinedTextField(
                        value = typedCode,
                        onValueChange = { typedCode = it },
                        label = { Text("Pairing code") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                    )
                    ActionButton(onClick = { pair(typedCode.trim()) }, enabled = typedCode.isNotBlank()) {
                        Text("Pair")
                    }
                }
            }
            message?.let {
                Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.error)
            }
        }

        TextButton(
            onClick = { if (scanning) scanning = false else startScanning() },
            enabled = !pairing,
        ) {
            Text(if (scanning) "Type the code instead" else "Scan with the camera")
        }
    }
}
