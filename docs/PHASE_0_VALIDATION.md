# Phase 0 Validation

This document separates repository validation from interactive Windows evidence.

## Automated scope

The repository defines checks for:

- public secret and private-path patterns;
- ESLint and TypeScript;
- shared contract and transition tests;
- frontend production build;
- Python formatting, linting, strict typing, tests, and deterministic health output;
- Rust formatting, checking, Clippy, SQLite tests, protocol tests, state tests, and overlay
  interactive-region tests;
- deterministic validation of region boundaries, malformed geometry, independent overlay
  state, and logical-coordinate conversion at 100%, 125%, and 150% display scaling.

CI runs the complete non-interactive set on a Windows runner with Node 22, Python 3.11,
pnpm, stable Rust, rustfmt, and Clippy. It does not publish installers or use secrets.

## Interactive scope

Automated checks do not prove:

- transparent WebView behavior on every Windows configuration;
- native click-through behavior under real Windows display scaling;
- always-on-top interaction with every application;
- reliable full-screen detection;
- multi-monitor positioning;
- Windows 10 packaging or installer behavior.

Use the manual checklist in `docs/WINDOWS_SETUP.md` before approving Phase 0 for the next
phase. Do not replace missing manual evidence with mocked results.

The click-through hotfix derives active regions from the rendered sprite, name label, and
thought indicator. React reports those regions only when layout or visibility changes;
Rust validates and stores them independently for each overlay and passes input through
outside the active regions. This contract is covered by deterministic tests, but native
WebView behavior still requires the focused manual retest in `docs/WINDOWS_SETUP.md`.

## Phase boundary

The current runtime supports health and shutdown only. No model request, conversation,
message persistence, memory, tool execution, network listener, Android code, or BielOS
integration is included.
