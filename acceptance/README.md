# Acceptance Framework

> Status: Contract docs only. No runner code yet.
> Source: [RM-01 Acceptance Framework Design](../rfcs/rm-01-acceptance-framework.md) and [Implementation Plan](../rfcs/rm-01-acceptance-framework-implementation-plan.md).

This directory holds the shared acceptance framework for the `specify` repo: the runner, assertion helpers, and outside-in suites that prove workflow skills, capability briefs, and orchestration prose still drive the `specify` CLI substrate correctly.

It does **not** hold every test fixture. Narrow capability scenarios stay close to the capability that owns them (see [Owner-local vs shared](#owner-local-scenarios-vs-shared-acceptance) below). What lives here is the infrastructure that all suites share, plus the cross-cutting suites that no single capability owns.

## Why This Exists

`specify-cli` already has strong deterministic Rust coverage for lifecycle mechanics. The `specify` repo's most important behavior lives in skill markdown, capability briefs, fixtures, transcripts, and manual scenario documents that an agent interprets. A normal Rust integration test cannot exercise that surface. The acceptance framework is the layer that does.

The framework keeps the CLI authoritative for lifecycle mechanics, then adds markdown-driven scenario packs and replayable outside-in runs that prove skills and capabilities still compose the CLI correctly.

## Testing Layers

The framework recognises five layers. The `specify` repo owns layers 1 through 4. Layer 0 lives in `specify-cli` and is referenced, not duplicated.

### Layer 0: CLI Substrate Tests (specify-cli)

Owner: `specify-cli`. Rust integration tests prove deterministic command behavior — lifecycle transitions, plan validation, registry mechanics, workspace sync, branch preparation, merge/residue commit boundaries, fake forge push/finalize, and stable JSON output shapes. The canonical example is `specify-cli/tests/cross_repo.rs`. This repo links to that proof but does not reproduce it.

### Layer 1: Static Framework Checks (specify)

Owner: `specify`, in [`scripts/checks.ts`](../scripts/checks.ts), surfaced as `make checks` and documented in [Consistency Checks](../docs/contributing/checks.md). Catches structural drift before any agent run is needed: markdown links, capability manifests, capability brief `needs` graphs, `SKILL.md` frontmatter and body constraints, skill references and directives, marketplace manifest consistency, and instruction file preambles. Read-only and deterministic.

Future scenario-pack metadata validation (added by a later change in the plan) will live here as opt-in checks.

### Layer 2: Markdown Scenario Packs (specify)

Owner: the capability, skill, or workflow area being tested. Scenario packs are markdown plus small fixture files that express user-facing acceptance scenarios. They should be readable by humans, executable by a runner, and reviewable in ordinary PRs.

The seed pattern is [Contract Test Scenarios](../capabilities/contracts/tests/README.md) — manual, human-driven prompts with expected change-local outputs and boundary expectations. The framework will normalise that pattern (consistent sections, scenario IDs) without throwing away the existing manual prose.

See [Scenario Pack Shape](#scenario-pack-shape) and [Scenario Discovery](#scenario-discovery) below for the concrete shape and discovery rules a runner can rely on.

### Layer 3: Replay Runner (specify, shared)

Owner: shared framework code under [`runner/`](runner/README.md). The runner prepares an isolated temp workspace, invokes a chosen backend, captures evidence, hands final state to assertion modules, and writes a compact summary.

Backends layer onto one runner so suites do not invent their own runtimes:

- manual instructions for current human-driven validation,
- deterministic stub backend for early automation without live generation,
- agent runtime backend when programmatic slash-command execution is available,
- recorded transcript backend for cheap structural regression after a trusted live run.

The runner is not the test oracle. It collects evidence and runs assertions. Assertion vocabulary is documented in [`assertions/README.md`](assertions/README.md).

### Layer 4: Outside-In Acceptance Suites (specify, shared)

Owner: a roadmap item or workflow area, with the suite content under [`suites/`](suites/README.md). These prove full user journeys across multiple skills, capabilities, repositories, and external boundaries. **RM-01 cross-repo** is the first suite. It reuses the runner and assertion vocabulary, and adds local Git remotes, fake `gh`, project registry setup, workspace routing, branch preparation, push handoff, external merge simulation, and finalize.

## Owner-local Scenarios vs Shared Acceptance

Two homes for scenarios. Choose based on **scope of behavior**, not on where automation runs.

| Behavior under test                                                           | Lives in                                            | Example                                            |
| ----------------------------------------------------------------------------- | --------------------------------------------------- | -------------------------------------------------- |
| One capability's slice loop in isolation (`/spec:define` → `/spec:build` → `/spec:merge`) | the capability tree (owner-local)                   | [`capabilities/contracts/tests/`](../capabilities/contracts/tests/README.md) |
| One skill's structural invariants                                             | the skill's `fixtures/` directory (owner-local)     | `plugins/change/skills/plan/fixtures/`             |
| A multi-skill, multi-capability, or multi-repo journey                        | `acceptance/suites/<suite-name>/` (shared)          | `acceptance/suites/rm01-cross-repo/` (planned)     |
| The runner, evidence model, assertion helpers, fake-forge support             | `acceptance/runner/`, `acceptance/assertions/` (shared) | the framework's own scaffolding                |

Owner-local fixtures stay close to the behavior they document so the capability or skill author owns drift. Shared acceptance infrastructure stays in one place so suites do not invent incompatible runners or fixture shapes.

## Scenario Discovery

A runner discovers scenarios by walking these paths in order. Suites C04/C06 and later code can rely on this contract.

1. `acceptance/suites/<suite>/scenario.md` — shared outside-in suites. The suite directory may also contain `inputs/`, `expected/`, `fixtures/`, and a per-suite `README.md`.
2. `capabilities/<capability>/tests/<scenario>.md` — flat owner-local capability scenarios. This is the current contracts shape and remains valid.
3. `capabilities/<capability>/tests/<scenario>/scenario.md` — directory-form owner-local capability scenarios when a scenario needs sibling input or fixture files.
4. `plugins/<plugin>/skills/<skill>/fixtures/<scenario>/scenario.md` — skill-owned fixtures that have been promoted to scenario-pack shape.

Rules:

- A `scenario.md` file is the canonical entry point. A flat `<scenario>.md` file at owner-local paths is also a scenario file when it sits inside a `tests/` directory of a capability.
- **Scenario IDs are kebab-case and unique across all opted-in scenario files** in the repo. The id may live in YAML frontmatter (preferred once frontmatter is adopted) or as an explicit `Scenario ID:` field in the body. Until then, the file path acts as the identity.
- Scenario discovery is opt-in: a runner only loads scenarios that conform to the [Scenario Pack Shape](#scenario-pack-shape). Existing prose-only fixtures are not forced into the schema.

To add a new capability scenario today: drop a markdown file under `capabilities/<capability>/tests/` (flat or directory form), follow the [Scenario Pack Shape](#scenario-pack-shape), and link it from that capability's `tests/README.md`.

To add a new outside-in suite: create `acceptance/suites/<suite-name>/` with a `scenario.md` and a per-suite `README.md`, then list it in [`suites/README.md`](suites/README.md).

## Scenario Pack Shape

The body remains markdown-first so a human can run a scenario manually with no tooling. Once a runner is reading scenarios, optional YAML frontmatter carries machine-readable routing.

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

Optional frontmatter (introduced once static validation lands as a follow-up change in the plan):

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

The body is canonical and human-readable. Frontmatter only carries routing and machine-readable defaults.

## Command Tiers

The framework defines a four-tier execution matrix. Tier 0 is the baseline; Tiers 1–3 are deterministic smokes; Tier 4 is operator-driven. The full reference (timings, when to run what, the touched-file selector, fault-domain taxonomy, recorded-trace regen, recommended GitHub Actions shape) lives at [`docs/contributing/acceptance.md`](../docs/contributing/acceptance.md). Quick reference:

| Target                                       | Tier | Meaning                                                                                                |
| -------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------------ |
| `make checks`                                | 0    | Static framework checks. Required on every PR. Stays fast (< 2s) and deterministic.                    |
| `make acceptance-smoke`                      | 1    | Narrow contracts scenario via the `fixture` backend. Cheap, no live model dependency.                  |
| `make acceptance-stub-smoke`                 | 1    | Deterministic stub backend driving the contracts-describe slice loop end-to-end through `specify`.     |
| `make acceptance-cross-repo-recorded-smoke`  | 1    | Re-runs the checked-in RM-01 trace against the live binary; CLI substrate regression pin.              |
| `make acceptance-cross-repo-{setup,plan,execute,finalize}-smoke` | 2 | Cross-repo deterministic flows (~1–4s each). Run on every PR that touches `plugins/spec/**`, `plugins/change/**`, or framework code. |
| `make acceptance-cross-repo-{contracts,omnia,vectis}-build-smoke` | 3 | Specialist build smokes. Run on PRs touching the matching capability or plugin tree.                   |
| `make acceptance-cross-repo-define-smoke`    | 4    | Operator-driven `/spec:define` run via the `agent` backend. Needs `OPERATOR_RESULTS` (or `--cursor-sdk`). Not gated on PR. |
| `make acceptance-cross-repo`                 | —    | Aggregator. Runs all nine cross-repo smokes serially; `define` skips when `OPERATOR_RESULTS` is unset. |
| `make acceptance-cross-repo-deterministic`   | —    | Aggregator. Same as `acceptance-cross-repo` minus `define`; safe for unattended CI.                    |
| `make acceptance-all`                        | —    | Aggregator. `acceptance-smoke` + `acceptance-stub-smoke` + `acceptance-cross-repo`. Pre-release sweep. |
| `make acceptance-tiers`                      | —    | Touched-file tier selector. Prints the recommended `make` targets for the current diff; never executes them.  |

Live agent runs (Tier 4) must not block every small documentation PR until runtime, cost, and flake behavior are understood.

## Run Evidence Policy

Every automated run writes evidence to a temp run directory, **not** into the repo tree.

Default location: under the OS temp directory, for example `${TMPDIR}/specify-acceptance/<suite>/<run-id>/`. The exact root is owned by the runner and may differ between local development and CI.

Recommended evidence shape (each suite extends as needed):

```text
<run-root>/<suite>/<run-id>/
  summary.md
  scenario.md           # copy of the executed scenario
  transcript.md         # when an agent backend is used
  tool-calls.jsonl
  stdout.log
  stderr.log
  final-tree.txt
  assertions.json
  artifacts/
  failures/
```

RM-01-shaped suites add workflow-specific evidence (registry, plan snapshot pre-finalize, workspace status, push and finalize JSON, hub and project Git logs, fake `gh` PR state). The exact filenames are owned by the suite that produces them.

Retention rules:

- **Pass:** evidence is discarded by default to keep local and CI environments tidy.
- **Failure:** the run directory is preserved automatically so the failure can be debugged.
- **`--preserve` opt-in:** an explicit runner flag forces preservation regardless of outcome, for use when an operator wants to inspect a successful run.

Run output is **never committed**. Small fixture inputs, intentionally promoted goldens, or recorded transcripts may be checked in only when the change that adds them documents why.

## CLI-Authoritative Invariant

The runner and every suite must drive Specify lifecycle state through the `specify` CLI.

- Lifecycle transitions, plan CRUD, registry mutation, workspace sync, branch preparation, merge, archive, push, and finalize all go through `specify` verbs (see [AGENTS.md](../AGENTS.md) for the verb surface).
- The runner **must not hand-edit `.specify/` lifecycle files** — no `mkdir -p .specify/...`, no `mv` into `.specify/archive/`, no rewriting `.metadata.yaml` or `plan.yaml` from runner code.
- A scenario may declare a deterministic stub backend whose stubbed phases write fixture outputs into the slice's working tree. Even then, the lifecycle transitions around those phases (`/spec:define`, `/spec:merge`, `specify change plan transition`, `specify change finalize`) still go through the CLI.
- Test-only state seeding for fixture *inputs* (a `docs/oauth-login.md` brief, a fake `gh` config, a local bare remote) is fine. Test-only seeding of `.specify/` state is not.

This invariant is what keeps the acceptance framework honest: it proves the framework as users experience it, rather than building a parallel lifecycle model.

## Vocabulary Reminder

The implementation plan and this directory use the post-RFC-13 vocabulary from [AGENTS.md](../AGENTS.md):

- **Slice** — the single unit through `define → build → merge`, one per `.specify/slices/<name>/`. Driven by `/spec:*` and `specify slice *`.
- **Change** — the operator-defined umbrella over one or more slices, expressed as `change.md` + `plan.yaml`. Driven by `/change:plan`, `/change:execute`, and `specify change *`.

There is no "change loop"; the per-loop unit is the *slice loop*. Suite docs and scenario prose should follow the same convention so a runner can match assertions on durable role names rather than legacy synonyms.

## What This Directory Does Not Own

- Recovery scenarios (RM-14) are out of scope for the first RM-01 happy-path suite. They will land in a separate suite that reuses the same framework.
- Live-agent integration beyond the Tier 4 operator-driven `define` smoke (Cursor SDK, hosted runtimes) is out of scope for C16. The aggregator and CI workflow are deliberately quiet on agent backends.

## Further Reading

- Operator guide: [Running The Acceptance Suite](../docs/contributing/acceptance.md) — tier matrix, when to run what, fault-domain taxonomy, CI workflow.
- Design: [RM-01 Acceptance Framework](../rfcs/rm-01-acceptance-framework.md).
- Plan: [RM-01 Acceptance Framework Implementation Plan](../rfcs/rm-01-acceptance-framework-implementation-plan.md).
- Outside-in handoff context: [RM-01 Outside-In Acceptance Harness Handoff](../rfcs/rm-01-outside-in-harness.md).
- Roadmap entry: [RM-01 in the Specify Roadmap](../rfcs/roadmap.md).
- Existing seed pattern: [Contract Test Scenarios](../capabilities/contracts/tests/README.md).
- Static checks reference: [Consistency Checks](../docs/contributing/checks.md).
