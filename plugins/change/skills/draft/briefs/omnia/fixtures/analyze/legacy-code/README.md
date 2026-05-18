# `plan/analyze/legacy-code/` fixture

Worked example for the legacy-code branch of [Omnia `analyze.md`](../../../analyze.md).

## Contents

- [`inputs/monolith/`](inputs/monolith/) — a tiny TypeScript monolith with four inferable adapters across `src/users`, `src/auth`, `src/common`, and `src/billing`.
- [`expected/discovery.md`](expected/discovery.md) — the byte-stable `$DISCOVERY` the brief is expected to produce, with four adapter summaries sorted alphabetically: `billing-subscription`, `email-verification`, `shared-validation`, `user-registration`.
- [`expected/plans/legacy-code/analyze/monolith/metadata.json`](expected/plans/legacy-code/analyze/monolith/metadata.json) — the structural-metadata sidecar the code branch writes alongside `$DISCOVERY`.
- [`notes.md`](notes.md) — idempotency acceptance notes.

## Invocation

Run from this directory:

```
/change:analyze legacy-code monolith ./inputs/monolith/ ./expected/plans/legacy-code/
```

The brief walks `./inputs/monolith/`, clusters its source tree into four adapters, emits each as a `<!-- source-key: monolith -->`- tagged `### <name>` block into `./expected/plans/legacy-code/discovery.md`, and writes the structural sidecar to `./expected/plans/legacy-code/analyze/monolith/metadata.json`.

## What this fixture pins

- On-disk shape of adapter summaries matches the [`analyze/SKILL.md` §Output contract](../../../../../../analyze/SKILL.md): `### <name>` heading plus fenced YAML block, fixed field order (`summary`, `sources`, `depends-on`, `hints`, `confidence`).
- Adapters sort alphabetically by name. `sources`, `depends-on`, `hints.entry_points`, `hints.external_deps` each sort alphabetically within their block.
- The `user-registration` entry reproduces the canonical sample from [`rfc-3a-monoliths.md` §*Plan-time analysis, define-time extraction*](../../../../../../../../../rfcs/archive/rfc-3a-monoliths.md) in the on-disk shape — same summary, same source set, same `depends-on`, same hints, same `confidence: high`. (The RFC snippet is in `adapters: - name: …` YAML-list form; the fixture renders it as `### user-registration` + fenced YAML per the SKILL's output contract, with `sources` in canonical alphabetical order.)
- `<!-- source-key: monolith -->` marker precedes every `### <name>` heading — this invocation passes `--source-key monolith` so the skill tags each adapter.
- The `shared-validation` block omits the `hints:` map entirely (no entry points, no external deps) — a legal shape per the output contract.
- `metadata.json` matches the v1 shape pinned by [`analyze/SKILL.md` §Structural metadata](../../../../../../analyze/SKILL.md): six required fields in fixed order, `top_level_modules` alphabetically sorted, no timestamps.
- No `## Constraints` or `## Open questions` appendix blocks — the legacy-code branch does not emit them (documentation-only, see §*Documentation branch* of the brief).

## Not covered here

- Multiple sources in one run — the discovery brief would invoke `/change:analyze` once per source key and union the results.
- A `confidence: low` adapter — this fixture stays at `high` or `medium`.
- Out-of-scope / vendored code handling — the propose brief eventually surfaces these via `scope.<k>.exclude`.
- A non-TypeScript source tree — language-specific `module_count` conventions are pinned in the brief but not exercised by this fixture.
