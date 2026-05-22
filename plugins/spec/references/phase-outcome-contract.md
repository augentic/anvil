# Phase outcome contract

RFC-25 retired per-slice `PhaseOutcome` stamping and `specify slice outcome set`. The `/spec:execute` driver parks on phase exit codes, slice lifecycle, and plan entry status — not an on-disk outcome field.

> See [Stop conditions](../skills/execute/references/stop-conditions.md) for the three halt paths and re-entry contract.

Durable run telemetry lives at `.specify/journal.jsonl` (RFC-25 [Observability](https://github.com/augentic/specify/blob/main/rfcs/rfc-25-workflow.md#observability-rfc-19)). Phase skills append structured JSON lines there; `/spec:execute` does not read the journal as a signalling channel.

Target adapter briefs link here for navigation; brief-local deltas describe merge/build failure handling under the stop-conditions model.
