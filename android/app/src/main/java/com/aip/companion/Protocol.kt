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
private fun unhex(s:String)=if(s.length!=64||!s.matches(Regex("[0-9a-f]{64}"))) throw ProtocolError.Auth else s.chunked(2).map{it.toInt(16).toByte()}.toByteArray()
private fun esc(s:String)=buildString{for(c in s)when(c){'\\'->append("\\\\");'"'->append("\\\"");'\n'->append("\\n");'\r'->append("\\r");'\t'->append("\\t");else->append(c)}}

data class Frame(val protocol:String,val kind:String,val clientId:String,val sessionId:String?,val nonce:String,val counter:Long,val payload:String,val mac:String)
object FrameCodec {
    fun canonical(f:Frame)=listOf(f.protocol,f.kind,f.clientId,f.sessionId?:"",f.nonce,f.counter.toString(),f.payload).joinToString("\u001f").toByteArray(StandardCharsets.UTF_8)
    fun encode(f:Frame):String { if(f.protocol!=PROTOCOL||f.kind.isEmpty()||f.kind.length>MAX_TEXT||f.clientId.isEmpty()||f.clientId.length>512||f.nonce.isEmpty()||f.nonce.length>128||f.counter<0||f.payload.toByteArray().size>MAX_PAYLOAD||!f.mac.matches(Regex("[0-9a-f]{64}")))throw ProtocolError.Fields; val s="{\"protocol\":\"${esc(f.protocol)}\",\"kind\":\"${esc(f.kind)}\",\"clientId\":\"${esc(f.clientId)}\",\"sessionId\":${f.sessionId?.let{"\"${esc(it)}\""}?:"null"},\"nonce\":\"${esc(f.nonce)}\",\"counter\":${f.counter},\"payload\":\"${esc(f.payload)}\",\"mac\":\"${f.mac}\"}"; if(s.toByteArray().size+1>MAX_FRAME)throw ProtocolError.Oversized; return "$s\n" }
    fun decode(raw:String):Frame { val s=raw.removeSuffix("\n"); if(raw.toByteArray().size>MAX_FRAME)throw ProtocolError.Oversized; val p=Parser(s); p.expect('{'); val values=linkedMapOf<String,Any?>(); val keys=listOf("protocol","kind","clientId","sessionId","nonce","counter","payload","mac"); for((i,key)in keys.withIndex()){if(i>0)p.expect(',');p.expectString(key);p.expect(':');values[key]=if(key=="sessionId")p.nullableString() else if(key=="counter")p.number() else p.string()};p.expect('}');if(!p.done())throw ProtocolError.Fields;return Frame(values["protocol"] as String,values["kind"] as String,values["clientId"] as String,values["sessionId"] as String?,values["nonce"] as String,(values["counter"] as Long),values["payload"] as String,values["mac"] as String).also{encode(it)} }
    private class Parser(private val s:String){var i=0;fun ws(){while(i<s.length&&s[i].isWhitespace())i++};fun expect(c:Char){ws();if(i>=s.length||s[i++]!=c)throw ProtocolError.Fields};fun expectString(v:String){if(string()!=v)throw ProtocolError.Fields};fun string():String{ws();if(i>=s.length||s[i++]!='"')throw ProtocolError.Fields;val o=StringBuilder();while(i<s.length){val c=s[i++];if(c=='"')return o.toString();if(c!='\\'){o.append(c);continue};if(i>=s.length)throw ProtocolError.Fields;when(val e=s[i++]){'"'->o.append('"');'\\'->o.append('\\');'n'->o.append('\n');'r'->o.append('\r');'t'->o.append('\t');'b'->o.append('\b');'f'->o.append('\u000c');'u'->{if(i+4>s.length)throw ProtocolError.Fields;o.append(s.substring(i,i+4).toIntOrNull(16)?.toChar()?:throw ProtocolError.Fields);i+=4};else->throw ProtocolError.Fields}};throw ProtocolError.Fields};fun nullableString():String?{ws();return if(s.startsWith("null",i)){i+=4;null}else string()};fun number():Long{ws();val st=i;while(i<s.length&&s[i].isDigit())i++;return s.substring(st,i).toLongOrNull()?:throw ProtocolError.Fields};fun done()=s.substring(i).trim().isEmpty()}
}

class CompanionProtocol(private var key:ByteArray, private val clientId:String="android-companion") {
    private var counter=0L;private var inboundCounter=0L;private val nonces=HashSet<String>();private val inboundNonces=HashSet<String>();private var revoked=false
    private fun mac(f:Frame)=hex(Mac.getInstance("HmacSHA256").apply{init(SecretKeySpec(key,"HmacSHA256"))}.doFinal(FrameCodec.canonical(f)))
    fun revoke(){revoked=true};fun rotate(newKey:ByteArray){require(newKey.size in 16..64);key=newKey;counter=0;inboundCounter=0;nonces.clear();inboundNonces.clear();revoked=false};fun challenge(value:String)=hex(Mac.getInstance("HmacSHA256").apply{init(SecretKeySpec(key,"HmacSHA256"))}.doFinal(value.toByteArray()))
    fun frame(kind:String,payload:String="",sessionId:String?=null,nonce:String="n${counter+1}"):String{if(revoked)throw ProtocolError.Revoked;require(kind.isNotEmpty()&&kind.length<=MAX_TEXT&&payload.toByteArray().size<=MAX_PAYLOAD&&nonce.length in 1..128);val f=Frame(PROTOCOL,kind,clientId,sessionId,nonce,++counter,payload,"");nonces+=nonce;return FrameCodec.encode(f.copy(mac=mac(f)))}
    fun accept(raw:String):Frame{if(revoked)throw ProtocolError.Revoked;val f=FrameCodec.decode(raw);if(f.protocol!=PROTOCOL||f.counter<0)throw ProtocolError.Version;if(f.counter<=inboundCounter||inboundNonces.contains(f.nonce))throw ProtocolError.Replay;if(!MessageDigest.isEqual(unhex(f.mac),unhex(mac(f))))throw ProtocolError.Auth;inboundNonces.add(f.nonce);inboundCounter=f.counter;counter=maxOf(counter,f.counter);return f}
}

sealed class ClientResult<out T>{data class Online<T>(val value:T):ClientResult<T>();data class Offline(val reason:String="offline"):ClientResult<Nothing>()}
interface SocketTransport{fun request(frame:String):String?}
private fun privateHost(host:String):Boolean{val p=host.split('.');if(p.size!=4||p.any{it.toIntOrNull()?.let{n->n !in 0..255}?:true})return false;val a=p[0].toInt();val b=p[1].toInt();return a==127||a==10||(a==192&&b==168)||(a==172&&b in 16..31)}
class CompanionSocketTransport(private val host:String,private val port:Int,private val timeoutMs:Int=1500):SocketTransport{init{require(privateHost(host)&&port in 1..65535&&timeoutMs in 1..30000)};override fun request(frame:String):String?=try{Socket().use{it.soTimeout=timeoutMs;it.connect(InetSocketAddress(host,port),timeoutMs);val w=BufferedWriter(OutputStreamWriter(it.getOutputStream(),Charsets.UTF_8));w.write(frame);w.flush();run { val b=ByteArray(MAX_FRAME+1); var n=0; while(n<b.size){ val c=it.getInputStream().read(); if(c<0 || c==10) break; b[n++]=c.toByte() }; if(n==b.size) null else String(b,0,n,Charsets.UTF_8) }}}catch(_:Exception){null}}
class CompanionClient(private val transport:SocketTransport,private val protocol:CompanionProtocol){fun request(kind:String,payload:String,sessionId:String?=null):ClientResult<Frame>{return try{val response=transport.request(protocol.frame(kind,payload,sessionId))?:return ClientResult.Offline();ClientResult.Online(protocol.accept(response))}catch(e:ProtocolError){throw e}catch(_:Exception){ClientResult.Offline()}};fun pair(payload:String,sessionId:String?=null)=request("pair",payload,sessionId);fun session(payload:String,sessionId:String?=null)=request("session",payload,sessionId);fun reconnect(payload:String,sessionId:String?=null)=request("reconnect",payload,sessionId);fun history(payload:String,sessionId:String?=null)=request("history",payload,sessionId);fun queuePreview(payload:String,sessionId:String?=null)=request("queue_preview",payload,sessionId);fun queueApprove(payload:String,sessionId:String?=null)=request("queue_approve",payload,sessionId);fun queueCancel(payload:String,sessionId:String?=null)=request("queue_cancel",payload,sessionId);fun queueRetry(payload:String,sessionId:String?=null)=request("queue_retry",payload,sessionId);fun revoke(){protocol.revoke()};fun rotate(key:ByteArray){protocol.rotate(key)}}
class OutgoingQueue{private val items=ArrayDeque<String>();fun preview(text:String){require(text.toByteArray().size in 1..MAX_TEXT&&items.size<MAX_QUEUE);items.addLast(text)};fun approve()=items.removeFirstOrNull();fun cancel()=items.removeFirstOrNull();fun retry(text:String)=preview(text);fun size()=items.size}
class ReadOnlyHistory(private val limit:Int=32){private val items=ArrayDeque<String>();fun add(text:String){if(text.toByteArray().size<=MAX_TEXT){if(items.size==limit)items.removeFirst();items.addLast(text)}};fun snapshot()=items.toList()}
