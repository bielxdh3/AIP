# Local model benchmarks

## Preliminary current-hardware profile

This is a preliminary local observation, not a model recommendation and not a
replacement for the future GTX 1060 6 GB profile.

- CPU: AMD Ryzen 7 5700G, 8 cores / 16 logical processors
- RAM: 32,056,164,352 bytes visible to Windows at measurement start
- Graphics: AMD Radeon integrated graphics
- Ollama execution: 100% CPU for every measured model
- VRAM: not available through the current hardware telemetry
- Date: 2026-07-26

The fixed task used a Portuguese prompt, disabled reasoning where supported, and
capped generated output at 64 tokens. Prompt and response content are not stored.

| Model        | Load ms | First content ms | Tokens/s | Loaded model bytes | Execution |
| ------------ | ------: | ---------------: | -------: | -----------------: | --------- |
| qwen3.5:9b   |   6,662 |            7,416 |     6.93 |      6,657,199,309 | 100% CPU  |
| qwen3.5:4b   |   5,150 |            5,568 |    12.60 |      3,435,973,837 | 100% CPU  |
| gemma3:4b    |   3,538 |            3,866 |    15.48 |      3,113,851,290 | 100% CPU  |
| llama3.2:1b  |   1,502 |            1,788 |    31.04 |      1,610,612,736 | 100% CPU  |
| llama3.2:3b  |   2,260 |            2,646 |    19.51 |      2,791,728,742 | 100% CPU  |
| qwen3.5:0.8b |   2,139 |            2,284 |    39.03 |      1,181,116,006 | 100% CPU  |

The `ollama` process working-set snapshots are retained in the JSON artifact as
observations only. They do not represent total model memory because the runtime may
use mapped or child-process memory. The `loadedModelBytes` value is the more useful
Ollama-reported loaded-size observation in this profile.

## Repeating the harness

Run the same task against every currently installed local model:

```powershell
node scripts/benchmark-local-models.mjs --output benchmarks/preliminary-current-hardware.json
```

To benchmark a selected model only:

```powershell
node scripts/benchmark-local-models.mjs qwen3.5:4b --output benchmarks/qwen3.5-4b.json
```

The GTX 1060 run must use this script and prompt policy again, but save to a separate
artifact and document the hardware as a separate profile. Do not combine the results
or use the current CPU-only profile to select a permanent default model.
