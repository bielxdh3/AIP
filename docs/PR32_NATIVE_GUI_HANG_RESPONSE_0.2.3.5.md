# PR #32 native GUI hang and interrupted-response recovery — 0.2.3.5

## Scope and identity

This is a Direct Codex correction on `fix/chat-ux-runtime-023`, continuing PR #32
without creating another PR. The exact head before this work was
`1e66f1d12a3557d2b6c9261e00851b34cc485451`. Strict internal SemVer remains `0.2.3`;
the visible Owner-facing build revision is `0.2.3.5`.

The audited path is:

```text
React composer -> Tauri command -> Rust ChatCoordinator/queue -> managed Python stdio
-> Ollama loopback stream -> Python terminal -> Rust persistence/events -> React merge
```

The historical Owner `.4` trace was not available. Its exact failure boundary is not
claimed here; the package-equivalent reproduction and the new content-free trace make
the next Owner run diagnosable.

## Root-cause answers

### Interrupted response

The locally reproduced partial-response class was a packaged-runtime encoding defect.
PyInstaller inherited the Windows console code page for sidecar stdout even when the
parent supplied `PYTHONIOENCODING=utf-8`. A pre-fix probe against an accented Portuguese
prompt produced 23 stdout lines that were not valid UTF-8. Rust's intentionally strict
`String::from_utf8` reader therefore emitted `runtime_protocol_decode_failed`; the
runtime disconnected, `ChatCoordinator::fail_all` attached `runtime_interrupted`, and
the failed message with received content was rendered as `Resposta interrompida`.

The fix explicitly reconfigures packaged stdout and stderr to UTF-8, strict errors,
newline-delimited write-through output before the server writes its first protocol
frame. The post-fix packaged probe observed zero malformed UTF-8 lines.

`Resposta interrompida` is not a fabricated success or a generic terminal replacement:
the existing UI uses it only for a failed message that has preserved content, while
`messageFailureCopy` retains the correlated reason. Provider stream exceptions use
`provider_stream_failed`; runtime/pipe loss is finalized as `runtime_interrupted`;
provider close/interruption and watchdog/persistence/protocol failures retain their
distinct error codes. Cancellation remains `generation.cancelled`/`Resposta cancelada`.

### Windows “Not responding” mechanism

The historical `.4` GUI hang cannot be proven without the Owner trace. The audited
mechanism that could starve the native UI was measurable and was removed narrowly:

- the old persistent path performed one SQLite `UPDATE` per streamed chunk;
- the old React path performed a synchronous `scrollHeight` read and scroll write for
  each chunk;
- dispatch and terminal work had wider scheduler/stream critical sections than needed.

The persistent stream now buffers at most 16 chunks or 4096 UTF-8 bytes and writes one
ordered SQLite append per batch. Chunk events still reach the UI incrementally, but
scroll work is coalesced to one `requestAnimationFrame`; no chunk handler invokes a
full `get_phase_one_state` reload or emits `state.changed`. Terminal persistence,
events, title scheduling, and the next dispatch run after the ordered stream guard is
released. This is a starvation reduction, not a claim that the unavailable historical
hang had one single proven cause.

## Return-path and amplification measurements

### Deterministic Rust/Tauri coordinator fixture

The fixture now runs two distinct complete turns, a partial provider failure, and a
1000-chunk stream through `ChatCoordinator`, managed stdio, SQLite, and the event path.

| Case | Rust chunks | Visible/persisted content | Terminal | Error | Durable batches |
| --- | ---: | ---: | --- | --- | ---: |
| `AZUL-17` | 1 | 7 chars | complete | — | 1 |
| `VERDE-93` | 1 | 8 chars | complete | — | 1 |
| `PARTIAL-FAIL` | 2 | `partial-prefix` | failed | `provider_stream_failed` | 1 |
| `MANY-CHUNKS` | 1000 | 1000 `x` chars | complete | — | 63 |

For the long case, the old hot path would issue 1000 per-chunk SQLite updates; the
bounded path issues 63 appends (62 full 16-chunk batches plus the final 8 chunks).
There are still 1000 intentional `generation.chunk` Tauri events and local React
merges, zero per-chunk `state.changed` events, and zero per-chunk
`get_phase_one_state` calls. Chunk trace codes are in-memory only; durable trace
snapshots are emitted only for allowlisted milestones, so trace-file writes are not
amplified by token count.

The partial case proves that received text remains attached to the correct assistant
message, the final status is `Failed`, and the provider error is preserved.

### Packaged sidecar + real Ollama (three exact prompts)

This probe launched the current PyInstaller sidecar directly, drained stdout and
stderr on separate reader threads, and sent the exact objective prompts to the local
loopback Ollama model `qwen3.5:0.8b`. It is package/native-equivalent runtime proof,
not installed WebView/Tauri Owner proof; Rust-layer counts are supplied by the
deterministic coordinator fixture above.

```text
Responda em 3 frases sobre o número 731 e termine com AIP-0235-A
Responda em 3 frases sobre o número 942 e termine com AIP-0235-B
Diga seu nome/identidade atual e termine com AIP-0235-C
```

All three requests were accepted, emitted `generation.started`, completed with exactly
one `generation.complete`, and left the process exit code at 0. Provider and runtime
trace counters agreed for every turn; `runtime_terminal_events` is 0 on the provider
completion milestone and 1 on the terminal milestone.

| Request ID | Provider/runtime chunks | UTF-8 bytes/chars | SHA-256 of visible text | First chunk | Duration | Max heartbeat |
| --- | ---: | ---: | --- | ---: | ---: | ---: |
| `real-0235-1` | 500 / 500 | 2990 / 2896 | `0c6155dadad86c580c4421591c5ffcbd907214a8e54d21b04fdf1f03f5b9d2af` | 16,611 ms | 18,809 ms | 0 ms* |
| `real-0235-2` | 170 / 170 | 1070 / 1034 | `a28ef0bcf75e2c36c9fbb5d4f92a2479647767b635b5480cc5f3a0650ac6baa8` | 66,274 ms | 67,062 ms | 0 ms* |
| `real-0235-3` | 89 / 89 | 421 / 412 | `3a1f1fe4b029466a3b4b7761ff261176358377fdf3af2708ec1471ea26e73662` | 43,041 ms | 43,438 ms | 0 ms* |

The durations and first-progress times are measured from each `generation.start`
write; last progress was 18,804/67,057/43,433 ms. Heartbeat values are rounded to
whole milliseconds (`0 ms` means sub-millisecond in this probe), with no stall at the
2-second threshold. The probe recorded 27 correlated runtime trace milestones and
zero malformed UTF-8 stdout lines.

## Liveness and diagnostics

The desktop runs a content-free `phase_one_heartbeat` every 500 ms while an active
request is present. The command records the active request ID, Rust sequence, runtime
PID/state/detail, watchdog phase, last progress age, last heartbeat, and maximum
heartbeat latency. A durable `liveness.heartbeat` milestone is rate-limited to at most
one every two seconds and contains only allowlisted bounded counters.

Future evidence is retained at:

```text
%LOCALAPPDATA%\\br.dev.biel.aip\\diagnostics\\generation-trace.ndjson
```

Retrieve it without exposing message content:

```powershell
Get-Content -LiteralPath "$env:LOCALAPPDATA\\br.dev.biel.aip\\diagnostics\\generation-trace.ndjson"
```

The packaged probe's separate stdout/stderr readers prove that trace volume cannot
block the protocol reader in that harness. Rust also has continuously draining reader
threads and bounded line readers for both pipes.

## Lock/blocking inventory

| Lock/state | Acquisition and scope | Slow work while held / ordering result |
| --- | --- | --- |
| `send_lock` | Serializes send/branch/temporary attempt creation and queue admission | Local validation and bounded SQLite attempt transaction only; released before dispatch/network/process work. |
| `scheduler_lock` | Queue admission and title-map state transitions | No DB, file, provider, process, callback, or event wait while the lock is held. |
| `stream_persist_lock` | Ordered buffering, bounded SQLite append, and active-queue removal | One bounded SQLite batch write is the only I/O under it; it is released before final status DB work, Tauri events, title generation, or dispatch. |
| queue mutex | Snapshot, sequence acceptance, counters, activation, terminal removal | Only short in-memory operations; the queue lock is dropped before DB/network/process work. |
| title-generation map/deferred queue | Correlation, cancellation, and metadata scheduling | Title context/settings DB reads and runtime sends happen outside the map lock. Pending user work cancels in-flight title work or keeps it deferred. |
| runtime controller/process state | Status/PID/command sender and stop/join | Reader threads continuously drain stdout/stderr; no chat lock is held while stopping or waiting for a child. |
| request-trace store/persistence | Bounded in-memory correlation; asynchronous snapshot writer | Chunk codes are not persisted; file writes run on the persistence worker. |
| temporary chat store/database connection | Temporary in-memory message mutation; each DB method owns its connection | No temporary-state lock is held during durable DB work; temporary content remains non-persistent. |

The audited order is queue state -> stream persistence (when required) -> durable
status/events -> title/next dispatch. There is no nested broad lock across provider
network, child waits, callbacks, or trace-file writes. The Rust dispatcher uses an
atomic in-flight guard and rechecks after release so a sender cannot strand a newly
queued request.

## Monotonic merge and terminal contract

The event-order regression exercises `chunk(1..10)`, stale snapshot length 5,
`chunk(11..20)`, stale snapshot length 15, `generation.complete(20)`, and both stale
generating and stale shorter-terminal snapshots. Final content remains length 20,
status `complete`, queue empty, and request sequence state cleared. Request identity,
agent, conversation, and assistant-message correlation prevent another request from
merging into the stream.

The Python server now marks terminal emission once, flushes the terminal JSON line,
clears its active slot immediately after the terminal write, then performs lower
priority trace/worker cleanup. Normal EOF is complete; provider exception is failed;
cancellation is cancelled; worker exceptions become correlated failure. Rust accepts
one matching terminal, ignores stale/duplicate terminals, flushes pending content
before finalization, clears the active queue slot, preserves partial content, and
fails an active request when stdout closes or the process exits without a terminal.

Terminal persistence and `frontend.message.visible` precede automatic title scheduling.
Manual-title precedence and one-shot title semantics remain intact; a queued user
generation preempts metadata-only title work so it cannot block a second turn.

## Validation evidence

- Frontend Vitest: 34 files, 177 tests passed.
- Contracts: 1 file, 23 tests passed.
- Rust/Tauri: `cargo fmt --check`, `cargo check --locked`, clippy `-D warnings`, and
  223 passed / 0 failed / 1 ignored (the ignored test requires a local
  `llama3.2:1b` model).
- Python: Ruff format/check, mypy, and 47 unittest tests passed.
- Workspace typecheck, ESLint, production Vite build, Phase H, secrets scan (219
  repository files), PowerShell parser, changed-file Prettier, and `git diff --check`
  passed.
- The repository-wide Prettier command still reports 12 pre-existing generated or
  unrelated files; none were reformatted by this correction.

## Build and installed smoke

The release build produced canonical internal-SemVer bundles and identity-preserving
Owner names. `aip-desktop.exe --print-build-identity` returns `0.2.3.5`.

| Artifact | Length | SHA-256 |
| --- | ---: | --- |
| `A.I.P._0.2.3.5_x64_en-US.msi` | 14,761,984 | `5610DA11E154A090998ADF6F9A2C1CFB75FFC25D226DEA0B37ECD4B1762E364A` |
| `A.I.P._0.2.3.5_x64-setup.exe` | 13,143,268 | `413CCEECFCF48D92B0F28F5800066CF5E6F73ADE885B1442963F88CC0298F49B` |
| `aip-runtime-x86_64-pc-windows-msvc.exe` | 9,470,004 | `21D36CCE4F77A0A1418321B07371391CC482C83F937C89F1159B20B87C694E39` |

The package's canonical internal filenames remain `A.I.P._0.2.3_x64_en-US.msi` and
`A.I.P._0.2.3_x64-setup.exe`; the `.5` copies are byte-identical identity-preserving
copies. No tag or release was created.

The exact local installed smoke command was run against the `.5` MSI. It stopped at
`msiexec.exe` exit 1603 before replacing the existing per-machine installation;
the retained installer log records Error 1730 (“You must be an Administrator to remove
this application”) at `RemoveExistingProducts`. Therefore there is no installed GUI,
WebView, Owner interaction, or uninstall pass to claim on this machine. The smoke
script itself now covers exact `.5` naming, installed-resource isolation, three
distinct fixture turns, 100-plus-chunk reconstruction, contiguous sequences, runtime
trace counter agreement, concurrent heartbeat, runtime-alive checks, and cleanup.

## CI, PR, and review boundary

No GitHub Actions run was started for this unpushed head, so there is no current
phase-zero/Android/package/installed-smoke job result or exact-head artifact ID/digest
to report. The historical `.4` CI result is not evidence for `.5`. The remote PR check
at handoff remains PR #32 OPEN on `fix/chat-ux-runtime-023`; no PR #33, merge,
auto-merge, tag, release, deploy, or `main` rewrite was performed.

One unrelated review P2 remains open and intentionally untouched:
`Block closes while temporary conversion is pending`. It is not counted as fixed by
this response-path correction.

Temporary-chat privacy/tombstones, strict SemVer/routing, explicit model behavior,
safe mode, cancellation/timeout recovery, title/manual-name rules, compact chat UI,
overlay fixes, and issues #24–#29 were preserved. Model-picker scrollbar/metadata
overlap, generation animation, provider styling, and general visual polish remain
deferred.

The final local commit SHA(s) and final `HEAD` are supplied by the handoff after the
working tree is committed; no remote publication is implied.

`AIP_CODEX_PR32_NATIVE_GUI_HANG_RESPONSE_0235_NEEDS_OWNER_PROOF`
