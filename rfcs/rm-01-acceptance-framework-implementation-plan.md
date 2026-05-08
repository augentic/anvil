# RM-01 Acceptance Framework Implementation Plan

> Status: Draft
> Source: [RM-01 Acceptance Framework Design](rm-01-acceptance-framework.md), [RM-01 Outside-In Acceptance Harness Handoff](rm-01-outside-in-harness.md), and [Specify Roadmap](roadmap.md).
> Purpose: break the acceptance framework into dependency-ordered changes that can be implemented by small, independent subagents.

## Planning Principles

- Keep the CLI authoritative. Runner code may call `specify` commands, but it must not mutate lifecycle files directly.
- Build from the smallest stable contract outward: documentation -> scenario metadata -> static validation -> runner -> narrow scenarios -> RM-01 outside-in suite.
- Prefer structural assertions over generated-prose goldens.
- Keep each change small enough for one subagent to complete without reading the whole repo.
- Make the contracts scenarios the first executable proof before adding RM-01 complexity.
- Keep heavy or live-agent acceptance out of default `make checks` until runtime and flake behavior are understood.

## Dependency Map

```text
C01 framework docs
  -> C02 contracts scenario normalization
    -> C03 static scenario validation
    -> C04 runner skeleton
      -> C05 contracts smoke runner
      -> C07 Git and fake forge support
        -> C08 deterministic stub backend
          -> C10 RM-01 execute with stubbed phases
            -> C11 RM-01 push/finalize
              -> C12 real define/merge
                -> C13 real contracts build
                  -> C14a real Omnia build
                  -> C14b real Vectis build

C01 framework docs
  -> C06 RM-01 scenario pack and fixture brief
    -> C07 Git and fake forge support
      -> C09 RM-01 plan-level outside-in suite
        -> C10 RM-01 execute with stubbed phases

C09 RM-01 plan-level outside-in suite
  -> C15 recorded transcript backend

C05 contracts smoke runner
C11 RM-01 push/finalize
C14a real Omnia build
C14b real Vectis build
  -> C16 CI and promotion posture
```

## Parallel Execution Waves

### Wave 0: Framework Contract

Run sequentially:

1. C01 Framework docs and directory contract.
2. C02 Contracts scenario normalization.

Why: every later subagent needs the same scenario shape and vocabulary.

### Wave 1: Static Validation And Runner Base

Run in parallel after C02:

- C03 Static scenario validation.
- C04 Runner skeleton and evidence model.
- C06 RM-01 scenario pack and fixture brief.

Why: these touch different surfaces. C03 owns `scripts/checks.ts`, C04 owns acceptance runtime infrastructure, and C06 owns suite content.

### Wave 2: First Executable Coverage

Run in parallel after C04:

- C05 Contracts smoke runner, after C02 and C04.
- C07 Git and fake forge support, after C04 and C06.

Why: contracts smoke proves the narrow suite while Git/fake forge support prepares the cross-repo lane.

### Wave 3: RM-01 Substrate

Run in parallel after C07:

- C08 Deterministic stub backend.
- C09 RM-01 plan-level outside-in suite.

Why: C08 proves execution without live generation; C09 proves real planning from fixture docs. They converge at C10.

### Wave 4: RM-01 Happy Path

Run sequentially:

1. C10 RM-01 execute with stubbed phases.
2. C11 RM-01 push/finalize.
3. C12 Real define/merge.
4. C13 Real contracts build.

Why: each step depends on the prior workflow state becoming reliable.

### Wave 5: Capability Expansion And Regression Cost Control

Run in parallel after C13:

- C14a Real Omnia build.
- C14b Real Vectis build.
- C15 Recorded transcript backend, after C09 or any later live run with a trusted trace.

Then run C16 after the required suite tier is stable.

## Changes

### C01: Framework Docs And Directory Contract

Goal: make the acceptance framework contract concrete before runner code lands.

Suggested scope:

- Add `acceptance/README.md`.
- Add lightweight `acceptance/runner/README.md`, `acceptance/assertions/README.md`, and `acceptance/suites/README.md` if useful.
- Document the four repo-owned layers from the RFC: static checks, scenario packs, replay runner, and outside-in suites.
- Document the intended command tiers: `make checks`, `make acceptance-smoke`, and `make acceptance-cross-repo`.
- Document run evidence policy: temporary by default, preserved on failure or explicit opt-in.

Out of scope:

- No runner implementation.
- No scenario validation code.
- No CI wiring.

Acceptance criteria:

- A maintainer can tell where to add a new capability scenario.
- The docs distinguish owner-local scenarios from shared acceptance infrastructure.
- The docs state that lifecycle state must be created through the CLI, not direct file edits.
- `make checks` still passes.

Subagent prompt:

```text
Implement C01 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Add acceptance framework documentation only. Do not implement runner code or checks.
Keep links valid under make checks, and avoid linking to files that do not exist yet.
```

### C02: Contracts Scenario Normalization

Goal: make the existing contracts manual scenarios conform to the first scenario-pack shape.

Suggested scope:

- Update `capabilities/contracts/tests/README.md` with the scenario-pack convention.
- Add consistent sections to each scenario: Intent, Workspace, Inputs, Invocation, Expected Artifacts, Assertions, Negative Expectations, Cleanup.
- Add scenario IDs in markdown, either as a visible field or YAML frontmatter.
- Keep the existing human-readable prompts intact.
- Keep the current flat layout (`capabilities/contracts/tests/<scenario>.md`) as documented in `acceptance/suites/README.md` §Scenario Discovery; do not migrate to directory form unless a scenario needs `inputs/` or `expected/` siblings.
- Add a reusable run summary template under `capabilities/contracts/tests/`, or document it in the README.
- Mark `update.md` explicitly as a boundary or negative scenario.

Out of scope:

- No static validator yet.
- No runner code.
- No generated contract artifacts.

Acceptance criteria:

- A human can run every contracts scenario and fill out the same summary shape.
- A future runner can discover the scenario ID, capability, invocation, expected files, and negative expectations without inferring from prose.
- Existing manual intent is preserved.
- `make checks` still passes.

Subagent prompt:

```text
Implement C02 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Normalize capabilities/contracts/tests into scenario-pack markdown while preserving the existing manual prompts.
Do not add automation yet.
```

### C03: Static Scenario Validation

Goal: catch broken scenario metadata without invoking agents or runner code.

Suggested scope:

- Extend `scripts/checks.ts` with opt-in scenario-pack validation.
- Discover scenario files using the four-location convention documented in `acceptance/README.md` §Scenario Discovery (shared suite, owner-local flat, owner-local directory, skill-fixture). The discovery roots stay one source of truth in the framework docs.
- Validate scenario IDs are present and unique across opted-in scenario files.
- Validate scenario-local relative links resolve.
- Validate expected artifact paths are relative and do not escape the scenario workspace.
- Validate declared capability identifiers are syntactically valid (`^[a-z][a-z0-9-]*@v\d+$`).
- If YAML frontmatter is adopted in C02, validate it against a small schema. C02 has landed; the frontmatter shape it adopted is documented in `capabilities/contracts/tests/README.md` (id, owner, kind, capability, backend, entrypoint, stages, isolation, plus optional authorship-mode, assertions, expected-artifacts, negative-expectations). Suggested home: `.cursor/schemas/scenario.schema.json`, validated via the same Ajv2020 path used for `skill.schema.json`.
- Treat scenario discovery as opt-in: a markdown file under one of the four `acceptance/README.md` §Scenario Discovery roots is validated only if it carries scenario frontmatter; prose-only fixtures are skipped.
- Verify the visible `Scenario ID:` body line (when present) matches frontmatter `id`.
- Require at least one `negative-expectations` entry when `kind: capability-boundary`.
- Treat `kind` as an open enum with `capability` and `capability-boundary` validated now; `suite` and `skill` will be added when C06 and any skill-fixture promotion opt in.
- Treat `stages` as a contiguous prefix of `[define, build, merge, drop]` starting at `define`.
- Update `docs/contributing/checks.md` to document the new checks.

Out of scope:

- No runner execution.
- No validation of generated files.
- No requirement that unrelated fixtures adopt the schema.

Acceptance criteria:

- Malformed opted-in scenario metadata fails `make checks` with actionable messages.
- Non-scenario markdown is not forced into the schema.
- Static validation remains read-only and fast.

Subagent prompt:

```text
Implement C03 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Add opt-in scenario metadata validation to scripts/checks.ts and document it.
Do not implement acceptance runner execution.
```

### C04: Runner Skeleton And Evidence Model

Goal: create a small runner foundation that discovers scenarios, creates isolated run directories, and records evidence.

Suggested scope:

- Add a Deno TypeScript runner entrypoint, for example `acceptance/runner/main.ts`.
- Add scenario discovery for the normalized contracts scenario shape.
- Create temp workspaces under the OS temp directory by default.
- Write evidence files such as `summary.md`, `scenario.md`, `stdout.log`, `stderr.log`, `assertions.json`, and `final-tree.txt`. Reserve optional `transcript.md` (agent backend) and `tool-calls.jsonl` (recorded backend) names so later backends do not re-invent file names.
- Add `--suite`, `--scenario`, and `--preserve` flags if they keep the first runner usable.
- Add a no-op or manual backend that records the scenario and prints the next human action.
- On failure, emit a fault-domain hint from the taxonomy in `acceptance/runner/README.md` (CLI substrate, skill orchestration, capability brief, specialist generation, runner setup, external fake boundary, live-agent nondeterminism). The hint may be `runner-setup` or `unknown` for the skeleton; the field must exist so later assertion modules can populate it without inventing vocabulary.

Out of scope:

- No live agent invocation.
- No fake forge support.
- No build or merge execution.
- No CI target unless needed only for local smoke.

Acceptance criteria:

- The runner can discover at least one contracts scenario.
- A dry or manual run creates a run directory with a summary and assertion placeholder.
- Evidence is not written into committed paths unless `--preserve` or a fixture-promotion path is explicit.
- `make checks` still passes.

Subagent prompt:

```text
Implement C04 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Create the minimal Deno-based acceptance runner skeleton and evidence output.
Do not invoke live agents, fake gh, or real workflow phases yet.
```

### C05: Contracts Smoke Runner

Goal: automate or semi-automate one narrow contracts scenario using the shared runner.

Suggested scope:

- Add assertion helpers for file existence, forbidden outputs, and verifier status placeholders. Helpers live under `acceptance/assertions/` (separate from `acceptance/runner/`) so they can be reused without pulling in backend code.
- Decide explicitly where assertion dispatch happens in the runner lifecycle. The Backend interface from C04 is `prepare → invoke → teardown`. Recommended addition: a runner-owned `assertions` stage that runs after `invoke` and before `teardown`, consuming `BackendResult.assertions` plus the on-disk evidence. Document the choice in `acceptance/runner/backends/README.md`.
- Preserve the `pending-operator` verdict introduced by C04 for non-asserting manual runs. Only assertion-helper outcomes upgrade a run to `passed` / `failed`.
- Teach the manual backend to guide the `describe.md` scenario end to end.
- Optionally add a deterministic backend that materializes known expected files for runner-only testing.
- Add `make acceptance-smoke` if the target can run without live model dependency.
- Expand from `describe.md` to other contracts scenarios only if the change remains small.
- Consider tightening: passing `--backend X` for a scenario whose frontmatter declares a different backend should be a hard error, not a warning. C04 currently warns. Make the policy explicit.

Out of scope:

- No RM-01 suite.
- No cross-repo Git setup.
- No live specialist build requirement unless already supported locally.

Acceptance criteria:

- `make acceptance-smoke` or an equivalent runner command exercises at least one contracts scenario repeatably.
- The run writes `summary.md` and `assertions.json`.
- Failure output points at missing expected files or verifier findings.
- The smoke command does not destabilize default PR checks.

Subagent prompt:

```text
Implement C05 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Use the runner skeleton to exercise the contracts describe scenario as the first acceptance smoke path.
Keep live model usage optional or manual.
```

### C06: RM-01 Scenario Pack And Fixture Brief

Goal: add the RM-01 suite content without requiring runner support yet.

Suggested scope:

- Add `acceptance/suites/rm01-cross-repo/scenario.md`.
- Add a concise fixture brief, for example OAuth login or dark mode.
- Declare the platform hub plus `shop-backend` and `shop-mobile` project roles.
- Declare expected plan roles: one contract slice, one backend implementation slice, one mobile implementation slice.
- Declare structural assertions for contract-first dependencies and project routing.
- Add expected evidence inventory for registry, plan, workspace status, push, finalize, Git logs, and fake forge state.

Out of scope:

- No runner backend implementation.
- No fixture repository bootstrap code.
- No live `/change:plan` invocation.

Acceptance criteria:

- The scenario pack is readable as a manual acceptance scenario.
- The scenario is metadata-ready for runner discovery.
- It references the shared framework vocabulary from C01.
- It keeps RM-01 happy-path only; RM-14 recovery cases are not included.

Subagent prompt:

```text
Implement C06 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Add the RM-01 cross-repo scenario pack and fixture brief only.
Do not implement the runner backend or execute the scenario.
```

### C07: Git, Registry, Workspace, And Fake Forge Support

Goal: give the runner reusable setup primitives for local multi-repo suites.

Suggested scope:

- Add helper code to create a temp registry-only hub via `specify init --hub`.
- Add helper code to initialize fixture project repos with local bare remotes.
- Add helper code to register projects through `specify registry add`.
- Add helper code to call `specify workspace sync` and inspect `specify workspace status --format json` if available.
- Add fake `gh` support modeled on the CLI cross-repo test strategy at `specify-cli/tests/cross_repo.rs`. Match the on-disk PR-state file format exactly: `gh-state/<repo>.pr` with five pipe-separated fields `number|state|merged|branch|url`. RM-11 idempotency assertions depend on flipping field 2 → `MERGED` and field 3 → `true` while preserving 1, 4, 5.
- Make the per-repo PR number policy a runner config (the CLI test hard-codes `shop-backend → 41`, `shop-mobile → 18`); RM-14 suites need to register their own.
- Expose a single `setup-hub(name, projects[])` primitive that produces hub state + bare remotes + seeded source repos together; do not split the consistency invariants across separate primitives.
- Land the four `setup-*` assertion ids reserved by C06 (`setup-hub-project-yaml-has-hub-true-and-no-capability`, `setup-registry-has-two-entries`, `setup-registry-entries-have-non-empty-descriptions`, `setup-registry-validate-clean`) as part of C07's helper exit contract — these are "setup is done" gates, not "plan is correct" gates, and missing them makes downstream failures hard to attribute.
- Capture hub and project Git logs into evidence.
- Workspace / registry / fake-`gh` helpers extend the runner's helper modules (`acceptance/runner/workspace.ts`, `acceptance/runner/<helper>.ts`); they do not extend the `Backend` interface from C04.

Out of scope:

- No real `/change:plan` assertions yet.
- No execute loop.
- No full forge abstraction beyond the local fake needed for RM-01.

Acceptance criteria:

- A runner setup command can create a temp hub and two registered project repos.
- Local bare remotes and fake `gh` state are captured in evidence.
- Setup uses CLI commands for Specify state.
- Failure leaves enough logs to distinguish Git setup issues from Specify behavior.

Subagent prompt:

```text
Implement C07 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Add reusable runner support for temp hubs, local fixture repos, workspace sync, and fake gh.
Keep it independent from live /change:plan execution.
```

### C08: Deterministic Stub Backend

Goal: support workflow replay without live generation so execution mechanics can be tested first.

Suggested scope:

- Define a stub backend protocol in scenario metadata. **Distinct from C05's `fixture` backend:** `fixture` materializes one scenario's expected-artifact set into the workspace for runner-plumbing smoke tests; `stub` drives lifecycle phase outcomes through `specify` CLI commands and writes minimal `proposal.md`/`spec.md`/`tasks.md` skeletons to satisfy phase preconditions. They are not substitutes; both names live in `BackendName` together.
- Reuse C07's `GitEnv` (from `acceptance/runner/git.ts`) and `runSpecify` / `runSpecifyJson` (from `acceptance/runner/specify-cli.ts`) for every CLI invocation. Do not invent a parallel backend env.
- For multi-repo scenarios, accept a `SetupHubResult` (from C07's `acceptance/runner/hub.ts`) on the backend and route lifecycle calls into the hub's `env`.
- Support deterministic effects for define, build, and merge phases.
- Ensure normal lifecycle transitions still go through `specify` CLI commands. The stub does NOT hand-edit `.specify/` lifecycle state; it shells out to `specify slice {create, transition, merge}` etc., and only writes the artifact bodies the lifecycle phase would otherwise generate via an agent.
- Add fixture outputs only where the scenario explicitly declares stubbed phases.
- Record which phases were stubbed in `summary.md` and `assertions.json`.

Out of scope:

- No live specialist generation.
- No prose goldens.
- No hidden mutation of `.specify` lifecycle files.

Acceptance criteria:

- A seeded or generated plan can be driven through deterministic phase results.
- Stubbed behavior is visible in evidence.
- The backend is reusable outside RM-01.
- The runner remains able to fail when route selection or transition legality is wrong.

Subagent prompt:

```text
Implement C08 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Add a deterministic stub backend for acceptance scenarios while preserving CLI-authoritative lifecycle transitions.
Do not add live generation.
```

### C09: RM-01 Plan-Level Outside-In Suite

Goal: run real `/change:plan` from fixture docs and assert the resulting plan structure.

Suggested scope:

- Use the RM-01 scenario pack from C06 and setup helpers from C07.
- Run C07's setup primitives first; register the four `setup-*` assertion handlers from `acceptance/assertions/setup.ts` via C05's dispatch table BEFORE any plan-* assertion. Setup failures must short-circuit the plan-level assertions to `skip` so failure attribution stays clean.
- Promote `RunContext` to optionally carry a `setup?: SetupHubResult` field (or pass setup outputs through a documented sidecar) so suite assertions can read `ctx.run.setup.hubDir` / `ctx.run.setup.env` without a per-suite shim.
- Ingest `acceptance/suites/rm01-cross-repo/expected/plan-roles.md` directly for the role-based assertion rules. Do NOT re-extract them from `scenario.md` prose; the rules must have one source of truth.
- Resolve the contract entry's id once via `runSpecifyJson` (`specify --format json change plan status`) and reuse it across rules; do not re-parse `plan.yaml` per assertion.
- Add a soft "extra-entry" warning policy: if the planner returns more entries than expected (e.g. mobile split into iOS + Android), record `live-agent-nondeterminism` rather than `fail`. Role-based assertions still pass.
- Add `acceptance/assertions/yaml.ts` (`assertYamlField(id, path, jsonPointer, expected)`) following the C05 helper pattern so plan/registry field comparisons are reusable.
- Invoke real `/change:plan` through the selected runner backend.
- Assert `plan.yaml` exists and `specify change plan validate` passes.
- Assert one contract role, one backend role, one mobile role.
- Assert implementation entries depend on the contract entry.
- Assert backend and mobile entries are routed to the expected projects.
- Preserve plan and registry evidence via `collectEvidence({ workspaceStatusJson })` from C07's `acceptance/runner/evidence-collectors.ts` so the C06 inventory paths populate without extra glue.

Out of scope:

- No execution/build/merge/finalize.
- No exact generated slice-name requirement.
- No live build or specialist output checks.

Acceptance criteria:

- Planning starts from the user-facing fixture brief, not a pre-seeded plan.
- Names may vary, but roles, dependencies, and routing are structurally correct.
- Failures identify whether the issue is setup, planning, validation, or assertion matching.

Subagent prompt:

```text
Implement C09 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Run the RM-01 fixture through real /change:plan and add structural plan assertions only.
Do not implement execute, build, push, or finalize.
```

### C10: RM-01 Execute With Stubbed Phases

Goal: run real `/change:execute loop` against the RM-01 plan while keeping phase outputs deterministic.

Suggested scope:

- Use the plan produced by C09's `scripted-plan` backend (i.e. start from the SAME setup + plan creation path C09 already exercises). Do not reimplement plan creation; reuse `ScriptedPlanBackend.prepare/invoke` or extract their guts into a shared helper.
- Configure stubbed contract, backend, and mobile phase outcomes. Reuse C08's `StubBackend` action sequence (`slice transition` + optional fixture copy + `slice merge run`) per entry, but parameterized by entry name and routed project. Recommend extending `StubBackend` with a `driveSlice(sliceName, opts)` method that the new C10 driver calls — the loop driver lives in C10; the stub stays a passive lifecycle executor.
- Pass `SetupHubResult.env` and `runSpecify` from C07 into the stub backend for multi-repo (`StubBackend(opts: { env?: GitEnv; hub?: SetupHubResult })`); do not synthesize a parallel `GitEnv`.
- Materialize workspace slots through `specify workspace sync`.
- Prepare routed project branches through the CLI (`specify workspace prepare-branch` per Layer 0 reference).
- Drive `/change:execute loop` until `all-done`. The "loop driver" is the new piece in C10: it picks `specify change plan next`, prepares the entry's routed project slot on `specify/<change-name>`, calls `StubBackend.driveSlice(...)`, transitions the entry, repeats. The actual `/change:execute loop` slash-command is deferred to a real agent backend; C10 ships the deterministic loop equivalent and documents the boundary the same way C09 documented `scripted-plan` ↔ real-agent.
- Add new assertion ids reserved by C06 in `expected/plan-roles.md`: `branch-prepared`, `baseline-merge-commit-clean`, `residue-commit-non-empty`, `workspace-clean-before-push`. Wire them through C05's dispatch behind the `if (ctx.setup && ctx.specifyBin)` guard.
- Assert final plan status (`all-done`) and per-entry transition legality (each entry reaches `done`, no `failed`/`blocked`).
- Assert routed entries ran under the expected workspace projects (the routed slice's `.specify/slices/...` dir lives under `.specify/workspace/<project>/.specify/slices/...`).

Out of scope:

- No real define/build/merge artifacts.
- No push/finalize yet.
- No recovery paths.

Acceptance criteria:

- The execution driver path is covered outside-in enough to catch skill orchestration drift.
- The plan reaches `all-done` with stubbed phases.
- Branch names use `specify/<change-name>`.
- Workspaces are clean after the stubbed execution path.

Subagent prompt:

```text
Implement C10 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Run RM-01 through /change:execute loop using the deterministic stub backend.
Do not replace stubbed phases with live define/build/merge yet.
```

### C11: RM-01 Push And Finalize

Goal: complete the RM-01 landing path with fake forge handoff and finalize.

Suggested scope:

- Add a new `ScriptedFinalizeBackend` (or `ScriptedPushFinalizeBackend`) following the C10 composition pattern: import `prepareScriptedHub` + `runPlanCreationSequence` from `scripted-shared.ts`; instantiate `ScriptedExecuteBackend` (or call its loop driver as a helper) for the execute phase; layer push/finalize on top. Do NOT extend `ScriptedExecuteBackend` in place.
- Add a `FinalizeState` interface on `RunContext` (`{ pushOutputJson, prNumbers, finalizeOutputJson }`) so `push-*` / `finalize-*` handlers gate themselves the same way `execute-*` handlers gate on `ctx.executeState`.
- Run `specify workspace push` after C10 execution.
- Capture push JSON, fake `gh` PR state, and project Git logs via `collectEvidence` from C07 (no new collector entrypoint required — pass `planYamlBeforeFinalize` and the new push/finalize JSON paths).
- Mark fake PRs merged externally via `markPrMerged` from C07's `acceptance/runner/fake-gh.ts`. The PR-state file format (`number|state|merged|branch|url`) is load-bearing in two places: do not change field count without updating both `acceptance/runner/fake-gh.ts` and any setup-* assertion that parses it.
- Add new assertion ids reserved by C06 in `expected/plan-roles.md`: `push-opens-pr-per-project`, `finalize-archives-plan`, `finalize-second-call-returns-plan-not-found`. Pin `push-output.json` and `finalize-output.json` JSON-shape assertions as `cli-substrate` fault-domain (so a regression in `specify --format json workspace push` shape doesn't get mis-attributed to the fake-`gh` boundary).
- Run `specify change finalize`.
- Assert finalization observes merged projects and archives the plan.
- Assert a second finalize returns the expected plan-not-found behavior if the CLI surface supports it (idempotency).
- Optionally exercise the `finalize-runs-before-prs-merged` negative expectation: call `finalize` while PRs are still OPEN and assert non-zero exit + clear error. If the CLI doesn't yet refuse, log a `cli-substrate` finding for `specify-cli` follow-up rather than failing the suite.

Out of scope:

- No real build outputs.
- No multi-forge abstraction.
- No RM-14 recovery cases.

Acceptance criteria:

- The happy-path RM-01 flow reaches finalized state under fake forge conditions.
- Push and finalize evidence is written in the RM-01 run directory.
- Baseline/residue commit boundaries are asserted if the stub backend produces both categories.

Subagent prompt:

```text
Implement C11 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Extend the stubbed RM-01 suite through workspace push, fake external merge, and change finalize.
Keep the scope to the happy path.
```

### C12: Real Define And Merge

Goal: replace stubbed define and merge behavior with real capability brief pipelines while keeping build outputs controllable.

Suggested scope:

- Promote the per-slice phase outcome producer to a `PhaseDriver` interface (refactor `StubBackend.driveSlice` into one implementation of `PhaseDriver`). Both `ScriptedExecuteBackend` and `ScriptedFinalizeBackend` accept a `phaseDriver: PhaseDriver` constructor option that defaults to the stub. Move `RESIDUE_PATHS` into the shared module so all backends agree.
- Add a new `AgentBackend` (or `agent-manual` backend) under `acceptance/runner/backends/agent.ts` that uses the same composition pattern as `scripted-execute`/`scripted-finalize` but plugs in an `AgentPhaseDriver` instead of the stub driver. Two implementation options:
  - **(A) Cursor SDK driver:** use `@cursor/sdk` (see `~/.cursor/skills-cursor/sdk/SKILL.md`) to invoke `/spec:define` programmatically per slice, capturing tool calls into `tool-calls.jsonl` and the agent transcript into `transcript.md` (filenames already reserved on `RunPaths`).
  - **(B) Agent-manual driver:** print the `/spec:define <slice>` prompt and pause for the operator to run it in their Cursor session; resume after operator confirms (file marker, stdin prompt, or `--operator-results` JSON path). Falls back to operator-driven execution when SDK is unavailable.
  - Recommend implementing (B) first as a reliable fallback, then optionally adding (A) as the higher-fidelity automated path. Both share the `AgentPhaseDriver` interface; only the inside differs. Skip-gracefully when neither is configured.
- Run real `/spec:define` for the contract, backend, and mobile slices via the `agent` backend (or operator-driven manual path).
- Keep build output stubbed if needed (the `phaseDriver` returns to writing residue stubs after `/spec:define` produces real artifacts; build is a separate phase the driver controls).
- Run real `/spec:merge` (CLI verb `specify slice merge run` is already CLI-authoritative — no agent runtime needed for merge itself; baseline promotion + archive move happens through the CLI).
- Add new assertion ids in `acceptance/assertions/define.ts`:
  - `slice-has-proposal` (per slice)
  - `slice-has-spec` (per slice)
  - `slice-has-design-when-required` (per slice; "required" determined by capability brief — for contracts/omnia/vectis this is per-capability policy)
  - `slice-has-tasks` (per slice)
  - `slice-baseline-promoted` (post-merge, per slice — `.specify/specs/<slice>/` files exist in baseline)
  - `slice-archived` (post-merge, per slice — `.specify/archive/<slice>/` exists)
  - `implementation-slice-reads-baseline-contract` (the implementation slices' `design.md` references `contracts/` baseline files rather than authoring new contract YAML inline; check via `grep` or YAML walk)
- Assert required slice artifacts: `proposal.md`, specs, `design.md` when required, and `tasks.md`.
- Assert baseline promotion and archive movement.
- Assert implementation slices read baseline contract context instead of authoring new interface shapes inline.
- Add `make acceptance-cross-repo-define-smoke` (operator-friendly: skip with clear message when neither SDK nor `--operator-results` available; succeed when an operator has completed the manual path).
- Document the boundary in `acceptance/runner/backends/README.md`: the `agent` backend proves real `/spec:define` works; the deterministic backends keep proving CLI lifecycle and assertion plumbing for cheap CI runs.

Out of scope:

- No full real build yet.
- No exact prose comparisons.
- No recovery cases.

Acceptance criteria:

- Capability define pipelines generate required artifacts in the RM-01 route.
- Merge remains CLI-authoritative.
- The cross-repo commit model still holds.

Subagent prompt:

```text
Implement C12 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Replace stubbed define and merge with real /spec:define and /spec:merge in the RM-01 suite.
Keep build behavior stubbed unless it is already reliable.
```

### C13: Real Contracts Build

Goal: make the contract slice fully real before implementation capabilities consume it.

Suggested scope:

- Build on the C12 `PhaseDriver` interface. The contract slice's build phase needs a different driver than backend/mobile slices. Two options:
  - **(A) Per-slice phase driver dispatch:** add a `phaseDriverFor(slice): PhaseDriver` callback on `ScriptedExecuteBackend`/`ScriptedFinalizeBackend` so the loop driver can pick a different driver per entry. The contract slice gets `ContractsBuildDriver`; backend/mobile slices keep the stub driver.
  - **(B) Composite driver:** a single `ContractFirstPhaseDriver` that dispatches internally based on `opts.sliceName === SLICE_CONTRACT`. Simpler but less general.
  - Recommend **(A)** — per-slice dispatch — because C14a/C14b will want the same pattern for backend/mobile.
- Add `acceptance/runner/backends/contracts-build-driver.ts` implementing `PhaseDriver`. Two implementation modes:
  - **Mode (i) Scripted contract build:** the driver writes deterministic contract YAML files (a small but realistic OpenAPI/JSON Schema for OAuth login) via the same "STUB:"-marked but valid pattern. Must produce files that pass `specify tool run contract -- <baseline>/contracts --format json` (the contracts WASI tool from RFC-13/RFC-15).
  - **Mode (ii) Agent contract build:** delegates to the C12 `AgentPhaseDriver` for `/spec:build` invocation; same operator-manual fallback pattern.
  - Ship **(i) first** as the reliable CI baseline; (ii) plugs in via the agent-backend composition.
- Enable real `/spec:build` for the RM-01 contract slice.
- Assert generated OpenAPI, AsyncAPI, or JSON Schema files based on the fixture brief. New assertion ids in `acceptance/assertions/contracts-build.ts`:
  - `contract-slice-emits-yaml-artifacts` (slice baseline `contracts/` has `>0` `.yaml` files matching `contracts/**/*.yaml`)
  - `contract-slice-yaml-validates-via-tool` (`specify tool run contract -- <slice-or-baseline>/contracts --format json` exits 0 with `status: clean`)
  - `contract-slice-includes-openapi-or-asyncapi` (at least one HTTP/messages contract present, matches the OAuth login brief)
  - `contract-slice-includes-required-schemas` (at least the schemas the brief constraints imply: token request/response, error response)
- Run the contracts validation surface used by the repository (`specify tool run contract -- <project>/contracts --format json`). Wrap in `acceptance/assertions/verifier.ts` (it already has the placeholder `assertVerifierStatus` from C05). Promote it from placeholder to real invocation here.
- Assert merge promotes contract artifacts into the root `contracts/` baseline (`hub-or-platform-contracts/` after slice merge run; verify against the existing `slice-baseline-promoted` assertion from C12 for shape, plus a new `contract-baseline-files-present` for the specific contract YAML paths).
- Wire handlers through `defaultDispatch(ctx)` behind `ctx.executeState` guard.
- Keep backend and mobile builds stubbed (the per-slice dispatch keeps the stub `PhaseDriver` for those slices).
- Add `make acceptance-cross-repo-contracts-build-smoke` (runs scenario via the contracts-build backend; deterministic mode (i) by default; skip gracefully when `specify tool run contract` is unavailable).
- Update `acceptance/suites/rm01-cross-repo/scenario.md` frontmatter assertions list and `expected/plan-roles.md` Rule blocks.
- Update `acceptance/runner/backends/README.md` with the per-slice dispatch pattern + the `contracts-build` row.

Out of scope:

- No Omnia or Vectis specialist generation.
- No exact contract prose or formatting goldens beyond stable structured fields.

Acceptance criteria:

- The contract slice produces valid baseline contract artifacts.
- Backend and mobile slices can consume the baseline contract context in later phases.
- Failures are classified as contract generation, contract validation, merge, or runner setup.

Subagent prompt:

```text
Implement C13 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Enable real contracts build in the RM-01 suite and keep implementation builds stubbed.
Assert structural contract artifacts and validation results.
```

### C14a: Real Omnia Build

Goal: replace the backend implementation stub with real Omnia specialist generation.

Suggested scope:

- Add `acceptance/runner/backends/omnia-build-driver.ts` with `OmniaBuildPhaseDriver implements PhaseDriver`. Mirror C13's `ContractsBuildPhaseDriver` shape. Mode (i) deterministic Omnia crate skeleton (Cargo.toml, lib.rs, minimal trait usage) marked with `# STUB:` headers; mode (ii) agent-delegated for `/spec:build` (deferred).
- Add `acceptance/runner/backends/omnia-build.ts` with `OmniaBuildBackend` composing `ScriptedExecuteBackend` (or `ScriptedFinalizeBackend`) with `phaseDriverFor: (entry) => entry.name === SLICE_CONTRACT ? contractsDriver : entry.capability === "omnia" ? omniaDriver : stubDriver`. Reuse the C13 contracts driver for the contract slice — backend slices need real contract artifacts as baseline.
- Enable real `/spec:build` for the backend slice only.
- Use the baseline contract artifacts produced by C13.
- Add `acceptance/assertions/omnia-build.ts` with new assertion ids:
  - `omnia-slice-emits-cargo-toml`
  - `omnia-slice-emits-lib-rs`
  - `omnia-slice-residue-under-routed-project`
  - `omnia-slice-no-output-outside-project` (forbidden-path check)
  - `omnia-baseline-files-present`
- Assert expected crate/test layout or the narrowest reliable Omnia validation command. If an Omnia validation WASI tool is declared (similar to `specify tool run contract`), use `acceptance/assertions/verifier.ts`'s pattern. Otherwise skip with a clear `cli-substrate` note.
- Assert Omnia output stays inside expected project-owned paths (`crates/<crate>/`).
- Preserve runtime, cost, and flake observations in run summaries.
- Add `make acceptance-cross-repo-omnia-build-smoke`.
- Update `scenario.md`, `expected/plan-roles.md`, `expected/evidence-inventory.md`, `backends/README.md`, `BackendName`, schema enum.

Out of scope:

- No Vectis build.
- No broad downstream application correctness testing.

Acceptance criteria:

- Backend generation succeeds against the RM-01 contract baseline.
- The resulting project passes the selected structural/build assertions.
- The suite still reaches merge, push, and finalize.

Subagent prompt:

```text
Implement C14a from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Enable real Omnia build for the RM-01 backend slice only.
Keep Vectis stubbed and keep assertions structural.
```

### C14b: Real Vectis Build

Goal: replace the mobile implementation stub with real Vectis specialist generation.

Suggested scope:

- Add `acceptance/runner/backends/vectis-build-driver.ts` with `VectisBuildPhaseDriver implements PhaseDriver`. Mirror C13/C14a shape. Mode (i) deterministic Vectis composition/design/scaffold artifacts (e.g. `composition.yaml`, minimal SwiftUI / Compose stubs) marked with `# STUB:` headers.
- Add `acceptance/runner/backends/vectis-build.ts` with `VectisBuildBackend` composing the per-slice dispatch.
- Enable real `/spec:build` for the mobile slice only.
- Use the baseline contract artifacts produced by C13.
- Add `acceptance/assertions/vectis-build.ts` with assertion ids:
  - `vectis-slice-emits-composition-yaml`
  - `vectis-slice-emits-screen-files`
  - `vectis-slice-residue-under-routed-project`
  - `vectis-slice-no-output-outside-project`
  - `vectis-baseline-files-present`
- Assert expected composition, design, scaffold, or the narrowest reliable Vectis validation command.
- Assert Vectis output stays inside expected project-owned paths (`apps/mobile/`, `composition.yaml`, etc.).
- Preserve runtime, cost, and flake observations in run summaries.
- Add `make acceptance-cross-repo-vectis-build-smoke`.
- Update `scenario.md`, `expected/plan-roles.md`, `expected/evidence-inventory.md`, `backends/README.md`, `BackendName`, schema enum.

Out of scope:

- No Omnia build unless C14a is explicitly merged into the same run.
- No broad downstream app correctness testing.

Acceptance criteria:

- Mobile generation succeeds against the RM-01 contract baseline.
- The resulting project passes the selected structural/build assertions.
- The suite still reaches merge, push, and finalize.

Subagent prompt:

```text
Implement C14b from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Enable real Vectis build for the RM-01 mobile slice only.
Keep Omnia independent and keep assertions structural.
```

### C15: Recorded Transcript Backend

Goal: create cheap regression coverage from a trusted live or semi-live run.

Suggested scope:

- Define the recorded transcript input format. Reuse the `stub-actions.jsonl` / `scripted-plan-actions.jsonl` / `scripted-execute-loop.jsonl` / `scripted-finalize-actions.jsonl` shapes already produced by C08-C11 backends. Each line is a `{ step, args, cwd, exitCode, ts, artifacts? }` record. C15's recorded format is the union of these.
- Add `acceptance/runner/backends/recorded.ts` with `RecordedBackend` implementing `Backend`. Its `prepare` reads the recorded trace from `--recorded-trace <path>.jsonl`; `invoke` replays each action: for `specify ...` calls, run the same argv against the live CLI and compare exit codes (fail with `live-agent-nondeterminism` on mismatch); for synthetic events (e.g. fixture writes), apply directly.
- Reuse C07's `setupHub` for the hub bootstrap (the recorded trace does not record hub creation; that's deterministic setup). Compare resulting `assertions.json` + `final-tree.txt` against the trace's recorded final state.
- Replay known tool decisions without invoking a live model.
- Compare stable tool-call intent (the argv list) and final structural state (the `assertions.json` records).
- Document how to regenerate a recorded trace: any successful `scripted-execute` / `scripted-finalize` / `agent` run already emits a JSONL trace under the run dir; copying that file under `acceptance/recorded/<scenario-id>/<trace-id>.jsonl` registers it.
- Add `make acceptance-cross-repo-recorded-smoke` that runs the suite via `RecordedBackend` against a checked-in trace at `acceptance/recorded/rm01-cross-repo/baseline.jsonl`.
- Mark recorded replay as complementary coverage, not a replacement for periodic live RM-01 runs.

Out of scope:

- No byte-for-byte transcript golden for live prose.
- No requirement that every scenario has a recording.

Acceptance criteria:

- A previously trusted RM-01 plan-level or execute run can be replayed cheaply.
- Failures distinguish transcript drift from runner/setup failures.
- Recorded artifacts are small enough to review.

Subagent prompt:

```text
Implement C15 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Add a recorded transcript backend for stable structural replay.
Do not use transcript byte-for-byte prose as the oracle.
```

### C16: CI And Promotion Posture

Goal: make acceptance useful without destabilizing ordinary PRs.

Suggested scope:

- Add or finalize `make acceptance-smoke` (already exists from C05).
- Add `make acceptance-cross-repo` as an aggregator target that runs the deterministic cross-repo suite tier (`setup`, `plan`, `execute`, `finalize`, `contracts-build`, `omnia-build`, `vectis-build`, `recorded`).
- Add a `make acceptance-cross-repo-deterministic` alias if useful (the same as `acceptance-cross-repo` minus `define` which requires operator-results).
- Document the tier matrix:
  - **Tier 0 (every PR):** `make checks` (existing).
  - **Tier 1 (every PR, fast):** `acceptance-smoke`, `acceptance-stub-smoke`, `acceptance-cross-repo-recorded-smoke` (per C15 amendment: cheap and high-signal CLI substrate regression detection).
  - **Tier 2 (every PR, longer):** `acceptance-cross-repo-setup-smoke`, `acceptance-cross-repo-plan-smoke`, `acceptance-cross-repo-execute-smoke`, `acceptance-cross-repo-finalize-smoke`. These run end-to-end deterministic flows (~1-2s each).
  - **Tier 3 (every PR or nightly per touched-file pattern):** `acceptance-cross-repo-contracts-build-smoke`, `acceptance-cross-repo-omnia-build-smoke`, `acceptance-cross-repo-vectis-build-smoke`. Run on PRs touching capability briefs, plugins, or specialist code; nightly otherwise.
  - **Tier 4 (operator-driven / manual):** `acceptance-cross-repo-define-smoke` with `--operator-results`. Reserved for live-agent runs; not gated on PR.
- Implement touched-file gating in `scripts/checks.ts` or a new `scripts/acceptance-tier.ts` helper that takes the changed-file list (from `git diff --name-only`) and selects the smallest sufficient smoke set. Useful for local pre-push and CI alike.
- Add a "trace freshness" check: if a PR diff touches `acceptance/recorded/**/*.jsonl`, require the operator to disclose the source run (run id + timestamp from `recorded-trace-header`) in the PR body or commit message. Bake into `scripts/checks.ts`.
- Preserve failure artifacts in CI without committing normal run output. CI must surface the temp run directory as a build artifact only on failure (or when `--preserve` was explicitly set), matching the "preserved on failure" rule in `acceptance/README.md`. Successful run trees must not be retained as CI artifacts.
- Add CI only for deterministic or recorded tiers at first. The `agent` backend (C12 mode ii) and any future Cursor SDK backend stay out of default CI.
- Document live-agent requirements separately from deterministic runner requirements in `acceptance/README.md`.
- Add `docs/contributing/acceptance.md` (or extend `acceptance/README.md`) with a "Running the suite" section: which target to run when, how to set `SPECIFY_BIN`, what `--preserve` does, how to interpret fault-domain hints in failure summaries.
- Add a GitHub Actions workflow file at `.github/workflows/acceptance.yml` (or extend an existing workflow if present) with the Tier 0–2 targets always-on and Tier 3 gated by touched-file patterns. If `.github/workflows/` does not exist or is owned by a separate process, document the recommended workflow shape in `docs/contributing/acceptance.md` instead.

Out of scope:

- No mandatory live model dependency for every PR.
- No hosted telemetry dependency.
- No RM-14 recovery suite.

Acceptance criteria:

- Fast checks remain fast.
- Heavy outside-in coverage is available before releases and on targeted workflow/capability changes.
- CI failure evidence points to CLI substrate, skill orchestration, capability brief, specialist generation, runner setup, external fake boundary, or live-agent nondeterminism.

Subagent prompt:

```text
Implement C16 from rfcs/rm-01-acceptance-framework-implementation-plan.md.
Wire documented acceptance targets and CI posture after the deterministic suites are stable.
Do not make live-agent runs mandatory for every PR.
```

## Suggested Subagent Assignment

Use one subagent per change unless a change is explicitly split:

| Change | Repo area | Depends on | Can run with |
| --- | --- | --- | --- |
| C01 | `acceptance/` docs | none | none |
| C02 | `capabilities/contracts/tests/` | C01 | none |
| C03 | `scripts/checks.ts`, docs | C02 | C04, C06 |
| C04 | `acceptance/runner/` | C02 | C03, C06 |
| C05 | contracts smoke | C02, C04 | C07 |
| C06 | RM-01 suite content | C01 | C03, C04 |
| C07 | runner setup/fake forge | C04, C06 | C05 |
| C08 | runner backend | C04, C07 | C09 |
| C09 | RM-01 plan suite | C06, C07 | C08 |
| C10 | RM-01 execute suite | C08, C09 | none |
| C11 | RM-01 push/finalize | C10 | none |
| C12 | real define/merge | C11 | C15 if a trusted trace exists |
| C13 | real contracts build | C12 | C15 |
| C14a | real Omnia build | C13 | C14b, C15 |
| C14b | real Vectis build | C13 | C14a, C15 |
| C15 | recorded replay | C09 or later trusted run | C12, C13, C14a, C14b |
| C16 | CI/docs promotion | C05 and desired RM-01 tier | none |

## Guardrails For Every Subagent

- Read `AGENTS.md`, `.cursor/rules/project.mdc`, and the source RFC before editing.
- Check the current git status before editing and do not revert unrelated user changes.
- Keep run outputs out of the repo unless intentionally adding fixture inputs or promoted goldens.
- Prefer Deno TypeScript for repo-local framework scripts unless a change explicitly needs Rust in `specify-cli`.
- Do not hand-edit `.specify` lifecycle state in fixtures; use CLI setup commands.
- Keep live-agent use behind an explicit backend or command flag.
- Run `make checks` for documentation/checking changes when practical.
- For runner changes, run the narrowest local runner command added by that change.

## Open Decisions To Settle Early

1. Scenario metadata format: markdown sections only, YAML frontmatter, or a later manifest. Recommendation: use YAML frontmatter once C03 begins, but keep the body human-readable.
2. Runner command location: `acceptance/runner/main.ts` versus `scripts/acceptance.ts`. Recommendation: keep implementation under `acceptance/runner/` and expose make targets.
3. Agent backend: Cursor SDK, CLI session launcher, deterministic stubs, or recorded replay. Recommendation: backend interface first, with manual/stub support before live runtime.
4. Golden policy: structural assertions by default; selected JSON/tool-call goldens only when stable and cheap to review.
5. Cross-repo ownership: keep RM-01 framework work in `augentic/specify`; only open `augentic/specify-cli` follow-ups when the runner exposes a concrete CLI gap.

## Definition Of Done For RM-01 Happy Path

RM-01 acceptance framework implementation is complete when:

- Contracts scenarios are normalized and statically validated.
- `make acceptance-smoke` proves at least one narrow contracts scenario without requiring live model behavior.
- The RM-01 suite can create a temp hub, register backend/mobile fixture repos, plan from a user-facing brief, execute contract-first dependencies, push through fake forge behavior, simulate external merge, and finalize.
- The suite records enough evidence to debug failures without preserving every run by default.
- Full live capability builds are either passing structurally or explicitly documented as a heavier tier with current limitations.
- Default PR checks remain fast and deterministic.
