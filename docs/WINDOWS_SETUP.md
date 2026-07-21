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

### Recorded Phase 0 click-through evidence

The first hotfix failed real Windows validation. The second native-region hotfix passed the
following checks against an unrelated ordinary application at runtime commit
`a6ccb1badf6aa8a1f317ea1818c247d87f311fe6`:

1. Test outer transparency above, below, left, and right of Astra and Luma.
2. Test transparent corners inside each 128x128 sprite box.
3. Confirm painted sprite pixels and visible names remain interactive.
4. Trigger the thought indicator with a double click and test the indicator itself.
5. Confirm the area around the indicator and its former area after disappearance pass through.
6. Confirm a normal click does not drag, movement beyond the threshold does drag, and both
   positions persist after restart.
7. Recheck click-through after full-screen hide/restore and safe-mode hide/restore.
8. Cross interactive boundaries repeatedly and check for an obvious idle CPU regression.
9. Confirm Git remains clean.

The recorded environment was Windows 11, 100% display scaling, 1920 x 1080 resolution, and
one active monitor. The visible thought indicator participates in the draggable interactive
region but has no separate Phase 0 button action. Windows 10, non-100% real display scaling,
multiple monitors, and installer behavior remain unvalidated manual configurations.
