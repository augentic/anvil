# Traffic ingest — ops runbook

Operational procedures for the `traffic-ingest` service. Each section below describes one procedure. Run steps exactly as written; escalate to the on-call lead if any step fails.

## Rotate the upstream ingest key

Trigger: the upstream secret provider has rotated the ingest key (scheduled every 90 days, or ad-hoc during a security incident).

External systems:

- Azure Key Vault — source of truth for the new key.
- Kafka — the downstream consumer config needs the new key.

Steps:

1. Fetch the new key from Azure Key Vault: `az keyvault secret show --name ingest-key`.
2. Update the Kafka consumer config in the `traffic-ingest` Helm chart at `values.yaml:kafka.ingestKey`.
3. Restart the ingest workers: `kubectl rollout restart deploy/traffic-ingest`.
4. Watch `kafka_consumer_lag` for five minutes. Lag MUST return to baseline before the rotation is considered complete.

Entry point: command `rotate-ingest-key` (runbook script at `ops/scripts/rotate-ingest-key.sh`).

Open question: is the 90-day rotation window still appropriate, or should security tighten it to 60 days? Security reviews this quarterly; the runbook does not commit either way.

## Drain the backpressure queue

Trigger: `ingest_backpressure_depth` has exceeded 80% of capacity for more than ten minutes. Before starting, verify `rotate-upstream-ingest-key` is not currently running — the two procedures MUST NOT overlap.

External systems:

- Redis — holds the backpressure queue.
- PagerDuty — raises the incident when depth crosses 95%.

Steps:

1. Scale ingest workers to 3x normal: `kubectl scale deploy/traffic-ingest --replicas=9`.
2. Drain the queue with `ops/scripts/drain-backpressure.sh`. The script reads from Redis and re-publishes to the primary Kafka topic at a controlled rate — no bulk dump is permitted.
3. Once `ingest_backpressure_depth` drops below 20%, scale back to the normal 3 replicas.

Entry point: command `drain-backpressure` (wraps `ops/scripts/drain-backpressure.sh`).
