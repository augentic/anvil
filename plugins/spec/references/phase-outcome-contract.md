# Phase outcome contract

Specify retired per-slice `PhaseOutcome` stamping and `specify slice outcome set`. The `/spec:execute` driver parks on phase exit codes, slice lifecycle, and plan entry status — not an on-disk outcome field.

> See [Stop conditions](https://specify.augentic.io/reference/change-skills/execute.html) halt paths in the execute skill references, and `references/spec-runtime/stop-conditions.md` in cached adapters.

Durable run telemetry lives at `.specify/journal.jsonl`; the journal event taxonomy is implemented in the CLI repo and summarized by the lifecycle references. Phase skills append structured JSON lines there; `/spec:execute` does not read the journal as a signalling channel.

Target adapter briefs link here for navigation; brief-local deltas describe merge/build failure handling under the stop-conditions model.
