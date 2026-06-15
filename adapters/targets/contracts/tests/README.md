# Contract Test Scenarios

These documents are owner-local eval scenarios for `contracts@1.0.0`
interface generation. They exercise the dedicated contract slice loop:

1. `/spec:refine` creates `proposal.md`, `specs/**/*.md`, and `tasks.md`.
2. `/spec:build` authors, imports, repairs, and verifies change-local
   `contracts/**/*.yaml` deltas.
3. `/spec:merge` promotes those deltas into the root `contracts/` baseline.

Implementation adapters such as Omnia and Vectis consume baseline contracts
as context. They do not generate new or changed interface shapes inline.

## Relationship To Evals

These are owner-local scenario documents. They live next to the `contracts`
adapter because the behavior under test is one adapter's slice loop in
isolation. Static checks validate their YAML frontmatter and scenario IDs; the
scenario bodies remain human-readable operator instructions.

Every scenario in this directory is operator-driven. A human or agent follows
the prose, runs the prompts, and fills out a
[run summary](run-summary-template.md).

## Scenario Index

| Scenario file                              | Scenario ID                  | Kind                  | Authorship mode          |
| ------------------------------------------ | ---------------------------- | --------------------- | ------------------------ |
| [`describe.md`](describe.md)               | `contracts-describe`         | `adapter`          | Generate from prose      |
| [`design.md`](design.md)                   | `contracts-design`           | `adapter`          | Generate from prose      |
| [`update.md`](update.md)                   | `contracts-update-boundary`  | `adapter-boundary` | Generate from prose      |
| [`import.md`](import.md)                   | `contracts-import`           | `adapter`          | Import existing contracts |
| [`source.md`](source.md)                   | `contracts-source`           | `adapter`          | Extract from source code  |

Scenario IDs are kebab-case, prefixed with the adapter name, and globally
unique within the opted-in scenario set in this repo. `update.md` is marked
`adapter-boundary` because the scenario asserts the *absence* of contract
output during the first define run; the regression path that produces real
contract artifacts is then exercised as a separate sequence within the same
file. See [Scenario Pack Shape](#scenario-pack-shape) for the canonical
sections every scenario file uses.

## Scenario Pack Shape

Every scenario file in this directory uses the same compact shape:

1. **YAML frontmatter** — machine-readable routing (id, owner, kind, adapter,
   entrypoint, stages, isolation, optional assertions and
   expected-artifacts hints).
2. **Heading + `Scenario ID:` line** — the id restated as a visible field so
   it survives any environment that suppresses frontmatter rendering.
3. **Intent** — what behavior the scenario proves.
4. **Workspace** — adapter, project shape, isolation rules, and any
   precondition the operator must satisfy before invocation.
5. **Inputs** — files or source trees the operator must create before
   invocation. The actual file bodies live here as fenced blocks; the prompts
   reference them by path.
6. **Invocation** — the slash-command prompt(s) to run, copied verbatim. These
   prompts are the human operator contract: they must remain executable as-is.
7. **Expected Artifacts** — the change-local contract files produced during
   `/spec:build` (and, after merge, the same paths in the baseline `contracts/`
   tree). For boundary scenarios this section may describe the artifacts of a
   regression path rather than the negative path.
8. **Assertions** — structural checks that define pass/fail, such as
   `files-exist` and `contract-validator-clean`.
9. **Negative Expectations** — boundary behavior that must not occur. For
   `update.md` this section is load-bearing: it is the primary oracle.
10. **Cleanup** — whether to drop, archive, or preserve slice and baseline
    state before moving to the next scenario.

The frontmatter is the source of truth for routing fields future automation may consume.
The body remains canonical for human-readable prose.

### Frontmatter fields used here

```yaml
---
id: contracts-describe                # required, kebab-case, globally unique
owner: contracts                      # required
kind: adapter                      # required: adapter | adapter-boundary
adapter: contracts@1.0.0              # required for adapter and adapter-boundary
entrypoint: /spec:refine              # required: slash-command, /<plugin>:<skill>
stages: [refine, build, merge]        # required: contiguous slice of plan | refine | build | merge | drop
isolation: fresh-project              # required: fresh-project | shared-baseline | shared-slice
authorship-mode: prose                # optional adapter-specific hint
assertions:                           # optional, named assertion ids
  - files-exist
  - contract-validator-clean
expected-artifacts:                   # optional, mirrors the body section
  - contracts/...
negative-expectations:                # optional, free-form forbidden-condition ids
  - implementation-design-emits-contract-yaml
---
```

The fields are intentionally compatible with the canonical `schemas/authoring/scenario.schema.json` (embedded in the CLI binary).
Static validation in `scripts/check.ts` enforces the frontmatter shape.

## Manual Test Flow

Run each scenario from a project initialized with the `contracts@1.0.0` schema, or
from a test workspace where `/spec:init` has already selected that schema. The
boundary scenario [`update.md`](update.md) is the one exception — its initial
prompt runs in an implementation-schema project (Omnia or Vectis) and its
regression path runs in a `contracts@1.0.0` project; the file documents both.

For each scenario:

1. Open the scenario file.
2. Create any source file described in the scenario's **Inputs** section, such
   as `docs/returns-api-design.md` or `vendor/ticket-api.openapi.yaml`.
3. Run the scenario's `/spec:refine ...` prompt from **Invocation**.
4. Review the generated `proposal.md`, `specs/**/*.md`, and `tasks.md`.
5. Run `/spec:build <change-name>`.
6. Verify the **Expected Artifacts** under `contracts/http/*.yaml`,
   `contracts/messages/*.yaml`, or `contracts/schemas/*.yaml` exist in the
   slice working tree.
7. Review verifier output for unresolved `$ref` failures, missing schema
   metadata, binding coverage failures, or manual-review warnings; record
   findings against the scenario's **Assertions** list.
8. Optionally run `/spec:merge <slice-name>` to promote the change-local
   contract deltas into the root `contracts/` baseline.
9. Drop or archive the slice before moving to the next scenario per the
   scenario's **Cleanup** rule.
10. Fill out the [run summary template](run-summary-template.md) for the
    scenario and keep it with the run evidence (or paste it into the
    operator's notes for a fully manual run).

The `update.md` scenario is expected to demonstrate a boundary. It should not
make an implementation `design.md` update act as the contract source; the
correct path is a separate `contracts@1.0.0` change. Its **Negative Expectations**
section is the primary oracle for that scenario.

## Run-All Prompt

Use this prompt when you want an agent to run every scenario in sequence without
asking for manual confirmation between steps:

```text
Run all contract test scenarios in adapters/targets/contracts/tests/ in this order:
1. describe.md
2. design.md
3. update.md
4. import.md
5. source.md

Do not ask for confirmation between scenarios. For each scenario:
- Read the scenario file completely before acting.
- Create any temporary source files the scenario requires.
- Run the listed /spec:refine prompt as a contracts@1.0.0 change.
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

## Run Summary Template

Every manual run should produce a summary using the shape in
[`run-summary-template.md`](run-summary-template.md) so reviewers can compare
runs consistently.
