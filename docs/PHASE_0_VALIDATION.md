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
- deterministic validation of alpha-mask geometry, malformed snapshots, independent overlay
  state, native adapter success/failure, teardown, gesture transitions, and coordinate
  conversion at 100%, 125%, 150%, 175%, and 200% display scaling.

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

The first click-through hotfix used DOM rectangles and cursor-dependent whole-window input
toggling. Manual Windows 11 validation of commit `9540f0a` proved that transparent areas still
intercepted clicks and that native drag prevented the thought gesture.

The second hotfix derives compact regions from painted sprite alpha, adds the visible label
and thought rectangles, converts CSS logical coordinates to physical window coordinates
exactly once, and installs the resulting shape with the Win32 window-region API. A small
click-versus-drag state machine delays native drag until movement crosses its threshold.
Automated tests cover this contract, but real unrelated-application pass-through remains
pending. The tested machine's resolution was 1920 x 1080; its Windows scaling was not recorded.

## Phase boundary

The current runtime supports health and shutdown only. No model request, conversation,
message persistence, memory, tool execution, network listener, Android code, or BielOS
integration is included.
