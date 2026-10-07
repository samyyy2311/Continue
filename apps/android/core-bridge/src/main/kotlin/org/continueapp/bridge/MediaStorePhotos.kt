package org.continueapp.bridge

import android.Manifest
import android.content.ContentUris
import android.content.Context
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import android.util.Size
import org.continueapp.bridge.ffi.PhotoFfi
import org.continueapp.bridge.ffi.PhotoLibraryFfi
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.IOException

private const val THUMBNAIL_PX = 256
private const val THUMBNAIL_QUALITY = 70
private const val KEEP_COPIES_MS = 60 * 60 * 1000L
private const val MS_PER_SECOND = 1000L
private const val JUST_TAKEN_MS = 60 * 1000L

@Suppress("DEPRECATION")
private val PATH = MediaStore.Images.Media.DATA

/** Where the camera and screenshots save. */
private val CAMERA_OR_SCREENSHOT = "$PATH LIKE '%/DCIM/%' OR $PATH LIKE '%/Screenshots/%'"

val PHOTOS_PERMISSION =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        Manifest.permission.READ_MEDIA_IMAGES
    } else {
        Manifest.permission.READ_EXTERNAL_STORAGE
    }

fun canReadPhotos(context: Context): Boolean =
    context.checkSelfPermission(PHOTOS_PERMISSION) == PackageManager.PERMISSION_GRANTED

fun canReadFiles(context: Context): Boolean =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
        Environment.isExternalStorageManager()
    } else {
        context.checkSelfPermission(Manifest.permission.READ_EXTERNAL_STORAGE) == PackageManager.PERMISSION_GRANTED
    }

internal class MediaStorePhotos(
    private val context: Context,
) : PhotoLibraryFfi {
    private val copies = File(context.cacheDir, "photos")

    override fun recent(limit: UInt): List<PhotoFfi>? = if (canReadPhotos(context)) query(null, limit.toInt()) else null

    /** The newest camera photo or screenshot, if it was taken in the last minute. */
    fun justTaken(): PhotoFfi? {
        if (!canReadPhotos(context)) return null
        val photo = query(CAMERA_OR_SCREENSHOT, 1).firstOrNull() ?: return null
        return photo.takeIf { System.currentTimeMillis() - it.takenAt.toLong() < JUST_TAKEN_MS }
    }

    private fun query(
        selection: String?,
        limit: Int,
    ): List<PhotoFfi> {
        val columns =
            arrayOf(
                MediaStore.Images.Media._ID,
                MediaStore.Images.Media.DISPLAY_NAME,
                MediaStore.Images.Media.DATE_TAKEN,
                MediaStore.Images.Media.DATE_ADDED,
            )
        val order = "${MediaStore.Images.Media.DATE_ADDED} DESC"
        val photos = mutableListOf<PhotoFfi>()
        context.contentResolver.query(IMAGES, columns, selection, null, order)?.use { rows ->
            val added = rows.getColumnIndexOrThrow(MediaStore.Images.Media.DATE_ADDED)
            while (photos.size < limit && rows.moveToNext()) {
                val id = rows.getLong(0)
                val thumbnail = thumbnail(id) ?: continue
                val takenAt = if (rows.isNull(2)) rows.getLong(added) * MS_PER_SECOND else rows.getLong(2)
                photos += PhotoFfi(id.toString(), rows.getString(1).orEmpty(), takenAt.toULong(), thumbnail)
            }
        }
        return photos
    }

    /** Copies the photo where the core can read it; MediaStore only hands out streams. */
    override fun file(id: String): String? {
        if (!canReadPhotos(context)) return null
        val uri = ContentUris.withAppendedId(IMAGES, id.toLongOrNull() ?: return null)
        val name = displayName(uri) ?: return null
        copies.mkdirs()
        // Copies older than an hour are stale; transfers finish well before that.
        copies.listFiles()?.filter { System.currentTimeMillis() - it.lastModified() > KEEP_COPIES_MS }
            ?.forEach { it.delete() }
        val copy = File(copies, File(name).name)
        return try {
            context.contentResolver.openInputStream(uri)?.use { input ->
                copy.outputStream().use { input.copyTo(it) }
            } ?: return null
            copy.absolutePath
        } catch (_: IOException) {
            null
        }
    }

    private fun displayName(uri: Uri): String? =
        context.contentResolver
            .query(uri, arrayOf(MediaStore.Images.Media.DISPLAY_NAME), null, null, null)
            ?.use { if (it.moveToFirst()) it.getString(0) else null }

    private fun thumbnail(id: Long): ByteArray? {
        val bitmap =
            try {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    val uri = ContentUris.withAppendedId(IMAGES, id)
                    context.contentResolver.loadThumbnail(uri, Size(THUMBNAIL_PX, THUMBNAIL_PX), null)
                } else {
                    @Suppress("DEPRECATION")
                    MediaStore.Images.Thumbnails.getThumbnail(
                        context.contentResolver,
                        id,
                        MediaStore.Images.Thumbnails.MINI_KIND,
                        null,
                    )
                }
            } catch (_: IOException) {
                null
            } ?: return null
        return ByteArrayOutputStream().use {
            bitmap.compress(Bitmap.CompressFormat.JPEG, THUMBNAIL_QUALITY, it)
            it.toByteArray()
        }
    }

    private companion object {
        val IMAGES: Uri = MediaStore.Images.Media.EXTERNAL_CONTENT_URI
    }
}
