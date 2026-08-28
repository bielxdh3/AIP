# Phase 9 supervised tools specification

Status: EXTERNAL-PREREQUISITE PRODUCTIZED — HUMAN VALIDATION PENDING.

Phase 9 adds a bounded, Owner-supervised tool boundary to the standalone AIP
desktop application. The checkpoint exercises
the contract, persistence, preview, approval, confirmation, cancellation,
compensation, and audit paths with bounded local workspace effects. Calendar and messaging remain provider-neutral fixture mocks; no shell, credential, network, or external provider is accessed.

## Scope and invariants

Rust and SQLite remain authoritative for manifests, sessions, permissions,
actions, results, and audit records. React renders Portuguese Owner controls;
it cannot grant a permission or bypass a state transition. Temporary chat is
never accepted by a tool mutation command. Safe mode blocks session creation,
preview, approval, confirmation, and execution; read-only catalog and existing
audit/history inspection remain available for recovery visibility.

Every tool call is one of the versioned manifest entries below. There is no
generic shell command, arbitrary code execution, unrestricted filesystem root,
Windows system-directory access, credential dumping, security-control change,
destructive disk operation, hidden execution, network access, or external
provider mutation.

## Manifest v1

| Tool identity | Classification | Adapter boundary | Scope | Owner controls |
| --- | --- | --- | --- | --- |
| `workspace.inspect_scope` | read-only | workspace mock | `fixture:workspace/*` | preview and read-only execution |
| `workspace.organize_files` | state-changing | workspace mock | `fixture:workspace/*` | preview, Owner approval, simulated execution, compensation record |
| `workspace.inspect_local` | read-only | bounded local filesystem | `workspace_root:<opaque-id>` | metadata-only preview and read-only execution |
| `workspace.organize_local` | state-changing | bounded local filesystem | `workspace_root:<opaque-id>` | preview, Owner approval, second confirmation, dry-run, move and safe rollback |
| `calendar.list_events` | read-only | calendar mock | `fixture:calendar/*` | preview and read-only execution |
| `calendar.create_event` | state-changing | calendar mock | `fixture:calendar/*` | preview, Owner approval, second confirmation, simulated execution, compensation record |
| `messaging.preview_message` | read-only | messaging mock | `fixture:messaging/*` | preview and read-only execution |
| `messaging.send_message` | state-changing | messaging mock | `fixture:messaging/*` | preview, Owner approval, second confirmation, simulated execution, compensation record |

The manifest is stored in SQLite with `manifest_version = 1`. Tool identity,
classification, adapter kind, scope kind, capability list, and confirmation
policy are read from that record before an action is accepted.

## Session and permission flow

The Owner creates a session with one fixture scope and a set of exact
tool/permission pairs. A session cannot mix workspace, calendar, or messaging
scopes. The supported permission values are:

- `preview`;
- `execute_read_only` for a read-only manifest;
- `execute_state_changing` for a state-changing manifest.

Rust validates the Owner/agent relationship, manifest version, scope prefix,
permission/classification pair, duplicate permissions, session size, and
idempotency key. A cancelled session cannot preview or execute new actions.

The action path is:

1. validate and normalize the input against the manifest and session scope;
2. persist an exact preview with summary, affected fixture resources, and exact
   effect;
3. require explicit Owner approval for every state-changing action;
4. require a second explicit confirmation when the manifest requests it;
5. execute only the deterministic mock, with an explicit button/command;
6. persist bounded, untrusted output and compensation metadata;
7. allow cancellation before completion and record every transition in audit.

Read-only actions still require a session permission and an explicit execution
command. Dry-run is persisted on the exact action and must match at execution.
Approval is bound to the persisted action id and validated input, so altered
arguments cannot reuse approval. The current checkpoint has no provider refusal
channel; therefore the second-confirmation step is the explicit forced-action
acknowledgement for manifests that permit it, and it never bypasses an
absolute prohibition.

## Input and output boundary

Inputs are tagged, versioned contract values. Local paths are relative to an
opaque configured workspace root; fixture paths remain relative fixture
references. Both reject traversal, absolute paths, backslashes, control
characters, and host-root syntax. Calendar dates/times, recipients, message
bodies, scope references, and identifiers have bounded validators.

Output is capped before SQLite persistence and is marked `untrusted: true`.
Fixture results report `changed: false`; local execution reports `changed: true`
only when a bounded host move occurred. Roots reject broad/system roots,
canonical containment escapes, links and reparse points. Local inspection is
metadata-only; moves revalidate immediately before rename and never delete or
overwrite.
The UI renders output as text and never interprets it as markup, commands, or
instructions.

## Persistence and audit

Migration `0016_phase9_tools.sql` is the original fixture-tool schema for the catalog, sessions, session
permissions, actions, and audit tables. Migration `0023_phase9_workspace_roots.sql` rebuilds and preserves that schema, including existing fixture rows and foreign keys, while adding `workspace_roots` and the two local manifests. Foreign keys preserve agent and Owner
isolation. Idempotency keys are unique within the Owner/session boundary.

Audit records contain bounded event metadata and a Portuguese summary, never
credentials, temporary-chat content, raw provider payloads, or unbounded tool
output. Records older than 30 days are removed when a new tool event is
written; the current list command returns at most 100 records per agent.

Cancellation and compensation are both persisted as action state and audit
events. Local compensation is Owner/agent/action scoped and reverses only
still-matching bounded paths; fixture compensation remains metadata-only.

## UI and validation

The Owner-facing controls live in the agent tools area of `App.tsx` and expose
the catalog, opaque workspace roots, fixture/local scopes, granular permissions, session selection, bounded
input forms, preview, approval/recusal, second confirmation, explicit mock
execution, cancellation, compensation, and recent audit records. Temporary
chat and safe mode disable all tool mutation controls and display the blocking
reason. No delete, overwrite, shell, network, credential, telemetry, watcher,
or provider mutation is included. Packaged-Windows behavior and human
confirmation UX remain deferred; this is not stable release approval.

Focused contract tests reject malformed manifests, sessions, action inputs,
unsafe result flags, and unknown error codes. Rust tests cover migration-backed
catalog loading, Owner approval and second confirmation, deterministic output,
compensation/audit records, temporary-chat rejection, and safe-mode rejection.
Live provider behavior and packaged-Windows runtime tests remain deferred when
the environment cannot spawn the required processes; this checkpoint does not
claim those validations.
