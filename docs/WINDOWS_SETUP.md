# Windows Development Setup

Phase 0 targets Windows 10 and Windows 11, both 64-bit. The desktop remains usable when
the Python runtime is unavailable, but building the Tauri application requires the native
Windows toolchain.

## Prerequisites

Install these tools through their official installers:

1. Node.js 22 LTS or a newer compatible release.
2. pnpm 11.9 or newer within the same major release.
3. Stable Rust with the `x86_64-pc-windows-msvc` target, `rustfmt`, and Clippy.
4. Microsoft C++ Build Tools with the Desktop development with C++ workload and a
   Windows SDK.
5. Microsoft Edge WebView2 Runtime.
6. Python 3.11 or newer.

No global Python package is required. Use the repository virtual environment.

## Install repository dependencies

```powershell
pnpm install
python -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install --disable-pip-version-check -e ".\services\runtime[dev]"
```

## Validate

```powershell
pnpm secrets:scan
pnpm lint
pnpm typecheck
pnpm test
pnpm build
pnpm python:check
pnpm tauri:check
```

`pnpm tauri:check` runs Rust formatting, checking, Clippy with warnings denied, and tests.

## Run

```powershell
pnpm dev
```

Normal startup initializes the local SQLite database, opens the main panel, creates two
overlay windows, and attempts the Python health handshake. If Python cannot start or the
handshake fails, the main panel remains available and reports a degraded state.

Safe mode persists locally, does not start Python, and hides both overlays.
The native executable also accepts `--safe-mode` for a forced recovery startup.

## Manual Windows checks

Perform these checks on a disposable Phase 0 database:

1. Confirm the Portuguese main panel opens without a model or Ollama.
2. Confirm Astra and Luma appear simultaneously outside safe mode.
3. Drag both agents, restart the app, and confirm their positions are restored.
4. Confirm safe mode hides overlays and prevents Python startup.
5. Temporarily make Python unavailable and confirm the main panel remains open.
6. Confirm the transparent outer overlay region passes clicks through while the character
   region remains draggable.
7. Confirm overlays hide over a full-screen application and return afterward.
8. Confirm no database, log, screenshot, or generated installer appears in Git status.

Click-through and full-screen detection are best-effort Phase 0 proofs. Record Windows
version, scaling, and multi-monitor limitations without including personal machine data.

### Click-through hotfix retest

Phase 0 remains pending until these directly related checks pass after the interactive-region
hotfix:

1. Confirm transparent space around Astra passes clicks to the application below.
2. Confirm transparent space around Luma passes clicks to the application below.
3. Confirm both visible agents and their labels remain draggable.
4. Restart AIP and confirm both dragged positions persist.
5. Confirm a full-screen application still hides both overlays and they return afterward.
6. Confirm safe mode still hides both overlays.
7. Record the Windows display scaling used for the test. If available, repeat click-through
   at 100%, 125%, and 150%.
