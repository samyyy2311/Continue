package org.continueapp.bridge

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ContinueInstrumentationTest {
    @Test
    fun useAppContext() {
        val appContext = InstrumentationRegistry.getInstrumentation().targetContext
        assertEquals("org.continueapp.bridge.test", appContext.packageName)
    }

    @Test
    fun bridgeInitializationOnAndroid() {
        val bridge = ContinueCoreBridge.create()
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val dbFile = context.getDatabasePath("continue_test.db")
        bridge.initCore(dbFile.absolutePath)
        assertNotNull(bridge.getDeviceFingerprint())
    }
}
