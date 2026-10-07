package org.continueapp.android

import android.content.ComponentName
import android.graphics.Bitmap
import android.media.AudioManager
import android.media.MediaMetadata
import android.media.session.MediaController
import android.media.session.MediaSessionManager
import android.media.session.PlaybackState
import android.os.Handler
import android.os.Looper
import org.continueapp.bridge.MediaCommand
import org.continueapp.bridge.MediaRemote
import org.continueapp.bridge.NowPlaying
import java.io.ByteArrayOutputStream
import kotlin.concurrent.thread

private const val ART_SIZE = 256
private const val ART_QUALITY = 80

/**
 * Follows what's playing on the phone and controls it for connected computers. Android only shares
 * media sessions with apps that have notification access.
 */
class PhoneMedia(
    private val app: ContinueApplication,
) : MediaRemote {
    @Volatile private var playing: MediaController? = null
    private val main = Handler(Looper.getMainLooper())
    private var watching = false

    private val changes =
        object : MediaController.Callback() {
            override fun onMetadataChanged(metadata: MediaMetadata?) = announce()

            override fun onPlaybackStateChanged(state: PlaybackState?) = announce()
        }

    /** Safe to call again; starts once notification access is granted. */
    fun watch() {
        if (watching || !hasNotificationAccess(app)) return
        watching = true
        val sessions = app.getSystemService(MediaSessionManager::class.java)
        val listener = ComponentName(app, ContinueNotificationListener::class.java)
        sessions.addOnActiveSessionsChangedListener({ follow(it.orEmpty()) }, listener, main)
        follow(sessions.getActiveSessions(listener))
    }

    private fun follow(controllers: List<MediaController>) {
        playing?.unregisterCallback(changes)
        playing = controllers.firstOrNull()?.also { it.registerCallback(changes, main) }
        announce()
    }

    private fun announce() {
        val controller = playing
        val metadata = controller?.metadata
        val state = controller?.playbackState
        val now =
            NowPlaying(
                title = metadata?.getString(MediaMetadata.METADATA_KEY_TITLE).orEmpty(),
                artist = metadata?.getString(MediaMetadata.METADATA_KEY_ARTIST).orEmpty(),
                app = controller?.packageName?.let { appLabel(app, it) }.orEmpty(),
                playing = state?.state == PlaybackState.STATE_PLAYING,
                durationMs = metadata?.getLong(MediaMetadata.METADATA_KEY_DURATION) ?: 0,
                positionMs = state?.position ?: 0,
                art = metadata?.let(::art),
            )
        thread { app.coreBridge.announceNowPlaying(now) }
    }

    override fun command(command: MediaCommand): Boolean {
        val controller = playing ?: return false
        val controls = controller.transportControls
        val isPlaying = controller.playbackState?.state == PlaybackState.STATE_PLAYING
        when (command) {
            MediaCommand.PLAY_PAUSE -> if (isPlaying) controls.pause() else controls.play()
            MediaCommand.NEXT -> controls.skipToNext()
            MediaCommand.PREVIOUS -> controls.skipToPrevious()
            MediaCommand.VOLUME_UP -> controller.adjustVolume(AudioManager.ADJUST_RAISE, 0)
            MediaCommand.VOLUME_DOWN -> controller.adjustVolume(AudioManager.ADJUST_LOWER, 0)
        }
        return true
    }

    private fun art(metadata: MediaMetadata): ByteArray? {
        val cover =
            metadata.getBitmap(MediaMetadata.METADATA_KEY_ALBUM_ART)
                ?: metadata.getBitmap(MediaMetadata.METADATA_KEY_ART)
                ?: return null
        val small =
            Bitmap.createScaledBitmap(
                cover,
                ART_SIZE,
                ART_SIZE * cover.height / cover.width.coerceAtLeast(1),
                true,
            )
        return ByteArrayOutputStream().use {
            small.compress(Bitmap.CompressFormat.JPEG, ART_QUALITY, it)
            it.toByteArray()
        }
    }
}
