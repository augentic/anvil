# `single-source-small`

Fixture proving the single-source already-S no-op: when a source's union-of-`touches` LOC is `acceptable` (< 1000), the source is emitted as a single terminal candidate covering every surface, with no surface-level decomposition.

## RFC behaviour proved

- RFC-20 §"Step 3" Decision 1: "If the source as a whole is `acceptable` (< 1000), emit it as a single terminal candidate covering every surface and stop."
- The candidate's `touches` is the deduplicated union of all surface `touches`.
- The candidate's `handler` is omitted because multiple handlers apply.
- The candidate name is the source-key.

## Sizing assumptions

Per-file production LOC, baked into the stub source tree under `inputs/legacy-widget-source/`:

| File | Production LOC |
|---|---|
| `src/handlers/get.ts` | 155 |
| `src/handlers/list.ts` | 150 |
| `src/handlers/create.ts` | 200 |
| `src/services/widget-service.ts` | 200 |
| `src/services/validate.ts` | 145 |
| `src/server.ts` | 0 (anchor for `declared-at` references) |

Total source LOC: 850. Union-of-touches LOC: 850 (< 1000 → Decision 1 fires).

## Candidates

| Name | Bucket | LOC | Surfaces |
|---|---|---|---|
| `legacy-widget` | acceptable | 850 | `http-get-widgets`, `http-get-widgets-id`, `http-post-widgets` |

## Contents

- [`inputs/sources.yaml`](inputs/sources.yaml) — batch sources file with one entry, pointing at the in-fixture stub source tree.
- [`inputs/legacy-widget-source/`](inputs/legacy-widget-source) — minimal TypeScript stub tree padded to the per-file LOC table above; satisfies the CLI's `path-under-root` check for every `touches[]` and `declared-at[]` entry.
- [`inputs/staged/legacy-widget.json`](inputs/staged/legacy-widget.json) — LLM-produced candidate `surfaces.json` (the input to `specify change survey`).
- [`inputs/discovery.md`](inputs/discovery.md) — pre-survey discovery with `## Candidate inventory` heading.
- [`expected/survey/legacy-widget/surfaces.json`](expected/survey/legacy-widget/surfaces.json) — canonical sidecar written by the CLI.
- [`expected/survey/legacy-widget/metadata.json`](expected/survey/legacy-widget/metadata.json) — canonical sidecar capturing LOC, module count, and top-level modules.
- [`expected/survey.md`](expected/survey.md) — byte-stable survey output with one source-level candidate.
- [`expected/discovery.md`](expected/discovery.md) — discovery after survey appends the single candidate block.
