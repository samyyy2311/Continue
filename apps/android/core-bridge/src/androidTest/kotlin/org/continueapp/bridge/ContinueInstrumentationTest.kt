package org.continueapp.bridge

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

@RunWith(AndroidJUnit4::class)
class ContinueInstrumentationTest {
    @Test
    fun useAppContext() {
        val appContext = InstrumentationRegistry.getInstrumentation().targetContext
        assertEquals("org.continueapp.bridge.test", appContext.packageName)
    }

    @Test
    fun nativeCoreKeepsItsIdentityAcrossRestarts() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val dataDir = File(context.cacheDir, "core-test").apply { deleteRecursively() }
        dataDir.mkdirs()
        val dbPath = File(dataDir, "continue.db").absolutePath
        val bridge = ContinueCoreBridge.create()

        bridge.initCore(dbPath)
        val fingerprint = bridge.getDeviceFingerprint()
        val spkiHash = bridge.getDeviceSpkiHash()
        assertTrue(fingerprint.isNotEmpty())
        assertTrue(bridge.listTrustedPeers().isEmpty())

        bridge.initCore(dbPath)
        assertEquals(fingerprint, bridge.getDeviceFingerprint())
        assertEquals(spkiHash, bridge.getDeviceSpkiHash())
    }

    @Test
    fun nativeCoreReportsInvalidQrCodes() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val bridge = ContinueCoreBridge.create()
        bridge.initCore(File(context.cacheDir, "qr-test.db").absolutePath)

        assertThrows(ContinueException.InvalidQrException::class.java) {
            bridge.pairFromQr("not a pairing code")
        }
    }
}
