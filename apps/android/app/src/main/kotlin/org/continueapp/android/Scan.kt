package org.continueapp.android

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Matrix
import android.graphics.pdf.PdfDocument
import android.media.ExifInterface
import android.net.Uri
import androidx.core.content.FileProvider
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/** A4's long side in PDF points; the page takes the photo's shape. */
private const val PAGE_LONG_SIDE = 842

/** Enough for print-sharp text without a huge file. */
private const val LONGEST_SIDE_PX = 2400

/** Where the camera app saves a photo for [scanPdf]. Shared through the app's FileProvider. */
fun scanPhotoUri(context: Context): Uri {
    val photo = File(context.cacheDir, "scans").apply { mkdirs() }.resolve("photo.jpg")
    return FileProvider.getUriForFile(context, "${context.packageName}.files", photo)
}

/** Turns the photo at [scanPhotoUri] upright and onto a PDF page, named for when it was taken. */
fun scanPdf(context: Context): Uri? {
    val folder = File(context.cacheDir, "scans")
    val photo = folder.resolve("photo.jpg")
    val page = uprightPhoto(photo) ?: return null
    // Earlier scans were sent already.
    folder.listFiles { file -> file.extension == "pdf" }?.forEach(File::delete)
    val scale = PAGE_LONG_SIDE.toFloat() / maxOf(page.width, page.height)
    val (width, height) = (page.width * scale).toInt() to (page.height * scale).toInt()
    val document = PdfDocument()
    val pdfPage = document.startPage(PdfDocument.PageInfo.Builder(width, height, 1).create())
    pdfPage.canvas.drawBitmap(page, null, android.graphics.Rect(0, 0, width, height), null)
    document.finishPage(pdfPage)
    val name = "Scan ${SimpleDateFormat("yyyy-MM-dd HH.mm", Locale.getDefault()).format(Date())}.pdf"
    val pdf = folder.resolve(name)
    pdf.outputStream().use(document::writeTo)
    document.close()
    photo.delete()
    return FileProvider.getUriForFile(context, "${context.packageName}.files", pdf)
}

private fun uprightPhoto(photo: File): Bitmap? {
    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    BitmapFactory.decodeFile(photo.path, bounds)
    if (bounds.outWidth <= 0) return null
    var sample = 1
    while (maxOf(bounds.outWidth, bounds.outHeight) / (sample * 2) >= LONGEST_SIDE_PX) sample *= 2
    val bitmap =
        BitmapFactory.decodeFile(
            photo.path,
            BitmapFactory.Options().apply {
                inSampleSize = sample
            },
        ) ?: return null
    val orientation = ExifInterface(photo.path).getAttributeInt(ExifInterface.TAG_ORIENTATION, 0)
    val degrees =
        when (orientation) {
            ExifInterface.ORIENTATION_ROTATE_90 -> 90
            ExifInterface.ORIENTATION_ROTATE_180 -> 180
            ExifInterface.ORIENTATION_ROTATE_270 -> 270
            else -> return bitmap
        }
    val turn = Matrix().apply { postRotate(degrees.toFloat()) }
    return Bitmap.createBitmap(bitmap, 0, 0, bitmap.width, bitmap.height, turn, true)
}
