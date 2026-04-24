# Monolith propose notes

## What this fixture pins

The C22 monolith discovery fixture produces three capability summaries keyed by the `<!-- source-key: monolith -->` marker. This fixture pins the **expected plan shape** from the three-slice inventory:

[`inputs/discovery.md`](inputs/discovery.md) (byte-identical to C22 `expected/discovery.md`) → [`expected/plan.yaml`](expected/plan.yaml): every entry carries file-path hints and delta-targeting intent in the `description` field. The define skill infers extract filters and baseline targets from description at execution time.

## 1:1 mapping — capability → plan entry

Per [`schemas/omnia/briefs/plan/propose.md` §Mapping rule](../../../../../../../schemas/omnia/briefs/plan/propose.md):

| Capability                                         | Plan entry `name`     | `sources`    | `description` (carries path hints + delta intent)                                                                   | `depends-on`                                    |
| -------------------------------------------------- | --------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------- |
| `email-verification` (`confidence: high`)          | `email-verification`  | `[monolith]` | Verify a newly registered account via a one-time email token. Focus on src/auth/verify.ts.                          | `[]`                                            |
| `shared-validation` (`confidence: medium`)         | `shared-validation`   | `[monolith]` | Validate common user-facing inputs with reusable primitives. Focus on src/common/validation.ts.                     | `[]`                                            |
| `user-registration` (`confidence: high`)           | `user-registration`   | `[monolith]` | Create new user accounts with email verification. Focus on src/auth/verify.ts, src/users/register.ts, src/users/validation.ts. Delta-targets email-verification and shared-validation. | `[email-verification, shared-validation]`       |

No field is invented; every value is lifted from the capability block. `hints.*` is intentionally dropped — the brief leaves it in `discovery.md` for operator reference and does not project it into `plan.yaml`.

## Emit order

Dependency-order + within-layer alphabetical:

1. `email-verification` (layer 0)
2. `shared-validation` (layer 0)
3. `user-registration` (layer 1, depends on both leaves)

Matches the order `specify plan next` would walk at execution time.

## Confidence flags

- All three capabilities are `high` or `medium`, so none trips the `⚠ review before accepting` flag in the loop.
- When `user-registration` is `confidence: low`, the brief surfaces the review flag and the human can enrich the description to narrow scope.

## Stability points

- **Initiative name.** `traffic`, lifted from the `# Discovery — traffic` header in the starting-state discovery files.
- **`sources.monolith` path.** `./inputs`, matching the C22 invocation `--source monolith=./inputs`.
- **Change ordering.** Dependency-order + within-layer alphabetical: `email-verification`, `shared-validation`, `user-registration`.
- **`depends-on` list order.** Alphabetical — `[email-verification, shared-validation]` for `user-registration`.
- **Field order inside each plan entry.** Matches the CLI's serde output: `name`, `project`, `status`, `depends-on`, `sources`, `description`, `status-reason`.

## Downstream consumers

- The define skill reads path hints from `description` to infer extract filters and reads change-name references to infer delta targets against baselines in `.specify/specs/`.
