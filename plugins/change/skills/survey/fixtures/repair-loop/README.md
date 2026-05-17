# `repair-loop`

Fixture proving the bounded-retry success path: the LLM's first candidate fails the CLI's `surfaces-touches-out-of-tree` invariant, the skill re-prompts with the structured validator output, and the second candidate validates cleanly. The skill writes the canonical sidecars from the repaired candidate, renders `survey.md`, and appends to `discovery.md` — exactly as if the first attempt had succeeded.

## RFC behaviour proved

- RFC-20 §"Determinism Policy" "Bounded repair loop": "When the candidate `surfaces.json` fails validation, the skill re-prompts the LLM with the structured error up to a small bounded retry count (v1: three retries). On exhaustion the skill exits with `surveyor-exhausted`."
- [`references/repair-loop.md`](../../references/repair-loop.md) §"Retry budget": each retry runs the CLI in `--validate-only` form; on success the skill re-invokes once without `--validate-only` to perform the canonical write.
- [`references/repair-loop.md`](../../references/repair-loop.md) §"Structured feedback grammar": the skill feeds the `Error::Diag` `code` + `detail` back to the LLM verbatim with a fixed "fix only the cited rule" instruction.

## Expected sequence

1. Skill writes `inputs/staged/legacy-svc.json` (the INITIAL candidate, with a `..` segment in `touches[1]`).
2. Skill invokes `specify change survey --sources … --staged inputs/staged --out … --validate-only`.
3. CLI exits non-zero with `Error::Diag { code: "surfaces-touches-out-of-tree", detail: "surfaces[0].touches[1]: ../escaped/path.ts (resolves outside source root)" }`.
4. Skill enters the repair loop (attempt 1 of 3): re-prompts the LLM with the candidate, the brief, and the structured failure envelope from [`references/repair-loop.md`](../../references/repair-loop.md).
5. LLM produces `inputs/staged/legacy-svc.repaired.json` (the corrected candidate; the skill overwrites `legacy-svc.json` with this content in production).
6. Skill invokes the CLI with `--validate-only` again; validation passes.
7. Skill re-invokes the CLI without `--validate-only` to perform the canonical write.
8. CLI writes `expected/survey/legacy-svc/{surfaces.json, metadata.json}`.
9. Skill renders `expected/survey.md` and appends the candidate block to `expected/discovery.md`.
10. Skill persists [`expected/repair-log.json`](expected/repair-log.json) as the operator-visible trace; no `<source-key>.last-failure.json` is written (that artifact is only persisted on `surveyor-exhausted`).

## Contents

- [`inputs/sources.yaml`](inputs/sources.yaml) — batch sources file with one entry, pointing at the in-fixture stub source tree.
- [`inputs/legacy-svc-source/`](inputs/legacy-svc-source) — minimal TypeScript stub tree (`src/handler.ts` 200 LOC + an empty `src/server.ts` anchor for the `declared-at` reference).
- [`inputs/discovery.md`](inputs/discovery.md) — pre-survey discovery with `## Candidate inventory` heading.
- [`inputs/staged/legacy-svc.json`](inputs/staged/legacy-svc.json) — the INITIAL (broken) staged candidate; `touches[1]` resolves outside the source root.
- [`inputs/staged/legacy-svc.repaired.json`](inputs/staged/legacy-svc.repaired.json) — the post-retry valid candidate the LLM produces after the validator feedback.
- [`expected/survey/legacy-svc/surfaces.json`](expected/survey/legacy-svc/surfaces.json) — canonical sidecar written by the CLI after ingesting the repaired candidate.
- [`expected/survey/legacy-svc/metadata.json`](expected/survey/legacy-svc/metadata.json) — canonical metadata.
- [`expected/survey.md`](expected/survey.md) — final survey output (looks identical to a fixture where the first attempt succeeded).
- [`expected/discovery.md`](expected/discovery.md) — discovery after survey appends the candidate block.
- [`expected/repair-log.json`](expected/repair-log.json) — operator-visible trace of the repair attempts: initial failure, retry 1 status, candidate paths.
