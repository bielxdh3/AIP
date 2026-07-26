# Phase 6 automated validation

## Candidate

The current local candidate is commit `15e1e9c6f6f16e1397096a943447677835375def`.
It is not published, tagged, signed, or pushed by this validation record.

## Completed automated checks

- `pnpm check` passed: secret scan, lint, TypeScript, contracts, desktop tests,
  production frontend build, Python checks, Rust formatting, Clippy, and Rust tests.
- The desktop test suites passed 78 tests. The Ollama-dependent Rust test remains
  excluded from the normal suite, but it was executed separately with the local
  `llama3.2:1b` model and passed.
- The Tauri release build produced both the NSIS and MSI artifacts.
- The MSI was administratively extracted into a temporary directory for inspection.
  The extracted payload contained the application executable and MSI metadata only;
  no database, logs, models, media, exports, or credentials were found.
- The NSIS package was silently installed into a temporary directory, its installed
  executable was started successfully, and the temporary installation was removed.
- The temporary inspection directory was removed after the check.

## Manual gate still required

The candidate is not a release approval. A real Windows user must still validate the
current packaged application for onboarding, agent isolation, temporary-chat
non-persistence, memories, states, pixel editing, safe mode, overlays, display scaling,
multiple monitors, installer startup/update behavior, and local Ollama interaction.

Record manual results in `V0_1_MANUAL_VALIDATION.md` before releasing AIP v0.1.
