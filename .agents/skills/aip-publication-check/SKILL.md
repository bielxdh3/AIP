---
name: aip-publication-check
description: Use when reviewing AIP public repository safety, scanning for secrets or private data, or preparing a public release.
---

# AIP Publication Check Skill

AIP is public by design. Treat every commit as immediately publishable.

Keep output compact. Do not narrate every search. Never print secret values.

## Goal

Determine whether the current repository state is reasonably safe for public visibility and distribution.

This is not a guarantee. Full assurance requires local full-history scanning and release artifact inspection.

## Required Checks

Inspect for:

- `.env` and `.env.*`;
- API keys, access tokens, and credentials;
- private keys or credential files;
- local databases;
- chat histories and memories;
- agent export packages;
- backups and dumps;
- model files;
- voice samples;
- screenshots and personal media;
- logs containing private values;
- real owner credentials;
- private BielOS operational details;
- hardcoded local paths that reveal personal information;
- release artifacts containing development data.

Check `.gitignore`, example configuration files, documentation, fixtures, tests, and generated release packages.

## Search Terms

Use searches such as:

- `.env`
- `API_KEY`
- `ACCESS_TOKEN`
- `AUTH_TOKEN`
- `CLIENT_SECRET`
- `BEGIN PRIVATE KEY`
- `ghp_`
- `github_pat_`
- `AKIA`
- `ASIA`
- `AIza`
- `xoxb`
- `sk_live`
- `sk_test`
- `Authorization`
- `Bearer`
- `password`
- `cookie`
- `session`
- `backup`
- `dump`
- `sqlite`
- `.db`
- `.gguf`
- `.safetensors`
- `AIP-Data`
- `.bielagent`

## Required Local Verification

When available, run:

```bash
pnpm secrets:scan
git status --short
git ls-files | grep -Ei '(^|/)(\.env|.*\.pem|.*\.key|.*\.p12|.*\.pfx|.*\.db|.*\.sqlite|.*\.dump|backup|exports?|models?|logs?)'
git grep -n -I -E 'ghp_|github_pat_|AKIA|ASIA|AIza|xox[baprs]-|sk_live_|sk_test_|BEGIN .*PRIVATE KEY|CLIENT_SECRET|API_KEY|AUTH_TOKEN|ACCESS_TOKEN|Authorization: Bearer'
```

Before a release, also inspect the generated installer and bundled resources.

If a real secret appears in history:

1. Rotate it immediately.
2. Treat it as compromised.
3. Remove it from the current tree.
4. Decide whether history rewriting is necessary.
5. Do not publish a release until corrected.

## Publication Verdicts

Use one:

- `Public-ready with normal caution`
- `Blocked until corrected`
- `Release artifact review required`
- `Create a sanitized release`

## Output Format

Use only:

- Status
- Current visibility
- Findings
- No obvious secret found
- Remaining risk
- Required local checks
- Recommendation

Do not claim the repository or history is fully safe unless the relevant scans were actually completed.