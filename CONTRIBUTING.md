# Contributing to AIP

Thanks for helping improve AIP.

AIP is a local-first desktop system with explicit privacy, persistence, process, permission, and model-runtime boundaries. Contributions should preserve those boundaries and remain small enough to review with evidence.

## Before you start

1. Read [README.md](README.md) for the shipped scope.
2. Read [AGENTS.md](AGENTS.md) before using an AI coding agent in this repository.
3. Review [SECURITY.md](SECURITY.md) and [docs/SECURITY_AND_PERMISSIONS.md](docs/SECURITY_AND_PERMISSIONS.md) for security-sensitive work.
4. Search existing issues and pull requests.
5. Keep one pull request focused on one coherent objective.

Do not mix unrelated cleanup, refactors, formatting, dependency upgrades, and features in the same pull request.

## Development setup

Requirements are documented in [README.md](README.md) and [docs/WINDOWS_SETUP.md](docs/WINDOWS_SETUP.md).

Typical setup:

```powershell
pnpm install
python -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install -e ".\services\runtime[dev]"
```

## Validation

Run the checks relevant to your change. The primary workspace validation is:

```powershell
pnpm check
```

Useful focused checks:

```powershell
pnpm secrets:scan
pnpm lint
pnpm typecheck
pnpm test
pnpm build
pnpm python:check
pnpm tauri:check
```

Android changes should also run the Gradle test, lint, and debug-build tasks used by CI.

Never claim a validation passed unless it actually completed.

## Security-sensitive areas

Changes require extra scrutiny when they affect:

- authoritative SQLite persistence;
- agent identity, memory, conversation, or temporary-context lifecycles;
- Rust/Python process boundaries;
- permissions or external tools;
- file-system access;
- local or remote networking;
- secrets and credentials;
- model/runtime execution;
- exports, imports, backups, or recovery;
- Android/private-LAN transport;
- safe mode;
- release packaging.

Preserve fail-closed behavior and least privilege. Treat model output and external input as untrusted data.

## Pull requests

A good pull request should include:

- the user or engineering problem;
- the intended behavior;
- the exact validation performed;
- tests for behavior changes;
- screenshots or native evidence for UI changes when useful;
- security/privacy implications when applicable;
- documentation changes for user-visible behavior.

Use the repository pull request template.

## AI-assisted contributions

AI-assisted work is welcome, but the contributor remains responsible for the submitted code and claims.

Before submitting:

- inspect the actual diff;
- run the relevant checks;
- verify no secret or private local data was introduced;
- verify generated code preserves AIP's trust boundaries;
- do not present generated output as reviewed evidence unless you reviewed it.

## License

Unless explicitly stated otherwise, contributions intentionally submitted for inclusion in AIP are accepted under the repository's [Apache License 2.0](LICENSE).
