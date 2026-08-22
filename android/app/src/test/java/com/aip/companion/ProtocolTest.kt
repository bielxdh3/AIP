package com.aip.companion
import kotlin.test.*
class ProtocolTest {
 @Test fun jsonRoundTripAndMac(){val p=CompanionProtocol(ByteArray(32){it.toByte()});val raw=p.frame("hello","{\"ok\":true}","s1","abc");assertEquals("hello",FrameCodec.decode(raw).kind);assertEquals(64,FrameCodec.decode(raw).mac.length)}
 @Test fun authVersionBounds(){val p=CompanionProtocol(ByteArray(32));val raw=p.frame("x");val receiver=CompanionProtocol(ByteArray(32));assertFailsWith<ProtocolError.Auth>{receiver.accept(raw.replace("\"kind\":\"x\"","\"kind\":\"y\""))};assertFails{p.frame("x","x".repeat(MAX_PAYLOAD+1))};assertFailsWith<ProtocolError.Version>{receiver.accept(raw.replace(PROTOCOL,"bad"))}}
 @Test fun replayNonceChallengeRotationRevocation(){val p=CompanionProtocol(ByteArray(32));val raw=p.frame("x","","s","same");assertFailsWith<ProtocolError.Replay>{p.accept(raw)};assertEquals(64,p.challenge("c").length);p.rotate(ByteArray(32){1});p.revoke();assertFailsWith<ProtocolError.Revoked>{p.frame("x")}}
 @Test fun offlineTimeout(){assertNull(CompanionSocketTransport("127.0.0.1",1,20).request("x\n"))}
 @Test fun queue(){val q=OutgoingQueue();repeat(MAX_QUEUE){q.preview("x")};assertFails{q.preview("x")}}
}
