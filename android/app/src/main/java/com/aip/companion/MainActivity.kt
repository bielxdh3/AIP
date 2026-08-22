package com.aip.companion

import android.app.Activity
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.provider.Settings
import android.widget.*

class MainActivity : Activity() {
    private val queue = OutgoingQueue()
    private lateinit var store: SecureStore
    private var client: CompanionClient? = null
    override fun onCreate(state: Bundle?) {
        super.onCreate(state); store = SecureStore(this)
        getSystemService(NotificationManager::class.java).createNotificationChannel(NotificationChannel("status", "Status do AIP", NotificationManager.IMPORTANCE_LOW))
        val box = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(24, 24, 24, 24) }
        box.addView(TextView(this).apply { text = "AIP Companion\nModo independente/offline"; textSize = 22f })
        val status = TextView(this).apply { text = "Offline: nenhuma conexão iniciada\nHistórico local somente leitura" }; box.addView(status)
        val host = EditText(this).apply { hint = "Host privado (127.0.0.1)"; setText("127.0.0.1") }; box.addView(host)
        val port = EditText(this).apply { hint = "Porta"; inputType = 2 }; box.addView(port)
        val code = EditText(this).apply { hint = "Código de pairing (uso transitório)" }; box.addView(code)
        box.addView(Button(this).apply { text = "Conectar explicitamente"; setOnClickListener {
            try { val key = store.load() ?: code.text.toString().trim().chunked(2).map { it.toInt(16).toByte() }.toByteArray().also { store.save(it) }; val p = CompanionProtocol(key); client = CompanionClient(CompanionSocketTransport(host.text.toString().trim(), port.text.toString().toInt()), p); status.text = when (client!!.request("status", "{}")) { is ClientResult.Online -> "Conectado; resposta autenticada"; is ClientResult.Offline -> "Offline: desktop não respondeu" } } catch (_: Exception) { status.text = "Offline: parâmetros inválidos ou desktop indisponível" }
        } })
        box.addView(Button(this).apply { text = "Limpar pairing"; setOnClickListener { store.clear(); client = null; status.text = "Pairing removido; conexão desativada" } })
        val input = EditText(this).apply { hint = "Mensagem curta"; maxLines = 3 }; box.addView(input)
        box.addView(Button(this).apply { text = "Pré-visualizar (${queue.size()})"; setOnClickListener { try { queue.preview(input.text.toString()); input.text.clear(); status.text = "Aguardando aprovação local (fila ${queue.size()}/$MAX_QUEUE)" } catch (_: Exception) { status.text = "Mensagem fora do limite" } } })
        box.addView(Button(this).apply { text = "Cancelar pendente"; setOnClickListener { queue.cancel(); status.text = "Item cancelado" } })
        box.addView(Button(this).apply { text = "Ativar ícone flutuante"; setOnClickListener { if (!Settings.canDrawOverlays(this@MainActivity)) startActivity(Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION, Uri.parse("package:$packageName"))) else startService(Intent(this@MainActivity, OverlayService::class.java)) } })
        setContentView(box)
    }
}
