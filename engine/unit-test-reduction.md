# Engine unit-test reduction — findings & plan

> Status: Proposal (read-only triage; no tests changed yet). All verdicts are coverage-gated — `cargo llvm-cov nextest --workspace --summary-only` TOTAL is the final arbiter at execution time, and a DELETE may downgrade to COLLAPSE if a line turns out to be load-bearing.

Companion to [`docs/standards/testing.md`](standards/testing.md) (integration-first posture: default to deletion; unit tests only for CLI-unreachable branches or dense pure matrices) and the adapters standard at [`specify-adapters/docs/standards/testing.md`](https://github.com/augentic/specify-adapters/blob/main/docs/standards/testing.md).

## 1. Why so many unit tests remain

The earlier sweep ran two different scopes:

- **Adapters (`specify-adapters`) — reduction.** `vectis` verify (17 unit tests re-homed to integration) and `contracts` (16 → 4 collapsed). `vectis` still has ~84 unit tests in its other modules (`materialize`, `app_icon`, `svg`, `canvas`, `icons`) that were not touched.
- **Engine (`specify/engine`) — decoupling only.** Per the chosen scope (`decouple_primary`, count reduction *secondary*), every engine crate had real adapter names in fixtures renamed to contrived placeholders (`demo-target`, `demo-source`, `demo-tool`, `ORG-*`, …). No engine unit test was deleted, collapsed, or re-homed — only string literals inside them changed.

So the engine's unit-test count is essentially unchanged from before. This document plans the actual reduction.

## 2. Current inventory

`#[test]` / `#[tokio::test]` counts (source-side unit vs `tests/` integration).

| Area | Unit (`src`) | Integration (`tests/`) |
| --- | --: | --: |
| engine `standards` | 193 | — |
| engine `workflow` | 188 | — |
| engine `registry` | 64 | — |
| engine `extension-manifest` | 28 | — |
| engine `schema` | 18 | — |
| engine binary `src` | 13 | — |
| engine `model` | 2 | — |
| **engine total** | **506** | **708** |
| adapters `vectis` | 84 | — |
| adapters `contracts` | 4 | — |
| **adapters total** | **88** | **167** |

Coverage baselines (whole-workspace TOTAL, `cargo llvm-cov nextest --workspace --summary-only`): **engine 82.83% line / 82.52% region**; **adapters 88.35% line / 86.74% region**. These must not drop.

## 3. Rubric & constraints

Assign one verdict per unit test (a file may split):

- **Delete** — behavior already asserted by an existing integration test. Must cite the covering `file::fn`.
- **Collapse** — a dense pure `(input → output/findings/code)` matrix becomes one table-driven `#[test]` (stays unit, fewer declarations, coverage-neutral by construction).
- **Re-home** — behavior reachable through the crate's **public** API, not already covered, moved to `crates/<crate>/tests/`.
- **Keep** — a genuinely CLI-unreachable defensive branch / error variant no public input can trigger, or a private-fn matrix that cannot collapse further. Add a one-line justification.

Hard constraints:

1. **Coverage gate.** `cargo llvm-cov nextest --workspace --summary-only` TOTAL line/region must hold at every step. A delete that drops coverage means the test was load-bearing → collapse or re-home instead.
2. **No visibility widening for tests.** A test exercising a **private** (`pub(crate)` / `pub(super)` / private) item cannot re-home — integration tests are a separate crate. Such tests are Delete / Collapse / Keep only.
3. **The CLI e2e deliberately defers per-`kind` eval semantics** (noted in `engine/tests/lint.rs`). The `standards/src/lint/eval/*` arms must be collapsed or kept, never deleted without a table replacement.

## 4. Bottom line

| | standards | workflow | combined |
| --- | --- | --- | --- |
| Unit tests today | 193 | 188 | **381** |
| Delete | 9 | 28 | **37** |
| Collapse | 137 → ~35 | 88 → ~18 | **225 → ~53** (saves ~172) |
| Re-home | 19 | 29 | **48** |
| Keep | 28 | 43 | **71** |

Net effect across the two crates: `src` unit tests drop **381 → ~124** (~67% fewer); 48 relocate to `tests/`; **~209 `#[test]` declarations removed workspace-wide** (37 deletes + ~172 collapse savings; re-homes are count-neutral relocations). These two crates are 75% of the engine's 506 src unit tests.

## 5. Findings — `specify-standards` (193 across 32 files)

Integration baseline: `crates/standards/tests/{lint_index(10),lint_hint(13),lint_engine_guards(6)}`; CLI e2e `engine/tests/{lint.rs(17),rules.rs(12)}`.

| File | n | Verdict |
| --- | --: | --- |
| `rules/resolve/tests.rs` | 20 | Re-home 18 (pub `resolve`/`map_resolve_error`) + Delete 2 |
| `lint/framework_tools/scenarios/catalog.rs` | 15 | Collapse → ~3 |
| `rules/resolve/filter/tests.rs` | 15 | Collapse → ~3 |
| `rules/parse/tests.rs` | 14 | Collapse → ~3 (1 private helper stays) |
| `lint/eval/cli_contract.rs` | 9 | Collapse 6 + Keep 3 (unsupported arms) |
| `lint/eval/set_coverage.rs` | 7 | Collapse → 2 |
| `lint/eval/cardinality.rs` | 6 | Collapse → 2 |
| `lint/eval/presence.rs` | 6 | Collapse → 2 |
| `lint/eval/schema.rs` | 6 | Collapse → 2 |
| `lint/eval/unique.rs` | 6 | Collapse → 2 |
| `lint/eval/finding.rs` | 6 | Keep (≥128 KiB clamp paths not reachable via public input) |
| `lint/framework_tools/rules.rs` | 6 | Collapse → 2 |
| `lint/framework_tools/scenarios.rs` | 6 | Collapse → 2 |
| `rules/resolve/sort/tests.rs` | 6 | Collapse 3→1 + Delete 2 + Re-home 1 |
| `lint/eval/constant_eq.rs` | 5 | Collapse → 2 |
| `lint/eval/fenced_block.rs` | 5 | Collapse → 2 |
| `lint/eval/field_grammar.rs` | 5 | Collapse → 2 |
| `lint/eval/reference_resolves.rs` | 5 | Collapse → 2 |
| `lint/eval/path_pattern.rs` | 5 | Collapse → 1 |
| `lint/eval/tool.rs` | 5 | Keep (fake-runner branches not in WASI path) |
| `lint/framework_tools/links_registry.rs` | 5 | Collapse → 1 |
| `lint/framework_tools/marketplace.rs` | 5 | Delete 1 + Collapse 4→1 |
| `lint/framework_tools.rs` | 4 | Delete 2 + Keep 2 (registry/inventory + unknown-checker dispatch) |
| `lint/framework_tools/extension.rs` | 4 | Delete 2 + Keep 2 |
| `lint/framework_tools/prose.rs` | 4 | Delete 1 + Collapse 3 |
| `lint/eval/regex.rs` | 3 | Keep (suffix-guard / binary-skip / compile-fail CLI-unreachable) |
| `lint/framework_tools/skill_body.rs` | 3 | Collapse → 1 |
| `lint/eval/cross_reference.rs` | 2 | Keep (already minimal) |
| `lint/framework_tools/support.rs` | 2 | Keep (Road B plumbing, not crate-public) |
| `lint/diagnostics.rs` | 1 | Keep (exhaustive D8 error-variant table) |
| `lint/eval/regex/config.rs` | 1 | Keep (private config parser edge) |
| `lint/eval/regex/logical_lines.rs` | 1 | Keep (private continuation lexer) |

### standards Delete cross-reference

| Unit test | Covered by |
| --- | --- |
| `resolve/tests.rs::duplicate_rule_id_errors` | `engine/tests/lint.rs::framework::duplicate_rule_id_aborts_fatally` |
| `resolve/tests.rs::rules_root_required_when_no_probe` | `engine/tests/rules.rs::export::negative_rules_root_required` |
| `sort/tests.rs::paths_anchored_not_absolute` | `engine/tests/rules.rs::export::paths_anchored_not_absolute` |
| `sort/tests.rs::build_byte_stable` | `engine/tests/rules.rs::export::stable_ordering_byte_identical` |
| `framework_tools.rs::marketplace_skips_absent_manifest` | `engine/tests/lint.rs::framework_adapters::adapters_only_root_lints_clean` |
| `framework_tools.rs::extension_silent_on_tree_without_declarations` | `engine/tests/lint.rs::framework_adapters::adapters_only_root_lints_clean` |
| `marketplace.rs::absent_manifest_is_silent` | `engine/tests/lint.rs::framework_adapters::adapters_only_root_lints_clean` |
| `extension.rs::declared_missing_crate_dir_is_flagged` | `engine/tests/lint.rs::framework_adapters::extension_rule_fires_for_missing_crate` |
| `prose.rs::absent_standards_doc_is_silent` | `engine/tests/lint.rs::framework_adapters::adapters_only_root_lints_clean` |

## 6. Findings — `specify-workflow` (188 across 20 files)

Integration baseline: `crates/workflow/tests/**` (12 files) + `engine/tests/**` CLI e2e (`plan.rs`, `slice.rs`, `init.rs`, `journal.rs`, `workspace.rs`, `adapter.rs`, `catalog_infer.rs`, `e2e.rs`, `bootstrap.rs`, `registry.rs`, `target.rs`, …).

| File | n | Verdict |
| --- | --: | --- |
| `change/plan/core/status/tests.rs` | 25 | Delete 9 + Keep 16 (journal/resume overlay) |
| `change/plan/core/model/tests.rs` | 20 | Collapse 18→2 + Keep 2 (`plan_finding_*`) |
| `design_system/tests.rs` | 19 | Collapse (~9 components ↔ `catalog_infer.rs`) + Re-home 5 (`parts_*`) |
| `init/adapter_uri/tests.rs` | 15 | Collapse 12→3 + Keep 2 (store-resolve) |
| `change/plan/core/validate/tests.rs` | 13 | Delete 8 + Keep 5 |
| `agents/fences/parse.rs` | 11 | Collapse → 1 |
| `merge/composition/tests.rs` | 11 | Re-home (pub merge kernel; delta matrix not in CLI) → collapse to 2 |
| `slice/build/wire/tests.rs` | 10 | Delete 4 + Collapse 3→1 + Keep 2 (output gate) |
| `adapter/core/tests.rs` | 9 | Collapse 9→3 (gate fns `pub(super)` → can't re-home) |
| `decisions/tests.rs` | 9 | Re-home 8 (pub `promote`) + Keep 1 (private `dec_number`) |
| `agents/fingerprint.rs` | 8 | Collapse 5→1 + Keep 1 (programmer-error branch) |
| `agents/detect/markers.rs` | 7 | Collapse → 2 |
| `agents/fences/render.rs` | 7 | Re-home (pub `plan_agents_write`) → collapse to 2 |
| `agents/render.rs` | 6 | Collapse → 1–2 |
| `agents/lock.rs` | 5 | Re-home 3 + Keep 2 (version gate, test-only diff helper) |
| `change/plan/core/propose/topology.rs` | 3 | Re-home (pub `apply_greenfield_seed`) |
| `slice/build/materialize_scope/needs_materialize.rs` | 3 | Delete (covered by `materialize_scope.rs`) |
| `change/plan/doctor/cycle.rs` | 3 | Keep (proptests beyond CLI spot-check) |
| `agents/detect.rs` | 2 | Keep (deterministic ordering + corrupt-marker warn) |
| `platform.rs` | 2 | Collapse 1 + Delete 1 (`parse_csv_edge_cases`) |

### workflow Delete cross-reference (selected)

| Unit test | Covered by |
| --- | --- |
| `needs_materialize.rs::{exports_present,ios_export_missing,empty_scope}` | `crates/workflow/tests/materialize_scope.rs::{needs_false_ios_imageset,needs_true_vector_no_export,needs_false_for_empty_scope}` |
| `validate/tests.rs::clean_plan_validates` | `engine/tests/plan.rs::plan_validate_clean_json` |
| `validate/tests.rs::duplicate_name_error` | `engine/tests/plan.rs::plan_validate_with_errors_json` |
| `validate/tests.rs::cycle_detection` | `engine/tests/plan.rs::validate_reports_all_health_diagnostics` |
| `validate/tests.rs::unknown_source_error` | `engine/tests/plan.rs::validate_reports_all_health_diagnostics` |
| `validate/tests.rs::source_key_uniqueness` (dup-key half) | `engine/tests/plan.rs::amend_add_source_duplicate_key_rejected` |
| `validate/tests.rs::authority_override_checks` | `engine/tests/plan.rs::plan_validate_authority_override_orphan` |
| `platform.rs::parse_csv_edge_cases` (reject half) | `engine/tests/init.rs::init_platforms_not_allowed_errors` |
| `status/tests.rs::pending_plan_stops` | `engine/tests/plan.rs::status_pending_plan_stops` |
| `status/tests.rs::fresh_active_refines` | `engine/tests/plan.rs::status_active_refine_json` |
| `status/tests.rs::lifecycle_dispatch` | `engine/tests/plan.rs::status_built_slice_dispatches_merge` |
| `status/tests.rs::drained_when_all_done` | `engine/tests/plan.rs::status_drained_renders_finalize_hint` |
| `status/tests.rs::awaited_build_failure_stops` | `engine/tests/plan.rs::status_build_failure_stops` |
| `status/tests.rs::eligible_pending_previews_refine` | `engine/tests/plan.rs::plan_next_picks_first_pending_json` |
| `wire/tests.rs::coherence_flags_mismatches` | `engine/tests/slice.rs::finalize_warns_unexpected_composition` |
| `wire/tests.rs::coherence_reads_delta_envelope` | `engine/tests/plan.rs::composition_inference_capstone` |
| `wire/tests.rs::blocking_finding_gate` | `engine/tests/slice.rs` build-finalize blocking-finding test |
| `design_system/tests.rs` (components subset ~9) | `engine/tests/catalog_infer.rs::{bind_persists_fingerprint,bind_rejects_non_hex_fingerprint,…}` |

## 7. Execution plan

Each pass is one reviewable PR, scoped to a single crate/area, and must end green on both gates:

```bash
cargo llvm-cov nextest --workspace --summary-only   # TOTAL line/region >= baseline
cargo make ci                                        # fmt-check, clippy -Dwarnings, nextest, docs, vet, deny
```

Per-test mechanical checklist:

1. Read the unit test and (for Delete) the cited integration test; confirm the same observable is asserted.
2. Apply the verdict:
   - **Delete:** remove the test (or the whole `#[cfg(test)] mod tests` if every test in it is Delete).
   - **Collapse:** build table-driven test(s) with one row per original input — every original input must be represented so coverage is neutral (see `specify-contract` `validate.rs` for the pattern).
   - **Re-home:** move to `crates/<crate>/tests/<area>.rs` exercising only `pub` API; wire via `#[path = "<area>.rs"] mod <area>;` in the crate's `tests/it.rs`; delete the `src` copy. Do **not** widen visibility.
   - **Keep:** add a one-line comment stating why an agent cannot get the signal from integration, if missing.
3. `cargo +nightly fmt -p <crate>` then `cargo clippy -p <crate> --all-targets --all-features --locked -- -D warnings`.
4. `cargo nextest run -p <crate>`, then the two gates above.

Suggested ordering (highest ROI / lowest risk first):

| Phase | Scope | Expected effect |
| --- | --- | --- |
| 1 | `workflow` status (25) + model (20) | 9 deletes + ~36 collapse savings; heavy `plan.rs` overlap, low risk |
| 2 | `workflow` deletes: validate (8), wire (4), needs_materialize (3), platform (1) | ~16 deletes, all cited |
| 3 | `workflow` re-homes: composition merge, decisions promote, agents lock/render, topology, design_system `parts_*` | ~29 relocated to `tests/`; pub-kernel coverage moves where it belongs |
| 4 | `workflow` remaining collapses: adapter/core, init/adapter_uri, fences/parse, fingerprint, detect/markers, render | ~30 collapse savings |
| 5 | `standards` deletes + `resolve` re-home (18) + `resolve` deletes (2) + framework_tools deletes | 9 deletes + 19 re-homes |
| 6 | `standards` `eval/*` collapses + framework_tools collapses + `scenarios/catalog` | ~100 collapse savings (the bulk); **collapse only — never delete eval arms** |
| 7 | Extend the same triage to the untriaged crates: `registry` (64), `extension-manifest` (28), `schema` (18), binary `src` (13), `model` (2), and adapters `vectis` (84) | follow-up sweep |

## 8. Risks & invariants

- **Green at every step** — both gates per PR. Coverage is the brake, not test count.
- **`eval/*` is collapse-only** — the CLI e2e deliberately does not assert per-`kind` eval semantics; deleting these without a table replacement drops coverage.
- **Private items cannot re-home** — `framework_tools` is a private module and `eval::evaluate` arms are `pub(crate)`; `adapter/core` and `init/adapter_uri` gate fns are `pub(super)`. Collapse or keep, do not widen visibility.
- **Keep set is legitimate (~71)** — evidence-clamp paths, fake-`ToolRunner` branches, journal/resume overlays, output-gate subcases, proptests, private parser helpers. The goal is not zero unit tests; it is no *redundant* or *integration-reachable* ones.
- **Re-homes are count-neutral** — they relocate coverage from `src` to `tests/`, they do not reduce total `#[test]` count.
