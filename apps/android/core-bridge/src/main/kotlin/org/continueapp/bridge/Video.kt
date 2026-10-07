package org.continueapp.bridge

import org.continueapp.bridge.ffi.CameraControlFfi
import org.continueapp.bridge.ffi.CameraSourceFfi
import org.continueapp.bridge.ffi.ScreenInputFfi
import org.continueapp.bridge.ffi.ScreenSourceFfi
import org.continueapp.bridge.ffi.TouchActionFfi
import org.continueapp.bridge.ffi.VideoStartFfi

enum class VideoKind { SCREEN, CAMERA }

/** A video stream's size, and the degrees to turn it clockwise to show it upright. */
data class VideoSize(
    val width: Int,
    val height: Int,
    val rotation: Int = 0,
)

/** Something a computer watching the screen asks for. Positions are fractions of the screen. */
sealed interface ScreenInput {
    enum class Action { DOWN, MOVE, UP }

    data class Touch(
        val action: Action,
        val x: Float,
        val y: Float,
    ) : ScreenInput

    data class Swipe(
        val fromX: Float,
        val fromY: Float,
        val toX: Float,
        val toY: Float,
        val durationMs: Long,
    ) : ScreenInput

    data object Back : ScreenInput

    data object Home : ScreenInput

    data object Recents : ScreenInput

    data class Text(
        val text: String,
    ) : ScreenInput

    data object Backspace : ScreenInput

    data object Enter : ScreenInput
}

interface ScreenShare {
    /** May block until the user answers Android's prompt. Null if declined. */
    fun start(maxSize: Int): VideoSize?

    fun input(input: ScreenInput)

    fun stop()
}

internal class ScreenShareFfi(
    private val share: ScreenShare,
) : ScreenSourceFfi {
    override fun start(maxSize: UInt): VideoStartFfi? = share.start(maxSize.toInt())?.toFfi()

    override fun input(input: ScreenInputFfi) = share.input(input.toScreenInput())

    override fun stop() = share.stop()
}

internal fun ScreenInput.toFfi(): ScreenInputFfi =
    when (this) {
        is ScreenInput.Touch -> ScreenInputFfi.Touch(TouchActionFfi.entries[action.ordinal], x, y)
        is ScreenInput.Swipe -> ScreenInputFfi.Swipe(fromX, fromY, toX, toY, durationMs.toUInt())
        ScreenInput.Back -> ScreenInputFfi.Back
        ScreenInput.Home -> ScreenInputFfi.Home
        ScreenInput.Recents -> ScreenInputFfi.Recents
        is ScreenInput.Text -> ScreenInputFfi.Text(text)
        ScreenInput.Backspace -> ScreenInputFfi.Backspace
        ScreenInput.Enter -> ScreenInputFfi.Enter
    }

internal fun ScreenInputFfi.toScreenInput(): ScreenInput =
    when (this) {
        is ScreenInputFfi.Touch -> ScreenInput.Touch(ScreenInput.Action.entries[action.ordinal], x, y)
        is ScreenInputFfi.Swipe -> ScreenInput.Swipe(fromX, fromY, toX, toY, durationMs.toLong())
        ScreenInputFfi.Back -> ScreenInput.Back
        ScreenInputFfi.Home -> ScreenInput.Home
        ScreenInputFfi.Recents -> ScreenInput.Recents
        is ScreenInputFfi.Text -> ScreenInput.Text(text)
        ScreenInputFfi.Backspace -> ScreenInput.Backspace
        ScreenInputFfi.Enter -> ScreenInput.Enter
    }

sealed interface CameraControl {
    data class Front(
        val front: Boolean,
    ) : CameraControl

    /** Keeps the person in the middle of the picture. */
    data class Framing(
        val on: Boolean,
    ) : CameraControl
}

interface CameraShare {
    fun start(
        maxSize: Int,
        front: Boolean,
    ): VideoSize?

    fun control(control: CameraControl)

    fun stop()
}

internal class CameraShareFfi(
    private val share: CameraShare,
) : CameraSourceFfi {
    override fun start(
        maxSize: UInt,
        front: Boolean,
    ): VideoStartFfi? = share.start(maxSize.toInt(), front)?.toFfi()

    override fun control(control: CameraControlFfi) =
        share.control(
            when (control) {
                is CameraControlFfi.Front -> CameraControl.Front(control.front)
                is CameraControlFfi.Framing -> CameraControl.Framing(control.on)
            },
        )

    override fun stop() = share.stop()
}

private fun VideoSize.toFfi() = VideoStartFfi(width.toUInt(), height.toUInt(), rotation.toUInt())
