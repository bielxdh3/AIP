# Owner-approved feature backlog — 2026-09-03

This document records product decisions approved by the repository owner on 2026-09-03.

It is a planning record only. An item appearing here does **not** mean it is implemented, validated, released, or authorized to bypass the repository's normal architecture/review/validation gates.

## Approved features

- [#24 — Memory Inspector / Brain Map](https://github.com/bielxdh3/AIP/issues/24)
- [#25 — Context Compiler Debugger](https://github.com/bielxdh3/AIP/issues/25)
- [#26 — Multi-model cognitive routing](https://github.com/bielxdh3/AIP/issues/26)
- [#27 — Integrated offline voice runtime](https://github.com/bielxdh3/AIP/issues/27)
- [#28 — Agent Timeline](https://github.com/bielxdh3/AIP/issues/28)
- [#29 — Replay / branch agent personality and state](https://github.com/bielxdh3/AIP/issues/29)

## Architectural ordering

1. Memory Inspector and Context Compiler Debugger should build on the canonical Context & Memory architecture rather than invent a second state model.
2. Multi-model cognitive routing must preserve Rust/SQLite authority and one canonical agent identity/state.
3. Agent Timeline should consume authoritative events/provenance and integrate naturally with Memory Inspector.
4. Replay/branch must remain isolated from canonical state by default and be regression-tested against cross-branch writes.
5. Integrated offline voice must reuse the normal conversation, memory, permission, and lifecycle boundaries.

## Status rule

The GitHub issues above are the authoritative trackers for scope and acceptance criteria. Existing roadmap phase status remains unchanged until implementation and required validation actually occur.
