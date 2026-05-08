# RM-01 Cross-Repo Suite

> Status: Scenario pack only. The runner backend, fixture bootstrap, and live
> `/change:plan` invocation land in follow-up changes from the
> [implementation plan](../../../rfcs/rm-01-acceptance-framework-implementation-plan.md)
> (C07 setup primitives, C09 plan-level assertions, C10 execute, C11 push and
> finalize).

The first **outside-in acceptance suite** for the [`specify`](../../../AGENTS.md)
framework. Layer 4 in the [acceptance framework](../../README.md#testing-layers).
The Layer 0 substrate counterpart is `specify-cli/tests/cross_repo.rs` (the
Rust test that seeds the same `oauth-login` plan and proves the CLI verbs
compose; this suite asks `/change:plan` to author it instead).

## What It Proves

A realistic cross-repo happy path from a user-facing brief through to a
finalized change, against a temp registry-only platform hub plus two routed
project repos:

- `/change:plan` reads `docs/oauth-login.md` and produces a plan with one
  contract slice (schema-targeted, project-less) and two implementation
  slices routed to `shop-backend` (Omnia) and `shop-mobile` (Vectis), with
  contract-first dependencies.
- `/change:execute loop` drives each slice through `/spec:define`,
  `/spec:build`, and `/spec:merge` on prepared `specify/oauth-login`
  branches, producing the documented baseline-merge / residue commit split
  in each routed project.
- `specify workspace push` opens one PR per routed project through fake
  `gh`. After the runner marks the PRs merged externally,
  `specify change finalize` archives `plan.yaml` and reports both projects
  merged.

The full per-stage assertion list, the role-based plan rules, and the
forbidden conditions all live in [`scenario.md`](scenario.md).

## What It Does Not Cover

- **RM-14 recovery paths.** Blocked entries, failed phase outcomes,
  interrupted driver runs, stale workspace clones, dirty unrelated work,
  and partial push/finalize states are explicitly out of scope. They will
  live in their own suite under `acceptance/suites/` and will reuse the
  same runner and assertion vocabulary.
- **Layer 0 CLI substrate behavior.** `specify-cli/tests/cross_repo.rs`
  already proves the deterministic CLI verbs that this suite depends on
  (lifecycle transitions, JSON output shapes, baseline/residue commit
  boundaries, fake forge handoff). This suite does not duplicate that
  proof; it builds the skill/capability layer on top of it.
- **Exact generated prose.** Live-agent output is allowed to vary in
  wording; assertions match plan **roles**, not exact slice names. See
  [`expected/plan-roles.md`](expected/plan-roles.md).
- **Live forge calls.** The suite uses local bare Git remotes plus a fake
  `gh` shim modelled on the one in `specify-cli/tests/cross_repo.rs`. No
  network call is made.

## Layout

```text
acceptance/suites/rm01-cross-repo/
  README.md                           # this file
  scenario.md                         # canonical scenario pack (frontmatter + body)
  inputs/
    docs/
      oauth-login.md                  # fixture feature brief read by /change:plan
  expected/
    registry.yaml.skeleton.md         # asserted shape of the hub's registry.yaml
    plan-roles.md                     # role-based plan rules + assertion ids (machine-liftable)
    evidence-inventory.md             # per-stage evidence files the runner writes
```

## Backend And Run Evidence

The scenario currently declares `backend: scripted-plan` (C09). The
`scripted-plan` backend is a **deterministic stand-in for `/change:plan`**:
it lands the hub via [`setupHub`](../../runner/hub.ts), then drives a
fixed sequence of `specify change create` + `specify change plan create`
+ three `specify change plan add` calls so the
[role-based plan assertions](expected/plan-roles.md) exercise end to
end. It does **not** read the fixture brief and does **not** prove that
`/change:plan` itself does the right thing on the brief — that work
belongs to the reserved `agent` backend (a future change in the
[implementation plan](../../../rfcs/rm-01-acceptance-framework-implementation-plan.md)).

Operator-driven path (real `/change:plan`):

1. `make acceptance-cross-repo-setup-smoke` lands a fresh hub at
   `<tmp>/shop-platform/` and skips with exit `0` if `specify` is not
   on `PATH`.
2. `cd <tmp>/shop-platform/` and run
   `/change:plan oauth-login source brief=docs/oauth-login.md` in your
   Cursor session.
3. Capture the resulting plan into a JSON `--operator-results` file
   and re-run via `acceptance/runner/main.ts --suite rm01-cross-repo
   --backend manual --operator-results <path> --allow-backend-mismatch`
   to score it through the same role-based rules.

Make targets:

- `make acceptance-cross-repo-setup-smoke` — C07 setup-only smoke
  (four `setup-*` invariants).
- `make acceptance-cross-repo-plan-smoke` — C09 plan-level smoke
  (four `setup-*` + nine `plan-*` assertions). Skips with exit `0`
  when `specify` is missing or pre-RFC-9.

C10/C11 will extend the same scenario file (and either re-use
`scripted-plan` or swap to `agent` once it lands) to drive execute,
push, and finalize — without changing the suite directory or the
assertion vocabulary.

Run evidence (registry, plan snapshot pre-finalize, workspace status, push
JSON, finalize JSON, hub and project Git logs, fake `gh` PR state) is
written to a temp run directory under the runner's control, never into
this repo tree. See
[Run Evidence Policy](../../README.md#run-evidence-policy) for the
retention rules and
[`expected/evidence-inventory.md`](expected/evidence-inventory.md) for the
per-file inventory this suite expects.

## Pointers

- Framework overview, layers, and scenario discovery rules:
  [`../../README.md`](../../README.md).
- Runner contract and failure-domain taxonomy:
  [`../../runner/README.md`](../../runner/README.md).
- Assertion vocabulary:
  [`../../assertions/README.md`](../../assertions/README.md).
- Cross-suite directory convention: [`../README.md`](../README.md).
- The contract-first invariant this suite mirrors at the cross-repo level:
  [`capabilities/contracts/tests/update.md`](../../../capabilities/contracts/tests/update.md).
- Layer 0 executable proof (Rust): `specify-cli/tests/cross_repo.rs`.
- Roadmap entry: [RM-01 in the Specify Roadmap](../../../rfcs/roadmap.md).
- Design and implementation plan:
  [RM-01 Acceptance Framework](../../../rfcs/rm-01-acceptance-framework.md),
  [Implementation Plan](../../../rfcs/rm-01-acceptance-framework-implementation-plan.md),
  and [Outside-In Harness Handoff](../../../rfcs/rm-01-outside-in-harness.md).
