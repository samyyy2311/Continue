package org.continueapp.android

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts

/** Camera services must start while the app is visible; this starts one, asking for the permission first. */
class CameraStartActivity : ComponentActivity() {
    private val ask =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            if (granted) start() else (application as ContinueApplication).phoneCamera.answered(null)
            finish()
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (savedInstanceState != null) return
        if (checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
            start()
            finish()
        } else {
            ask.launch(Manifest.permission.CAMERA)
        }
    }

    private fun start() {
        val maxSize = intent.getIntExtra(EXTRA_MAX_SIZE, CameraService.DEFAULT_MAX_SIZE)
        startForegroundService(CameraService.start(this, maxSize, intent.getBooleanExtra(EXTRA_FRONT_CAMERA, true)))
    }
}
