# Phase H versioning and shipped behavior

Status: 0.2.2 is unreleased development. The current public stable installer is
the v0.1.0 MSI linked from the README. No tag, release, or installer publication
is implied by the active development metadata.

## Versioning policy

AIP uses SemVer (`MAJOR.MINOR.PATCH`) for active workspace, desktop, contract,
runtime, and Tauri package metadata. A coordinated product/runtime change keeps
the active manifests aligned; the Rust package version is also the source used
by the desktop application version display. The active development version is
0.2.2.

The v0.1.0 tag, release asset, release notes, and validation records are
historical evidence and remain unchanged. Fixture snapshots and contract test
versions are historical test data, not active package metadata.

Phase H version validation reads the active workspace, desktop, contract,
runtime, and Tauri manifests, using the root `package.json` version as the
canonical value. With pnpm 11.9.0 and lockfile v9, `pnpm-lock.yaml` does not
encode the workspace package's own version under `importers: .`; it remains a
dependency lock and is checked through `pnpm install --frozen-lockfile` rather
than a fabricated root-importer version field.

## Theme and shared controls

The desktop theme is local UI state. `ThemeControls` supports concrete dark and
warm paper modes, primary and secondary colors, compact and soft radius presets,
interface fonts, and reduced-motion behavior. Legacy `system`, `light`,
`standard`, and system-font values are normalized deterministically when read.
The `ThemeProvider` applies the resolved values to the document and persists them
locally.

Shared `AipSelect` and `FilePicker` controls provide the common labeled and
keyboard-accessible selection and file-input behavior used by the settings and
desktop surfaces. These controls do not change Rust authority or model state on
their own.

## Auto/Equilibrado model policy

The local model preferences surface exposes `Auto/Equilibrado`, quality,
speed, and manual/advanced modes. Saved preferences are local to the computer:
they can hide a model from selectors, exclude it from routing, mark it as
fallback-only, or mark a preferred model. They do not install, remove, or load
models.

For Auto/Equilibrado, quality, and speed, Rust validates the policy and ranks
healthy, compatible candidates. An explicitly selected model is used as a
preference when no explicit preferred model is saved, while another compatible
candidate may be used when that selection is unavailable. Manual mode requires
the selected model exactly and does not perform this fallback.

## Node and provider terminology

The orchestration layer represents available compute as a node and a model host
as a provider. The current local synchronization uses the `local-ollama` node,
the `ollama` provider, and bounded model references. Provider and node health are
updated from the local provider snapshot; these names do not claim cloud hosting
or a remote model service.

## Runtime auto-readiness

Outside safe mode, the Rust desktop core starts the managed Python runtime during
application setup. The runtime reports a bounded starting/ready lifecycle after
the health handshake, and provider discovery then reports local model
availability. Runtime failure produces stable status codes while the desktop
state, settings, history, and safe mode remain available. Safe mode deliberately
does not start the runtime.

## Routing and fallback boundary

Rust/SQLite remains authoritative for lifecycle, queue, reservations, policy
validation, and chat state. The UI sends bounded policy data through the existing
Tauri chat commands; it does not schedule generations independently. Candidate
selection respects health, capability, exclusions, fallback-only rules, queue
capacity, and the safety guardrails before a generation is admitted.

## Authenticated remote transport boundary

The bounded `aip-gateway-v1` transport is a local/private checkpoint with
authenticated framed HMAC messages and Rust/SQLite authority. The validated path
is loopback; private-LAN binding requires explicit confirmation and remains a
separate workflow. The gateway is separate from the internal Python runtime and
does not create a public relay, router port-forward, cloud/BielOS integration, or
unauthenticated remote model path. Remote actions remain subject to ownership,
replay, approval, revocation, safe-mode, and temporary-chat gates in Rust.
