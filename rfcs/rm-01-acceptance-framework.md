# RM-01 Acceptance Framework Design

> Status: Draft
> Purpose: define a consistent, staged framework for testing Specify framework behavior in the `specify` repo, including the RM-01 outside-in multi-repo harness.

## Abstract

Specify already has strong deterministic coverage in `specify-cli`: Rust tests exercise CLI state machines, JSON output shapes, local workspace materialisation, fake forge handoff, declared WASI tools, and cross-repo finalization. The `specify` repo has a different gap. Its most important behavior lives in markdown skills, capability briefs, fixtures, transcripts, and manual scenario documents interpreted by an agent.

This design introduces a layered acceptance framework for the `specify` repo. The framework keeps the CLI authoritative for lifecycle mechanics, then adds markdown-driven scenario packs and replayable outside-in runs that prove skills and capabilities still compose the CLI correctly.

RM-01 becomes one acceptance suite within that framework: the first realistic cross-repo, multi-slice proof path. It should share the same runner, scenario shape, assertion vocabulary, and evidence capture used by smaller capability suites such as the existing contracts scenarios.

## Context

The current RM-01 handoff describes the missing layer above `specify-cli/tests/cross_repo.rs`: an outside-in harness that starts from user-facing brief/docs and runs the actual workflow skills and capability pipelines. It correctly observes that a normal Rust integration test cannot fully exercise markdown skills unless it either shells out to an agent runtime or replaces agent behavior with deterministic fixtures.

At the same time, the `specify` repo already has three partial testing idioms:

- `make checks` validates static framework structure: links, capability manifests, brief frontmatter, skill metadata, skill references, variables, directives, and plugin consistency.
- Skill-owned fixtures under `plugins/change/skills/*/fixtures/` pin expected plan shapes, transcripts, and behavioral invariants for human review.
- `capabilities/contracts/tests/` contains user-initiated manual regression scenarios for contract generation, including prompts, source inputs, expected contract files, and negative boundary behavior.

These idioms should converge into one framework rather than remain isolated conventions.

## Goals

- Provide one consistent testing vocabulary for framework repo behavior: static checks, scenario packs, replay runs, and outside-in acceptance suites.
- Preserve the CLI/skill boundary: deterministic lifecycle authority remains in `specify-cli`; agent-facing skills are tested by observing whether they drive that substrate correctly.
- Make the existing contracts manual scenarios the first automation target, because they are narrow and already specify inputs plus expected artifact structure.
- Make RM-01 a staged outside-in suite built on the same framework, not a bespoke harness.
- Assert structural invariants rather than exact generated prose.
- Capture enough evidence from every replay to debug drift in skills, briefs, capability manifests, specialist generators, or orchestration prose.
- Keep fast PR checks fast, while allowing heavier acceptance suites to run on demand, nightly, pre-release, or on targeted PRs.

## Non-Goals

- Do not duplicate Rust CLI integration tests in the `specify` repo.
- Do not require exact byte-for-byte generated markdown from live agents.
- Do not make live model behavior a mandatory dependency for every PR check.
- Do not introduce a second lifecycle authority outside the `specify` CLI.
- Do not include RM-14 recovery cases in the first RM-01 happy-path suite.
- Do not use this framework to test generated downstream application logic exhaustively; capability-owned structural checks and downstream build/test commands are sufficient at this layer.

## Testing Layers

### Layer 0: CLI Substrate Tests

Owner: `specify-cli`.

Purpose: prove deterministic command behavior that skills rely on.

Examples:

- lifecycle transitions
- plan validation and `plan next`
- capability and tool resolution
- workspace sync and branch preparation
- merge, archive, residue commit boundaries
- fake forge push/finalize
- stable JSON output shapes

These remain Rust tests. They should run in normal CLI CI and should not know about agent transcripts or markdown scenario packs except where fixtures are useful test input.

### Layer 1: Static Framework Checks

Owner: `specify`.

Purpose: catch structural drift before any agent run is needed.

Current surface:

- `make checks`
- `scripts/checks.ts`
- documented in [Consistency Checks](../docs/contributing/checks.md)

Examples:

- markdown links resolve
- capability manifests validate
- capability brief paths and `needs` graphs are coherent
- skill frontmatter and body constraints are valid
- skill references and directives resolve
- marketplace manifest matches plugins

Future additions may validate scenario-pack manifests, but this layer should remain read-only and deterministic.

### Layer 2: Markdown Scenario Packs

Owner: the capability, skill, or workflow area being tested.

Purpose: express user-facing acceptance scenarios as markdown plus small fixture files.

Scenario packs are the bridge between prose fixtures and automated replay. They should be readable by humans, executable by a runner, and reviewable in ordinary PRs.

The existing contracts scenarios are the seed pattern:

- [Contract Test Scenarios](../capabilities/contracts/tests/README.md)
- prompt text
- optional source files to create
- expected change-local outputs
- boundary or negative expectations
- verifier warnings/failures to inspect

This layer can start as markdown only, then gain optional frontmatter once automation needs machine-readable metadata.

### Layer 3: Replay Runner

Owner: shared framework code in the `specify` repo.

Purpose: execute scenario packs in isolated temp workspaces and assert durable outcomes.

The runner should support multiple backends:

- manual runner instructions for current human-driven validation
- deterministic test-double runner for early automation
- agent runtime runner when programmatic slash-command execution is available
- recorded transcript runner for stable regression of known tool decisions

The runner is not itself the test oracle. It prepares workspaces, invokes the selected backend, captures evidence, and hands final state to assertion modules.

### Layer 4: Outside-In Acceptance Suites

Owner: roadmap item or workflow area.

Purpose: prove full user journeys across multiple skills, capabilities, repositories, and external boundaries.

RM-01 belongs here. It should reuse the same runner and assertion vocabulary as smaller scenario packs, while adding local Git remotes, fake `gh`, project registry setup, workspace routing, branch preparation, push handoff, external merge simulation, and finalization.

## Scenario Pack Shape

The first implementation can keep scenario packs markdown-first. Automation should prefer frontmatter over ad hoc parsing once more than one suite is executable.

Recommended file layout:

```text
<owner>/tests/<scenario-name>/
  scenario.md
  inputs/
  expected/
  fixtures/
```

For existing simple suites, a flat layout such as `capabilities/contracts/tests/describe.md` is acceptable. The runner can initially support both shapes.

Recommended `scenario.md` sections:

```markdown
# Scenario Title

## Intent

What behavior this scenario proves.

## Workspace

Capability, project shape, registry shape, and isolation rules.

## Inputs

Files or source trees the runner must create before invocation.

## Invocation

Slash command or command sequence to run.

## Expected Artifacts

Files, directories, and baseline or change-local paths expected after the run.

## Assertions

Structural checks that define pass/fail.

## Negative Expectations

Boundary behavior that must not occur.

## Cleanup

Whether the runner should drop, archive, or preserve state after completion.
```

Optional frontmatter, once needed:

```yaml
---
id: contracts-describe
owner: contracts
kind: capability
backend: agent
capability: contracts@v1
entrypoint: /spec:define
stages: [define, build]
assertions:
  - files-exist
  - contract-validator-clean
isolation: fresh-project
---
```

The markdown body remains the canonical human-readable description. Frontmatter only carries routing and machine-readable defaults.

## Assertion Model

Assertions should target stable structures:

- files exist or do not exist
- YAML/JSON fields match expected roles
- CLI validation passes
- lifecycle statuses transition legally
- expected capability-owned paths are touched
- forbidden paths remain untouched
- generated code builds or tests when the capability requires it
- branch names and commit boundaries match the workflow contract
- fake forge state reaches the expected PR status

Assertions should not target unstable structures:

- exact generated proposal prose
- exact generated implementation code beyond capability-owned structural contracts
- exact live-agent wording
- ordering of independent explanatory bullets unless ordering is itself the behavior

For live agent runs, role-based matching is preferred over exact names. For example, an RM-01 plan should contain one contract slice and two routed implementation slices; exact slice names can vary if roles, dependencies, and project routing are correct.

## Runner Design

### Responsibilities

The runner prepares and observes the world around a scenario:

- create temp project roots
- install or point to local Specify plugins
- initialise hub or project `.specify/` state through the CLI where possible
- create source documents and fixture repositories
- configure local bare remotes for Git-backed workspace tests
- install fake `gh` and fake SSH where forge behavior is under test
- invoke a backend
- collect stdout, stderr, transcript, tool calls, file tree, relevant JSON command output, and final Git state
- run assertion modules
- write a compact run summary

The runner must not hand-edit lifecycle files to create normal state. It may seed fixtures for deterministic phases when a test phase explicitly declares a stub backend, but normal lifecycle transitions should still go through the CLI.

### Backend Types

#### Manual Backend

The first backend can be documentation-only: a human or agent follows the scenario pack and records results. This is the current contracts harness behavior.

Useful for:

- proving scenario wording
- discovering missing assertions
- validating expensive or nondeterministic flows before automation

#### Deterministic Stub Backend

A local script or runner implementation performs known phase effects without invoking a live skill. This is useful for `/change:execute loop` tests where the goal is to prove routing and plan transitions, not generated artifacts.

Useful for:

- early RM-01 execute coverage
- recovery fixtures
- testing workspace routing without paying model/runtime cost

#### Agent Runtime Backend

A backend invokes slash-command workflows in a temp workspace using a pinned runtime when that becomes available.

Useful for:

- real `/change:plan`
- real `/spec:define`
- real `/spec:build`
- specialist skill delegation

Live runs must assert structural outcomes and capture evidence. They must not compare full transcripts byte-for-byte.

#### Recorded Transcript Backend

A backend replays known tool decisions from a recorded run. This provides cheap regression coverage after a live run has produced a trusted trace.

Useful for:

- guarding orchestration drift
- making release checks cheaper
- isolating failures between runner mechanics and model behavior

Recorded replay should be treated as complementary coverage, not a substitute for periodic live outside-in runs.

## Output Evidence

Every automated run should write evidence to a temp run directory. The directory is kept only on failure by default, with an option to preserve all runs.

Recommended shape:

```text
acceptance-runs/<suite>/<timestamp-or-run-id>/
  summary.md
  scenario.md
  transcript.md
  tool-calls.jsonl
  stdout.log
  stderr.log
  final-tree.txt
  assertions.json
  artifacts/
  failures/
```

RM-01 adds workflow-specific evidence:

```text
acceptance-runs/rm01-cross-repo/<run-id>/
  registry.yaml
  plan.yaml.before-finalize
  workspace-status.json
  push-output.json
  finalize-output.json
  git/
    hub.log
    shop-backend.log
    shop-mobile.log
  fake-gh/
    prs.json
```

Run output should not be committed by default. Small fixture inputs and intentionally promoted goldens may be checked in.

## Directory Strategy

Use owner-local scenarios for narrow tests and shared `acceptance/` infrastructure for the runner.

Recommended final shape:

```text
acceptance/
  README.md
  runner/
  assertions/
  suites/
    rm01-cross-repo/
      README.md
      scenario.md
      inputs/
      expected/

capabilities/contracts/tests/
  README.md
  describe.md
  design.md
  import.md
  source.md
  update.md

plugins/change/skills/plan/fixtures/
plugins/change/skills/execute/fixtures/
```

Owner-local fixtures stay close to the behavior they document. Shared acceptance infrastructure stays in one place so capability suites do not invent incompatible runners.

## CI Posture

The framework should define at least three execution tiers:

```bash
make checks
make acceptance-smoke
make acceptance-cross-repo
```

Recommended meaning:

- `make checks`: current static checks, required on every PR.
- `make acceptance-smoke`: cheap scenario-pack validation, likely contracts define/build and selected recorded replays.
- `make acceptance-cross-repo`: RM-01 or other heavy outside-in suites, run on demand, nightly, pre-release, or on PRs touching workflow skills/capabilities.

Live agent runs should not block every small documentation PR until runtime, cost, and flake behavior are understood.

## Relationship To Existing Roadmap Items

RM-01 remains the first outside-in proof path. This design supplies the reusable framework around it.

RM-07 and RFC-5 continue to own the static framework linter path. Scenario-pack manifest validation can later move into `specify check`, but scenario execution belongs to acceptance runner commands.

RM-14 should extend the same acceptance framework to recovery behavior:

- blocked entries
- failed phase outcomes
- interrupted driver runs
- stale workspace clones
- dirty unrelated work
- partial push/finalize states

RM-16 local structured workflow events can eventually simplify evidence capture. The runner should consume those events when available, but must not depend on hosted telemetry.

## Staged Implementation Plan

### Stage 0: Document The Contract

Goal: make the framework reviewable before writing runner code.

Tasks:

- [ ] Land this design.
- [ ] Add `acceptance/README.md` describing layers, scenario pack conventions, and command tiers.
- [ ] Decide whether scenario metadata starts as markdown sections only or YAML frontmatter.
- [ ] Document how static checks, scenario packs, and outside-in suites differ.

Acceptance criteria:

- Maintainers can classify an existing fixture or test as Layer 0, 1, 2, 3, or 4.
- New capability scenarios have an obvious place to live.
- RM-01 references the shared framework rather than defining its own runner vocabulary.

### Stage 1: Normalize Contracts Scenarios

Goal: turn the existing contracts manual harness into the first conforming scenario pack.

Tasks:

- [ ] Add machine-readable scenario IDs to each contracts test, either through frontmatter or a sidecar manifest.
- [ ] Split source-file setup from invocation instructions where needed.
- [ ] Express expected contract files as assertions.
- [ ] Mark `update.md` explicitly as a boundary/negative scenario.
- [ ] Add a run summary template.

Acceptance criteria:

- A human can run every contracts scenario and fill out the same summary shape.
- A future runner can discover scenario name, capability, required setup, invocation, expected files, and negative expectations without inferring from prose.

### Stage 2: Static Scenario Validation

Goal: extend deterministic checks without invoking agents.

Tasks:

- [ ] Add static validation for scenario IDs, duplicate IDs, and broken scenario-local links.
- [ ] Validate that expected file paths are relative and stay within the project/slice boundary.
- [ ] Validate that declared capability identifiers are syntactically valid.
- [ ] Validate that scenario frontmatter, if adopted, matches a JSON schema.
- [ ] Wire static scenario validation into `make checks` or the future `specify check` port.

Acceptance criteria:

- Broken scenario metadata fails fast.
- Static validation remains read-only and cheap.
- Existing non-automated fixtures are not forced into the scenario schema unless they opt in.

### Stage 3: Manual-To-Executable Contracts Runner

Goal: automate the current contracts scenario loop with minimal runtime assumptions.

Tasks:

- [ ] Create `acceptance/runner` with temp workspace creation and evidence capture.
- [ ] Support a manual or semi-automated backend that prints the next command and records operator-provided results.
- [ ] Add assertion helpers for file existence, forbidden output, and validator clean/warning status.
- [ ] Run the contracts `describe` scenario end to end.
- [ ] Expand to `design`, `import`, `source`, and `update`.

Acceptance criteria:

- `make acceptance-smoke` can run at least one contracts scenario or guide it through a repeatable manual flow.
- The runner writes `summary.md` and `assertions.json`.
- Failure output points at missing files or verifier warnings clearly enough for skill authors to fix drift.

### Stage 4: Deterministic Replay Backend

Goal: support workflow execution tests before full live-agent automation is stable.

Tasks:

- [ ] Define a stub phase protocol for scenarios that want deterministic `/spec:define`, `/spec:build`, and `/spec:merge` outcomes.
- [ ] Reuse CLI verbs for plan transitions and lifecycle operations.
- [ ] Add runner support for local Git remotes and fake `gh` based on the CLI test strategy.
- [ ] Port one `/change:execute` fixture into an executable replay.
- [ ] Capture final plan status, workspace status, and Git state.

Acceptance criteria:

- A replay can drive a seeded plan to `all-done` without live generation.
- The runner proves route selection, branch preparation, transition legality, and evidence capture.
- Stubbed behavior is clearly declared in scenario metadata.

### Stage 5: RM-01 Plan-Level Outside-In Suite

Goal: run real `/change:plan` against a temp multi-repo hub and assert the plan structure.

Tasks:

- [ ] Create `acceptance/suites/rm01-cross-repo/`.
- [ ] Seed a registry-only hub plus `shop-backend` and `shop-mobile` fixture repos.
- [ ] Provide one concise feature brief such as OAuth login or dark mode.
- [ ] Invoke real `/change:plan`.
- [ ] Assert one contract role, one backend role, one mobile role, contract-first dependencies, and correct project routing.
- [ ] Run `specify change plan validate`.

Acceptance criteria:

- The suite proves planning from user docs, not a pre-seeded plan.
- Names may vary, but roles and dependencies are structurally correct.
- No execution/build/finalize is included yet.

### Stage 6: RM-01 Execute With Stubbed Phases

Goal: run real `/change:execute loop` against the RM-01 plan while keeping phase outputs deterministic.

Tasks:

- [ ] Add a stub backend for contract, backend, and mobile phase outcomes.
- [ ] Materialize workspace slots through `specify workspace sync`.
- [ ] Prepare project branches through the CLI.
- [ ] Commit baseline and residue outputs in routed projects where required.
- [ ] Drive the plan to `all-done`.
- [ ] Assert branch names, clean workspaces, baseline/residue commit split, and final plan status.

Acceptance criteria:

- The execution driver path is covered outside-in enough to catch skill orchestration drift.
- Capability generation remains stubbed and separately tested.
- The suite reuses runner evidence and assertions rather than bespoke scripts.

### Stage 7: Real Define And Merge

Goal: replace stubbed define/merge behavior with real capability brief pipelines.

Tasks:

- [ ] Run real `/spec:define` for the contract, backend, and mobile slices.
- [ ] Keep build output stubbed if needed.
- [ ] Run real `/spec:merge`.
- [ ] Assert expected slice artifacts, baseline promotion, archive movement, and merge/residue boundaries.
- [ ] Confirm implementation slices read baseline contract context rather than authoring new interface shapes inline.

Acceptance criteria:

- Capability define pipelines generate required artifacts.
- Merge behavior stays CLI-authoritative.
- The cross-repo commit model remains valid.

### Stage 8: Full Capability Build

Goal: exercise real specialist skill delegation one capability at a time.

Tasks:

- [ ] Enable real contracts build first.
- [ ] Assert generated contract files and verifier output.
- [ ] Enable real Omnia build with minimal crate/test expectations.
- [ ] Enable real Vectis build with minimal composition/design/scaffold expectations.
- [ ] Keep assertions structural and capability-owned.
- [ ] Record runtime/cost/flake behavior.

Acceptance criteria:

- Contracts author/import/verify behavior works in the RM-01 path.
- Omnia and Vectis generate expected project-owned outputs.
- The full flow survives merge, push, external merge simulation, and finalize.

### Stage 9: CI Integration And Promotion

Goal: make the suite useful without destabilizing ordinary PRs.

Tasks:

- [ ] Add `make acceptance-smoke`.
- [ ] Add `make acceptance-cross-repo`.
- [ ] Preserve failure artifacts in CI.
- [ ] Define when live-agent runs are required.
- [ ] Add docs for regenerating recorded replays or promoted goldens.
- [ ] Track average runtime and failure categories for the first several releases.

Acceptance criteria:

- Fast checks remain fast.
- Heavy outside-in coverage is available before releases and on relevant workflow/capability changes.
- Failures produce enough evidence to identify whether drift is in CLI substrate, skill orchestration, capability briefs, specialist generation, or runner infrastructure.

## Open Decisions

### Runner Implementation Language

Options:

- TypeScript/Deno, matching current `scripts/checks.ts`
- Rust, sharing future `specify check` parsers and CLI test helpers
- shell plus small assertion tools, lowest ceremony but weakest structure

Recommendation: start with the smallest viable runner, but keep scenario metadata language-neutral so it can move to Rust later if RFC-5/RM-07 consolidation makes that attractive.

### Agent Runtime

Options:

- Cursor SDK runner
- Cursor/agent CLI session launcher
- recorded transcript replay
- deterministic stubs only

Recommendation: design the runner around a backend interface. Do not block Stage 1-4 on live-agent automation.

### Scenario Metadata

Options:

- markdown sections only
- YAML frontmatter per scenario
- sidecar `scenario.yaml`

Recommendation: use markdown sections for current manual scenarios, then add YAML frontmatter when Stage 2 static validation starts. Avoid sidecars until scenarios need large structured data.

### Golden Outputs

Options:

- no goldens, structural assertions only
- selected JSON goldens for CLI command outputs
- recorded replay goldens for tool calls

Recommendation: avoid prose goldens. Allow JSON/tool-call goldens only when the output is intentionally stable and cheap to review.

## Risks

### Live Model Nondeterminism

Mitigation: assert role structure, files, validation results, and lifecycle state; do not assert exact prose. Use recorded replay as complementary regression coverage.

### Runtime And Cost

Mitigation: keep Layer 1 static checks as the default PR gate. Run live Layer 4 suites on demand, nightly, pre-release, or on targeted changes.

### Runner Becoming A Second Product

Mitigation: keep the runner small. It prepares workspaces, invokes backends, captures evidence, and runs assertions. It must not reimplement Specify lifecycle logic.

### Fixture Drift

Mitigation: require owner-local scenario packs to live beside the skill or capability they test. Static scenario validation should catch broken references and stale expected paths.

### Ambiguous Failures

Mitigation: failure summaries must classify likely fault domains: CLI substrate, skill orchestration, capability brief, specialist generation, runner setup, external fake boundary, or live-agent nondeterminism.

## Recommended Next Change

Start with Stage 0 and Stage 1:

1. Add `acceptance/README.md` with the layer model and scenario conventions.
2. Normalize `capabilities/contracts/tests/` into scenario-pack format while preserving the existing manual workflow.
3. Add a tiny static scenario validator only after the scenario shape has survived one review cycle.

Do not begin RM-01 execution automation until contracts has proven the scenario-pack contract. The multi-repo suite should inherit that vocabulary instead of discovering it under higher complexity.
