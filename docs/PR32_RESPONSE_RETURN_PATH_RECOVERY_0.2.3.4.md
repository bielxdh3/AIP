# PR #32 response return-path recovery — 0.2.3.4

## Scope and identity

This correction continues PR #32 on `fix/chat-ux-runtime-023`. The live head before
the correction was `7ea2a8af0c5095b65b04f869aed1aae0dbbb84f1`. The application keeps
strict internal SemVer `0.2.3`; the visible build revision is `0.2.3.4`.

The audited return path is:

```text
Ollama NDJSON -> Python parser -> Python protocol stdout -> Rust notice parser
-> correlated queue -> SQLite append -> terminal commit -> Tauri event -> React merge
```

Production diagnostics contain request identity, bounded milestones, and counters only;
they never persist provider response text.

## Root-cause answers

1. **Did Ollama generate the full answer in the Owner's failing path?** No correlated
   `0.2.3.3` Owner trace was supplied, so this cannot be proven for either failing PC.
   Controlled real-Ollama requests did return multi-chunk content, and the deterministic
   fixture proves the rest of the path with known content.
2. **How many chunks/bytes did Ollama produce?** The historical failing count is
   unavailable. In an accounted real run against `qwen3.5:0.8b`, the first two controlled
   marker turns each produced 7 chunks, 12 UTF-8 bytes, and 12 characters. A separate
   natural-language probe produced 209 chunks/1,343 bytes and 207 chunks/1,343 bytes for
   turns 1/2; its third turn completed with no text chunks. The deterministic fixture
   emits 4 chunks for `AIP-STREAM-TEST-OK` and one-character streams of 540/542 chunks
   for the two long cases (each contains 128 numbered suffix segments).
3. **How many did Python receive?** For the accounted controlled turns, Python's
   `provider_chunks` counters were 7 and 7 (and 0 for the model's empty third result).
4. **How many did Python emit?** The corresponding `runtime_chunks` counters were 7,
   7, and 0. The runtime test requires provider and emitted counts to match for every
   non-empty provider content frame.
5. **How many did Rust receive?** The deterministic queue accepts every contiguous
   chunk for one request; the 128-chunk regression ends with `rust_chunks=128`. The
   coordinator fixture also records one accepted Rust chunk for each sequential turn.
   A real-Ollama run through the installed GUI was not available on this development
   machine, so no real Owner Rust count is claimed.
6. **How many characters were persisted?** SQLite append/reopen coverage persists and
   reconstructs exact content for 1, 10, and 200 UTF-8 chunks. The coordinator fixture
   records the exact persisted character totals (7 and 8 for its two marker turns).
   The installed runtime smoke exercises the runtime side of the boundary; its clean
   Rust/GUI result is gated on CI/Owner installation.
7. **At which exact boundary was historical content lost?** It remains unproven for
   the Owner failure because the old build had no correlated content-free counters.
   No loss was observed in the controlled provider -> Python -> Rust -> SQLite path.
   The concrete defect found and fixed was a full React phase reload being allowed to
   replace a longer active in-memory prefix with an older persisted prefix. The merge
   now preserves a monotonic active stream until an authoritative terminal snapshot.
8. **Was `Oi` first-chunk persistence, last-chunk overwrite, stale reuse, or something
   else?** The available evidence cannot select one historical cause. The audited Rust
   and React event paths append (`content + chunk`), and terminal completion carries no
   replacement content. A stale shorter reload was a real truncation risk and is now
   covered by regression tests; it is not asserted to be the cause of the Owner's
   one-word symptom without its trace.
9. **What caused the second-turn freeze?** It was not reproduced in the controlled
   coordinator or runtime probes. The active queue is cleared before terminal dispatch,
   late/stale events are rejected, and health remained `ready` after each successful
   controlled turn. The empty third result in one real-model rerun was a provider/model
   output result with a clean terminal and healthy runtime, not evidence of a UI lock.
   An exact historical freeze mechanism therefore remains unknown pending Owner trace.
10. **Was auto-title involved?** It cannot overwrite assistant content. Title context is
    read only after the chat database transaction has committed. Scheduling now occurs
    after the terminal event and `frontend.message.visible` milestone, so title work
    cannot consume chat chunks or hold the chat request open. Existing one-shot/manual
    title tests remain green.
11. **Why did the previous installed smoke pass?** It checked provider receipts and two
    short responses, but did not reconstruct an exact full response, require 100+ chunks,
    exercise a third same-conversation turn, check liveness between turns, or correlate
    the new return-path milestones. It therefore could pass while an Owner-visible
    truncation remained possible.
12. **What prevents regression now?** The `.4` smoke requires the exact MSI filename,
    exact `AIP-STREAM-TEST-OK` reconstruction, contiguous 128-chunk reconstruction for
    two additional turns, terminal success, three distinct same-conversation receipts,
    post-turn health responses, runtime trace milestones, and uninstall/temporary-file
    cleanup. Phase H and CI bind those checks to visible build revision `0.2.3.4`.

## Return-path accounting added

The Python runtime emits bounded counters at provider and protocol milestones. Rust
accepts only an allowlisted bounded counter map and records the same map in the durable
request trace. The chat queue tracks Rust chunk/byte/character totals and records
`rust.terminal.received`, `rust.message.finalized`, and `frontend.message.visible`.
Per-chunk trace entries remain in memory only; they are excluded from the durable file
to avoid rewriting the trace for every token. The durable file remains:

```text
%LOCALAPPDATA%\br.dev.biel.aip\diagnostics\generation-trace.ndjson
```

The frontend merge keeps an active assistant's longer monotonic prefix across a stale
reload, while an equal-length terminal snapshot is authoritative and clears the active
request sequence. `frontend.message.visible` is the Rust-to-Tauri event handoff
milestone; actual DOM paint is covered by frontend tests, not counted as a synchronous
Rust observation.

## Evidence and validation

- Real controlled marker probe (`qwen3.5:0.8b`): one clean run completed turns 1/2/3
  with 7 chunks, 12 characters, exact markers `AIP-REAL-731`, `AIP-REAL-942`, and
  `AIP-REAL-553`, contiguous sequences, terminal `generation.complete`, and `health=ready`
  after every turn. An accounting rerun reproduced turns 1/2 and recorded 7/7/0 chunks;
  this model variability is recorded rather than hidden.
- Python: 47 unit tests, Ruff format/lint, and mypy all pass.
- Rust/Tauri library: 223 passed, 0 failed, 1 explicitly ignored provider test;
  clippy with `-D warnings` and `cargo fmt --check` pass.
- Frontend: 174 Vitest tests pass, including ordered chunk growth and stale-reload
  protection. Typecheck and production Vite build pass.
- Phase H, secret scan, and changed-file formatting pass. The repository-wide Prettier
  command still reports pre-existing generated/unrelated files; no unrelated files were
  reformatted by this correction.
- The local release build produced the internal-SemVer MSI and NSIS bundles. Local
  installed smoke was blocked by a pre-existing per-machine A.I.P. registration that
  requires Administrator elevation (`Error 1730`/`1603`); this is not claimed as an
  installed-pass result. Clean Windows CI run `34496733666` at head
  `ed2d0647d185f23eeea69414955f5da3b95c502a` passed Android, phase-zero, package, and
  installed-windows-smoke. Its artifact is
  `aip-windows-0.2.3.4-ed2d0647d185f23eeea69414955f5da3b95c502a` (ID `10161152568`,
  digest `sha256:687010976513ff28e7e7975cf6362e748d36d9e965d4e7e4ace39f1b3b796260`).
  The installed smoke verified the exact MSI
  `A.I.P._0.2.3.4_x64_en-US.msi` (SHA256
  `B271B11497420316B7A6DF13EF9501B2C07CFD276A0C4810BA15F515EDB55EA2`), three
  same-conversation turns with 4/540/542 chunks, per-request milestones/counters,
  liveness, and cleanup/uninstall.
- The two response-return-path review threads were replied to and resolved after the
  reader-bound and per-request correlation corrections. One unrelated temporary-chat
  conversion race remains open and is outside this correction's scope.

## Preserved scope and owner gate

Temporary-chat privacy/tombstones, durable trace retention/privacy, cancellation
watchdogs, loopback Ollama policy, routing/model fail-closed behavior, safe mode, title
architecture, compact chat UI, overlays, and issues #24–#29 remain in place. Model-picker
scrollbars, metadata overlap, generation animation, provider styling, and unrelated
visual polish remain deferred.

PR #32 stays open. No merge, auto-merge, release, tag, deploy, or `main` rewrite is part
of this correction. Clean-admin CI has completed the scripted installation gate; the
next Owner smoke must install the exact `0.2.3.4` MSI in the Owner environment and
verify three full responsive turns. If it fails, retain the generation trace for
boundary identification.
