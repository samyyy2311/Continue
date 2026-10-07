package org.continueapp.bridge

const val CAPABILITY_CONTROL_ID = 0
const val CAPABILITY_FILE_TRANSFER_ID = 1
const val CAPABILITY_CLIPBOARD_ID = 2
const val CAPABILITY_NOTIFICATIONS_ID = 3
const val CAPABILITY_PHOTOS_ID = 4
const val CAPABILITY_MESSAGES_ID = 5
const val CAPABILITY_FILES_ID = 6
const val CAPABILITY_CALLS_ID = 7
const val CAPABILITY_SCREEN_ID = 8
const val CAPABILITY_CAMERA_ID = 9
const val CAPABILITY_MEDIA_ID = 10
const val CAPABILITY_FIND_ID = 11
const val CAPABILITY_POINTER_ID = 12
const val CAPABILITY_ACTIONS_ID = 13
const val CAPABILITY_SNIPPETS_ID = 14
const val CAPABILITY_SEARCH_ID = 15

enum class CallState { RINGING, TALKING, ENDED }

/** Signal bars run 0–4; null without that network. */
data class PhoneStatus(
    val batteryPercent: Int,
    val charging: Boolean,
    val cellBars: Int?,
    val wifiBars: Int?,
    val carrier: String,
    /** "5G", "LTE" and so on; empty when unknown. */
    val network: String,
)

/** A computer showing a pairing code on this network. */
data class NearbyComputer(
    val name: String,
    val code: String,
)

data class TrustedPeer(
    val fingerprint: String,
    val displayName: String,
    val pairedAt: Long,
)

enum class Capability(val id: Int) {
    CONTROL(CAPABILITY_CONTROL_ID),
    FILE_TRANSFER(CAPABILITY_FILE_TRANSFER_ID),
    CLIPBOARD(CAPABILITY_CLIPBOARD_ID),
    NOTIFICATIONS(CAPABILITY_NOTIFICATIONS_ID),
    PHOTOS(CAPABILITY_PHOTOS_ID),
    MESSAGES(CAPABILITY_MESSAGES_ID),
    FILES(CAPABILITY_FILES_ID),
    CALLS(CAPABILITY_CALLS_ID),
    SCREEN(CAPABILITY_SCREEN_ID),
    CAMERA(CAPABILITY_CAMERA_ID),
    MEDIA(CAPABILITY_MEDIA_ID),
    FIND(CAPABILITY_FIND_ID),
    POINTER(CAPABILITY_POINTER_ID),
    ACTIONS(CAPABILITY_ACTIONS_ID),
    SNIPPETS(CAPABILITY_SNIPPETS_ID),
    SEARCH(CAPABILITY_SEARCH_ID),
    ;

    companion object {
        fun fromId(id: Int): Capability? = entries.firstOrNull { it.id == id }
    }
}

/** A file or text a paired device sent to this phone. */
sealed interface Received {
    /** Its saved history entry, if saving worked. */
    val historyId: Long?
    val peerFingerprint: String
    val peerName: String
}

/** A file waiting at [path] in the app's own storage. */
data class ReceivedFile(
    override val historyId: Long?,
    override val peerFingerprint: String,
    override val peerName: String,
    val path: String,
    val name: String,
    val size: Long,
) : Received

data class ReceivedText(
    override val historyId: Long?,
    override val peerFingerprint: String,
    override val peerName: String,
    val text: String,
) : Received

/** A PNG a computer copied, for the clipboard. Images aren't kept in history. */
class ReceivedImage(
    override val peerFingerprint: String,
    override val peerName: String,
    val png: ByteArray,
) : Received {
    override val historyId: Long? = null
}

/** One of this phone's notifications, to show on connected computers. */
data class PhoneNotification(
    val id: String,
    val packageName: String,
    val appName: String,
    val title: String,
    val text: String,
    /** Unix time in milliseconds. */
    val postedAt: Long,
    val buttons: List<NotificationButton>,
)

data class NotificationButton(
    val id: String,
    val label: String,
    /** Whether it takes typed text, like a messaging app's Reply. */
    val isReply: Boolean,
)

/** What a computer did with one of this phone's notifications. */
sealed interface NotificationEvent {
    data class Pressed(
        val notificationId: String,
        val buttonId: String,
        val replyText: String,
    ) : NotificationEvent

    data class Dismissed(
        val notificationId: String,
    ) : NotificationEvent

    /** The computer doesn't want this app's notifications any more. */
    data class Muted(
        val packageName: String,
    ) : NotificationEvent
}

/** A saved send or receive. */
data class HistoryEntry(
    val id: Long,
    /** Unix time in milliseconds. */
    val at: Long,
    val received: Boolean,
    val isText: Boolean,
    /** The file name, or the text itself. */
    val label: String,
    val peerFingerprint: String,
    val peerName: String,
    val size: Long,
    val failed: Boolean,
    /** Where the file is on this phone, when known. */
    val location: String?,
)

sealed class ContinueException(
    message: String,
    cause: Throwable? = null,
) : Exception(message, cause) {
    class InternalErrorException(message: String) : ContinueException(message)

    class InvalidQrException(message: String) : ContinueException(message)

    class PairingFailedException(message: String) : ContinueException(message)

    class DatabaseErrorException(message: String) : ContinueException(message)

    class NotInitializedException(message: String) : ContinueException(message)
}

/** A file on its way in. */
data class IncomingFile(
    val transferId: String,
    val peerName: String,
    val fileName: String,
    val received: Long,
    val total: Long,
)
