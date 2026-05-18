# Discovery — platform-modernisation

## Adapter inventory

<!-- source-key: monolith -->
### ingest-pipeline-refactor

```yaml
summary: |
  Drain the ingest queue under back-pressure rather than dropping events
  on the floor when the downstream pipeline saturates.
sources:
  - src/ingest/queue.ts
  - src/ingest/handlers.ts
depends-on: []
hints:
  baseline-overlap: ingest-pipeline (existing baseline spec)
confidence: high
```

<!-- source-key: monolith -->
### operator-dashboard-alerts

```yaml
summary: |
  Surface ingest back-pressure events on the operator dashboard so that
  the on-call sees saturation before the pager wakes them up.
sources:
  - src/dashboard/feed.ts
  - src/dashboard/alerts.ts
depends-on:
  - ingest-pipeline-refactor
hints:
  baseline-overlap: user-alerts (existing baseline spec on command-centre)
confidence: high
```

<!-- source-key: monolith -->
### alpha-gateway-extract

```yaml
summary: |
  Carve the inbound HTTP edge layer (rate limiting, auth challenge,
  request normalisation) out of the monolith into a standalone gateway
  service. The edge layer has no shared mutable state with the rest of
  the monolith and is one of the hottest paths under load.
sources:
  - src/edge/gateway.ts
  - src/edge/ratelimit.ts
  - src/edge/normalise.ts
depends-on: []
hints:
  baseline-overlap: none — adapter is greenfield in spec terms
  schema-tier: backend service (no shell-side adapter)
confidence: medium
```

## Open questions

- The ratelimit configuration is currently fed from a Redis instance the
  monolith owns; the gateway extraction will need to keep that
  behaviour but carve out the secret rotation flow separately. Tracked
  as a follow-up open question; not a blocker for plan authoring.
