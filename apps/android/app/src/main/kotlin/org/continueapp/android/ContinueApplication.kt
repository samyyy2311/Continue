package org.continueapp.android

import android.annotation.SuppressLint
import android.app.Application
import android.app.WallpaperColors
import android.app.WallpaperManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.database.ContentObserver
import android.graphics.Bitmap
import android.net.Uri
import android.net.wifi.WifiManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import android.provider.Telephony
import androidx.annotation.RequiresApi
import androidx.compose.runtime.mutableStateOf
import androidx.core.content.ContextCompat
import androidx.core.graphics.drawable.toBitmap
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.ProcessLifecycleOwner
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.continueapp.android.ui.theme.ThemeMode
import org.continueapp.bridge.ContinueCoreBridge
import org.continueapp.bridge.canReadFiles
import org.continueapp.bridge.canReadMessages
import org.continueapp.bridge.canSeeCalls
import java.io.ByteArrayOutputStream

class ContinueApplication : Application() {
    lateinit var coreBridge: ContinueCoreBridge
        private set

    /** Shared by the screens and the background work, so both see the same thing. */
    val state by lazy { AppState(coreBridge) }

    val phoneScreen = PhoneScreen(this)
    val phoneCamera = PhoneCamera(this)
    private val phoneMedia = PhoneMedia(this)
    val phoneRinger = PhoneRinger(this)

    /** For work that outlives any one screen. */
    val scope = MainScope()

    val onScreen: Boolean
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

    /** Whether computers are kept from connecting. */
    var paused: Boolean
        get() = settings.getBoolean(KEY_PAUSED, false)
        set(value) {
            settings.edit().putBoolean(KEY_PAUSED, value).apply()
            // Closing connections waits on the network.
            scope.launch(Dispatchers.IO) { runCatching { coreBridge.setPaused(value) } }
        }

    val presence by lazy { PhonePresence(this) }

    /** Whether the phone sends its presence over Bluetooth, for computers that lock when it goes. */
    var announcesPresence: Boolean
        get() = settings.getBoolean(KEY_PRESENCE, false)
        set(value) {
            settings.edit().putBoolean(KEY_PRESENCE, value).apply()
            if (value) presence.start() else presence.stop()
        }

    /** Apps whose notifications aren't forwarded, muted from a computer. */
    var mutedApps: Set<String>
        get() = settings.getStringSet(KEY_MUTED_APPS, null).orEmpty()
        set(value) = settings.edit().putStringSet(KEY_MUTED_APPS, value).apply()

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
        coreBridge =
            ContinueCoreBridge.create(
                this,
                phoneScreen,
                phoneCamera,
                phoneMedia,
                phoneRinger,
                ControlService.pointerTarget,
            )
        val dbFile = getDatabasePath("continue_android.db")
        dbFile.parentFile?.mkdirs()
        coreBridge.initCore(dbFile.absolutePath)
        if (paused) coreBridge.setPaused(true)
        if (announcesPresence) presence.start()
        coreBridge.setDeviceName(phoneName())
        reportStatus()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O_MR1) reportLook()
        applyVisibility(visible)
        createNotificationChannels(this)
        watchPhone()
        listen()
    }

    /** Takes in what paired computers send, for as long as the process runs. */
    private fun listen() {
        // On screen, the app shows these itself; otherwise they become notifications.
        state.incoming.onArrival = { title, detail, open -> if (!onScreen) notifyArrival(this, title, detail, open) }
        scope.launch {
            state.recent.load()
            // With background receiving off, only while the app is on screen.
            state.incoming.listen(this@ContinueApplication, { saveFolder }) { onScreen || receiveInBackground }
        }
        scope.launch { watchConnections() }
        scope.launch(Dispatchers.IO) {
            while (true) coreBridge.nextNotificationEvent(EVENT_WAIT_MS)?.let(ContinueNotificationListener::handle)
        }
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

    /** Battery changes are pushed straight away; signal has no broadcast, so it's polled. */
    private fun reportStatus() {
        val report = { readPhoneStatus(this)?.let(coreBridge::setDeviceStatus) }
        val receiver =
            object : BroadcastReceiver() {
                override fun onReceive(
                    context: Context,
                    intent: Intent,
                ) {
                    report()
                }
            }
        ContextCompat.registerReceiver(
            this,
            receiver,
            IntentFilter(Intent.ACTION_BATTERY_CHANGED),
            ContextCompat.RECEIVER_NOT_EXPORTED,
        )
        scope.launch {
            while (true) {
                delay(SIGNAL_EVERY_MS)
                report()
            }
        }
    }

    private var watchingCalls = false
    private var watchingTexts = false
    private var watchingDrop = false
    private var textsChanged: Job? = null

    /** Safe to call again once the user grants more access. */
    fun watchPhone() {
        NewPhotoJob.schedule(this)
        phoneMedia.watch()
        if (!watchingDrop && canReadFiles(this)) {
            watchingDrop = true
            DropFolder(this).start()
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O_MR1) reportLook()
        if (!watchingCalls && canSeeCalls(this)) {
            watchingCalls = true
            CallWatcher(this, coreBridge).start()
        }
        if (watchingTexts || !canReadMessages(this)) return
        watchingTexts = true
        val observer =
            object : ContentObserver(Handler(Looper.getMainLooper())) {
                override fun onChange(selfChange: Boolean) {
                    // One text arriving changes several rows; computers hear about it once.
                    textsChanged?.cancel()
                    textsChanged =
                        scope.launch(Dispatchers.IO) {
                            delay(TEXTS_SETTLE_MS)
                            coreBridge.announceMessagesChanged()
                        }
                }
            }
        contentResolver.registerContentObserver(Telephony.Sms.CONTENT_URI, true, observer)
    }

    /** Computers draw the phone with its wallpaper, or its main colour when the image can't be read. */
    @RequiresApi(Build.VERSION_CODES.O_MR1)
    private fun reportLook() {
        val wallpapers = getSystemService(WallpaperManager::class.java) ?: return
        val report = { colors: WallpaperColors? ->
            // Live wallpapers often have no colours; Material You's are drawn from the wallpaper too.
            val materialYou =
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                    getColor(android.R.color.system_accent1_400)
                } else {
                    null
                }
            val color = colors?.primaryColor?.toArgb() ?: materialYou ?: 0
            coreBridge.setLook(color and RGB, wallpaperImage(wallpapers))
        }
        report(wallpapers.getWallpaperColors(WallpaperManager.FLAG_SYSTEM))
        wallpapers.addOnColorsChangedListener(
            { colors, which -> if (which and WallpaperManager.FLAG_SYSTEM != 0) report(colors) },
            Handler(Looper.getMainLooper()),
        )
    }

    /** A live wallpaper's own thumbnail needs no permission; a picture needs file access. */
    @SuppressLint("MissingPermission") // canReadFiles checks it.
    private fun wallpaperImage(wallpapers: WallpaperManager): ByteArray? {
        val picture =
            wallpapers.wallpaperInfo?.loadThumbnail(packageManager)
                ?: if (canReadFiles(this)) runCatching { wallpapers.drawable }.getOrNull() else null
        picture ?: return null
        val height = WALLPAPER_HEIGHT
        val width = (height.toLong() * picture.intrinsicWidth / picture.intrinsicHeight.coerceAtLeast(1)).toInt()
        val bitmap = picture.toBitmap(width.coerceAtLeast(1), height)
        return ByteArrayOutputStream().use {
            bitmap.compress(Bitmap.CompressFormat.JPEG, WALLPAPER_QUALITY, it)
            it.toByteArray()
        }
    }

    private companion object {
        const val RGB = 0xFFFFFF
        const val WALLPAPER_HEIGHT = 640
        const val WALLPAPER_QUALITY = 80
        const val EVENT_WAIT_MS = 1_000L
        const val TEXTS_SETTLE_MS = 500L
        const val SIGNAL_EVERY_MS = 30_000L
        const val KEY_VISIBLE = "visible"
        const val KEY_THEME = "theme"
        const val KEY_WALLPAPER_COLORS = "wallpaper_colors"
        const val KEY_BACKGROUND = "receive_in_background"
        const val KEY_SEND_COPIES = "send_new_copies"
        const val KEY_PAUSED = "paused"
        const val KEY_PRESENCE = "presence"
        const val KEY_MUTED_APPS = "muted_apps"
        const val KEY_COPY_SEEN = "last_copy_seen"
        const val KEY_ASKED_NOTIFICATIONS = "asked_notifications"
        const val KEY_SAVE_FOLDER = "save_folder"
        const val ON_SCREEN_REFRESH_MS = 2_000L
        const val BACKGROUND_REFRESH_MS = 10_000L
    }
}
