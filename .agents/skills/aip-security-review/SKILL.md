---
name: aip-security-review
description: Use for AIP security, permissions, local process isolation, secrets, temporary chat, audit, tool execution, data protection, exports, or remote-access preparation.
---

# AIP Security Review Skill

Use this skill for security-sensitive AIP work.

Keep output minimal and precise. Do not narrate routine inspection. Do not print secret or private values.

## Security Model

AIP is local-first. Preserve these boundaries unless the user explicitly changes them:

1. The desktop UI remains usable without the AI runtime.
2. Rust owns authoritative persistence and system integration.
3. The Python runtime is a managed, replaceable child process.
4. Initial Rust-to-Python communication uses managed stdio rather than an exposed network service.
5. Ollama defaults to loopback access.
6. Agent identity and memory remain independent from model files.
7. Temporary chat content remains in memory only.
8. State-changing and destructive actions require explicit permission according to policy.
9. Safe mode disables models, extensions, autonomous work, and agent overlays.
10. Public repository contents must not include private local data or BielOS operational secrets.

## Sensitive Data Rules

Never expose or commit:

- `.env` values;
- API keys or access tokens;
- model provider credentials;
- private keys;
- session or approval tokens;
- local chat histories;
- memory databases;
- agent exports;
- backups;
- local databases;
- model files;
- voice samples;
- screenshots or media with personal data;
- raw private paths when a placeholder is sufficient;
- private BielOS credentials or infrastructure details.

## Review Checklist

Check for:

- non-loopback local servers;
- unauthenticated local control endpoints;
- shell command construction from untrusted text;
- path traversal;
- unsafe file overwrite or deletion;
- temporary-chat writes to disk, logs, telemetry, crash reports, or memory stores;
- Python runtime access to authoritative storage without a defined contract;
- model output treated as trusted commands;
- permissions that are broader than the requested action;
- secrets in logs, exceptions, fixtures, examples, or CI;
- committed databases, exports, model files, or local media;
- unsafe deserialization or import handling;
- public docs leaking private BielOS operations;
- safe mode depending on the component it is supposed to disable.

## Temporary Chat Rules

Temporary chat is read-only with respect to external state.

It may:

- converse;
- search or consult read-only tools;
- inspect data that the current owner is allowed to view.

It must not:

- create, edit, move, rename, or delete files;
- modify folders;
- install software or extensions;
- send messages or emails;
- create or edit calendar data;
- execute external state-changing actions;
- persist conversation content, summaries, memories, or learning data.

Minimal action audit may be retained without conversation content according to the configured retention policy.

## Model and Tool Rules

- Treat model output as untrusted structured input.
- Validate tool names, arguments, paths, and permissions before execution.
- Never grant unrestricted shell access by default.
- Preview affected files and expected results before approval.
- Forced execution cannot bypass absolute prohibitions.
- Destructive or external actions require individual approval.

## Output Format

Use only:

- Status
- Security verdict
- Findings
- Required fixes
- Acceptable limitations
- Validations run
- Validations not run

Use severity when useful: Critical, High, Medium, Low, Note.

Do not exaggerate or present speculation as fact.

If no critical issue is found, state:

`No critical issue found in the reviewed scope.`