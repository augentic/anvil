### drain-backpressure-queue

```yaml
summary: Drain the backpressure queue when ingest depth stays above 80% of capacity.
sources:
  - ops-runbook.md#drain-the-backpressure-queue
depends-on: []
hints:
  entry_points: [drain-backpressure]
  external_deps: [pagerduty, redis]
confidence: high
```

### rotate-upstream-ingest-key

```yaml
summary: Rotate the upstream ingest key when the secret provider rolls it.
sources:
  - ops-runbook.md#rotate-the-upstream-ingest-key
depends-on: []
hints:
  entry_points: [rotate-ingest-key]
  external_deps: [azure-key-vault, kafka]
confidence: high
```

## Constraints (from documentation)

- Backpressure drain MUST re-publish from Redis to Kafka at a controlled rate — no bulk dump (source: `ops-runbook.md#drain-the-backpressure-queue`).
- Post-rotation, `kafka_consumer_lag` MUST return to baseline before the rotation is considered complete (source: `ops-runbook.md#rotate-the-upstream-ingest-key`).
- `rotate-upstream-ingest-key` and `drain-backpressure-queue` MUST NOT run concurrently (source: `ops-runbook.md#drain-the-backpressure-queue`).

## Open questions (from documentation)

- Is the 90-day rotation window still appropriate, or should security tighten it to 60 days? (source: `ops-runbook.md#rotate-the-upstream-ingest-key`)
