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
        bridge.startDiscovery()
        bridge.stopDiscovery()
    }

    @Test
    fun pairingFlowAndPeers() {
        bridge.initCore(":memory:")
        val pairedPeer = bridge.pairFromQr("continue://pair?endpoint=192.168.1.50:41235")
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
    fun everythingIsAllowedUntilTurnedOff() {
        bridge.initCore(":memory:")
        val peerFp = "peer-test-123"
        assertTrue(bridge.isAllowed(peerFp, Capability.FILE_TRANSFER.id))

        bridge.setAllowed(peerFp, Capability.FILE_TRANSFER.id, false)
        assertFalse(bridge.isAllowed(peerFp, Capability.FILE_TRANSFER.id))
        assertTrue(bridge.isAllowed(peerFp, Capability.CLIPBOARD.id))

        bridge.setAllowed(peerFp, Capability.FILE_TRANSFER.id, true)
        assertTrue(bridge.isAllowed(peerFp, Capability.FILE_TRANSFER.id))
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
    }
}
