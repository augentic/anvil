# RFC-35 Implementation Plan

This plan implements [RFC-35: Synthesis Determinism](rfc-35-synthesis-determinism.md) in small sequential sessions across `augentic/specify` and `augentic/specify-cli`. Each session can assume preceding steps are already implemented and should update the progress tracker before handing off.

## Summary Progress Tracker

Status values: `not-started`, `in-progress`, `done`, `blocked`.

```text
Step  Session                                             Repo(s)                 Status       Primary verification
0     Preflight and baseline inventory                    specify, specify-cli    done         git status + focused rg map
1     Core synthesis reference contract                   specify                 done         focused rg + make check (passed)
2     Refine skill, shared docs, and refine fixtures      specify                 done         make check (passed)
3     Vectis target brief alignment                       specify                 done         make check (passed) + focused rg clean
4     Omnia target brief alignment                        specify                 done         make check (passed) + focused rg clean
5     Contracts target brief alignment                    specify                 done         make check (passed) + focused rg clean
6     Resolver JSON briefs-dir output                     specify-cli             done         source/target resolve tests
7     Proposal Units validator rename                     specify-cli             done         validator tests + goldens
8     Spec file-location diagnostics                      specify-cli             done         slice validate tests + cargo make check
9     Cross-repo acceptance and operator notes            both                    done         make check + cargo make ci
```

## Sequencing Rules

Start every session with `git status` in the relevant repo and preserve unrelated user changes. Do not mix `specify` and `specify-cli` edits in the same session except Step 9 acceptance notes. When a step touches `specify-cli`, also read that repo's `AGENTS.md` and the relevant standards document before editing Rust.

## Step 0 - Preflight and Baseline Inventory

Goal: confirm the current repo state and produce a short implementation note for the next agent.

Actions:

- In `augentic/specify`, inspect `git status` and focused hits for `## Scenarios`, `#### Scenario`, `## Crates`, `## Features`, `## Units`, and root-level `spec.md` wording.
- In `augentic/specify-cli`, inspect resolver output code, proposal/cross validator rules, provenance drift messages, and existing tests.
- Record any extra touchpoints discovered before editing.

Exit criteria:

- No files changed unless the operator explicitly wants an inventory note committed as markdown.
- The next session has a confirmed file list.

### Step 0 inventory

Both repos are clean on `rfc-35` branches. The full touchpoint map follows.

**`## Scenarios` H2 (contradictory — D1 targets):**

| File | Line(s) | What |
| --- | --- | --- |
| `plugins/spec/references/synthesis/substeps.md` | 29 | The source of friction F1; says scenarios live under `## Scenarios` H2 |
| `plugins/spec/references/synthesis/claim-reconciliation.md` | 12 | `criterion` kind landing describes `## Scenarios` H2 as fallback |
| `tests/fixtures/skills/refine/combined-docs-and-legacy/expected/spec.md` | 15 | Fixture uses `## Scenarios` H2 |

**`## Motivation` / `## Scope` (proposal sections — D5 targets):**

| File | Line(s) | What |
| --- | --- | --- |
| `plugins/spec/references/synthesis/substeps.md` | 14 | Prescribes `## Motivation`, `## Scope`, `## Non-goals` |
| `plugins/spec/references/synthesis/claim-reconciliation.md` | 14, 22, 59 | `section` and `intent` kind landing says `proposal.md ## Motivation` |
| `tests/fixtures/skills/refine/*/expected/proposal.md` | (all 5) | All use `## Motivation` / `## Scope` |

**`## Features` (Vectis-specific — D5 rename to `## Units`):**

| File | Line(s) | What |
| --- | --- | --- |
| `adapters/targets/vectis/briefs/shape.md` | 22 | `## Features` lists business adapters |
| `tests/fixtures/targets/vectis/task-list/input/proposal.md` | 15 | Uses `## Features` |

**Root `spec.md` path (D4 — must become `specs/<unit>/spec.md`):**

| File | Line(s) | What |
| --- | --- | --- |
| `adapters/targets/omnia/briefs/build.md` | 10 | `$SPEC_PATH = $SLICE_DIR/spec.md` variable assignment |
| `adapters/targets/omnia/briefs/build/crate.md` | (multiple) | Generic `spec.md` reading references |
| `adapters/targets/omnia/briefs/build/test.md` | (multiple) | `spec.md` scenario references |
| `adapters/targets/omnia/briefs/shape.md` | 61-63 | `### spec.md` subsection |
| `adapters/targets/contracts/briefs/shape.md` | 3, 9, 34, 41 | Generic `spec.md` references |
| `adapters/targets/contracts/briefs/build.md` | 18, 24 | `spec.md` reading instructions |
| `adapters/targets/contracts/briefs/build/openapi.md` | 9 | "the slice's `spec.md`" |
| `adapters/targets/contracts/briefs/build/asyncapi.md` | 9 | "the slice's `spec.md`" |
| `adapters/targets/contracts/briefs/build/json-schema.md` | 9 | "the slice's `spec.md`" |

**CLI validator (`## Crates` → `## Units` rename — D5):**

| File | What |
| --- | --- |
| `crates/domain/src/validate/registry/proposal.rs` | `proposal_crates_listed` fn, `## Crates` heading check, rule id `proposal.crates-listed` |
| `crates/domain/src/validate/registry/cross.rs` | `cross_proposal_crates_have_specs` fn, rule id, "crates" in detail message |
| `crates/domain/src/validate/primitives.rs` | `extract_deliverables(proposal, "## Crates")`, unit tests with `## Crates` content |
| `crates/domain/src/validate.rs` | doc comment on `CrossRule` references `cross.proposal-crates-have-specs` |
| `crates/domain/tests/fixtures/change-good/proposal.md` | `## Crates` |
| `crates/domain/tests/fixtures/change-bad/proposal.md` | `## Crates` |
| `tests/fixtures/e2e/good-slice/proposal.md` | `## Crates` |
| `tests/fixtures/e2e/bad-slice/proposal.md` | `## Crates` |
| `crates/domain/tests/fixtures/change-good.golden.json` | `proposal.crates-listed`, `cross.proposal-crates-have-specs` |
| `crates/domain/tests/fixtures/change-bad.golden.json` | `proposal.crates-listed`, `cross.proposal-crates-have-specs`, "crates" detail |
| `tests/fixtures/e2e/goldens/validate-good.json` | `proposal.crates-listed`, `cross.proposal-crates-have-specs` |
| `tests/fixtures/e2e/goldens/validate-bad.json` | `proposal.crates-listed`, `cross.proposal-crates-have-specs`, "crates" detail |

**CLI resolver (`briefs-dir` — D9):**

| File | What |
| --- | --- |
| `src/runtime/commands.rs` | `ResolveBody` struct (add `briefs_dir` field), `write_resolve_text`, `resolve_adapter` |
| `tests/source.rs` | Add assertion for `briefs-dir` existence, absoluteness, `briefs` suffix |
| `tests/target.rs` | Same assertion pattern |

**CLI file-location diagnostic (`specs.file-location` — D8):**

| File | What |
| --- | --- |
| `src/runtime/commands/slice/validate.rs` | Add `specs.file-location` check before provenance drift |
| `crates/domain/src/slice/provenance.rs` | Refine `slice-provenance-drift` message wording |
| `tests/slice.rs` | Add tests for root `spec.md` → `specs.file-location` diagnostic |

**Not affected (false positives confirmed):**

- `adapters/targets/contracts/briefs/build.md` `## Scope` is the build brief's own scope, not a proposal section.
- `change.md` fixtures use `## Scope` — change-level artifact, not `proposal.md`.
- `adapters/targets/vectis/adapter.yaml` mentions `spec.md` in its description — acceptable generic vocabulary.
- `adapters/targets/contracts/references/*-conventions.md` `## Scope Boundary` is a conventions section, not a proposal section.

## Step 1 - Core Synthesis Reference Contract

Goal: make the workflow-owned synthesis references say one thing about scenario headings, proposal sections, and spec layout.

Primary files:

- [`plugins/spec/references/synthesis/substeps.md`](../../plugins/spec/references/synthesis/substeps.md)
- [`plugins/spec/references/synthesis/requirement-block.md`](../../plugins/spec/references/synthesis/requirement-block.md)
- [`plugins/spec/references/synthesis/claim-reconciliation.md`](../../plugins/spec/references/synthesis/claim-reconciliation.md)
- [`plugins/spec/references/synthesis/README.md`](../../plugins/spec/references/synthesis/README.md)

Actions:

- Replace `## Scenarios` H2 guidance with inline `#### Scenario:` H4 guidance using WHEN/THEN examples.
- In `claim-reconciliation.md` line 12, update the `criterion` kind's fallback landing from `## Scenarios` H2 to `#### Scenario:` H4 inline within the parent requirement block.
- In `claim-reconciliation.md` lines 14, 22, 59, update `proposal.md ## Motivation` references to `proposal.md ## Why`.
- Standardize proposal guidance on `## Why`, `## Units`, and `## Non-goals`.
- Standardize spec output on `specs/<unit>/spec.md`, with units declared one-to-one under `## Units`.
- Keep [`plugins/spec/references/spec-format.md`](../../plugins/spec/references/spec-format.md) as the canonical heading reference; update only if a contradiction is found.

Exit criteria:

- Focused `rg` no longer finds contradictory `## Scenarios` or `## Motivation` / `## Scope` guidance in synthesis references.
- Run `make check` if this step leaves the repo internally consistent; otherwise record expected downstream fixture failures for Step 2.

## Step 2 - Refine Skill, Shared Docs, and Refine Fixtures

Goal: make `/spec:refine` and its tests follow the new core contract before target-specific brief work begins.

Primary files:

- [`plugins/spec/skills/refine/SKILL.md`](../../plugins/spec/skills/refine/SKILL.md)
- [`docs/reference/slice-skills/refine.md`](../../docs/reference/slice-skills/refine.md)
- [`plugins/spec/rules/spec.mdc`](../../plugins/spec/rules/spec.mdc)
- [`tests/fixtures/skills/refine/`](../../tests/fixtures/skills/refine/)
- Shared artifact docs such as [`docs/reference/artifact-format.md`](../../docs/reference/artifact-format.md) and [`docs/explanation/augentic-specify-usage.md`](../../docs/explanation/augentic-specify-usage.md)

Actions:

- Add `spec-format.md` to the refine skill references.
- Update refine step 4 to write `proposal.md`, `specs/<unit>/spec.md`, `design.md`, `tasks.md`, and `provenance.yaml`.
- Update validation failure wording to mention `specs/<unit>/spec.md`, not root `spec.md`.
- Move/update refine expected fixtures from root `expected/spec.md` to `expected/specs/<unit>/spec.md` and update proposal fixtures from `## Scope`/`## Crates`/`## Features` to `## Units`.

Exit criteria:

- Refine fixtures reflect the new layout and section names.
- `make check` is attempted; any failures should point only to target brief/doc references scheduled for Steps 3-5.

### Step 2 session notes

Completed. `make check` passes with zero failures.

Files changed:

- `plugins/spec/skills/refine/SKILL.md` — step 4 now writes `specs/<unit>/spec.md`; added `spec-format.md` to References; closing hints updated.
- `docs/reference/slice-skills/refine.md` — step 4 artifact path, closing hint, and error table updated.
- `plugins/spec/rules/spec.mdc` — refine breakout summary updated.
- `docs/reference/artifact-format.md` — "Crates (or Adapters)" → "Units"; "per adapter" → "per unit".
- `docs/explanation/augentic-specify-usage.md` — "Crates" section → "Units"; "per adapter or crate" → "per unit".
- `tests/fixtures/skills/refine/README.md` — layout template uses `specs/<unit>/spec.md`; fixed pre-existing source typo in matrix.
- All 5 fixture `expected/proposal.md` — `## Motivation` → `## Why`, `## Scope` → `## Units` (with `- <slug> — <summary>` bullet format).
- All 5 fixture `expected/spec.md` — moved to `expected/specs/<unit>/spec.md`. Unit slugs: `user-list`, `user-registration`, `password-reset`, `password-reset-expiry`, `audit-trail-retention`.
- `combined-docs-and-legacy` spec fixture — converted `## Scenarios` H2 to inline `#### Scenario:` H4 headings per the Step 1 reference correction.

Observations for future steps:

- `docs/explanation/augentic-specify-usage.md` retains "Crate Name" vocabulary in spec format example templates (lines 58, 63, 95). These are Omnia-specific display examples appropriate for Step 4.
- No changes required to the plan sequencing or scope of Steps 3–9.

## Step 3 - Vectis Target Brief Alignment

Goal: keep Vectis-specific behavior while making it obey the workflow-owned artifact contract.

Primary files:

- [`adapters/targets/vectis/briefs/shape.md`](../../adapters/targets/vectis/briefs/shape.md)
- [`adapters/targets/vectis/briefs/build.md`](../../adapters/targets/vectis/briefs/build.md)
- [`adapters/targets/vectis/briefs/build/test.md`](../../adapters/targets/vectis/briefs/build/test.md)
- [`adapters/targets/vectis/briefs/build/ios/write.md`](../../adapters/targets/vectis/briefs/build/ios/write.md)
- [`adapters/targets/vectis/briefs/build/android/write.md`](../../adapters/targets/vectis/briefs/build/android/write.md)
- [`tests/fixtures/targets/vectis/task-list/`](../../tests/fixtures/targets/vectis/task-list/)

Actions:

- Rename the proposal `## Features` contract to `## Units`; describe each Vectis unit as a business feature.
- Keep Vectis-only sections such as `## Source` and `## Platforms` as additional sections, not replacements for core sections.
- Ensure build/test/write briefs refer to `specs/<unit>/spec.md` and existing traceability examples remain valid.

Exit criteria:

- Focused `rg` in `adapters/targets/vectis` no longer finds target-owned replacements for core section names.
- `make check` is attempted and Vectis fixtures are updated if required.

### Step 3 session notes

Completed. `make check` passes with zero failures. Focused `rg` confirms no `## Features`, `FEATURE_NAME`, `specs/<feature>`, or `<feature_snake>` references remain in `adapters/targets/vectis/` or `tests/fixtures/targets/vectis/`.

Files changed:

- `adapters/targets/vectis/briefs/shape.md` — `## Features` → `## Units`; "business adapters" → "business features"; `specs/<feature>/spec.md` → `specs/<unit>/spec.md`; "Modified Features" → "Modified units".
- `adapters/targets/vectis/briefs/build.md` — `FEATURE_NAME` symbol → `UNIT_NAME`; "feature" → "unit" in description.
- `adapters/targets/vectis/briefs/build/composition.md` — `specs/<feature>/spec.md` → `specs/<unit>/spec.md`.
- `adapters/targets/vectis/briefs/build/core/write.md` — `${FEATURE_NAME}` → `${UNIT_NAME}` in spec path.
- `adapters/targets/vectis/briefs/build/test.md` — traceability comment `<feature>` → `<unit>`; baseline capture path `${FEATURE_NAME}` → `${UNIT_NAME}`.
- `adapters/targets/vectis/references/test-spec-mapping.md` — all `<feature>` / `<feature_snake>` path templates and naming conventions → `<unit>` / `<unit_snake>`.
- `adapters/targets/vectis/references/test-runbook.md` — `$FEATURE_NAME` argument → `$UNIT_NAME`; all traceability and naming convention references updated.
- `adapters/targets/vectis/references/review/logic-checks.md` — traceability comment template `specs/<feature>/spec.md` → `specs/<unit>/spec.md`.
- `tests/fixtures/targets/vectis/task-list/input/proposal.md` — `## Features` / `### New Features` / `### Modified Features` → `## Units` / `### New Units` / `### Modified Units`.
- `tests/fixtures/targets/vectis/task-list/input/spec.md` — moved to `input/specs/task-list/spec.md` to match canonical layout.
- `tests/fixtures/targets/vectis/task-list/README.md` — layout diagram updated for `specs/task-list/spec.md` path; shape brief cross-reference updated.
- `tests/fixtures/targets/vectis/task-list/expected/shape-evidence.md` — file path references updated; "feature" → "unit" in workflow vocabulary.
- `tests/fixtures/targets/vectis/task-list/expected/composition.yaml` — header comment updated for new spec path.

No changes to `build/ios/write.md`, `build/android/write.md`, or `merge.md` (no `feature`/`FEATURE` workflow vocabulary present).

Additional files beyond the plan's primary list: `test-spec-mapping.md`, `test-runbook.md`, and `review/logic-checks.md` required updates because they contained `FEATURE_NAME`, `<feature_snake>`, and `specs/<feature>/spec.md` templates that directly support the build briefs. These are Vectis-specific references, so they fall within Step 3's scope.

Observations for future steps:

- No changes required to the plan sequencing or scope of Steps 4–9.

## Step 4 - Omnia Target Brief Alignment

Goal: map Omnia units to crates or service surfaces without using root `spec.md` or `## Crates` as workflow vocabulary.

Primary files:

- [`adapters/targets/omnia/briefs/shape.md`](../../adapters/targets/omnia/briefs/shape.md)
- [`adapters/targets/omnia/briefs/build.md`](../../adapters/targets/omnia/briefs/build.md)
- [`adapters/targets/omnia/briefs/build/crate.md`](../../adapters/targets/omnia/briefs/build/crate.md)
- [`adapters/targets/omnia/briefs/build/test.md`](../../adapters/targets/omnia/briefs/build/test.md)
- Other Omnia build/review briefs that cite `spec.md` directly.

Actions:

- Define Omnia `## Units` guidance: for a single generated crate, the unit normally equals the crate name; for broader work, the unit is the service surface slug.
- Replace root `$SLICE_DIR/spec.md` style references with `specs/<unit>/spec.md`.
- Use target-specific wording only inside explanatory prose; validator-facing names stay `Units` and `specs/<unit>/spec.md`.

Exit criteria:

- Focused `rg` in `adapters/targets/omnia` shows no root `spec.md` instruction and no validator-facing `## Crates` contract.
- `make check` is attempted.

### Step 4 session notes

Completed. `make check` passes with zero failures. Focused `rg` confirms no `$SLICE_DIR/spec.md`, root `spec.md` instruction, or `## Crates` references remain in `adapters/targets/omnia/`.

Files changed:

- `adapters/targets/omnia/briefs/shape.md` — added `## Omnia units` section with guidance on how units map to crates or service surfaces; updated intro to reference `specs/<unit>/spec.md`; renamed `### spec.md` subsection to `### specs/<unit>/spec.md`; updated WASM guardrails and tasks.md sections.
- `adapters/targets/omnia/briefs/build.md` — replaced `$SPEC_PATH = $SLICE_DIR/spec.md` with `$SPEC_PATH = $SLICE_DIR/specs/$UNIT_NAME/spec.md`; added `$UNIT_NAME` binding; updated intro, phase order step 1, and regression check prose.
- `adapters/targets/omnia/briefs/build/crate.md` — updated all `spec.md` references to `specs/<unit>/spec.md` (intro, authority hierarchy rule 1, critical path step 1).
- `adapters/targets/omnia/briefs/build/test.md` — updated all `spec.md` references to `specs/<unit>/spec.md` (intro, authority hierarchy, test generation steps 1/3, quality checklist).
- `adapters/targets/omnia/references/cross-cutting-matrices.md` — updated traceability verification instruction from `spec.md` to `specs/<unit>/spec.md`.
- `docs/explanation/augentic-specify-usage.md` — "Crate Name" → "Unit Name" in spec template heading; "Baseline / New Crate" → "Baseline / New Unit"; "Modified Crate" → "Modified Unit"; "what this crate or adapter does" → "what this unit does".

Files not changed (confirmed no workflow vocabulary issues): `build/guest.md` (no `spec.md` references), `build/review.md` (no `spec.md` references), `build/replay.md` (no `spec.md` references), `references/spec-to-test-mapping.md` (already uses `specs/<unit>/spec.md` paths correctly).

Observations for future steps:

- No changes required to the plan sequencing or scope of Steps 5–9.

## Step 5 - Contracts Target Brief Alignment

Goal: map contracts units to contract surfaces while preserving OpenAPI/AsyncAPI/JSON Schema build flows.

Primary files:

- [`adapters/targets/contracts/briefs/shape.md`](../../adapters/targets/contracts/briefs/shape.md)
- [`adapters/targets/contracts/briefs/build.md`](../../adapters/targets/contracts/briefs/build.md)
- [`adapters/targets/contracts/briefs/build/openapi.md`](../../adapters/targets/contracts/briefs/build/openapi.md)
- [`adapters/targets/contracts/briefs/build/asyncapi.md`](../../adapters/targets/contracts/briefs/build/asyncapi.md)
- [`adapters/targets/contracts/briefs/build/json-schema.md`](../../adapters/targets/contracts/briefs/build/json-schema.md)

Actions:

- Define contract `## Units` guidance: HTTP API, event family, or schema vocabulary slugs map to `specs/<unit>/spec.md`.
- Replace root `spec.md` reading instructions with canonical unit spec paths.
- Preserve contract-specific author/import/verify mode guidance.

Exit criteria:

- Focused `rg` in `adapters/targets/contracts` shows no root `spec.md` instruction.
- `make check` passes in `augentic/specify`, or remaining failures are documented as out of scope.

### Step 5 session notes

Completed. `make check` passes with zero failures. Focused `rg` confirms every `spec.md` reference in `adapters/targets/contracts/` now uses the canonical `specs/<unit>/spec.md` path — no bare root `spec.md` instructions remain.

Files changed:

- `adapters/targets/contracts/briefs/shape.md` — added `## Contract units` section with guidance on how units map to contract surfaces (HTTP API domain, event family, schema vocabulary scope); updated intro to reference `specs/<unit>/spec.md`; updated `## What core synthesises` bullet from `spec.md` to `specs/<unit>/spec.md`; updated `## Source-driven authoring vs import` provenance reference; updated `## What synthesis MUST NOT do` inline-YAML prohibition.
- `adapters/targets/contracts/briefs/build.md` — updated `## Inputs` from `spec.md` to `specs/<unit>/spec.md` with "(one file per `proposal.md ## Units` entry)" clarification; updated closing guidance from "synthesised `spec.md`" to "synthesised `specs/<unit>/spec.md` files".
- `adapters/targets/contracts/briefs/build/openapi.md` — updated critical path step 1 from "the slice's `spec.md`" to "the slice's `specs/<unit>/spec.md` files".
- `adapters/targets/contracts/briefs/build/asyncapi.md` — updated critical path step 1 from "the slice's `spec.md`" to "the slice's `specs/<unit>/spec.md` files".
- `adapters/targets/contracts/briefs/build/json-schema.md` — updated critical path step 1 from "the slice's `spec.md`" to "the slice's `specs/<unit>/spec.md` files".

Files not changed (confirmed no workflow vocabulary issues): `merge.md` (no `spec.md` references), `references/*-conventions.md` (`## Scope Boundary` is a conventions section, not a proposal section), `references/` directory (no `spec.md` references).

Observations for future steps:

- No changes required to the plan sequencing or scope of Steps 6–9.

## Step 6 - Resolver JSON `briefs-dir` Output

Goal: make `specrun source resolve --format json` and `specrun target resolve --format json` self-contained for brief discovery.

Primary files in `augentic/specify-cli`:

- `src/runtime/commands.rs`
- `tests/source.rs`
- `tests/target.rs`
- `docs/standards/handler-shape.md`

Actions:

- Add a kebab-case `briefs-dir` field to the resolve JSON body.
- Populate it as an absolute path to the resolved adapter manifest's `briefs/` directory.
- Keep the change additive for existing parsers; optionally mirror the field in text output if that matches local command conventions.
- Add source and target integration assertions that the field exists, is absolute, and ends in `briefs`.

Exit criteria:

- Source and target resolve tests pass.
- Run `cargo make check` if feasible; otherwise run the narrow integration tests and record why the full check was deferred.

### Step 6 session notes

Completed. `cargo make check` passes with zero failures. Source and target resolve integration tests both verify the new `briefs-dir` field is present, absolute, and correctly formed.

Files changed:

- `src/runtime/commands.rs` — added `briefs_dir: String` field to `ResolveBody` struct (serialises as `briefs-dir` via `rename_all = "kebab-case"`); populated in both `Axis::Source` and `Axis::Target` arms of `resolve_adapter` as `location.path().join("briefs")`; added `briefs-dir:` line to `write_resolve_text` for text-format output.
- `tests/source.rs` — added assertions that `briefs-dir` ends with `/briefs`, contains the expected adapter path segment, and is absolute (verified against raw stdout before tempdir substitution).
- `tests/target.rs` — same assertion pattern for the target adapter resolve test.

The change is strictly additive: existing JSON parsers that do not read `briefs-dir` are unaffected. The text output gains one new `briefs-dir:` line after `location:`.

Observations for future steps:

- No changes required to the plan sequencing or scope of Steps 7–9.

## Step 7 - Proposal `Units` Validator Rename

Goal: align CLI validation rule names and parsing with RFC-35 D5.

Primary files in `augentic/specify-cli`:

- `crates/domain/src/validate/registry/proposal.rs`
- `crates/domain/src/validate/registry/cross.rs`
- `crates/domain/src/validate/primitives.rs`
- `crates/domain/src/validate.rs` (doc comment on `CrossRule` struct references old rule id)
- Validator fixtures and goldens under `crates/domain/tests/` and integration fixtures under `tests/` (4 proposal fixtures + 4 golden JSONs — see Step 0 inventory)

Actions:

- Rename `proposal.crates-listed` to `proposal.units-listed` and parse `## Units` bullets.
- Rename `cross.proposal-crates-have-specs` to `cross.proposal-units-have-specs` and map each unit to `specs/<unit>/spec.md`.
- Update diagnostics to use target-neutral wording: `unit` and `spec file`, not `crate`.
- Update the `CrossRule` doc comment in `validate.rs` to reference the new rule id.
- Update all affected fixtures and golden outputs.

Exit criteria:

- Validator unit/integration tests pass.
- Golden outputs no longer mention the old crate-specific rule IDs unless they are historical docs.

### Step 7 session notes

Completed. `cargo make check` passes with zero failures (1105 tests passed, 1 skipped). All validator rule IDs, descriptions, detail messages, fixtures, and goldens updated from crate-specific to target-neutral unit vocabulary.

Files changed:

- `crates/domain/src/validate/registry/proposal.rs` — renamed `proposal_crates_listed` → `proposal_units_listed`; rule ID `proposal.crates-listed` → `proposal.units-listed`; description `Crates` → `Units`; checks `## Units` instead of `## Crates`.
- `crates/domain/src/validate/registry/cross.rs` — renamed `cross_proposal_crates_have_specs` → `cross_proposal_units_have_specs`; rule ID `cross.proposal-crates-have-specs` → `cross.proposal-units-have-specs`; description and detail wording `crate` → `unit`.
- `crates/domain/src/validate/primitives.rs` — `extract_deliverables` call changed from `"## Crates"` to `"## Units"`; doc comments updated; `extract_deliverables` doc comment `### New Crates` → `### New Units`; all three unit tests (`deliverable_specs_on_disk`, `deliverables_backticked_names`, absent-section) updated to use `## Units` / `### New Units`.
- `crates/domain/src/validate.rs` — `CrossRule` doc comment example updated from `cross.proposal-crates-have-specs` to `cross.proposal-units-have-specs`.
- `crates/domain/tests/fixtures/change-good/proposal.md` — `## Crates` → `## Units`, `### New Crates` → `### New Units`.
- `crates/domain/tests/fixtures/change-bad/proposal.md` — same section rename.
- `tests/fixtures/e2e/good-slice/proposal.md` — same section rename.
- `tests/fixtures/e2e/bad-slice/proposal.md` — same section rename.
- `crates/domain/tests/fixtures/change-good.golden.json` — rule ID, description updated for both `proposal.units-listed` and `cross.proposal-units-have-specs`.
- `crates/domain/tests/fixtures/change-bad.golden.json` — rule ID, description, detail updated for both rules.
- `tests/fixtures/e2e/goldens/validate-good.json` — rule ID, description updated for both rules.
- `tests/fixtures/e2e/goldens/validate-bad.json` — rule ID, description, detail updated for both rules.

Observations for future steps:

- The bad-slice proposal fixtures retain `missing-crate` as a unit slug in their `## What Changes` prose — this is fixture-specific content, not validator vocabulary, so it was left unchanged. The validator correctly treats it as a unit name and checks `specs/missing-crate/spec.md`.
- No changes required to the plan sequencing or scope of Steps 8–9.

## Step 8 - Spec File-Location Diagnostics

Goal: make `specrun slice validate` report the file-location problem before misleading provenance drift or heading-format messages.

Primary files in `augentic/specify-cli`:

- `src/runtime/commands/slice/validate.rs`
- `crates/domain/src/slice/provenance.rs`
- `crates/domain/src/validate/primitives.rs`
- `crates/domain/src/validate/run.rs`
- `tests/slice.rs`

Actions:

- Add rule `specs.file-location` when no canonical `specs/**/*.md` files are found but root `spec.md` exists.
- Put the corrective action in the existing validation summary shape, likely `detail`, because the current envelope has no separate `hint` field.
- Refine `slice-provenance-drift` wording so missing headings and wrong file location are distinguishable.
- Add tests for root `spec.md` with no canonical specs, and update existing provenance drift message tests.

Exit criteria:

- `tests/slice.rs` covers the new `specs.file-location` diagnostic.
- Provenance drift tests assert the new non-misleading messages.
- `cargo make check` is attempted.

### Step 8 session notes

Completed. `cargo make check` passes with zero failures (302 tests passed, 1 skipped). The new `specs.file-location` diagnostic fires before provenance drift and all three integration test cases pass.

Files changed:

- `src/runtime/commands/slice/validate.rs` — added `collect_spec_file_location_findings` function emitting `specs.file-location` when root `spec.md` exists but no canonical `specs/<unit>/spec.md` files found; integrated into `validate_pre_adapter_gates` as gate #1 (before provenance drift); updated doc comment to document five gates instead of four.
- `crates/domain/src/slice/provenance.rs` — refined `ProvenanceDrift::into_summary` messages: `MissingProvenanceRequirement` now says "appears in spec files under `specs/`" instead of "appears in spec.md"; `ExtraProvenanceRequirement` now says "no requirement block with `ID: {req_id}` exists in any spec file under `specs/`" instead of "no matching `REQ-*` heading exists in spec.md"; `rule` description updated to "stays in sync with specs/ REQ ids" for consistency.
- `tests/slice.rs` — added three integration tests: `validate_emits_file_location_when_root_spec_md_exists_but_no_canonical_specs` (positive case), `validate_does_not_emit_file_location_when_canonical_specs_exist` (canonical specs present + stale root copy), `validate_does_not_emit_file_location_when_no_root_spec_md` (empty slice with neither root nor canonical specs).

Implementation note: the plan listed `crates/domain/src/validate/primitives.rs` and `crates/domain/src/validate/run.rs` as primary files. Neither required changes — the file-location check lives in the CLI handler (`validate.rs`) because it needs filesystem stat calls on the slice directory, which the domain-layer runner already delegates to the CLI-level pre-adapter gates. The `validate_slice` domain function already handles `specs/**/*.md` via its glob expansion; the file-location gate runs upstream of that.

Observations for future steps:

- No changes required to the plan sequencing or scope of Step 9.

## Step 9 - Cross-Repo Acceptance and Operator Notes

Goal: prove the full RFC-35 behavior and leave a concise handoff for operators.

Actions:

- Run `make check` in `augentic/specify`.
- Run `cargo make check` or `cargo make ci` in `augentic/specify-cli`; prefer `cargo make ci` before merge if time and toolchain allow.
- Manually verify resolver JSON includes `briefs-dir` for one source adapter and one target adapter.
- Manually verify a misplaced root `spec.md` produces `specs.file-location` rather than only a provenance drift heading error.
- Update the progress tracker and summarize any remaining non-goals: no `provenance.yaml` writer, no generic journal emitter, no schema changes.

Exit criteria:

- Both repos have clean intended diffs.
- Acceptance commands and any skipped checks are recorded in the final session handoff.

### Step 9 session notes

Completed. All acceptance criteria met.

**Acceptance results:**

- `make check` in `augentic/specify` — passed ("All checks passed."). First run hit a transient build-cache filesystem error (`failed to write bytecode … No such file or directory`); retry succeeded cleanly.
- `cargo make ci` in `augentic/specify-cli` — passed (302 tests, 1 skipped; lint, test, test-docs, doc, vet, outdated, deny all green).
- Resolver `briefs-dir` — verified via `source_resolve_local_returns_resolved_manifest` and `target_resolve_local_returns_resolved_manifest` integration tests. Both assert the field is present, absolute, ends with `/briefs`, and contains the correct adapter path segment.
- `specs.file-location` diagnostic — verified via three integration tests: positive case (root `spec.md` without canonical `specs/<unit>/spec.md` emits the diagnostic with `specs/<unit>/spec.md` guidance and `slice root` mention), and two negative cases (canonical specs present silences it; no root `spec.md` at all silences it).
- Both repos are clean on `rfc-35` branches with no untracked or unstaged changes.

**Commit history:**

`augentic/specify` (`rfc-35` branch, 12 commits ahead of `main`):

- Steps 0–5: synthesis reference corrections, refine skill/fixture updates, Vectis/Omnia/contracts brief alignment.
- Step 8: implementation plan session notes (specify-side documentation of the CLI step 8 work).
- RFC and implementation plan authoring.

`augentic/specify-cli` (`rfc-35` branch, 3 commits ahead of `main`):

- Step 6: `briefs-dir` field in resolver JSON output.
- Step 7: `proposal.units-listed` / `cross.proposal-units-have-specs` validator rename and fixture updates.
- Step 8: `specs.file-location` diagnostic gate and provenance drift message refinement.

**Remaining non-goals (unchanged from RFC):**

- No `specrun slice provenance` verb — `provenance.yaml` authoring stays skill-driven with existing schema + drift validation.
- No `specrun journal emit` verb — journal events stay owned by deterministic CLI commands.
- No Evidence schema, `provenance.yaml` schema, or slice lifecycle changes.
- No new workspace crates or public API surfaces.

**Skipped checks:** None. `cargo make ci` was run (the full CI suite), not the smaller `cargo make check` subset.
