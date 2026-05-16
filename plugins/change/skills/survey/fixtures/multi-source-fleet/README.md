# `multi-source-fleet`

Fixture proving multi-source handling: a change with two `legacy-code` source-keys, producing one combined inventory with separate source-local candidates. No cross-source pairing in v1.

## RFC behaviour proved

- RFC-20 §"Single-Source And Multi-Source Behavior": the algorithm is identical for each source; the source count changes the breadth of the inventory, not the planning model.
- RFC-20 §"Step 4": surface ids in `surfaces[]` are always namespaced `<source-key>:<surface-id>` so the same identifier from two repos remains distinguishable.
- Candidates from different sources remain separate; survey does not merge cross-source.

## Sources

| Source | Language | LOC | Surfaces | Decision |
|---|---|---|---|---|
| `legacy-api` | typescript | 1200 | 2 | Union LOC ≥ 1000 → surface candidates (no overlap → no clustering) |
| `legacy-billing` | typescript | 780 | 2 | Union LOC < 1000 → one source-level candidate |

## Sizing assumptions

`legacy-api` per-file LOC:

| File | LOC |
|---|---|
| `src/users/list.ts` | 180 |
| `src/users/repository.ts` | 360 |
| `src/orders/create.ts` | 200 |
| `src/orders/repository.ts` | 280 |
| `src/orders/validate.ts` | 180 |

`legacy-billing` total union-of-touches: 780 LOC.

## Candidates

| Name | Source | Bucket | LOC |
|---|---|---|---|
| `user-list` | legacy-api | acceptable | 540 |
| `order-creation` | legacy-api | acceptable | 660 |
| `legacy-billing` | legacy-billing | acceptable | 780 |

Ordering: alphabetical by source-key, then by first surface id within each source.

## Contents

- [`inputs/sources.yaml`](inputs/sources.yaml) — batch sources file with two entries.
- [`inputs/legacy-api/surfaces.json`](inputs/legacy-api/surfaces.json) — two surfaces for `legacy-api`.
- [`inputs/legacy-api/metadata.json`](inputs/legacy-api/metadata.json) — metadata for `legacy-api`.
- [`inputs/legacy-billing/surfaces.json`](inputs/legacy-billing/surfaces.json) — two surfaces for `legacy-billing`.
- [`inputs/legacy-billing/metadata.json`](inputs/legacy-billing/metadata.json) — metadata for `legacy-billing`.
- [`inputs/discovery.md`](inputs/discovery.md) — pre-survey discovery with `## Candidate inventory` heading.
- [`expected/survey.md`](expected/survey.md) — byte-stable survey output with namespaced surface ids.
- [`expected/discovery.md`](expected/discovery.md) — discovery after survey appends candidate blocks from both sources.
