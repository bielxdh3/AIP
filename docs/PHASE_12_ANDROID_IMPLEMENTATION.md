# Phase 12 Android client slice

This slice is a real Kotlin Android client (`com.aip.companion`) with native
Views, bounded Portuguese offline queue/history UI, explicit overlay permission
flow, privacy-safe status channel, Android Keystore AES-GCM credential custody,
and a bounded `aip-companion-v1` protocol client with HMAC, counters, replay,
version, and revocation checks.

Desktop transport integration is intentionally deferred to the next scoped
commit. This client never claims desktop delivery and remains usable offline.
