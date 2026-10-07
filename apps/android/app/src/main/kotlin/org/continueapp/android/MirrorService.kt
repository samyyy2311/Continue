package org.continueapp.android

import android.app.Activity
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.hardware.display.DisplayManager
import android.hardware.display.VirtualDisplay
import android.media.projection.MediaProjection
import android.media.projection.MediaProjectionManager
import android.os.Handler
import android.os.HandlerThread
import androidx.core.app.ServiceCompat
import androidx.core.content.IntentCompat
import org.continueapp.bridge.VideoKind
import org.continueapp.bridge.VideoSize

private const val EXTRA_CONSENT = "consent"
private const val ACTION_STOP = "stop"

class MirrorService : Service() {
    private val thread = HandlerThread("screen").apply { start() }
    private var projection: MediaProjection? = null
    private var display: VirtualDisplay? = null
    private var encoder: H264Encoder? = null
    private var sound: ScreenSound? = null

    override fun onBind(intent: Intent?) = null

    override fun onStartCommand(
        intent: Intent?,
        flags: Int,
        startId: Int,
    ): Int {
        val consent = intent?.let { IntentCompat.getParcelableExtra(it, EXTRA_CONSENT, Intent::class.java) }
        if (intent?.action == ACTION_STOP || consent == null || projection != null) {
            stopSelf()
            return START_NOT_STICKY
        }
        // ServiceCompat leaves the type out on versions without one.
        val type = ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION
        val notification = sharingNotification(this, "Sharing your screen", stopIntent(this))
        ServiceCompat.startForeground(this, SCREEN_NOTIFICATION_ID, notification, type)

        val app = application as ContinueApplication
        val handler = Handler(thread.looper)
        val size = frameSize(screenSize(this), intent.getIntExtra(EXTRA_MAX_SIZE, DEFAULT_MAX_SIZE))
        val projection =
            getSystemService(MediaProjectionManager::class.java).getMediaProjection(Activity.RESULT_OK, consent)
        this.projection = projection
        projection.registerCallback(
            object : MediaProjection.Callback() {
                override fun onStop() {
                    app.coreBridge.endVideo(VideoKind.SCREEN)
                    stopSelf()
                }
            },
            handler,
        )
        val encoder = H264Encoder(app.coreBridge, VideoKind.SCREEN, size, handler) { stopSelf() }
        this.encoder = encoder
        display =
            projection.createVirtualDisplay(
                "Continue",
                size.width,
                size.height,
                resources.displayMetrics.densityDpi,
                DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
                encoder.surface,
                null,
                handler,
            )
        if (canCaptureSound(this)) {
            sound = ScreenSound(projection, app.coreBridge).also { it.start() }
        }
        app.phoneScreen.answered(VideoSize(size.width, size.height))
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        sound?.stop()
        display?.release()
        encoder?.release()
        projection?.stop()
        thread.quitSafely()
        super.onDestroy()
    }

    companion object {
        const val DEFAULT_MAX_SIZE = 1280

        fun start(
            context: Context,
            consent: Intent,
            maxSize: Int,
        ): Intent =
            Intent(context, MirrorService::class.java)
                .putExtra(EXTRA_CONSENT, consent)
                .putExtra(EXTRA_MAX_SIZE, maxSize)

        private fun stopIntent(context: Context) = Intent(context, MirrorService::class.java).setAction(ACTION_STOP)
    }
}
