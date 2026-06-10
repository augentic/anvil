# Augentic specialist usage

Augentic-specific supplement: how specialist skills (Omnia, Vectis, and friends) *consume* the standard artifacts. For artifact structure see [Artifact format](https://specify.augentic.io/reference/artifact-format.html) and [Artifacts in depth](https://specify.augentic.io/explanation/artifacts.html).

Augentic uses stock Specify as its executable workflow contract. Specialist skills read the four artifacts `proposal.md`, `spec.md`, `design.md`, and `tasks.md` during `/spec:plan → /spec:execute` (which drives `/spec:refine → /spec:build → /spec:merge` per slice), but they must not redefine the runtime contract. Artifact validation runs automatically inside `/spec:build` before implementation begins.

## Where specialists read and write

```text
$PROJECT_DIR        = <workspace>
$SLICE_DIR          = $PROJECT_DIR/.specify/slices/<slice-name>
$SPECS_DIR          = $SLICE_DIR/specs
$DESIGN_PATH        = $SLICE_DIR/design.md
$PROPOSAL_PATH      = $SLICE_DIR/proposal.md
$TASKS_PATH         = $SLICE_DIR/tasks.md
$DECISIONS_DIR      = $SLICE_DIR/decisions          # optional, slice-authored
$BASELINE_SPECS     = $PROJECT_DIR/.specify/specs
$BASELINE_DECISIONS = $PROJECT_DIR/.specify/decisions
```

Merged and dropped slices are archived to `$PROJECT_DIR/.specify/archive/YYYY-MM-DD-<slice-name>/` — a prunable convenience cache, not the system of record.

## What belongs to specialists vs the artifacts

The dividing line matters because it keeps specs portable across targets:

- **Specs stay behavioral.** They must not encode Omnia trait bindings, WASM implementation details, or generator-specific instructions. See [spec format](https://specify.augentic.io/reference/artifact-format.html#spec-files-behavioral-what).
- **`design.md` carries the technical "how"** — domain models, API and message shapes, business logic, integrations, configuration, and risks. See [design format](https://specify.augentic.io/reference/artifact-format.html#design-document-technical-how). Cite stable requirement IDs (e.g. `REQ-003`) rather than requirement titles.
- **Generator-owned binding decisions stay in the target adapter's build brief**, never in the artifacts. Omnia trait composition and Crux effect types are decided by the target adapter's `build` brief that writes the code, guided by its `shape` brief — not hardcoded into the behavioral contract.

## Deriving specs from source code

When a source adapter's `extract` reconstructs behaviour from legacy code, the specialist builds each `specs/<domain>/spec.md` like this:

1. **Purpose** from the role of the handler or function.
2. **Requirements** from distinct business rules, assigning stable IDs in spec order (`REQ-001`, `REQ-002`, …).
3. **Scenarios** from happy paths, edge cases, and failures (WHEN/THEN).
4. **Error conditions** from observed failure behavior.
5. **Metrics** only when explicit in the source.

## Writing agent-completable tasks

`tasks.md` is the specialist's implementation checklist (see the [tasks format](https://specify.augentic.io/reference/artifact-format.html#tasks-document)). The Augentic constraint on top of the base format: **every task must be agent-completable** — a coding agent can perform it and verify completion through code, local tooling, mocks, fixtures, contract validators, build commands, or reviewer skills.

Never generate tasks that require human-only action: manual app testing, visual inspection, real-world API credentials, production services, physical-device checks, app-store review, or "ask the user to verify". When behaviour appears to need manual validation, write the agent-verifiable equivalent instead (a mocked API test, a replay, a simulator or build check, a contract test, or a scripted smoke test).

`specify slice validate` checks only the checkbox and grouping *shape* of `tasks.md`; it does not inspect task intent. Agent-completability is therefore judged at write time and re-checked by `/spec:build` as a preflight.

Tasks are implemented by the active target adapter's `build` brief, which carries the specialist orchestration inline; they do not route to standalone specialist skills.

## Decision Records

A slice may author **Decision Records** at `$DECISIONS_DIR/<slug>.md` for the durable *why* behind a design choice. `/spec:merge` promotes them into `$BASELINE_DECISIONS/DEC-NNNN-<slug>.md`. See [Decision Records](https://specify.augentic.io/reference/artifact-format.html#decision-records-design-why) for the format and promotion rules. Accepted decisions also sharpen the project's plan-time routing identity (a third axis beside what the project does and what recently changed).

## See also

- [From sources to slices](./reconciliation.md) — how leads become slices and evidence becomes a spec
- [Anatomy of an adapter](https://specify.augentic.io/explanation/adapter-anatomy.html) — how source and target adapters compose with synthesis
