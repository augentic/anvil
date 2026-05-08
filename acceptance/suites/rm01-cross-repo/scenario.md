---
id: rm01-cross-repo
owner: rm01
kind: suite
backend: scripted-plan
entrypoint: /change:plan
stages: [define, build, merge]
isolation: fresh-project
authorship-mode: prose
assertions:
  - setup-hub-project-yaml-has-hub-true-and-no-capability
  - setup-registry-has-two-entries
  - setup-registry-entries-have-non-empty-descriptions
  - setup-registry-validate-clean
  - plan-yaml-exists
  - plan-validate-clean
  - plan-has-one-contract-slice
  - plan-has-one-backend-slice
  - plan-has-one-mobile-slice
  - backend-slice-routed-to-shop-backend
  - mobile-slice-routed-to-shop-mobile
  - implementation-slices-depend-on-contract
  - contract-slice-projectless
  - branch-prepared
  - baseline-merge-commit-clean
  - residue-commit-non-empty
  - workspace-clean-before-push
  - slice-has-proposal
  - slice-has-spec
  - slice-has-design-when-required
  - slice-has-tasks
  - slice-baseline-promoted
  - slice-archived
  - implementation-slice-reads-baseline-contract
  - contract-slice-emits-yaml-artifacts
  - contract-slice-yaml-validates-via-tool
  - contract-slice-includes-openapi-or-asyncapi
  - contract-slice-includes-required-schemas
  - contract-baseline-files-present
  - omnia-slice-emits-cargo-toml
  - omnia-slice-emits-lib-rs
  - omnia-slice-residue-under-routed-project
  - omnia-slice-no-output-outside-project
  - omnia-baseline-files-present
  - vectis-slice-emits-composition-yaml
  - vectis-slice-emits-screen-files
  - vectis-slice-residue-under-routed-project
  - vectis-slice-no-output-outside-project
  - vectis-baseline-files-present
  - push-opens-pr-per-project
  - push-output-json-shape-clean
  - finalize-runs-before-prs-merged
  - finalize-archives-plan
  - finalize-output-json-shape-clean
  - finalize-second-call-returns-plan-not-found
  - recorded-trace-replays-cleanly
  - recorded-trace-final-state-matches
  - recorded-trace-no-extra-actions
expected-artifacts:
  - registry.yaml
  - plan.yaml
  - .specify/workspace/shop-backend/
  - .specify/workspace/shop-mobile/
negative-expectations:
  - implementation-slices-author-contract-shapes-inline
  - baseline-merge-commit-contains-residue
  - residue-commit-contains-baseline-files
  - finalize-runs-before-prs-merged
  - rm14-recovery-paths-included
---

# RM-01 Cross-Repo Happy Path

Scenario ID: `rm01-cross-repo`

The first outside-in acceptance suite for the [`specify`](../../../AGENTS.md)
framework. Drives the complete cross-repo happy path from a user-facing feature
brief through plan generation, slice execution, push, external merge
simulation, and finalize across a registry-only platform hub plus two routed
project repos. See [`README.md`](README.md) for a one-screen orientation and
the [acceptance framework overview](../../README.md) for the layer model and
shared vocabulary.

## Intent

Prove the **outside-in cross-repo happy path** for RM-01: that workflow skills
and capability briefs can drive the [`specify`](../../../AGENTS.md) CLI
substrate end to end across a multi-project hub, starting from a user-facing
brief and finishing in a `finalized` change. The suite is the Layer 4
counterpart to `specify-cli/tests/cross_repo.rs` (the Layer 0 substrate test):
that test seeds the plan; this suite asks `/change:plan` to author it.

The chosen feature is **OAuth login**. Two reasons for picking it over the
RFC's other suggestion (dark mode):

- **Discriminating contract surface.** OAuth login forces a real shared HTTP
  contract (token issuance, redirect, refresh) that both backend and mobile
  must consume. A planner that skips the contract slice or inlines interface
  shapes inside an implementation slice is immediately observable. Dark mode
  is mostly client-side and may legitimately produce a single-slice plan,
  which would not exercise contract-first dependencies — the load-bearing
  invariant for RM-01.
- **Continuity with Layer 0.** `specify-cli/tests/cross_repo.rs` already uses
  `oauth-login` as the change name and `oauth-login-contract` /
  `add-oauth-tokens` / `add-oauth-screens` as the slice shape. Reusing the
  feature lets a future runner reuse the same fake-`gh` PR-numbering shim and
  the same fixture project descriptions without divergence.

This is a **happy-path** suite. RM-14 (failure recovery, blocked entries,
stale workspace clones, dirty unrelated work, partial push/finalize states)
is explicitly out of scope and will live in its own suite under
`acceptance/suites/`.

## Workspace

- **Capabilities under test:** `contracts@v1`, `omnia@v1`, `vectis@v1`. The
  registry-only platform hub does not declare a capability; each routed
  project carries its own.
- **Project shape (multi-repo, three roles):**
  - `shop-platform/` — registry-only platform hub created with
    `specify init --hub` (per [AGENTS.md](../../../AGENTS.md) and RFC-9 §1D).
    Owns `change.md`, `plan.yaml`, `registry.yaml`, and the
    `.specify/workspace/<peer>/` materializations. Has no `capability:` field
    in `project.yaml`.
  - `shop-backend/` — Omnia capability project (`capability: omnia@v1`).
    Implements OAuth provider integration, token persistence, and the
    authoritative HTTP API.
  - `shop-mobile/` — Vectis capability project (`capability: vectis@v1`).
    Implements iOS and Android login screens, the OAuth redirect handler,
    and token-refresh flows.
- **Registry shape:** two project entries (`shop-backend`, `shop-mobile`),
  each registered with a description and a fake-GitHub URL. The hub itself
  is not a registry entry. See
  [`expected/registry.yaml.skeleton.md`](expected/registry.yaml.skeleton.md)
  for the asserted shape.
- **Capability availability:** `contracts@v1`, `omnia@v1`, and `vectis@v1`
  must be resolvable from the runner's local capability roots before the
  suite runs (see `specify capability resolve` in
  [AGENTS.md](../../../AGENTS.md)). The runner is expected to pin these
  rather than fetch from the network during a run.
- **Forge:** local bare Git remotes plus a fake `gh` shim modelled on the
  one in `specify-cli/tests/cross_repo.rs`. No real forge is contacted.
- **Isolation:** `fresh-project` — every run starts from an empty temp root,
  initialises the hub, registers both projects, and seeds the brief. No
  state survives between runs.
- **Backend:** `scripted-plan` for the plan stage (C09). The
  scripted-plan backend is a deterministic stand-in for `/change:plan`
  — it lands the hub via [`setupHub`](../../runner/hub.ts) and drives a
  fixed sequence of `specify change plan {create, add}` calls so the
  role-based plan assertions exercise end to end. It does **not**
  prove `/change:plan` itself; that requires the reserved `agent`
  backend (future change). The operator-driven path through real
  `/change:plan` is documented in [`README.md`](README.md#backend-and-run-evidence).
  Later changes in the
  [implementation plan](../../../rfcs/rm-01-acceptance-framework-implementation-plan.md)
  swap in real specialist builds (C13/C14a/C14b) and the recorded
  transcript backend (C15).

### Stage Reference

The `stages: [define, build, merge]` frontmatter field carries the
**slice-loop** stages each routed slice goes through (per the C03 schema's
contiguous-prefix rule from `[define, build, merge, drop]`).

The change-level stages this suite also exercises in prose — `plan`, `push`,
and `finalize` — are not part of the slice-loop stage vocabulary; they are
documented here as the broader operator journey that the suite asserts:

```text
brief
  -> /change:plan          # this scenario's primary entrypoint
    -> /change:execute loop  # drives /spec:define -> /spec:build -> /spec:merge per slice
      -> specify workspace push     # transports specify/<change> branches, opens PRs
        -> operator merges PRs in fake gh   # external boundary
          -> specify change finalize  # observes merged PRs, archives plan.yaml
```

C09 will assert the `plan` stage; C10/C11 will extend the same scenario file
to assert the `push` and `finalize` stages without changing the slice
`stages` field.

## Inputs

The runner must seed the temp hub before invoking `/change:plan`. Setup is
expected to come from the C07 helper code (temp hub via `specify init --hub`,
fixture projects with local bare remotes, registration via `specify registry
add`, fake `gh` and fake SSH installed on `PATH`).

### Hub layout

```text
<temp-root>/
  shop-platform/                       # cwd for every specify invocation
    .specify/
      project.yaml                     # written by `specify init --hub` (carries `hub: true`)
    registry.yaml                      # written by `specify registry add` calls
    docs/
      oauth-login.md                   # the fixture brief (see Inputs §Brief)
    .specify/workspace/                # populated by `specify workspace sync` after the plan lands
      shop-backend/                    # bare-remote-backed clone of shop-backend
      shop-mobile/                     # bare-remote-backed clone of shop-mobile
  remotes/
    shop-backend.git/                  # local bare remote
    shop-mobile.git/                   # local bare remote
  sources/
    shop-backend/                      # working source repo seeded into the bare remote
    shop-mobile/                       # working source repo seeded into the bare remote
  bin/
    gh                                 # fake gh (C07 install)
    fake-ssh                           # fake ssh transport (C07 install)
  gh-state/                            # fake gh persists PR state here
```

### Project seeds

Each fixture project is seeded with the minimum needed for the workflow
skills to find a routable target:

```yaml
# sources/shop-backend/.specify/project.yaml
name: shop-backend
capability: omnia@v1
```

```yaml
# sources/shop-mobile/.specify/project.yaml
name: shop-mobile
capability: vectis@v1
```

The registry is populated through the CLI (no hand-edit), with descriptions
that disambiguate routing for the planner's project-assignment step
(RFC-3b):

```text
specify registry add shop-backend \
  --url git@github.com:shop/shop-backend.git \
  --schema omnia@v1 \
  --description "User registration, account management, OAuth provider integration, token storage, and the authoritative HTTP API."

specify registry add shop-mobile \
  --url git@github.com:shop/shop-mobile.git \
  --schema vectis@v1 \
  --description "iOS and Android mobile clients with login screens, OAuth redirect handling, and token refresh flows."
```

The fake `gh` shim must persist PR state per repo so the runner can later
mark PRs merged externally; see `specify-cli/tests/cross_repo.rs` for the
`gh-state/<repo>.pr` line shape that the C07 helper should mirror.

### Brief

The fixture feature brief lives at
[`inputs/docs/oauth-login.md`](inputs/docs/oauth-login.md) and is the
**only** prose input the planner is allowed to read. It is concise (one
screen) and user-facing — not a pre-seeded plan.

## Invocation

The suite's eventual primary entrypoint is the **orchestrate** mode of
`/change:plan`, which drives the Layer 4 cross-repo umbrella from brief to
finalize in one operator action (per RFC-9 §2C and [AGENTS.md](../../../AGENTS.md)):

```text
/change:plan oauth-login orchestrate

Brief: docs/oauth-login.md
Shape: new-feature
```

C09 ships the **plan-level assertion plumbing** through the
[`scripted-plan` backend](../../runner/backends/README.md#scripted-plan-vs-real-agent-boundary-c09)
— a deterministic stand-in that lands the hub, copies the brief into
`docs/oauth-login.md`, then drives a fixed sequence of CLI calls
mirroring what the planner skill would emit:

```text
specify change create oauth-login
specify change plan create oauth-login
specify change plan add oauth-login-contract --schema contracts@v1 --description "..."
specify change plan add add-oauth-tokens --project shop-backend --depends-on oauth-login-contract --description "..."
specify change plan add add-oauth-screens --project shop-mobile --depends-on oauth-login-contract --description "..."
specify workspace sync
specify --format json workspace status
```

The scripted-plan backend does **not** read the fixture brief and does
**not** vary slice names or routing based on prose content. It exists to
prove the assertion plumbing (setup → plan-shape → role-based rules) end
to end against a deterministic baseline; running real `/change:plan`
against the same fixture is reserved for the `agent` backend
(future change). The non-orchestrate operator path equivalent for the
plan stage:

```text
/change:plan oauth-login source brief=docs/oauth-login.md
```

remains documented for the operator-driven loop described in
[`README.md`](README.md#backend-and-run-evidence).

C10 added the `/change:execute loop` driver step. C11 has now landed
`specify workspace push`, the external-merge simulation (the runner marking
fake PRs merged in `gh-state/`), and `specify change finalize`. None of
those steps need a new scenario file; they extend the same `Assertions` and
`Negative Expectations` lists below. C11 ships through the
`scripted-finalize` backend, a strict superset of `scripted-execute` that
layers push + mark-merged + finalize on top of the same composition pattern
documented in
[`backends/README.md` §Composition Pattern](../../runner/backends/README.md#composition-pattern-for-c11--future-backends).

## Expected Artifacts

These are the structural artifacts the suite asserts on, grouped by stage.
For the per-stage **evidence** captured into the runner's temp run
directory, see [`expected/evidence-inventory.md`](expected/evidence-inventory.md).
For the role-level plan structure, see
[`expected/plan-roles.md`](expected/plan-roles.md).

### After hub setup (Inputs)

- `shop-platform/.specify/project.yaml` carries `hub: true` and **omits**
  `capability:`.
- `shop-platform/registry.yaml` lists `shop-backend` and `shop-mobile` with
  their capabilities and descriptions; see
  [`expected/registry.yaml.skeleton.md`](expected/registry.yaml.skeleton.md).
- `shop-platform/docs/oauth-login.md` exists and matches the fixture brief.

### After `/change:plan` (C09)

- `shop-platform/change.md` exists.
- `shop-platform/plan.yaml` exists.
- `shop-platform/plan.yaml` lists three entries with the role structure
  fixed in [`expected/plan-roles.md`](expected/plan-roles.md): one
  `schema: contracts@v1` entry without a `project:` field, and two
  implementation entries each carrying `project:` and `depends-on:`.
- `shop-platform/.specify/workspace/shop-backend/` and
  `shop-platform/.specify/workspace/shop-mobile/` are materialised by the
  planner's sync-peers step (RFC-9 §2C; only when the registry declares
  multiple projects).

### After `/change:execute loop` (C10) — extends in a later change

- Each routed slice's project workspace clone is checked out at exactly
  branch `specify/oauth-login` (per
  [`acceptance/assertions/README.md`](../../assertions/README.md)).
- The contract slice produces a baseline merge commit under the hub's
  `.specify/specs/` and `.specify/archive/` paths.
- Each implementation slice's clone carries two commits: a baseline merge
  commit (`specify: merge <slice>`) touching only `.specify/specs/` and
  `.specify/archive/`, and a residue commit (`specify: residue <slice>`)
  carrying generated project outputs outside `.specify/`.

### After `specify workspace push` and finalize (C11) — extends in a later change

- Fake `gh` PR state shows one open PR per routed project, each on branch
  `specify/oauth-login`.
- After the runner marks both PRs merged externally, `specify change
  finalize` archives `plan.yaml` and reports both projects as merged.
- A second `specify change finalize` returns `plan-not-found`.

## Assertions

The C09 change has lifted the assertion ids below verbatim into the
frontmatter `assertions:` list. The structural rules each plan-* id
checks live in [`expected/plan-roles.md`](expected/plan-roles.md); the
four setup-* invariants live in
[`expected/registry.yaml.skeleton.md`](expected/registry.yaml.skeleton.md)
and are implemented in
[`acceptance/assertions/setup.ts`](../../assertions/setup.ts). The
vocabulary itself comes from
[`acceptance/assertions/README.md`](../../assertions/README.md).

The runner sorts setup-* ids ahead of plan-* ids so a setup failure
demotes every plan-level rule to `skip` (not `fail`) and the failure
attribution stays clean.

The full set required for the cross-repo happy path:

Setup invariants (run first):

- `setup-hub-project-yaml-has-hub-true-and-no-capability` —
  `shop-platform/.specify/project.yaml` carries `hub: true` and omits
  `capability:`.
- `setup-registry-has-two-entries` — `shop-platform/registry.yaml`
  lists exactly two project entries.
- `setup-registry-entries-have-non-empty-descriptions` — both project
  entries carry a non-empty `description:`.
- `setup-registry-validate-clean` — `specify registry validate` exits
  `0`.

Plan rules (run after setup; demoted to `skip` on setup failure):

- `plan-yaml-exists` — `shop-platform/plan.yaml` exists after the planner
  returns.
- `plan-validate-clean` — `specify change plan validate` exits `0` on the
  generated plan.
- `plan-has-one-contract-slice` — exactly one entry has
  `schema: contracts@v1`. No `project:` field on that entry.
- `plan-has-one-backend-slice` — exactly one entry has
  `project: shop-backend`.
- `plan-has-one-mobile-slice` — exactly one entry has
  `project: shop-mobile`.
- `backend-slice-routed-to-shop-backend` — the unique backend entry's
  `project` field equals `shop-backend` (registry routing produced the
  expected target, not a sibling).
- `mobile-slice-routed-to-shop-mobile` — the unique mobile entry's
  `project` field equals `shop-mobile`.
- `implementation-slices-depend-on-contract` — both implementation entries
  list the contract entry (matched by id) in their `depends-on`.
- `contract-slice-projectless` — the contract entry has no `project:` field
  (it lives at the hub, not in a routed project).

Names are role-based, not exact. The planner is allowed to call the
contract slice anything from `oauth-login-contract` to
`oauth-shared-http-api`; the role-based matching above covers all such
names. The same applies to the implementation slices. See
[`expected/plan-roles.md`](expected/plan-roles.md) for the matching rules
in machine-liftable form.

Execute rules (run after plan; demoted to `skip` on setup-*, plan-*, or
missing-execute-state failure):

- `branch-prepared` — every routed clone is on `specify/oauth-login` after
  `specify workspace prepare-branch`.
- `baseline-merge-commit-clean` — the per-slice baseline merge commit
  (`specify: merge <slice>`, HEAD~1 in each routed clone) touches only
  `.specify/specs/` and `.specify/archive/`.
- `residue-commit-non-empty` — the per-slice residue commit (`specify:
  residue <slice>`, HEAD in each routed clone) is non-empty and touches
  paths entirely outside `.specify/`.
- `workspace-clean-before-push` — every routed clone has empty
  `git status --porcelain` output before push.

Under the `scripted-plan` backend (plan-only smoke), the four execute-*
ids cleanly demote to `skip` because no execute backend ran; the C09
plan-only smoke target therefore stays green even with the ids in the
list. Under `scripted-execute` (C10) all four assert.

Push / finalize rules (C11; run after execute; demoted to `skip` on
setup-*, plan-*, execute-*, or missing-finalize-state failure):

- `push-opens-pr-per-project` — fake `gh` shows one PR file per routed
  project on branch `specify/oauth-login`, each carrying the PR number
  reported by `workspace push --format json`.
- `push-output-json-shape-clean` — `cli-substrate` fault-domain pin
  against `specify --format json workspace push` output drift
  (per-project `name`, `status: pushed`, `branch`, numeric `pr`).
- `finalize-runs-before-prs-merged` — pre-merge `change finalize`
  refuses while PRs are still OPEN (RFC-14 guard). Surfaces as a
  `cli-substrate` finding (not a hard fail) when the CLI accepts.
- `finalize-archives-plan` — `specify change finalize` removes the
  live `plan.yaml`, reports an `archived` path that exists on disk,
  and creates a `.specify/archive/plans/<change>-*` directory.
- `finalize-output-json-shape-clean` — `cli-substrate` fault-domain
  pin against `specify --format json change finalize` output drift
  (`initiative`, `finalized: true`, per-project `merged` status,
  `summary.merged`, `archived`).
- `finalize-second-call-returns-plan-not-found` — second
  `specify --format json change finalize` exits non-zero with
  `error: plan-not-found` (idempotency).

Under `scripted-execute` (execute-only smoke), every push-* /
finalize-* id cleanly demotes to `skip` because no finalize backend
ran. Under `scripted-finalize` (C11) all six assert.

Contract-build rules (C13; run after define-* and execute-*; demoted
to `skip` on setup-*, plan-*, or missing-execute-state failure):

- `contract-slice-emits-yaml-artifacts` — at least one `.yaml` file
  lives under `<hub>/contracts/**` after the contract slice has run.
- `contract-slice-yaml-validates-via-tool` — `specify tool run
  contract -- <hub>/contracts --format json` exits cleanly (`ok:
  true`). Skips with `cli-substrate` rationale when the contract
  WASM cannot be located (e.g. operator hasn't built
  `cargo build -p contract-validate --release`).
- `contract-slice-includes-openapi-or-asyncapi` — at least one file
  exists under `contracts/http/` or `contracts/messages/`.
- `contract-slice-includes-required-schemas` — the OAuth-relevant
  schemas (token request / token response / error response) exist
  under `contracts/schemas/`.
- `contract-baseline-files-present` — every YAML the contract slice
  emits survives the merge and lives at the expected baseline path.

Under `scripted-plan` / `scripted-execute` / `scripted-finalize` /
`agent` (non-C13 backends), the contract-build ids cleanly demote
to `skip` because no driver wrote contract YAML — the handlers
detect an empty `<hub>/contracts/` tree and emit a
"`contracts-build` backend not active" skip rationale instead of
failing. Under `contracts-build` (C13) all five assert and pin the
fault domain to `specialist-generation` (YAML missing /
malformed), `cli-substrate` (validator binary unavailable / mis-
resolved), or `skill-orchestration` (baseline merge dropped the
file).

Omnia-build rules (C14a; run after define-* and execute-*; demoted
to `skip` on setup-*, plan-*, missing-execute-state, or no-omnia-
slice failure):

- `omnia-slice-emits-cargo-toml` — every Omnia-capability
  implementation slice emits `crates/<crate>/Cargo.toml` into the
  routed project clone, with a `[package]`-bearing TOML body that
  cargo can parse. The C14a fixture uses
  `crates/oauth_tokens/Cargo.toml` for the `add-oauth-tokens`
  slice.
- `omnia-slice-emits-lib-rs` — every Omnia slice emits a non-empty
  `crates/<crate>/src/lib.rs` (the residue file the per-slice
  driver writes through `driveSliceWithBodies`).
- `omnia-slice-residue-under-routed-project` — every Omnia output
  path lives under the routed project's `crates/` tree
  (boundary-shape check; complements the generic
  `residue-commit-non-empty` rule with an Omnia-specific
  expectation).
- `omnia-slice-no-output-outside-project` — the routed clone has
  no Omnia-shaped output outside `crates/` (forbidden-path probe
  for common build mistakes: stray top-level `Cargo.toml`,
  `src/lib.rs`, `lib.rs`, or `target/`).
- `omnia-baseline-files-present` — after merge, the Omnia slice's
  baseline `.specify/specs/<slice>/` dir (with `proposal.md` +
  `tasks.md`) and `.specify/archive/<slice>/` dir exist in the
  routed project clone.

Under `scripted-plan` / `scripted-execute` / `scripted-finalize` /
`contracts-build` / `agent` (non-C14a backends), the omnia-build
ids cleanly demote to `skip` because no Omnia slice ran through
`OmniaBuildPhaseDriver` — the handlers detect an absent crate tree
(or, more precisely, no Omnia-routed slice on `executeState`) and
emit a "no omnia slice on executeState.slices" skip rationale
instead of failing. Under `omnia-build` (C14a) all five assert and
pin the fault domain to `specialist-generation` (Cargo.toml /
lib.rs missing or malformed) or `skill-orchestration` (baseline
merge dropped the slice spec dir). C14b adds the Vectis
counterpart for the mobile slice.

Vectis-build rules (C14b; run after define-* and execute-*; demoted
to `skip` on setup-*, plan-*, missing-execute-state, or no-vectis-
slice failure):

- `vectis-slice-emits-composition-yaml` — every Vectis-capability
  implementation slice emits `composition.yaml` at the routed
  project clone's root, with a body that parses as YAML carrying
  `version: 1` and either a `screens` map (with at least one
  entry) or a `delta` block (mutually exclusive per
  [`capabilities/vectis/composition.schema.json`](../../../capabilities/vectis/composition.schema.json)).
  The C14b fixture emits one `login` screen for the
  `add-oauth-screens` slice.
- `vectis-slice-emits-screen-files` — every Vectis slice emits a
  non-empty platform shell file at the residue path the per-slice
  driver writes through `driveSliceWithBodies`. The C14b fixture
  uses `apps/mobile/login_screen.swift` (matching the
  `RESIDUE_PATHS` policy in `scripted-shared.ts`).
- `vectis-slice-residue-under-routed-project` — every Vectis
  output path is either `composition.yaml` at the project root or
  lives under the routed project's `apps/` tree (boundary-shape
  check; complements the generic `residue-commit-non-empty` rule
  with a Vectis-specific expectation).
- `vectis-slice-no-output-outside-project` — the routed clone has
  no Vectis-shaped output at common-mistake locations
  (forbidden-path probe: a stray top-level `LoginScreen.swift` /
  `MainActivity.kt` / `LoginActivity.kt`, vendored package dirs
  `Pods/` / `node_modules/`, build trees `build/` / `DerivedData/`).
- `vectis-baseline-files-present` — after merge, the Vectis slice's
  baseline `.specify/specs/<slice>/` dir (with `proposal.md` +
  `tasks.md`) and `.specify/archive/<slice>/` dir exist in the
  routed project clone.

Under `scripted-plan` / `scripted-execute` / `scripted-finalize` /
`contracts-build` / `omnia-build` / `agent` (non-C14b backends),
the vectis-build ids cleanly demote to `skip` because no Vectis
slice ran through `VectisBuildPhaseDriver` — the handlers detect
an absent `composition.yaml` (or, more precisely, no Vectis-routed
slice on `executeState`) and emit a "no vectis slice on
executeState.slices" skip rationale instead of failing. Under
`vectis-build` (C14b) all five assert and pin the fault domain to
`specialist-generation` (composition.yaml / screen file missing
or malformed) or `skill-orchestration` (baseline merge dropped
the slice spec dir).

## Negative Expectations

These are forbidden conditions. Each id is meant to be liftable verbatim by
the C09/C10/C11 changes into structured assertion entries.

- `implementation-slices-author-contract-shapes-inline` — **the load-bearing
  RM-01 contract-first invariant.** Implementation slices (`add-oauth-*`)
  must not author new `contracts/**/*.yaml` shapes inline. They must
  `depends-on` the contract slice and read the merged baseline `contracts/`
  files as context. This mirrors `contracts-update-boundary` from
  [`capabilities/contracts/tests/update.md`](../../../capabilities/contracts/tests/update.md)
  but at the cross-repo plan level.
- `baseline-merge-commit-contains-residue` — the per-slice baseline merge
  commit (`specify: merge <slice>`) must touch only `.specify/specs/` and
  `.specify/archive/`. Generated project outputs (Omnia crates, Vectis
  shells, etc.) must live in the residue commit, never the baseline commit.
- `residue-commit-contains-baseline-files` — the residue commit
  (`specify: residue <slice>`) must not touch `.specify/specs/` or
  `.specify/archive/`. The split is enforced by the workflow contract and
  asserted on the project clones in C10/C11.
- `finalize-runs-before-prs-merged` — `specify change finalize` must refuse
  to archive when one or more per-project PRs are still open in fake `gh`.
  The orchestrate path waits for an operator merge in the forge UI; Specify
  must not call `gh pr merge` itself (per RFC-14, [AGENTS.md](../../../AGENTS.md)).
- `rm14-recovery-paths-included` — this suite must not assert recovery
  behavior (blocked entries, failed phase outcomes, interrupted driver
  runs, stale workspace clones, dirty unrelated work, partial push or
  finalize states). RM-14 owns those.

The fault-domain hint (per
[`acceptance/runner/README.md` §Failure Reporting](../../runner/README.md#failure-reporting))
the runner should attach when one of these is violated:

- contract-shape leakage → `skill-orchestration` or `capability-brief`,
- baseline/residue boundary violation → `cli-substrate` or
  `skill-orchestration`,
- premature finalize → `cli-substrate` (the verb itself should refuse) or
  `external-fake-boundary` (the fake `gh` lied about merge state).

## Cleanup

- **Per-run:** the runner discards the temp root on pass and preserves it
  on failure, per the
  [Run Evidence Policy](../../README.md#run-evidence-policy). An explicit
  `--preserve` flag preserves all runs.
- **In-suite:** no slice or change is left half-archived. After C11, the
  successful path produces `.specify/archive/plans/<YYYYMMDD>-oauth-login/`
  (matching the shape `specify-cli/tests/cross_repo.rs` already asserts on
  for Layer 0). On failure the suite leaves whatever state the failure
  produced — the runner does not attempt to "fix" partial state.
- **No commits to the framework repo** are produced by the suite. Run
  output (registry snapshots, plan snapshots, `gh-state/`, project Git
  logs) goes to the temp run directory only. The fixture brief, the
  expected role spec, the registry skeleton, and the evidence inventory
  in this directory are the only checked-in artifacts the suite needs.
