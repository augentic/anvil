# Proposal — platform-v2

## Slices

| # | Slice | Source(s) | Depends on | Decision | Plan entry |
|---|---|---|---|---|---|
| 1 | ingest-pipeline | monolith | — | accepted | ingest-pipeline |
| 2 | operator-dashboard | — | ingest-pipeline | accepted | operator-dashboard |
| 3 | shared-types | — | — | accepted | shared-types |

## Assignment

| # | Entry | Project | Rationale |
|---|---|---|---|
| 1 | ingest-pipeline | traffic | description overlap: ingestion, Kafka consumers |
| 2 | operator-dashboard | command-centre | baseline spec: user-alerts exists in command-centre |
| 3 | shared-types | traffic | operator override (originally unresolved: matched both projects) |
