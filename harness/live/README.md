# live harness

This non-shipped package holds the one explicit live-model workflow test: a single ignored native trial that drives the same fixture workflow as the scripted suites — the adversarial lead set with a cross-source overlap, an authority disagreement, and an evidence gap — against the configured cursor model, then grades it with the deterministic validators only (schema, coverage, provenance, tags, lifecycle, build output). Per-leg repair counts are reported without being asserted; the temporary project tree is retained on failure.

Run it from the repository root:

```shell
cargo make test-live
```

Requires cursor-agent on `PATH` with credentials (`cursor-agent login` or `CURSOR_API_KEY`). Single trial, never CI.

**Cadence** — manual means scheduled by convention, not never: run `cargo make test-live` before tagging a release and after any change to the judgment prompts (`crates/slice/prompts/`, `crates/change/prompts/`) or the answer schemas. A judgment leg drifting from zero repairs toward the repair budget is the early warning that a change degraded the model's first answer, visible before it becomes a failure.
