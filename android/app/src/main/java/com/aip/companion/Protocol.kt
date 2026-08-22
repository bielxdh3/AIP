package com.aip.companion

import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import javax.crypto.Mac
import javax.crypto.spec.SecretKeySpec

const val PROTOCOL = "aip-companion-v1"
const val MAX_FRAME = 16384
const val MAX_TEXT = 4096
const val MAX_PAYLOAD = 8192
const val MAX_QUEUE = 16

sealed class ProtocolError : Exception() {
    data object Version : ProtocolError()
    data object Oversized : ProtocolError()
    data object Replay : ProtocolError()
    data object Auth : ProtocolError()
    data object Revoked : ProtocolError()
}

private fun hex(bytes: ByteArray) = bytes.joinToString("") { "%02x".format(it) }
private fun unhex(value: String) = value.chunked(2).map { it.toInt(16).toByte() }.toByteArray()

/** JVM-safe bounded wire codec. Fields are length-delimited, so payload text cannot alter framing. */
data class Frame(val version: String, val kind: String, val counter: Long, val payload: String, val mac: String)

object FrameCodec {
    fun encode(f: Frame): String {
        val fields = listOf(f.version, f.kind, f.counter.toString(), f.payload, f.mac)
        val raw = fields.joinToString("\n") { "${it.toByteArray(StandardCharsets.UTF_8).size}:$it" }
        if (raw.toByteArray(StandardCharsets.UTF_8).size > MAX_FRAME) throw ProtocolError.Oversized
        return raw
    }
    fun decode(raw: String): Frame {
        if (raw.toByteArray(StandardCharsets.UTF_8).size > MAX_FRAME) throw ProtocolError.Oversized
        var at = 0; val fields = mutableListOf<String>()
        repeat(5) {
            val colon = raw.indexOf(':', at); if (colon < at) throw ProtocolError.Auth
            val length = raw.substring(at, colon).toIntOrNull() ?: throw ProtocolError.Auth
            val start = colon + 1; val end = start + length
            if (length < 0 || end > raw.length) throw ProtocolError.Auth
            val value = raw.substring(start, end)
            if (value.toByteArray(StandardCharsets.UTF_8).size != length) throw ProtocolError.Auth
            fields += value; at = if (it == 4) end else end + 1
        }
        if (at != raw.length) throw ProtocolError.Auth
        return Frame(fields[0], fields[1], fields[2].toLongOrNull() ?: throw ProtocolError.Auth, fields[3], fields[4])
    }
}

class CompanionProtocol(private val key: ByteArray) {
    private var counter = 0L; private var revoked = false
    fun revoke() { revoked = true }
    fun rotate(newKey: ByteArray) = CompanionProtocol(newKey)
    private fun mac(bytes: ByteArray) = hex(Mac.getInstance("HmacSHA256").apply { init(SecretKeySpec(key, "HmacSHA256")) }.doFinal(bytes))
    private fun authenticated(v: Frame) = listOf(v.version, v.kind, v.counter, v.payload).joinToString("\n").toByteArray(StandardCharsets.UTF_8)
    fun frame(kind: String, payload: String = ""): String {
        if (revoked) throw ProtocolError.Revoked
        require(kind.isNotEmpty() && kind.length <= MAX_TEXT && payload.toByteArray(StandardCharsets.UTF_8).size <= MAX_PAYLOAD)
        val next = counter + 1; val unsigned = Frame(PROTOCOL, kind, next, payload, "")
        counter = next; return FrameCodec.encode(unsigned.copy(mac = mac(authenticated(unsigned))))
    }
    fun accept(raw: String): Frame {
        if (revoked) throw ProtocolError.Revoked
        val v = FrameCodec.decode(raw)
        if (v.version != PROTOCOL) throw ProtocolError.Version
        if (v.counter <= counter) throw ProtocolError.Replay
        if (v.payload.toByteArray(StandardCharsets.UTF_8).size > MAX_PAYLOAD || !MessageDigest.isEqual(unhex(v.mac), unhex(mac(authenticated(v))))) throw ProtocolError.Auth
        counter = v.counter; return v
    }
}

class OutgoingQueue {
    private val items = ArrayDeque<String>()
    fun preview(text: String) { require(text.toByteArray().size in 1..MAX_TEXT && items.size < MAX_QUEUE); items.addLast(text) }
    fun approve() = items.removeFirstOrNull()
    fun cancel() = items.removeFirstOrNull()
    fun retry(text: String) = preview(text)
    fun size() = items.size
}

class ReadOnlyHistory(private val limit: Int = 32) {
    private val items = ArrayDeque<String>()
    fun add(text: String) { if (text.toByteArray().size <= MAX_TEXT) { if (items.size == limit) items.removeFirst(); items.addLast(text) } }
    fun snapshot() = items.toList()
}
