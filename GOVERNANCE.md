# Governance

AIP currently uses a maintainer-led governance model.

## Maintainer

The repository owner, `@bielxdh3`, is the final decision maker for repository scope, roadmap, merges, releases, security response, compatibility, and project policy.

## Decision model

- Issues capture reproducible problems, proposals, and acceptance criteria.
- Pull requests implement focused changes and provide review evidence.
- `main` is the integrated source of truth.
- Stable-release claims must remain separate from unreleased roadmap or phase work.
- Security, privacy, persistence, permission, and shipped-behavior invariants take precedence over convenience.

## Security-sensitive decisions

Changes affecting persistence authority, memory lifecycle, temporary context, permissions, process isolation, network exposure, credentials, external tools, safe mode, or release boundaries require explicit review.

## Releases

Releases should satisfy the applicable gates in [docs/RELEASE_CHECKLIST.md](docs/RELEASE_CHECKLIST.md). A merged implementation is not automatically a stable shipped capability.

## Contributions

Contributions are welcome, but a technically valid change may still be declined when it conflicts with project scope, trust boundaries, maintainability, or the current roadmap.

## Governance changes

This file may evolve as sustained contribution volume or additional maintainers make a broader model useful.
