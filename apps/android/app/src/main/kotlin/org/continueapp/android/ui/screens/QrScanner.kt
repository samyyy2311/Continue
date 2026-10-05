package org.continueapp.android.ui.screens

import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.PlanarYUVLuminanceSource
import com.google.zxing.ReaderException
import com.google.zxing.common.HybridBinarizer
import com.google.zxing.qrcode.QRCodeReader
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

/** Reports the first QR code the back camera reads, once. Wrap it in a new `key` to scan again. */
@Composable
fun QrScanner(
    onScanned: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val latestOnScanned by rememberUpdatedState(onScanned)
    val previewView = remember { PreviewView(context) }

    DisposableEffect(lifecycleOwner) {
        val analysisExecutor = Executors.newSingleThreadExecutor()
        val mainExecutor = ContextCompat.getMainExecutor(context)
        val providerFuture = ProcessCameraProvider.getInstance(context)
        val reported = AtomicBoolean(false)
        val reader = QRCodeReader()

        providerFuture.addListener({
            val preview = Preview.Builder().build()
            preview.setSurfaceProvider(previewView.surfaceProvider)
            val analysis =
                ImageAnalysis.Builder()
                    .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                    .build()
            analysis.setAnalyzer(analysisExecutor) { image ->
                val text = image.use { decodeQr(reader, it) }
                if (text != null && reported.compareAndSet(false, true)) {
                    mainExecutor.execute { latestOnScanned(text) }
                }
            }
            runCatching {
                val provider = providerFuture.get()
                provider.unbindAll()
                provider.bindToLifecycle(lifecycleOwner, CameraSelector.DEFAULT_BACK_CAMERA, preview, analysis)
            }
        }, mainExecutor)

        onDispose {
            if (providerFuture.isDone) {
                runCatching { providerFuture.get().unbindAll() }
            }
            analysisExecutor.shutdown()
        }
    }

    AndroidView(factory = { previewView }, modifier = modifier)
}

private fun decodeQr(
    reader: QRCodeReader,
    image: ImageProxy,
): String? {
    // The first plane of a YUV camera frame is the brightness channel ZXing reads.
    val plane = image.planes[0]
    val bytes = ByteArray(plane.buffer.remaining()).also { plane.buffer.get(it) }
    return decodeQr(reader, bytes, plane.rowStride, image.width, image.height)
}

internal fun decodeQr(
    reader: QRCodeReader,
    luminance: ByteArray,
    rowStride: Int,
    width: Int,
    height: Int,
): String? {
    val source = PlanarYUVLuminanceSource(luminance, rowStride, height, 0, 0, width, height, false)
    return try {
        reader.decode(BinaryBitmap(HybridBinarizer(source)), mapOf(DecodeHintType.TRY_HARDER to true)).text
    } catch (_: ReaderException) {
        null
    } finally {
        reader.reset()
    }
}
