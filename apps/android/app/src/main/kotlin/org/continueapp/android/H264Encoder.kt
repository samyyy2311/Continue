package org.continueapp.android

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.os.Bundle
import android.os.Handler
import android.os.SystemClock
import android.util.Size
import android.view.Surface
import org.continueapp.bridge.ContinueCoreBridge
import org.continueapp.bridge.VideoKind

private const val FRAME_RATE = 30
private const val BITS_PER_PIXEL = 4
private const val KEY_FRAME_EVERY_S = 2
private const val REPEAT_AFTER_US = 100_000L
private const val KEY_REQUEST_GAP_MS = 500L

/** Key frames carry SPS/PPS so the receiver can start at any of them. */
class H264Encoder(
    private val bridge: ContinueCoreBridge,
    private val kind: VideoKind,
    size: Size,
    handler: Handler,
    private val onError: () -> Unit,
) {
    /** Sent with each key frame. */
    @Volatile var rotation = 0

    private val codec = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_VIDEO_AVC)
    private var parameterSets = ByteArray(0)
    private var lastKeyRequest = 0L
    val surface: Surface

    init {
        codec.setCallback(Frames(), handler)
        codec.configure(format(size), null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
        surface = codec.createInputSurface()
        codec.start()
    }

    fun requestKeyFrame() {
        val now = SystemClock.elapsedRealtime()
        if (now - lastKeyRequest < KEY_REQUEST_GAP_MS) return
        lastKeyRequest = now
        codec.setParameters(Bundle().apply { putInt(MediaCodec.PARAMETER_KEY_REQUEST_SYNC_FRAME, 0) })
    }

    fun release() {
        runCatching { codec.stop() }
        codec.release()
        surface.release()
    }

    private inner class Frames : MediaCodec.Callback() {
        override fun onOutputBufferAvailable(
            codec: MediaCodec,
            index: Int,
            info: MediaCodec.BufferInfo,
        ) {
            val buffer = codec.getOutputBuffer(index)
            if (buffer != null && info.size > 0) {
                val bytes = ByteArray(info.size)
                buffer.position(info.offset)
                buffer.get(bytes)
                if (info.flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG != 0) {
                    parameterSets = bytes
                } else {
                    val key = info.flags and MediaCodec.BUFFER_FLAG_KEY_FRAME != 0
                    val frame = if (key) parameterSets + bytes else bytes
                    if (!bridge.pushVideoFrame(kind, frame, key, rotation)) requestKeyFrame()
                }
            }
            codec.releaseOutputBuffer(index, false)
        }

        override fun onInputBufferAvailable(
            codec: MediaCodec,
            index: Int,
        ) = Unit

        override fun onOutputFormatChanged(
            codec: MediaCodec,
            format: MediaFormat,
        ) = Unit

        override fun onError(
            codec: MediaCodec,
            e: MediaCodec.CodecException,
        ) = onError()
    }

    private companion object {
        fun format(size: Size) =
            MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, size.width, size.height).apply {
                setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
                setInteger(MediaFormat.KEY_BIT_RATE, size.width * size.height * BITS_PER_PIXEL)
                setInteger(MediaFormat.KEY_FRAME_RATE, FRAME_RATE)
                setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, KEY_FRAME_EVERY_S)
                // A still picture produces no frames otherwise, and the computer would wait.
                setLong(MediaFormat.KEY_REPEAT_PREVIOUS_FRAME_AFTER, REPEAT_AFTER_US)
            }
    }
}
