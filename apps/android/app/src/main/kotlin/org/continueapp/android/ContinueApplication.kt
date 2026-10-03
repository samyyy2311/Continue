package org.continueapp.android

import android.app.Application
import android.content.Context
import android.net.wifi.WifiManager
import org.continueapp.android.ui.theme.ThemeMode
import org.continueapp.bridge.ContinueCoreBridge

class ContinueApplication : Application() {
    lateinit var coreBridge: ContinueCoreBridge
        private set

    // Android drops incoming multicast without this lock, and mDNS discovery relies on it.
    // It's kept in a field because the lock is released once it's garbage collected.
    private var multicastLock: WifiManager.MulticastLock? = null

    private val settings by lazy { getSharedPreferences("settings", Context.MODE_PRIVATE) }

    /** Whether paired devices can find this phone on the network. */
    var visible: Boolean
        get() = settings.getBoolean(KEY_VISIBLE, true)
        set(value) {
            settings.edit().putBoolean(KEY_VISIBLE, value).apply()
            applyVisibility(value)
        }

    var themeMode: ThemeMode
        get() = ThemeMode.entries.firstOrNull { it.name == settings.getString(KEY_THEME, null) } ?: ThemeMode.System
        set(value) = settings.edit().putString(KEY_THEME, value.name).apply()

    var wallpaperColors: Boolean
        get() = settings.getBoolean(KEY_WALLPAPER_COLORS, false)
        set(value) = settings.edit().putBoolean(KEY_WALLPAPER_COLORS, value).apply()

    override fun onCreate() {
        super.onCreate()
        coreBridge = ContinueCoreBridge.create()
        val dbFile = getDatabasePath("continue_android.db")
        dbFile.parentFile?.mkdirs()
        coreBridge.initCore(dbFile.absolutePath)
        applyVisibility(visible)
    }

    private fun applyVisibility(visible: Boolean) {
        if (visible) {
            multicastLock =
                multicastLock ?: getSystemService(WifiManager::class.java)
                    ?.createMulticastLock("continue-discovery")
                    ?.apply {
                        setReferenceCounted(false)
                        acquire()
                    }
            coreBridge.startDiscovery()
        } else {
            coreBridge.stopDiscovery()
            multicastLock?.release()
            multicastLock = null
        }
    }

    private companion object {
        const val KEY_VISIBLE = "visible"
        const val KEY_THEME = "theme"
        const val KEY_WALLPAPER_COLORS = "wallpaper_colors"
    }
}
