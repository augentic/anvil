# Contract Test Scenarios

These documents are manual regression scenarios for `contracts@v1` interface
generation. They exercise the dedicated contract-change flow:

1. `/spec:define` creates `proposal.md`, `specs/**/*.md`, and `tasks.md`.
2. `/spec:build` authors, imports, repairs, and verifies change-local
   `contracts/**/*.yaml` deltas.
3. `/spec:merge` promotes those deltas into the root `contracts/` baseline.

Implementation schemas such as Omnia and Vectis consume baseline contracts as
context. They do not generate new or changed interface shapes inline.

## Scenarios

- [`describe.md`](describe.md) — generate JSON Schema and OpenAPI artifacts from
  a prose description passed to `/spec:define`.
- [`design.md`](design.md) — generate contract artifacts from a prose design
  document named as source material.
- [`update.md`](update.md) — boundary test showing that implementation
  `design.md` updates are not a contract generation source.
- [`import.md`](import.md) — import and normalize an existing OpenAPI document.
- [`source.md`](source.md) — reverse-engineer JSON Schema and OpenAPI artifacts
  from a legacy TypeScript codebase whose API surface a prior `/spec:analyze`
  run has identified.

## Manual Test Flow

Run each scenario from a project initialized with the `contracts@v1` schema, or
from a test workspace where `/spec:init` has already selected that schema.

For each scenario:

1. Open the scenario file.
2. Create any source file described by the scenario, such as
   `docs/returns-api-design.md` or `vendor/ticket-api.openapi.yaml`.
3. Run the scenario's `/spec:define ...` prompt.
4. Review the generated `proposal.md`, `specs/**/*.md`, and `tasks.md`.
5. Run `/spec:build <change-name>`.
6. Verify the expected `contracts/http/*.yaml`, `contracts/messages/*.yaml`, or
   `contracts/schemas/*.yaml` files were produced in the change.
7. Review verifier output for unresolved `$ref` failures, missing schema
   metadata, binding coverage failures, or manual-review warnings.
8. Optionally run `/spec:merge <change-name>` to promote the change-local
   contract deltas into the root `contracts/` baseline.
9. Drop or archive the change before moving to the next scenario if you want each
   run to start from an empty baseline.

The `update.md` scenario is expected to demonstrate a boundary. It should not
make an implementation `design.md` update act as the contract source; the correct
path is a separate `contracts@v1` change.

## Run-All Prompt

Use this prompt when you want an agent to run every scenario in sequence without
asking for manual confirmation between steps:

```text
Run all contract test scenarios in schemas/contracts/tests/ in this order:
1. describe.md
2. design.md
3. update.md
4. import.md
5. source.md

Do not ask for confirmation between scenarios. For each scenario:
- Read the scenario file completely before acting.
- Create any temporary source files the scenario requires.
- Run the listed /spec:define prompt as a contracts@v1 change.
- Run /spec:build for the generated change.
- Check that the expected change-local contracts/**/*.yaml files exist.
- Check verifier output for failures or manual-review warnings.
- Summarize pass/fail before moving to the next scenario.
- If a scenario is a boundary or negative test, evaluate it against the expected
  behavior documented in that file rather than trying to force contract output.

Keep each scenario isolated. If a generated change would affect the next test's
baseline, drop or archive it before continuing unless the scenario explicitly
requires the previous baseline. At the end, report:
- each scenario name
- pass/fail status
- generated contract files
- verifier warnings or failures
- any cleanup performed
```
