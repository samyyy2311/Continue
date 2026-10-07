package org.continueapp.bridge

import org.continueapp.bridge.ffi.PointerInputFfi
import org.continueapp.bridge.ffi.PointerTargetFfi

/** The computer's mouse and keyboard, while its pointer is on the phone. */
sealed interface PointerInput {
    /** In the computer's pixels. */
    data class Move(
        val dx: Float,
        val dy: Float,
    ) : PointerInput

    data class Press(
        val down: Boolean,
    ) : PointerInput

    /** The other button, for a menu. */
    data class PressSecondary(
        val down: Boolean,
    ) : PointerInput

    /** Wheel notches; positive scrolls up. */
    data class Scroll(
        val notches: Float,
    ) : PointerInput

    /** Typing and the phone's own buttons. */
    data class Screen(
        val input: ScreenInput,
    ) : PointerInput
}

interface PointerTarget {
    /**
     * Shows the pointer, coming in at [y] (0 top, 1 bottom) by the left or right edge. False if
     * input can't be taken. When it goes back over that edge, call [ContinueCoreBridge.pointerLeft].
     */
    fun start(
        y: Float,
        fromLeft: Boolean,
    ): Boolean

    fun input(input: PointerInput)

    fun stop()
}

internal class PointerTargetFfiAdapter(
    private val target: PointerTarget,
) : PointerTargetFfi {
    override fun start(
        y: Float,
        fromLeft: Boolean,
    ) = target.start(y, fromLeft)

    override fun input(input: PointerInputFfi) =
        target.input(
            when (input) {
                is PointerInputFfi.Move -> PointerInput.Move(input.dx, input.dy)
                is PointerInputFfi.Press -> PointerInput.Press(input.down)
                is PointerInputFfi.PressSecondary -> PointerInput.PressSecondary(input.down)
                is PointerInputFfi.Scroll -> PointerInput.Scroll(input.notches)
                is PointerInputFfi.Screen -> PointerInput.Screen(input.input.toScreenInput())
            },
        )

    override fun stop() = target.stop()
}

internal fun PointerInput.toFfi(): PointerInputFfi =
    when (this) {
        is PointerInput.Move -> PointerInputFfi.Move(dx, dy)
        is PointerInput.Press -> PointerInputFfi.Press(down)
        is PointerInput.PressSecondary -> PointerInputFfi.PressSecondary(down)
        is PointerInput.Scroll -> PointerInputFfi.Scroll(notches)
        is PointerInput.Screen -> PointerInputFfi.Screen(input.toFfi())
    }
