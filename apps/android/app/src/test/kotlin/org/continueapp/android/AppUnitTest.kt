package org.continueapp.android

import org.continueapp.bridge.ContinueCoreBridge
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class AppUnitTest {
    private lateinit var bridge: ContinueCoreBridge

    @Before
    fun setUp() {
        bridge = ContinueCoreBridge.mock()
        bridge.initCore(":memory:")
    }

    @Test
    fun appInitializationLoadsDeviceIdentity() {
        val fingerprint = bridge.getDeviceFingerprint()
        val spki = bridge.getDeviceSpkiHash()

        assertNotNull(fingerprint)
        assertTrue(fingerprint.isNotEmpty())
        assertNotNull(spki)
        assertTrue(spki.isNotEmpty())
    }

    @Test
    fun appPairingFlowAddsPeer() {
        val qrPayload = bridge.generateQrPayload("192.168.1.10:41235")
        val peer = bridge.pairFromQr(qrPayload)

        assertNotNull(peer)
        val peers = bridge.listTrustedPeers()
        assertEquals(1, peers.size)
        assertEquals(peer.fingerprint, peers[0].fingerprint)

        val removed = bridge.removeTrustedPeer(peer.fingerprint)
        assertTrue(removed)
        assertEquals(0, bridge.listTrustedPeers().size)
    }
}
