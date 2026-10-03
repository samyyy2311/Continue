package org.continueapp.bridge

import org.continueapp.bridge.ffi.ContinueFfiException
import org.continueapp.bridge.ffi.PermissionDecisionFfi
import org.continueapp.bridge.ffi.PermissionRequestFfi
import org.continueapp.bridge.ffi.TrustedPeerFfi
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit.MILLISECONDS
import org.continueapp.bridge.ffi.answerPermissionRequest as coreAnswerPermissionRequest
import org.continueapp.bridge.ffi.awaitPairingResult as coreAwaitPairingResult
import org.continueapp.bridge.ffi.cancelPairing as coreCancelPairing
import org.continueapp.bridge.ffi.connectToPeer as coreConnectToPeer
import org.continueapp.bridge.ffi.disconnect as coreDisconnect
import org.continueapp.bridge.ffi.generateQrPayload as coreGenerateQrPayload
import org.continueapp.bridge.ffi.getCapabilities as coreGetCapabilities
import org.continueapp.bridge.ffi.getDeviceFingerprint as coreGetDeviceFingerprint
import org.continueapp.bridge.ffi.getDeviceSpkiHash as coreGetDeviceSpkiHash
import org.continueapp.bridge.ffi.initCore as coreInitCore
import org.continueapp.bridge.ffi.isPeerConnected as coreIsPeerConnected
import org.continueapp.bridge.ffi.listTrustedPeers as coreListTrustedPeers
import org.continueapp.bridge.ffi.nextPermissionRequest as coreNextPermissionRequest
import org.continueapp.bridge.ffi.pairFromQr as corePairFromQr
import org.continueapp.bridge.ffi.queryPermission as coreQueryPermission
import org.continueapp.bridge.ffi.reconnect as coreReconnect
import org.continueapp.bridge.ffi.removeTrustedPeer as coreRemoveTrustedPeer
import org.continueapp.bridge.ffi.revokePermission as coreRevokePermission
import org.continueapp.bridge.ffi.sendClipboardText as coreSendClipboardText
import org.continueapp.bridge.ffi.sendFile as coreSendFile
import org.continueapp.bridge.ffi.sendNotification as coreSendNotification
import org.continueapp.bridge.ffi.setPermission as coreSetPermission
import org.continueapp.bridge.ffi.startDiscovery as coreStartDiscovery
import org.continueapp.bridge.ffi.startPairingServer as coreStartPairingServer
import org.continueapp.bridge.ffi.stopDiscovery as coreStopDiscovery

private const val SECONDS_DIVISOR = 1000L
private const val QR_SUFFIX_LENGTH = 4

@Suppress("TooManyFunctions")
interface ContinueCoreBridge {
    fun initCore(dbPath: String)

    fun getDeviceFingerprint(): String

    fun getDeviceSpkiHash(): String

    /** Advertises this device's listener and connects to paired devices as they appear. */
    fun startDiscovery(protocolVersion: Long = 1L)

    fun stopDiscovery()

    fun generateQrPayload(endpoint: String): String

    fun startPairingServer(
        listenPort: Int = 41235,
        advertisedEndpoint: String,
    ): String

    fun awaitPairingResult(timeoutSecs: Long = 60L): TrustedPeer

    fun cancelPairing()

    fun pairFromQr(qrPayload: String): TrustedPeer

    fun listTrustedPeers(): List<TrustedPeer>

    fun removeTrustedPeer(fingerprint: String): Boolean

    fun getCapabilities(): List<Int>

    fun queryPermission(
        peerFingerprint: String,
        capabilityId: Int,
    ): String

    fun setPermission(
        peerFingerprint: String,
        capabilityId: Int,
        grant: String,
    )

    fun revokePermission(
        peerFingerprint: String,
        capabilityId: Int,
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

    fun sendNotification(
        peerFingerprint: String,
        title: String,
        body: String,
        appName: String,
    )

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
    val answers = ConcurrentHashMap<Long, PermissionAnswer>()

    /** Puts a question to the app as a device set to Ask would. */
    fun ask(question: PermissionQuestion) {
        questions.put(question)
    }

    override fun initCore(dbPath: String) {
        initialized = true
    }

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

    override fun generateQrPayload(endpoint: String): String {
        checkInitialized()
        return "continue://pair?endpoint=$endpoint&pubkey=mock-public-key"
    }

    override fun startPairingServer(
        listenPort: Int,
        advertisedEndpoint: String,
    ): String {
        checkInitialized()
        return "mock-pin-123456"
    }

    override fun awaitPairingResult(timeoutSecs: Long): TrustedPeer {
        checkInitialized()
        val peer =
            TrustedPeer(
                fingerprint = "mock-paired-peer-1",
                displayName = "Mock Trusted Device",
                pairedAt = System.currentTimeMillis() / SECONDS_DIVISOR,
            )
        peers[peer.fingerprint] = peer
        return peer
    }

    override fun cancelPairing() {
        checkInitialized()
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

    override fun getCapabilities(): List<Int> {
        checkInitialized()
        return Capability.entries.map { it.id }
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

    override fun revokePermission(
        peerFingerprint: String,
        capabilityId: Int,
    ) {
        checkInitialized()
        val key = "$peerFingerprint:$capabilityId"
        permissions.remove(key)
    }

    override fun nextPermissionQuestion(timeoutMs: Long): PermissionQuestion? = questions.poll(timeoutMs, MILLISECONDS)

    override fun answerPermissionQuestion(
        id: Long,
        answer: PermissionAnswer,
    ) {
        answers[id] = answer
    }

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

    override fun sendNotification(
        peerFingerprint: String,
        title: String,
        body: String,
        appName: String,
    ) {
        checkInitialized()
    }

    private fun checkInitialized() {
        if (!initialized) {
            throw ContinueException.NotInitializedException("Core runtime engine is not initialized")
        }
    }
}

/** Calls the Rust core through the UniFFI bindings generated at build time. */
@Suppress("TooManyFunctions")
class NativeContinueCoreBridge : ContinueCoreBridge {
    override fun initCore(dbPath: String) = native { coreInitCore(dbPath) }

    override fun getDeviceFingerprint(): String = native { coreGetDeviceFingerprint() }

    override fun getDeviceSpkiHash(): String = native { coreGetDeviceSpkiHash() }

    override fun startDiscovery(protocolVersion: Long) = native { coreStartDiscovery(protocolVersion.toUInt()) }

    override fun stopDiscovery() = native { coreStopDiscovery() }

    override fun generateQrPayload(endpoint: String): String = native { coreGenerateQrPayload(endpoint) }

    override fun startPairingServer(
        listenPort: Int,
        advertisedEndpoint: String,
    ): String = native { coreStartPairingServer(listenPort.toUShort(), advertisedEndpoint) }

    override fun awaitPairingResult(timeoutSecs: Long): TrustedPeer =
        native { coreAwaitPairingResult(timeoutSecs.toUInt()).toTrustedPeer() }

    override fun cancelPairing() = native { coreCancelPairing() }

    override fun pairFromQr(qrPayload: String): TrustedPeer = native { corePairFromQr(qrPayload).toTrustedPeer() }

    override fun listTrustedPeers(): List<TrustedPeer> = native { coreListTrustedPeers().map { it.toTrustedPeer() } }

    override fun removeTrustedPeer(fingerprint: String): Boolean = native { coreRemoveTrustedPeer(fingerprint) }

    override fun getCapabilities(): List<Int> = native { coreGetCapabilities().map { it.toInt() } }

    override fun queryPermission(
        peerFingerprint: String,
        capabilityId: Int,
    ): String = native { coreQueryPermission(peerFingerprint, capabilityId.toUInt()) }

    override fun setPermission(
        peerFingerprint: String,
        capabilityId: Int,
        grant: String,
    ) = native { coreSetPermission(peerFingerprint, capabilityId.toUInt(), grant) }

    override fun revokePermission(
        peerFingerprint: String,
        capabilityId: Int,
    ) = native { coreRevokePermission(peerFingerprint, capabilityId.toUInt()) }

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

    override fun sendNotification(
        peerFingerprint: String,
        title: String,
        body: String,
        appName: String,
    ) = native { coreSendNotification(peerFingerprint, title, body, appName) }
}

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
    )

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
            is ContinueFfiException.PairingTimeout ->
                ContinueException.PairingTimeoutException(message)
            is ContinueFfiException.DatabaseException ->
                ContinueException.DatabaseErrorException(message)
            is ContinueFfiException.NotInitialized ->
                ContinueException.NotInitializedException(message)
        }
    }
