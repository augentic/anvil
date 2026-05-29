# RFC-35 Implementation Plan

This plan implements [RFC-35: Synthesis Determinism](rfc-35-synthesis-determinism.md) in small sequential sessions across `augentic/specify` and `augentic/specify-cli`. Each session can assume preceding steps are already implemented and should update the progress tracker before handing off.

## Summary Progress Tracker

Status values: `not-started`, `in-progress`, `done`, `blocked`.

```text
Step  Session                                             Repo(s)                 Status       Primary verification
0     Preflight and baseline inventory                    specify, specify-cli    done         git status + focused rg map
1     Core synthesis reference contract                   specify                 done         focused rg + make check (passed)
2     Refine skill, shared docs, and refine fixtures      specify                 not-started  make check or fixture-specific checks
3     Vectis target brief alignment                       specify                 not-started  make check + Vectis fixture checks
4     Omnia target brief alignment                        specify                 not-started  make check
5     Contracts target brief alignment                    specify                 not-started  make check
6     Resolver JSON briefs-dir output                     specify-cli             not-started  source/target resolve tests
7     Proposal Units validator rename                     specify-cli             not-started  validator tests + goldens
8     Spec file-location diagnostics                      specify-cli             not-started  slice validate tests
9     Cross-repo acceptance and operator notes            both                    not-started  make check + cargo make check/ci
```

## Sequencing Rules

Start every session with `git status` in the relevant repo and preserve unrelated user changes. Do not mix `specify` and `specify-cli` edits in the same session except Step 9 acceptance notes. When a step touches `specify-cli`, also read that repo's `AGENTS.md` and the relevant standards document before editing Rust.

## Step 0 - Preflight and Baseline Inventory

Goal: confirm the current repo state and produce a short implementation note for the next agent.

Actions:

- In `augentic/specify`, inspect `git status` and focused hits for `## Scenarios`, `#### Scenario`, `## Crates`, `## Features`, `## Units`, and root-level `spec.md` wording.
- In `augentic/specify-cli`, inspect resolver output code, proposal/cross validator rules, fusion drift messages, and existing tests.
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
| `plugins/spec/references/synthesis/claim-fusion.md` | 12 | `criterion` kind landing describes `## Scenarios` H2 as fallback |
| `tests/fixtures/skills/refine/combined-docs-and-legacy/expected/spec.md` | 15 | Fixture uses `## Scenarios` H2 |

**`## Motivation` / `## Scope` (proposal sections — D5 targets):**

| File | Line(s) | What |
| --- | --- | --- |
| `plugins/spec/references/synthesis/substeps.md` | 14 | Prescribes `## Motivation`, `## Scope`, `## Non-goals` |
| `plugins/spec/references/synthesis/claim-fusion.md` | 14, 22, 59 | `section` and `intent` kind landing says `proposal.md ## Motivation` |
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
| `src/runtime/commands/slice/validate.rs` | Add `specs.file-location` check before fusion drift |
| `crates/domain/src/slice/fusion.rs` | Refine `slice-fusion-drift` message wording |
| `tests/slice.rs` | Add tests for root `spec.md` → `specs.file-location` diagnostic |

**Not affected (false positives confirmed):**

- `adapters/targets/contracts/briefs/build.md` `## Scope` is the build brief's own scope, not a proposal section.
- `change.md` fixtures use `## Scope` — change-level artifact, not `proposal.md`.
- `adapters/targets/vectis/adapter.yaml` mentions `spec.md` in its description — acceptable generic vocabulary.
- `adapters/targets/contracts/references/*-conventions.md` `## Scope Boundary` is a conventions section, not a proposal section.

## Step 1 - Core Synthesis Reference Contract

Goal: make the workflow-owned synthesis references say one thing about scenario headings, proposal sections, and spec layout.

Primary files:

- [`plugins/spec/references/synthesis/substeps.md`](../plugins/spec/references/synthesis/substeps.md)
- [`plugins/spec/references/synthesis/requirement-block.md`](../plugins/spec/references/synthesis/requirement-block.md)
- [`plugins/spec/references/synthesis/claim-fusion.md`](../plugins/spec/references/synthesis/claim-fusion.md)
- [`plugins/spec/references/synthesis/README.md`](../plugins/spec/references/synthesis/README.md)

Actions:

- Replace `## Scenarios` H2 guidance with inline `#### Scenario:` H4 guidance using WHEN/THEN examples.
- In `claim-fusion.md` line 12, update the `criterion` kind's fallback landing from `## Scenarios` H2 to `#### Scenario:` H4 inline within the parent requirement block.
- In `claim-fusion.md` lines 14, 22, 59, update `proposal.md ## Motivation` references to `proposal.md ## Why`.
- Standardize proposal guidance on `## Why`, `## Units`, and `## Non-goals`.
- Standardize spec output on `specs/<unit>/spec.md`, with units declared one-to-one under `## Units`.
- Keep [`plugins/spec/references/spec-format.md`](../plugins/spec/references/spec-format.md) as the canonical heading reference; update only if a contradiction is found.

Exit criteria:

- Focused `rg` no longer finds contradictory `## Scenarios` or `## Motivation` / `## Scope` guidance in synthesis references.
- Run `make check` if this step leaves the repo internally consistent; otherwise record expected downstream fixture failures for Step 2.

## Step 2 - Refine Skill, Shared Docs, and Refine Fixtures

Goal: make `/spec:refine` and its tests follow the new core contract before target-specific brief work begins.

Primary files:

- [`plugins/spec/skills/refine/SKILL.md`](../plugins/spec/skills/refine/SKILL.md)
- [`docs/reference/slice-skills/refine.md`](../docs/reference/slice-skills/refine.md)
- [`plugins/spec/rules/spec.mdc`](../plugins/spec/rules/spec.mdc)
- [`tests/fixtures/skills/refine/`](../tests/fixtures/skills/refine/)
- Shared artifact docs such as [`docs/reference/artifact-format.md`](../docs/reference/artifact-format.md) and [`docs/explanation/augentic-specify-usage.md`](../docs/explanation/augentic-specify-usage.md)

Actions:

- Add `spec-format.md` to the refine skill references.
- Update refine step 4 to write `proposal.md`, `specs/<unit>/spec.md`, `design.md`, `tasks.md`, and `fusion.yaml`.
- Update validation failure wording to mention `specs/<unit>/spec.md`, not root `spec.md`.
- Move/update refine expected fixtures from root `expected/spec.md` to `expected/specs/<unit>/spec.md` and update proposal fixtures from `## Scope`/`## Crates`/`## Features` to `## Units`.

Exit criteria:

- Refine fixtures reflect the new layout and section names.
- `make check` is attempted; any failures should point only to target brief/doc references scheduled for Steps 3-5.

## Step 3 - Vectis Target Brief Alignment

Goal: keep Vectis-specific behavior while making it obey the workflow-owned artifact contract.

Primary files:

- [`adapters/targets/vectis/briefs/shape.md`](../adapters/targets/vectis/briefs/shape.md)
- [`adapters/targets/vectis/briefs/build.md`](../adapters/targets/vectis/briefs/build.md)
- [`adapters/targets/vectis/briefs/build/test.md`](../adapters/targets/vectis/briefs/build/test.md)
- [`adapters/targets/vectis/briefs/build/ios/write.md`](../adapters/targets/vectis/briefs/build/ios/write.md)
- [`adapters/targets/vectis/briefs/build/android/write.md`](../adapters/targets/vectis/briefs/build/android/write.md)
- [`tests/fixtures/targets/vectis/task-list/`](../tests/fixtures/targets/vectis/task-list/)

Actions:

- Rename the proposal `## Features` contract to `## Units`; describe each Vectis unit as a business feature.
- Keep Vectis-only sections such as `## Source` and `## Platforms` as additional sections, not replacements for core sections.
- Ensure build/test/write briefs refer to `specs/<unit>/spec.md` and existing traceability examples remain valid.

Exit criteria:

- Focused `rg` in `adapters/targets/vectis` no longer finds target-owned replacements for core section names.
- `make check` is attempted and Vectis fixtures are updated if required.

## Step 4 - Omnia Target Brief Alignment

Goal: map Omnia units to crates or service surfaces without using root `spec.md` or `## Crates` as workflow vocabulary.

Primary files:

- [`adapters/targets/omnia/briefs/shape.md`](../adapters/targets/omnia/briefs/shape.md)
- [`adapters/targets/omnia/briefs/build.md`](../adapters/targets/omnia/briefs/build.md)
- [`adapters/targets/omnia/briefs/build/crate.md`](../adapters/targets/omnia/briefs/build/crate.md)
- [`adapters/targets/omnia/briefs/build/test.md`](../adapters/targets/omnia/briefs/build/test.md)
- Other Omnia build/review briefs that cite `spec.md` directly.

Actions:

- Define Omnia `## Units` guidance: for a single generated crate, the unit normally equals the crate name; for broader work, the unit is the service surface slug.
- Replace root `$SLICE_DIR/spec.md` style references with `specs/<unit>/spec.md`.
- Use target-specific wording only inside explanatory prose; validator-facing names stay `Units` and `specs/<unit>/spec.md`.

Exit criteria:

- Focused `rg` in `adapters/targets/omnia` shows no root `spec.md` instruction and no validator-facing `## Crates` contract.
- `make check` is attempted.

## Step 5 - Contracts Target Brief Alignment

Goal: map contracts units to contract surfaces while preserving OpenAPI/AsyncAPI/JSON Schema build flows.

Primary files:

- [`adapters/targets/contracts/briefs/shape.md`](../adapters/targets/contracts/briefs/shape.md)
- [`adapters/targets/contracts/briefs/build.md`](../adapters/targets/contracts/briefs/build.md)
- [`adapters/targets/contracts/briefs/build/openapi.md`](../adapters/targets/contracts/briefs/build/openapi.md)
- [`adapters/targets/contracts/briefs/build/asyncapi.md`](../adapters/targets/contracts/briefs/build/asyncapi.md)
- [`adapters/targets/contracts/briefs/build/json-schema.md`](../adapters/targets/contracts/briefs/build/json-schema.md)

Actions:

- Define contract `## Units` guidance: HTTP API, event family, or schema vocabulary slugs map to `specs/<unit>/spec.md`.
- Replace root `spec.md` reading instructions with canonical unit spec paths.
- Preserve contract-specific author/import/verify mode guidance.

Exit criteria:

- Focused `rg` in `adapters/targets/contracts` shows no root `spec.md` instruction.
- `make check` passes in `augentic/specify`, or remaining failures are documented as out of scope.

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

## Step 8 - Spec File-Location Diagnostics

Goal: make `specrun slice validate` report the file-location problem before misleading fusion drift or heading-format messages.

Primary files in `augentic/specify-cli`:

- `src/runtime/commands/slice/validate.rs`
- `crates/domain/src/slice/fusion.rs`
- `crates/domain/src/validate/primitives.rs`
- `crates/domain/src/validate/run.rs`
- `tests/slice.rs`

Actions:

- Add rule `specs.file-location` when no canonical `specs/**/*.md` files are found but root `spec.md` exists.
- Put the corrective action in the existing validation summary shape, likely `detail`, because the current envelope has no separate `hint` field.
- Refine `slice-fusion-drift` wording so missing headings and wrong file location are distinguishable.
- Add tests for root `spec.md` with no canonical specs, and update existing fusion drift message tests.

Exit criteria:

- `tests/slice.rs` covers the new `specs.file-location` diagnostic.
- Fusion drift tests assert the new non-misleading messages.
- `cargo make check` is attempted.

## Step 9 - Cross-Repo Acceptance and Operator Notes

Goal: prove the full RFC-35 behavior and leave a concise handoff for operators.

Actions:

- Run `make check` in `augentic/specify`.
- Run `cargo make check` or `cargo make ci` in `augentic/specify-cli`; prefer `cargo make ci` before merge if time and toolchain allow.
- Manually verify resolver JSON includes `briefs-dir` for one source adapter and one target adapter.
- Manually verify a misplaced root `spec.md` produces `specs.file-location` rather than only a fusion drift heading error.
- Update the progress tracker and summarize any remaining non-goals: no `fusion.yaml` writer, no generic journal emitter, no schema changes.

Exit criteria:

- Both repos have clean intended diffs.
- Acceptance commands and any skipped checks are recorded in the final session handoff.
