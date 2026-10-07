package org.continueapp.android

import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.net.wifi.WifiManager
import android.os.BatteryManager
import android.os.Build
import android.telephony.TelephonyManager
import org.continueapp.bridge.PhoneStatus
import org.continueapp.bridge.canSeeCalls

private const val PERCENT = 100
private const val BARS = 4
private const val LEGACY_LEVELS = BARS + 1

/** Null until Android has a battery reading. */
fun readPhoneStatus(context: Context): PhoneStatus? {
    // The battery broadcast is sticky, so registering without a receiver reads the latest.
    val battery = context.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED)) ?: return null
    val level = battery.getIntExtra(BatteryManager.EXTRA_LEVEL, -1)
    val scale = battery.getIntExtra(BatteryManager.EXTRA_SCALE, -1)
    if (level < 0 || scale <= 0) return null

    val telephony = context.getSystemService(TelephonyManager::class.java)
    val hasSim = telephony?.simState == TelephonyManager.SIM_STATE_READY
    val cellBars =
        if (hasSim && Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) telephony.signalStrength?.level else null
    return PhoneStatus(
        batteryPercent = level * PERCENT / scale,
        charging = battery.getIntExtra(BatteryManager.EXTRA_PLUGGED, 0) != 0,
        cellBars = cellBars,
        wifiBars = wifiBars(context),
        carrier = if (hasSim) telephony.networkOperatorName.orEmpty() else "",
        network = if (hasSim) networkKind(context, telephony) else "",
    )
}

/** "5G", "LTE", "3G" or "2G"; empty without permission to read the phone's state. */
@SuppressLint("MissingPermission") // Checked first.
private fun networkKind(
    context: Context,
    telephony: TelephonyManager,
): String {
    if (!canSeeCalls(context)) return ""
    // 5G alongside LTE reports as LTE; telling them apart needs TelephonyDisplayInfo.
    return when (telephony.dataNetworkType) {
        TelephonyManager.NETWORK_TYPE_NR -> "5G"
        TelephonyManager.NETWORK_TYPE_LTE, TelephonyManager.NETWORK_TYPE_IWLAN -> "LTE"
        TelephonyManager.NETWORK_TYPE_HSPAP, TelephonyManager.NETWORK_TYPE_HSPA, TelephonyManager.NETWORK_TYPE_HSDPA,
        TelephonyManager.NETWORK_TYPE_HSUPA, TelephonyManager.NETWORK_TYPE_UMTS,
        -> "3G"
        TelephonyManager.NETWORK_TYPE_EDGE, TelephonyManager.NETWORK_TYPE_GPRS, TelephonyManager.NETWORK_TYPE_GSM,
        -> "2G"
        else -> ""
    }
}

private fun wifiBars(context: Context): Int? {
    val connectivity = context.getSystemService(ConnectivityManager::class.java) ?: return null
    val network = connectivity.getNetworkCapabilities(connectivity.activeNetwork) ?: return null
    if (!network.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) return null
    val wifi = context.getSystemService(WifiManager::class.java) ?: return null

    @Suppress("DEPRECATION")
    val rssi = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) network.signalStrength else wifi.connectionInfo.rssi
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
        wifi.calculateSignalLevel(rssi) * BARS / wifi.maxSignalLevel.coerceAtLeast(1)
    } else {
        @Suppress("DEPRECATION")
        WifiManager.calculateSignalLevel(rssi, LEGACY_LEVELS)
    }
}
