package org.continueapp.bridge

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.ContactsContract
import android.provider.ContactsContract.CommonDataKinds.Phone
import android.provider.Telephony.Sms
import android.telephony.SmsManager
import org.continueapp.bridge.ffi.ContactFfi
import org.continueapp.bridge.ffi.ConversationFfi
import org.continueapp.bridge.ffi.MessageStoreFfi
import org.continueapp.bridge.ffi.TextMessageFfi

/** How far back conversations are gathered from, so a long history isn't read every time. */
private const val SCAN_LIMIT = 2000

val MESSAGES_PERMISSIONS =
    arrayOf(Manifest.permission.READ_SMS, Manifest.permission.SEND_SMS, Manifest.permission.READ_CONTACTS)

fun canReadMessages(context: Context): Boolean =
    context.checkSelfPermission(Manifest.permission.READ_SMS) == PackageManager.PERMISSION_GRANTED

// SMS only: MMS and RCS messages aren't read.

internal class TelephonyMessages(
    private val context: Context,
) : MessageStoreFfi {
    override fun conversations(limit: UInt): List<ConversationFfi>? {
        if (!canReadMessages(context)) return null
        val latest = linkedMapOf<Long, ConversationFfi>()
        val columns = arrayOf(Sms.THREAD_ID, Sms.ADDRESS, Sms.BODY, Sms.DATE, Sms.TYPE, Sms.READ)
        context.contentResolver.query(Sms.CONTENT_URI, columns, null, null, "${Sms.DATE} DESC")?.use { rows ->
            var scanned = 0
            while (latest.size < limit.toInt() && scanned++ < SCAN_LIMIT && rows.moveToNext()) {
                val thread = rows.getLong(0)
                if (thread in latest) continue
                val address = rows.getString(1).orEmpty()
                latest[thread] =
                    ConversationFfi(
                        id = thread.toString(),
                        address = address,
                        name = contactName(context, address).orEmpty(),
                        snippet = rows.getString(2).orEmpty(),
                        at = rows.getLong(3).toULong(),
                        unread = rows.getInt(4) == Sms.MESSAGE_TYPE_INBOX && rows.getInt(5) == 0,
                    )
            }
        }
        return latest.values.toList()
    }

    override fun conversation(
        id: String,
        limit: UInt,
    ): List<TextMessageFfi>? {
        if (!canReadMessages(context)) return null
        val texts = mutableListOf<TextMessageFfi>()
        val columns = arrayOf(Sms._ID, Sms.BODY, Sms.DATE, Sms.TYPE)
        context.contentResolver
            .query(Sms.CONTENT_URI, columns, "${Sms.THREAD_ID} = ?", arrayOf(id), "${Sms.DATE} DESC")
            ?.use { rows ->
                while (texts.size < limit.toInt() && rows.moveToNext()) {
                    texts +=
                        TextMessageFfi(
                            id = rows.getLong(0).toString(),
                            body = rows.getString(1).orEmpty(),
                            at = rows.getLong(2).toULong(),
                            outgoing = rows.getInt(3) != Sms.MESSAGE_TYPE_INBOX,
                        )
                }
            }
        return texts.asReversed()
    }

    override fun send(
        address: String,
        body: String,
    ): Boolean {
        if (context.checkSelfPermission(Manifest.permission.SEND_SMS) != PackageManager.PERMISSION_GRANTED) {
            return false
        }
        val sms =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                context.getSystemService(SmsManager::class.java)
            } else {
                @Suppress("DEPRECATION")
                SmsManager.getDefault()
            }
        return try {
            sms.sendMultipartTextMessage(address, null, sms.divideMessage(body), null, null)
            true
        } catch (_: IllegalArgumentException) {
            false
        } catch (_: SecurityException) {
            false
        }
    }

    override fun contacts(limit: UInt): List<ContactFfi>? {
        if (context.checkSelfPermission(Manifest.permission.READ_CONTACTS) != PackageManager.PERMISSION_GRANTED) {
            return null
        }
        val columns = arrayOf(Phone.DISPLAY_NAME, Phone.NUMBER, Phone.STARRED, Phone.PHOTO_THUMBNAIL_URI)
        val order = "${Phone.STARRED} DESC, ${Phone.DISPLAY_NAME} COLLATE NOCASE ASC"
        val contacts = mutableListOf<ContactFfi>()
        // A contact can list the same number more than once, written differently.
        val seen = mutableSetOf<String>()
        context.contentResolver.query(Phone.CONTENT_URI, columns, null, null, order)?.use { cursor ->
            while (contacts.size < limit.toInt() && cursor.moveToNext()) {
                val number = cursor.getString(1) ?: continue
                if (!seen.add(number.filter(Char::isDigit).takeLast(9))) continue
                val photo =
                    cursor.getString(3)?.let { uri ->
                        runCatching { context.contentResolver.openInputStream(Uri.parse(uri))?.use { it.readBytes() } }
                            .getOrNull()
                    }
                contacts +=
                    ContactFfi(
                        name = cursor.getString(0).orEmpty(),
                        number = number,
                        favorite = cursor.getInt(2) == 1,
                        photo = photo ?: ByteArray(0),
                    )
            }
        }
        return contacts
    }
}

/** Null without contacts permission or a match. */
fun contactName(
    context: Context,
    number: String,
): String? {
    if (number.isBlank() ||
        context.checkSelfPermission(Manifest.permission.READ_CONTACTS) != PackageManager.PERMISSION_GRANTED
    ) {
        return null
    }
    val lookup = Uri.withAppendedPath(ContactsContract.PhoneLookup.CONTENT_FILTER_URI, Uri.encode(number))
    return context.contentResolver
        .query(lookup, arrayOf(ContactsContract.PhoneLookup.DISPLAY_NAME), null, null, null)
        ?.use { if (it.moveToFirst()) it.getString(0) else null }
}
