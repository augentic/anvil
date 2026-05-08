# Mixed-input discovery notes

## What this fixture pins

The invocation in [`invocation.txt`](invocation.txt) supplies one `documentation` input (`ops-runbook.md`, defaulting to `kind: documentation` via `--from`) and one `legacy-code` input (`--source legacy=./inputs/legacy-service`, defaulting to `kind: legacy-code`). Both kinds dispatch to [`/spec:analyze`](../../../../../../spec/skills/analyze/SKILL.md) per the [discovery brief](../../../briefs/omnia/discovery.md), one invocation per input:

| Kind            | Skill invocation                                               | Output shape                                                      |
| --------------- | -------------------------------------------------------------- | ----------------------------------------------------------------- |
| `documentation` | `/spec:analyze <input> <plan-dir> documentation ...`    | `### <name>` + fenced YAML capability summary, plus appendix blocks. |
| `legacy-code`   | `/spec:analyze <input> <plan-dir> legacy-code ...`      | `### <name>` + fenced YAML capability summary, plus `metadata.json`. |

Both branches emit the same capability-summary shape, so `expected/discovery.md` carries a single inventory block sorted alphabetically across both kinds. Documentation inputs additionally contribute the `## Constraints (from documentation)` and `## Open questions (from documentation)` appendix blocks; legacy-code inputs additionally write `expected/plans/traffic/analyze/legacy/metadata.json`.

## Capabilities pinned

Four capabilities, alphabetical order:

1. **`drain-backpressure-queue`** (`source-key: ops-runbook`, `confidence: high`) — from the runbook's "Drain the backpressure queue" procedure.
2. **`ingest-replay`** (`source-key: legacy`, `confidence: high`) — the `replay` handler in `src/ingest.rs`. Depends on `ingest-submit` (shares the primary topic).
3. **`ingest-submit`** (`source-key: legacy`, `confidence: high`) — the `submit` handler in `src/ingest.rs`.
4. **`rotate-upstream-ingest-key`** (`source-key: ops-runbook`, `confidence: high`) — from the runbook's "Rotate the upstream ingest key" procedure.

## Downstream consumers

The propose brief (RFC-3a C24) consumes this fixture's `expected/discovery.md` as a multi-source mixed-kinds regression example: one plan entry per capability, `sources:` tagged by the capability's `<!-- source-key: <k> -->` marker, `scope.<k>.include` pre-filled from legacy-code capabilities' `sources:` lists.

## Fixture scope

The code under [`inputs/legacy-service/`](inputs/legacy-service/) is a deliberate two-handler stub, not a realistic monolith — the fixture's job is to pin the multi-source `discovery.md` shape, not to exercise `/spec:analyze`'s clustering heuristics on a large tree. The dedicated monolith fixture at [`../monolith/`](../monolith/) pins the single-source three-capability legacy-code path on a purpose-built tree.
