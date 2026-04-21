# Monolith propose notes

## What this fixture pins

The C22 monolith discovery fixture produces three capability
summaries keyed by the `<!-- source-key: monolith -->` marker. This
fixture pins two **expected plan shapes** from the same three-slice
inventory:

1. **Glob default** — [`inputs/discovery.md`](inputs/discovery.md)
   (byte-identical to C22 `expected/discovery.md`) →
   [`expected/plan.yaml`](expected/plan.yaml): every entry uses
   `scope.monolith.include` lifted verbatim from each capability's
   `sources:` list.
2. **Stage C manifest** —
   [`inputs/discovery-manifest.md`](inputs/discovery-manifest.md)
   (only difference: `user-registration` has `confidence: low`) →
   [`expected/plan-manifest.yaml`](expected/plan-manifest.yaml):
   the two leaves stay glob-scoped; `user-registration` carries
   `scope.monolith.manifest` and the pinned body lives under
   [`expected/slices/user-registration.yaml`](expected/slices/user-registration.yaml).

## 1:1 mapping — capability → plan entry (glob branch)

Per [`schemas/omnia/briefs/plan/propose.md` §Mapping rule](../../../../../../../schemas/omnia/briefs/plan/propose.md):

| Capability                                         | Plan entry `name`     | `sources`    | `scope.monolith.include`                                                              | `depends-on`                                    | `description` (← `summary`)                                   |
| -------------------------------------------------- | --------------------- | ------------ | ------------------------------------------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------- |
| `email-verification` (`confidence: high`)          | `email-verification`  | `[monolith]` | `[src/auth/verify.ts]`                                                                | `[]`                                            | Verify a newly registered account via a one-time email token. |
| `shared-validation` (`confidence: medium`)         | `shared-validation`   | `[monolith]` | `[src/common/validation.ts]`                                                          | `[]`                                            | Validate common user-facing inputs with reusable primitives.  |
| `user-registration` (`confidence: high`)           | `user-registration`   | `[monolith]` | `[src/auth/verify.ts, src/users/register.ts, src/users/validation.ts]`                | `[email-verification, shared-validation]`       | Create new user accounts with email verification.             |

No field is invented; every value is lifted verbatim from the
capability block. `hints.*` is intentionally dropped — the brief
leaves it in `discovery.md` for operator reference and does not
project it into `plan.yaml`.

## Manifest branch (C27)

When `user-registration` is **`confidence: low`** and its `sources:`
still overlaps `email-verification` on `src/auth/verify.ts`, Stage
C replaces `scope.monolith.include` on **that** entry with
`scope.monolith.manifest: .specify/plans/traffic/slices/user-registration.yaml`
and pins the manifest file at
[`expected/slices/user-registration.yaml`](expected/slices/user-registration.yaml)
(`version: 1` + `include:` paths relative to `sources.monolith`).
Invocations are pinned in
[`expected/create-invocations-manifest.md`](expected/create-invocations-manifest.md)
(`--scope-manifest monolith=.specify/plans/traffic/slices/user-registration.yaml`).
The leaves (`email-verification`, `shared-validation`) are unchanged
from the glob branch — one mixed authoring run, two scope forms.

## Emit order

Dependency-order + within-layer alphabetical:

1. `email-verification` (layer 0)
2. `shared-validation` (layer 0)
3. `user-registration` (layer 1, depends on both leaves)

Matches the order `specify initiative next` would walk at
execution time.

## Expected `scope-overlap` warning (glob branch)

`src/auth/verify.ts` appears in **both** `email-verification`'s
and `user-registration`'s `scope.monolith.include`. This is the
C22-pinned overlap (the capability block for `user-registration`
includes `src/auth/verify.ts` verbatim). The propose brief emits
glob-based slices anyway; `specify initiative validate` surfaces
this as a `scope-overlap` warning (RFC-3a §*Validation*, C05) and
the human resolves during the accept/edit loop — typical fix is to
narrow `user-registration`'s scope to `src/users/**` so
`verify.ts` stays exclusively with `email-verification`.

Per C24 guardrails the brief **does not** auto-resolve the overlap
in the glob branch. C27 adds the manifest branch when low
confidence meets that tangled overlap.

## Confidence flags

- **Glob branch:** all three capabilities are `high` or `medium`, so
  none trips the `⚠ review before accepting` flag in the loop.
- **Manifest branch:** `user-registration` is `confidence: low`, so
  the brief surfaces the review flag **and** emits the slice
  manifest per Stage C.

## Stability points

- **Initiative name.** `traffic`, lifted from the `# Discovery —
  traffic` header in the starting-state discovery files.
- **`sources.monolith` path.** `./inputs`, matching the C22
  invocation `--source monolith=./inputs`.
- **Change ordering.** Dependency-order + within-layer
  alphabetical: `email-verification`, `shared-validation`,
  `user-registration`.
- **`depends-on` list order.** Alphabetical —
  `[email-verification, shared-validation]` for
  `user-registration`.
- **`scope.monolith.include` list order (glob branch).** Lifted
  verbatim from the capability's `sources:` (which C22 pins as
  alphabetical).
- **Field order inside each plan entry.** Matches the CLI's serde
  output: `name`, `status`, `depends-on`, `affects`, `sources`,
  `scope`, `description`, `status-reason`.
- **`affects: []` + `status-reason: null`.** Emitted as the CLI's
  natural round-trip shape (matches sibling
  [`../expected-plan.yaml`](../expected-plan.yaml) and
  [`../../propose-vectis/expected-plan.yaml`](../../propose-vectis/expected-plan.yaml)).

## Downstream consumers

- **C25** (monolith-scale lint) reads the same `metadata.json` the
  C22 fixture pins and fires a warning on monolith-scale sources
  without a scope entry. This fixture's plan is fully scoped, so
  it never trips the warning.
