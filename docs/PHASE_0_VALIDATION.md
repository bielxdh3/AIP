# Phase 0 Validation

This document separates repository validation from interactive Windows evidence.

## Automated scope

The repository defines checks for:

- public secret and private-path patterns;
- ESLint and TypeScript;
- shared contract and transition tests;
- frontend production build;
- Python formatting, linting, strict typing, tests, and deterministic health output;
- Rust formatting, checking, Clippy, SQLite tests, protocol tests, and state tests.

CI runs the complete non-interactive set on a Windows runner with Node 22, Python 3.11,
pnpm, stable Rust, rustfmt, and Clippy. It does not publish installers or use secrets.

## Interactive scope

Automated checks do not prove:

- transparent WebView behavior on every Windows configuration;
- click-through geometry under all scaling values;
- always-on-top interaction with every application;
- reliable full-screen detection;
- multi-monitor positioning;
- Windows 10 packaging or installer behavior.

Use the manual checklist in `docs/WINDOWS_SETUP.md` before approving Phase 0 for the next
phase. Do not replace missing manual evidence with mocked results.

## Phase boundary

The current runtime supports health and shutdown only. No model request, conversation,
message persistence, memory, tool execution, network listener, Android code, or BielOS
integration is included.
