# `/change:execute loop` — `platform-v2` meta-fixture

This is the exit-gate meta-fixture for `/change:execute loop`. It pins the behaviour of the driver against the full nine-entry `platform-v2` plan, exercising every argument-resolution shape end to end:

- greenfield (`notification-preferences`)
- description-driven delta targeting, single target (`registration-duplicate-email-crash`)
- description-driven delta targeting, multiple targets (`extract-shared-validation`)
- `sources`-only, local path (`product-catalog` → `monolith`)
- `sources`-only, git URL (`shopping-cart` → `orders`)
- pre-`failed` entry (`checkout-api`, preserved)
- pre-`in-progress` entry reclaimed on startup (`email-verification`)

The `combined/` shape (both `sources` and a description-driven delta target on one entry) is pinned by `../field-wiring/combined/`; no entry in RFC-2 §"The Plan" as authored demonstrates it, which is why the field-wiring fixtures exist as a separate set.

## Files

| File | Role |
|---|---|
| `plan.yaml.before` | Seed — RFC-2 §"The Plan" YAML verbatim. |
| `metadata-email-verification-crashed.yaml` | Illustrative `.specify/slices/email-verification/.metadata.yaml` the seeded workspace carries — outcome: success on merge, stamped by a prior driver that crashed before running `specify plan transition`. |
| `plan.yaml.after` | Terminal plan state — seven entries `done`, `checkout-api` still `failed`, `checkout-ui` still `pending` (unmet dep on `checkout-api`). |
| `transcript.md` | Narrative timeline: self-heal + five iterations + terminal summary. |

## Relationship to the sibling fixture

`../e2e-platform-v2-with-crash/` uses the same seed but injects a SIGKILL mid-iteration-4 (`/spec:build product-catalog`). A second `/change:execute loop` run's self-heal resumes the interrupted build phase, and the change continues to the same terminal state. The two `plan.yaml.after` files in these fixtures are byte-for-byte identical — crash recovery is observably indistinguishable from the uncrashed run at the plan level.

## Authoring → execution path

The `plan.yaml.before` seed here is the *output* of an authoring run of `/change:draft`. For a pinned five-slice authoring run that produces an analogous plan shape, see [`../../../draft/fixtures/propose/`](../../../draft/fixtures/propose/):

- `discovery.md` — the inventory `/change:draft` step 4(a) wrote.
- `transcript.md` — the interactive accept / edit / reject loop.
- `expected-proposal.md` — the authoring audit trail.
- `expected-plan.yaml` — the final `plan.yaml` after the five `specify plan add` calls.

Together the two fixture sets pin the full `/change:draft → /change:execute loop` path: authoring turns `source` / `from` inputs into a validated `plan.yaml`, and execution drives that plan to `all-done` (or `stuck`, or a self-healable interrupt) without further human intervention.

## Terminal classification note

The RFC-2 Change L2.I acceptance says "drives… to `all-done`" but the seed contains a pre-`failed` `checkout-api` whose dependant `checkout-ui` is therefore permanently ineligible. The loop drains every entry it *can* complete (seven `done`) and surfaces the rest as `Completion: stuck` — which is the faithful exit-gate guarantee for the driver. `transcript.md` has a dedicated section explaining the tension. A true `Completion: all-done` run requires a seed without dangling failed-dep chains, which is what `../loop/all-done/` pins on a three-entry plan.
