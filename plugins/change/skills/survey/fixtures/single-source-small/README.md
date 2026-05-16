# `single-source-small`

Fixture proving the single-source already-S no-op: when a source's union-of-`touches` LOC is `acceptable` (< 1000), the source is emitted as a single terminal candidate covering every surface, with no surface-level decomposition.

## RFC behaviour proved

- RFC-20 §"Step 3" Decision 1: "If the source as a whole is `acceptable` (< 1000), emit it as a single terminal candidate covering every surface and stop."
- The candidate's `touches` is the deduplicated union of all surface `touches`.
- The candidate's `handler` is omitted because multiple handlers apply.
- The candidate name is the source-key.

## Input shape

Source `legacy-widget` with 3 HTTP route surfaces totalling 850 production LOC across 5 unique touched files. All under the 1000 LOC threshold.

## Candidates

| Name | Bucket | LOC | Surfaces |
|---|---|---|---|
| `legacy-widget` | acceptable | 850 | `http-get-widgets`, `http-get-widgets-id`, `http-post-widgets` |

## Contents

- [`inputs/sources.yaml`](inputs/sources.yaml) — batch sources file with one entry.
- [`inputs/surfaces.json`](inputs/surfaces.json) — three surfaces for `legacy-widget`.
- [`inputs/metadata.json`](inputs/metadata.json) — source metadata (850 LOC).
- [`inputs/discovery.md`](inputs/discovery.md) — pre-survey discovery with `## Candidate inventory` heading.
- [`expected/survey.md`](expected/survey.md) — byte-stable survey output with one source-level candidate.
- [`expected/discovery.md`](expected/discovery.md) — discovery after survey appends the single candidate block.
