# `/spec:execute --loop` — RFC-2 §"The Plan" meta-fixture

This is the Layer 2 exit-gate meta-fixture for RFC-2 Change L2.I. It
pins the behaviour of `/spec:execute --loop` driven against the full
nine-entry `platform-v2` plan from [RFC-2 §"The Plan"](../../../../../../rfcs/rfc-2-execution.md),
exercising every argument-resolution shape end to end:

- greenfield (`notification-preferences`)
- `affects`-only, single target (`registration-duplicate-email-crash`)
- `affects`-only, multiple targets (`extract-shared-validation`)
- `sources`-only, local path (`product-catalog` → `monolith`)
- `sources`-only, git URL (`shopping-cart` → `orders`)
- pre-`failed` entry (`checkout-api`, preserved)
- pre-`in-progress` entry reclaimed on startup (`email-verification`)

The `combined/` shape (both `sources` and `affects` on one entry) is
pinned by `../field-wiring/combined/`; no entry in RFC-2 §"The Plan"
as authored demonstrates it, which is why the field-wiring fixtures
exist as a separate set.

## Files

| File | Role |
|---|---|
| `plan.yaml.before` | Seed — RFC-2 §"The Plan" YAML verbatim. |
| `metadata-email-verification-crashed.yaml` | Illustrative `.specify/changes/email-verification/.metadata.yaml` the seeded workspace carries — outcome: success on merge, stamped by a prior driver that crashed before running `specify plan transition`. |
| `plan.yaml.after` | Terminal plan state — seven entries `done`, `checkout-api` still `failed`, `checkout-ui` still `pending` (unmet dep on `checkout-api`). |
| `transcript.md` | Narrative timeline: self-heal + five iterations + terminal summary. |

## Relationship to the sibling fixture

`../e2e-platform-v2-with-crash/` uses the same seed but injects a
SIGKILL mid-iteration-4 (`/spec:build product-catalog`). A second
`/spec:execute --loop` run's self-heal resumes the interrupted build
phase, and the initiative continues to the same terminal state. The
two `plan.yaml.after` files in these fixtures are byte-for-byte
identical — crash recovery is observably indistinguishable from the
uncrashed run at the plan level.

## Terminal classification note

The RFC-2 Change L2.I acceptance says "drives… to `all-done`" but
the seed contains a pre-`failed` `checkout-api` whose dependant
`checkout-ui` is therefore permanently ineligible. The loop drains
every entry it *can* complete (seven `done`) and surfaces the rest
as `Completion: stuck` — which is the faithful Layer 2 guarantee.
`transcript.md` has a dedicated section explaining the tension. A
true `Completion: all-done` run requires a seed without dangling
failed-dep chains, which is what `../loop/all-done/` pins on a
three-entry plan.
