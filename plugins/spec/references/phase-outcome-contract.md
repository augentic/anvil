# Phase outcome contract

Specify has no per-slice `PhaseOutcome` stamp or `slice outcome set` CLI verb. The `/spec:execute` driver parks on `specify plan status`, which projects phase outcomes from slice lifecycle, plan entry status, and the journal's phase-terminal events — not from an on-disk outcome field.

> See [Stop conditions](https://specify.augentic.io/reference/change-skills/execute.html) halt paths in the execute skill references, and `references/spec-runtime/stop-conditions.md` in cached adapters.

Durable run telemetry lives at `.specify/journal.jsonl`; the journal event taxonomy is implemented in the CLI repo and summarized by the lifecycle references. CLI verbs append structured JSON lines there as a side effect of each phase; skills never read the file directly — `specify plan status` is the projection that turns the journal tail into the loop's stop classification.

Target adapter briefs link here for navigation; brief-local deltas describe merge/build failure handling under the stop-conditions model.
