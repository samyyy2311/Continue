package org.continueapp.bridge

import java.util.concurrent.ConcurrentHashMap

private const val SECONDS_DIVISOR = 1000L
private const val QR_SUFFIX_LENGTH = 4

@Suppress("TooManyFunctions")
interface ContinueCoreBridge {
    fun initCore(dbPath: String)

    fun getDeviceFingerprint(): String

    fun getDeviceSpkiHash(): String

    fun startDiscovery(
        port: Int = 41234,
        protocolVersion: Long = 1L,
    )

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

    fun connectToPeer(
        peerFingerprint: String,
        endpoint: String,
    )

    fun isPeerConnected(peerFingerprint: String): Boolean

    fun disconnect(peerFingerprint: String)

    companion object {
        fun create(forceMock: Boolean = false): ContinueCoreBridge {
            if (forceMock) {
                return MockContinueCoreBridge()
            }
            return try {
                NativeContinueCoreBridge()
            } catch (_: UnsatisfiedLinkError) {
                MockContinueCoreBridge()
            } catch (_: NoClassDefFoundError) {
                MockContinueCoreBridge()
            }
        }

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

    override fun startDiscovery(
        port: Int,
        protocolVersion: Long,
    ) {
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
        return permissions[key] ?: PermissionGrant.PROMPT.rawValue
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

    private fun checkInitialized() {
        if (!initialized) {
            throw ContinueException.NotInitializedException("Core runtime engine is not initialized")
        }
    }
}

@Suppress("TooManyFunctions")
class NativeContinueCoreBridge : ContinueCoreBridge {
    init {
        System.loadLibrary("continue_ffi")
    }

    override fun initCore(dbPath: String) {
        initCoreNative(dbPath)
    }

    override fun getDeviceFingerprint(): String = getDeviceFingerprintNative()

    override fun getDeviceSpkiHash(): String = getDeviceSpkiHashNative()

    override fun startDiscovery(
        port: Int,
        protocolVersion: Long,
    ) {
        startDiscoveryNative(port, protocolVersion)
    }

    override fun stopDiscovery() {
        stopDiscoveryNative()
    }

    override fun generateQrPayload(endpoint: String): String = generateQrPayloadNative(endpoint)

    override fun startPairingServer(
        listenPort: Int,
        advertisedEndpoint: String,
    ): String = startPairingServerNative(listenPort, advertisedEndpoint)

    override fun awaitPairingResult(timeoutSecs: Long): TrustedPeer = awaitPairingResultNative(timeoutSecs)

    override fun cancelPairing() {
        cancelPairingNative()
    }

    override fun pairFromQr(qrPayload: String): TrustedPeer = pairFromQrNative(qrPayload)

    override fun listTrustedPeers(): List<TrustedPeer> = listTrustedPeersNative().toList()

    override fun removeTrustedPeer(fingerprint: String): Boolean = removeTrustedPeerNative(fingerprint)

    override fun getCapabilities(): List<Int> = getCapabilitiesNative().toList()

    override fun queryPermission(
        peerFingerprint: String,
        capabilityId: Int,
    ): String = queryPermissionNative(peerFingerprint, capabilityId)

    override fun setPermission(
        peerFingerprint: String,
        capabilityId: Int,
        grant: String,
    ) {
        setPermissionNative(peerFingerprint, capabilityId, grant)
    }

    override fun revokePermission(
        peerFingerprint: String,
        capabilityId: Int,
    ) {
        revokePermissionNative(peerFingerprint, capabilityId)
    }

    override fun connectToPeer(
        peerFingerprint: String,
        endpoint: String,
    ) {
        connectToPeerNative(peerFingerprint, endpoint)
    }

    override fun isPeerConnected(peerFingerprint: String): Boolean = isPeerConnectedNative(peerFingerprint)

    override fun disconnect(peerFingerprint: String) {
        disconnectNative(peerFingerprint)
    }

    private external fun initCoreNative(dbPath: String)

    private external fun getDeviceFingerprintNative(): String

    private external fun getDeviceSpkiHashNative(): String

    private external fun startDiscoveryNative(
        port: Int,
        protocolVersion: Long,
    )

    private external fun stopDiscoveryNative()

    private external fun generateQrPayloadNative(endpoint: String): String

    private external fun startPairingServerNative(
        listenPort: Int,
        advertisedEndpoint: String,
    ): String

    private external fun awaitPairingResultNative(timeoutSecs: Long): TrustedPeer

    private external fun cancelPairingNative()

    private external fun pairFromQrNative(qrPayload: String): TrustedPeer

    private external fun listTrustedPeersNative(): Array<TrustedPeer>

    private external fun removeTrustedPeerNative(fingerprint: String): Boolean

    private external fun getCapabilitiesNative(): IntArray

    private external fun queryPermissionNative(
        peerFingerprint: String,
        capabilityId: Int,
    ): String

    private external fun setPermissionNative(
        peerFingerprint: String,
        capabilityId: Int,
        grant: String,
    )

    private external fun revokePermissionNative(
        peerFingerprint: String,
        capabilityId: Int,
    )

    private external fun connectToPeerNative(
        peerFingerprint: String,
        endpoint: String,
    )

    private external fun isPeerConnectedNative(peerFingerprint: String): Boolean

    private external fun disconnectNative(peerFingerprint: String)
}
