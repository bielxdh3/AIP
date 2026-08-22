package com.aip.companion

import android.app.Service
import android.content.Intent
import android.graphics.Color
import android.graphics.PixelFormat
import android.os.IBinder
import android.provider.Settings
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import android.widget.TextView

class OverlayService : Service() {
    private var window: WindowManager? = null
    private var view: View? = null
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (!Settings.canDrawOverlays(this) || view != null) return START_NOT_STICKY
        val label = TextView(this).apply { text = "AIP"; setTextColor(Color.WHITE); setBackgroundColor(Color.rgb(40, 40, 48)); setPadding(18, 10, 18, 10); contentDescription = "Ícone do AIP" }
        val params = WindowManager.LayoutParams(WindowManager.LayoutParams.WRAP_CONTENT, WindowManager.LayoutParams.WRAP_CONTENT, WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY, WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL, PixelFormat.TRANSLUCENT).apply { gravity = Gravity.TOP or Gravity.END; x = 16; y = 96 }
        window = getSystemService(WINDOW_SERVICE) as WindowManager
        return try { window?.addView(label, params); view = label; START_NOT_STICKY } catch (_: SecurityException) { stopSelf(); START_NOT_STICKY }
    }
    override fun onDestroy() { view?.let { runCatching { window?.removeView(it) } }; view = null; window = null; super.onDestroy() }
    override fun onBind(intent: Intent?): IBinder? = null
}
