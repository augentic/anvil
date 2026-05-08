# RM-01 Integration Acceptance Harness Handoff

> Purpose: handoff notes for implementing the skill/capability-level acceptance layer above the existing Rust CLI substrate test.

## Context

RM-01 is about proving a realistic multi-slice, multi-repo Specify workflow end to end:

- plan generation
- registry routing
- dependent slice execution
- workspace sync and branch preparation
- baseline and residue commit behavior
- push and PR/MR handoff
- finalize after external merge

The current executable proof is `specify-cli/tests/cross_repo.rs`. That test is valuable, but it is a CLI substrate test. It seeds the plan and simulates phase outputs, then proves that deterministic CLI behavior composes correctly across local fixture repositories and fake forge behavior.

The missing layer is an integration harness that starts where users start: change brief and documentation, then runs the actual workflow skills and capability pipelines.

## What The Existing Rust Test Proves

`specify-cli/tests/cross_repo.rs` proves the deterministic contract that skills rely on:

- `specify init --hub` creates a registry-only hub.
- `specify registry add` creates a multi-project registry.
- `specify change plan *` can represent a hub-level contract slice and routed implementation slices.
- `specify workspace sync` materializes remote-backed workspace slots.
- `specify workspace prepare-branch` checks out exact `specify/<change>` branches.
- Plan dependency order can be replayed through `specify change plan next`.
- Routed projects can carry split commits:
  - baseline merge commit under `.specify/specs/` and `.specify/archive/`
  - non-baseline residue commit as `specify: residue <slice-name>`
- `specify workspace push` opens fake PRs through a fake `gh`.
- `specify change finalize` observes externally merged PRs and archives the plan.
- A second finalize returns `plan-not-found`.

This confirms the CLI supports RM-01, but it does not exercise `/change:plan`, `/change:execute`, `/spec:define`, `/spec:build`, `/spec:merge`, or specialist capability skills.

## What The Integration Layer Must Prove

The integration harness should prove this full path:

```text
change brief/docs
  -> /change:plan
  -> capability brief pipelines
  -> /change:execute loop
  -> /spec:define
  -> /spec:build
  -> specialist skills
  -> /spec:merge
  -> workspace push
  -> external PR merge simulation
  -> change finalize
```

The goal is not to duplicate CLI unit tests. The goal is to catch drift where skills, briefs, capability manifests, specialist generators, or orchestration prose no longer drive the CLI substrate correctly.

## Core Design Choice

Build a skill/capability replay harness, not another Rust-only integration test.

The hard problem is replayable agent execution. Skills are agent-driven: they read context, make judgment calls, write artifacts, invoke CLI verbs, and delegate to specialist skills. A normal Rust test cannot invoke that behavior unless it either shells out to a real agent runtime or replaces the agent layer with deterministic fixtures.

Recommended approach: hybrid integration replay.

- Run real workflow surfaces where possible: `/change:plan`, `/change:execute loop`, `/spec:define`, `/spec:build`, `/spec:merge`.
- Use tightly scoped fixture inputs so outputs are structurally predictable.
- Assert structural invariants rather than byte-for-byte generated prose.
- Reuse the local Git/fake forge strategy from `specify-cli/tests/cross_repo.rs`.
- Keep the first version happy-path only; RM-14 owns recovery paths.

## Harness Components

### 1. Runner

Create a runner that can execute slash-command workflows in a temp workspace.

It must provide:

- a temp platform hub
- local fixture repositories for at least two registered projects
- fake GitHub remotes backed by local bare repos
- fake `gh` behavior for PR create/list/view/edit
- a way to mark fake PRs as merged between runs
- capture of tool calls, stdout/stderr, and final filesystem state

Possible implementations:

- Cursor SDK agent runner if programmatic slash-command execution is available.
- A project-local script that launches a Cursor/agent CLI session with fixed prompts.
- A recorded transcript runner that replays known agent/tool decisions.
- A deterministic test-double runner for phase skills as an interim step.

The strongest version uses a real agent runtime with pinned model/runtime and structural assertions.

### 2. Fixture Scenario

Use one small docs-driven scenario, for example `oauth-login` or `dark-mode`.

Initial shape:

```text
shop-platform/            # registry-only hub
shop-backend/             # omnia@v1
shop-mobile/              # vectis@v1
docs/oauth-login.md       # concise feature brief
```

Expected plan:

```yaml
changes:
  - name: oauth-login-contract
    schema: contracts@v1
    depends-on: []
  - name: add-oauth-tokens
    project: shop-backend
    depends-on: [oauth-login-contract]
  - name: add-oauth-screens
    project: shop-mobile
    depends-on: [oauth-login-contract]
```

The exact names can vary in a live-agent version, but the harness should normalize by role:

- exactly one contract slice
- exactly one backend implementation slice
- exactly one mobile implementation slice
- implementation slices depend on the contract slice
- implementation slices route to the expected projects

### 3. Capability Fixtures

The harness needs real capability availability for:

- `contracts@v1`
- `omnia@v1`
- `vectis@v1`

The fixture repositories must include enough project state for the skills and validators to run:

- `.specify/project.yaml`
- capability identifiers resolvable from local or cached capability roots
- any expected baseline directories
- minimal source layout needed by specialist build skills

If running full code generation is too expensive for the first pass, define a staged rollout:

1. Run real `/change:plan`; stub define/build/merge.
2. Run real `/spec:define`; stub build specialists.
3. Run real `/spec:build` for one capability at a time.
4. Run the full three-capability flow.

### 4. Assertions

Prefer structural assertions over prose exact matches.

Plan assertions:

- `plan.yaml` exists.
- Plan validates cleanly.
- Contract slice is schema-targeted and project-less.
- Implementation slices have `project`.
- Dependencies enforce contract-first execution.

Artifact assertions:

- Each slice has `proposal.md`.
- Each slice has specs.
- Each slice has `design.md` when the capability requires it.
- Each slice has `tasks.md`.
- Contract slice produces or updates a contract artifact.
- Backend/mobile implementation slices reference the contract context.

Capability assertions:

- Contracts validator passes for generated contract artifacts.
- Omnia output has expected crate/test layout or passes its configured check.
- Vectis output has expected composition/design/scaffold artifacts or passes its configured validator.
- Specialist skill outputs do not write outside expected capability-owned paths.

Execution assertions:

- `/change:execute loop` reaches `all-done`.
- Routed entries run in `.specify/workspace/<project>/`.
- Prepared branch is exactly `specify/<change-name>`.
- Baseline merge commits contain only `.specify/specs/` and `.specify/archive/`.
- Residue commits contain generated project outputs.
- Workspace clones are clean before push.

Landing assertions:

- `specify workspace push` creates or updates PRs through fake `gh`.
- The harness marks fake PRs merged externally.
- `specify change finalize` reports merged projects.
- `plan.yaml` moves to archive.
- Re-running finalize returns `plan-not-found`.

### 5. Output Artifacts

Each harness run should save enough evidence for debugging:

```text
acceptance-runs/<timestamp-or-test-name>/
  transcript.md
  tool-calls.jsonl
  final-tree.txt
  plan.yaml.before-finalize
  registry.yaml
  workspace-status.json
  push-output.json
  finalize-output.json
  failures/
```

This should not become a permanent checked-in artifact on every run. Checked-in fixtures should be small and deterministic; run outputs should be temporary unless intentionally promoted to a golden.

## Recommended Implementation Phases

### Phase 1: Skill Runner Spike

Goal: prove the harness can invoke one slash command in a temp project and capture outputs.

Suggested target:

```text
/change:plan oauth-login from ./docs/oauth-login.md
```

Success criteria:

- runner creates a temp hub
- runner registers two fixture projects
- slash command runs
- `plan.yaml` exists
- `specify change plan validate` passes
- transcript/tool-call capture works

### Phase 2: Plan-Level Integration Test

Goal: real `/change:plan`, deterministic assertions.

Success criteria:

- discovery reads fixture docs
- sync-peers materializes workspace slots
- plan contains expected role structure
- assignment routes backend/mobile slices correctly
- no execution yet

### Phase 3: Execute With Stubbed Phase Outputs

Goal: real `/change:execute loop`, but phase skills are deterministic test doubles.

Success criteria:

- executor performs plan-next loop
- executor routes into workspace projects
- branch preparation happens through CLI helper
- stubbed phases stamp outcomes correctly
- plan reaches `all-done`

This phase is useful if full `/spec:define` and `/spec:build` are too nondeterministic initially.

### Phase 4: Real Define/Merge, Stub Build

Goal: prove capability brief pipelines generate artifacts and merge works.

Success criteria:

- `/spec:define` writes capability artifacts
- `/spec:merge` updates baseline
- build output can still be fixture/stubbed
- merge/residue split remains valid

### Phase 5: Full Capability Build

Goal: real `/spec:build` and specialist skill delegation.

Success criteria:

- contracts skill authors/verifies contract artifact
- Omnia specialist output is generated and testable
- Vectis specialist output is generated and validatable
- all outputs survive merge/finalize path

### Phase 6: CI Integration

Goal: make the harness usable without destabilizing normal PR checks.

Recommended command:

```bash
make acceptance-cross-repo
```

Suggested CI posture:

- not part of the fastest Rust unit-test path
- run pre-release, nightly, or on demand
- optionally run on PRs that touch skills, capabilities, or workflow docs

## Risks And Decisions

### Live Model Nondeterminism

If the harness uses a live model, do not assert byte-for-byte prose. Assert durable structures: files, YAML fields, status transitions, commit boundaries, and validation results.

### Cost And Runtime

Full integration execution may be slow. Keep a cheap CLI test (`specify-cli/tests/cross_repo.rs`) for every PR and a heavier integration harness for release confidence.

### Capability Scope

The first full run should use one narrow feature and three capabilities only:

- `contracts@v1` for shared API contract
- `omnia@v1` for backend implementation
- `vectis@v1` for mobile implementation

Do not include RT replay or client-facing skills in RM-01; those can be follow-up suites.

### What Counts As Passing

Passing should mean:

- the workflow skills can drive the CLI substrate
- the capability pipelines produce valid artifacts
- the specialist skills can generate expected project outputs
- the multi-repo Git/forge lifecycle completes without manual filesystem repair

It should not require exact generated prose or exact implementation code beyond capability-owned structural checks.

## Relationship To RM-01 And RM-14

This handoff targets the missing integration layer for RM-01.

Keep RM-01 scoped to the happy path. Do not add recovery cases here unless they are required to make the happy path reliable.

RM-14 should extend the same harness family to:

- blocked entries
- failed phase outcomes
- interrupted driver runs
- stale workspace clones
- dirty unrelated work
- partial push/finalize states

## Suggested Next Prompt For Another Agent

```text
Implement the integration RM-01 acceptance harness described in
rfcs/rm-01-integration-harness.md.

Start with Phase 1 and Phase 2 only:
- create a runner that can invoke /change:plan against a temp multi-repo hub
- use local fixture repositories and fake remotes
- assert the generated plan has one contract slice and two routed implementation slices
- do not implement execute/build/finalize yet

Do not modify the existing Rust CLI acceptance test except to reuse helper ideas.
Keep generated run outputs temporary unless promoting a small fixture/golden is necessary.
```
