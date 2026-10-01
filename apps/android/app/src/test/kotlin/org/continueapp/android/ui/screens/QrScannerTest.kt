package org.continueapp.android.ui.screens

import com.google.zxing.BarcodeFormat
import com.google.zxing.qrcode.QRCodeReader
import com.google.zxing.qrcode.QRCodeWriter
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class QrScannerTest {
    private val code = "AQFZ3x9kR2p0bG9uZy1wYWlyaW5nLWNvZGUtZm9yLXRlc3Q"

    @Test
    fun readsAQrCodeFromACameraFrame() {
        val matrix = QRCodeWriter().encode(code, BarcodeFormat.QR_CODE, SIZE, SIZE)
        // Camera frames often pad each row, so rows are wider than the image.
        val rowStride = SIZE + ROW_PADDING
        val frame = ByteArray(rowStride * SIZE) { WHITE }
        for (y in 0 until SIZE) {
            for (x in 0 until SIZE) {
                if (matrix[x, y]) frame[y * rowStride + x] = BLACK
            }
        }

        assertEquals(code, decodeQr(QRCodeReader(), frame, rowStride, SIZE, SIZE))
    }

    @Test
    fun ignoresFramesWithoutACode() {
        val blank = ByteArray(SIZE * SIZE) { WHITE }
        assertNull(decodeQr(QRCodeReader(), blank, SIZE, SIZE, SIZE))
    }

    private companion object {
        const val SIZE = 400
        const val ROW_PADDING = 32
        const val WHITE: Byte = -1
        const val BLACK: Byte = 0
    }
}
