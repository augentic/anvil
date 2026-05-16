# `survey-end-to-end`

Pipeline-spanning fixture exercising the full `/change:draft` → discovery brief → `/change:survey` handshake on a single `legacy-code` Express source. Asserts `## Candidate inventory` is emitted exactly once in the final `discovery.md`.

## RFC behaviour proved

- RFC-20 §"Step 2": the discovery brief writes the `## Candidate inventory` heading wrapper into `discovery.md` exactly once, before either analyze or survey runs.
- RFC-20 §"Step 3–6": `/change:survey` invokes the CLI, composes the inventory, and appends candidate blocks under the heading.
- RFC-20 §"Step 6": the heading appears exactly once — `grep -c '^## Candidate inventory' expected/discovery.md` returns 1.
- RFC-20 §"Step 3" Decision 1: source LOC < 1000 → one source-level candidate covering every surface.

## Cross-reference to CLI fixture

The source tree under `inputs/source/` is mirrored from [`specify-cli/crates/domain/tests/fixtures/detectors/express/synthetic-app/`](https://github.com/augentic/specify-cli/tree/main/crates/domain/tests/fixtures/detectors/express/synthetic-app). The expected `surfaces.json` matches the shape and conventions of that detector's golden, with `source-key` set to `express-app` for the plan context. This ensures the fixture exercises the same detector output the CLI produces for Express sources.

## Input shape

- `inputs/change.md` — minimal change recording one `legacy-code` source.
- `inputs/source/` — the Express synthetic app (4 TypeScript files, ~31 production LOC).
- `inputs/discovery.md` — discovery state after the discovery brief writes the `## Candidate inventory` heading but before survey runs.

## Expected output shape

- `expected/discovery.md` — final discovery with exactly one `## Candidate inventory` heading and one survey-derived candidate block.
- `expected/survey.md` — full survey output with Summary, Source inventory, and Candidate inventory sections.
- `expected/survey/express-app/surfaces.json` — CLI-produced surfaces matching the Express detector golden.
- `expected/survey/express-app/metadata.json` — CLI-produced source metadata.

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
| [`inputs/source/`](inputs/source/) | Express app mirrored from CLI detector fixture. |
| [`expected/discovery.md`](expected/discovery.md) | Final discovery — heading appears exactly once. |
| [`expected/survey.md`](expected/survey.md) | Full survey output. |
| [`expected/survey/express-app/surfaces.json`](expected/survey/express-app/surfaces.json) | CLI detector output. |
| [`expected/survey/express-app/metadata.json`](expected/survey/express-app/metadata.json) | CLI source metadata. |
