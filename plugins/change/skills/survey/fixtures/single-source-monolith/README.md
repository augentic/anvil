# `single-source-monolith`

Fixture proving the core happy path: a single `legacy-code` source whose total LOC is `too-large` (≥ 1000), producing surface-sized candidates with one minimal same-source cluster.

## RFC behaviour proved

- RFC-20 §"Step 3" Decision 1 fails (union-of-`touches` LOC ≥ 1000), so the algorithm descends to surface-level candidates.
- RFC-20 §"Step 4" minimal clustering: two surfaces (`http-get-users-id` and `http-post-users`) share ≥ 50% `touches` overlap and their combined LOC remains `acceptable` (< 1000). They are merged into one candidate.
- A third surface (`http-post-orders`) has no overlap with either and remains standalone.

## Sizing assumptions

Per-file production LOC used to compute candidate sizes (no actual source tree ships with this fixture):

| File | Production LOC |
|---|---|
| `src/users/get.ts` | 140 |
| `src/users/register.ts` | 200 |
| `src/users/repository.ts` | 220 |
| `src/users/validate.ts` | 140 |
| `src/orders/create.ts` | 200 |
| `src/orders/pricing.ts` | 130 |
| `src/orders/repository.ts` | 170 |
| `src/orders/validate.ts` | 120 |

Union-of-touches LOC: 1320 (≥ 1000 → surface candidates).

## Clustering walkthrough

- `http-get-users-id` touches: {`get.ts`, `repository.ts`} (2 files).
- `http-post-users` touches: {`register.ts`, `repository.ts`, `validate.ts`} (3 files).
- Intersection: {`repository.ts`} = 1 file. Smaller set: 2. Overlap: 1 / 2 = 50% → merge signal fires.
- Combined touches: {`get.ts`, `register.ts`, `repository.ts`, `validate.ts`} = 700 LOC < 1000 → merge.
- `http-post-orders` has zero overlap with either → standalone.

## Candidates

| Name | Bucket | LOC | Surfaces |
|---|---|---|---|
| `user-management` | acceptable | 700 | `http-get-users-id`, `http-post-users` |
| `order-creation` | acceptable | 620 | `http-post-orders` |

## Contents

- [`inputs/sources.yaml`](inputs/sources.yaml) — batch sources file with one entry.
- [`inputs/surfaces.json`](inputs/surfaces.json) — three surfaces for `legacy-monolith`.
- [`inputs/metadata.json`](inputs/metadata.json) — source metadata (2400 total LOC).
- [`inputs/discovery.md`](inputs/discovery.md) — pre-survey discovery with `## Candidate inventory` heading.
- [`expected/survey.md`](expected/survey.md) — byte-stable survey output.
- [`expected/discovery.md`](expected/discovery.md) — discovery after survey appends candidate blocks.
