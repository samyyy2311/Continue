package org.continueapp.android

import android.content.Intent
import org.continueapp.bridge.CameraControl
import org.continueapp.bridge.CameraShare
import org.continueapp.bridge.VideoSize
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit
import java.util.concurrent.TimeoutException

const val EXTRA_FRONT_CAMERA = "front_camera"
private const val START_WAIT_S = 60L

/** Android only starts the camera while the app is visible, so this goes through [CameraStartActivity]. */
class PhoneCamera(
    private val app: ContinueApplication,
) : CameraShare {
    @Volatile private var waiting: CompletableFuture<VideoSize?>? = null

    override fun start(
        maxSize: Int,
        front: Boolean,
    ): VideoSize? {
        val answer = CompletableFuture<VideoSize?>()
        waiting = answer
        val starting =
            Intent(app, CameraStartActivity::class.java)
                .putExtra(EXTRA_MAX_SIZE, maxSize)
                .putExtra(EXTRA_FRONT_CAMERA, front)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        if (app.onScreen) {
            app.startActivity(starting)
        } else {
            notifyArrival(
                app,
                "Use this phone as a webcam?",
                "Your computer wants to use the camera. Tap to start.",
                starting,
            )
        }
        return try {
            answer.get(START_WAIT_S, TimeUnit.SECONDS)
        } catch (_: TimeoutException) {
            null
        } finally {
            waiting = null
        }
    }

    fun answered(size: VideoSize?) {
        waiting?.complete(size)
    }

    override fun control(control: CameraControl) {
        val camera = CameraService.current ?: return
        when (control) {
            is CameraControl.Front -> camera.useFront(control.front)
            is CameraControl.Framing -> camera.frame(control.on)
        }
    }

    override fun stop() {
        app.stopService(Intent(app, CameraService::class.java))
    }
}
