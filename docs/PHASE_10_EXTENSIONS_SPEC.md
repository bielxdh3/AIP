# Phase 10 bounded extension runtime specification

Status: FUNCTIONAL — AUTOMATED VALIDATION COMPLETE.

Phase 10 adds a bounded, local extension-management boundary to the standalone
AIP desktop application. It stores and reviews metadata and executes only
closed declarative JSON instruction packages interpreted by Rust.

## Closed package format and execution

An optional package uses format `aip-extension-package/v1`, entrypoint `main`,
at most 32 instructions, and a lowercase SHA-256 `integritySha256` over the
canonical package payload with that field excluded. The only instructions are
`emit_text`, `read_agent_context`, `list_tool_catalog`, and `yield`. Unknown,
duplicate, oversized, malformed, or tampered instructions are rejected. No
filesystem, network, shell, subprocess, dynamic library, credential, or
expression host exists.

Execution binds to the exact Owner-approved active revision and package hash.
Input/output are bounded to 4096/8192 bytes, execution to 32 steps and 5
seconds, and cancellation is checked between instructions and persisted as
termination. Host context contains only validated agent identity and bounded
deterministic tool IDs.

## Scope and invariants

Rust and SQLite remain authoritative for extension identity, manifest
revisions, proposals, permissions, lifecycle transitions, and audit records.
React renders Portuguese Owner controls and cannot grant capabilities, bypass
review, activate a revision, or weaken a gate.

The supported catalog is private and local. Entries are always marked
'untrusted: true', use manifest version 1, SDK version
'aip-extension-sdk/v1', 'metadata_only' sandbox policy, and
'local_fixture_only' admission policy. A local fixture reference is metadata;
it is not opened or resolved by AIP.

## Manifest and source policy

Manifest v1 contains:

- a bounded lowercase extension identifier;
- semantic x.y.z version;
- display name;
- SDK, sandbox, and admission policy;
- a bounded list of declared capabilities;
- an optional fixture:extension/... reference;
- an explicit untrusted marker.

The only source kinds are administrator_selected and agent_created.
Agent-created entries are proposals only: they remain pending until the Owner
reviews them and explicitly activates an approved revision.

The checkpoint has no open-ended package loader, compiler, or plugin host;
the closed declarative Rust VM is the strongly justified sandbox and has an
actual execution test. It has no
network fetch, shell, host-filesystem access, credential access, remote code
execution, public marketplace, hidden execution, or automatic activation.

## Proposal and lifecycle flow

1. A local metadata proposal creates revision 1 and a pending review record.
2. The Owner approves or rejects the proposal and selects the capabilities
   that may be granted.
3. Approval changes the record to approved; activation is a separate explicit
   Owner action.
4. An update creates the next manifest revision and disables the current
   entry until the new revision is reviewed and explicitly activated.
5. Any capability expansion is therefore re-reviewed; it never inherits an
   activation silently.
6. Rollback is an explicit Owner action to a previously approved revision.
7. Disable is an explicit Owner action with a bounded reason.

All transitions are persisted and audited. Each mutation uses an operation-scoped
idempotency key with a bounded normalized request and exact prior result; a
replay returns that result, a conflicting payload fails closed, and a replay
does not append another audit event. Audit details are bounded, retained for at
most 30 days when new events are written, and the list command returns at most
100 records.

## Safety gates and UI

Durable extension mutations reject temporary chat and safe mode in Rust.
The Portuguese UI applies the same fail-closed gate to proposal creation,
review, activation, update, rollback, and disable controls. Catalog,
proposal, and audit inspection remain visible for recovery and review.

Catalog inspection checks the local Owner relationship. Durable mutation
requests carry a distinct `owner_user_id`, which Rust validates against the
Owner user role; `agent_id` remains the proposer/audit agent identity. An
agent-created source must equal its proposing agent, and that agent cannot
review its own proposal. Tauri command arguments are versioned contract
values; the UI never becomes an authority for persistence or capability grants.

Compatibility is computed from the pinned `aip-extension-sdk/v1` contract on
manifest persistence and readback. The `recovery_required` lifecycle value is
retained as the bounded `MetadataOnly` closed declarative sandbox policy for
compatibility; execution remains through the versioned host context with
limits, cancellation, approval, rollback, and failure containment.

## Validation boundary

Focused Rust tests cover migration-backed proposal/review/activation, deterministic
package execution through the closed sandbox and bounded host contract,
agent-proposal review-only behavior, update re-review, rollback, audit,
temporary-chat rejection, safe-mode rejection, and invalid manifests.
Contract tests reject trusted, code-like, out-of-bounds, incompatible-shape,
and unknown extension payloads. Arbitrary plugin/native-code behavior, external
providers, package signing beyond the integrity hash, and release approval remain
reserved work.
