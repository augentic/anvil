# Improve & Optimise Review — `specify` + `specify-cli`

Scope: a comprehensive, no-new-features review of both repos to decide **where to spend effort**.
Mode: *improve & optimise*, with a deliberate emphasis on **test quality (unit + integration)** per request.

This file lives in `specify-cli/` but covers **both** repos. Findings are anchored with `file:line`
where possible and ranked by impact. A "Verification notes" section at the end records claims that
were checked against the live tree (and three that turned out to be wrong).

Snapshot of the tree at review time:

- `specify-cli`: ~84k LOC of Rust across 10 workspace crates + the `specify` binary; **1,821** `#[test]`/`#[tokio::test]` fns; `cargo make ci` is the gate.
- `specify`: markdown skills/refs/rules + shell scripts; `make lint` (→ `specify lint framework`) is the gate.

---

## 0. Executive summary — where to focus

In priority order, the highest leverage work is:

1. **Centralise time injection in `specify-cli`** (a standards rule that is being violated in library code). It is the single change that most unblocks deterministic testing — see A1 and B-determinism. *Highest leverage: fixes a standards violation AND removes a whole class of test flakiness.*
2. **Move domain logic out of binary handlers** (`src/runtime/commands/plan/lifecycle.rs`, `slice/merge.rs`) into `specify-workflow`. This both follows `handler-shape.md` and lets that logic be unit-tested without a `Ctx`.
3. **Close the integration-test gaps on the actively-churning surfaces**: `--platforms`, `plan propose --reconcile-platforms`, exit code 4, and init-time `AGENTS.md`/`context.lock`. These are the surfaces with recent git churn and the weakest end-to-end coverage.
4. **Split the god modules** (`journal.rs` 927, `adapter/core.rs` 762) and relocate inline `#[cfg(test)]` blocks into sibling test modules.
5. **In `specify`: fix `specrun` → `specify` in agent-facing vectis surfaces** (`adapters/targets/vectis/briefs/build.md`, `.../rules/platform-shell-presence.md`). Agents currently read a non-existent binary name.
6. **In `specify`: rewrite `docs/reference/targets/{vectis,omnia,contracts}.md`** which still describe a retired 1.x "Define phase".

Everything else is incremental polish.

---

## Part A — `specify-cli`: code quality & architecture

The crate graph is healthy: leaf→root layering is real, the standards/workflow split is a type-system
invariant, `specify-schema` centralises embedded schemas, and the plan domain is already nicely split
under `change/plan/core/`. The debt is concentrated in a few god modules, the binary handlers, and a
documented-but-violated time-injection rule.

### A1. Time injection is not centralised — a standards violation **(do first)**
`architecture.md` requires `now` to be sourced from `src/runtime/commands/*.rs` so tests can pin it.
Library code reads the clock directly in several places:

- `crates/workflow/src/journal.rs:833,912` (`emit_best_effort`, `emit_lint_completed`)
- `crates/workflow/src/slice/validate.rs:129` (`append_synthesis_journal`)
- `src/runtime/commands/slice/merge.rs:55,73` (two separate `Timestamp::now()` reads — can disagree across midnight; A-perf below)

**Why it matters:** every one of these blocks deterministic golden tests and is the root cause of the
date-dependent test fragility in Part B. **Fix:** thread a single `now: Timestamp` from the handler (or
add `Ctx::now`) and forbid `Timestamp::now()` in `specify-workflow` via a small `rust_quality` predicate.

### A2. Domain logic living in binary handlers (violates `handler-shape.md`)
Handlers should bracket journal + `ctx.write`; deterministic work belongs in workspace crates.

- `src/runtime/commands/plan/lifecycle.rs:53–105` `topology_cache_staleness` — registry projection + YAML loads inside the handler. Move next to `registry/topology.rs`.
- `src/runtime/commands/plan/lifecycle.rs:111–170` `next` — assembles `NextBody` from `with_state`/`validate`/`detect`/`resolve_topology`/`resolve_target`. Extract `plan_next_body(...) -> NextBody` into `specify-workflow` (mirrors `plan_finding`).
- `src/runtime/commands/slice/merge.rs:50–84,357–385` — git side effects (`auto_commit`, `eprintln!` warnings) in the handler. Belongs in `specify-workflow::merge`, mirroring the two-phase build split.

**Why it matters:** these are the exact modules with the thinnest unit coverage (Part B). Moving the logic in-crate is the cheapest way to make it testable.

### A3. God modules to split
- `crates/workflow/src/journal.rs` (**927**): closed taxonomy + wire DTOs + `append_batch` + best-effort emit + dropped sidecar + lint completion. Split into `event.rs` / `append.rs` / `emit.rs`.
- `crates/workflow/src/adapter/core.rs` (**762**): manifest types + dual `resolve` (417/479) + axis-collision (607–652) + schema validation. Extract `resolve.rs` + `validate_manifest.rs`.
- `crates/standards/src/framework/check/skill_frontmatter.rs` (**452**): five `Check` impls that each re-walk skills via `load_skill_entries`. Split per-check and cache the walk once per `Context`.

### A4. Inline `#[cfg(test)]` in production modules inflates file sizes
`crates/workflow/src/plugins.rs:523+`, `upgrade.rs:489+`, `crates/tool/src/validate.rs:400+` carry large
inline test modules (≈220 lines in `plugins.rs` alone). `coding-standards.md` wants these in sibling
`<module>/tests.rs`. This also distorts the "largest files" signal below.

### A5. Duplication to consolidate
- **Language-extension tables duplicated**: `crates/standards/src/lint/index/files.rs:235–252` (HashMap) vs `.../index/framework.rs:255–272` (Vec). Single `lint/index/languages.rs`.
- **Parallel JSON-schema caches** with different keying/failure modes: `crates/schema/src/validate.rs:164–188` (embedded `&'static str` keys), `crates/standards/src/framework/context.rs:110–132` (per-`PathBuf` `Mutex<HashMap>`), `crates/workflow/src/schema.rs:156–159,219–222`. Route framework lint through `specify_schema::cached_validator`.
- **Req-ID grammar duplicated**: `crates/validate/src/primitives.rs:34–40` vs `specify_model::spec` regexes — share one constant to prevent drift.

### A6. Error-handling & DTO polish (per `style.md` / `coding-standards.md`)
- `crates/schema/src/validate.rs:177,181` — `RwLock::...expect("validator cache not poisoned")` panics the CLI on a poisoned lock. Map to `Error::Diag { code: "schema-cache-poisoned", .. }`.
- `crates/error/src/error.rs:185` — `Cow::Owned(code.clone())` allocates per validation render; store `code: Cow<'static, str>` since most sites pass literals.
- `crates/workflow/src/change/plan/core/validate.rs:32–48` — `plan_finding` does `code.to_string()` on a hot path (checks × entries). Use `&'static str` / `Cow`.
- `src/runtime/commands/plan/lifecycle.rs:439–447,151–154` — `NextBody.reason: Some("drained".into())` uses magic strings; use a typed enum with `serde(rename_all = "kebab-case")`.
- Registry/workspace paths lean heavily on stringly `Error::Diag { code, detail }` (20+ sites under `registry/workspace/`); consider a budgeted `Error::Workspace { kind, detail }` to kill `code` typos.

### A7. Archaeology in `///` docs (style.md caps history)
RFC/Phase prose in module docs: `change/plan/core/propose/kernel.rs:1–6`, `slice/build/wire.rs:1–8`,
`change/plan/core/model.rs:191–200`. Move pointers to `DECISIONS.md`, keep ≤3-line "what today".
`docs/quality-debt.md:27` already lists this hotspot — `RustSourceQuality` archaeology is the only
remaining burn-down predicate and is **not CI-gated**, so it will keep regressing silently (see B13).

### A8. Performance (lower priority, but cheap)
- `plan/lifecycle.rs:72–90` — O(registry × slot) `ProjectConfig::load` + `TopologyProject::resolve` per `plan validate`; compare `topology.lock` fingerprint first.
- `slice/merge.rs:55 vs 73` — double clock read (folds into A1).
- `crates/standards/src/lint/eval.rs:28–43` — `reserved_skipped` scaffolding documented as always-empty in real runs; gate behind `cfg` or delete until a reserved kind ships.

### Largest production files (≥430 LOC, excluding inline tests where noted)
`journal.rs` 927 · `adapter/core.rs` 762 · `plugins.rs` 744 (~220 inline tests) · `plan/core/model.rs` 614 ·
`diagnostics/diagnostic.rs` 611 · `workflow/schema.rs` 588 · `model/spec/provenance.rs` 578 ·
`plan/lifecycle.rs` 545 · `migrate.rs` 540 · `tool/validate.rs` 523 · `upgrade.rs` 522 · `tool/manifest.rs` 499 ·
`tool/host.rs` 481 · `cli.rs` 464 (clap, expected) · `skill_frontmatter.rs` 452 · `lint/index.rs` 447 ·
`rules/resolve.rs` 437 · `fingerprint.rs` 436 · `slice/merge.rs` 431 · `validate/primitives.rs` 423.

---

## Part B — `specify-cli`: tests (the side-interest focus)

The suite is genuinely strong: ~35 root integration binaries driving `assert_cmd`, co-located unit tests
in crates, golden discipline via `REGENERATE_GOLDENS`, and the `rust_quality` meta-gate. Integration-first
is followed well. The gaps are concentrated on **new/churning CLI surfaces** and a handful of
maintainability/determinism issues.

### B1. Integration gaps on actively-evolving surfaces **(highest test ROI)**
These have unit coverage but **zero** integration coverage — and the git status shows active churn in exactly these files (`propose/platforms.rs`, `init/*.rs`, `change.rs`).

| Surface                                         | Unit cover                                              | Integration gap                                             |
| ----------------------------------------------- | ------------------------------------------------------- | ----------------------------------------------------------- |
| `specify init --platforms`                      | `init/regular/tests.rs:474–563`, `init/tests.rs:68–134` | none under `tests/`                                         |
| `plan propose --from --reconcile-platforms`     | `propose/tests.rs:542–776`                              | flag at `plan/cli.rs:171`, never invoked in a test          |
| init `AGENTS.md` + `.specify/context.lock`      | in-crate `specify-agents` tests                         | `init_shapes.rs:42–73` checks dirs, never the fences/lock   |
| **exit code 4** (`ProjectNeedsMigration`)       | `config/tests.rs:111–118`                               | no CLI test seeds a v1 pin → normal verb → exit 4           |
| `tool gc`                                       | —                                                       | `tests/tool.rs:140–142` only checks `--help`                |
| `init/cache.rs` (`cache_adapter`/`cache_codex`) | `init/regular/tests.rs:342` (one path)                  | no cache miss/hit/`.cache-meta.yaml` test                   |
| `lint/eval/authoring_predicate.rs` (RFC-31)     | —                                                       | **no test anywhere** — the migration safety net is untested |

**Action:** add ~6 small integration tests (init platforms happy + `project-platforms-not-allowed`; `reconcile-platforms` → bootstrap-slice names + `plan.reconcile.completed` journal golden; v1-pin → exit 4 envelope; greenfield → `AGENTS.md` fences + `context.lock`; `tool gc` prune; one migrated CORE rule through `authoring-predicate`).

### B2. Oversized test files
- `crates/standards/tests/core_parity.rs` (**2431**) — 9 CORE parity modules + inline reference impls in one binary. Split into per-CORE `[[test]]` targets or extract a shared `parity_support` module (`make_rule`/`NoToolRunner` at 23–66 are copy-pasted).
- `crates/workflow/tests/workspace.rs` (**1192**), `tests/journal.rs` (671), `tests/fan_in_fan_out.rs` (681) — candidates for submodule splits like `tests/slice/`.

### B3. Brittle full-text assertions → assert structure instead
- `acceptance/suites/plan-authoring_orchestrate/mutate.rs:41–44,186–187` — `saved.contains("name: foo")` on raw YAML (whitespace/order sensitive).
- `tests/slice_build.rs:122–144,180–223` — journal checks via `raw.contains(r#""event":"slice.build.started""#)`; will break on field reordering.
- `acceptance/suites/plan-authoring_orchestrate/archive.rs:50,92` — stdout substring checks alongside good JSON goldens.

**Action:** parse JSON journal lines (the good pattern is `tests/journal.rs:37–44`) and read parsed YAML fields. Publish a shared `common::read_journal_normalized(root)` helper.

### B4. Weak "success-only" setup steps
`acceptance/suites/plan-authoring_orchestrate/next.rs:11,44,65`, `source_binding.rs:187,207,230,250`, `validate.rs:16,150` call
`.assert().success()` as setup without asserting the side effect. Extract `support::add_pending_entry(...)`
helpers that assert exit 0 **and** the resulting file state in one place.

### B5. Determinism / flakiness
- **Date-dependent**: `acceptance/suites/plan-authoring_orchestrate/archive.rs:8–9` `today_yyyymmdd()` feeds path assertions at 55/100/138/183/260/301/319 — UTC-midnight fragile. The filename test at 152–173 already uses a regex (do that everywhere, or inject a fixed clock once A1 lands).
- Journal timestamp normalisation is done well in `tests/journal.rs:21–57` but **not** reused in `source_survey.rs`/`slice_build.rs` — promote it to `common`.
- CI cost: `Makefile.toml:68–71` makes `test` depend on `clean` + `vectis-wasm`, forcing a cold rebuild every run. Consider decoupling the wasm-fixture build from the default `test` task for local iteration.

### B6. Golden regeneration friction
Inconsistent regen hints: `tests/e2e.rs:12` (correct `nextest`) vs `tests/common/mod.rs:440` ("cargo test")
vs `plan_orchestrate/create.rs:64` (per-binary). Goldens are scattered across `acceptance/examples/{e2e,plan,journal,rules-export}/`
and `crates/workflow/tests/migrate/`. **Action:** one canonical panic message
(`REGENERATE_GOLDENS=1 cargo nextest run --test <binary>`) and a `tests/README.md` index mapping binary → fixture dir.

### B7. Duplicated harness boilerplate
`copy_dir` is reimplemented in `tests/common/mod.rs:187–198`, `crates/workflow/tests/runner.rs:17–28`,
`crates/standards/tests/check_links.rs:21+`. `snapshot_tree` (`tests/init.rs:268–291`) is private. Publish both
from `tests/common`.

### B8. Binary unit tests that should move to crates (integration-first policy)
`src/runtime/commands/agents.rs:63–196`, `slice/merge.rs:389–432`, `source/prep.rs:261+`, `upgrade.rs:196+`,
`lint/run_tests.rs:1–20` keep unit tests in the binary. Once A2 moves the logic into `specify-workflow`/`specify-agents`,
these become proper in-crate unit tests.

### B9. Thin unit tests worth growing
`slice/outcome/tests.rs:4–8` (Display/serde only), `slice/metadata/tests.rs:31–38` (one round-trip, no error
paths), `error.rs:227–235` (only `diag_round_trip` — no `CliTooOld`/`ProjectNeedsMigration`/`Validation`).
`slice/actions/discard.rs:24–36` has no unit test for the illegal-terminal-transition path.

### B10. Opportunities: property-based & table-driven
`proptest` is used nowhere today. Good first adoptions are pure functions that already have clustered table
tests: `detect_missing_platforms`, plan cycle detection (`plan/doctor/tests.rs:45–98`), `needs_migration`/`major`
(`config/tests.rs:104–133`), and `diagnostics/fingerprint.rs` (stability across field order). A single
table-driven `tests/cli_errors.rs` mapping representative `Error` → exit code (0/1/2/3/4) would lock down
`src/runtime/output.rs`, which currently has no direct test.

### B11. `rust_quality` meta-gate is partial
`tests/rust_quality.rs:15–28` hard-gates only `rust.test-fn-name-too-long`. `RustSourceQuality` (archaeology,
bare `#[allow]`) is burn-down-only per `docs/quality-debt.md:34–35`, so A7-style regressions never fail CI.
Decide whether to promote it to a gate now that the test-name burn-down is complete.

---

## Part C — `specify` (plugins / docs / rules)

All 10 `SKILL.md` files are within the 200/45/512 caps and broadly follow house style. The real debt is
**agent-facing CLI naming drift** and **stale 1.x docs**, not the skills themselves.

### C1. `specrun` → `specify` in agent-facing vectis surfaces **(do first)**
The canonical binary is `specify`. These agent-loaded files still emit `specrun`, so an agent following them runs a non-existent command:

- `adapters/targets/vectis/briefs/build.md:43,66` — `specrun tool run vectis -- scaffold/verify …`
- `adapters/targets/vectis/rules/platform-shell-presence.md` — `specrun tool run vectis -- verify …` (already modified in the working tree)
- `docs/reference/targets/vectis.md` — `specrun init vectis …`

Lower priority (roadmap/historical, fine to leave or tag as history): `docs/explanation/decision-log.md`,
`rfcs/roadmap.md`, `rfcs/future/rfc-{19,21,22,24}-*.md`, `rfcs/future/rfc-33b-*`.

**Note:** the ~hundreds of `specrun` hits under `docs/book/html/**` are *generated mdbook output*; fix the
sources and regenerate. **Action:** add a `make lint` / CI grep gate forbidding `specrun` outside an explicit
historical allowlist.

### C2. Target reference docs describe a retired 1.x "Define phase"
`docs/reference/targets/{vectis,omnia,contracts}.md` still document a "Define phase" where target briefs
*write* `proposal.md`/`spec.md`. The correct 2.0 model (in `docs/reference/targets/index.md`) is: core
`/spec:refine` synthesises artifacts; target ops are **`shape` | `build` | `merge`** only. Rewrite the three
pages to match `index.md`. This is the biggest single doc win for adapter authors.

### C3. Contributor docs drift
- `docs/contributing/checks.md`: §5 says SKILL `name` must equal directory (now plugin-qualified, e.g. `specify-merge`); rule-id table presents imperative `check::*` as primary (now declarative `CORE-*` + `authoring-predicate` bridge, imperative ≈ `CORE-009` only); scenario example uses `stages: [define, build, merge]` (should be `refine`).
- `docs/contributing/cli-architecture.md:95` cites `specify_change::Plan` / `specify_config::ProjectConfig` — crates are `specify-workflow` / `specify-model`.
- `docs/contributing/skills-test-coverage.md` still counts retired skills (`change-analyze`, `specify-extract`) and `stages: [define, …]`.
- `docs/explanation/decision-log.md` mixes pre-2.0 history under a "Current maintained docs" heading — add per-entry `Status: historical | current`.

### C4. Materialised `references/spec-runtime/` duplication (architecture, not active drift)
~120 generated copies under `adapters/{sources,targets}/*/references/spec-runtime/` mirror the canonical
`plugins/spec/references/`. **Verified:** the working tree currently has only **2** modified files (not a mass
drift), so this is an *architectural* maintenance concern, not an emergency. The `README.md` already flags the
missing **CI `sync && git diff --exit-code` gate** — add it so canonical edits and `scripts/sync-adapter-spec-runtime.sh`
can't desync.

### C5. RFC / link hygiene
- **RFC-31** is accepted with all phases complete but sits at repo root. `rfcs/done/` **exists and is empty** — move RFC-31 there (or add an "implemented" banner). Its links to `../DIAGNOSTICS.md` are broken from this repo (that file lives in `specify-cli`).
- `rfcs/roadmap.md` links to non-existent `rfc-36-project-identity.md` / `rfc-38-reconciliation-2.md`.
- `docs/explanation/decision-log.md` references retired `plugins/change/skills/plan/...` paths.
- `CORE-019`/`CORE-002` catch broken relative links on `make lint`, but RFC prose links slip through.

### C6. Rules quality
- ~35 `CORE-*` rules still delegate via the `authoring-predicate` bridge with boilerplate "Resolve the
  violation…" Fix lines. Burning these down to native hints (start with agent-facing `CORE-035/036`
  descriptions and `CORE-021` symlinks) speeds `make lint` and makes rule bodies actionable.
- `core/README.md`: the "chassis quirk" (`applicability.artifacts` is ineffective on the framework profile,
  authors must use `path-pattern`) is a footgun — either fix the chassis in `specify-cli` or add a prominent
  authoring checklist.
- `UNI-*` rules are mostly broad/review-only; `README.md` says "when implemented" for `specify lint` — document
  which UNI ids are review-only vs deterministically exported today. `UNI-019` (injection) overlaps `UNI-002`
  (unvalidated input) — cross-link rather than duplicate.

---

## Verification notes (claims checked against the live tree)

To keep this review trustworthy, several sub-findings were verified directly; three were **wrong** and have
been removed/corrected:

- ❌ "Orphan skill dirs `define/ explore/ status/ verify/` without `SKILL.md`" — **false**; every dir under `plugins/spec/skills/` has a `SKILL.md`.
- ❌ "Extra unregistered plugins `plugins/omnia`, `plugins/rt`, `plugins/plan`" — **false**; only `capture`, `client`, `spec` exist, and all three are in `marketplace.json`.
- ❌ "Many modified `spec-runtime` materialised copies (active drift)" — **overstated**; only 2 files are modified in the working tree. Reframed C4 as architectural duplication + missing CI gate.
- ✅ `specrun` in `adapters/targets/vectis/briefs/build.md:43,66` and `.../rules/platform-shell-presence.md` — confirmed.
- ✅ `rfcs/done/` exists but is empty — RFC-31 archival is valid.
- ✅ 1,821 test fns; no pre-existing `REVIEW.md` in either repo.

---

## Suggested execution order (cross-repo, by impact × effort)

1. **`specify-cli` A1** — centralise time (`Ctx::now` + thread `now`), forbid `Timestamp::now()` in workflow via a `rust_quality` predicate. *Unblocks B5 determinism.*
2. **`specify-cli` B1** — add the ~6 missing integration tests (`--platforms`, `--reconcile-platforms`, exit 4, `AGENTS.md`/`context.lock`, `tool gc`, `authoring_predicate`).
3. **`specify` C1** — `specrun` → `specify` in vectis brief/rule + add a CI grep gate.
4. **`specify-cli` A2** — move `topology_cache_staleness` / `plan_next_body` / merge git logic into `specify-workflow`; then B8 unit tests follow.
5. **`specify-cli` A3 + A4** — split `journal.rs` / `adapter/core.rs`; relocate inline test modules.
6. **`specify` C2** — rewrite the three target reference docs to the 2.0 `shape/build/merge` model.
7. **`specify-cli` B2 + B6 + B7** — split `core_parity.rs`, unify golden-regen messaging, publish shared harness helpers.
8. **`specify` C3–C6** — contributor-doc refresh, spec-runtime CI gate, RFC-31 archival, `authoring-predicate` burn-down.
9. **`specify-cli` A5/A6/A7/A8 + B10/B11** — duplication, error/DTO polish, archaeology gate, first `proptest`/table-driven adoptions.

---

## Post-mortem

One line per applied finding: actual ΔLOC vs predicted, did the "done when" assertion flip cleanly, did anything regress.

- **A1** (centralise time injection): actual **+105/−16** across 12 files (net +89) vs predicted small-to-moderate — ran net-positive per the helper/threading prior (added `Ctx::now()` seam, threaded explicit `now: Timestamp` params, a new `rust.workflow-clock-read` predicate + its wiring). Done-when flipped cleanly: 0 `Timestamp::now()` calls remain in `crates/workflow/src/**` (surviving rg hits are doc comments + a test fixture), a `rust_quality` predicate now gates it, and `merge.rs`'s double clock-read collapsed to one injected `now`. No regress: clippy `-Dwarnings` + `fmt` clean; `specify-workflow` 946 pass, binary `specify` 351 pass, `rust_quality` 2 pass; gate confirmed to fail when a clock read is reintroduced.
- **B1** (integration gaps on evolving surfaces): actual **+398/−3** across 5 files vs predicted "~6 small tests" (additive, net-positive as expected). Landed **5 of 6** — `init --platforms` happy + `project-platforms-not-allowed`; `propose --reconcile-platforms` greenfield `app-foundation` + incremental `bootstrap-{ios,android}` with parsed `plan.reconcile.completed` event; greenfield `AGENTS.md` fences + `context.lock`; `tool gc` prune; `authoring-predicate` via `lint framework` (`CORE-042`). **Finding 3 (exit 4) infeasible**: binary is pre-1.0 (major 0), so a v1 pin yields exit 3 (`CliTooOld`, already covered), never exit 4 — no integration path reaches it until a ≥1.0 ships. Done-when flipped cleanly for the 5 reachable surfaces. No regress — and the authoring-predicate test surfaced a **real latent bug** (RFC-31 bridge never rebased finding paths like `check::finalize` does, so migrated `CORE-*` rules emitted absolute paths failing `diagnostic.schema.json` and silently emptying the envelope); fixed minimally (+29 in `authoring_predicate.rs`). clippy/fmt clean; `specify-standards` 282/282, new test binaries all green.
- **C1** (`specrun`→`specify` in vectis surfaces + CI grep gate): actual **+23/−8** across 4 tracked files + new `scripts/check-no-specrun.sh` vs predicted small. Fixed **6** agent-facing occurrences (`vectis/briefs/build.md` ×2, `rules/platform-shell-presence.md` ×1, `docs/reference/targets/vectis.md` ×3); **~77** historical occurrences across 6 files explicitly allowlisted (`decision-log.md`, `rfcs/roadmap.md`, `rfcs/future/**`, etc.); ~58 gitignored `docs/book/**` generated-HTML files auto-excluded (git-grep is tracked-only). Gate wired into a `check-no-specrun` Makefile target (a `lint` prereq) + CI `ci.yaml`; allowlist is an append-only bash `:(exclude)` pathspec array. Done-when flipped cleanly: 0 `specrun` in agent-facing sources, gate exits 0, reintroduction → exit 1 (verified both directions, reverted). No regress from my change; `make lint` still exits 2 on **pre-existing** CORE-016/CORE-051 findings in concurrent-agent WIP files I didn't touch.
- **A2 + B8** (move domain logic out of handlers): actual **net ≈ +91** (tracked −53 = 218 ins/271 del + new `merge/clone_commit.rs` 105 + `tests.rs` 39) vs predicted net-positive (matches refactor prior). A2.1 `topology_cache_staleness`→`specify_workflow::registry::cache_staleness`; A2.2 new pure `plan_next_body(...)->NextBody` in `change/plan/core/next.rs` (mirrors `plan_finding`); A2.3 git side effects→new `specify_workflow::merge::clone_commit` (`auto_commit` returns `Vec<String>` warnings the handler renders, journal events stay bracketed). All three handlers now thin journal+write brackets. B8: merge tests followed to `merge/clone_commit/tests.rs`; `agents.rs`/`upgrade.rs`/`source/prep.rs`/`lint/run_tests.rs` left as genuine binary-only glue (logic not crate-resident — correctly not force-moved). Done-when flipped cleanly; cross-repo `rg` found no parent-repo references to changed symbols. No regress: fmt/clippy clean, `specify-workflow` 668/668. **Follow-up fix:** the 2 failures the A2 subagent saw were mine from earlier steps — B1's 4 over-long test fn names (renamed ≤40 chars) and C1's rule-body edit drifting `rules_export::vectis_golden` (regenerated `tests/fixtures/rules-export/vectis.json`); both gates now green (`rust_quality` 2/2, `rules_export` 12/12).
- **A3 + A4** (split god modules + relocate inline tests): roughly LOC-neutral split (façade decls + re-export blocks) as predicted. **A3**: `journal.rs` 929→**192** (+ `event.rs` 593/`append.rs` 118/`emit.rs` 60); `adapter/core.rs` 762→**521** (+ `resolve.rs` 172/`validate_manifest.rs` 118); `skill_frontmatter.rs` 452→**49** (+ 6 per-check submodules) with a new `Context::memoize` so `load_skill_entries` walks once per `Context` (2→1 internal walks). **A4**: inline tests relocated to `plugins/tests.rs` (219), `upgrade/tests.rs` (92), `validate/tests.rs` (213). Done-when flipped cleanly: public API byte-identical (façades re-export exact paths); the 2 files still >450 (`event.rs` 593, `core.rs` 521) are single irreducible concerns (closed `EventKind` enum / manifest DTO model). Cross-repo `rg` (item 5): parent references only module/public paths, all preserved — no caller breaks. No regress: clippy clean (fixed 2 surfaced nits via a `SkillEntries` newtype + `ScanCache` alias), fmt clean, 1369 pass / 1 skipped; goldens byte-identical.
- **C2** (rewrite 3 target reference docs to 2.0 model): actual **+44/−56** (net −12) across `docs/reference/targets/{vectis,omnia,contracts}.md` vs predicted "rewrite" (net-negative, matches deletion-heavy doc edits). Each page's retired `### Define phase` (target brief *writing* `proposal.md`/`spec.md`) replaced with a `shape | build | merge` op model matching `index.md` and each `adapter.yaml` `briefs.keys()`: shape = read-only input to core `/spec:refine` synthesis, build/merge = the target's own ops (vectis composition+shells+verify, omnia trait/guest/WASM, contracts OpenAPI/AsyncAPI/JSON-Schema sub-flows). 3 stray `specrun init`→`specify init` also fixed. Done-when flipped cleanly: 0 "Define phase"/target-writes-spec framing remain (grep-verified), all 21 brief links resolve. `make lint` flags none of the 3 files (its exit-2 findings are the pre-existing concurrent-agent CORE-016/051 in other files).
- **B2 + B6 + B7** (test infrastructure): actual **+282/−2586** tracked + 18 new files (16 per-CORE modules 2318, `standards/tests/common/mod.rs` 33, `tests/README.md` 54) — net-negative, dominated by the `core_parity.rs` 2431→**49** collapse. **B2**: kept one `core_parity` binary, split the file into 16 per-CORE modules under `tests/core_parity/` via `#[path]` (largest now `core_007.rs` 239); `make_rule`/`hint`/`NoToolRunner` single-sourced in `eval_support` (behaviour-safe — evaluator ignores `origin`/`path`/`title`). Left `workspace.rs`/`journal.rs`/`fan_in_fan_out.rs` (flat/cohesive, no seams — splitting = churn). **B6**: canonical `REGENERATE_GOLDENS=1 cargo nextest run [-p <crate>] --test <binary>` across 11 sites + `tests/README.md` binary→fixture index. **B7**: chose per-crate `tests/common` (not a support crate — over-engineering a 12-line helper); `copy_dir` 8→3 copies, `snapshot_tree` published. Done-when flipped cleanly for all three. No regress: fmt/clippy clean, 1305 pass / 1 skipped, counts unchanged (zero `#[test]` added/removed).
- **C3–C6** (contributor-doc refresh, spec-runtime gate, RFC-31 archival, rules quality): actual **+125/−17** tracked + new `rfcs/done/rfc-31-declarative-lints.md`, integrated additively around the concurrent agent's in-flight files. **C3**: `checks.md` §5 → plugin-qualified SKILL `name` (`specify-merge`≠dir `merge`), scenario `[define,…]`→`[refine,…]`, `codex.*`→`rules.*` wire-ids; `cli-architecture.md` `specify_change`/`specify_config`→both `specify_workflow` (the brief's `specify-model` guess was wrong — `ProjectConfig` lives in `crates/workflow/src/config.rs`); `skills-test-coverage.md` retired-skill de-tally + `refine`; decision-log per-entry `Status: current|historical` (49 markers, 8 historical). **C4**: added the missing CI `sync && git diff --exit-code` gate (verified passing). **C5**: completed RFC-31 archival to lowercase `rfcs/done/rfc-31-declarative-lints.md` (matches the `checks.md` link, clears `CORE-002`), fixed `../DIAGNOSTICS.md`/self links; roadmap dead links de-linked; decision-log `plugins/change/`→`plugins/spec/`. **C6**: chassis-quirk authoring checklist (core README) + UNI review-only note + UNI-019↔UNI-002 cross-links; **deferred** the ~35-rule CORE→native-hint burn-down (needs specify-cli parity modules; CORE tree is concurrent-agent's active zone — high collision, doc-only gain). Done-when flipped cleanly per finding. `make lint` 24→23 (cleared the 1 fixable finding, added 0); remaining 23 pre-existing (CORE-016 on untracked `REVIEW.md`/untouched `checks.md` prose, CORE-051 on adapter manifests).
- **A5 + A6 + A8** (dedup + error/DTO polish + perf): actual **+251/−214** across 11 files + new `lint/index/languages.rs` (39), ~LOC-neutral as predicted. **A5**: single `languages.rs` (`LANGUAGES`/`infer_language`) feeds `files.rs`+`framework.rs` (include-filters left separate — intentionally not the lang map); framework lint schema validation routed through `specify_schema::cached_validator` (ad-hoc `Mutex<HashMap>` deleted; `$ref` registry statics correctly kept); REQ-ID grammar single-sourced as `REQ_ID_PATTERN` in `specify-model`. **A6**: schema-cache poison → `Error::Diag{schema-cache-poisoned}` (no CLI panic); `error.rs` field → `code: Cow<'static,str>`; `plan_finding` hot path → `&'static str`; `NextBody.reason` → typed `NextReason` kebab enum (wire `drained`/`stuck`/`in-progress` byte-identical). **A6.5 deferred** (Error::Workspace: ~65 Diag sites; a closed kind enum is sprawling and half-migrates the Diag-first policy). **A8.2** confirmed fixed by A1; **A8.3** removed always-empty `reserved_skipped`; **A8.1 deferred** (lock-fingerprint: `cache_staleness` is content-equality — mtime fingerprint would falsely skip; a faithful digest needs a cross-repo `topology.lock` schema+golden change). Done-when flipped cleanly for the applied items. Cross-repo: parent consumes the binary only, no API coupling. No regress: fmt/clippy clean, 1428 pass/1 skipped. **Follow-up fix:** the 5 `rules_export` failures were mine — C6's `UNI-002`/`UNI-019` cross-link edits drift the omnia/contracts/vectis exports; regenerated those goldens → `rules_export` 10/10.
- **A7 + B10 + B11** (archaeology + proptest/table-driven + gate decision): actual **+300/−70** across 12 files + new `tests/cli_errors.rs` (92) + `proptest` dev-dep (Cargo.lock +54). **A7**: moved genuine history → `DECISIONS.md` pointers (≤3-line "what today") in `propose/kernel.rs`, `slice/build/wire.rs`, `plan/core/model.rs` + 1 found workspace-wide (`framework/check/links.rs`); archaeology **214→202**. Done-when flipped **partially**: residual 202 are **false positives** — the predicate's `RFC-`/`Phase ` markers fire on canonical contract vocabulary (`RFC-29 D2`, `RFC-36`) that `AGENTS.md` uses as stable anchors; reaching 0 would delete legitimate references. **B10**: `proptest 1.11.0` (dev-dep on workflow + diagnostics), 9 property tests across 3 adoptions (config version-ordering, plan cycle detection, fingerprint stability) + table-driven `cli_errors.rs` locking `Exit::from(&Error)` over exit codes 0–4 (re-exported `Exit` from `runtime`). **B11**: **promoted** `rust.allow-without-reason` → hard gate `no_bare_allow_attributes` (0 real bare allows; confirmed fail-when-violated then green); **deferred** the archaeology gate (perpetually red given the 202 canonical residuals); `docs/quality-debt.md` updated (bare-allow complete, archaeology reduced to 202 with rationale). B10/B11 flipped cleanly. No regress: fmt/clippy clean, 730 workflow+diagnostics pass + 9 new proptests + 6 root `rust_quality`/`cli_errors`; goldens byte-identical.
- **B3 + B4 + B5 + B9** (sweep — brittle assertions, weak setup, determinism, thin tests): **B3** — `mutate.rs` now parses YAML (`load_plan()`+`entry.status`/`depends_on`); `slice_build.rs`+`source_survey.rs` parse journal via the new published `common::read_journal_normalized` helper (modeled on `journal.rs`). **B4** — `support::add_pending_entry`/`add_entry_with` (assert exit 0 **and** the entry landed) adopted across `mutate`/`source_binding` (6 sites)/`next`/`validate`. **B5** — archive-date tests made deterministic via the **regex** approach (verified there's no clock-injection seam: `Ctx::now()` hardcodes `Timestamp::now()`) + `date_window()` collision seeding; normalisation reused in survey/build; `Makefile.toml` `test` drops `clean`, keeps `vectis-wasm` (incremental local iteration; `RUSTFLAGS=-Dwarnings` still forces rebuild). **B9** — grew `outcome/tests.rs` (round-trips + unknown-variant rejection), `metadata/tests.rs` (missing→`ArtifactNotFound`, malformed→`YamlDe`), `error.rs` (`CliTooOld`/`ProjectNeedsMigration`/`Validation` discriminant/display/hint), `discard.rs` (illegal-terminal-transition path). Done-when flipped cleanly for all four (artifacts grep-verified present). No regress: fmt/clippy clean, **`cargo make test` = 1700 pass / 1 skipped** (decoupled task), `rust_quality` 3/3; goldens byte-identical. (Several findings were already materialised on the shared branch — verified each genuinely landed rather than re-implementing.)

### Final validation

- **`cargo make ci` (specify-cli): PASSES end-to-end** — fmt + lint + test (1700 pass / 1 skipped) + test-docs + doc + vet + outdated + deny all green (exit 0). Two completion fixes were needed beyond the per-step light cadence (which ran clippy+nextest, not `doc`/`deny`): (1) A3's `skill_frontmatter.rs` + `journal.rs` façade module-docs linked to now-private submodules → rustdoc `-Dwarnings` failed; demoted those intra-doc links to plain code spans. (2) B10's `proptest` dev-dep was declared in two crates → `cargo deny` `workspace-duplicate` ban; hoisted to `[workspace.dependencies]` and pointed both crates at `{ workspace = true }`.
- **`make lint` (specify): exits 2 on 29 findings, none a regression from this work** — 15× CORE-016 on the **untracked** `REVIEW.md` scratch (post-mortem prose cites RFC-31/39/30; deliberately uncommitted), 8× CORE-051 on `adapters/*/adapter.yaml` (untouched, pre-existing baseline), 6× CORE-016 on `docs/contributing/checks.md:46/323` (pre-existing RFC-31 prose, not the §5 lines C3 edited).
- **Concurrency note:** a parallel agent performed a large `references/spec-runtime/` → `references/runtime/` migration mid-run (deleting ~120 materialised copies + `scripts/sync-adapter-spec-runtime.sh`), which **superseded C1's `scripts/check-no-specrun.sh` grep-gate and C4's sync-gate** (both removed by that refactor). C1's content fixes (`specrun`→`specify` in vectis briefs/rules/docs) and C2/C3/C5/C6 edits remain intact in the tree.

