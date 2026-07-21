# AIP Python Runtime

This package is the replaceable Python process boundary managed by the Rust desktop core.
Phase 0 implements only a versioned health handshake over NDJSON stdin/stdout. It does not
open a network port, load a model, access SQLite, or retain user content.

From the repository root:

```powershell
$env:PYTHONPATH = "services/runtime/src"
python -m aip_runtime --health
python -m aip_runtime --stdio
```

`--stdio` reserves stdout for protocol envelopes. Diagnostics are fixed, bounded messages
written to stderr.
