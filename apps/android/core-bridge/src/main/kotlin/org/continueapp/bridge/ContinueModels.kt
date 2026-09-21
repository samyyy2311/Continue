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

enum class PermissionGrant(val rawValue: String) {
    ALWAYS_ALLOW("AlwaysAllow"),
    PROMPT("Prompt"),
    DENY("Deny"),
    ;

    companion object {
        fun fromRaw(raw: String): PermissionGrant {
            return entries.firstOrNull { it.rawValue.equals(raw, ignoreCase = true) } ?: PROMPT
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
