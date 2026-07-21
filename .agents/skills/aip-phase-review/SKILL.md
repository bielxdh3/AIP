---
name: aip-phase-review
description: Use when reviewing an AIP commit or completed phase, comparing implementation with scope, or selecting the next AIP phase.
---

# AIP Phase Review Skill

Use this skill to review completed AIP phases and prepare the next step.

Keep output compact. Do not narrate file inspection or provide progress updates.

## Review Goals

Verify whether the reviewed change:

1. Matches the requested phase and acceptance criteria.
2. Stayed within scope.
3. Preserved standalone AIP boundaries.
4. Preserved agent identity and model separation.
5. Kept the UI usable when the runtime or model is unavailable.
6. Avoided unrelated refactors and future-phase implementation.
7. Avoided secrets, local user data, model files, databases, exports, or backups.
8. Updated documentation when behavior or architecture changed.
9. Added or preserved relevant validations.
10. Preserved Windows 10 64-bit as the minimum target.

## Required Inspection

Inspect:

- latest relevant commit SHA and message;
- changed files and diff;
- requested phase and acceptance criteria;
- relevant tests and validations;
- architecture and data-boundary changes;
- security-sensitive behavior;
- documentation changes.

Do not claim CI passed unless confirmed.

If workflow results are unavailable, state:

`Remote CI not confirmed.`

## Verdict Labels

Use one:

- `Approved`
- `Approved with reservations`
- `Hotfix required`
- `Rejected`

## Codex Level Guidance

- `Low`: copy, docs, tiny deterministic changes.
- `Medium`: repository bootstrap, simple UI, scripts, tests, small Rust/TypeScript changes.
- `High`: persistence, IPC, process lifecycle, permissions, temporary-chat guarantees, schema migrations, resource scheduling.
- `Extra High`: critical security review, repeated failures, or broad architectural migration.

## Next Phase Prompt

When asked for the next step, provide a complete Codex prompt containing:

- current latest commit;
- selected skill;
- task name;
- source-of-truth documents;
- scope;
- acceptance criteria;
- explicit do-not rules;
- documentation expectations;
- validation expectations;
- expected commit message;
- required final response format.

Keep the prompt phase-scoped.

## Output Format

Use only:

- Status
- Commit
- Files changed
- Behavior changed
- Review verdict
- Issues found
- CI/validations
- Next recommended phase
- Codex level