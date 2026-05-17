# `repair-loop-exhausted`

Fixture proving the bounded-retry exhaustion contract: the LLM cannot fix the candidate within the v1 retry budget (3 retries per source), so the skill exits `surveyor-exhausted` and persists the last failing candidate + validator output for the operator. No `survey.md` and no `discovery.md` are written — this is the fail-closed rule from RFC-20 §"Guardrails".

## RFC behaviour proved

- RFC-20 §"Determinism Policy" "Bounded repair loop": "On exhaustion the skill exits with `surveyor-exhausted` and emits the last failing candidate alongside the validator output, so the operator can edit it by hand or re-run with a tighter brief."
- [`references/repair-loop.md`](../../references/repair-loop.md) §"Retry budget": v1 budget is 3 retries per source.
- [`references/repair-loop.md`](../../references/repair-loop.md) §"Exhaustion contract": skill exits non-zero, prints the last failing candidate and validator output, and persists both under `.specify/plans/<change>/survey/staged/<source-key>.last-failure.json`.
- RFC-20 §"Guardrails" "Fail-closed on unknowns": no `survey.md` and no `discovery.md` candidate blocks are written when any source ends in `surveyor-exhausted`.

## Expected sequence

1. Skill writes [`inputs/staged/legacy-flaky.json`](inputs/staged/legacy-flaky.json) (the INITIAL candidate). `touches[1]` resolves outside the source root.
2. Skill invokes `specify change survey --validate-only` — exits non-zero with `surfaces-touches-out-of-tree`.
3. Retry 1 — skill re-prompts the LLM; the LLM produces [`inputs/staged/legacy-flaky.retry-1.json`](inputs/staged/legacy-flaky.retry-1.json) (two surfaces share `id: dup-id`). CLI exits non-zero with `surfaces-id-collision`.
4. Retry 2 — skill re-prompts; the LLM produces [`inputs/staged/legacy-flaky.retry-2.json`](inputs/staged/legacy-flaky.retry-2.json) (surface `kind: frobnitz` is outside the closed enum). CLI exits non-zero with `surfaces-validation-failed`.
5. Retry 3 — skill re-prompts; the LLM produces [`inputs/staged/legacy-flaky.retry-3.json`](inputs/staged/legacy-flaky.retry-3.json) (`touches[1]: src/does-not-exist.ts` is not on disk). CLI exits non-zero with `surfaces-touches-out-of-tree`.
6. Budget exhausted (3 of 3). Skill emits the discriminant `surveyor-exhausted`, prints the last failing candidate and validator output, and persists [`expected/staged/legacy-flaky.last-failure.json`](expected/staged/legacy-flaky.last-failure.json) under `.specify/plans/<change>/survey/staged/`.
7. Skill exits non-zero with [`expected/exit.json`](expected/exit.json) shape. Neither `survey.md` nor `discovery.md` is appended to or written.

The retry candidates deliberately fail under three different discriminants (`surfaces-id-collision`, `surfaces-validation-failed`, `surfaces-touches-out-of-tree`) to make clear that any combination of repair-eligible failures counts against the same budget.

## Contents

- [`inputs/sources.yaml`](inputs/sources.yaml) — batch sources file with one entry, pointing at the in-fixture stub source tree.
- [`inputs/legacy-flaky-source/`](inputs/legacy-flaky-source) — minimal TypeScript stub tree (`src/handler.ts` 100 LOC + empty `src/server.ts` anchor).
- [`inputs/discovery.md`](inputs/discovery.md) — pre-survey discovery with `## Candidate inventory` heading.
- [`inputs/staged/legacy-flaky.json`](inputs/staged/legacy-flaky.json) — INITIAL staged candidate; fails `surfaces-touches-out-of-tree`.
- [`inputs/staged/legacy-flaky.retry-1.json`](inputs/staged/legacy-flaky.retry-1.json) — retry 1; fails `surfaces-id-collision`.
- [`inputs/staged/legacy-flaky.retry-2.json`](inputs/staged/legacy-flaky.retry-2.json) — retry 2; fails `surfaces-validation-failed` (unknown `kind`).
- [`inputs/staged/legacy-flaky.retry-3.json`](inputs/staged/legacy-flaky.retry-3.json) — retry 3; fails `surfaces-touches-out-of-tree` (file not on disk).
- [`expected/staged/legacy-flaky.last-failure.json`](expected/staged/legacy-flaky.last-failure.json) — last failing candidate plus validator output, persisted by the skill on `surveyor-exhausted`.
- [`expected/exit.json`](expected/exit.json) — operator-visible exit envelope for the `surveyor-exhausted` discriminant.

## What is absent

- No `expected/survey.md` — the fail-closed rule forbids rendering `survey.md` when any source ends in `surveyor-exhausted`.
- No `expected/discovery.md` — same rule; the skill must not append candidate blocks to `discovery.md` when the run failed.
- No `expected/survey/<source-key>/` directory — the CLI never gets past `--validate-only` for this source, so no canonical sidecars are written.
