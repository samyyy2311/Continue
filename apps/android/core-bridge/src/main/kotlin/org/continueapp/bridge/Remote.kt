package org.continueapp.bridge

import org.continueapp.bridge.ffi.ComputerActionFfi
import org.continueapp.bridge.ffi.MediaCommandFfi
import org.continueapp.bridge.ffi.MediaControlFfi
import org.continueapp.bridge.ffi.RingerFfi

/** Title is empty when nothing is playing. */
data class NowPlaying(
    val title: String,
    val artist: String,
    val app: String,
    val playing: Boolean,
    val durationMs: Long,
    val positionMs: Long,
    /** JPEG, when the app provides cover art. */
    val art: ByteArray?,
)

enum class MediaCommand { PLAY_PAUSE, NEXT, PREVIOUS, VOLUME_UP, VOLUME_DOWN }

/** Pinned text, kept on every paired device. */
data class Snippet(
    val id: String,
    val text: String,
)

/** Something to have a paired computer do. */
sealed interface ComputerAction {
    data object Lock : ComputerAction

    data object Sleep : ComputerAction

    /** Typed into whatever has focus there. */
    data class TypeText(
        val text: String,
    ) : ComputerAction

    /** Opened in its browser; only http and https links are. */
    data class OpenLink(
        val url: String,
    ) : ComputerAction
}

internal fun ComputerAction.toFfi(): ComputerActionFfi =
    when (this) {
        ComputerAction.Lock -> ComputerActionFfi.Lock
        ComputerAction.Sleep -> ComputerActionFfi.Sleep
        is ComputerAction.TypeText -> ComputerActionFfi.TypeText(text)
        is ComputerAction.OpenLink -> ComputerActionFfi.OpenLink(url)
    }

fun interface Ringer {
    /** Rings at full volume, even on silent, or stops. */
    fun ring(on: Boolean): Boolean
}

internal class RingerFfiAdapter(
    private val ringer: Ringer,
) : RingerFfi {
    override fun ring(on: Boolean) = ringer.ring(on)
}

fun interface MediaRemote {
    /** False when nothing is playing to control. */
    fun command(command: MediaCommand): Boolean
}

internal class MediaRemoteFfi(
    private val remote: MediaRemote,
) : MediaControlFfi {
    override fun command(command: MediaCommandFfi) = remote.command(MediaCommand.entries[command.ordinal])
}
