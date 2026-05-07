# Mixed-input discovery fixture

Pins the combined `discovery.md` shape produced by the Omnia discovery brief when a single `/change:plan` invocation receives both a `documentation` input and a `legacy-code` input. Both kinds dispatch to [`/spec:analyze`](../../../../../../spec/skills/analyze/SKILL.md) (RFC-3a C19 + C23); the two emitted blocks share the fenced-YAML capability-summary shape and collate alphabetically under `## Capability inventory`.

| Path                                                              | Role                                                                                                  |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| [`invocation.txt`](invocation.txt)                                | Operator invocation exercised by this fixture.                                                        |
| [`inputs/ops-runbook.md`](inputs/ops-runbook.md)                  | Documentation input. Compact two-procedure runbook; dispatches to `/spec:analyze --kind documentation`. |
| [`inputs/legacy-service/`](inputs/legacy-service/)                | Legacy-code input. Single-file stub; dispatches to `/spec:analyze --kind legacy-code`.                |
| [`expected/discovery.md`](expected/discovery.md)                  | Byte-stable combined output. Four capability summaries (YAML), alphabetically sorted, plus the documentation appendix blocks. |
| [`expected/plans/traffic/analyze/legacy/metadata.json`](expected/plans/traffic/analyze/legacy/metadata.json) | Structural-metadata sidecar written by the legacy-code branch of `/spec:analyze` for the `legacy` source. |
| [`notes.md`](notes.md)                                            | Capability-level notes and downstream-consumer pointers.                                              |

Read [`notes.md`](notes.md) before extending the fixture — per- capability signal choices (`ingest-replay` → `ingest-submit` dependency, entry-point strings) are deliberate.
