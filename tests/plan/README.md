# Plan Generation Manual Scenarios

These documents are manual scenarios for `/change:draft` from an operator's point of view. They exercise plan authoring only:

1. turn a short brief or source input into `plan.yaml`
2. validate the plan before execution starts
3. inspect generated slice roles, dependencies, and project routing
4. record structural outcomes that are stable across prose differences

This is intentionally not an automated harness. There is no runner, fake forge, recorded transcript, CI target, or golden output comparison in this scenario pack. The point is to keep a small repeatable manual script first, then decide which checks are stable enough to automate later.

## Relationship To Acceptance

These are shared plan-generation scenario documents. They live under `tests/plan/` because `/change:draft` is orchestration: it authors a change-level plan that coordinates slices rather than owning one adapter's slice loop.

End-to-end cross-repo acceptance remains in [`tests/cross-repo/`](../cross-repo/). Adapter-local tests, such as the contracts scenarios, stay under `adapters/<adapter>/tests/`.

Static checks validate the YAML frontmatter and scenario ID for files in this directory that start with frontmatter. Prose-only documents such as this README and [`run-summary-template.md`](run-summary-template.md) are skipped by the scenario frontmatter check.

## Scenario Index

| Scenario file                                | Scenario ID           | Kind    | Backend  |
| -------------------------------------------- | --------------------- | ------- | -------- |
| [`single-project.md`](single-project.md)     | `plan-single-project` | `suite` | `manual` |
| [`contract-routing.md`](contract-routing.md) | `contract-routing`    | `suite` | `manual` |

## Scenario Pack Shape

Every scenario file in this directory uses the same compact shape:

1. **YAML frontmatter** - machine-readable routing and assertions.
2. **Heading + `Scenario ID:` line** - visible copy of the scenario ID.
3. **Intent** - what plan-generation behavior the scenario proves.
4. **Workspace** - project shape, isolation, prerequisites, and non-goals.
5. **Inputs** - files or source material the operator creates before running.
6. **Invocation** - slash-command and CLI prompts to run.
7. **Expected Artifacts** - files or plan state to check.
8. **Assertions** - structural pass/fail checks.
9. **Negative Expectations** - forbidden machinery for this first pass.
10. **Cleanup** - how to preserve or discard the run state.

The frontmatter is the source of truth for future automation. The body remains the human-readable operator contract.

## Manual Test Flow

Run each scenario from a disposable workspace. Plan generation writes local Specify state, so avoid using an important working tree.

For each run:

1. Open the scenario file.
2. Create the temporary workspace described in **Workspace**.
3. Create any source files described in **Inputs**.
4. Run the `/change:draft` prompt from **Invocation** exactly as written unless the scenario documents an allowed local substitution.
5. Run the listed validation and inspection commands.
6. Check each item in **Assertions** and **Negative Expectations**.
7. Preserve failure evidence: `plan.yaml`, `.specify/plans/<change-name>/`, command output, registry state when relevant, and `specify plan status` output.
8. Fill out [`run-summary-template.md`](run-summary-template.md).

## Run-All Prompt

Use this prompt when you want an agent to run every plan scenario in sequence without asking for manual confirmation between steps:

```text
Run all plan-generation test scenarios in tests/plan/ in this order:
1. single-project.md
2. contract-routing.md

Do not ask for confirmation between scenarios. For each scenario:
- Read the scenario file completely before acting.
- Create disposable workspaces and source files described by the scenario.
- Run only the listed /change:draft prompt and validation commands.
- Do not run /change:execute, workspace push, finalize, a scenario runner, or a
  golden-output comparison.
- Check that plan.yaml exists and validates.
- Summarize generated plan entries by role, project, dependencies, and status.
- Evaluate every assertion and negative expectation before moving on.

Keep each scenario isolated. At the end, report:
- each scenario name
- pass/fail status
- generated plan entries
- validation output summary
- any cleanup performed
```

## Run Summary

Every manual run should produce a summary using [`run-summary-template.md`](run-summary-template.md). Keep the completed summary with the run evidence, or paste it into the operator notes for a fully manual run.
