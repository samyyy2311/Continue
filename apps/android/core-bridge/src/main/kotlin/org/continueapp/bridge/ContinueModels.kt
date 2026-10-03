package org.continueapp.bridge

const val CAPABILITY_CONTROL_ID = 0
const val CAPABILITY_FILE_TRANSFER_ID = 1
const val CAPABILITY_CLIPBOARD_ID = 2
const val CAPABILITY_NOTIFICATIONS_ID = 3

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
    ;

    companion object {
        fun fromId(id: Int): Capability? = entries.firstOrNull { it.id == id }
    }
}

/** A file or text a paired device sent to this phone. */
sealed interface Received {
    val peerFingerprint: String
    val peerName: String
}

/** A file waiting at [path] in the app's own storage. */
data class ReceivedFile(
    override val peerFingerprint: String,
    override val peerName: String,
    val path: String,
    val name: String,
    val size: Long,
) : Received

data class ReceivedText(
    override val peerFingerprint: String,
    override val peerName: String,
    val text: String,
) : Received

/** Something a device set to Ask wants to send. */
data class PermissionQuestion(
    val id: Long,
    val peerFingerprint: String,
    val peerName: String,
    val capability: Capability?,
    /** The file name, for files. */
    val detail: String?,
    /** Unix time in milliseconds when the core declines it if nobody answers. */
    val expiresAt: Long,
)

enum class PermissionAnswer {
    /** Just this once. */
    ALLOW,

    /** This one, and everything of this kind from the device from now on. */
    ALWAYS_ALLOW,
    DECLINE,
}

/** Grant strings as the Rust core reads and writes them. */
enum class PermissionGrant(val rawValue: String) {
    ALLOW("Allow"),
    ALLOW_ONCE("AllowOnce"),
    ASK("Ask"),
    DENY("Deny"),
    ;

    companion object {
        fun fromRaw(raw: String): PermissionGrant {
            return entries.firstOrNull { it.rawValue.equals(raw, ignoreCase = true) } ?: ASK
        }
    }
}

sealed class ContinueException(
    message: String,
    cause: Throwable? = null,
) : Exception(message, cause) {
    class InternalErrorException(message: String) : ContinueException(message)

    class InvalidQrException(message: String) : ContinueException(message)

    class PairingFailedException(message: String) : ContinueException(message)

    class PairingTimeoutException(message: String) : ContinueException(message)

    class DatabaseErrorException(message: String) : ContinueException(message)

    class NotInitializedException(message: String) : ContinueException(message)
}
