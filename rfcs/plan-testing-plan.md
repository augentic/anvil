# Plan Generation Test Plan

> Goal: add a small reusable manual scenario pack for `/change:plan` under
> `tests/plan/`, focused on durable plan structure rather than generated prose.

## Principles

- Keep the first pass manual. Do not add a runner, fake forge, transcript
  replay, CI target, or golden-output comparison.
- Use `tests/plan/` for shared plan-generation behavior because `/change:plan`
  is orchestration, not capability-owned slice work.
- Keep capability-local `/spec:*` slice-loop scenarios under
  `capabilities/<capability>/tests/`.
- Validate scenario metadata with the existing frontmatter checks.
- Judge scenarios by structural plan outcomes: files exist, validation passes,
  slice roles are present, dependencies are correct, and project routing is
  deterministic.

## Target Shape

Add a `tests/plan/` scenario pack:

```text
tests/plan/
├── README.md
├── run-summary-template.md
├── single-project.md
├── cross-repo-contract-first.md
└── existing-code-extraction.md
```

The exact scenario names can change, but the root should support multiple flat
scenario files like `capabilities/contracts/tests/`.

## Scenario Contract

Each scenario file should use the existing manual scenario shape:

1. YAML frontmatter.
2. Heading plus `Scenario ID:` line.
3. `Intent`.
4. `Workspace`.
5. `Inputs`.
6. `Invocation`.
7. `Expected Artifacts`.
8. `Assertions`.
9. `Negative Expectations`.
10. `Cleanup`.

Suggested frontmatter:

```yaml
---
id: plan-cross-repo-contract-first
owner: plan
kind: suite
backend: manual
entrypoint: /change:plan
stages: [define]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - slices-match-expected-shape
  - dependencies-correct
  - routing-deterministic
expected-artifacts:
  - plan.yaml
negative-expectations:
  - automated-runner-added
  - golden-output-required
---
```

For the first pass, keep `stages: [define]` for plan-only scenarios. Treat it as
the downstream slice phase represented by generated plan entries, not as a new
planning phase. If this becomes confusing, add a later schema update for
plan-specific lifecycle metadata.

## Initial Scenarios

### 1. Single-Project Plan

Proves that a short brief in one initialized project produces a small valid
`plan.yaml` with one or more local slice entries.

Assertions:

- `plan-exists`: `plan.yaml` exists after `/change:plan`.
- `plan-validates`: `specify change plan validate` exits cleanly.
- `slices-match-expected-shape`: entries are named, scoped, and ordered
  consistently with the input brief.
- `no-project-routing-required`: entries do not invent project assignments in a
  single-project workspace.

### 2. Cross-Repo Contract-First Plan

Proves the plan-generation part of the current cross-repo acceptance scenario
without executing, pushing, or finalizing.

Assertions:

- `contract-slice-present`: the plan includes a contract slice.
- `implementation-slices-routed`: implementation slices route to the expected
  projects.
- `dependencies-correct`: implementation slices depend on the contract slice.
- `routing-deterministic`: project assignments match registry descriptions.

### 3. Existing-Code Extraction Plan

Proves that source material from an existing code tree can be converted into a
reviewable plan without requiring immediate execution.

Assertions:

- `sources-recorded`: the plan authoring trail records the supplied source.
- `slices-coherent`: generated entries have clear roles and non-overlapping
  scopes.
- `plan-validates`: the plan validates before any build work starts.

## Implementation Steps

### Change 01: Scenario Discovery

Update `scripts/checks.ts` so `tests/plan/*.md` files with leading frontmatter
are discovered as opted-in scenarios. Skip prose files such as `README.md` and
`run-summary-template.md` through the existing frontmatter opt-in rule.

Acceptance checks:

- `tests/plan/single-project.md` is validated by `make checks`.
- `tests/plan/README.md` and `tests/plan/run-summary-template.md` are ignored
  unless they start with frontmatter.
- Existing `tests/<suite>/scenario.md`, capability, and skill fixture discovery
  still works.

### Change 02: Documentation

Update `docs/contributing/checks.md` and `docs/contributing/acceptance.md` to
document `tests/plan/` as the shared manual plan-generation scenario pack.

Acceptance checks:

- The docs distinguish `tests/plan/` from capability-local
  `capabilities/<capability>/tests/`.
- The docs state that the first pass is manual and structural, not golden based.

### Change 03: Scenario Pack

Add `tests/plan/README.md`, `run-summary-template.md`, and the first one or two
scenario files.

Acceptance checks:

- Each scenario has globally unique `id` frontmatter.
- Each body `Scenario ID:` matches the frontmatter id.
- Expected artifacts use relative paths.
- Assertions are structural and do not require byte-for-byte prose comparison.

## Run Summary Template

The run summary should capture:

- scenario id and file path
- operator or agent
- workspace root
- exact `/change:plan` prompt
- exact validation commands
- generated plan entries
- assertion verdicts
- negative-expectation verdicts
- preserved evidence paths
- final pass/fail verdict and fault domain

## Out Of Scope

- Automated scenario runner.
- Fake forge or fake `gh`.
- Transcript replay.
- CI acceptance target.
- Golden comparisons for generated prose, specs, or plan text.
- Moving capability-local tests into `tests/`.

## Suggested Order

1. Extend scenario discovery for `tests/plan/*.md`.
2. Add the plan scenario README and run summary template.
3. Add the single-project scenario.
4. Add the cross-repo contract-first plan-only scenario.
5. Run `make checks` and adjust docs for any frontmatter or link failures.
