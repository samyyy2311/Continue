package org.continueapp.bridge

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.media.AudioManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.telecom.TelecomManager
import org.continueapp.bridge.ffi.CallControlFfi

val CALLS_PERMISSIONS =
    arrayOf(
        Manifest.permission.READ_PHONE_STATE,
        Manifest.permission.READ_CALL_LOG,
        Manifest.permission.ANSWER_PHONE_CALLS,
        Manifest.permission.CALL_PHONE,
        Manifest.permission.READ_CONTACTS,
    )

fun canSeeCalls(context: Context): Boolean =
    context.checkSelfPermission(Manifest.permission.READ_PHONE_STATE) == PackageManager.PERMISSION_GRANTED

/** Call audio stays on the phone. */
internal class TelecomCalls(
    private val context: Context,
) : CallControlFfi {
    private val telecom get() = context.getSystemService(TelecomManager::class.java)

    private fun mayAnswer() =
        context.checkSelfPermission(Manifest.permission.ANSWER_PHONE_CALLS) == PackageManager.PERMISSION_GRANTED

    override fun answer(): Boolean {
        if (!mayAnswer()) return false
        @Suppress("DEPRECATION", "MissingPermission")
        telecom?.acceptRingingCall() ?: return false
        return true
    }

    override fun decline(): Boolean {
        if (!mayAnswer() || Build.VERSION.SDK_INT < Build.VERSION_CODES.P) return false
        @Suppress("DEPRECATION", "MissingPermission")
        return telecom?.endCall() == true
    }

    /** Only the phone's dialer may silence a call itself, so the ringtone is muted until the call stops ringing. */
    override fun silence(): Boolean {
        val audio = context.getSystemService(AudioManager::class.java) ?: return false
        // Refused with Do Not Disturb rules some phones apply to ringer changes.
        val muted =
            runCatching {
                audio.adjustStreamVolume(
                    AudioManager.STREAM_RING,
                    AudioManager.ADJUST_MUTE,
                    0,
                )
            }.isSuccess
        ringerMuted = ringerMuted || muted
        return muted
    }

    override fun dial(number: String): Boolean {
        val allowed = context.checkSelfPermission(Manifest.permission.CALL_PHONE) == PackageManager.PERMISSION_GRANTED
        if (!allowed || number.isBlank()) return false
        @Suppress("MissingPermission")
        telecom?.placeCall(Uri.fromParts("tel", number, null), Bundle()) ?: return false
        return true
    }
}

@Volatile private var ringerMuted = false

/** Undoes [TelecomCalls.silence] once the call is no longer ringing. */
fun restoreRinger(context: Context) {
    if (!ringerMuted) return
    ringerMuted = false
    runCatching {
        context.getSystemService(
            AudioManager::class.java,
        )?.adjustStreamVolume(AudioManager.STREAM_RING, AudioManager.ADJUST_UNMUTE, 0)
    }
}
