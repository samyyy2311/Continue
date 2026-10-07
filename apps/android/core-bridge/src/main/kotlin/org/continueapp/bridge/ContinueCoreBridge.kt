package org.continueapp.bridge

import android.content.Context
import android.os.Environment
import org.continueapp.bridge.ffi.CallFfi
import org.continueapp.bridge.ffi.CallStateFfi
import org.continueapp.bridge.ffi.ContinueFfiException
import org.continueapp.bridge.ffi.DeviceStatusFfi
import org.continueapp.bridge.ffi.HistoryEntryFfi
import org.continueapp.bridge.ffi.NotificationActionFfi
import org.continueapp.bridge.ffi.NotificationEventFfi
import org.continueapp.bridge.ffi.NotificationFfi
import org.continueapp.bridge.ffi.NowPlayingFfi
import org.continueapp.bridge.ffi.ReceivedFfi
import org.continueapp.bridge.ffi.TrustedPeerFfi
import org.continueapp.bridge.ffi.VideoKindFfi
import java.io.File
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit.MILLISECONDS
import org.continueapp.bridge.ffi.actOnComputer as coreActOnComputer
import org.continueapp.bridge.ffi.announceCall as coreAnnounceCall
import org.continueapp.bridge.ffi.announceMessagesChanged as coreAnnounceMessagesChanged
import org.continueapp.bridge.ffi.announceNowPlaying as coreAnnounceNowPlaying
import org.continueapp.bridge.ffi.announcePhoto as coreAnnouncePhoto
import org.continueapp.bridge.ffi.cancelIncoming as coreCancelIncoming
import org.continueapp.bridge.ffi.clearHistory as coreClearHistory
import org.continueapp.bridge.ffi.confirmNearbyPairing as coreConfirmNearbyPairing
import org.continueapp.bridge.ffi.connectToPeer as coreConnectToPeer
import org.continueapp.bridge.ffi.disconnect as coreDisconnect
import org.continueapp.bridge.ffi.endVideo as coreEndVideo
import org.continueapp.bridge.ffi.forwardNotification as coreForwardNotification
import org.continueapp.bridge.ffi.forwardNotificationRemoved as coreForwardNotificationRemoved
import org.continueapp.bridge.ffi.getDeviceFingerprint as coreGetDeviceFingerprint
import org.continueapp.bridge.ffi.getDeviceSpkiHash as coreGetDeviceSpkiHash
import org.continueapp.bridge.ffi.initCore as coreInitCore
import org.continueapp.bridge.ffi.isAllowed as coreIsAllowed
import org.continueapp.bridge.ffi.isPeerConnected as coreIsPeerConnected
import org.continueapp.bridge.ffi.listHistory as coreListHistory
import org.continueapp.bridge.ffi.listIncoming as coreListIncoming
import org.continueapp.bridge.ffi.listTrustedPeers as coreListTrustedPeers
import org.continueapp.bridge.ffi.nearbyComputers as coreNearbyComputers
import org.continueapp.bridge.ffi.nextNotificationEvent as coreNextNotificationEvent
import org.continueapp.bridge.ffi.nextReceived as coreNextReceived
import org.continueapp.bridge.ffi.pairFromQr as corePairFromQr
import org.continueapp.bridge.ffi.pairNearby as corePairNearby
import org.continueapp.bridge.ffi.pairingWords as corePairingWords
import org.continueapp.bridge.ffi.peerWallpaper as corePeerWallpaper
import org.continueapp.bridge.ffi.pinSnippet as corePinSnippet
import org.continueapp.bridge.ffi.pointerLeft as corePointerLeft
import org.continueapp.bridge.ffi.presenceToken as corePresenceToken
import org.continueapp.bridge.ffi.pushAudio as corePushAudio
import org.continueapp.bridge.ffi.pushVideoFrame as corePushVideoFrame
import org.continueapp.bridge.ffi.reconnect as coreReconnect
import org.continueapp.bridge.ffi.removeTrustedPeer as coreRemoveTrustedPeer
import org.continueapp.bridge.ffi.ringComputer as coreRingComputer
import org.continueapp.bridge.ffi.sendClipboardImage as coreSendClipboardImage
import org.continueapp.bridge.ffi.sendClipboardText as coreSendClipboardText
import org.continueapp.bridge.ffi.sendFile as coreSendFile
import org.continueapp.bridge.ffi.sendFileLater as coreSendFileLater
import org.continueapp.bridge.ffi.sendTextLater as coreSendTextLater
import org.continueapp.bridge.ffi.setAllowed as coreSetAllowed
import org.continueapp.bridge.ffi.setCallControl as coreSetCallControl
import org.continueapp.bridge.ffi.setCameraSource as coreSetCameraSource
import org.continueapp.bridge.ffi.setDeviceName as coreSetDeviceName
import org.continueapp.bridge.ffi.setDeviceStatus as coreSetDeviceStatus
import org.continueapp.bridge.ffi.setHistoryLocation as coreSetHistoryLocation
import org.continueapp.bridge.ffi.setKeyStore as coreSetKeyStore
import org.continueapp.bridge.ffi.setLook as coreSetLook
import org.continueapp.bridge.ffi.setMediaControl as coreSetMediaControl
import org.continueapp.bridge.ffi.setMessageStore as coreSetMessageStore
import org.continueapp.bridge.ffi.setPaused as coreSetPaused
import org.continueapp.bridge.ffi.setPhoneSearch as coreSetPhoneSearch
import org.continueapp.bridge.ffi.setPhotoLibrary as coreSetPhotoLibrary
import org.continueapp.bridge.ffi.setPointerTarget as coreSetPointerTarget
import org.continueapp.bridge.ffi.setRinger as coreSetRinger
import org.continueapp.bridge.ffi.setScreenSource as coreSetScreenSource
import org.continueapp.bridge.ffi.setSharedFolder as coreSetSharedFolder
import org.continueapp.bridge.ffi.snippets as coreSnippets
import org.continueapp.bridge.ffi.startDiscovery as coreStartDiscovery
import org.continueapp.bridge.ffi.stopDiscovery as coreStopDiscovery
import org.continueapp.bridge.ffi.touchpadInput as coreTouchpadInput
import org.continueapp.bridge.ffi.touchpadStart as coreTouchpadStart
import org.continueapp.bridge.ffi.touchpadStop as coreTouchpadStop
import org.continueapp.bridge.ffi.unpinSnippet as coreUnpinSnippet

private const val SECONDS_DIVISOR = 1000L
private const val QR_SUFFIX_LENGTH = 4

@Suppress("TooManyFunctions")
interface ContinueCoreBridge {
    fun initCore(dbPath: String)

    /** The name paired computers see for this phone, from the next connection on. */
    fun setDeviceName(name: String)

    fun setDeviceStatus(status: PhoneStatus)

    /** [color] is 0xRRGGBB; [wallpaper] is a small JPEG when the app can read it. */
    fun setLook(
        color: Int,
        wallpaper: ByteArray?,
    )

    /** JPEG, once the computer has sent one. */
    fun peerWallpaper(peerFingerprint: String): ByteArray?

    /** Announces a camera photo or screenshot taken in the last minute. */
    fun announceNewPhoto()

    fun announceMessagesChanged()

    /** False if nobody is watching or the frame was dropped; the next frame should be a key frame. */
    fun pushVideoFrame(
        kind: VideoKind,
        data: ByteArray,
        key: Boolean,
        rotation: Int,
    ): Boolean

    /** 16-bit stereo PCM at 48 kHz, what's playing while the screen is shared. */
    fun pushAudio(
        kind: VideoKind,
        pcm: ByteArray,
    ): Boolean

    fun endVideo(kind: VideoKind)

    fun announceNowPlaying(playing: NowPlaying)

    fun announceCall(
        state: CallState,
        number: String,
        name: String,
    )

    fun getDeviceFingerprint(): String

    fun getDeviceSpkiHash(): String

    /** Advertises this device's listener and connects to paired devices as they appear. */
    fun startDiscovery(protocolVersion: Long = 1L)

    fun stopDiscovery()

    fun pairFromQr(qrPayload: String): TrustedPeer

    /** Computers showing a pairing code on this network, gathered for [waitMs]. */
    fun nearbyComputers(waitMs: Int): List<NearbyComputer>

    /** Pairs with a computer found nearby; returns six digits to compare with its screen. */
    fun pairNearby(code: String): String

    /** The computer once accepted; null when turned down because the codes differed. */
    fun confirmNearbyPairing(accept: Boolean): TrustedPeer?

    fun listTrustedPeers(): List<TrustedPeer>

    fun removeTrustedPeer(fingerprint: String): Boolean

    /** True unless turned off for this computer. */
    fun isAllowed(
        peerFingerprint: String,
        capabilityId: Int,
    ): Boolean

    fun setAllowed(
        peerFingerprint: String,
        capabilityId: Int,
        allowed: Boolean,
    )

    /** Waits up to [timeoutMs] for the next file or text a paired device sent, or returns null. */
    fun nextReceived(timeoutMs: Long): Received?

    /** Files coming in right now, oldest first. */
    fun listIncoming(): List<IncomingFile>

    /** Stops a file part way. False if it already finished. */
    fun cancelIncoming(transferId: String): Boolean

    /** What this phone sent and received, newest first. */
    fun listHistory(limit: Int): List<HistoryEntry>

    fun clearHistory()

    /** Notes where a received file ended up, from [Received.historyId]. */
    fun setHistoryLocation(
        id: Long,
        location: String,
    )

    fun connectToPeer(
        peerFingerprint: String,
        endpoint: String,
    )

    fun isPeerConnected(peerFingerprint: String): Boolean

    fun disconnect(peerFingerprint: String)

    /** Lets a device disconnected with [disconnect] connect automatically again. */
    fun reconnect(peerFingerprint: String)

    /** Drops every connection and turns new ones away until unpaused. */
    fun setPaused(paused: Boolean)

    /** The computer's pointer went back over the edge it came in by, at [y] (0 top, 1 bottom). */
    fun pointerLeft(y: Float)

    /** Uses the phone as the computer's touchpad. False when the computer can't take input. */
    fun touchpadStart(peerFingerprint: String): Boolean

    /** False once the computer has gone. */
    fun touchpadInput(input: PointerInput): Boolean

    fun touchpadStop()

    /** The four words the computer shows for this pairing too. */
    fun pairingWords(peerFingerprint: String): String

    /** What to broadcast over Bluetooth now so paired computers know the phone is near. */
    fun presenceToken(): ByteArray

    /** Newest first. */
    fun snippets(): List<Snippet>

    /** Pins text on every paired computer too. */
    fun pinSnippet(text: String): Snippet

    fun unpinSnippet(id: String)

    /** Rings the computer so it can be found, or stops it. False if it didn't. */
    fun ringComputer(
        peerFingerprint: String,
        on: Boolean,
    ): Boolean

    /** False if the computer didn't do it, such as with that switched off there. */
    fun actOnComputer(
        peerFingerprint: String,
        action: ComputerAction,
    ): Boolean

    fun sendFile(
        peerFingerprint: String,
        filePath: String,
    ): Long

    /** Kept until the computer connects. The file is moved, so pass a copy. */
    fun sendFileLater(
        peerFingerprint: String,
        filePath: String,
    )

    fun sendTextLater(
        peerFingerprint: String,
        text: String,
    )

    fun sendClipboardImage(
        peerFingerprint: String,
        png: ByteArray,
    )

    fun sendClipboardText(
        peerFingerprint: String,
        text: String,
    )

    /** Shows [notification] on connected computers. Returns straight away. */
    fun forwardNotification(notification: PhoneNotification)

    /** Takes a notification that went away on the phone off connected computers. Returns straight away. */
    fun forwardNotificationRemoved(id: String)

    /** Waits up to [timeoutMs] for a computer to act on one of this phone's notifications, or returns null. */
    fun nextNotificationEvent(timeoutMs: Long): NotificationEvent?

    companion object {
        fun create(
            context: Context,
            screen: ScreenShare,
            camera: CameraShare,
            media: MediaRemote,
            ringer: Ringer,
            pointer: PointerTarget,
        ): ContinueCoreBridge = NativeContinueCoreBridge(context, screen, camera, media, ringer, pointer)

        fun mock(): MockContinueCoreBridge = MockContinueCoreBridge()
    }
}

@Suppress("TooManyFunctions")
class MockContinueCoreBridge : ContinueCoreBridge {
    private var initialized: Boolean = false
    private var isDiscovering: Boolean = false
    private val peers = ConcurrentHashMap<String, TrustedPeer>()
    private val turnedOff = ConcurrentHashMap.newKeySet<String>()
    private val connectedPeers = ConcurrentHashMap<String, String>()
    private val received = LinkedBlockingQueue<Received>()
    private val notificationEvents = LinkedBlockingQueue<NotificationEvent>()

    /** Hands the app a file or text as if a paired device had sent it. */
    fun receive(item: Received) {
        received.put(item)
    }

    override fun initCore(dbPath: String) {
        initialized = true
    }

    var deviceName = ""
        private set

    override fun setDeviceName(name: String) {
        checkInitialized()
        deviceName = name
    }

    override fun setDeviceStatus(status: PhoneStatus) = checkInitialized()

    override fun setLook(
        color: Int,
        wallpaper: ByteArray?,
    ) = checkInitialized()

    override fun peerWallpaper(peerFingerprint: String): ByteArray? = null

    override fun announceNewPhoto() = checkInitialized()

    override fun announceMessagesChanged() = checkInitialized()

    override fun announceNowPlaying(playing: NowPlaying) = checkInitialized()

    override fun announceCall(
        state: CallState,
        number: String,
        name: String,
    ) = checkInitialized()

    override fun pushVideoFrame(
        kind: VideoKind,
        data: ByteArray,
        key: Boolean,
        rotation: Int,
    ) = false

    override fun pushAudio(
        kind: VideoKind,
        pcm: ByteArray,
    ) = false

    override fun endVideo(kind: VideoKind) = Unit

    override fun getDeviceFingerprint(): String {
        checkInitialized()
        return "mock-device-fingerprint-0001"
    }

    override fun getDeviceSpkiHash(): String {
        checkInitialized()
        return "mock-device-spki-hash-0001"
    }

    override fun startDiscovery(protocolVersion: Long) {
        checkInitialized()
        isDiscovering = true
    }

    override fun stopDiscovery() {
        checkInitialized()
        isDiscovering = false
    }

    override fun nearbyComputers(waitMs: Int): List<NearbyComputer> {
        checkInitialized()
        return emptyList()
    }

    override fun pairNearby(code: String): String {
        checkInitialized()
        return "123456"
    }

    override fun confirmNearbyPairing(accept: Boolean): TrustedPeer? {
        checkInitialized()
        return null
    }

    override fun pairFromQr(qrPayload: String): TrustedPeer {
        checkInitialized()
        if (!qrPayload.startsWith("continue://pair")) {
            throw ContinueException.InvalidQrException("Malformed QR code payload")
        }
        val suffix = System.currentTimeMillis().toString().takeLast(QR_SUFFIX_LENGTH)
        val peer =
            TrustedPeer(
                fingerprint = "mock-qr-peer-$suffix",
                displayName = "QR Paired Device",
                pairedAt = System.currentTimeMillis() / SECONDS_DIVISOR,
            )
        peers[peer.fingerprint] = peer
        return peer
    }

    override fun listTrustedPeers(): List<TrustedPeer> {
        checkInitialized()
        return peers.values.toList()
    }

    override fun removeTrustedPeer(fingerprint: String): Boolean {
        checkInitialized()
        turnedOff.removeIf { it.startsWith("$fingerprint:") }
        connectedPeers.remove(fingerprint)
        return peers.remove(fingerprint) != null
    }

    override fun isAllowed(
        peerFingerprint: String,
        capabilityId: Int,
    ): Boolean {
        checkInitialized()
        return "$peerFingerprint:$capabilityId" !in turnedOff
    }

    override fun setAllowed(
        peerFingerprint: String,
        capabilityId: Int,
        allowed: Boolean,
    ) {
        checkInitialized()
        val key = "$peerFingerprint:$capabilityId"
        if (allowed) turnedOff.remove(key) else turnedOff.add(key)
    }

    override fun nextReceived(timeoutMs: Long): Received? = received.poll(timeoutMs, MILLISECONDS)

    override fun listIncoming(): List<IncomingFile> = emptyList()

    override fun cancelIncoming(transferId: String): Boolean = false

    override fun listHistory(limit: Int): List<HistoryEntry> = emptyList()

    override fun clearHistory() = Unit

    override fun setHistoryLocation(
        id: Long,
        location: String,
    ) = Unit

    override fun connectToPeer(
        peerFingerprint: String,
        endpoint: String,
    ) {
        checkInitialized()
        connectedPeers[peerFingerprint] = endpoint
    }

    override fun isPeerConnected(peerFingerprint: String): Boolean {
        checkInitialized()
        return connectedPeers.containsKey(peerFingerprint)
    }

    override fun disconnect(peerFingerprint: String) {
        checkInitialized()
        connectedPeers.remove(peerFingerprint)
    }

    override fun reconnect(peerFingerprint: String) {
        checkInitialized()
        connectedPeers[peerFingerprint] = "reconnected"
    }

    override fun setPaused(paused: Boolean) {
        checkInitialized()
        if (paused) connectedPeers.clear()
    }

    override fun pointerLeft(y: Float) = Unit

    override fun touchpadStart(peerFingerprint: String): Boolean {
        checkInitialized()
        return connectedPeers.containsKey(peerFingerprint)
    }

    override fun touchpadInput(input: PointerInput) = true

    override fun touchpadStop() = Unit

    override fun ringComputer(
        peerFingerprint: String,
        on: Boolean,
    ): Boolean {
        checkInitialized()
        return connectedPeers.containsKey(peerFingerprint)
    }

    override fun actOnComputer(
        peerFingerprint: String,
        action: ComputerAction,
    ): Boolean {
        checkInitialized()
        return connectedPeers.containsKey(peerFingerprint)
    }

    override fun pairingWords(peerFingerprint: String): String {
        checkInitialized()
        return "acorn river table zebra"
    }

    override fun presenceToken(): ByteArray {
        checkInitialized()
        return ByteArray(8)
    }

    private val pinned = mutableListOf<Snippet>()

    override fun snippets(): List<Snippet> {
        checkInitialized()
        return pinned.toList()
    }

    override fun pinSnippet(text: String): Snippet {
        checkInitialized()
        return pinned.firstOrNull { it.text == text } ?: Snippet(
            UUID.randomUUID().toString(),
            text,
        ).also { pinned.add(0, it) }
    }

    override fun unpinSnippet(id: String) {
        checkInitialized()
        pinned.removeAll { it.id == id }
    }

    override fun sendFile(
        peerFingerprint: String,
        filePath: String,
    ): Long {
        checkInitialized()
        return 0L
    }

    override fun sendFileLater(
        peerFingerprint: String,
        filePath: String,
    ) = checkInitialized()

    override fun sendTextLater(
        peerFingerprint: String,
        text: String,
    ) = checkInitialized()

    override fun sendClipboardImage(
        peerFingerprint: String,
        png: ByteArray,
    ) = checkInitialized()

    override fun sendClipboardText(
        peerFingerprint: String,
        text: String,
    ) {
        checkInitialized()
    }

    override fun forwardNotification(notification: PhoneNotification) = checkInitialized()

    override fun forwardNotificationRemoved(id: String) = checkInitialized()

    override fun nextNotificationEvent(timeoutMs: Long): NotificationEvent? =
        notificationEvents.poll(timeoutMs, MILLISECONDS)

    private fun checkInitialized() {
        if (!initialized) {
            throw ContinueException.NotInitializedException("Core runtime engine is not initialized")
        }
    }
}

/** Calls the Rust core through the UniFFI bindings generated at build time. */
@Suppress("TooManyFunctions")
class NativeContinueCoreBridge(
    private val context: Context,
    private val screen: ScreenShare,
    private val camera: CameraShare,
    private val media: MediaRemote,
    private val ringer: Ringer,
    private val pointer: PointerTarget,
) : ContinueCoreBridge {
    override fun initCore(dbPath: String) =
        native {
            // Beside the database, like the files the keys move out of.
            val sealedKeys = File(dbPath).absoluteFile.parentFile?.resolve("sealed-keys")
            if (sealedKeys != null) coreSetKeyStore(KeystoreSecretStore(sealedKeys))
            coreSetPhotoLibrary(photos)
            coreSetMessageStore(TelephonyMessages(context))
            coreSetPhoneSearch(PhoneContentSearch(context))
            coreSetSharedFolder(Environment.getExternalStorageDirectory().path)
            coreSetCallControl(TelecomCalls(context))
            coreSetScreenSource(ScreenShareFfi(screen))
            coreSetCameraSource(CameraShareFfi(camera))
            coreSetMediaControl(MediaRemoteFfi(media))
            coreSetRinger(RingerFfiAdapter(ringer))
            coreSetPointerTarget(PointerTargetFfiAdapter(pointer))
            coreInitCore(dbPath)
        }

    override fun setDeviceName(name: String) = native { coreSetDeviceName(name) }

    override fun setDeviceStatus(status: PhoneStatus) =
        native {
            coreSetDeviceStatus(
                DeviceStatusFfi(
                    batteryPercent = status.batteryPercent.toUInt(),
                    charging = status.charging,
                    cellBars = status.cellBars?.toUInt(),
                    wifiBars = status.wifiBars?.toUInt(),
                    carrier = status.carrier,
                    network = status.network,
                ),
            )
        }

    override fun setLook(
        color: Int,
        wallpaper: ByteArray?,
    ) = native { coreSetLook(color.toUInt(), wallpaper) }

    override fun peerWallpaper(peerFingerprint: String): ByteArray? = corePeerWallpaper(peerFingerprint)

    private val photos = MediaStorePhotos(context)
    private var announced: String? = null

    override fun announceNewPhoto() {
        val photo = photos.justTaken() ?: return
        // MediaStore reports one photo several times while it's being saved.
        if (photo.id == announced) return
        announced = photo.id
        native { coreAnnouncePhoto(photo) }
    }

    override fun announceMessagesChanged() = native { coreAnnounceMessagesChanged() }

    override fun announceNowPlaying(playing: NowPlaying) =
        native {
            coreAnnounceNowPlaying(
                NowPlayingFfi(
                    title = playing.title,
                    artist = playing.artist,
                    app = playing.app,
                    playing = playing.playing,
                    durationMs = playing.durationMs.toULong(),
                    positionMs = playing.positionMs.toULong(),
                    art = playing.art,
                ),
            )
        }

    override fun announceCall(
        state: CallState,
        number: String,
        name: String,
    ) = native {
        val ffiState =
            when (state) {
                CallState.RINGING -> CallStateFfi.RINGING
                CallState.TALKING -> CallStateFfi.TALKING
                CallState.ENDED -> CallStateFfi.ENDED
            }
        coreAnnounceCall(CallFfi(ffiState, number, name))
    }

    override fun pushVideoFrame(
        kind: VideoKind,
        data: ByteArray,
        key: Boolean,
        rotation: Int,
    ) = corePushVideoFrame(kind.toFfi(), data, key, rotation.toUInt())

    override fun pushAudio(
        kind: VideoKind,
        pcm: ByteArray,
    ) = corePushAudio(kind.toFfi(), pcm)

    override fun endVideo(kind: VideoKind) = coreEndVideo(kind.toFfi())

    private fun VideoKind.toFfi() =
        when (this) {
            VideoKind.SCREEN -> VideoKindFfi.SCREEN
            VideoKind.CAMERA -> VideoKindFfi.CAMERA
        }

    override fun getDeviceFingerprint(): String = native { coreGetDeviceFingerprint() }

    override fun getDeviceSpkiHash(): String = native { coreGetDeviceSpkiHash() }

    override fun startDiscovery(protocolVersion: Long) = native { coreStartDiscovery(protocolVersion.toUInt()) }

    override fun stopDiscovery() = native { coreStopDiscovery() }

    override fun pairFromQr(qrPayload: String): TrustedPeer = native { corePairFromQr(qrPayload).toTrustedPeer() }

    override fun nearbyComputers(waitMs: Int) =
        native { coreNearbyComputers(waitMs.toUInt()).map { NearbyComputer(it.name, it.code) } }

    override fun pairNearby(code: String) = native { corePairNearby(code) }

    override fun confirmNearbyPairing(accept: Boolean) = native { coreConfirmNearbyPairing(accept)?.toTrustedPeer() }

    override fun listTrustedPeers(): List<TrustedPeer> = native { coreListTrustedPeers().map { it.toTrustedPeer() } }

    override fun removeTrustedPeer(fingerprint: String): Boolean = native { coreRemoveTrustedPeer(fingerprint) }

    override fun isAllowed(
        peerFingerprint: String,
        capabilityId: Int,
    ): Boolean = native { coreIsAllowed(peerFingerprint, capabilityId.toUInt()) }

    override fun setAllowed(
        peerFingerprint: String,
        capabilityId: Int,
        allowed: Boolean,
    ) = native { coreSetAllowed(peerFingerprint, capabilityId.toUInt(), allowed) }

    override fun nextReceived(timeoutMs: Long): Received? = coreNextReceived(timeoutMs.toUInt())?.toReceived()

    override fun listIncoming(): List<IncomingFile> =
        coreListIncoming().map {
            IncomingFile(it.transferId, it.peerName, it.fileName, it.received.toLong(), it.total.toLong())
        }

    override fun cancelIncoming(transferId: String): Boolean = coreCancelIncoming(transferId)

    override fun listHistory(limit: Int): List<HistoryEntry> {
        val entries = native { coreListHistory(limit.toUInt()) }
        return entries.map { it.toHistoryEntry() }
    }

    override fun clearHistory() = native { coreClearHistory() }

    override fun setHistoryLocation(
        id: Long,
        location: String,
    ) = native { coreSetHistoryLocation(id, location) }

    override fun connectToPeer(
        peerFingerprint: String,
        endpoint: String,
    ) = native { coreConnectToPeer(peerFingerprint, endpoint) }

    override fun isPeerConnected(peerFingerprint: String): Boolean = native { coreIsPeerConnected(peerFingerprint) }

    override fun disconnect(peerFingerprint: String) = native { coreDisconnect(peerFingerprint) }

    override fun reconnect(peerFingerprint: String) = native { coreReconnect(peerFingerprint) }

    override fun setPaused(paused: Boolean) = native { coreSetPaused(paused) }

    override fun pointerLeft(y: Float) = corePointerLeft(y)

    override fun touchpadStart(peerFingerprint: String) = native { coreTouchpadStart(peerFingerprint) }

    override fun touchpadInput(input: PointerInput) = coreTouchpadInput(input.toFfi())

    override fun touchpadStop() = coreTouchpadStop()

    override fun actOnComputer(
        peerFingerprint: String,
        action: ComputerAction,
    ) = native { coreActOnComputer(peerFingerprint, action.toFfi()) }

    override fun ringComputer(
        peerFingerprint: String,
        on: Boolean,
    ) = native { coreRingComputer(peerFingerprint, on) }

    override fun pairingWords(peerFingerprint: String) = native { corePairingWords(peerFingerprint) }

    override fun presenceToken() = native { corePresenceToken() }

    override fun snippets() = native { coreSnippets().map { Snippet(it.id, it.text) } }

    override fun pinSnippet(text: String) = native { corePinSnippet(text).let { Snippet(it.id, it.text) } }

    override fun unpinSnippet(id: String) = native { coreUnpinSnippet(id) }

    override fun sendFile(
        peerFingerprint: String,
        filePath: String,
    ): Long = native { coreSendFile(peerFingerprint, filePath).toLong() }

    override fun sendFileLater(
        peerFingerprint: String,
        filePath: String,
    ) = native { coreSendFileLater(peerFingerprint, filePath) }

    override fun sendTextLater(
        peerFingerprint: String,
        text: String,
    ) = native { coreSendTextLater(peerFingerprint, text) }

    override fun sendClipboardImage(
        peerFingerprint: String,
        png: ByteArray,
    ) = native { coreSendClipboardImage(peerFingerprint, png) }

    override fun sendClipboardText(
        peerFingerprint: String,
        text: String,
    ) = native { coreSendClipboardText(peerFingerprint, text) }

    override fun forwardNotification(notification: PhoneNotification) =
        native { coreForwardNotification(notification.toFfi()) }

    override fun forwardNotificationRemoved(id: String) = native { coreForwardNotificationRemoved(id) }

    override fun nextNotificationEvent(timeoutMs: Long): NotificationEvent? =
        when (val event = coreNextNotificationEvent(timeoutMs.toUInt())) {
            is NotificationEventFfi.Action ->
                NotificationEvent.Pressed(event.notificationId, event.actionId, event.replyText)
            is NotificationEventFfi.Dismiss -> NotificationEvent.Dismissed(event.notificationId)
            is NotificationEventFfi.Mute -> NotificationEvent.Muted(event.packageName)
            null -> null
        }
}

private fun PhoneNotification.toFfi() =
    NotificationFfi(
        notificationId = id,
        packageName = packageName,
        appName = appName,
        title = title,
        body = text,
        timestamp = postedAt.toULong(),
        actions = buttons.map { NotificationActionFfi(it.id, it.label, it.isReply) },
    )

private fun TrustedPeerFfi.toTrustedPeer() =
    TrustedPeer(
        fingerprint = fingerprint,
        displayName = displayName,
        pairedAt = pairedAt.toLong(),
    )

private fun HistoryEntryFfi.toHistoryEntry() =
    HistoryEntry(
        id = id,
        at = at.toLong(),
        received = received,
        isText = isText,
        label = label,
        peerFingerprint = peerFingerprint,
        peerName = peerName,
        size = size.toLong(),
        failed = failed,
        location = location,
    )

private fun ReceivedFfi.toReceived(): Received {
    image?.let { return ReceivedImage(peerFingerprint, peerName, it) }
    val path = filePath ?: return ReceivedText(historyId, peerFingerprint, peerName, text.orEmpty())
    return ReceivedFile(
        historyId = historyId,
        peerFingerprint = peerFingerprint,
        peerName = peerName,
        path = path,
        name = fileName ?: path.substringAfterLast('/'),
        size = size.toLong(),
    )
}

/** Runs a core call, turning its errors into the bridge's own exception types. */
private inline fun <T> native(call: () -> T): T =
    try {
        call()
    } catch (e: ContinueFfiException) {
        val message = e.message.orEmpty()
        throw when (e) {
            is ContinueFfiException.InternalException ->
                ContinueException.InternalErrorException(message)
            is ContinueFfiException.InvalidQr ->
                ContinueException.InvalidQrException(message)
            is ContinueFfiException.PairingFailed ->
                ContinueException.PairingFailedException(message)
            is ContinueFfiException.DatabaseException ->
                ContinueException.DatabaseErrorException(message)
            is ContinueFfiException.NotInitialized ->
                ContinueException.NotInitializedException(message)
        }
    }
