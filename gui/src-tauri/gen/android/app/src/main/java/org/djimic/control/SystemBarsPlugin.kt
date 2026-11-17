package org.djimic.control

import android.app.Activity
import android.graphics.Color
import androidx.core.view.WindowCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@InvokeArg
class BarStyle {
    var dark: Boolean = false
    /** The page background as a CSS hex colour, e.g. "#0b0b0c". */
    var color: String? = null
}

/**
 * Matches the status and navigation bars to the app's own light/dark theme,
 * which the user can set independently of the system's.
 */
@TauriPlugin
class SystemBarsPlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun setSystemBars(invoke: Invoke) {
        val style = invoke.parseArgs(BarStyle::class.java)
        activity.runOnUiThread {
            style.color
                ?.let { runCatching { Color.parseColor(it) }.getOrNull() }
                ?.let { activity.window.decorView.setBackgroundColor(it) }
            WindowCompat.getInsetsController(activity.window, activity.window.decorView).apply {
                isAppearanceLightStatusBars = !style.dark
                isAppearanceLightNavigationBars = !style.dark
            }
            invoke.resolve()
        }
    }
}
