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

`legacy-api` per-file production LOC, baked into the stub source tree under `inputs/legacy-api-source/`:

| File | Production LOC |
|---|---|
| `src/users/list.ts` | 180 |
| `src/users/repository.ts` | 360 |
| `src/orders/create.ts` | 200 |
| `src/orders/repository.ts` | 280 |
| `src/orders/validate.ts` | 180 |
| `src/server.ts` | 0 (anchor for `declared-at` references) |

`legacy-billing` per-file production LOC, baked into the stub source tree under `inputs/legacy-billing-source/`:

| File | Production LOC |
|---|---|
| `src/invoices/list.ts` | 180 |
| `src/invoices/repository.ts` | 220 |
| `src/payments/create.ts` | 180 |
| `src/payments/repository.ts` | 200 |
| `src/server.ts` | 0 (anchor for `declared-at` references) |

`legacy-billing` total union-of-touches: 780 LOC.

## Candidates

| Name | Source | Bucket | LOC |
|---|---|---|---|
| `user-list` | legacy-api | acceptable | 540 |
| `order-creation` | legacy-api | acceptable | 660 |
| `legacy-billing` | legacy-billing | acceptable | 780 |

Ordering: alphabetical by source-key, then by first surface id within each source.

## Contents

- [`inputs/sources.yaml`](inputs/sources.yaml) — batch sources file with two entries, each pointing at an in-fixture stub source tree.
- [`inputs/legacy-api-source/`](inputs/legacy-api-source) — minimal TypeScript stub tree for `legacy-api`, padded to the per-file LOC table above.
- [`inputs/legacy-billing-source/`](inputs/legacy-billing-source) — minimal TypeScript stub tree for `legacy-billing`, padded to the per-file LOC table above.
- [`inputs/staged/legacy-api.json`](inputs/staged/legacy-api.json) — LLM-produced candidate `surfaces.json` for `legacy-api`.
- [`inputs/staged/legacy-billing.json`](inputs/staged/legacy-billing.json) — LLM-produced candidate `surfaces.json` for `legacy-billing`.
- [`inputs/discovery.md`](inputs/discovery.md) — pre-survey discovery with `## Candidate inventory` heading.
- [`expected/survey/legacy-api/surfaces.json`](expected/survey/legacy-api/surfaces.json) — canonical sidecar written by the CLI for `legacy-api`.
- [`expected/survey/legacy-api/metadata.json`](expected/survey/legacy-api/metadata.json) — canonical metadata for `legacy-api`.
- [`expected/survey/legacy-billing/surfaces.json`](expected/survey/legacy-billing/surfaces.json) — canonical sidecar written by the CLI for `legacy-billing`.
- [`expected/survey/legacy-billing/metadata.json`](expected/survey/legacy-billing/metadata.json) — canonical metadata for `legacy-billing`.
- [`expected/survey.md`](expected/survey.md) — byte-stable survey output with namespaced surface ids.
- [`expected/discovery.md`](expected/discovery.md) — discovery after survey appends candidate blocks from both sources.
