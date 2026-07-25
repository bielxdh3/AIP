# Phase 1 Validation

## Status

Phase 1 is implemented but not approved. Automated checks and a local commit do not replace the
real Windows and Ollama smoke test below. Record the exact commit SHA in the validation report;
the implementation commit message is `feat: add local conversation vertical slice`.

## Failed implementation evidence

Manual validation of `2f41be2ae558b4e7536cbd2755349c3a688a4500` failed and that commit
remains unchanged. The recorded environment was Windows 10, 100% display scaling, 1920 x 1080,
one monitor, Ollama 0.30.11, and the selected `llama3.2:1b` model.

Ollama discovery, initial Astra and Luma streaming, persistence, bubbles, click-through,
fullscreen, and safe mode passed before the failure. Ollama and its API remained healthy while
the managed AIP Python process disappeared. Active cancellation failed. FIFO, queued
cancellation, late-token, provider-interruption, simultaneous-bubble, complete history-isolation,
and idle-resource checks were not completed against that SHA.

The deterministic reproduction found the process-termination path: an exception raised while
closing the active streaming connection during cancellation could escape the Python server loop.
The command dispatcher and module entry point did not contain that exception, while Rust discarded
Python stderr, so the child could exit without a usable diagnostic cause. A second lifecycle race
allowed Python to emit a terminal event before clearing its active generation, so Rust could
dispatch the next FIFO job while Python still reported itself busy.

## Runtime lifecycle hotfix

The focused hotfix commit uses `fix: stabilize local chat runtime lifecycle`; record its exact SHA
before the new manual test. It:

- contains request-level and worker exceptions without exposing raw tracebacks or conversation
  content;
- cancels the HTTP stream through a one-shot socket interruption, outside the active-state lock;
- keeps the Python server alive after completion, cancellation, and provider failure;
- clears Python active-generation state before emitting the terminal event;
- records one authoritative Rust cancellation request, ignores later chunks, and advances FIFO
  once after the terminal event;
- captures only a 16-entry bounded allowlist of stable Python stderr codes plus the child exit code;
- reports genuine process death separately from ordinary provider failure or cancellation;
- requires explicit runtime retry and does not add an automatic restart loop.

## Remaining streamed-request failure evidence

Manual validation of `9910418c803b62e756a7966980b321c601990b04` kept Ollama and the
managed Python process healthy, but some persisted assistant attempts failed with the stable
internal class `provider_internal_error`. Direct local Ollama requests and repeated managed-runtime
requests using `llama3.2:1b` completed successfully, so the evidence does not support classifying
those failures as runtime process death.

Before the current hotfix, failed AIP sends reached provider discovery but did not produce a
generation POST in the Ollama log. The Python runtime rejected a persisted assistant message above
the user-message byte limit before `OllamaClient.stream_chat` ran, so `/api/chat` was never called.
The current hotfix applies the user limit only to user messages while retaining the aggregate context
limit, adds a bounded validation diagnostic, and incrementally decodes streamed UTF-8 NDJSON.

The remaining defect was request-level error collapse: an exception from stream cleanup could escape
the adapter's successful path and reach the Python worker's generic catch, where it became
`provider_internal_error`. The desktop then rendered every failed attempt with the same
runtime-unavailable guidance, hiding the actual class. The streamed-request hotfix:

- treats a missing provider terminal record as `provider_stream_closed`;
- prevents stream cleanup errors from replacing an already-complete terminal state;
- emits the final accepted sequence on every Python terminal event;
- rejects mismatched terminal sequences and retains a bounded, content-free request trace for queue
  dispatch, stream acceptance, persistence, finalization, and stale-event rejection;
- keeps terminal persistence write-once and queue handoff single-shot;
- prevents stale asynchronous frontend loads from replacing newer authoritative state; and
- maps provider, protocol, persistence, and runtime failures to distinct concise Portuguese text.

The new local hotfix commit must be recorded in the validation report before manual retest. Phase 1
remains pending.

Focused tests now cover `llama3.2:1b` dispatch to `POST /api/chat`, assistant context above the
user-message limit, classified pre-dispatch validation failure, split multi-byte UTF-8 stream input,
and initial/bottom-follow conversation scrolling. Manual Windows UI retest remains pending.

## Automated boundary

CI and local automated tests use synthetic provider data and temporary SQLite databases. They do
not require Ollama, an installed model, or external internet. Coverage includes:

- migration 1 to 2, fresh initialization, idempotent reopen, isolation, ordering, transitions,
  settings, and interrupted recovery;
- loopback adapter discovery, zero models, malformed and oversized data, show details, streaming,
  metrics exclusion, cancellation, shutdown, tool blocking, reasoning exclusion, and redirects;
- persistent runtime protocol parsing, bounded correlation, FIFO queue, cancellation races,
  queue bounds, safe clear, and a synthetic persistence/restart pipeline;
- frontend provider/model copy, agent-isolated streaming reduction, duplicate/out-of-order events,
  terminal idempotence, compact preview, gesture thresholds, and bubble-region add/remove;
- Phase 0 native-region, scale conversion, safe-mode, and overlay isolation regression tests.

The lifecycle hotfix additionally covers a single persistent Python server sequence of health,
completion, health, cancellation, health, provider failure, health, another completion, and
explicit shutdown. Rust process-fixture coverage verifies completion/cancellation/provider-failure
survival, sanitized bounded stderr capture, unexpected exit classification, and explicit restart.
Queue and frontend tests cover idempotent cancellation, late/stale event rejection, FIFO
advancement, duplicate-cancel prevention, authoritative terminal status, and per-agent isolation.

## Local implementation gate

The pre-commit implementation gate completed successfully on the implementation worktree:

- secret/privacy scan: 88 repository files checked;
- contracts: 2 tests passed;
- desktop frontend: 19 tests passed;
- Python runtime: 23 tests passed;
- Rust/Tauri core: 34 tests passed;
- ESLint, TypeScript, Ruff, mypy, Cargo fmt, Cargo check, and Clippy passed;
- Vite production build and `tauri build --no-bundle` passed;
- `git diff --check` passed.

The repository-wide Prettier check still reports only the four unchanged Tauri-generated JSON
schemas already present before Phase 1. All changed frontend and contract files pass formatting.
These automated results do not approve the phase or replace the manual checks below.

A local Ollama lifecycle smoke test using the installed `llama3.2:1b` model also completed a
response, cancelled an active streaming response with no late chunks, passed health checks after
both terminal paths, completed a subsequent response in the same Python process, and exited cleanly
only after explicit shutdown. Model output was suppressed and was not persisted by the test.

## Manual environment record

Record without screenshots or conversation contents:

- commit SHA;
- Windows version;
- display scaling, resolution, and monitor count;
- Ollama version;
- selected provider-qualified model reference, without filesystem details.

## Degraded startup and discovery

1. Stop Ollama and start AIP.
2. Confirm the panel and both histories remain readable.
3. Confirm `Ollama indisponível` and a clear disabled-send reason.
4. Start Ollama, choose `Atualizar modelos`, and recover without restarting AIP.
5. Confirm installed models appear and no model is downloaded.
6. Select one installed model and set a keep-alive value.
7. Restart AIP and confirm both settings persist.
8. If a saved selection is unavailable, confirm it remains visible as unavailable without a
   silent fallback. Do not delete a valuable model solely for this test.

## Conversations, streaming, and persistence

1. Send one synthetic short message to Astra.
2. Confirm the user message appears promptly and assistant text streams incrementally.
3. Confirm the response reaches `Concluída`.
4. Restart and confirm Astra history persists.
5. Send one synthetic message to Luma and confirm the histories remain separate.
6. Confirm provider/model output is plain text and no reasoning or tool action appears.

## Queue and cancellation

1. Start a longer Astra generation and enqueue Luma before it completes.
2. Confirm only Astra generates first and Luma says `Aguardando processamento…`.
3. Confirm Luma starts after Astra reaches one terminal state.
4. Cancel one queued attempt and confirm it never contacts the model.
5. Cancel one active generation while text is streaming.
6. Confirm valid partial text is preserved as cancelled, no late chunks return, and the next
   queued generation proceeds.

## Speech bubbles and overlay regression

For both Astra and Luma:

1. Single click opens the compact bubble and its preview is no more than three lines.
2. Expand to read full assistant text, reply, cancel, and open the correct full conversation.
3. Keep both bubbles open simultaneously; completing one must not close the other.
4. Confirm agent outer transparency and transparent sprite pixels pass clicks through.
5. Confirm painted pixels, labels, and the visible bubble are interactive.
6. Confirm transparent space around each bubble passes through and closing removes its region.
7. Confirm normal click does not drag, movement beyond the threshold does drag, and double click
   opens the correct conversation.
8. Restart and confirm agent positions persist.
9. Confirm full-screen and safe mode hide and restore agents and previously open bubbles.
10. Confirm click-through remains correct after both restoration paths.

## Provider interruption and resources

1. Start a synthetic longer generation, stop Ollama, and confirm AIP remains open.
2. Confirm valid partial text becomes a generic failed response and history stays readable.
3. Restart Ollama, refresh, and confirm a new message succeeds without duplicating the old user
   message.
4. Confirm there is no obvious idle CPU regression, the UI stays responsive, and only one heavy
   generation runs at a time.
5. Close AIP and confirm Python exits and `git status` remains clean.

## Approval rule

Phase 1 may become `[DONE]` only after all checks pass against the exact local commit, the phase
review finds no unresolved material defect, push is explicitly authorized, and the exact pushed
SHA receives confirmed remote CI. Until then:

`Phase 1 remains pending manual Windows and Ollama validation.`
