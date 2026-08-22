package com.aip.companion

import kotlin.test.*

class ProtocolTest {
    @Test fun codecHandshakeAndReplay() { val key = ByteArray(32) { it.toByte() }; val a = CompanionProtocol(key); val b = CompanionProtocol(key); val f = a.frame("hello", "ok"); assertEquals("hello", b.accept(f).kind); assertFailsWith<ProtocolError.Replay> { b.accept(f) } }
    @Test fun boundsAndQueue() { val q = OutgoingQueue(); repeat(MAX_QUEUE) { q.preview("x") }; assertFails { q.preview("x") }; assertFails { q.preview("x".repeat(MAX_TEXT + 1)) } }
    @Test fun revocationAndHistory() { val p = CompanionProtocol(ByteArray(32)); p.revoke(); assertFailsWith<ProtocolError.Revoked> { p.frame("x") }; val h = ReadOnlyHistory(2); h.add("a"); h.add("b"); h.add("c"); assertEquals(listOf("b", "c"), h.snapshot()) }
}
