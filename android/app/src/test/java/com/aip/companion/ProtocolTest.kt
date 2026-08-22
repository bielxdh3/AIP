package com.aip.companion
import kotlin.test.*
class ProtocolTest {
 private class FakeSocket(private val server:CompanionProtocol):SocketTransport{override fun request(frame:String):String?{val request=server.accept(frame);return server.frame(request.kind,request.payload,request.sessionId,"response-${request.counter}")}}
 @Test fun jsonRoundTripAndMac(){val p=CompanionProtocol(ByteArray(32){it.toByte()});val raw=p.frame("hello","{\"ok\":true}","s1","abc");assertEquals("hello",FrameCodec.decode(raw).kind);assertEquals(64,FrameCodec.decode(raw).mac.length)}
 @Test fun authVersionBounds(){val p=CompanionProtocol(ByteArray(32));val raw=p.frame("x");val receiver=CompanionProtocol(ByteArray(32));val tampered=raw.replace("\"kind\":\"x\"","\"kind\":\"tampered\"");assertFailsWith<ProtocolError.Auth>{receiver.accept(tampered)};assertFails{p.frame("x","x".repeat(MAX_PAYLOAD+1))};assertFails{receiver.accept(raw.replace(PROTOCOL,"bad"))}}
 @Test fun escapingRoundTrip(){val p=CompanionProtocol(ByteArray(32));val raw=p.frame("escape", "quote=\" slash=\\ newline=\n tab=\t");assertEquals("quote=\" slash=\\ newline=\n tab=\t",FrameCodec.decode(raw).payload)}
 @Test fun replayNonceChallengeRotationRevocation(){val p=CompanionProtocol(ByteArray(32));val raw=p.frame("x","","s","same");val receiver=CompanionProtocol(ByteArray(32));receiver.accept(raw);assertFailsWith<ProtocolError.Replay>{receiver.accept(raw)};assertEquals(64,p.challenge("c").length);p.rotate(ByteArray(32){1});p.revoke();assertFailsWith<ProtocolError.Revoked>{p.frame("x")}}
 @Test fun offlineTimeout(){assertNull(CompanionSocketTransport("127.0.0.1",1,20).request("x\n"))}
 @Test fun queue(){val q=OutgoingQueue();repeat(MAX_QUEUE){q.preview("x")};assertFails{q.preview("x")}}
 @Test fun signedClientServerRoundTrips(){val key=ByteArray(32){7};val client=CompanionClient(FakeSocket(CompanionProtocol(key)),CompanionProtocol(key));assertIs<ClientResult.Online<*>>(client.pair("pair","session-1"));assertIs<ClientResult.Online<*>>(client.session("session"));assertIs<ClientResult.Online<*>>(client.reconnect("reconnect"));assertIs<ClientResult.Online<*>>(client.history("history"));assertIs<ClientResult.Online<*>>(client.queuePreview("queue"))}
}
