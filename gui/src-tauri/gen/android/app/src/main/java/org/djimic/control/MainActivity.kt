package org.djimic.control

import android.os.Build
import android.os.Bundle
import android.view.View
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
      // No translucent scrim behind the 3-button navigation bar: it already
      // takes the app's background, with icons set for contrast.
      window.isNavigationBarContrastEnforced = false
    }
    // The app draws edge to edge (Android 15+ requires it), so keep the web
    // content clear of the status and navigation bars. The bars then show the
    // window background, which SystemBarsPlugin matches to the app's theme.
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
      val bars = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
      )
      view.setPadding(bars.left, bars.top, bars.right, bars.bottom)
      WindowInsetsCompat.CONSUMED
    }
  }
}
