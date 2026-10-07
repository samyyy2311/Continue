package org.continueapp.bridge

import android.Manifest
import android.content.ContentResolver
import android.content.Context
import android.content.pm.PackageManager
import android.database.Cursor
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.provider.ContactsContract.CommonDataKinds.Phone
import android.provider.MediaStore
import android.provider.Telephony.Sms
import org.continueapp.bridge.ffi.PhoneSearchFfi
import org.continueapp.bridge.ffi.SearchKindFfi
import org.continueapp.bridge.ffi.SearchResultFfi

/** Looks through the phone's files, texts and contacts, each only with permission to read them. */
internal class PhoneContentSearch(
    private val context: Context,
) : PhoneSearchFfi {
    override fun search(
        query: String,
        limit: UInt,
    ): List<SearchResultFfi>? {
        val like = "%" + query.replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_") + "%"
        val search = Search(like, limit.toInt())
        val found = listOfNotNull(search.files(), search.texts(), search.contacts())
        if (found.isEmpty()) return null
        // Newest first, contacts after the rest.
        return found.flatten().sortedByDescending { it.at }.take(search.limit)
    }

    private fun allowed(permission: String) =
        context.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED

    /** One search, run against each place it looks. */
    private inner class Search(
        val like: String,
        val limit: Int,
    ) {
        fun files(): List<SearchResultFfi>? {
            val readsAll =
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    Environment.isExternalStorageManager()
                } else {
                    allowed(Manifest.permission.READ_EXTERNAL_STORAGE)
                }
            if (!readsAll) return null
            val root = Environment.getExternalStorageDirectory().path + "/"

            // _data is the only column with the full path; it's still filled in for apps that may read all files.
            @Suppress("DEPRECATION")
            val path = MediaStore.Files.FileColumns.DATA
            val columns =
                arrayOf(MediaStore.Files.FileColumns.DISPLAY_NAME, path, MediaStore.Files.FileColumns.DATE_MODIFIED)
            val selection =
                "${MediaStore.Files.FileColumns.DISPLAY_NAME} LIKE ? ESCAPE '\\' " +
                    "AND ${MediaStore.Files.FileColumns.MIME_TYPE} IS NOT NULL"
            return query(MediaStore.Files.getContentUri("external"), columns, selection, "${columns[2]} DESC") {
                val file = it.getString(1) ?: return@query null
                if (!file.startsWith(root)) return@query null
                val relative = file.removePrefix(root)
                SearchResultFfi(
                    kind = SearchKindFfi.FILE,
                    title = it.getString(0),
                    detail = relative.substringBeforeLast('/', ""),
                    reference = relative,
                    at = it.getLong(2).toULong() * 1000u,
                )
            }
        }

        fun texts(): List<SearchResultFfi>? {
            if (!canReadMessages(context)) return null
            val columns = arrayOf(Sms.THREAD_ID, Sms.ADDRESS, Sms.BODY, Sms.DATE)
            return query(Sms.CONTENT_URI, columns, "${Sms.BODY} LIKE ? ESCAPE '\\'", "${Sms.DATE} DESC") {
                val address = it.getString(1).orEmpty()
                SearchResultFfi(
                    kind = SearchKindFfi.TEXT,
                    title = contactName(context, address) ?: address,
                    detail = it.getString(2).orEmpty(),
                    reference = it.getLong(0).toString(),
                    at = it.getLong(it.getColumnIndexOrThrow(Sms.DATE)).toULong(),
                )
            }
        }

        fun contacts(): List<SearchResultFfi>? {
            if (!allowed(Manifest.permission.READ_CONTACTS)) return null
            val columns = arrayOf(Phone.DISPLAY_NAME, Phone.NUMBER)
            return query(Phone.CONTENT_URI, columns, "${Phone.DISPLAY_NAME} LIKE ? ESCAPE '\\'", Phone.DISPLAY_NAME) {
                SearchResultFfi(
                    kind = SearchKindFfi.CONTACT,
                    title = it.getString(0).orEmpty(),
                    detail = it.getString(1).orEmpty(),
                    reference = "",
                    at = 0u,
                )
            }
        }

        private fun query(
            uri: Uri,
            columns: Array<String>,
            selection: String,
            sort: String,
            row: (Cursor) -> SearchResultFfi?,
        ): List<SearchResultFfi> {
            val arguments =
                Bundle().apply {
                    putString(ContentResolver.QUERY_ARG_SQL_SELECTION, selection)
                    putStringArray(ContentResolver.QUERY_ARG_SQL_SELECTION_ARGS, arrayOf(like))
                    putString(ContentResolver.QUERY_ARG_SQL_SORT_ORDER, sort)
                    putInt(ContentResolver.QUERY_ARG_LIMIT, limit)
                }
            val results = mutableListOf<SearchResultFfi>()
            runCatching {
                context.contentResolver.query(uri, columns, arguments, null)?.use { cursor ->
                    while (results.size < limit && cursor.moveToNext()) row(cursor)?.let(results::add)
                }
            }
            return results
        }
    }
}
