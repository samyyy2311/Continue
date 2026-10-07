package org.continueapp.android

import android.os.Environment
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import java.io.File

private const val POLL_MS = 3_000L

/**
 * Download/Continue Drop: what's put there goes to every paired computer, straight away or when
 * it next connects, and moves into Sent so it only goes once. Watched while the app runs.
 */
class DropFolder(
    private val app: ContinueApplication,
) {
    private val folder =
        File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS), "Continue Drop")

    fun start() {
        app.scope.launch(Dispatchers.IO) {
            File(folder, "Sent").mkdirs()
            var sizes = emptyMap<File, Long>()
            while (true) {
                delay(POLL_MS)
                val files = folder.listFiles { file -> file.isFile && !file.name.startsWith(".") }.orEmpty().toList()
                // Only once a file stops growing, so one still being copied in isn't cut short.
                val settled = files.filter { sizes[it] == it.length() }
                sizes = files.associateWith { it.length() } - settled.toSet()
                settled.forEach { file -> moveToSent(file)?.let(::send) }
            }
        }
    }

    /** Numbered if a file of that name went before. */
    private fun moveToSent(file: File): File? {
        val sent =
            generateSequence(1) { it + 1 }
                .map { n ->
                    val name = if (n == 1) file.name else "${file.nameWithoutExtension} ($n).${file.extension}"
                    File(File(folder, "Sent"), name.removeSuffix("."))
                }.first { !it.exists() }
        return sent.takeIf { file.renameTo(it) }
    }

    private fun send(file: File) {
        val bridge = app.coreBridge
        for (peer in runCatching { bridge.listTrustedPeers() }.getOrDefault(emptyList())) {
            // A failure leaves it in Sent, from where it can be put back to try again.
            runCatching {
                if (bridge.isPeerConnected(peer.fingerprint)) {
                    bridge.sendFile(peer.fingerprint, file.absolutePath)
                } else {
                    // Kept until it connects; the core takes this copy, so Sent keeps the file.
                    val copy = File(app.cacheDir, "outgoing/${System.nanoTime()}").apply { mkdirs() }.resolve(file.name)
                    bridge.sendFileLater(peer.fingerprint, file.copyTo(copy).absolutePath)
                }
            }
        }
    }
}
