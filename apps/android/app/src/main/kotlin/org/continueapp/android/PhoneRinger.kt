package org.continueapp.android

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.media.AudioAttributes
import android.media.AudioManager
import android.media.MediaPlayer
import android.media.RingtoneManager
import android.os.Handler
import android.os.Looper
import org.continueapp.bridge.Ringer

private const val RING_FOR_MS = 30_000L

/**
 * Rings so a computer can find the phone. The alarm stream plays through silent and vibrate, and
 * its volume is put back once the ringing stops.
 */
class PhoneRinger(
    private val app: ContinueApplication,
) : Ringer {
    private val main = Handler(Looper.getMainLooper())
    private val audio get() = app.getSystemService(AudioManager::class.java)
    private var player: MediaPlayer? = null
    private var volumeBefore = 0

    override fun ring(on: Boolean): Boolean {
        main.post { if (on) start() else stop() }
        return true
    }

    fun stop() {
        val playing = player ?: return
        player = null
        main.removeCallbacksAndMessages(null)
        playing.stop()
        playing.release()
        audio.setStreamVolume(AudioManager.STREAM_ALARM, volumeBefore, 0)
        cancelRinging(app)
    }

    private fun start() {
        if (player != null) return
        volumeBefore = audio.getStreamVolume(AudioManager.STREAM_ALARM)
        audio.setStreamVolume(AudioManager.STREAM_ALARM, audio.getStreamMaxVolume(AudioManager.STREAM_ALARM), 0)
        val tone =
            RingtoneManager.getActualDefaultRingtoneUri(app, RingtoneManager.TYPE_ALARM)
                ?: RingtoneManager.getDefaultUri(RingtoneManager.TYPE_RINGTONE)
        val alarm = AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_ALARM).build()
        player =
            MediaPlayer().apply {
                setAudioAttributes(alarm)
                setDataSource(app, tone)
                isLooping = true
                prepare()
                start()
            }
        notifyRinging(app, Intent(app, StopRingingReceiver::class.java))
        main.postDelayed(::stop, RING_FOR_MS)
    }
}

class StopRingingReceiver : BroadcastReceiver() {
    override fun onReceive(
        context: Context,
        intent: Intent,
    ) = (context.applicationContext as ContinueApplication).phoneRinger.stop()
}
