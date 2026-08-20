# Phase 11 on-demand screen vision specification

Status: implemented local metadata-only checkpoint; automated validation is
phase-scoped, human visual/privacy validation remains pending, and this is not
release approval.

Phase 11 adds a bounded Screen Vision control path to the standalone AIP
desktop application. It models an Owner-requested visual analysis using
synthetic monitor fixtures and a synthetic visual-model fixture. The checkpoint
does not capture the Windows desktop and does not process real pixels.

## Scope and invariants

Rust and SQLite remain authoritative for fixture selection, Owner identity,
session permissions, privacy policy, quotas, job lifecycle, resource
scheduling, cleanup, cancellation, and audit records. React renders Portuguese
Owner controls and cannot grant permission, confirm a job, bypass safe mode, or
make a visual result durable.

The supported path is:

1. The Owner selects one of the bounded synthetic monitor fixtures.
2. Rust creates an explicit session for one agent with both fixture-capture and
   fixture-analysis permissions, a privacy policy, and per-session quotas.
3. The Owner requests a preview. The preview contains monitor dimensions and
   redaction metadata only and requires confirmation.
4. The Owner explicitly confirms the preview. Rust reserves the single
   `reference-gpu` resource, marks the model fixture as running, produces a
   bounded uncertain hypothesis, and immediately unloads/releases all
   transient model/frame metadata.
5. Rust records the lifecycle in the bounded audit log. The hypothesis is
   diagnostic-free, never a sensitive-attribute inference, and never durable
   visual state.

The operation uses bounded idempotency keys. A replay returns the original
record; a conflicting request fails closed. Listing sessions, jobs, and audit
records is read-only and remains available for recovery visibility.

## Synthetic fixtures and privacy

The only fixtures are metadata records for `monitor-1` and `monitor-2`, each
marked `synthetic: true` and `metadataOnly: true`. Their dimensions, scale,
fixture reference, and display name are not images. The only model reference is
`fixture:visual-model/screen-neutral-v1`.

Every session must enable `excludeSensitiveContent` and include an enabled
`exclude_sensitive_regions` redaction hook. The policy is stored with the
session and copied to each job preview. The redaction hook is a safety contract
for a future real adapter; it does not authorize a real capture in this
checkpoint.

No sensitive attribute is inferred. Results are explicitly uncertain,
non-diagnostic, non-durable, and bounded to a short text hypothesis. Rust
returns `screenshotBytesPersisted: false` and never accepts image, pixel, or
capture-path fields in the versioned contract parser.

## Lifecycle, scheduling, and cleanup

The database migration creates separate session, permission, job, audit, and
idempotency tables. A job can be previewed, queued, running, completed,
cancelled, failed, or cleaned. The local implementation completes the
synthetic run inside the authoritative Rust transaction so the checkpoint has
no worker, polling loop, or background capture path.

The resource schedule is deliberately conservative:

- at most four active sessions per agent;
- at most eight jobs per session;
- a requested job duration between 100 ms and 15 seconds;
- one active `reference-gpu` job across the database;
- preview metadata expires after ten minutes and is automatically cleaned on
  the next Screen Vision operation;
- audit reads return at most 100 records and new audit writes retain at most
  30 days of records;
- analysis text is bounded to 1,024 bytes by the Rust result guard.

Cancellation can clean a preview, queued/running fixture job, or failed job.
Session cancellation closes the session and cleans its remaining transient
job metadata. Cleanup releases the model/resource lifecycle and clears
`frame_metadata_json`; `result_durable` remains false.

## Authority and safety gates

Every mutating request includes the agent ID, the explicit local Owner user ID,
an operation-scoped idempotency key, and the temporary-chat flag. Rust checks
the Owner role and agent ownership against SQLite. Confirmation also requires
`confirmed: true`; a preview alone never runs the model fixture.

Rust rejects Screen Vision mutations when:

- the request is from temporary chat;
- application safe mode is enabled;
- the agent is suspended or in safe mode;
- the Owner identity, fixture, permission set, privacy policy, quota, or
  lifecycle state is invalid;
- the reference GPU is already reserved by another active job.

The Tauri commands repeat the temporary-chat gate at the desktop boundary, but
the database gate remains authoritative. React disables the same controls for
usability only. Read-only history and audit inspection may remain visible in
safe mode or temporary chat, while no durable Screen Vision mutation is
allowed.

## Explicit non-goals

This phase must not add or imply:

- a Windows screenshot API, desktop/window capture, webcam input, or real
  pixels;
- screenshot bytes, image files, thumbnails, or visual embeddings in SQLite,
  logs, backups, exports, or chat history;
- continuous analysis, polling capture, surveillance, or background capture;
- a network service, remote model, cloud provider, remote access, or model
  download;
- host filesystem, shell, process, credential, browser, or account access;
- sensitive-attribute, biometric, identity, health, or other diagnostic
  inference;
- durable visual memory, autonomous follow-up, or an unbounded result;
- Android, BielOS, gateway, or Phase 12+ work.

## Validation boundary

Focused Rust tests cover migration-backed synthetic preview, explicit
confirmation, cleanup, Owner/privacy/mode gates, fixture selection,
idempotency, resource contention, and cancellation. Contract tests cover
versioned metadata records and reject pixels, unsafe privacy, durable visual
state, and certain/diagnostic hypotheses. Desktop tests cover Portuguese
controls, authoritative command wiring, and temporary-chat/safe-mode fail
closed behavior.

Real screen privacy behavior, Windows packaging, visual usability, and any
future real adapter remain human/release validation work. Their absence is not
represented as an implemented capture capability.
