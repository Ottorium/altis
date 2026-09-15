package com.altis.app
import android.graphics.Color
import android.os.Bundle
import android.view.View
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Loads the native library and the notification channels before the webview can ask for
        // background polling. The service does the same, for when it is what starts the process.
        BackgroundPoller.prepare(this)

        // Android 15+ always draws edge-to-edge for apps targeting it, so do the same everywhere and
        // keep the webview out from under the bars by padding the content view with their insets
        WindowCompat.setDecorFitsSystemWindows(window, false)

        // The bars are transparent, so what shows behind them is the window background. Match it to
        // the app's dark background (bootstrap's bg-dark) and use light icons on top of it
        window.decorView.setBackgroundColor(Color.parseColor("#212529"))
        @Suppress("DEPRECATION")
        window.statusBarColor = Color.TRANSPARENT
        @Suppress("DEPRECATION")
        window.navigationBarColor = Color.TRANSPARENT
        WindowCompat.getInsetsController(window, window.decorView).apply {
            isAppearanceLightStatusBars = false
            isAppearanceLightNavigationBars = false
        }

        // The keyboard is included, because adjustResize doesn't work edge-to-edge and it would
        // otherwise cover the focused input
        val content = findViewById<View>(android.R.id.content)
        ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
            val bars = insets.getInsets(
                WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
            )
            val ime = insets.getInsets(WindowInsetsCompat.Type.ime())
            view.setPadding(bars.left, bars.top, bars.right, maxOf(bars.bottom, ime.bottom))
            WindowInsetsCompat.CONSUMED
        }
    }
}