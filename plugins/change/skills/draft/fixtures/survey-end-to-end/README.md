# `survey-end-to-end`

Pipeline-spanning fixture exercising the full `/change:draft` → discovery brief → `/change:survey` handshake on a single `legacy-code` Express source. Asserts `## Candidate inventory` is emitted exactly once in the final `discovery.md`.

## RFC behaviour proved

- RFC-20 §"`/change:draft` Analysis Flow" — Step 2 ("Analyze Documentation Inputs"): the discovery brief writes the `## Candidate inventory` heading wrapper into `discovery.md` exactly once, before either analyze or survey runs.
- RFC-20 §"`/change:draft` Analysis Flow" — Step 3 ("Source Survey And Decomposition"): `/change:survey` drives the per-language enumeration brief, hands the candidate `surfaces.json` to `specify change survey` for validation and canonical write, reads back the canonicalized sidecars, runs the candidate algorithm, and appends candidate blocks under the discovery-owned heading.
- RFC-20 §"`/change:draft` Analysis Flow" — Step 6 ("Hand Candidates To Propose"): the heading appears exactly once — `grep -c '^## Candidate inventory' expected/discovery.md` returns 1.
- RFC-20 §"`/change:draft` Analysis Flow" — Step 3 Decision 1 (size check): source LOC < 1000 → one source-level candidate covering every surface.

## Producer model

The Express source tree under `inputs/source/` is a tiny synthetic app (4 TypeScript files, ~31 production LOC) whose surfaces an LLM would enumerate by following the TypeScript enumeration brief at [`plugins/change/skills/survey/briefs/enumerate/typescript.md`](../../../survey/briefs/enumerate/typescript.md). The agent-produced candidate that brief yields is staged at `inputs/staged/express-app.json`; `specify change survey` then validates, canonicalises, captures metadata, and writes the canonical sidecars under `expected/survey/express-app/`. On a validator failure the skill enters the bounded repair loop documented in [`plugins/change/skills/survey/references/repair-loop.md`](../../../survey/references/repair-loop.md); the happy-path fixture exercises a candidate that validates on the first attempt.

## Input shape

- `inputs/change.md` — minimal change recording one `legacy-code` source.
- `inputs/source/` — the Express synthetic app (4 TypeScript files, ~31 production LOC).
- `inputs/discovery.md` — discovery state after the discovery brief writes the `## Candidate inventory` heading but before survey runs.
- `inputs/sources.yaml` — the `--sources` batch file passed to `specify change survey` (one row: `key: express-app`, `path: ./source`).
- `inputs/staged/express-app.json` — the agent-produced candidate the LLM emits when following the TypeScript enumeration brief; consumed by `specify change survey --staged inputs/staged`.

## Expected output shape

- `expected/discovery.md` — final discovery with exactly one `## Candidate inventory` heading and one survey-derived candidate block.
- `expected/survey.md` — full survey output with Summary, Source inventory, and Candidate inventory sections.
- `expected/survey/express-app/surfaces.json` — canonicalized surfaces written by `specify change survey`.
- `expected/survey/express-app/metadata.json` — source metadata captured by `specify change survey`.

## Heading assertion

```bash
grep -c '^## Candidate inventory' expected/discovery.md
# → 1
```

## Candidates

| Name | Bucket | LOC | Surfaces |
|---|---|---|---|
| `express-app` | acceptable | 31 | `http-get-health`, `http-get-users`, `http-post-users` |

## Contents

| Path | Role |
|---|---|
| [`inputs/change.md`](inputs/change.md) | Simulated change with one `legacy-code` source. |
| [`inputs/discovery.md`](inputs/discovery.md) | Discovery state pre-survey. |
| [`inputs/source/`](inputs/source/) | Synthetic Express app the agent enumerates against. |
| [`inputs/sources.yaml`](inputs/sources.yaml) | Batch `--sources` file passed to `specify change survey`. |
| [`inputs/staged/express-app.json`](inputs/staged/express-app.json) | Agent-produced candidate `surfaces.json` (pre-canonicalization). |
| [`expected/discovery.md`](expected/discovery.md) | Final discovery — heading appears exactly once. |
| [`expected/survey.md`](expected/survey.md) | Full survey output. |
| [`expected/survey/express-app/surfaces.json`](expected/survey/express-app/surfaces.json) | Canonical `surfaces.json` written by the CLI. |
| [`expected/survey/express-app/metadata.json`](expected/survey/express-app/metadata.json) | Source metadata written by the CLI. |
