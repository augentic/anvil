# Improve & Optimise Review — `specify` + `specify-cli`

Comprehensive code, test, and documentation review across both repos, scoped for **improve/optimise** work (no new features). Findings are prioritised and carry concrete file/line anchors so they can be picked up directly.

- Reviewed: `augentic/specify` (~629 markdown, 61 yaml, 5 shell; prompt/docs/adapters) and `augentic/specify-cli` (~80k LOC Rust, 10 crates, ~112 test files).
- Cross-crate dependency invariants in the CLI are **clean** — no `workflow ↔ standards` leakage, `validate` depends only on `model`. The architecture is sound; the work below is refinement, not restructuring.
- Existing tracking (`specify-cli/docs/quality-debt.md`) is healthy: lint suppressions are minimal and triaged. This review extends beyond what that file already covers.

---

## Where to focus first (the short list)

If you only touch five things, do these — highest value-to-effort, all verified against current source:

1. **Cache compiled JSON Schemas** in `specify-schema`. Every `validate_value` call recompiles the schema (`crates/schema/src/validate.rs:123`). This is the single biggest CPU win, hit on `slice validate`, `plan`, `lint`, and adapter resolve. (CLI §1.1)
2. **Make merge baseline writes atomic.** The module doc claims "atomic baseline writes" but the loop uses plain `fs::write` (`crates/workflow/src/merge/slice/write.rs:1` vs `:26`); a crash mid-merge corrupts the baseline. (CLI §2.1)
3. **Close the slice-lifecycle test gap.** The FSM has 2 unit tests for a 5-state machine (`crates/workflow/src/slice/lifecycle/tests.rs`). A table-driven edge matrix is ~30 lines and closes the biggest coverage hole. (Tests §F1)
4. **Reconcile `docs/standards/cli-contract.md` with the live `specrun` surface.** It still documents retired verbs (`specrun slice outcome|journal`, `specify adapter`, `plan lock`, `workspace status`). Operators and agents are reading a stale contract. (Docs §1)
5. **Sweep `specify` → `specrun` binary naming** across operator-facing docs and skill guardrails (~25+ files). The shipped runtime binary is `specrun`; authoring is `specdev`. (Docs §2)

---

## Part A — `specify-cli` code quality & performance

### A1. Performance (verified)

| # | Severity | Finding | Location | Fix |
|---|----------|---------|----------|-----|
| 1.1 | **High** | JSON Schema recompiled on every validation call (no validator cache). Hot on `validate_evidence_dir` (per `evidence/*.yaml`), adapter `run_schema` (2 schemas × 2 loops), `validate_plan_yaml`. | `crates/schema/src/validate.rs:120-133`; callers `crates/workflow/src/schema.rs:310-318,77,439`, `adapter/core.rs:704-717` | `LazyLock<HashMap<&str, Validator>>` in `specify-schema`; expose `validate_value_cached`. The existing `SYNTHESIS_VALIDATOR`/`BUILD_REPORT_VALIDATOR` (`workflow/src/schema.rs:156-221`) are the intended pattern. |
| 1.2 | **High** | Lint `kind: schema` recompiles registered schemas per rule evaluation. | `crates/standards/src/lint/eval/schema.rs:93-133,56-60` | Cache validators keyed by registered id; cache resolved project-relative paths. |
| 1.3 | Med | Diagnostic schema recompiled per finding validation. | `crates/diagnostics/src/validate.rs:92-97` | `static DIAGNOSTIC_VALIDATOR: LazyLock<Validator>`. |
| 1.4 | Med | `slice validate` double-reads/parses every `evidence/*.yaml` (schema pass, then `EvidenceFacts::read` for drift). | `crates/workflow/src/slice/validate.rs:80-82,198,486-516` | Single evidence pass: validate once, stash parsed docs, reuse in drift gates. |
| 1.5 | Med | `LeadCatalog::contains` heap-allocates two `String`s per lookup, called in nested propose loops. | `crates/workflow/src/change/plan/core/propose/catalog.rs:32-34` | Compare `(&str, &str)` against the set without allocating, or intern keys. |
| 1.6 | Med | Identity projection reads the *entire* `journal.jsonl` to keep only the last 10 events; grows linearly with history. | `crates/workflow/src/journal.rs:639-650`, `registry/identity.rs:168-181` | Tail-read last N lines, or maintain a small `recent` sidecar at merge time. |
| 1.7 | Low | Lint index clones `relative`/`language` per file under `rayon`. | `crates/standards/src/lint/index.rs:111-118,186-193` | `Arc<str>` for relative paths in the discovery struct. |

**Suggested order:** 1.1 → 1.2 → 1.3 (one shared caching pattern clears all three), then 1.4 and 1.6.

### A2. Robustness / `.specify/` state integrity

| # | Severity | Finding | Location | Fix |
|---|----------|---------|----------|-----|
| 2.1 | **High** | Merge baseline writes are **non-atomic** despite the module doc saying "atomic". Partial writes leave baseline inconsistent with slice metadata. | `crates/workflow/src/merge/slice/write.rs:17-30`; commit order `merge/slice.rs:203-226` | Use `specify_model::atomic::bytes_write` per file, or stage under `.specify/.merge-staging/<slice>/` then rename-commit (mirror `migrate.rs:526-536`). Fix the docstring either way. |
| 2.2 | Med | Decision-record promotion uses plain `fs::write`; crash mid-merge leaves half-written files. | `crates/workflow/src/decisions.rs:356-367` | `bytes_write` / temp+rename; coordinate with 2.1 staging. |
| 2.3 | Med | Journal lifecycle events silently dropped on append error (logged to stderr, swallowed); verb still exits 0. | `crates/workflow/src/journal.rs:713-717,724-741` | Document as intentional with operator-visible warning, or write a `.specify/journal.dropped` sidecar; consider strict-CI failure. |
| 2.4 | Low | `init` scaffolds `wasm-pkg.toml` with non-atomic write (create-only, lower risk). | `crates/workflow/src/init.rs:208-216` | `bytes_write` for consistency. |

### A3. Maintainability / structure

| # | Severity | Finding | Location | Fix |
|---|----------|---------|----------|-----|
| 3.1 | **High** | `slice/validate.rs` is a 1073-line god module (pre-adapter gates, model drift, decision gates, catalog drift, spec layout, provenance, plus ~90 lines of tests). | `crates/workflow/src/slice/validate.rs` | Split into `validate/{pre_adapter,model_drift,decisions,catalog,spec_location}.rs` + thin orchestrator; move tests to `validate/tests.rs`. |
| 3.2 | Med | REQ/TASK ID grammar duplicated across three layers (drift risk). | `slice/validate.rs:418-476` (manual), `crates/validate/src/primitives.rs` (regex), `crates/model/src/spec.rs` (`REQ_ID_PATTERN`) | Single authority in `specify-model` (`fn is_req_id`); call from workflow + validate. |
| 3.3 | Med | Large modules still due decomposition: `framework/check/skill_body.rs` (667), `lint/eval.rs` (643), `framework/check/scenarios.rs` (556), `lint/model.rs` (475). | `crates/standards/src/...` | Extract eval arms into submodules; split `skill_body` by concern. |
| 3.4 | Med | Archaeology (RFC/Phase prose) in production module docs — already a standards policy violation that `rust_source.rs` flags. | `crates/standards/src/framework.rs:1-33`, `workflow/src/journal.rs` (e.g. 87-93,508-510), `slice/validate.rs:6-7,146,418` | Trim `//!`/`///` to ≤3 lines of current behaviour; move history to `DECISIONS.md`. Matches `quality-debt.md:25-27`. |
| 3.5 | Low | Framework crate module-level `#![allow]` (pedantic/missing_docs/nursery) over ~100 check structs. | `crates/standards/src/framework.rs:34-40` | Burn down per-predicate (T2 in `quality-debt.md`); don't expand the allow list. |

### A4. Consistency with `docs/standards/`

| # | Severity | Finding | Location | Fix |
|---|----------|---------|----------|-----|
| 4.1 | Med | Handlers `match ctx.format` to emit a stderr side channel — `handler-shape.md` forbids this. | `src/runtime/commands/tool/fetch.rs:35-37`, `tool/gc.rs:34` | Fold warnings into the text renderer or a unified envelope field. |
| 4.2 | Med | DTO uses `String` for a closed-domain field instead of the enum. | `src/runtime/commands/source/preview.rs:31-36,77-78` (`operation: String`) | `operation: SourceOperation` with `#[serde(rename_all = "kebab-case")]`. |
| 4.3 | Low | Doctor/refresh report DTOs use `String` paths. | `crates/workflow/src/plugins.rs:493-519` | `PathBuf` per DTO allowlist. |
| 4.4 | Low | Handler `expect` encodes prep invariants (panics if prep regresses). | `source/extract.rs:267-274`, `preview.rs:88-90` | `let Some(dir) = … else { return Err(Error::Diag{…}) }` for fail-closed CLI behaviour. |

### A5. Panics / `unwrap` / `expect`

Non-test counts are modest and mostly legitimate (regex/schema `LazyLock`, corrupt-binary messages): ~75 in `crates/workflow/src`, ~48 in `crates/standards/src` (largely `Regex::new(...).expect`). No action required broadly; the one worth hardening is `lint/eval/unique.rs:64` (`expect("len >= 2")` → `debug_assert` + `if let`).

---

## Part B — `specify-cli` tests (unit & integration)

The suite is **mature and integration-first by design** (~335 root CLI integration tests via `assert_cmd`; ~900+ unit tests inside crates). Plan orchestration, journal events, merge engine, adapter resolution, and schema parity are well covered. Gaps cluster in state-machine units, mutation engines, the merge I/O layer, binary handlers, and doc/harness drift.

### B1. High-value coverage gaps

| # | Severity | Gap | Location | Fix |
|---|----------|-----|----------|-----|
| F1 | **High** | Slice lifecycle FSM: only 2 of ~15 edges tested (`Refining→Refined`, `Refining→Built`). | `crates/workflow/src/slice/lifecycle/tests.rs` (machine at `lifecycle.rs:37-51`) | Table-driven test over all legal/illegal `(from,to)` pairs; assert `code == "lifecycle"` on rejects. **~30 lines, biggest single win.** |
| F2 | **High** | `authority_override` mutation engine has **no** unit tests (set/clear ordering, per-kind dedup, orphan rejection only via CLI). | `crates/workflow/src/change/plan/core/authority_override.rs` | Add `authority_override/tests.rs` covering `mutate_authority_overrides` + journal-event ordering. |
| F3 | **High** | Merge slice I/O layer untested (engine tested, orchestration not). | `crates/workflow/src/merge/slice/{read,parse,write}.rs` | Unit-test read→parse→write reusing `tests/fixtures/parity/case-*` deltas. |
| F4 | **High** | `slice/actions/*` (8 files) have tests only for `prune`. CLI tests hand-edit metadata instead of exercising the action layer. | `crates/workflow/src/slice/actions/` | Unit-test `transition`/`archive` side-effects; let integration call the CLI only. |
| F5 | **High** (process) | `tests/cross_repo.rs` (RM-05 acceptance harness) is referenced in `testing.md:13,32,37` but **does not exist** (verified). | `docs/standards/testing.md` | Add the harness or repoint docs at `fan_in_fan_out.rs` / `plan_orchestrate/propose.rs`. |

### B2. Test-quality smells

- **Hand-editing `.metadata.yaml`** violates `testing.md:45`: `tests/slice_merge.rs:78-79,409` (`replace("status: built", …)`), `tests/slice/metadata.rs:18`. Use `specrun slice transition` / `common::stamp_slice_outcome`. (F7)
- **Timing flakiness:** `sleep(10ms)` for mtime ordering in conflict-check tests, `tests/slice_merge.rs:149,219`. Set explicit file times instead. (F8)
- **Stale regeneration command:** `tests/rules_export.rs:32` says `cargo test --test codex_export`; the binary is `rules_export`. Comments also mix `cargo test` vs the mandated `cargo nextest`. (F12)
- **Trivial/framework tests:** `slice/outcome/tests.rs:4-8` asserts strum `Display` only — low value.
- **Redundant golden + structural asserts:** several `e2e`/`plan_orchestrate` tests assert summary fields *and* full JSON goldens; drop the golden where structure already locks behaviour. Golden usage is otherwise disciplined, not overused.
- **Duplicated harness helper:** `copy_dir` in `tests/common/mod.rs:187` vs `crates/workflow/tests/goldens.rs:33-44`. (F11)

### B3. Thinly-tested but logic-heavy modules

`registry/workspace/*` (~10 files, 0 unit tests — only integration), `registry/{branch,forge,catalog,validate}.rs` (0 unit), `merge/artifact_class.rs` (0), `src/runtime/commands/*` (~30+ handlers, ~16 tests total in 5 files). Of 20 `framework/check/*` modules only 4 carry inline `mod tests`. Exit code 4 (`EXIT_MIGRATION_REQUIRED`) has **no** integration assertion (acknowledged in `tests/migrate.rs:182-185`).

**Quick wins (highest ROI):** lifecycle table tests (F1) · fix `rules_export` doc (F12) · replace 3 metadata hand-edits (F7) · 3–5 `authority_override` unit tests (F2) · one `merge/slice/parse` unit test from existing fixtures (F3).

---

## Part C — `specify` (skills, docs, adapters)

House style is mostly sound: **10/10** skills are under the 200/45/512 caps with no RFC citations in bodies, **8/8** parent adapter briefs are within limits, the agent-teams symlinks resolve, and the `.cursor/schemas/` mirrors match the CLI. The problems are **drift** between the docs and the live `specrun` surface.

### C1. High severity (operator/agent-facing correctness)

| # | Finding | Location | Fix |
|---|---------|----------|-----|
| 1 | `cli-contract.md` documents retired/non-existent verbs: `specrun slice outcome|journal`, `specify adapter {resolve,check,pipeline}`, `specrun plan {doctor,lock}`, `specrun workspace status`. (Verified against `specify-cli` command tree.) | `docs/standards/cli-contract.md:26-34,43,47-48` | Rewrite the verb tree to match `specrun --help` / `docs/reference/cli/*.md`; move retirements to `decision-log.md`. |
| 2 | `specify` vs `specrun` binary naming drift across ~25+ files (docs tell agents to shell out to "the `specify` CLI"; shipped binary is `specrun`). | `docs/standards/cli-contract.md:3-9`, `docs/explanation/layered-stack.md`, `CONTRIBUTING.md:20`, `plugins/spec/skills/{build,drop}/SKILL.md`, acceptance tests | One rule: product "Specify", runtime `specrun`, authoring `specdev`. Sweep docs/skills/test preambles. |
| 3 | Always-on Cursor rule cites non-existent adapter brief paths (`sources/<name>/briefs/`, not `adapters/sources/<name>/briefs/`). | `.cursor/rules/project.mdc:31` | Fix paths; link to `docs/reference/directory-layout.md` instead of duplicating. |
| 4 | `agent-teams.md` canonical path documented wrong: docs say it targets `docs/reference/review-team-protocol.md`, but symlinks resolve to `adapters/shared/references/runtime/review-team-protocol.md`. CORE-008's path-equality predicate may disagree with layout. | `AGENTS.md:123`, `adapters/shared/rules/core/CORE-008-*.md:14-16`, `docs/contributing/checks.md` | Single canonical file + one documented symlink target; align CORE-008 + AGENTS. |
| 5 | mdBook TOC mislabels the adapter CLI page "specify adapter" (page documents `specrun source/target resolve`). | `docs/SUMMARY.md:69` | Rename TOC entry. |
| 6 | `acceptance.md` says `make ci` runs only `make lint`; Makefile `ci` = `lint` + `check-schemas`. | `docs/contributing/acceptance.md:19` vs `Makefile:19-20` | One-line correction. |

### C2. Medium severity (drift risk / single-source-of-truth)

- **`spec-runtime` duplication:** synthesis/guardrails/plan-lock/provenance content is materialised into all 8 adapters (~120 files) by `scripts/sync-adapter-spec-runtime.sh`. High drift risk. Document "edit canonical only" in `adapters/shared/references/runtime/README.md` and add a CI diff gate that fails if sync wasn't run. (§7)
- **Triplicated workflow/CLI-split narrative** across `AGENTS.md`, `project.mdc`, `skill-authoring.md` — any CLI change needs 3 edits. Reduce `project.mdc` to pointers. (§8)
- **`schemas/` referenced as a top-level dir** in `project.mdc:73` and `layered-stack.md:21`, but this repo only has `.cursor/schemas/` mirrors (authoritative schemas live in the CLI). (§9)
- **`directory-layout.md:38-51` omits `model.yaml`** from the slice tree though AGENTS/refine treat it as the structured synthesis artifact. (§10)
- **`cli-contract.md` vs `plan-lock.md` contradiction** on whether `specrun plan lock *` exists (it doesn't — shell `flock` only); `plan-lock.md:5` still says "v1". (§11)
- **Replay hooks reference `specrun slice journal append`** (no such verb; use `specrun journal emit`). `adapters/shared/target-hooks/replay/{hook-contract,journal-payload}.md`. (§12)
- **`phase-outcome-contract` is doc-mandated but absent from all phase skills** and has no predicate — pure doc rule that will drift. Either add the one-line section or demote it. (§14)
- **`rfcs/future/*` and `decision-log.md:40`** use pre-2.0 verb names (`specify slice merge`, `specify adapter`). Add a "verbs frozen at draft time" banner or sweep. (§15, §16)
- **`adapter.yaml` schema-URL inconsistency:** `github.com/.../raw/main/...` vs `raw.githubusercontent.com/...` across the 8 manifests. Pick one. (§17)

### C3. Low severity

- `client/sow-writer/SKILL.md:3` uses the discouraged "Use when an operator wants to X" pattern (`skill-authoring.md:12`). (§13)
- Inconsistent relative-link style in `plugins/spec/skills/execute/SKILL.md:15` vs `24-28`. (§19)
- `CONTRIBUTING.md:19` lists TypeScript for the `specify` repo (Markdown/YAML only). (§20)
- All 10 skills pass caps but with headroom only; the un-linted `phase-outcome` and `CRITICAL_PATH_MIN_LINES` rules are fragile.
- Scripts are solid; note `use-local-plugins.sh` does a destructive `rm -rf ~/.cursor/plugins/cache/augentic` (intentional) and `check-schema-mirror.sh` runs only in `make ci`, not bare `make lint`.

---

## Suggested sequencing

**Phase 1 — correctness & safety (small, high-value):**
- CLI 2.1 atomic merge writes; 2.2 decision-record writes.
- Docs C1 (#1 cli-contract, #2 specrun naming, #3 project.mdc paths) — agents currently read stale instructions.
- Tests F1 lifecycle matrix, F7 metadata hand-edits, F12 doc fix.

**Phase 2 — performance (one shared pattern):**
- CLI 1.1–1.3 schema validator caching, then 1.4 evidence single-pass and 1.6 journal tail-read.

**Phase 3 — maintainability & test depth:**
- CLI 3.1 split `slice/validate.rs`, 3.2 unify ID grammar, 3.4 archaeology strip.
- Tests F2/F3/F4 mutation + merge-I/O + actions units; F5 cross_repo harness decision.

**Phase 4 — docs single-source-of-truth:**
- Docs C2 spec-runtime duplication gate, triplicated narratives, schema/binary wording, `model.yaml` in layout.

---

*Notes:* No edits were made during this review. Findings were produced by static analysis; the highest-impact CLI claims (schema recompilation, non-atomic merge writes, missing `cross_repo.rs`, retired verbs in `cli-contract.md`) were spot-verified against current source. `cargo make ci` / `make lint` were not run — run them locally to confirm no link/deployable checks regress when applying the doc fixes (and note that a tracked `REVIEW.md` at the `specify` root was previously removed to satisfy `specdev lint` CI).*

---

## Post-mortem

One line per applied finding: actual ΔLOC vs predicted, did the "done when" assertion flip cleanly, did anything regress.

- **2.1** (atomic merge writes): actual **+19/−12** (`merge/slice/write.rs`) vs predicted ~handful — ran net-positive because helper-reuse added a `use` import + expanded docstring (matches "helper refactors net less negative" prior). Chose per-file `bytes_write` over staging-dir+rename after reasoning that `commit()` already flips metadata only after all writes return, so the real defect was torn (truncate-then-write) files, not cross-file atomicity. Done-when flipped cleanly (writes crash-safe + docstring accurate). No regress: clippy/fmt clean, workflow 572/572, slice_merge 16/16.
- **2.2** (atomic decision-record writes): actual **+13/−11** (`decisions.rs`) vs predicted ~+12/−9 — close; extra churn was the `match` arm remapping `bytes_write`'s `Error::Io` back to `Error::Filesystem{op:"write"}` to preserve the `filesystem-write` wire contract. Dropped redundant `create_dir_all` (bytes_write creates parents). Done-when flipped cleanly (promotion crash-safe + safely re-runnable). No regress: clippy/fmt clean, workflow 572/572.
- **C1#1** (cli-contract verb tree): actual **+10/−9** (`docs/standards/cli-contract.md`) vs predicted ~+10–13/−7–9 — close. Verified against CLI clap enums (`commands/**/cli.rs`) + both `AGENTS.md` inventories. Removed `slice {list,status,archive,outcome,journal}`, `plan {doctor,status,lock}`, `registry show`, `workspace status`, `specify adapter`; added real `slice {synthesize,build,model show,provenance}`, `source/target` verbs, corrected `tool {run,fetch,gc,schema}`. Done-when flipped cleanly. `make lint` exits 2 only on pre-existing `REVIEW.md:128` (CORE-016) + 8 adapter CORE-051 — no new findings.
- **C1#2** (specify→specrun naming sweep): actual **+61/−61** (balanced, ~64 refs across 40 files) vs predicted balanced 1-token swaps — matched. Confirmed defect at source (`specify-cli/Cargo.toml` ships `specrun`/`specdev`, no `specify` bin). Left repo/package names, `specify` tool namespace, env vars (`SPECIFY_BIN`), 0.1.0 history, and `rfcs/future/*` + `decision-log.md:40` pre-2.0 prose untouched. Done-when flipped cleanly (0 residual runtime-binary `specify` invocations). No new `make lint` findings.
- **C1#3** (project.mdc adapter brief paths): actual **+1/−1** vs predicted single-line fix — matched exactly. Corrected `sources/<name>/briefs/`→`adapters/sources/...` and `targets/...` (verified 10+25 brief files exist on disk; bare forms don't). No directory-layout.md link (already linked from Repository Layout section). Done-when flipped cleanly. No new `make lint` findings.
- **F1** (lifecycle FSM matrix): actual **+43/−0** vs predicted ~30 — ran higher as expected once the `LEGAL_EDGES` table + match arms included. One table test covers all 25 `(from,to)` pairs (6 legal, 19 illegal incl. self-transitions); states derived from `LifecycleStatus::value_variants()`. Verified (not assumed) reject discriminant: `Error::Diag{code:"lifecycle"}` with both endpoints in detail. Done-when flipped cleanly. workflow 572→573, clippy/fmt/rust_quality green.
- **F7** (metadata hand-edits): actual **+41/−30** (net +11) vs predicted near-neutral/slightly positive — matched. Added `stage_refined_slice` helper driving `slice create`→copy specs→`slice transition refined` to replace two `replace("status: built"…)` surgeries; `metadata.rs:18` now uses `slice create` (omits `outcome` via `skip_serializing_if`). Verified `commit` gates on status before reading specs so the `failed`-path test hits the same gate. Done-when flipped cleanly (residual `replace("status` in tests = 0). 66+5 tests pass, rust_quality green.
- **F12** (rules_export regen doc): actual **+2/−2** vs predicted 1-spot — found 2 spots (module doc + panic message), both stale `codex_export`/`cargo test`. Mirrored canonical phrasing from `lint_ignore_directive_pass.rs`; now `REGENERATE_GOLDENS=1 cargo nextest run --test rules_export`. Done-when flipped cleanly (0 residual `codex_export`). clippy/fmt clean, 10/10 pass.
- **1.1** (schema validator cache): actual **+141/−23** (4 files) vs predicted net-positive — matched (win is CPU not LOC). Added `validate_value_cached` keyed on `&'static str` pointer identity (`as_ptr() as usize`) via `LazyLock<RwLock<HashMap<usize, Arc<Validator>>>>` with double-checked compile-under-write-lock. Verified `Validator: Send+Sync`. `$ref` schemas (synthesis/build-report) kept on their dedicated statics. Routed evidence/lead/plan/components/proposal/build-request + adapter `run_schema` through it. Done-when flipped cleanly (new test asserts `Arc::ptr_eq` across calls + cached==uncached). No regress: 623/623, clippy/fmt clean.
- **1.2** (lint kind:schema cache): actual **+115/−19** (net +96) vs predicted ~+50 — overage from API-preserving `evaluate_with_cache` wrapper + doc comments. Correctly chose a run-scoped `SchemaCache` (`&mut`-threaded, no lock) over reusing 1.1's `validate_value_cached` (evaluator needs `Validator::iter_errors` for per-error diagnostics; project `$ref`s are runtime files → run scope avoids cross-run staleness). Caches validators + resolved paths. Done-when flipped cleanly (compiled once per run; goldens byte-identical). No regress: standards 277/277, lint 8/8.
- **1.3** (diagnostic schema cache): actual **+18/−4** (net +14, 8 lines are doc comment) vs predicted small net-positive — matched. Local `static DIAGNOSTIC_VALIDATOR: LazyLock<Validator>` (not `validate_value_cached` — needs raw `iter_errors` for its `; `-joined error message). 1 validator cached (report schema validated in workflow layer, untouched). Behaviour byte-identical except corrupt-schema now panics on first use (accepted idiom). Done-when flipped cleanly. No regress: diagnostics 53/53.
- **1.4** (evidence single-pass): actual **~+74/−44** (net +30, across `slice/validate.rs` + `schema.rs`) vs predicted near-neutral/nudge-positive — matched. Found it was actually 3 reads (schema pass + catalog-drift + model-drift `EvidenceFacts::read`); collapsed to one. `validate_evidence_dir` now returns `Vec<EvidenceDoc>` threaded into both drift gates; `EvidenceFacts::read`→`from_docs` (infallible, no I/O). Preserved per-file attribution + finding order + short-circuit. Done-when flipped cleanly (diagnostics byte-identical). No regress: workflow 573, slice suites 65, schema 50, goldens 2.
- **1.6** (journal tail-read): actual **+276/−17** (bulk is edge-case tests) vs predicted net-positive (tests dominate) — matched. Added backward chunked reader `for_each_line_rev` (8KB windows, carry/partial-line handling, UTF-8-safe, trailing-newline `str::lines` parity) + match-bounded `read_recent` (filter-then-tail; stops at Nth match). Preserved exact malformed/blank-line leniency + empty/short/absent behaviour; N=`RECENT_TAIL=10` unchanged; `journal::read()` kept. 6 new edge-case tests. Done-when flipped cleanly (no whole-file read; early-stop test proves head untouched). No regress: workflow 578.
- **1.5** (LeadCatalog no-alloc): actual **+27/−15** (`catalog.rs`+`kernel.rs`) vs predicted near-neutral micro-opt — matched (under newtype ceiling). Re-keyed `BTreeSet<(String,String)>`→`BTreeMap<String,BTreeSet<String>>`; `contains` borrows `&str` via `String:Borrow<str>` (zero per-lookup alloc). Rejected `hashbrown::Equivalent` to preserve deterministic sorted order `plan-reconcile-partition` depends on. Done-when flipped cleanly (semantics byte-identical). No regress: workflow 578.
- **1.7** (lint index clone): actual **+44/−28** (`lint/index.rs`) vs predicted minimal — slightly higher. Chose consume-and-move (`par_iter`→`into_par_iter`, move `relative`/`language` into the `File` DTO) over `Arc<str>` — avoids serde `rc` feature + `WorkspaceModel` schema/DTO ripple; `discovered` is unused after. Done-when flipped cleanly (no per-file deep clone; golden byte-identical). No regress: standards 277/277, lint 8/8.
- **3.1** (split slice/validate.rs): actual **+77 net** total / orchestrator **−885** (1078→193 lines) vs predicted roughly-neutral+small-positive — matched. Split into `validate/{pre_adapter(235),model_drift(394),decisions(120),catalog(88),spec_location(35),tests(90)}` + 193-line orchestrator; folded provenance scan into `pre_adapter`/`model_drift` (no standalone gate). `pub(super)` sufficed everywhere; 1.4 `EvidenceDoc` single-pass threading preserved verbatim. Done-when flipped cleanly (largest module 394, under 400 cap; pure move). No regress: workflow 578, slice suites 65. NOTE: surfaced a pre-existing `rust_quality` long-test-name failure from 1.6's `journal/tests.rs` — fixed by renaming 3 tests (`for_each_line_rev_missing_file`, `for_each_line_rev_early_stop`, `read_recent_last_n_matching`) to ≤40 chars; rust_quality now green.
- **3.2** (unify REQ/TASK ID grammar): actual **+83/−13** (net +70, mostly doc + 2 boundary tests) vs predicted modest — slightly above. Added `is_req_id`/`is_task_id` + `TASK_ID_PATTERN` in `specify-model` (OnceLock regex, full-span `find` to match byte-predicate newline-rejection exactly). Rewired workflow `model_drift` (deleted `is_three_digit_id`) + validate `primitives` (scanner body sourced from `REQ_ID_PATTERN`). Kept validate's scanner UNanchored (anchoring would change outcomes). Accepted set unchanged (boundary-verified). Done-when flipped cleanly. No regress: model+validate+workflow 739.
- **3.4** (archaeology strip): actual **~+11/−33** total (framework.rs module doc −22 net via interrupted-then-resumed attempt; validate submodules +7/−7 in-place RFC-ref trims) vs predicted net-negative — matched. `RustSourceQuality` markers (`RFC-`,`Phase `,`formerly `,…) cleared: validate module 7→0; journal.rs already clean (its long per-variant docs are current-behaviour). Trimmed RFC cross-refs in-place (kept load-bearing intent); no DECISIONS.md relocation needed. quality-debt.md untouched (these files weren't tracked there). Done-when flipped cleanly (RustSourceQuality 7→0). No regress: workflow+standards 855, rust_quality green.
- **3.3** (decompose standards modules): actual net roughly-neutral (orchestrators shrank: skill_body 667→99, scenarios 556→30, model 475→191; `eval.rs` skipped — already 351 post prior-finding split). Threshold is a ≤400-line guideline (no hard gate). Extracted cohesive submodules (`skill_body/{critical_path,envelope,section,variables}`, `scenarios/{frontmatter,trace,discovery}`, `model/{facts,tests}`); `pub(super)` for checks, `pub use` preserved public surfaces. Pure move. Done-when flipped cleanly. Independently re-verified: standards 277/277, clippy clean. (Subagent's "concurrent writer" note was a self-read misinterpretation; tree consistent.)
- **A5** (unique.rs expect→debug_assert): actual **+4/−1** vs predicted tiny near-neutral — matched. Replaced `expect("len >= 2")` with `debug_assert!` + `let Some(first) = sorted.first() else { continue }`. Invariant already guaranteed by preceding `if paths.len() < 2 { continue }`, so behaviour unchanged for real inputs; release builds skip gracefully instead of panicking. Used `let-else` to satisfy `clippy::manual_let_else`. Done-when flipped cleanly. No regress: standards 277/277.
- **F2** (authority_override unit tests): actual **+253/−0** vs predicted net-positive additive — matched. Found engine surface: `mutate_authority_overrides` returns events directly (engine emits, handler only parses flags); dedup via `BTreeMap<(slice,kind),value>`; orphan gate `reject_orphan_overrides`→`Error::Validation{code:"slice-authority-override-orphan-source"}` (verified). Added 8 tests: set/clear resolution+suppression, dedup, distinct-kind coexist, orphan reject/accept, unknown-slice refusal, event sort order, clear_all per-kind. Done-when flipped cleanly. No regress: workflow +9 tests, rust_quality green.
- **F3** (merge slice I/O tests): actual **+229/−0** vs predicted net-positive — matched. Added 11 tests as `<module>/tests.rs`: read (discovery + merge-plan + opaque classification), parse (time conversion), write (atomic baseline write + no-temp-leak + opaque copy + summary), plus a read→parse→write round-trip reusing `tests/fixtures/parity/case-04-modified` in a tempdir slice tree. Pure test additions, exercises post-2.1 atomic write. Done-when flipped cleanly. No regress: workflow 587→598, rust_quality green.
- **F4** (slice/actions tests): actual **+246/−0** vs predicted net-positive — matched. 8 action files; only `prune` had tests. Added 10 tests for `transition` (legal stamps+persists+emits `slice.transition.refined`; illegal→`Error::Diag{code:"lifecycle"}` leaves metadata+journal untouched; idempotent stamp), `archive` (moves dir to `archive/<date>-<name>`, no-basename→`slice-dir-no-basename`), `create` (seed+reload, `invalid-name`, `slice-already-exists`, Continue/Restart). Real fs/metadata/journal assertions. Done-when flipped cleanly. No regress: workflow 598→608.
- **F5** (cross_repo harness decision): actual **+3/−3** (docs only) — chose repoint over building a harness (RM-05 genuinely covered by green `tests/fan_in_fan_out.rs`, confirmed via roadmap/acceptance/DECISIONS). Repointed all 3 `testing.md` refs (binary list→`fan_in_fan_out`; helpers→`tests/common/mod.rs`+`workspace.rs`; harness pointer→`fan_in_fan_out.rs`+`plan_orchestrate/`). Done-when flipped cleanly (no dangling ref; targets green 93/93). Follow-up flagged: stale `cross_repo` comment at `crates/workflow/tests/adapter.rs:248` (parent-repo .ts harness, out of scope).
- **2.3** (journal dropped-events): actual **+126/−9** vs predicted modest net-positive — slightly higher (helper + sidecar writer + 2 tests). Chose observable+durable: `record_dropped` helper emits structured `warning:` (scope, path, io error, recovery location) via repo's stderr idiom AND appends to `.specify/journal.dropped` sidecar; wired into `emit_best_effort` + `emit_lint_completed`; docstrings document the swallow as intentional. No strict mode (no CI hook exists); verb still exits 0. 2 tests added. Done-when flipped cleanly (no longer silent/undocumented). No regress: workflow 610/610.
- **2.4** (init wasm-pkg.toml atomic): actual **+2/−4** (net −2) vs predicted tiny near-neutral — matched. Kept create-only `path.exists()` guard (don't clobber operator edits), replaced `create_dir_all`+`fs::write` with single `bytes_write` (creates parents), swapped unused `use std::fs` for the helper import. Clobber semantics preserved. Done-when flipped cleanly. No regress: workflow 610/610 incl. `reinit_preserves_wasm_pkg_config`.
- **4.1** (handler ctx.format side-channel): actual **+27/−20** (net +7) vs predicted slightly-negative/neutral — shared `write_warnings` helper cost a few lines. Removed `match ctx.format` stderr branch from `tool/{fetch,gc}.rs`; warnings now render to stdout via the text renderer (matching `registry/remove` idiom); JSON already carried `warnings` so no golden changed. Left `tool run`'s stderr shim (transparent WASI, out of scope). Done-when flipped cleanly (residual `ctx.format` in tool/ = 0). No regress: tool 12/12.
- **4.2** (source preview operation enum): actual **+3/−2** (net +1) vs predicted near-neutral — matched. `BriefEntry.operation` `String`→`SourceOperation` (enum already derives `Serialize`+`#[serde(rename_all="kebab-case")]`+strum `Display`, both kebab-case); construction `op.to_string()`→`*op`. Wire byte-identical (verified by existing `briefs[].operation` JSON assertions). No goldens moved. Done-when flipped cleanly. No regress: 36/36.
- **4.3** (plugins.rs PathBuf DTOs): actual **+17/−17** (net 0) vs predicted near-neutral — matched. `DoctorReport`/`RefreshOutcome` `marketplace`/`cache_root` `String`→`PathBuf`, `deleted_paths` `Vec<String>`→`Vec<PathBuf>`; construction drops `.display().to_string()`; render uses `.display()`; kept journal `EventKind::PluginsRefreshed` String boundary (closed taxonomy). Verified PathBuf serde wire-identical (independently confirmed fields are PathBuf in tree). Done-when flipped cleanly. No regress: workflow 458 lib + 4 plugins integration.
- **4.4** (handler expect→fail-closed): actual **+25/−12** vs predicted near-neutral — modestly additive (let-else + multi-line detail strings wordier than one-line expects). `extract.rs`: `evidence_dir(...)` now returns `Result`, fails closed with `Error::Diag{code:"source-extract-prep-missing"}` (both call sites `?`). `preview.rs`: `let Some(evidence_dir) = prepared.evidence_dir else { Err(Error::Diag{code:"source-preview-dir-missing"}) }` (didn't disturb 4.2). Both Diag→exit 1. No debug_assert (matches house fail-closed idiom). Done-when flipped cleanly (`expect` in source/ = 0). No regress: 10/10.
- **F8** (mtime timing flakiness): actual **+29/−11** vs predicted near-neutral+maybe-dep — slightly higher, NO dep added. Review locations stale (`extraction_cache.rs` doesn't exist — cache is mtime-free; `journal.rs:159` had no sleep). Real sites: `tests/slice_merge.rs` two `conflict_check` drift tests. Both compare baseline mtime vs fixed `defined_at: 2020`, so sleep was cosmetic; replaced with `stamp_mtime_after_defined_at` helper setting explicit mtime (epoch+1.7e9s) via `std::fs::FileTimes`/`File::set_times` (stable, no `filetime` dep → no deny/vet churn). Same `mtime > defined_at` path. Done-when flipped cleanly (sleep removed, deterministic). Stable 3× reruns. No regress: 36/36.
- **F11** (dedupe copy_dir helper): actual **+9/−69** (net −60) vs predicted net-negative — matched. 4 duplicates (`tool.rs::copy_dir`, `target.rs`/`source.rs`/`source_preview.rs::copy_dir_recursive`) → single `common::copy_dir`. All 4 files already had `mod common;` (no additions). Reconciled `tool.rs`'s `target`-skip guard out (verified no fixture has a `target/` dir — flagged). Dropped now-unused fs/path imports. Done-when flipped cleanly (`fn copy_dir` in tests/ = 1). No regress: 24/24.
- **C1#4** (agent-teams.md canonical path): actual **+3/−3** (balanced) — reviewer's premise was a one-hop `readlink` artifact: `agent-teams.md` symlinks (omnia/vectis) point at the shared overlay `adapters/shared/references/runtime/review-team-protocol.md` which *resolves* to the canonical real file `docs/reference/review-team-protocol.md`. Disk internally consistent (no symlink re-pointed). Corrected prose at `AGENTS.md:123` + CORE-008 Rule/Fix to describe the two-hop accurately (`checks.md` already correct). Done-when flipped cleanly. `make lint` baseline unchanged (2× CORE-016 REVIEW.md + 8× CORE-051 adapters), CORE-008 green, no new findings.
- **C1#5** (SUMMARY.md TOC rename): actual **+1/−1** — `docs/SUMMARY.md:69` label `specify adapter` (retired verb) → `specrun source and target resolve` (matches the page's H1 + sibling plain-command TOC style). Link target unchanged. Done-when flipped cleanly. `make lint` baseline unchanged.
- **C1#6** (acceptance.md make ci): actual **+1/−1** (balanced) — `Makefile:19` `ci: lint check-schemas`; corrected `acceptance.md:19` from "`make ci` runs `make lint`" to note it also runs `check-schemas` (`scripts/check-schema-mirror.sh`, verifies `.cursor/schemas/` mirrors) and that bare `make lint` skips it. Done-when flipped cleanly. `make lint` baseline unchanged.
- **C2** (drift/SSOT cluster, 8 sub-items): actual **+27/−12** across 14 files (excl. generated `spec-runtime/` copies), all ground-truthed. §7 augmented SSOT README "edit canonical only" + flagged CI diff-gate as follow-up (materialised trees already drift in-tree → gate needs clean baseline first). §8 real dup was `skill-authoring.md` (not project.mdc, already pointers) → collapsed to pointer. §9 `schemas/`→`.cursor/schemas/` in project.mdc+layered-stack. §10 added `model.yaml` to directory-layout slice tree. §11 `plan lock` verb claim already correct; removed stale "v1" framing. §12 `specrun slice journal append`→`specrun journal emit` in 2 replay hook docs. §14 demoted phase-outcome-contract mandate to optional (zero skill edits). §15/16 "verbs frozen at draft time" banners on rfcs/future/{19,21,22,24} + decision-log:40 inline fix. §17 normalized omnia adapter.yaml schema URL to majority `github.com/.../raw/main` form (7/8). Done-when flipped cleanly per item. `make lint` baseline unchanged (0 new), CORE-051 held at 8. Follow-up: stale `plan lock *` doc-comment at specify-cli `plan/cli.rs:2`.
- **C3** (low-severity cluster, 5 sub-items): actual **+2** net — 3 edits, 2 conscious no-actions. §13 rewrote sow-writer description to drop "operator wants to" anti-pattern (within caps). §19 links already consistent in HEAD → no-op. §20 CONTRIBUTING language list TypeScript→shell (verified no `.ts` in plugin repo). 4th no-action: `CRITICAL_PATH_MIN_LINES` is an implemented specify-cli predicate (out of scope), phase-outcome already handled in C2 §14. 5th added one-line safety comment at scoped `rm -rf` in `use-local-plugins.sh`. Done-when per item flipped cleanly. `make lint` baseline unchanged, skill predicates green.
