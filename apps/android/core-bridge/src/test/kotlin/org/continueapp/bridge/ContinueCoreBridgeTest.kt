package org.continueapp.bridge

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class ContinueCoreBridgeTest {
    private lateinit var bridge: ContinueCoreBridge

    @Before
    fun setUp() {
        bridge = ContinueCoreBridge.mock()
    }

    @Test
    fun uninitializedCoreThrowsException() {
        assertThrows(ContinueException.NotInitializedException::class.java) {
            bridge.getDeviceFingerprint()
        }
    }

    @Test
    fun initCoreAndGetDeviceFingerprint() {
        bridge.initCore(":memory:")
        val fingerprint = bridge.getDeviceFingerprint()
        val spki = bridge.getDeviceSpkiHash()

        assertNotNull(fingerprint)
        assertTrue(fingerprint.isNotEmpty())
        assertNotNull(spki)
        assertTrue(spki.isNotEmpty())
    }

    @Test
    fun discoveryLifecycle() {
        bridge.initCore(":memory:")
        bridge.startDiscovery(41234, 1L)
        bridge.stopDiscovery()
    }

    @Test
    fun pairingFlowAndPeers() {
        bridge.initCore(":memory:")
        val qrPayload = bridge.generateQrPayload("192.168.1.50:41235")
        assertTrue(qrPayload.startsWith("continue://pair"))

        val pairedPeer = bridge.pairFromQr(qrPayload)
        assertNotNull(pairedPeer.fingerprint)

        val peers = bridge.listTrustedPeers()
        assertEquals(1, peers.size)
        assertEquals(pairedPeer.fingerprint, peers[0].fingerprint)

        val removed = bridge.removeTrustedPeer(pairedPeer.fingerprint)
        assertTrue(removed)
        assertEquals(0, bridge.listTrustedPeers().size)
    }

    @Test
    fun invalidQrThrowsException() {
        bridge.initCore(":memory:")
        assertThrows(ContinueException.InvalidQrException::class.java) {
            bridge.pairFromQr("invalid-url-schema")
        }
    }

    @Test
    fun permissionsLifecycle() {
        bridge.initCore(":memory:")
        val peerFp = "peer-test-123"

        val defaultGrant = bridge.queryPermission(peerFp, Capability.FILE_TRANSFER.id)
        assertEquals(PermissionGrant.PROMPT.rawValue, defaultGrant)

        bridge.setPermission(peerFp, Capability.FILE_TRANSFER.id, PermissionGrant.ALWAYS_ALLOW.rawValue)
        val updatedGrant = bridge.queryPermission(peerFp, Capability.FILE_TRANSFER.id)
        assertEquals(PermissionGrant.ALWAYS_ALLOW.rawValue, updatedGrant)

        bridge.revokePermission(peerFp, Capability.FILE_TRANSFER.id)
        val revokedGrant = bridge.queryPermission(peerFp, Capability.FILE_TRANSFER.id)
        assertEquals(PermissionGrant.PROMPT.rawValue, revokedGrant)
    }

    @Test
    fun connectionLifecycle() {
        bridge.initCore(":memory:")
        val peerFp = "peer-test-conn"

        assertFalse(bridge.isPeerConnected(peerFp))

        bridge.connectToPeer(peerFp, "192.168.1.100:41235")
        assertTrue(bridge.isPeerConnected(peerFp))

        bridge.disconnect(peerFp)
        assertFalse(bridge.isPeerConnected(peerFp))
    }

    @Test
    fun capabilityMethodsExecution() {
        bridge.initCore(":memory:")
        val peerFp = "peer-test-caps"

        val bytesSent = bridge.sendFile(peerFp, "/tmp/sample.txt")
        assertEquals(0L, bytesSent)

        bridge.sendClipboardText(peerFp, "Test clipboard payload")
        bridge.sendNotification(peerFp, "Alert", "Incoming message", "Messages")
    }
}
