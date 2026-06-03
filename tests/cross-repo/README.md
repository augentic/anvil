# Cross-Repo Manual Acceptance Scenarios

These documents are manual acceptance scenarios for the cross-repo Specify
workflow from an operator's point of view, expressed through the three-skill
change lifecycle (`/spec:plan` → operator review → `/spec:execute` → `/spec:finalize`):

1. draft a multi-slice change from a short feature brief (`/spec:plan`)
2. review the draft plan at the operator pause (`inspect plan.yaml`,
   optional `specify plan amend`)
3. execute contract and implementation slices in dependency order
   (`/spec:execute loop`)
4. drive the post-execute tail (`/spec:finalize`): push prepared workspace
   branches, observe PR state, and archive the change after the project
   branches are merged externally

This is intentionally not an automated harness. There is no runner, fake forge,
recorded transcript, CI target, or golden output comparison in this scenario
pack. The point is to get a simple repeatable manual script first, then decide
which parts are stable enough to automate later.

## Relationship To Acceptance

These are shared outside-in scenario documents. They live under
`tests/cross-repo/` because the cross-repo workflow spans the workspace, registry,
workspace, change plan, contract adapter, and implementation adapters.
Adapter-local tests, such as the contracts scenarios, stay under
`adapters/<adapter>/tests/`.

Static checks validate the YAML frontmatter and scenario ID. The scenario body
remains the human-readable operator contract.

## Scenario Index

| Scenario file | Scenario ID | Kind | Backend |
| --- | --- | --- | --- |
| [`scenario.md`](scenario.md) | `cross-repo-contract-flow` | `suite` | `manual` |

## Scenario Pack Shape

The scenario follows the same compact shape used by
[`adapters/targets/contracts/tests/`](../../adapters/targets/contracts/tests/README.md):

1. **YAML frontmatter** - machine-readable routing and assertions.
2. **Heading + `Scenario ID:` line** - visible copy of the scenario ID.
3. **Intent** - what behavior the scenario proves.
4. **Workspace** - project shape, isolation, prerequisites, and non-goals.
5. **Inputs** - files or source material the operator creates before running.
6. **Invocation** - slash-command and CLI prompts to run, organised by
   lifecycle stage (draft, review, execute, finalize).
7. **Expected Artifacts** - files or state transitions to check.
8. **Assertions** - structural pass/fail checks, including the durable
   end-state outcomes (archived plan path, merged-PR list, archived
   `change.md`).
9. **Negative Expectations** - forbidden machinery for this first pass.
10. **Cleanup** - how to preserve or discard the run state.

## Manual Test Flow

Run the scenario from a disposable workspace. The run creates local projects and
branches, so avoid using an important working tree.

For each run:

1. Open [`scenario.md`](scenario.md).
2. Create the temporary workspace and project workspaces described in **Workspace**.
3. Create the feature brief from **Inputs**.
4. Run the prompts and commands from **Invocation** exactly as written unless
   the scenario documents an allowed local substitution. The `/spec:finalize`
   stage is invoked twice intentionally — the first invocation halts on
   `pr-not-merged`; the operator merges the named PRs externally; the second
   invocation archives the plan.
5. Check each item in **Assertions**, including the parity-with-umbrella
   snapshot in the run summary.
6. Preserve failure evidence: `plan.yaml`, `registry.yaml`, command output,
   workspace status, generated artifacts, PR or branch state, and every
   `/spec:finalize` invocation's output.
7. Fill out [`run-summary-template.md`](run-summary-template.md).

## Run Summary

Every manual run should produce a summary using
[`run-summary-template.md`](run-summary-template.md). Keep the completed summary
with the run evidence, or paste it into the operator notes for a fully manual
run.
