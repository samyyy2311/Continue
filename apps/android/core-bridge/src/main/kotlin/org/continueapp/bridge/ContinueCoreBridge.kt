package org.continueapp.bridge

import org.continueapp.bridge.ffi.ContinueFfiException
import org.continueapp.bridge.ffi.HistoryEntryFfi
import org.continueapp.bridge.ffi.NotificationActionFfi
import org.continueapp.bridge.ffi.NotificationEventFfi
import org.continueapp.bridge.ffi.NotificationFfi
import org.continueapp.bridge.ffi.PermissionDecisionFfi
import org.continueapp.bridge.ffi.PermissionRequestFfi
import org.continueapp.bridge.ffi.ReceivedFfi
import org.continueapp.bridge.ffi.TrustedPeerFfi
import java.io.File
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit.MILLISECONDS
import org.continueapp.bridge.ffi.answerPermissionRequest as coreAnswerPermissionRequest
import org.continueapp.bridge.ffi.cancelIncoming as coreCancelIncoming
import org.continueapp.bridge.ffi.clearHistory as coreClearHistory
import org.continueapp.bridge.ffi.connectToPeer as coreConnectToPeer
import org.continueapp.bridge.ffi.disconnect as coreDisconnect
import org.continueapp.bridge.ffi.forwardNotification as coreForwardNotification
import org.continueapp.bridge.ffi.forwardNotificationRemoved as coreForwardNotificationRemoved
import org.continueapp.bridge.ffi.getDeviceFingerprint as coreGetDeviceFingerprint
import org.continueapp.bridge.ffi.getDeviceSpkiHash as coreGetDeviceSpkiHash
import org.continueapp.bridge.ffi.initCore as coreInitCore
import org.continueapp.bridge.ffi.isPeerConnected as coreIsPeerConnected
import org.continueapp.bridge.ffi.listHistory as coreListHistory
import org.continueapp.bridge.ffi.listIncoming as coreListIncoming
import org.continueapp.bridge.ffi.listTrustedPeers as coreListTrustedPeers
import org.continueapp.bridge.ffi.nextNotificationEvent as coreNextNotificationEvent
import org.continueapp.bridge.ffi.nextPermissionRequest as coreNextPermissionRequest
import org.continueapp.bridge.ffi.nextReceived as coreNextReceived
import org.continueapp.bridge.ffi.pairFromQr as corePairFromQr
import org.continueapp.bridge.ffi.queryPermission as coreQueryPermission
import org.continueapp.bridge.ffi.reconnect as coreReconnect
import org.continueapp.bridge.ffi.removeTrustedPeer as coreRemoveTrustedPeer
import org.continueapp.bridge.ffi.sendClipboardText as coreSendClipboardText
import org.continueapp.bridge.ffi.sendFile as coreSendFile
import org.continueapp.bridge.ffi.setDeviceName as coreSetDeviceName
import org.continueapp.bridge.ffi.setDeviceStatus as coreSetDeviceStatus
import org.continueapp.bridge.ffi.setHistoryLocation as coreSetHistoryLocation
import org.continueapp.bridge.ffi.setKeyStore as coreSetKeyStore
import org.continueapp.bridge.ffi.setPermission as coreSetPermission
import org.continueapp.bridge.ffi.startDiscovery as coreStartDiscovery
import org.continueapp.bridge.ffi.stopDiscovery as coreStopDiscovery

private const val SECONDS_DIVISOR = 1000L
private const val QR_SUFFIX_LENGTH = 4

@Suppress("TooManyFunctions")
interface ContinueCoreBridge {
    fun initCore(dbPath: String)

    /** The name paired computers see for this phone, from the next connection on. */
    fun setDeviceName(name: String)

    /** Sends the battery to connected computers, now and whenever they connect. */
    fun setDeviceStatus(
        batteryPercent: Int,
        charging: Boolean,
    )

    fun getDeviceFingerprint(): String

    fun getDeviceSpkiHash(): String

    /** Advertises this device's listener and connects to paired devices as they appear. */
    fun startDiscovery(protocolVersion: Long = 1L)

    fun stopDiscovery()

    fun pairFromQr(qrPayload: String): TrustedPeer

    fun listTrustedPeers(): List<TrustedPeer>

    fun removeTrustedPeer(fingerprint: String): Boolean

    fun queryPermission(
        peerFingerprint: String,
        capabilityId: Int,
    ): String

    fun setPermission(
        peerFingerprint: String,
        capabilityId: Int,
        grant: String,
    )

    /**
     * Waits up to [timeoutMs] for the next question for the user, or returns null. The core
     * declines a question nobody answers within 30 seconds.
     */
    fun nextPermissionQuestion(timeoutMs: Long): PermissionQuestion?

    fun answerPermissionQuestion(
        id: Long,
        answer: PermissionAnswer,
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

    fun sendFile(
        peerFingerprint: String,
        filePath: String,
    ): Long

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
        fun create(): ContinueCoreBridge = NativeContinueCoreBridge()

        fun mock(): MockContinueCoreBridge = MockContinueCoreBridge()
    }
}

@Suppress("TooManyFunctions")
class MockContinueCoreBridge : ContinueCoreBridge {
    private var initialized: Boolean = false
    private var isDiscovering: Boolean = false
    private val peers = ConcurrentHashMap<String, TrustedPeer>()
    private val permissions = ConcurrentHashMap<String, String>()
    private val connectedPeers = ConcurrentHashMap<String, String>()
    private val questions = LinkedBlockingQueue<PermissionQuestion>()
    private val received = LinkedBlockingQueue<Received>()
    private val notificationEvents = LinkedBlockingQueue<NotificationEvent>()
    val answers = ConcurrentHashMap<Long, PermissionAnswer>()

    /** Puts a question to the app as a device set to Ask would. */
    fun ask(question: PermissionQuestion) {
        questions.put(question)
    }

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

    override fun setDeviceStatus(
        batteryPercent: Int,
        charging: Boolean,
    ) = checkInitialized()

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
        permissions.keys.filter { it.startsWith("$fingerprint:") }.forEach { permissions.remove(it) }
        connectedPeers.remove(fingerprint)
        return peers.remove(fingerprint) != null
    }

    override fun queryPermission(
        peerFingerprint: String,
        capabilityId: Int,
    ): String {
        checkInitialized()
        val key = "$peerFingerprint:$capabilityId"
        return permissions[key] ?: PermissionGrant.ASK.rawValue
    }

    override fun setPermission(
        peerFingerprint: String,
        capabilityId: Int,
        grant: String,
    ) {
        checkInitialized()
        val key = "$peerFingerprint:$capabilityId"
        permissions[key] = grant
    }

    override fun nextPermissionQuestion(timeoutMs: Long): PermissionQuestion? = questions.poll(timeoutMs, MILLISECONDS)

    override fun answerPermissionQuestion(
        id: Long,
        answer: PermissionAnswer,
    ) {
        answers[id] = answer
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

    override fun sendFile(
        peerFingerprint: String,
        filePath: String,
    ): Long {
        checkInitialized()
        return 0L
    }

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
class NativeContinueCoreBridge : ContinueCoreBridge {
    override fun initCore(dbPath: String) =
        native {
            // Beside the database, like the files the keys move out of.
            val sealedKeys = File(dbPath).absoluteFile.parentFile?.resolve("sealed-keys")
            if (sealedKeys != null) coreSetKeyStore(KeystoreSecretStore(sealedKeys))
            coreInitCore(dbPath)
        }

    override fun setDeviceName(name: String) = native { coreSetDeviceName(name) }

    override fun setDeviceStatus(
        batteryPercent: Int,
        charging: Boolean,
    ) = native { coreSetDeviceStatus(batteryPercent.toUInt(), charging) }

    override fun getDeviceFingerprint(): String = native { coreGetDeviceFingerprint() }

    override fun getDeviceSpkiHash(): String = native { coreGetDeviceSpkiHash() }

    override fun startDiscovery(protocolVersion: Long) = native { coreStartDiscovery(protocolVersion.toUInt()) }

    override fun stopDiscovery() = native { coreStopDiscovery() }

    override fun pairFromQr(qrPayload: String): TrustedPeer = native { corePairFromQr(qrPayload).toTrustedPeer() }

    override fun listTrustedPeers(): List<TrustedPeer> = native { coreListTrustedPeers().map { it.toTrustedPeer() } }

    override fun removeTrustedPeer(fingerprint: String): Boolean = native { coreRemoveTrustedPeer(fingerprint) }

    override fun queryPermission(
        peerFingerprint: String,
        capabilityId: Int,
    ): String = native { coreQueryPermission(peerFingerprint, capabilityId.toUInt()) }

    override fun setPermission(
        peerFingerprint: String,
        capabilityId: Int,
        grant: String,
    ) = native { coreSetPermission(peerFingerprint, capabilityId.toUInt(), grant) }

    override fun nextPermissionQuestion(timeoutMs: Long): PermissionQuestion? =
        coreNextPermissionRequest(timeoutMs.toUInt())?.toPermissionQuestion()

    override fun answerPermissionQuestion(
        id: Long,
        answer: PermissionAnswer,
    ) = coreAnswerPermissionRequest(
        id.toULong(),
        when (answer) {
            PermissionAnswer.ALLOW -> PermissionDecisionFfi.ALLOW
            PermissionAnswer.ALWAYS_ALLOW -> PermissionDecisionFfi.ALWAYS_ALLOW
            PermissionAnswer.DECLINE -> PermissionDecisionFfi.DECLINE
        },
    )

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

    override fun sendFile(
        peerFingerprint: String,
        filePath: String,
    ): Long = native { coreSendFile(peerFingerprint, filePath).toLong() }

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

private fun PermissionRequestFfi.toPermissionQuestion() =
    PermissionQuestion(
        id = id.toLong(),
        peerFingerprint = peerFingerprint,
        peerName = peerName,
        capability = Capability.fromId(capabilityId.toInt()),
        detail = detail,
        expiresAt = expiresAt.toLong(),
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
