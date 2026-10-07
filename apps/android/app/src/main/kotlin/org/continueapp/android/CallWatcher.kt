package org.continueapp.android

import android.content.Context
import android.telephony.PhoneStateListener
import android.telephony.TelephonyManager
import org.continueapp.bridge.CallState
import org.continueapp.bridge.ContinueCoreBridge
import org.continueapp.bridge.contactName
import org.continueapp.bridge.restoreRinger
import kotlin.concurrent.thread

/**
 * Tells connected computers when the phone rings, is answered and hangs up. Uses the deprecated
 * PhoneStateListener because TelephonyCallback no longer passes the caller's number.
 */
@Suppress("DEPRECATION")
class CallWatcher(
    private val context: Context,
    private val bridge: ContinueCoreBridge,
) : PhoneStateListener() {
    private var number = ""
    private var previous = TelephonyManager.CALL_STATE_IDLE

    @Deprecated("Deprecated in Java")
    override fun onCallStateChanged(
        state: Int,
        phoneNumber: String?,
    ) {
        if (!phoneNumber.isNullOrBlank()) number = phoneNumber
        val call =
            when {
                state == TelephonyManager.CALL_STATE_RINGING -> CallState.RINGING
                state == TelephonyManager.CALL_STATE_OFFHOOK -> CallState.TALKING
                previous != TelephonyManager.CALL_STATE_IDLE -> CallState.ENDED
                else -> return
            }
        previous = state
        val caller = number
        if (call == CallState.ENDED) number = ""
        if (call != CallState.RINGING) restoreRinger(context)
        thread { bridge.announceCall(call, caller, contactName(context, caller).orEmpty()) }
    }

    fun start() {
        context.getSystemService(TelephonyManager::class.java)?.listen(this, LISTEN_CALL_STATE)
    }
}
