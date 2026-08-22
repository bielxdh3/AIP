package com.aip.companion

import android.app.*
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.Build
import android.provider.Settings
import android.content.pm.PackageManager
import android.widget.*

class MainActivity : Activity() {
    private val queue = OutgoingQueue()
    private lateinit var store: SecureStore
    private var client: CompanionClient? = null
    private val history = ReadOnlyHistory()
    private var lastPending: String? = null
    private lateinit var status: TextView
    private fun notifyStatus(text: String) { try { getSystemService(NotificationManager::class.java).notify(12, Notification.Builder(this, "status").setSmallIcon(android.R.drawable.ic_dialog_info).setContentTitle("AIP Companion").setContentText(text).setAutoCancel(true).build()) } catch (_: SecurityException) {} }
    private fun requestNotifications() { if (Build.VERSION.SDK_INT >= 33 && checkSelfPermission("android.permission.POST_NOTIFICATIONS") != PackageManager.PERMISSION_GRANTED) requestPermissions(arrayOf("android.permission.POST_NOTIFICATIONS"), 7) }
    override fun onCreate(state: Bundle?) {
        super.onCreate(state); store = SecureStore(this)
        getSystemService(NotificationManager::class.java).createNotificationChannel(NotificationChannel("status", "Status do AIP", NotificationManager.IMPORTANCE_LOW)); requestNotifications()
        val box = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(24, 24, 24, 24) }
        box.addView(TextView(this).apply { text = "AIP Companion\nModo independente/offline"; textSize = 22f })
        status = TextView(this).apply { text = "Offline: nenhuma conexão iniciada" }; box.addView(status)
        val historyView = TextView(this).apply { text = "Histórico local somente leitura\n(vazio)"; setPadding(0, 12, 0, 12) }; box.addView(historyView)
        box.addView(Button(this).apply { text = "Atualizar histórico"; setOnClickListener { historyView.text = "Histórico local somente leitura\n" + (history.snapshot().joinToString("\n").ifEmpty { "(vazio)" }) } })
        val host = EditText(this).apply { hint = "Host privado (127.0.0.1)"; setText("127.0.0.1") }; box.addView(host)
        val port = EditText(this).apply { hint = "Porta"; inputType = 2 }; box.addView(port)
        val code = EditText(this).apply { hint = "Código de pairing (uso transitório)" }; box.addView(code)
        box.addView(Button(this).apply { text = "Conectar explicitamente"; setOnClickListener {
            try { val key = store.load() ?: code.text.toString().trim().chunked(2).map { it.toInt(16).toByte() }.toByteArray().also { store.save(it) }; val p = CompanionProtocol(key); client = CompanionClient(CompanionSocketTransport(host.text.toString().trim(), port.text.toString().toInt()), p); status.text = when (client!!.request("status", "{}")) { is ClientResult.Online -> "Conectado; resposta autenticada"; is ClientResult.Offline -> "Offline: desktop não respondeu" }.also { notifyStatus(it) } } catch (_: Exception) { status.text = "Offline: parâmetros inválidos ou desktop indisponível"; notifyStatus(status.text.toString()) }
        } })
        box.addView(Button(this).apply { text = "Limpar pairing"; setOnClickListener { store.clear(); client = null; status.text = "Pairing removido; conexão desativada" } })
        val input = EditText(this).apply { hint = "Mensagem curta"; maxLines = 3 }; box.addView(input)
        box.addView(Button(this).apply { text = "Pré-visualizar (${queue.size()})"; setOnClickListener { try { lastPending = input.text.toString(); queue.preview(lastPending!!); input.text.clear(); status.text = "Aguardando aprovação local (fila ${queue.size()}/$MAX_QUEUE)" } catch (_: Exception) { status.text = "Mensagem fora do limite" } } })
        box.addView(Button(this).apply { text = "Aprovar pendente"; setOnClickListener { queue.approve()?.let { history.add("Aprovado: item de texto (${it.length} caracteres)"); lastPending = null; status.text = "Item aprovado localmente"; notifyStatus(status.text.toString()) } ?: run { status.text = "Fila vazia" } } })
        box.addView(Button(this).apply { text = "Cancelar pendente"; setOnClickListener { lastPending = queue.cancel() ?: lastPending; status.text = "Item cancelado"; notifyStatus(status.text.toString()) } })
        box.addView(Button(this).apply { text = "Tentar novamente"; setOnClickListener { try { lastPending?.let { queue.retry(it); status.text = "Item recolocado para aprovação"; notifyStatus(status.text.toString()) } ?: run { status.text = "Nenhum item para repetir" } } catch (_: Exception) { status.text = "Fila cheia" } } })
        box.addView(Button(this).apply { text = "Ativar ícone flutuante"; setOnClickListener { if (!Settings.canDrawOverlays(this@MainActivity)) startActivity(Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION, Uri.parse("package:$packageName"))) else startService(Intent(this@MainActivity, OverlayService::class.java)) } })
        setContentView(box)
    }
}
