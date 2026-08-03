# Security Policy

## Project status

AIP is under active development. The stable baseline is local-first and Windows-focused, but security still depends on the exact revision, local model/runtime configuration, operating-system permissions, and any future external-tool or extension capabilities.

## Reporting a vulnerability

Do not publish exploit details, real conversations, memories, credentials, local paths, model data, or reproduction artifacts in a public issue or pull request.

Use GitHub's private vulnerability-reporting or Security Advisory flow for this repository when available. Otherwise contact the maintainer privately through the GitHub profile before disclosing technical details.

A useful report should include:

- affected revision or release;
- affected component;
- impact and required preconditions;
- minimal reproduction using disposable data;
- whether the issue crosses an agent, conversation, memory, runtime, or operating-system boundary;
- a suggested remediation when known.

Never test with another person's data or on a system you do not own or have explicit permission to assess.

## Sensitive data

Do not commit real conversations, memories, databases, model files, credentials, private keys, local environment files, generated packages, or private BielOS data. If a real credential is committed, revoke or rotate it immediately; deleting the latest copy alone does not remove it from Git history.