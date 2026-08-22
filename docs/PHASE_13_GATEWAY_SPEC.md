# Phase 13 local gateway specification

Status: FUNCTIONAL — HUMAN VALIDATION PENDING. The
standalone AIP desktop implements and tests a bounded `aip-gateway-v1` framed
HMAC TCP transport and Rust/SQLite authority. Loopback is the validated path;
this is not BielOS integration, a stable release, or a remote recovery claim.

## Purpose and authority

Phase 13 defines the bounded gateway contract for the standalone AIP desktop.
Rust/SQLite is authoritative for the versioned protocol metadata, Owner
ownership, account metadata, transfer preview and approval, session
authentication, replay counters, recovery approval, revocation, audit, safe
mode, and temporary-chat gates. React exposes Portuguese controls and invokes
Tauri commands; it cannot transfer, authenticate, recover, revoke, or approve
independently.

The current fixture is the local Owner's `agt_luma_provisional` agent and a
synthetic administrative client. Contracts use protocol version 1, bounded
newline-delimited frames, lower-case HMAC-SHA256, replay/idempotency checks,
metadata-only account and transfer records, and an explicit standalone fallback.

## Local state and lifecycle

The desktop surface reads protocol, account, transfer, session, recovery,
audit, and revocation records through the existing gateway commands. Mutating
actions are delegated to Rust:

- transfer preparation and Owner approval;
- local session connection and authenticated reconnect;
- administrative recovery request and Owner approval;
- session and transfer revocation.

Every mutation carries the local Owner identity, a bounded fixture reference,
an idempotency key, and the temporary-chat flag. Rust validates ownership,
compatibility, integrity, replay freshness, state transitions, safe mode, and
temporary chat before changing SQLite state. Read-only state and audit remain
visible when mutations are blocked.

## Cloudflare and external boundaries

Cloudflare Tunnel/Access values are configuration metadata only: the fixture
reports a metadata-only mode and absent credentials. The gateway binds only
localhost by default; private-LAN binding requires explicit confirmation. It
does not create a tunnel, relay traffic, contact BielOS, access a remote
account, or perform recovery outside the local database.

The checkpoint imports neither BielOS runtime code nor Python internals. It
does not read credentials, `.env` files, the host filesystem, or the shell,
and no external effect is represented as performed. The transfer and recovery
records are synthetic metadata for validating ownership and approval paths.

## Desktop validation boundary

`GatewayControls` renders Portuguese local/private listener status and exposes
explicit Owner-confirmed start/stop controls. The one-time pairing code stays
in transient component state. Vitest coverage checks status loading, exact
start arguments, transient pairing display, stop, and blocked-mode behavior;
Rust loopback coverage drives signed TCP frames through SQLite authority for
the complete transfer/session/recovery/revocation lifecycle.

Private-LAN smoke testing, hardware/manual permission and recovery UX, packaged
device tests, release signing, remote CI, Cloudflare credentials, BielOS
ownership exchange, external accounts, public relay/tunnel behavior, and
stable-release approval remain pending.
