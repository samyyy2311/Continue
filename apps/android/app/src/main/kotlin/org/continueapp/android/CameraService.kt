package org.continueapp.android

import android.annotation.SuppressLint
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.Rect
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CaptureRequest
import android.hardware.camera2.TotalCaptureResult
import android.hardware.display.DisplayManager
import android.media.MediaCodec
import android.os.Handler
import android.os.HandlerThread
import android.os.SystemClock
import android.util.Range
import android.util.Size
import android.view.Display
import androidx.core.app.ServiceCompat
import org.continueapp.bridge.VideoKind
import org.continueapp.bridge.VideoSize

private const val EXTRA_FRONT = "front"
private const val ACTION_STOP = "stop"
private const val FPS = 30
private const val DEGREES = 360
private const val QUARTER_TURN = 90

/** FRAMING_EASE is how far the crop moves towards the face per update, 0 to 1. */
private const val FRAMING_ZOOM = 1.6f
private const val FRAMING_EASE = 0.15f
private const val FRAMING_EVERY_MS = 100L

class CameraService : Service() {
    private val thread = HandlerThread("camera").apply { start() }
    private val handler = Handler(thread.looper)
    private var camera: CameraDevice? = null
    private var session: CameraCaptureSession? = null
    private var encoder: H264Encoder? = null
    private var maxSize = DEFAULT_MAX_SIZE
    private var front = true
    private var framing = false
    private var focus: Pair<Float, Float>? = null
    private var lastFraming = 0L

    private val app get() = application as ContinueApplication

    override fun onBind(intent: Intent?) = null

    override fun onCreate() {
        super.onCreate()
        current = this
    }

    override fun onStartCommand(
        intent: Intent?,
        flags: Int,
        startId: Int,
    ): Int {
        if (intent?.action == ACTION_STOP) {
            app.coreBridge.endVideo(VideoKind.CAMERA)
            stopSelf()
            return START_NOT_STICKY
        }
        val type = ServiceInfo.FOREGROUND_SERVICE_TYPE_CAMERA
        val notification = sharingNotification(this, "Your computer is using the camera", stopIntent(this))
        ServiceCompat.startForeground(this, CAMERA_NOTIFICATION_ID, notification, type)
        maxSize = intent?.getIntExtra(EXTRA_MAX_SIZE, DEFAULT_MAX_SIZE) ?: DEFAULT_MAX_SIZE
        handler.post { open(intent?.getBooleanExtra(EXTRA_FRONT, true) ?: true) }
        return START_NOT_STICKY
    }

    fun useFront(front: Boolean) = handler.post { if (front != this.front) open(front) }

    fun frame(on: Boolean) =
        handler.post {
            framing = on
            focus = null
            repeat()
        }

    @SuppressLint("MissingPermission") // CameraStartActivity asks first.
    private fun open(front: Boolean) {
        close()
        this.front = front
        val cameras = getSystemService(CameraManager::class.java)
        val facing = if (front) CameraCharacteristics.LENS_FACING_FRONT else CameraCharacteristics.LENS_FACING_BACK
        val id =
            cameras.cameraIdList.firstOrNull {
                cameras.getCameraCharacteristics(it).get(CameraCharacteristics.LENS_FACING) == facing
            }
        if (id == null) {
            app.phoneCamera.answered(null)
            return
        }
        val about = cameras.getCameraCharacteristics(id)
        val size = cameraSize(about, maxSize)
        val encoder = H264Encoder(app.coreBridge, VideoKind.CAMERA, size, handler) { stopSelf() }
        encoder.rotation = rotation(about)
        this.encoder = encoder
        cameras.openCamera(
            id,
            object : CameraDevice.StateCallback() {
                override fun onOpened(device: CameraDevice) {
                    camera = device
                    startSession(device, encoder)
                    app.phoneCamera.answered(VideoSize(size.width, size.height, encoder.rotation))
                }

                override fun onDisconnected(device: CameraDevice) = stopSelf()

                override fun onError(
                    device: CameraDevice,
                    error: Int,
                ) {
                    app.phoneCamera.answered(null)
                    stopSelf()
                }
            },
            handler,
        )
    }

    private fun startSession(
        device: CameraDevice,
        encoder: H264Encoder,
    ) {
        @Suppress("DEPRECATION") // The newer form needs API 28.
        device.createCaptureSession(
            listOf(encoder.surface),
            object : CameraCaptureSession.StateCallback() {
                override fun onConfigured(configured: CameraCaptureSession) {
                    session = configured
                    repeat()
                    encoder.requestKeyFrame()
                }

                override fun onConfigureFailed(failed: CameraCaptureSession) = stopSelf()
            },
            handler,
        )
    }

    private fun repeat(crop: Rect? = null) {
        val device = camera ?: return
        val session = session ?: return
        val encoder = encoder ?: return
        val request =
            device.createCaptureRequest(CameraDevice.TEMPLATE_RECORD).apply {
                addTarget(encoder.surface)
                set(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, Range(FPS, FPS))
                if (framing) {
                    set(CaptureRequest.STATISTICS_FACE_DETECT_MODE, CaptureRequest.STATISTICS_FACE_DETECT_MODE_SIMPLE)
                }
                crop?.let { set(CaptureRequest.SCALER_CROP_REGION, it) }
            }
        session.setRepeatingRequest(request.build(), if (framing) Framing() else null, handler)
    }

    /** Follows the largest face, a few updates a second. */
    private inner class Framing : CameraCaptureSession.CaptureCallback() {
        override fun onCaptureCompleted(
            session: CameraCaptureSession,
            request: CaptureRequest,
            result: TotalCaptureResult,
        ) {
            val now = SystemClock.elapsedRealtime()
            if (now - lastFraming < FRAMING_EVERY_MS) return
            lastFraming = now
            val face = result.get(TotalCaptureResult.STATISTICS_FACES)?.maxByOrNull { it.bounds.width() } ?: return
            val sensor =
                camera?.id?.let {
                    getSystemService(CameraManager::class.java)
                        .getCameraCharacteristics(it)
                        .get(CameraCharacteristics.SENSOR_INFO_ACTIVE_ARRAY_SIZE)
                } ?: return
            val (faceX, faceY) = face.bounds.exactCenterX() to face.bounds.exactCenterY()
            val (x, y) = focus ?: (faceX to faceY)
            val eased = x + (faceX - x) * FRAMING_EASE to y + (faceY - y) * FRAMING_EASE
            focus = eased
            repeat(crop(sensor, eased))
        }
    }

    private fun close() {
        session?.close()
        camera?.close()
        encoder?.release()
        session = null
        camera = null
        encoder = null
    }

    override fun onDestroy() {
        handler.post { close() }
        thread.quitSafely()
        current = null
        super.onDestroy()
    }

    /** Clockwise degrees that show frames upright for the current display rotation. */
    private fun rotation(about: CameraCharacteristics): Int {
        val sensor = about.get(CameraCharacteristics.SENSOR_ORIENTATION) ?: 0
        val display = getSystemService(DisplayManager::class.java).getDisplay(Display.DEFAULT_DISPLAY)
        val held = display.rotation * QUARTER_TURN
        return if (front) (sensor + held) % DEGREES else (sensor - held + DEGREES) % DEGREES
    }

    companion object {
        const val DEFAULT_MAX_SIZE = 1920

        @Volatile var current: CameraService? = null
            private set

        fun start(
            context: Context,
            maxSize: Int,
            front: Boolean,
        ): Intent =
            Intent(context, CameraService::class.java)
                .putExtra(EXTRA_MAX_SIZE, maxSize)
                .putExtra(EXTRA_FRONT, front)

        private fun stopIntent(context: Context) = Intent(context, CameraService::class.java).setAction(ACTION_STOP)
    }
}

/** Largest 16:9 recording size that fits [maxSize] on its long side. */
private fun cameraSize(
    about: CameraCharacteristics,
    maxSize: Int,
): Size {
    val sizes = about.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP)?.getOutputSizes(MediaCodec::class.java)
    val fitting = sizes.orEmpty().filter { maxOf(it.width, it.height) <= maxSize }
    val wide = fitting.filter { it.width * 9 == it.height * 16 }
    return (wide.ifEmpty { fitting }).maxByOrNull { it.width * it.height } ?: Size(1280, 720)
}

private fun crop(
    sensor: Rect,
    focus: Pair<Float, Float>,
): Rect {
    val width = (sensor.width() / FRAMING_ZOOM).toInt()
    val height = (sensor.height() / FRAMING_ZOOM).toInt()
    val left = (focus.first - width / 2).toInt().coerceIn(sensor.left, sensor.right - width)
    val top = (focus.second - height / 2).toInt().coerceIn(sensor.top, sensor.bottom - height)
    return Rect(left, top, left + width, top + height)
}
