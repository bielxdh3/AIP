# PR #32 native generation recovery — 0.2.3.3

## Root-cause report

1. The exact failing boundary in the Owner's installed `0.2.3.2` is not proven. The supplied result proves that the GUI accepted a user message and entered a generating state, while Ollama did not reliably produce inference, but no correlated Owner trace or runtime log was supplied to distinguish packaged sidecar startup, protocol dispatch, provider readiness, HTTP connection, or UI finalization.
2. Earlier package-equivalent and real-Ollama probes passed because they did not install and launch the MSI's actual executable/resource layout through the Owner-facing GUI. They therefore did not prove Tauri's installed `current_exe()` lookup, sibling sidecar resolution, MSI environment, or the GUI-to-Rust queue path together.
3. The missing test boundary was an installed MSI. The extracted package layout contains `aip-desktop.exe` and `aip-runtime.exe` as installed siblings; previous tests exercised source-relative Python or a standalone sidecar instead.
4. From the Owner description only the React/Tauri acceptance and a generating-state transition are evidenced. Reachability of the Rust queue, packaged runtime process, Python server, Ollama adapter, Ollama HTTP endpoint, and terminal response is otherwise unproven for `0.2.3.2`.
5. PC A's stall has no proven cause without its local trace. It may be an acceptance/readiness/stream timeout, but that would be an inference rather than evidence.
6. PC B's terminal failure likewise has no proven cause without its local trace; the concise UI error intentionally did not identify the internal class.
7. The two symptoms may be different timeout/failure classes or manifestations of one packaged-path defect. The available evidence cannot safely choose between those explanations.
8. `0.2.3.3` adds an installed-path smoke harness and deterministic loopback Ollama fixture; preserves the MSI sibling-runtime lookup; makes packaged binaries testable in the controller; propagates content-free correlated runtime traces (`ollama.connected`, request milestones, error codes); moves trace persistence off the generation thread while loading and merging bounded prior traces across restarts; tombstones converted temporary sessions until an explicit new temporary session starts; tightens first-send duplicate locking; keeps bounded acceptance/watchdog recovery; and makes the build identity and delivered MSI filename explicit.
9. `scripts/installed-windows-smoke.ps1` installs the exact `A.I.P._0.2.3.3_x64_en-US.msi` into a clean test directory, launches the installed `aip-runtime.exe`, exercises discovery plus two distinct streaming generations against `scripts/fake-ollama-server.py`, verifies terminal success and stderr trace milestones, then uninstalls and cleans up.
10. The durable trace is at `%LOCALAPPDATA%\br.dev.biel.aip\diagnostics\generation-trace.ndjson` on Windows (the path is resolved from Tauri's app-local-data directory). Temporary-chat entries are filtered before durable serialization.

The external/managed Ollama policy is deterministic: AIP first reuses a healthy validated loopback service; it starts a managed process only from the explicit Owner-provided `AIP_OLLAMA_EXECUTABLE` or bounded `AIP_OLLAMA_CONFIG`; it never guesses or downloads an executable, and rejects remote or wildcard endpoints.

This report records the evidence boundary rather than attributing the Owner's two PCs to an unobserved root cause. The next Owner smoke must use the exact 0.2.3.3 MSI and retain the trace if either generation stalls or fails.
