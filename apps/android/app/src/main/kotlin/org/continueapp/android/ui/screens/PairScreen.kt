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
import androidx.compose.material.icons.outlined.Laptop
import androidx.compose.material3.AlertDialog
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
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.continueapp.android.AppState
import org.continueapp.android.ui.components.ActionButton
import org.continueapp.android.ui.components.PageTitle
import org.continueapp.android.ui.components.ScreenPadding
import org.continueapp.bridge.NearbyComputer

/** Scans or takes a computer's code, or picks a computer nearby and compares six digits. */
@Composable
fun PairScreen(
    state: AppState,
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
            val error = state.pair(code)
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
                scanning -> key(scanAttempt) { Scanner(onScanned = ::pair) }
                else -> TypedCode(typedCode, onChange = { typedCode = it }, onPair = { pair(typedCode.trim()) })
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

        NearbyPairing(
            state = state,
            pairing = pairing,
            onPairing = { busy ->
                pairing = busy
                if (busy) message = null
            },
            onMessage = { message = it },
            onDone = onDone,
        )
    }
}

/** Lists computers nearby; picking one pairs with it once the person confirms the six digits match. */
@Composable
private fun NearbyPairing(
    state: AppState,
    pairing: Boolean,
    onPairing: (Boolean) -> Unit,
    onMessage: (String) -> Unit,
    onDone: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    var nearby by remember { mutableStateOf(emptyList<NearbyComputer>()) }
    // The computer picked and the digits to compare with its screen.
    var comparing by remember { mutableStateOf<Pair<String, String>?>(null) }

    LaunchedEffect(Unit) {
        while (true) {
            nearby = state.nearbyComputers()
            delay(NEARBY_REFRESH_MS)
        }
    }

    fun pick(computer: NearbyComputer) {
        onPairing(true)
        scope.launch {
            state
                .pairNearby(computer.code)
                .onSuccess { digits -> comparing = computer.name to digits }
                .onFailure { onMessage("Couldn't pair with ${computer.name}. Try again.") }
            onPairing(false)
        }
    }

    fun answer(accept: Boolean) {
        comparing = null
        scope.launch {
            val error = state.confirmNearbyPairing(accept)
            if (!accept) return@launch
            if (error == null) onDone() else onMessage(error)
        }
    }

    Nearby(nearby, enabled = !pairing, onPick = ::pick)
    comparing?.let { (name, digits) -> CompareDigits(name, digits, ::answer) }
}

@Composable
private fun Scanner(onScanned: (String) -> Unit) {
    QrScanner(
        onScanned = onScanned,
        modifier =
            Modifier
                .fillMaxWidth()
                .aspectRatio(1f)
                .clip(MaterialTheme.shapes.large)
                .border(2.dp, MaterialTheme.colorScheme.primary, MaterialTheme.shapes.large),
    )
}

@Composable
private fun TypedCode(
    code: String,
    onChange: (String) -> Unit,
    onPair: () -> Unit,
) {
    OutlinedTextField(
        value = code,
        onValueChange = onChange,
        label = { Text("Pairing code") },
        modifier = Modifier.fillMaxWidth(),
        singleLine = true,
    )
    ActionButton(onClick = onPair, enabled = code.isNotBlank()) {
        Text("Pair")
    }
}

@Composable
private fun Nearby(
    computers: List<NearbyComputer>,
    enabled: Boolean,
    onPick: (NearbyComputer) -> Unit,
) {
    if (computers.isEmpty()) return
    Text("Nearby", style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(top = 16.dp))
    computers.forEach { computer ->
        TextButton(onClick = { onPick(computer) }, enabled = enabled) {
            Icon(Icons.Outlined.Laptop, contentDescription = null)
            Text(computer.name, modifier = Modifier.padding(start = 12.dp))
        }
    }
}

/** Asks whether [name] shows the same [digits], in two groups of three so they're easier to compare. */
@Composable
private fun CompareDigits(
    name: String,
    digits: String,
    onAnswer: (Boolean) -> Unit,
) {
    AlertDialog(
        onDismissRequest = { onAnswer(false) },
        title = { Text("Check $name") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Pair if it shows the same six digits:")
                Text(
                    "${digits.take(DIGIT_GROUP)} ${digits.drop(DIGIT_GROUP)}",
                    style = MaterialTheme.typography.displaySmall,
                )
            }
        },
        confirmButton = { TextButton(onClick = { onAnswer(true) }) { Text("Pair") } },
        dismissButton = { TextButton(onClick = { onAnswer(false) }) { Text("Not mine") } },
    )
}

/** How often the list of computers nearby is looked for again. */
private const val NEARBY_REFRESH_MS = 1_500L

private const val DIGIT_GROUP = 3
