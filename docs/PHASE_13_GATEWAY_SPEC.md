# Phase 13 local gateway specification

Status: implemented as a standalone, local metadata-only architecture
checkpoint. This is not BielOS integration, a network release, or a remote
recovery claim.

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
references, idempotency keys, metadata-only account and transfer records, and
an explicit standalone fallback.

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
reports a metadata-only mode, an absent credential state, and no network
listener. Phase 13 does not open a socket, bind a listener, create a tunnel,
relay traffic, contact BielOS, access a remote account, or perform a recovery
outside the local database.

The checkpoint imports neither BielOS runtime code nor Python internals. It
does not read credentials, `.env` files, the host filesystem, or the shell,
and no external effect is represented as performed. The transfer and recovery
records are synthetic metadata for validating ownership and approval paths.

## Desktop validation boundary

`GatewayControls` renders Portuguese local-only status and delegates actions
through the existing Tauri gateway command names. Focused Vitest coverage
checks local read commands, transfer-preparation delegation, metadata-only
copy, and fail-closed controls for temporary chat and safe mode. Rust module
coverage remains responsible for lifecycle, replay, ownership, idempotency,
revocation, and safety invariants.

This checkpoint does not implement a public gateway, BielOS account exchange,
Cloudflare credentials, transport cryptography, remote/mobile delivery, or
end-to-end recovery. Those require a separately authorized phase and security
and release review.
