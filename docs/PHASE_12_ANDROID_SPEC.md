# Phase 12 local Android companion specification

Status: PRODUCTIZED FOR DEBUG COMPANION — HUMAN DEVICE VALIDATION PENDING. The real Android APK has an
explicit authenticated local/private transport client and deterministic JVM
loopback coverage. Physical-device, private-LAN, manual, and release-signing
validation remain reserved.

## Purpose and authority

Phase 12 defines the contract between the standalone AIP desktop and the
Android companion without importing BielOS. The debug APK uses an explicit,
authenticated local/private socket client; deterministic JVM loopback tests
cover the host-side transport boundary. Rust/SQLite is authoritative for device ownership, pairing, sessions, replay counters,
queue approval, key rotation, revocation, audit, safe mode, and temporary-chat
gates. React only renders Portuguese Owner controls and cannot grant authority.

The versioned protocol is `aip-companion-v1`: exactly one JSON object per line,
bounded to 16 KiB, with fixed fields `protocol`, `kind`, `clientId`, nullable
`sessionId`, `nonce`, `counter`, `payload`, and `mac`. Canonical MAC bytes are
the UTF-8 values in that order excluding `mac`, joined by U+001F; `mac` is
lowercase HMAC-SHA-256 hex. Unknown/missing fields, invalid UTF-8, bad lengths,
version mismatch, repeated nonce, and non-monotonic counters fail closed.
No relay, public listener, tunnel, shell, Python runtime, host filesystem, or
external account exists; offline fallback remains truthful until a valid response.
Android never auto-connects, scans, or opens a listener; Keystore credentials are
not logged or persisted in plaintext.

## Pairing and authenticated session model

The Owner starts pairing for the bounded `android-fixture-01` device and must
confirm the exact fingerprint and pairing nonce metadata. Only metadata is
stored: fingerprints and nonce references are not private keys or credentials.
Pairing expires after a bounded interval and is idempotent per Owner,
operation, and request key. A device is limited to the local Owner's agent and
cannot self-approve.

After pairing, a session negotiates protocol version 1 and records an
authenticated proof containing device/session nonce metadata, key fingerprint,
application version, and a strictly increasing replay counter. Nonce metadata
is unique per device; stale or repeated counters fail closed. Reconnect performs
the same compatibility and ownership checks. Rotation advances the key version
and records old/new fingerprints and a reason. Revocation closes sessions and
queue entries and remains visible in the revocation and audit records.

## Offline history and outgoing queue

History is read-only and bounded. The outgoing queue accepts only bounded
metadata payloads for text, audio, image, file, and task items. Audio/image/file
entries contain type, dimensions/duration, name, and byte-length metadata only;
media bytes are rejected and the database invariants keep
`media_bytes_persisted = false`.

Every item is previewed before it can be queued and requires explicit Owner
approval. The Owner may cancel or retry within a bounded retry count. Each
operation is idempotent and writes a concise audit record. Temporary chat and
global/agent safe mode fail closed for all mutations; read-only history and
audit remain available for recovery visibility.

## Desktop surface and validation boundary

`CompanionControls` exposes pairing/confirmation, session connection, queue
preview/approval/cancel/retry, key rotation, revocation, history, and audit
status in Portuguese. It calls the Tauri commands and validates every response
through the versioned contracts parser.

Focused coverage exercises bounded payloads, parser rejection of forbidden
fields, synthetic fixture labels, and fail-closed UI copy. Rust migration and
module tests cover pairing expiry, replay/compatibility checks, queue approval
and cancellation, revocation, key rotation, idempotency, and safety gates.

## Explicit non-goals and follow-up

This checkpoint builds a real debug APK and exercises the authenticated local/
private socket client through deterministic JVM loopback tests; it does not
claim physical-device or private-LAN delivery. Microphone/camera capture, media
persistence, Android accounts, relay/tunnel access, and end-to-end mobile
delivery remain out of scope. Android lifecycle/permission and overlay UX,
notification quality, packaged-device testing, recovery, and release signing
remain reserved human/release gates. Phase 13 BielOS/Cloudflare gateway
integration is intentionally out of scope.
