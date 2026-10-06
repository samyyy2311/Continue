package org.continueapp.android

import android.app.Application
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.Uri
import android.net.wifi.WifiManager
import android.os.BatteryManager
import android.os.Build
import android.provider.Settings
import androidx.compose.runtime.mutableStateOf
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.ProcessLifecycleOwner
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.continueapp.android.ui.theme.ThemeMode
import org.continueapp.bridge.ContinueCoreBridge

class ContinueApplication : Application() {
    lateinit var coreBridge: ContinueCoreBridge
        private set

    /** Shared by the screens and the background work, so both see the same thing. */
    val state by lazy { AppState(coreBridge) }

    /** For work that outlives any one screen. */
    val scope = MainScope()

    private val onScreen: Boolean
        get() = ProcessLifecycleOwner.get().lifecycle.currentState.isAtLeast(Lifecycle.State.STARTED)

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

    /** Whether opening the app sends anything newly copied to the computer. */
    var sendNewCopies: Boolean
        get() = settings.getBoolean(KEY_SEND_COPIES, true)
        set(value) = settings.edit().putBoolean(KEY_SEND_COPIES, value).apply()

    /** The folder picked for received files, or null for Downloads/Continue. */
    var saveFolder: Uri?
        get() = settings.getString(KEY_SAVE_FOLDER, null)?.let(Uri::parse)
        set(value) = settings.edit().putString(KEY_SAVE_FOLDER, value?.toString()).apply()

    /** When the newest copy the app has already dealt with was made. */
    var lastCopySeen: Long
        get() = settings.getLong(KEY_COPY_SEEN, 0L)
        set(value) = settings.edit().putLong(KEY_COPY_SEEN, value).apply()

    /**
     * Something shared from another app, waiting for the user to pick a computer. Kept here
     * so it survives the screen being rebuilt, as when the phone is turned.
     */
    val pendingShare = mutableStateOf<Shared?>(null)

    /** Whether the app has asked to show notifications. Android stops asking after refusals. */
    var askedForNotifications: Boolean
        get() = settings.getBoolean(KEY_ASKED_NOTIFICATIONS, false)
        set(value) = settings.edit().putBoolean(KEY_ASKED_NOTIFICATIONS, value).apply()

    /** Who's connected, as the background notification says it. */
    var connectionStatus = "Looking for your computer"
        private set

    /** Whether files and text keep arriving with the app closed. */
    var receiveInBackground: Boolean
        get() = settings.getBoolean(KEY_BACKGROUND, true)
        set(value) {
            settings.edit().putBoolean(KEY_BACKGROUND, value).apply()
            applyBackground()
        }

    /**
     * Starts or stops the background service to match the setting. Android only allows
     * starting it while the app is on screen, so screens call this, not [onCreate].
     */
    fun applyBackground() {
        val service = Intent(this, ConnectionService::class.java)
        if (receiveInBackground) startForegroundService(service) else stopService(service)
    }

    override fun onCreate() {
        super.onCreate()
        coreBridge = ContinueCoreBridge.create()
        val dbFile = getDatabasePath("continue_android.db")
        dbFile.parentFile?.mkdirs()
        coreBridge.initCore(dbFile.absolutePath)
        coreBridge.setDeviceName(phoneName())
        reportBattery()
        applyVisibility(visible)
        createNotificationChannels(this)
        listen()
    }

    /** Takes in what paired computers send, for as long as the process runs. */
    private fun listen() {
        // On screen, the app shows these itself; otherwise they become notifications.
        state.incoming.onArrival = { title, detail, open -> if (!onScreen) notifyArrival(this, title, detail, open) }
        state.questions.onChange = { question ->
            if (question != null && !onScreen) notifyQuestion(this, question) else cancelQuestion(this)
        }
        scope.launch {
            state.recent.load()
            // With background receiving off, only while the app is on screen.
            state.incoming.listen(this@ContinueApplication, { saveFolder }) { onScreen || receiveInBackground }
        }
        scope.launch { state.questions.listen() }
        scope.launch { watchConnections() }
    }

    /** Keeps connection state fresh, often while on screen and now and then otherwise. */
    private suspend fun watchConnections() {
        while (true) {
            state.refresh()
            val status = describeConnections()
            if (status != connectionStatus) {
                connectionStatus = status
                if (receiveInBackground) updateBackgroundNotification(this)
            }
            delay(if (onScreen) ON_SCREEN_REFRESH_MS else BACKGROUND_REFRESH_MS)
        }
    }

    private fun describeConnections(): String {
        val names = state.peers.filter { it.fingerprint in state.connected }.map { it.displayName }
        return when {
            state.peers.isEmpty() -> "Not paired with a computer yet"
            names.isEmpty() -> "Looking for your computer"
            names.size == 1 -> "Connected to ${names.single()}"
            else -> "Connected to ${names.size} computers"
        }
    }

    /** The name the owner gave the phone in Settings → About phone, or its model. */
    private fun phoneName(): String {
        val ownerSet = Settings.Global.getString(contentResolver, Settings.Global.DEVICE_NAME)
        if (!ownerSet.isNullOrBlank()) return ownerSet
        val model = Build.MODEL.orEmpty()
        val maker = Build.MANUFACTURER.orEmpty().replaceFirstChar { it.uppercase() }
        return if (model.startsWith(maker, ignoreCase = true)) model else "$maker $model".trim()
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

    /** Keeps connected computers up to date with the battery. The core only sends changes. */
    private fun reportBattery() {
        val receiver =
            object : BroadcastReceiver() {
                override fun onReceive(
                    context: Context,
                    intent: Intent,
                ) {
                    val level = intent.getIntExtra(BatteryManager.EXTRA_LEVEL, -1)
                    val scale = intent.getIntExtra(BatteryManager.EXTRA_SCALE, -1)
                    if (level < 0 || scale <= 0) return
                    val charging = intent.getIntExtra(BatteryManager.EXTRA_PLUGGED, 0) != 0
                    coreBridge.setDeviceStatus(level * PERCENT / scale, charging)
                }
            }
        // The battery broadcast is sticky, so this also delivers the current reading.
        ContextCompat.registerReceiver(
            this,
            receiver,
            IntentFilter(Intent.ACTION_BATTERY_CHANGED),
            ContextCompat.RECEIVER_NOT_EXPORTED,
        )
    }

    private companion object {
        const val PERCENT = 100
        const val KEY_VISIBLE = "visible"
        const val KEY_THEME = "theme"
        const val KEY_WALLPAPER_COLORS = "wallpaper_colors"
        const val KEY_BACKGROUND = "receive_in_background"
        const val KEY_SEND_COPIES = "send_new_copies"
        const val KEY_COPY_SEEN = "last_copy_seen"
        const val KEY_ASKED_NOTIFICATIONS = "asked_notifications"
        const val KEY_SAVE_FOLDER = "save_folder"
        const val ON_SCREEN_REFRESH_MS = 2_000L
        const val BACKGROUND_REFRESH_MS = 10_000L
    }
}
