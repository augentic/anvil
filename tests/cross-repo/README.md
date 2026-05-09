# Cross-Repo Manual Acceptance Scenarios

These documents are manual acceptance scenarios for the cross-repo Specify
workflow from an operator's point of view:

1. plan a multi-slice change from a short feature brief
2. route work through a registry-only hub into implementation projects
3. execute contract and implementation slices in dependency order
4. push prepared workspace branches
5. finalize the change after the project branches are merged

This is intentionally not an automated harness. There is no runner, fake forge,
recorded transcript, CI target, or golden output comparison in this scenario
pack. The point is to get a simple repeatable manual script first, then decide
which parts are stable enough to automate later.

## Relationship To Acceptance

These are shared outside-in scenario documents. They live under
`tests/cross-repo/` because the cross-repo workflow spans the hub, registry,
workspace, change plan, contract capability, and implementation capabilities.
Capability-local tests, such as the contracts scenarios, stay under
`capabilities/<capability>/tests/`.

Static checks validate the YAML frontmatter and scenario ID. The scenario body
remains the human-readable operator contract.

## Scenario Index

| Scenario file | Scenario ID | Kind | Backend |
| --- | --- | --- | --- |
| [`scenario.md`](scenario.md) | `cross-repo-contract-flow` | `suite` | `manual` |

## Scenario Pack Shape

The scenario follows the same compact shape used by
[`capabilities/contracts/tests/`](../../capabilities/contracts/tests/README.md):

1. **YAML frontmatter** - machine-readable routing and assertions.
2. **Heading + `Scenario ID:` line** - visible copy of the scenario ID.
3. **Intent** - what behavior the scenario proves.
4. **Workspace** - project shape, isolation, prerequisites, and non-goals.
5. **Inputs** - files or source material the operator creates before running.
6. **Invocation** - slash-command and CLI prompts to run.
7. **Expected Artifacts** - files or state transitions to check.
8. **Assertions** - structural pass/fail checks.
9. **Negative Expectations** - forbidden machinery for this first pass.
10. **Cleanup** - how to preserve or discard the run state.

## Manual Test Flow

Run the scenario from a disposable workspace. The run creates local projects and
branches, so avoid using an important working tree.

For each run:

1. Open [`scenario.md`](scenario.md).
2. Create the temporary hub and project workspaces described in **Workspace**.
3. Create the feature brief from **Inputs**.
4. Run the prompts and commands from **Invocation** exactly as written unless
   the scenario documents an allowed local substitution.
5. Check each item in **Assertions**.
6. Preserve failure evidence: `plan.yaml`, `registry.yaml`, command output,
   workspace status, generated artifacts, PR or branch state, and finalization
   output.
7. Fill out [`run-summary-template.md`](run-summary-template.md).

## Run Summary

Every manual run should produce a summary using
[`run-summary-template.md`](run-summary-template.md). Keep the completed summary
with the run evidence, or paste it into the operator notes for a fully manual
run.
