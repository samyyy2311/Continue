package org.continueapp.android

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioPlaybackCaptureConfiguration
import android.media.AudioRecord
import android.media.projection.MediaProjection
import android.os.Build
import androidx.annotation.RequiresApi
import org.continueapp.bridge.ContinueCoreBridge
import org.continueapp.bridge.VideoKind
import kotlin.concurrent.thread

private const val SAMPLE_RATE = 48_000

/** 20 ms of 16-bit stereo. */
private const val CHUNK_BYTES = SAMPLE_RATE / 50 * 4

fun canCaptureSound(context: Context): Boolean =
    Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q &&
        context.checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED

/**
 * What's playing on the phone while its screen is shared, sent alongside the picture. Apps that
 * keep their sound private, as calls do, aren't captured.
 */
@RequiresApi(Build.VERSION_CODES.Q)
class ScreenSound(
    projection: MediaProjection,
    private val bridge: ContinueCoreBridge,
) {
    @SuppressLint("MissingPermission") // canCaptureSound checks it before this is made.
    private val record =
        AudioRecord
            .Builder()
            .setAudioFormat(
                AudioFormat
                    .Builder()
                    .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                    .setSampleRate(SAMPLE_RATE)
                    .setChannelMask(AudioFormat.CHANNEL_IN_STEREO)
                    .build(),
            ).setBufferSizeInBytes(CHUNK_BYTES * 4)
            .setAudioPlaybackCaptureConfig(
                AudioPlaybackCaptureConfiguration
                    .Builder(projection)
                    .addMatchingUsage(AudioAttributes.USAGE_MEDIA)
                    .addMatchingUsage(AudioAttributes.USAGE_GAME)
                    .addMatchingUsage(AudioAttributes.USAGE_UNKNOWN)
                    .build(),
            ).build()

    @Volatile private var running = false

    fun start() {
        running = true
        record.startRecording()
        thread(name = "screen sound") {
            val chunk = ByteArray(CHUNK_BYTES)
            while (running) {
                val read = record.read(chunk, 0, chunk.size)
                if (read > 0) bridge.pushAudio(VideoKind.SCREEN, chunk.copyOf(read))
            }
        }
    }

    fun stop() {
        running = false
        runCatching { record.stop() }
        record.release()
    }
}
