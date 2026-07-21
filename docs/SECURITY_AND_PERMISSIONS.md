# AIP Security and Permission Model

## 1. Scope

This document defines AIP security and permission rules from the standalone desktop implementation through later supervised tools and remote integration.

AIP is local-first, but local software still handles private conversations, memories, model files, media, and future actions on the operating system. Local does not mean consequence-free, apparently a fact software enjoys teaching through data loss.

## 2. Trust boundaries

### Trusted deterministic core

Rust application core owns:

- authoritative persistence;
- process lifecycle;
- mode enforcement;
- safe mode;
- permission decisions;
- approval state;
- validated filesystem paths;
- audit records;
- model and runtime configuration.

### Untrusted or limited-trust components

Treat as untrusted input:

- user-provided files;
- imported pixel-art documents;
- model output;
- Python runtime output;
- extension output;
- external tool results;
- remote mobile requests;
- imported agent packages;
- web content.

React is not a security boundary. UI controls must be backed by Rust validation.

## 3. Local process security

Initial Python runtime rules:

- launched and stopped by Rust;
- communicate through managed stdio;
- no exposed TCP listener in the initial architecture;
- structured versioned messages only;
- bounded request and response sizes;
- sanitized stderr;
- runtime crash must not crash the main application;
- no unrestricted shell or arbitrary-code RPC method;
- no direct authoritative database ownership.

Ollama defaults to loopback.

A custom endpoint is advanced configuration and must clearly warn when it is not loopback or trusted local network infrastructure.

## 4. Data classification

### Public repository data

May include:

- source code;
- documentation;
- original placeholder assets;
- example configuration without credentials;
- synthetic test fixtures;
- schemas and migrations.

### Private local data

Must never be committed:

- conversations;
- memories;
- user profiles;
- agent exports;
- databases;
- model files;
- voice data;
- images and attachments;
- backups;
- logs containing personal content;
- credentials;
- tokens;
- private BielOS data.

### Sensitive operational data

Must not appear in normal logs:

- assembled prompts containing private memory;
- raw model credentials;
- approval tokens;
- remote session tokens;
- private filesystem paths when avoidable;
- contents of imported packages;
- raw exception payloads containing conversation content.

## 5. Logging

Logs are diagnostic, not a second secret history database.

Rules:

- structured event codes;
- no raw message content by default;
- no raw memory content by default;
- no secrets;
- no model prompt dump by default;
- bounded retention;
- clear opt-in before any future verbose debugging mode;
- public bug reports must use sanitized diagnostics.

## 6. Temporary chat

Temporary chat is memory-only and externally read-only.

Allowed:

- conversation;
- read-only search;
- read-only consultation of data available to the current owner;
- model generation;
- temporary in-memory context.

Forbidden:

- persistent history;
- persistent summary;
- persistent memory;
- learning or personality update;
- file or folder creation;
- file or folder editing;
- moving or renaming;
- deletion;
- extension installation;
- sending messages or emails;
- calendar changes;
- state-changing external tools.

The implementation must test that no temporary message is written to SQLite or local files.

A minimal audit may record that a read-only tool was used, without conversation content, and later follows the 30-day retention policy.

## 7. Modes and enforcement

### Normal mode

Normal user-initiated behavior is available.

### Voice-muted mode

No audible output when voice exists. Text, visual behavior, and allowed background behavior remain.

### Silent mode

- no spontaneous agent action;
- no autonomous task;
- no autonomous agent-to-agent conversation;
- direct text call receives text response;
- later direct voice call receives voice response.

Silent mode enforcement belongs to Rust policy, not only UI state.

### Safe mode

- model runtime not used;
- extensions not used;
- autonomous work disabled;
- overlays hidden;
- administration, history, settings, diagnostics, and recovery remain;
- later remote administrative recovery may stay available while agent network access is blocked.

Safe mode must remain available even when Python, Ollama, extensions, or agent data fail to load.

### Suspension

Suspension pauses:

- activities;
- goals;
- fictional state progression.

Age continues. Suspended agent appears offline.

## 8. Permission model

Later tool permissions are granular and session-scoped.

Permission dimensions include:

- tool;
- action;
- resource class;
- exact path, account, application, or calendar;
- read or write;
- session duration;
- agent;
- owner;
- risk level.

Examples:

- read Downloads directory;
- move one named file;
- create one calendar event;
- send one reviewed message;
- open one application;
- use microphone for the current session.

Do not grant broad access when a narrow grant satisfies the request.

## 9. Approval model

Before a state-changing action, show:

- requesting agent;
- owner;
- exact action;
- reason;
- tool;
- affected resources;
- current state;
- expected result;
- preview or diff when possible;
- rollback availability;
- risk level;
- approve, deny, or modify.

Destructive or external actions require individual approval even when other session permissions exist.

Approval must be bound to the exact validated action. A model cannot reuse approval for altered arguments.

## 10. Forced execution

An agent may refuse because of risk or disagreement.

The owner of that agent may force eligible actions after:

1. seeing the refusal and reason;
2. receiving a second explicit warning;
3. confirming the exact ignored objection and action.

The agent records that it continues to disagree.

Repeated forced actions may later affect fictional opinion and relationship state.

Forced execution cannot bypass absolute prohibitions.

## 11. Absolute prohibitions

Examples include:

- deliberate destruction of the operating system;
- exposure or transmission of credentials without legitimate protected handling;
- illegal actions;
- bypassing AIP permission enforcement;
- silently weakening security controls;
- executing unreviewed arbitrary model-generated shell commands;
- using temporary chat for state-changing actions;
- pretending an action succeeded when it was not verified.

## 12. Agent differences

Owner-oriented agent:

- may later request broader tools;
- remains approval-bound;
- prioritizes technical tasks;
- may request mobile approval.

Girlfriend-oriented agent:

- prioritizes conversation;
- has more restrictive computer control;
- may use agenda and send small approved items in later phases;
- extension creation receives more careful review;
- cannot freely control the Owner's desktop.

## 13. Extension security

Later extension requirements:

- manifest with version and author;
- granular permission declaration;
- signed or hashed package integrity when practical;
- sandbox for newly created or external extensions;
- explicit source and behavior review;
- tests and compatibility information;
- install disabled by default until approved;
- rollback;
- automatic update only when no new permission is requested;
- new permissions block update activation until approval;
- one agent's approval does not automatically grant another agent access;
- external catalog entries are selected by the administrator.

Extensions included in a legitimate AIP export package may skip full re-sandboxing according to the final product decision, but integrity and package-origin checks remain required.

## 14. Memory security

- memory belongs to one agent and owner;
- cross-agent retrieval is denied unless the conversation explicitly supports shared context;
- private chat memory is visible only to the agent owner;
- agent-to-agent conversations are public to both owners;
- internet content is provisional until confirmed;
- model inference is not stored as verified fact;
- corrections do not silently rewrite original history;
- memory deletion and archive actions are owner-visible.

## 15. Public repository security

Every commit must be safe for public visibility.

Required protections:

- strong `.gitignore`;
- no local databases;
- no model binaries;
- no agent exports;
- no backup archives;
- no real `.env`;
- no personal media;
- no private BielOS operational details;
- secret scanning before release;
- release artifact inspection.

## 16. Export and import security

Later full export includes highly private data and physically included models.

Export requires:

- explicit owner confirmation;
- clear warning that the package contains all private information;
- deterministic manifest;
- integrity metadata;
- no background sharing;
- no automatic cloud upload.

Import requires:

- package structure validation;
- path traversal protection;
- size limits and free-space checks;
- manifest preview;
- included models, extensions, permissions, and private data preview;
- new derived identity assignment;
- no silent overwrite of an existing agent.

## 17. Remote access preparation

Remote access is not in v0.1.

Later design requirements:

- trusted infrastructure such as Cloudflare Tunnel and Access;
- no router port forwarding;
- separate remote gateway from internal Python runtime;
- authenticated ownership;
- read/write permission separation;
- mobile approvals bound to exact action;
- offline queue confirmation after reconnect;
- ambiguous or expired requests require clarification;
- safe mode may preserve authenticated administrative recovery while blocking agent network activity.

## 18. Audit

Later complete audit records:

- requester;
- approver;
- agent;
- model;
- tool;
- permission;
- affected resources;
- action result;
- rollback information;
- timestamps.

Retention target is 30 days, followed by automatic deletion.

Audit must not contain credentials or temporary-chat conversation content.

## 19. Security validation priorities

High-risk changes include:

- IPC changes;
- local server introduction;
- filesystem tools;
- database migrations;
- temporary-chat persistence paths;
- safe mode;
- permissions and approvals;
- extension execution;
- export/import;
- remote access;
- voice data;
- ownership transfer.

These require tests and explicit limitations in the final implementation report.