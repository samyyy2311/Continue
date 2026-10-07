package org.continueapp.android

import android.content.Context
import android.content.Intent
import android.hardware.display.DisplayManager
import android.util.DisplayMetrics
import android.util.Size
import android.view.Display
import org.continueapp.bridge.ScreenInput
import org.continueapp.bridge.ScreenShare
import org.continueapp.bridge.VideoSize
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit
import java.util.concurrent.TimeoutException

const val EXTRA_MAX_SIZE = "max_size"
private const val CONSENT_WAIT_S = 60L
private const val ALIGN = 16

class PhoneScreen(
    private val app: ContinueApplication,
) : ScreenShare {
    @Volatile private var waiting: CompletableFuture<VideoSize?>? = null

    override fun start(maxSize: Int): VideoSize? {
        val answer = CompletableFuture<VideoSize?>()
        waiting = answer
        val consent =
            Intent(app, MirrorConsentActivity::class.java)
                .putExtra(EXTRA_MAX_SIZE, maxSize)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        if (app.onScreen) {
            app.startActivity(consent)
        } else {
            notifyArrival(
                app,
                "Share your screen?",
                "Your computer wants to show this phone's screen. Tap to start.",
                consent,
            )
        }
        return try {
            answer.get(CONSENT_WAIT_S, TimeUnit.SECONDS)
        } catch (_: TimeoutException) {
            null
        } finally {
            waiting = null
        }
    }

    fun answered(size: VideoSize?) {
        val answer = waiting
        if (answer == null && size != null) stop() else answer?.complete(size)
    }

    override fun input(input: ScreenInput) = ControlService.act(input)

    override fun stop() {
        app.stopService(Intent(app, MirrorService::class.java))
    }
}

fun screenSize(context: Context): Size {
    val display = context.getSystemService(DisplayManager::class.java).getDisplay(Display.DEFAULT_DISPLAY)
    val metrics = DisplayMetrics()
    @Suppress("DEPRECATION")
    display.getRealMetrics(metrics)
    return Size(metrics.widthPixels, metrics.heightPixels)
}

/** Fits [maxSize] on the long side, rounded down to multiples of 16 for the encoder. */
fun frameSize(
    screen: Size,
    maxSize: Int,
): Size {
    val scale = minOf(1f, maxSize.toFloat() / maxOf(screen.width, screen.height))

    fun fit(side: Int) = (side * scale).toInt() / ALIGN * ALIGN
    return Size(fit(screen.width), fit(screen.height))
}
