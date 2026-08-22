package com.aip.companion

import java.io.BufferedReader
import java.io.BufferedWriter
import java.io.InputStreamReader
import java.io.OutputStreamWriter
import java.net.InetSocketAddress
import java.net.Socket
import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import javax.crypto.Mac
import javax.crypto.spec.SecretKeySpec

const val PROTOCOL = "aip-companion-v1"
const val MAX_FRAME = 16384
const val MAX_TEXT = 4096
const val MAX_PAYLOAD = 8192
const val MAX_QUEUE = 16

sealed class ProtocolError : Exception() { data object Version:ProtocolError(); data object Oversized:ProtocolError(); data object Replay:ProtocolError(); data object Auth:ProtocolError(); data object Revoked:ProtocolError(); data object Fields:ProtocolError(); data object Offline:ProtocolError() }
private fun hex(b:ByteArray)=b.joinToString(""){ "%02x".format(it) }
private fun unhex(s:String)=if(s.length%2!=0||!s.matches(Regex("[0-9a-f]+"))) throw ProtocolError.Auth else s.chunked(2).map{it.toInt(16).toByte()}.toByteArray()
private fun esc(s:String)=s.replace("\\","\\\\").replace("\"","\\\"").replace("\n","\\n").replace("\r","\\r")
private fun unesc(s:String)=s.replace("\\n","\n").replace("\\r","\r").replace("\\\"","\"").replace("\\\\","\\")

data class Frame(val protocol:String,val kind:String,val clientId:String,val sessionId:String?,val nonce:String,val counter:Long,val payload:String,val mac:String)

object FrameCodec {
    fun canonical(f:Frame)=listOf(f.protocol,f.kind,f.clientId,f.sessionId?:"",f.nonce,f.counter.toString(),f.payload).joinToString("\u001f").toByteArray(StandardCharsets.UTF_8)
    fun encode(f:Frame):String { if(f.payload.toByteArray().size>MAX_PAYLOAD) throw ProtocolError.Oversized; val s="{\"protocol\":\"${esc(f.protocol)}\",\"kind\":\"${esc(f.kind)}\",\"clientId\":\"${esc(f.clientId)}\",\"sessionId\":${f.sessionId?.let{"\"${esc(it)}\""}?:"null"},\"nonce\":\"${esc(f.nonce)}\",\"counter\":${f.counter},\"payload\":\"${esc(f.payload)}\",\"mac\":\"${f.mac}\"}"; if(s.toByteArray().size>MAX_FRAME) throw ProtocolError.Oversized; return s+"\n" }
    fun decode(raw:String):Frame { val s=raw.removeSuffix("\n"); if(s.toByteArray().size>MAX_FRAME) throw ProtocolError.Oversized; val r=Regex("^\\{\\\"protocol\\\":\\\"(.*?)\\\",\\\"kind\\\":\\\"(.*?)\\\",\\\"clientId\\\":\\\"(.*?)\\\",\\\"sessionId\\\":(null|\\\"(.*?)\\\"),\\\"nonce\\\":\\\"(.*?)\\\",\\\"counter\\\":([0-9]+),\\\"payload\\\":\\\"(.*?)\\\",\\\"mac\\\":\\\"([0-9a-f]+)\\\"\\}$"); val m=r.matchEntire(s)?:throw ProtocolError.Fields; val c=m.groupValues[7].toLongOrNull()?:throw ProtocolError.Fields; if(c<0||m.groupValues[9].length!=64) throw ProtocolError.Fields; return Frame(unesc(m.groupValues[1]),unesc(m.groupValues[2]),unesc(m.groupValues[3]),if(m.groupValues[4]=="null")null else unesc(m.groupValues[5]),unesc(m.groupValues[6]),c,unesc(m.groupValues[8]),m.groupValues[9]) }
}

class CompanionProtocol(private var key:ByteArray, private val clientId:String="android-companion") {
    private var counter=0L; private val nonces=HashSet<String>(); private var revoked=false
    private fun mac(f:Frame)=hex(Mac.getInstance("HmacSHA256").apply{init(SecretKeySpec(key,"HmacSHA256"))}.doFinal(FrameCodec.canonical(f)))
    fun revoke(){revoked=true}; fun rotate(newKey:ByteArray){require(newKey.isNotEmpty());key=newKey;counter=0;nonces.clear()}
    fun challenge(value:String)=hex(Mac.getInstance("HmacSHA256").apply{init(SecretKeySpec(key,"HmacSHA256"))}.doFinal(value.toByteArray()))
    fun frame(kind:String,payload:String="",sessionId:String?=null,nonce:String="n${counter+1}"):String { if(revoked)throw ProtocolError.Revoked; require(kind.isNotEmpty()&&kind.length<=MAX_TEXT&&payload.toByteArray().size<=MAX_PAYLOAD&&nonce.length in 1..128); val f=Frame(PROTOCOL,kind,clientId,sessionId,nonce,++counter,payload,""); nonces+=nonce; return FrameCodec.encode(f.copy(mac=mac(f))) }
    fun accept(raw:String):Frame { if(revoked)throw ProtocolError.Revoked; val f=FrameCodec.decode(raw); if(f.protocol!=PROTOCOL)throw ProtocolError.Version; if(f.counter<=counter||!nonces.add(f.nonce))throw ProtocolError.Replay; if(f.payload.toByteArray().size>MAX_PAYLOAD||!MessageDigest.isEqual(unhex(f.mac),unhex(mac(f))))throw ProtocolError.Auth; counter=f.counter; return f }
}

interface SocketTransport { fun request(frame:String):String? }
class CompanionSocketTransport(private val host:String,private val port:Int,private val timeoutMs:Int=1500):SocketTransport {
    override fun request(frame:String):String?=try{Socket().use{it.soTimeout=timeoutMs;it.connect(InetSocketAddress(host,port),timeoutMs); val w=BufferedWriter(OutputStreamWriter(it.getOutputStream(),Charsets.UTF_8));w.write(frame);w.flush(); BufferedReader(InputStreamReader(it.getInputStream(),Charsets.UTF_8)).readLine()}}catch(_:Exception){null}
}

class OutgoingQueue { private val items=ArrayDeque<String>(); fun preview(text:String){require(text.toByteArray().size in 1..MAX_TEXT&&items.size<MAX_QUEUE);items.addLast(text)};fun approve()=items.removeFirstOrNull();fun cancel()=items.removeFirstOrNull();fun retry(text:String)=preview(text);fun size()=items.size }
class ReadOnlyHistory(private val limit:Int=32){private val items=ArrayDeque<String>();fun add(text:String){if(text.toByteArray().size<=MAX_TEXT){if(items.size==limit)items.removeFirst();items.addLast(text)}};fun snapshot()=items.toList()}
