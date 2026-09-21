package org.continueapp.android

import android.app.Application
import org.continueapp.bridge.ContinueCoreBridge

class ContinueApplication : Application() {
    lateinit var coreBridge: ContinueCoreBridge
        private set

    override fun onCreate() {
        super.onCreate()
        coreBridge = ContinueCoreBridge.create()
        val dbFile = getDatabasePath("continue_android.db")
        dbFile.parentFile?.mkdirs()
        coreBridge.initCore(dbFile.absolutePath)
    }
}
