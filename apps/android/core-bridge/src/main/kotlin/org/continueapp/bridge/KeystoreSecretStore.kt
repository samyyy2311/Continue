package org.continueapp.bridge

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import org.continueapp.bridge.ffi.SecretStoreFfi
import org.continueapp.bridge.ffi.SecretStoreFfiException
import java.io.File
import java.io.IOException
import java.security.GeneralSecurityException
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

private const val KEYSTORE = "AndroidKeyStore"
private const val WRAPPING_KEY = "continue-device-keys"
private const val TRANSFORMATION = "AES/GCM/NoPadding"
private const val KEY_BITS = 256
private const val IV_BYTES = 12
private const val TAG_BITS = 128

/**
 * Keeps the device keys sealed with an AES key that lives in the Android Keystore and never
 * leaves it. Each secret is a file in [dir] holding the IV and the ciphertext; its label is
 * bound in as associated data, so one sealed file can't stand in for another.
 */
internal class KeystoreSecretStore(
    private val dir: File,
) : SecretStoreFfi {
    override fun store(
        label: String,
        secret: ByteArray,
    ) = guarded {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, wrappingKey())
        cipher.updateAAD(label.toByteArray())
        val sealed = cipher.iv + cipher.doFinal(secret)
        // Written whole and then swapped in, so a crash never leaves half a key.
        dir.mkdirs()
        val partial = File(dir, "${fileName(label)}.partial")
        partial.writeBytes(sealed)
        if (!partial.renameTo(fileFor(label))) {
            throw SecretStoreFfiException.Unavailable("couldn't save $label")
        }
    }

    override fun load(label: String): ByteArray? =
        guarded {
            val file = fileFor(label)
            if (!file.exists()) return@guarded null
            val sealed = file.readBytes()
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(
                Cipher.DECRYPT_MODE,
                wrappingKey(),
                GCMParameterSpec(TAG_BITS, sealed, 0, IV_BYTES),
            )
            cipher.updateAAD(label.toByteArray())
            cipher.doFinal(sealed, IV_BYTES, sealed.size - IV_BYTES)
        }

    override fun delete(label: String) =
        guarded {
            val file = fileFor(label)
            if (file.exists() && !file.delete()) {
                throw SecretStoreFfiException.Unavailable("couldn't delete $label")
            }
        }

    private fun wrappingKey(): SecretKey {
        val keyStore = KeyStore.getInstance(KEYSTORE).apply { load(null) }
        (keyStore.getKey(WRAPPING_KEY, null) as? SecretKey)?.let { return it }
        val spec =
            KeyGenParameterSpec
                .Builder(WRAPPING_KEY, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(KEY_BITS)
                .build()
        return KeyGenerator
            .getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE)
            .apply { init(spec) }
            .generateKey()
    }

    private fun fileFor(label: String) = File(dir, "${fileName(label)}.sealed")

    private fun fileName(label: String) = label.replace(Regex("[^A-Za-z0-9_-]"), "_")

    /** Reports Keystore and file errors to the core as an unavailable store. */
    private inline fun <T> guarded(block: () -> T): T =
        try {
            block()
        } catch (error: GeneralSecurityException) {
            throw SecretStoreFfiException.Unavailable("$error")
        } catch (error: IOException) {
            throw SecretStoreFfiException.Unavailable("$error")
        }
}
