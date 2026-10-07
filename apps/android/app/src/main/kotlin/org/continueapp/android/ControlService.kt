package org.continueapp.android

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.PixelFormat
import android.graphics.PointF
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.provider.Settings
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import org.continueapp.bridge.PointerInput
import org.continueapp.bridge.PointerTarget
import org.continueapp.bridge.ScreenInput

private const val SHORTEST_STROKE_MS = 1L
private const val LONGEST_STROKE_MS = 2_000L

// Phone pixels per computer pixel: phones pack more pixels into each inch.
private const val POINTER_SPEED = 1.5f
private const val POINTER_DP = 18
private const val SCROLL_STEP_DP = 60
private const val SCROLL_MS = 120L

/** Performs the computer's input through accessibility gestures, the only route without system privileges. */
@Suppress("TooManyFunctions")
class ControlService : AccessibilityService() {
    override fun onServiceConnected() {
        current = this
    }

    override fun onUnbind(intent: Intent?): Boolean {
        hidePointer()
        current = null
        return super.onUnbind(intent)
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) = Unit

    override fun onInterrupt() = Unit

    private var finger: GestureDescription.StrokeDescription? = null
    private var fingerAt = PointF()
    private var fingerTime = 0L

    private fun perform(input: ScreenInput) {
        when (input) {
            is ScreenInput.Touch -> touch(input.action, onScreen(input.x, input.y))
            is ScreenInput.Swipe -> {
                val path = line(onScreen(input.fromX, input.fromY), onScreen(input.toX, input.toY))
                val duration = input.durationMs.coerceIn(SHORTEST_STROKE_MS, LONGEST_STROKE_MS)
                dispatch(GestureDescription.StrokeDescription(path, 0, duration))
            }
            ScreenInput.Back -> performGlobalAction(GLOBAL_ACTION_BACK)
            ScreenInput.Home -> performGlobalAction(GLOBAL_ACTION_HOME)
            ScreenInput.Recents -> performGlobalAction(GLOBAL_ACTION_RECENTS)
            is ScreenInput.Text -> editFocused { it + input.text }
            ScreenInput.Backspace -> editFocused { it.dropLast(1) }
            ScreenInput.Enter ->
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    focused()?.performAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_IME_ENTER.id)
                }
        }
    }

    private var dot: View? = null
    private var pointerAt = PointF()
    private var enteredLeft = true
    private var pressed = false

    /** Set once the pointer has gone back, so it isn't handed back twice. */
    private var leaving = false

    private fun showPointer(
        y: Float,
        fromLeft: Boolean,
    ) {
        val screen = screenSize(this)
        pointerAt = PointF(if (fromLeft) 0f else screen.width - 1f, y.coerceIn(0f, 1f) * screen.height)
        enteredLeft = fromLeft
        pressed = false
        leaving = false
        if (dot == null) {
            val size = (POINTER_DP * resources.displayMetrics.density).toInt()
            val flags =
                WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE or
                    WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN or WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS
            val type = WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY
            val params =
                WindowManager.LayoutParams(size, size, type, flags, PixelFormat.TRANSLUCENT).apply {
                    gravity = Gravity.TOP or Gravity.START
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                        layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
                    }
                }
            dot = PointerDot(this).also { getSystemService(WindowManager::class.java).addView(it, params) }
        }
        placePointer()
    }

    private fun placePointer() {
        val view = dot ?: return
        val params = view.layoutParams as WindowManager.LayoutParams
        params.x = (pointerAt.x - params.width / 2).toInt()
        params.y = (pointerAt.y - params.height / 2).toInt()
        getSystemService(WindowManager::class.java).updateViewLayout(view, params)
    }

    private fun hidePointer() {
        if (pressed) press(false)
        dot?.let { getSystemService(WindowManager::class.java).removeView(it) }
        dot = null
    }

    private fun pointer(input: PointerInput) {
        if (dot == null || leaving) return
        when (input) {
            is PointerInput.Move -> movePointer(input.dx * POINTER_SPEED, input.dy * POINTER_SPEED)
            is PointerInput.Press -> press(input.down)
            // The computer's other button means Back, as it does when the phone's screen is in a window.
            is PointerInput.PressSecondary -> if (input.down) performGlobalAction(GLOBAL_ACTION_BACK)
            is PointerInput.Scroll -> scroll(input.notches)
            is PointerInput.Screen -> perform(input.input)
        }
    }

    private fun movePointer(
        dx: Float,
        dy: Float,
    ) {
        val screen = screenSize(this)
        val x = pointerAt.x + dx
        val y = (pointerAt.y + dy).coerceIn(0f, screen.height - 1f)
        val outside = if (enteredLeft) x < 0 else x >= screen.width
        if (outside && !pressed) {
            leaving = true
            (application as ContinueApplication).coreBridge.pointerLeft(y / screen.height)
            return
        }
        pointerAt = PointF(x.coerceIn(0f, screen.width - 1f), y)
        if (pressed) touch(ScreenInput.Action.MOVE, pointerAt)
        placePointer()
    }

    private fun press(down: Boolean) {
        if (down == pressed) return
        pressed = down
        touch(if (down) ScreenInput.Action.DOWN else ScreenInput.Action.UP, pointerAt)
    }

    /** The wheel turning up drags the page down, as a finger would. */
    private fun scroll(notches: Float) {
        val distance = notches * SCROLL_STEP_DP * resources.displayMetrics.density
        val to = PointF(pointerAt.x, (pointerAt.y + distance).coerceIn(0f, screenSize(this).height - 1f))
        dispatch(GestureDescription.StrokeDescription(line(pointerAt, to), 0, SCROLL_MS))
    }

    private fun touch(
        action: ScreenInput.Action,
        at: PointF,
    ) {
        val now = SystemClock.uptimeMillis()
        val elapsed = (now - fingerTime).coerceIn(SHORTEST_STROKE_MS, LONGEST_STROKE_MS)
        val stroke =
            when (action) {
                ScreenInput.Action.DOWN -> GestureDescription.StrokeDescription(line(at, at), 0, 1, true)
                else -> {
                    val down = finger ?: return
                    down.continueStroke(line(fingerAt, at), 0, elapsed, action == ScreenInput.Action.MOVE)
                }
            }
        finger = stroke.takeIf { action != ScreenInput.Action.UP }
        fingerAt = at
        fingerTime = now
        dispatch(stroke)
    }

    private fun onScreen(
        x: Float,
        y: Float,
    ): PointF {
        val screen = screenSize(this)
        return PointF(x.coerceIn(0f, 1f) * screen.width, y.coerceIn(0f, 1f) * screen.height)
    }

    private fun line(
        from: PointF,
        to: PointF,
    ) = Path().apply {
        moveTo(from.x, from.y)
        lineTo(to.x, to.y)
    }

    private fun dispatch(stroke: GestureDescription.StrokeDescription) {
        dispatchGesture(GestureDescription.Builder().addStroke(stroke).build(), null, null)
    }

    private fun focused(): AccessibilityNodeInfo? = rootInActiveWindow?.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)

    private fun editFocused(change: (String) -> String) {
        val field = focused() ?: return
        val current = if (field.isShowingHintText) "" else field.text?.toString().orEmpty()
        val text =
            Bundle().apply {
                putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, change(current))
            }
        field.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, text)
    }

    companion object {
        @Volatile private var current: ControlService? = null
        private val main = Handler(Looper.getMainLooper())

        fun act(input: ScreenInput) {
            main.post { current?.perform(input) }
        }

        /** The computer's pointer, drawn and driven by the service while it's on. */
        val pointerTarget =
            object : PointerTarget {
                override fun start(
                    y: Float,
                    fromLeft: Boolean,
                ): Boolean {
                    if (current == null) return false
                    main.post { current?.showPointer(y, fromLeft) }
                    return true
                }

                override fun input(input: PointerInput) {
                    main.post { current?.pointer(input) }
                }

                override fun stop() {
                    main.post { current?.hidePointer() }
                }
            }

        fun isOn(context: Context): Boolean {
            val ours = ComponentName(context, ControlService::class.java).flattenToString()
            val enabled =
                Settings.Secure.getString(context.contentResolver, Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES)
            return enabled?.split(':')?.any { it.equals(ours, ignoreCase = true) } == true
        }
    }
}

private class PointerDot(
    context: Context,
) : View(context) {
    private val outline = 2 * context.resources.displayMetrics.density
    private val fill = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.WHITE }
    private val edge =
        Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.BLACK
            style = Paint.Style.STROKE
            strokeWidth = outline
        }

    override fun onDraw(canvas: Canvas) {
        val middle = width / 2f
        canvas.drawCircle(middle, middle, middle - outline, fill)
        canvas.drawCircle(middle, middle, middle - outline, edge)
    }
}
