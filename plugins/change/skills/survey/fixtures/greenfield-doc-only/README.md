# `greenfield-doc-only`

Fixture proving the greenfield documentation-only pass-through: when a change has no `legacy-code` sources, `/change:survey` is skipped entirely. No `survey.md` is written and no survey-derived candidate blocks are appended to `discovery.md`.

## RFC behaviour proved

- RFC-20 §"Migration": "Documentation-only changes skip `/change:survey` entirely. With no `legacy-code` source, the pipeline reaches `propose` directly from discovery — there is nothing to decompose and the survey gate adds ceremony without value."
- The `## Candidate inventory` heading is still present in `discovery.md` because the discovery brief writes it unconditionally. Documentation-derived candidate blocks from `/change:analyze` appear under it, but no survey-derived blocks are added.

## Contents

- [`inputs/discovery.md`](inputs/discovery.md) — discovery as written by the discovery brief and `/change:analyze` for a documentation-only change. Contains one doc-derived candidate block under the `## Candidate inventory` heading.
- [`expected/discovery.md`](expected/discovery.md) — identical to the input: no survey blocks appended because survey was skipped.

## What is absent

- No `survey.md` in `expected/` — survey does not run for documentation-only changes.
- No `surfaces.json` or `metadata.json` — no legacy-code sources to scan.
- No `sources.yaml` — no batch file is written.
