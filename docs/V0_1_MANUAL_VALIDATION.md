# AIP v0.1 manual validation

The automated release build does not replace Windows validation. Before approval, verify onboarding, profile edits, conversation switching, temporary-chat non-persistence, memory workflows, state/mode behavior, pixel editing, click-through, fullscreen, safe mode, DPI, multi-monitor recovery, packaged startup, updates, and local Ollama generation.

The current candidate includes locally built NSIS and MSI packages. Python quality checks require the repository development dependency `ruff`; its absence is an environment prerequisite, not a runtime claim.

## Approval record

Manual Windows validation of the installed package was approved on 2026-07-30.

- Tested packaged SHA: `6b5dc1a0a18d3e346d04c6bd89de13775c681434`
- CI run: [30474813207](https://github.com/bielxdh3/AIP/actions/runs/30474813207) (successful)
- Artifact: `aip-windows-6325dffdbaca951f6417310208da7e68148e13bc`
- Result: the packaged application opened, remained usable, and closed and reopened successfully. The blank-screen hook-order crash was resolved.

The accepted v0.1 limitations are that AIP does not start Ollama automatically, and the current visual design is functional but awaits a dedicated redesign phase.

## Generation cancellation and recovery

1. Start Ollama and the installed AIP package, then confirm the runtime is ready.
2. Refresh models, select an installed model, and send one short prompt. Confirm one variant only.
3. Cancel once before the first token, then after several streamed tokens, and repeat the cancel control. Each request must settle as cancelled and leave the queue usable.
4. Send another prompt without restarting AIP. It must complete normally.
5. Stop Ollama during a stream, restart it, refresh models, and confirm discovery is no longer stuck checking.
6. Regenerate explicitly and confirm exactly one additional variant appears. Restart AIP and confirm no message remains pending or streaming.
