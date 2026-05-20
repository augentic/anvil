# `plan/analyze/documentation/` fixture

Worked example for the documentation branch of [Omnia `analyze.md`](../../../analyze.md).

## Contents

- [`inputs/ops-runbook.md`](inputs/ops-runbook.md) — a tiny traffic-ingest operations runbook with two procedures and one deferred decision.
- [`expected/discovery.md`](expected/discovery.md) — the byte-stable `$DISCOVERY` the brief is expected to produce for the invocation below.

## Invocation

Run from this directory:

```
/change:analyze documentation ./inputs/ ./expected/
```

The brief walks `./inputs/`, extracts two adapters (`drain-backpressure-queue`, `rotate-upstream-ingest-key` — alphabetical), emits constraints and open questions into the two appendix blocks, and writes the merged result to `./expected/discovery.md`.

## What this fixture pins

- On-disk shape of adapter summaries matches the `analyze/SKILL.md` §Output contract (see the brief for the full link) — `### <name>` heading plus fenced YAML block, fixed field order.
- Adapters sort alphabetically by name; `sources`, `depends-on`, `hints.entry_points`, `hints.external_deps` each sort alphabetically within their block.
- `confidence: high` on both adapters — the runbook specifies each procedure's boundary, entry point, and external deps concretely. A less specific runbook would drop to `medium` or `low`.
- `## Constraints (from documentation)` and `## Open questions (from documentation)` appendices each cite their source artifact path (with heading fragment) so a reviewer can audit the extraction.
- No `<!-- source-key: ... -->` markers — this invocation omits `--source-key`; the skill would inject them before each `###` heading when the flag is supplied.

## Not covered here

- Multi-file input (directory with several artifacts) — the same contract applies; sources would carry distinct paths.
- OpenAPI inputs — deep-link via JSON pointer (e.g. `api-spec.yaml#/paths/~1users/post`). Not exercised here.
- `--source-key` tagging — exercised by the scaffold fixture under `plugins/change/skills/analyze/fixtures/scaffold-example/`.
- Code-branch output — lands with RFC-3a C21.
