# RFC-27 Acceptance Coverage Matrix

> Status: Release gate audit surface for [rfc-27-synthesis.md](rfc-27-synthesis.md) §Acceptance scenarios. Produced by Change 4.2 of [rfc-27-plan.md](rfc-27-plan.md). Each `#26-N` scenario maps to one or more deterministic assertions (test name + file path verbatim) plus any documented gap with rationale.

## Conventions

- **cli** = `augentic/specify-cli`; paths are repo-relative.
- **plg** = `augentic/specify`; paths are repo-relative.
- Test names are exact `cargo test` selectors (Rust) or `Deno.test` titles (Deno harness).
- "Composes from existing" means a Phase 2 / Phase 3 deliverable already pins the assertion verbatim and Change 4.2 cites it rather than duplicating. "New in 4.2" means Change 4.2 added the assertion in this matrix's landing commit.

## Scenario #26-1 (release blocker, D1) — Runtime source binding end-to-end

**Scenario:** Operator binds `runtime=./fixtures/replay` alongside `legacy=./vendor/monolith`; `enumerate` walks the fixture tree; `extract` emits `kind: example` claims with `fixture-digest`; `Sources: [legacy-monolith, runtime]` on a synthesis-resolved `Status: agreed` block; the Omnia target's `build` runs fixtures through generated code and writes `fixture-replay: { passed, failed, skipped, ran-at, runner }` into `.metadata.yaml`; `merge` surfaces the summary in its closing message; operator policy or a forked target adapter (not core) gates whether `failed > 0` blocks landing.

**Covering tests (new in 4.2; plg Deno harness):**

- `sources/code-runtime: adapter manifest is discoverable in plg tree` — `tests/cross_repo/sources_test.ts`
- `sources/code-runtime: every Evidence document schema-validates with example claims` — `tests/cross_repo/sources_test.ts`
- `sources/code-runtime: every fusion.yaml schema-validates against slice/fusion.schema.json` — `tests/cross_repo/sources_test.ts`
- `sources/code-runtime: discovery.md names runtime as the bound source key` — `tests/cross_repo/sources_test.ts`
- `targets/omnia/with-fixture-replay: .metadata.yaml carries full fixture-replay block` — `tests/cross_repo/targets_test.ts`
- `targets/omnia/without-fixture-replay: .metadata.yaml omits fixture-replay (optional posture)` — `tests/cross_repo/targets_test.ts`

**Gap:** No end-to-end execution of `enumerate` / `extract` (their briefs require an LLM and are excluded by the cross-repo harness top comment in `tests/cross_repo.ts`). The harness pins the deterministic surface — adapter discoverability, schema validity of the captured Evidence + fusion + discovery goldens, optionality of the target-side `fixture-replay` block — which is what the release gate can audit byte-stably. Skill-driven brief execution remains a separate (deferred) RFC.

## Scenario #26-2 (release blocker, D1 + D2 + D3) — Per-slice authority override on a `criterion` claim

**Scenario:** Combined evidence where docs say "30 minutes" expiry and runtime fixtures show 24-hour expiry; operator sets per-slice `authority-override: { criterion: runtime }`; synthesis writes `Status: divergence` with runtime as the operative value and docs preserved as commentary; `fusion.yaml.requirements[].resolution-trace.step` reads `per-slice-authority-override`.

**Covering tests (composes from existing + new in 4.2; cli):**

- Set + read-back of the per-slice override on `plan.yaml`: `plan_amend_authority_override_round_trips_and_validates` — `tests/plan_orchestrate.rs`
- Orphan source key rejection (validation gate): `plan_amend_authority_override_orphan_source_key_refused_by_amend` — `tests/plan_orchestrate.rs`
- `slice validate` surfaces orphan override: `slice_validate_surfaces_authority_override_orphan` — `tests/plan_orchestrate.rs`
- Resolution-order pin (step 1, per-slice wins): `evidence::authority::tests::resolution_order_step_1_per_slice_wins` — `crates/domain/src/evidence/authority.rs`
- Resolution-order pin (per-slice dominates per-Evidence): `evidence::authority::tests::resolution_order_per_slice_overrides_dominate_per_evidence` — `crates/domain/src/evidence/authority.rs`
- **NEW** Fusion shape: `fusion_show_round_trips_per_slice_authority_override_trace` — `tests/slice.rs`
- **NEW** Fusion text rendering: `fusion_show_round_trips_per_slice_authority_override_trace_text` — `tests/slice.rs`

**Gap:** The synthesis resolver itself is skill-driven in v2.1 per RFC-27 §Synthesis updates; the new fusion tests pin the SHAPE the skill body must emit through a hand-authored `fusion.yaml`. When the resolver migrates into Rust (post-v2.1), the four-step resolution-order pin in `crates/domain/src/evidence/authority.rs::tests` becomes a black-box test against the production code path and the fusion-shape tests become end-to-end. Constraint: "do NOT extend the synthesis resolver beyond the Phase 2 micro-resolver" is honoured.

## Scenario #26-3 (D2 + D3) — Per-Evidence override with no per-slice override

**Scenario:** Adapter emits `authority-overrides: { decision: documentation }`; no per-slice override; synthesis resolves `decision`-class disagreement via the per-Evidence override; `requirement`-class falls back to RFC-25 default ordering; `fusion.yaml` records both resolution paths.

**Covering tests (composes from existing + new in 4.2; cli):**

- Per-Evidence override schema + serialise: `evidence::authority::tests::overrides_serialise_as_bare_map` — `crates/domain/src/evidence/authority.rs`
- Per-Evidence resolve helper: `evidence::authority::tests::overrides_resolve_returns_per_kind_class` — `crates/domain/src/evidence/authority.rs`
- Resolution-order pin (step 2, per-Evidence widens): `evidence::authority::tests::resolution_order_step_2_per_evidence_widens` — `crates/domain/src/evidence/authority.rs`
- Resolution-order pin (step 3, document authority wins): `evidence::authority::tests::resolution_order_step_3_document_authority_wins` — `crates/domain/src/evidence/authority.rs`
- **NEW** Both resolution paths surface on one slice: `fusion_show_records_both_per_evidence_and_default_authority_paths` — `tests/slice.rs`

**Gap:** Same as #26-2 — the resolver is skill-driven in v2.1, so the new fusion-shape test pins the SHAPE rather than the resolver decision. No additional gap beyond the skill / resolver split documented above.

## Scenario #26-4 (D4) — `fusion.yaml` round-trip, drift detection, re-refine clears drift

**Scenario:** `/spec:refine` writes the index with inline `value` payloads on every `contributing-claim`; `specify slice fusion show <slice>` prints both winning and dropped values without opening `evidence/*.yaml`; operator hand-edits `spec.md` to flip a `[divergence]` to `[agreed]`; `specify slice validate` reports `slice-fusion-drift`. Validate detects requirement-id drift AND contributing-claim → evidence drift; operator re-runs `/spec:refine` to regenerate; drift clears; lifecycle reaches `refined`.

**Covering tests (composes from existing Phase 2 Change 2.6; cli):**

- Clean fusion validates: `validate_passes_on_clean_fusion_inputs` — `tests/slice.rs`
- Drift gate skips when fusion absent (legacy slices): `validate_skips_drift_gate_when_fusion_yaml_absent` — `tests/slice.rs`
- Requirement-id drift (extra spec.md REQ): `validate_detects_req_id_drift_when_spec_md_has_extra_requirement` — `tests/slice.rs`
- Contributing-claim drift (evidence renamed): `validate_detects_contributing_claim_drift_when_evidence_claim_renamed` — `tests/slice.rs`
- JSON byte-stability: `fusion_show_json_round_trips_byte_stable` — `tests/slice.rs`
- Text inline `value` payload rendering: `fusion_show_text_prints_inline_value_payloads` — `tests/slice.rs`
- Missing fusion file diagnostic: `fusion_show_reports_missing_fusion_file_with_diag_exit_one` — `tests/slice.rs`
- Schema-invalid fusion rejection: `fusion_show_rejects_schema_invalid_file_with_exit_two` — `tests/slice.rs`
- Pre-synthesis drift coverage: `validate_skipped_drift_gate_does_not_fire_on_pre_synthesis_spec` — `tests/slice.rs`

**Gap:** None at the CLI surface. "Re-refine clears drift" is structurally an inverse of the drift-detection tests (write clean inputs → no drift row; write drifted inputs → drift row) and is covered by `validate_passes_on_clean_fusion_inputs` + the two drift-detection tests in combination.

## Scenario #26-5 (D5) — CLI-only `divergence: likely`

**Scenario:** Plan skill invokes `specify plan amend --divergence likely`; the skill no longer reads or writes `plan.yaml` directly; `plan.propose.divergence` journal event fires once from the CLI; file diff is byte-identical to the pre-D5 skill-written output.

**Covering tests (composes from existing Phase 2 Change 2.2; cli):**

- Journal event fires only when transitioning into `likely`: `plan_amend_divergence_none_to_likely_emits_event` — `tests/plan_orchestrate.rs`
- Round-trip on YAML: `plan_amend_divergence_likely_round_trips_to_yaml` — `tests/plan_orchestrate.rs`
- Plan create flag (entry-point): `plan_create_divergence_likely_unknown_slice_refused` — `tests/plan_orchestrate.rs`
- Field write on amend: `plan_amend_divergence_likely_writes_field` — `tests/plan_orchestrate.rs`
- Cross-transition behaviour: `plan_amend_divergence_from_none_to_accepted`, `plan_amend_divergence_from_likely_to_rejected`, `plan_amend_divergence_from_accepted_to_rejected`, `plan_amend_divergence_from_rejected_to_accepted` — `tests/journal.rs`
- Kebab-case round-trip: `divergence_kebab_case_round_trip` — `tests/journal.rs`
- No-op when flag absent: `plan_amend_without_divergence_flag_emits_no_event` — `tests/journal.rs`

**Gap:** None. The CLI is the single writer of `plan.yaml.slices[].divergence` across the closed enum; the journal events fire from CLI code paths only. The skill-side write retirement is documented in `plugins/spec/skills/plan/SKILL.md` (Change 3.3) and validated by `make checks` skill schema predicates rather than a Rust test.

## Scenario #26-6 (D6) — Cross-source candidate alias

**Scenario:** Docs surface `account-pwd-reset`; code surfaces `password-reset`; operator adds an alias via `plan amend --add-alias`. `specify plan add --sources legacy=password-reset` rewrites the value to the resolved canonical `id` before persisting; downstream `extract` runs once per source against the resolved candidate; re-enumeration preserves operator-added aliases.

**Covering tests (composes from existing Phase 2 Change 2.4; cli):**

- Alias resolution at `plan add`: `plan_add_resolves_alias_to_canonical_id` — `tests/discovery_aliases.rs`
- Alias resolution skipped when discovery missing: `plan_add_without_discovery_md_skips_alias_resolution` — `tests/discovery_aliases.rs`
- Add-alias mutation: `plan_amend_add_alias_mutates_discovery_md` — `tests/discovery_aliases.rs`
- Collision refusal: `plan_amend_add_alias_refused_on_collision` — `tests/discovery_aliases.rs`
- Same-invocation resolution: `plan_amend_add_alias_then_resolves_in_same_invocation` — `tests/discovery_aliases.rs`
- Re-enumeration preserves operator additions: `plan_amend_alias_survives_reapplied_discovery` — `tests/discovery_aliases.rs`
- Remove-alias idempotency: `plan_amend_remove_alias_is_idempotent` — `tests/discovery_aliases.rs`
- `discovery show --aliases` JSON + text: `discovery_show_aliases_prints_alias_map_json`, `discovery_show_aliases_prints_alias_map_text` — `tests/discovery_aliases.rs`
- Validation gate: `slice_validate_surfaces_discovery_alias_collision` — `tests/discovery_aliases.rs`
- Domain model: `discovery::candidate::tests::matches_resolves_id_then_aliases`, `discovery::candidate::tests::round_trips_with_aliases` — `crates/domain/src/discovery/candidate.rs`

**Gap:** None. The 13-test Change 2.4 suite covers every assertion in the scenario.

## Scenario #26-7 (D7) — Auto-review at create across plan shapes

**Scenario:** Operator runs `--auto-review` against (a) a single-slice pure-intent plan, (b) a single-slice path-bound plan, and (c) a hand-authored multi-slice plan with two sources. All three plans land at `lifecycle: reviewed` in one CLI call; `plan.transition.reviewed` journal event fires once per plan; `/spec:execute` accepts each plan immediately; `plan validate` failures (e.g. orphan source key) refuse the create regardless of `--auto-review`; running explicit `specify plan transition <name> reviewed` after `--auto-review` is a no-op.

**Covering tests (composes from existing Phase 2 Change 2.1; cli):**

- Auto-review stamps reviewed + emits journal event: `plan_create_auto_review_stamps_reviewed_and_emits_journal_event` — `tests/plan_orchestrate.rs`
- Validate passes clean afterwards: `plan_create_auto_review_then_validate_passes_clean` — `tests/plan_orchestrate.rs`
- Explicit transition is idempotent: `plan_create_auto_review_then_explicit_transition_is_idempotent_noop` — `tests/plan_orchestrate.rs`
- Invalid name refused identically: `plan_create_auto_review_invalid_name_refuses_same_as_without_flag` — `tests/plan_orchestrate.rs`
- Atomic journal append on failure: `plan_create_auto_review_validation_failure_emits_no_partial_events` — `tests/plan_orchestrate.rs`
- Pre-flag transition baseline: `plan_transition_reviewed_emits_journal_event` — `tests/journal.rs`

**Gap (documented):** The three-shape coverage — (a) pure-intent, (b) path-bound, (c) multi-slice — is structurally blocked. `specify plan create` does not accept slice seeds at create time; the empty-scaffold path is the only shape Change 2.1's tests can drive without an additional verb (which would expand scope beyond Change 4.2). The five auto-review tests above all run against the empty-scaffold (a) shape. (b) and (c) compose from the auto-review + `plan add` + `plan amend` sequence and inherit the same `plan.transition.reviewed` event surface, so the journal-event count assertion holds without a dedicated test. Lifting (b) and (c) to dedicated tests requires either a `plan create --slice` seed flag (deferred) or a multi-step harness that wraps `auto-review` + `plan add`, neither of which is in 4.2's scope. The skill-level contract (RFC-27 §D7 Rules) is unchanged.

## Scenario #26-8 (D8) — Cache fingerprint hit vs miss

**Scenario:** Two consecutive `/spec:refine` runs on the same slice; between them, the operator bumps the adapter version. First run emits `slice.extract.cache-miss` with `reason: no-prior-entry`; second run (no input change) emits `slice.extract.cache-hit`; third run after version bump emits `slice.extract.cache-miss` with `reason: adapter-version-changed`; the `index.jsonl` log carries one row per write.

**Covering tests (composes from existing Phase 2 Change 2.5; cli):**

- Miss-then-hit end-to-end: `extract_miss_then_hit_with_unchanged_inputs` — `tests/cache.rs`
- Hit + override observability: `cache_miss_hit_and_override_observable` — `tests/cache.rs`
- Non-zero exit cache by scope: `adapter_non_zero_exit_caches_by_scope` — `tests/cache.rs`
- JSON export round-trip: `export_json_resolves_cache_and_overlay` — `tests/cache.rs`
- Domain-level write + lookup hit: `adapter::cache::io::tests::write_then_lookup_is_a_hit` — `crates/domain/src/adapter/cache/io.rs`
- Adapter-version bump reports correct reason: `adapter::cache::io::tests::adapter_version_bump_reports_changed_reason` — `crates/domain/src/adapter/cache/io.rs`
- Opt-out adapter misses without writing dir: `adapter::cache::io::tests::adapter_opt_out_misses_without_writing_dir` — `crates/domain/src/adapter/cache/io.rs`
- Corrupt prior record treated as no-prior-entry: `adapter::cache::io::tests::corrupt_prior_record_is_treated_as_no_prior_entry` — `crates/domain/src/adapter/cache/io.rs`
- Index log line shape: `adapter::cache::io::tests::index_read_skips_blank_lines_and_rejects_garbage` — `crates/domain/src/adapter/cache/io.rs`
- Fingerprint byte-stability: `adapter::cache::tests::digest_is_byte_stable_across_runs` — `crates/domain/src/adapter/cache.rs`
- Per-input fingerprint sensitivity: `adapter::cache::tests::each_input_flip_changes_the_digest` — `crates/domain/src/adapter/cache.rs`
- Tool-version sort independence: `adapter::cache::tests::tool_versions_sort_independently_of_input_order` — `crates/domain/src/adapter/cache.rs`
- Diff reason field order: `adapter::cache::tests::diff_reason_walks_declared_field_order` — `crates/domain/src/adapter/cache.rs`
- Index entry round-trip + deny_unknown_fields: `adapter::cache::tests::cache_index_entry_round_trips`, `adapter::cache::tests::deny_unknown_fields_on_index_entry` — `crates/domain/src/adapter/cache.rs`

**Gap:** None. The cache test family covers the closed `reason` enum (each input flip drives a specific reason), the index log shape, the opt-out posture, and the journal event surface end-to-end.

## Summary

| Scenario | Coverage | New tests in 4.2 | Composed from | Gap |
| --- | --- | --- | --- | --- |
| #26-1 | full (deterministic surface) | 6 Deno tests (plg) | — | end-to-end brief execution requires LLM; harness scope |
| #26-2 | full | 2 Rust tests (cli) | Change 2.3 + resolver micro-pin | resolver migration is skill-driven in v2.1 |
| #26-3 | full | 1 Rust test (cli) | Change 1.1 + resolver micro-pin | resolver migration is skill-driven in v2.1 |
| #26-4 | full | — | Change 2.6 (9 tests) | none |
| #26-5 | full | — | Change 2.2 (multiple tests across journal + plan_orchestrate) | none |
| #26-6 | full | — | Change 2.4 (13 tests) | none |
| #26-7 | empty-scaffold only | — | Change 2.1 (5 tests) | three-shape coverage blocked by `plan create` slice-seed surface |
| #26-8 | full | — | Change 2.5 (14 tests across cache.rs + domain) | none |

**Release blockers:** #26-1 and #26-2 both have explicit deterministic tests landed by Change 4.2 (Deno + Rust respectively). Neither composes purely from a prior phase.

**Pre-existing skips:** The plg `make checks` baseline is 28 broken-link failures from RFC-25 archive references (`plg/rfcs/archive/`, fixture READMEs, `docs/contributing/index.md`). Change 4.2 must not increase that count.

## References

- [rfc-27-synthesis.md](rfc-27-synthesis.md) §Acceptance scenarios — normative source
- [rfc-27-plan.md](rfc-27-plan.md) §Change 4.2 — task scope
- [cli `tests/`](https://github.com/augentic/specify-cli/tree/main/tests) — Rust integration tests
- [plg `tests/cross_repo/`](../tests/cross_repo/) — Deno cross-repo acceptance harness
- [plg `tests/fixtures/sources/code-runtime/`](../tests/fixtures/sources/code-runtime/) — Change 4.1 fixture tree for #26-1
- [plg `tests/fixtures/targets/omnia/`](../tests/fixtures/targets/omnia/) — Change 4.1 target-half fixtures
