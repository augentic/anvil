# `too-large-unresolved`

Fixture proving the `too-large` post-clustering `unresolved: true` path: a source where surfaces share a heavy common core, clustering cannot reduce the candidate below the 1000 LOC threshold, and the too-large leaf is emitted with `unresolved: true`.

## RFC behaviour proved

- RFC-20 §"Step 4": "`too-large` candidate that cannot be split further by the signals above → leaf is `unresolved: true`; the operator either edits the candidate during `propose` or rescopes the change."
- RFC-20 §"Step 3" Decision 4: "Any candidate whose LOC >= 1000 after clustering (or any surface candidate that was already `too-large` and could not be merged) is emitted with `unresolved: true`."
- Survey exits 0; `propose` is the gate.

## Input shape

Source `legacy-billing`, TypeScript, 1320 total LOC. Two surfaces sharing `src/billing/core.ts` (a 600 LOC common module):

| Surface | Touches | LOC |
|---|---|---|
| `message-sub-payment-settled` | `core.ts`, `settlement.ts`, `subscriptions.ts` | 900 |
| `scheduled-job-invoice-sync` | `core.ts`, `invoices.ts` | 1020 |

Per-file LOC: `core.ts` 600, `invoices.ts` 420, `settlement.ts` 140, `subscriptions.ts` 160.

## Clustering walkthrough

1. Union-of-touches LOC = 1320 ≥ 1000 → surface candidates (Decision 1 fails).
2. Overlap check: intersection = {`core.ts`} = 1 file. Smaller set = 2. Overlap = 1 / 2 = 50% → merge signal fires.
3. Combined touches: {`core.ts`, `invoices.ts`, `settlement.ts`, `subscriptions.ts`} = 1320 LOC ≥ 1000 → merge refused.
4. Individual candidate sizing: `scheduled-job-invoice-sync` = 1020 LOC ≥ 1000 → `unresolved: true`. `message-sub-payment-settled` = 900 LOC → `acceptable`.

## Candidates

| Name | Bucket | LOC | Unresolved |
|---|---|---|---|
| `payment-settled` | acceptable | 900 | no |
| `invoice-sync` | too-large | 1020 | yes |

## Contents

- [`inputs/sources.yaml`](inputs/sources.yaml) — batch sources file with one entry.
- [`inputs/surfaces.json`](inputs/surfaces.json) — two surfaces for `legacy-billing`.
- [`inputs/metadata.json`](inputs/metadata.json) — source metadata (1320 LOC).
- [`inputs/discovery.md`](inputs/discovery.md) — pre-survey discovery with `## Candidate inventory` heading.
- [`expected/survey.md`](expected/survey.md) — byte-stable survey output with one `unresolved: true` candidate.
- [`expected/discovery.md`](expected/discovery.md) — discovery after survey appends candidate blocks.
