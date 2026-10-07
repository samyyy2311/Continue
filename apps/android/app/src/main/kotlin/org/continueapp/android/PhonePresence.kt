package org.continueapp.android

import android.Manifest
import android.annotation.SuppressLint
import android.bluetooth.BluetoothManager
import android.bluetooth.le.AdvertiseCallback
import android.bluetooth.le.AdvertiseData
import android.bluetooth.le.AdvertiseSettings
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.ParcelUuid
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

/** The service the presence token is sent under, shared with the desktop app. */
private val PRESENCE_SERVICE = ParcelUuid.fromString("fc36928a-e182-4ac1-adef-598feab5b6c8")

/** The token changes every ten minutes; checking each minute also picks up Bluetooth coming back on. */
private const val CHECK_MS = 60_000L

fun canAdvertise(context: Context): Boolean =
    Build.VERSION.SDK_INT < Build.VERSION_CODES.S ||
        context.checkSelfPermission(Manifest.permission.BLUETOOTH_ADVERTISE) == PackageManager.PERMISSION_GRANTED

/**
 * Sends the phone's presence token over Bluetooth Low Energy, so a paired computer can lock when
 * the phone moves away. Only paired computers can tell the token is this phone's.
 */
class PhonePresence(
    private val app: ContinueApplication,
) {
    private var job: Job? = null
    private var advertising: Pair<ByteArray, AdvertiseCallback>? = null

    fun start() {
        if (job != null || !canAdvertise(app)) return
        job =
            app.scope.launch {
                while (isActive) {
                    advertise()
                    delay(CHECK_MS)
                }
            }
    }

    fun stop() {
        job?.cancel()
        job = null
        stopAdvertising()
    }

    @SuppressLint("MissingPermission") // start() checks it.
    private fun advertise() {
        val advertiser = app.getSystemService(BluetoothManager::class.java)?.adapter?.bluetoothLeAdvertiser
        val token = runCatching { app.coreBridge.presenceToken() }.getOrNull()
        if (advertiser == null || token == null) {
            // Bluetooth is off; Android drops advertising with it.
            advertising = null
            return
        }
        if (advertising?.first?.contentEquals(token) == true) return
        stopAdvertising()
        val settings =
            AdvertiseSettings
                .Builder()
                .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_LOW_POWER)
                .setTxPowerLevel(AdvertiseSettings.ADVERTISE_TX_POWER_MEDIUM)
                .setConnectable(false)
                .build()
        val data = AdvertiseData.Builder().addServiceData(PRESENCE_SERVICE, token).build()
        val callback = object : AdvertiseCallback() {}
        advertiser.startAdvertising(settings, data, callback)
        advertising = token to callback
    }

    @SuppressLint("MissingPermission")
    private fun stopAdvertising() {
        val (_, callback) = advertising ?: return
        app.getSystemService(BluetoothManager::class.java)?.adapter?.bluetoothLeAdvertiser?.stopAdvertising(callback)
        advertising = null
    }
}
