package org.continueapp.android

import android.app.Application
import android.net.wifi.WifiManager
import org.continueapp.bridge.ContinueCoreBridge

class ContinueApplication : Application() {
    lateinit var coreBridge: ContinueCoreBridge
        private set

    // Android drops incoming multicast without this lock, and mDNS discovery relies on it.
    // It's kept in a field because the lock is released once it's garbage collected.
    @Suppress("UnusedPrivateProperty")
    private var multicastLock: WifiManager.MulticastLock? = null

    override fun onCreate() {
        super.onCreate()
        coreBridge = ContinueCoreBridge.create()
        val dbFile = getDatabasePath("continue_android.db")
        dbFile.parentFile?.mkdirs()
        coreBridge.initCore(dbFile.absolutePath)

        val wifi = getSystemService(WifiManager::class.java)
        multicastLock =
            wifi?.createMulticastLock("continue-discovery")?.apply {
                setReferenceCounted(false)
                acquire()
            }
        coreBridge.startDiscovery()
    }
}
