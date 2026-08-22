# Phase 12 Android client slice

This slice is a real Kotlin Android client (`com.aip.companion`) with native
Views, bounded Portuguese offline queue/history UI, explicit overlay permission
flow, privacy-safe status channel, Android Keystore AES-GCM credential custody,
and a bounded `aip-companion-v1` protocol client with HMAC, counters, replay,
version, and revocation checks.

The APK has an explicit-connect-only typed client for the authenticated desktop
transport. Physical-device, private-LAN, and release-signing checks remain
reserved. The shared wire contract is one newline-delimited JSON object per line:
`protocol`, `kind`, `clientId`, nullable `sessionId`, `nonce`, non-negative
`counter`, bounded UTF-8 `payload` string, and lowercase 64-character `mac`.
The MAC input is the UTF-8 concatenation of those fields excluding `mac`, in
that order, joined by U+001F. This client remains offline until a valid
authenticated response and never scans, auto-connects, or listens.
