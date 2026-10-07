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
        val bridge =
            ContinueCoreBridge.create(
                context,
                PhoneFeatures(NoScreen, NoCamera, { false }, { false }, NoPointer),
            )

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
    fun nativeCoreStartsAndStopsDiscovery() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val bridge =
            ContinueCoreBridge.create(
                context,
                PhoneFeatures(NoScreen, NoCamera, { false }, { false }, NoPointer),
            )
        bridge.initCore(File(context.cacheDir, "discovery-test.db").absolutePath)

        bridge.startDiscovery()
        bridge.startDiscovery()
        bridge.stopDiscovery()
    }

    @Test
    fun nativeCoreReportsInvalidQrCodes() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val bridge =
            ContinueCoreBridge.create(
                context,
                PhoneFeatures(NoScreen, NoCamera, { false }, { false }, NoPointer),
            )
        bridge.initCore(File(context.cacheDir, "qr-test.db").absolutePath)

        assertThrows(ContinueException.InvalidQrException::class.java) {
            bridge.pairFromQr("not a pairing code")
        }
    }
}

private object NoScreen : ScreenShare {
    override fun start(maxSize: Int): VideoSize? = null

    override fun input(input: ScreenInput) = Unit

    override fun stop() = Unit
}

private object NoCamera : CameraShare {
    override fun start(
        maxSize: Int,
        front: Boolean,
    ): VideoSize? = null

    override fun control(control: CameraControl) = Unit

    override fun stop() = Unit
}

private object NoPointer : PointerTarget {
    override fun start(
        y: Float,
        fromLeft: Boolean,
    ) = false

    override fun input(input: PointerInput) = Unit

    override fun stop() = Unit
}
