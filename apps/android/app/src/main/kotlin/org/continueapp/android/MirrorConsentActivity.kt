package org.continueapp.android

import android.Manifest
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts

private const val KEY_ASKED_FOR_SOUND = "asked_for_sound"

class MirrorConsentActivity : ComponentActivity() {
    private val ask =
        registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
            val consent = result.data
            if (result.resultCode == RESULT_OK && consent != null) {
                val share =
                    MirrorService.start(
                        this,
                        consent,
                        intent.getIntExtra(EXTRA_MAX_SIZE, MirrorService.DEFAULT_MAX_SIZE),
                    )
                startForegroundService(share)
            } else {
                (application as ContinueApplication).phoneScreen.answered(null)
            }
            finish()
        }

    /** Sharing goes ahead either way; without this the computer gets the picture but no sound. */
    private val allowSound =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { askToShare() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (savedInstanceState != null) return
        val asked = getPreferences(MODE_PRIVATE).getBoolean(KEY_ASKED_FOR_SOUND, false)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q && !asked && !canCaptureSound(this)) {
            // Asked once; after a no, it's in the app's permissions in Android's settings.
            getPreferences(MODE_PRIVATE).edit().putBoolean(KEY_ASKED_FOR_SOUND, true).apply()
            allowSound.launch(Manifest.permission.RECORD_AUDIO)
        } else {
            askToShare()
        }
    }

    private fun askToShare() {
        ask.launch(getSystemService(MediaProjectionManager::class.java).createScreenCaptureIntent())
    }
}
