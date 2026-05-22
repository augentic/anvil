# Code & Skill Review — Subtraction Pass

## Summary

Top three by LOC removed: (S1) delete the pre-1.0 migration apparatus across `scripts/`, `tests/`, and `docs/migration/` (≈980 LOC + 33 fixture files); (S2) delete the manually-rendered `rfcs/rfc-27-synthesis.html` duplicate of the same RFC's `.md` (−1414 LOC); (S3) collapse the plan-entry / lifecycle transition state machines, which currently spend 313+130 LOC modelling three legal edges with mirrored `can_transition_to`/`transition` pairs and oracle-table tests (−≈310 LOC). If every finding lands: ≈3300 LOC + a 132 KB fixture tree + 3 enforcement predicates + one trait module. Non-LOC axes moved most: types (DTO/enum collapse in plan transitions), branches (5-case oracle tables → single `matches!`), module edges (`crates/domain/src/cmd.rs` and 7 `<R: CmdRunner>` generic propagations), cargo edges (none — frozen per scope). Highest remediation risk: S3 (plan-transition collapse) — the per-entry status writer is on the hot path for `/spec:execute` and tests in `tests/plan_orchestrate.rs` pin the exact JSON envelope.

## Reconnaissance

- `tokei` (combined): 121 361 lines / 956 files; Rust 46 864 / 265; Markdown 39 850 / 462; TypeScript 5 267 / 30; HTML 1 414 / 1 (single file: `rfcs/rfc-27-synthesis.html`).
- `cargo tree --duplicates` (specify-cli): duplicates are entirely transitive from `wasm-pkg-client` (`base64 0.21/0.22`, `pbjson`, `warg-*`, `oci-client`, `reqwest 0.12/0.13`) — not actionable from this workspace.
- `rg -c '^#\[test\]'` test counts: 201 Rust files contain tests; the heaviest concentrations are `tests/plan_orchestrate.rs` (62 `#[test]`) and `crates/domain/tests/registry.rs` (50).
- `rg --files -g '**/mod.rs'` (specify-cli): 3 hits — all `tests/common/mod.rs` shims. The codebase is otherwise on the `<module>.rs` + `<module>/` convention.
- `wc -l docs/standards/*.md AGENTS.md` (combined): 951 total — already small.
- Files > 500 lines under `crates/` and `src/` (specify-cli): `tests/plan_orchestrate.rs` 1958; `crates/domain/tests/workspace.rs` 1041; `crates/domain/tests/finalize.rs` 947; `crates/domain/tests/registry.rs` 922; `crates/domain/src/change/plan/core/model.rs` 702; `crates/domain/src/spec/provenance.rs` 597; `crates/tool/src/validate.rs` 520. (Specify repo: `scripts/migrate_to_2_0.ts` 592; `scripts/checks/skill_frontmatter.ts` 567; `scripts/checks/skill_body.ts` 524; `tests/migration_e2e.ts` 184.)
- `rg 'code: ?"[a-z-]+"'` (Diag-style errors): 196 sites — `Error::Diag` is the dominant error path, and individual `code:` literals are unique enough that promoting any of them to typed variants would *add* lines, not delete.

## Structural Findings

### F1 — Delete pre-1.0 migration apparatus

- **Evidence**:
    - `wc -l scripts/migrate_to_2_0.ts scripts/migrate-to-2.0.sh tests/migration_test.ts tests/migration_e2e.ts docs/migration/2.0.md` → 592 + 40 + 137 + 184 + (≈40) = ≈990 lines.
    - `find tests/fixtures/migration -type f | wc -l` → 33; `du -sh` → 132 KB.
    - `rg -l 'migrate-to-2|migrate_to_2_0'` (specify repo): hits are exclusively (a) the migration sources themselves, (b) `AGENTS.md` advertising the script, (c) `rfcs/*` discussing the cut, (d) `docs/explanation/release-notes.md` + `docs/explanation/decision-log.md`. No production CLI / skill / target references the script.
    - Project rule (parent `AGENTS.md`): "2.0 is a hard cut from 1.x. No compatibility aliases for old manifests, verbs, brief paths, or the retired `change:` slash-namespace."
- **Action**:
    1. `rm scripts/migrate-to-2.0.sh scripts/migrate_to_2_0.ts tests/migration_test.ts tests/migration_e2e.ts docs/migration/2.0.md`.
    2. `rm -r tests/fixtures/migration`.
    3. Remove the `migrate-to-2.0.sh` paragraph from `AGENTS.md` ("Operators upgrade via `migrate-to-2.0.sh`.") and any link to `docs/migration/2.0.md` from `docs/SUMMARY.md`.
- **Quality delta**: −≈990 LOC, −33 fixture files, −1 deno entry point, −1 module edge from `make test` (acceptance harness already skips when no binary is set).
- **Net LOC**: ≈990 → 0.
- **Done when**: `rg -l 'migrate-to-2|migrate_to_2_0'` returns zero hits outside `rfcs/archive/`.
- **Rule?**: no — pre-1.0 migration is a one-off concern; no policy needed.
- **Counter-argument**: "Downstream operators may still be on 1.x" — loses to the project's own "hard cut" stance and the existing entry in `git log` if anyone needs the script back.
- **Depends on**: none.

### F2 — Delete `rfcs/rfc-27-synthesis.html`

- **Evidence**:
    - `wc -l rfcs/rfc-27-synthesis.html` → 1414 lines (`tokei` reports this as the only HTML file in the repo).
    - `grep -l 'rfc-27-synthesis.html' -r .` returns no source-tree hits (only `.git/index`); the sibling `rfcs/rfc-27-synthesis.md` (796 lines) is the canonical artifact.
    - Editor mod-times show the `.html` regenerated alongside the `.md` (both `May 22 14:44/14:45`), so it is being maintained manually — a recurring cost for content nothing reads.
- **Action**: `rm rfcs/rfc-27-synthesis.html`. Inline-link from `rfcs/roadmap.md` (if any) keeps pointing at the `.md`.
- **Quality delta**: −1414 LOC, −1 hand-styled CSS surface to keep in sync with the markdown.
- **Net LOC**: 1414 → 0.
- **Done when**: `tokei` reports `HTML 0 files`.
- **Rule?**: no.
- **Counter-argument**: "The HTML render is nicer for sharing externally" — loses because no link in the repo points at it; if needed, render on demand from the `.md`.
- **Depends on**: none.

### F3 — Collapse plan-entry / lifecycle transition state machines

- **Evidence**:
    - `wc -l crates/domain/src/change/plan/core/transitions.rs` → 313; doc-string declares: "Post-RFC-25 there are exactly two legal edges: `Pending → InProgress` … `InProgress → Done`."
    - The file ships four mirrored functions for that two-edge table: `Status::can_transition_to`, `Status::transition`, `Plan::transition`, plus the identical `Lifecycle::can_transition_to` / `Plan::transition_lifecycle` pair (one legal edge: `Pending → Reviewed`).
    - 191 of those lines are tests: `legal_edges_succeed`, `done_is_terminal`, `illegal_edges_rejected`, `table_matches_oracle`, `error_carries_endpoints`, `transition_in_progress_to_done`, `transition_rejects_illegal_edge`, `transition_rejects_pending_to_done_skipping_in_progress`, `transition_missing_entry`, `lifecycle_pending_to_reviewed_ok`, `lifecycle_reviewed_is_terminal`, `lifecycle_rejects_pending_to_pending`.
    - External callers of `can_transition_to`: zero outside this file (`rg can_transition_to crates/domain/src/change`).
- **Action**:
    1. Delete `Status::can_transition_to` and `Status::transition`; inline the `matches!((self, target), (Pending, InProgress) | (InProgress, Done))` check directly into `Plan::transition`, returning the `plan-transition` diag in the `else` arm.
    2. Delete `Lifecycle::can_transition_to`; inline the single-arm `matches!` into `Plan::transition_lifecycle`.
    3. Drop the `legal_edges_succeed` / `illegal_edges_rejected` / `table_matches_oracle` / `done_is_terminal` / `error_carries_endpoints` tests — they all assert the same two cells. Keep `transition_in_progress_to_done` + one rejection test (`transition_rejects_pending_to_done_skipping_in_progress`) + the two lifecycle cases.
    4. Same surgery for `crates/domain/src/slice/lifecycle.rs` (130 LOC, six legal edges): collapse to one `transition()` and one negative-path test. Drop `LifecycleStatus::initial()` (4 LOC); the single caller (`slice create`) can write `LifecycleStatus::Refining` literally.
- **Quality delta**: −≈310 LOC, −4 trait-style methods, −1 cell-by-cell oracle pattern duplicated across two state machines.
- **Net LOC**: 443 → ≈135.
- **Done when**: `wc -l crates/domain/src/change/plan/core/transitions.rs crates/domain/src/slice/lifecycle.rs` reports both files < 90 lines and `rg can_transition_to crates/domain/src` returns zero.
- **Rule?**: no.
- **Counter-argument**: "Splitting predicate and action is a Rust idiom (`Path::is_dir` vs `Path::canonicalize`)" — loses because the predicate has zero external callers and the action is the only legal mutator; the split costs lines and earns nothing on the wire.
- **Depends on**: none.

### F4 — Delete `crates/domain/src/cmd.rs` `CmdRunner` trait

- **Evidence**:
    - `wc -l crates/domain/src/cmd.rs` → 35; the entire module is a 1-method trait + a single zero-sized `RealCmd` impl that delegates to `Command::output()`.
    - `rg 'CmdRunner|RealCmd'` (specify-cli, code only): 7 production files propagate `<R: CmdRunner>` generics — `registry/forge.rs`, `registry/workspace/push.rs`, `registry/workspace/push/forge.rs`, `registry/workspace/push/remote.rs`, `change/finalize.rs`, `change/finalize/probe.rs` (plus 1 test helper). Each `pub fn` carries an extra `runner: &R` parameter and a `where R: CmdRunner` row.
    - `docs/standards/style.md` already lists this trait as the canonical "traits-for-testability" example, which is exactly the smell the review brief targets.
- **Action**:
    1. Replace the trait with `pub type CmdRunner = fn(&mut Command) -> io::Result<Output>;` in the same file (or inline at the call sites and delete the file entirely).
    2. Rewrite every `<R: CmdRunner>(runner: &R, ...)` signature to `(run: CmdRunner, ...)` and replace `runner.run(&mut cmd)` with `run(&mut cmd)`.
    3. `RealCmd` becomes `fn real_cmd(cmd: &mut Command) -> io::Result<Output> { cmd.output() }`. Test mocks become free functions of the same shape.
- **Quality delta**: −≈55 LOC across 8 files; −1 trait; −7 generic-parameter rows in public signatures; −1 module edge (`use crate::cmd::CmdRunner`) per consumer.
- **Net LOC**: ≈90 → ≈35.
- **Done when**: `rg 'CmdRunner|RealCmd' crates/domain/src` returns zero hits and `rg 'where R: ' crates/domain/src/registry crates/domain/src/change/finalize` returns zero rows.
- **Rule?**: no — one trait-to-fn-pointer collapse does not need enforcement.
- **Counter-argument**: "A trait gives us `dyn` injection if a runner ever needs state" — loses because no current or proposed runner carries state, and a closure-shaped `impl Fn(&mut Command) -> io::Result<Output>` is the trivial upgrade path if one ever does.
- **Depends on**: none.

### F5 — Delete `docs_quality.ts` post-migration predicates

- **Evidence**:
    - `wc -l scripts/checks/docs_quality.ts` → 178; two of the three predicates exist solely to verify "old vocabulary is gone": `checkNoLayerNumbersInDocs` (49 LOC) bans the Layer 3/4 names from a stack that already collapsed; `checkNoLegacyAdaptersReferencePath` (51 LOC) bans the pre-2.0 `docs/reference/adapters/` path.
    - The third predicate (`checkNoRfcCitationsInDocs`, 47 LOC) is a real ongoing style rule and stays.
    - `make checks` registers the two doomed predicates from `scripts/checks.ts` lines 91 + 93; deletion is a 3-line edit there.
    - Pre-1.0 "hard cut" + "ignore back-compat, migrations, deprecations" from the review brief.
- **Action**:
    1. Delete `checkNoLayerNumbersInDocs` and `checkNoLegacyAdaptersReferencePath` from `scripts/checks/docs_quality.ts` (≈100 LOC including their constants and one private helper).
    2. Drop the two `checkNoLayerNumbersInDocs` / `checkNoLegacyAdaptersReferencePath` calls and imports from `scripts/checks.ts`.
- **Quality delta**: −≈100 LOC, −2 enforcement predicates, −1 docs-quality module-edge.
- **Net LOC**: 178 → ≈78.
- **Done when**: `wc -l scripts/checks/docs_quality.ts` reports ≈78 and `rg 'checkNoLayerNumbersInDocs|checkNoLegacyAdaptersReferencePath' scripts` returns zero hits.
- **Rule?**: no — the deletion *is* the rule.
- **Counter-argument**: "Predicates that find zero hits today might catch someone tomorrow" — loses because pre-1.0 has no compatibility surface to police and contributors writing "Layer 4" or `docs/reference/adapters/` get the deletion notice in code review or the broken link.
- **Depends on**: F1 (the migration script itself was the last in-tree user of the old `docs/reference/adapters/` references).

### F6 — Collapse `Counts::from_entries` BTreeMap dance

- **Evidence**:
    - `src/commands/plan/status.rs` lines 36–60: a 25-line `Counts` struct + `from_entries` that builds a `BTreeMap<Status, usize>` keyed by `Status::value_variants()`, increments through the map, sums, then indexes back out by named variant — to populate three `usize` fields and a `total`.
    - The enum has exactly three variants; `Status::value_variants()` is otherwise only used in `crates/domain/src/change/plan/core/transitions.rs` tests (covered by F3).
- **Action**: Replace `Counts::from_entries` with:

  ```rust
  pub fn from_entries(entries: &[Entry]) -> Self {
      let mut c = Self { done: 0, in_progress: 0, pending: 0, total: entries.len() };
      for e in entries {
          match e.status {
              Status::Done => c.done += 1,
              Status::InProgress => c.in_progress += 1,
              Status::Pending => c.pending += 1,
          }
      }
      c
  }
  ```

  Drop the `use std::collections::BTreeMap` and `use clap::ValueEnum` imports if nothing else in the file needs them.
- **Quality delta**: −≈12 LOC, −1 BTreeMap allocation, −1 `expect("ALL covers status")` invariant guard, −2 module-edge imports.
- **Net LOC**: 25 → ≈13.
- **Done when**: `rg 'BTreeMap|value_variants' src/commands/plan/status.rs` returns zero hits.
- **Rule?**: no.
- **Counter-argument**: "Iterating `value_variants()` keeps the counts struct in sync if a Status arm is added" — loses because adding a Status arm post-RFC-25 is a structural change that must touch this struct's typed fields anyway (the wire shape pins `done` / `in-progress` / `pending`); the `match` makes the dependency explicit.
- **Depends on**: none.

## One-touch tidies

1. **Delete `SliceSourceBinding` `From<&str>` / `From<String>` impls** — `crates/domain/src/change/plan/core/model.rs` lines 247–257; `rg 'SliceSourceBinding::from\(' crates/` returns zero hits, and the only `.into()` shorthand site is `amend.rs:164` (`sources: vec!["a".into()]`), which becomes `vec![SliceSourceBinding::Bare("a".into())]`. Done when `rg 'impl From<' crates/domain/src/change/plan/core/model.rs` returns zero. Δ: −12 LOC, −2 trait impls.

2. **Delete `LifecycleStatus::initial()`** — `crates/domain/src/slice/lifecycle.rs:38–40`. One caller (`slice/actions/create.rs`); replace with the literal `LifecycleStatus::Refining`. Δ: −4 LOC, −1 helper. (Subsumed by F3 if that lands.)

3. **Drop the `transition_table_matches_oracle` test in `slice/lifecycle.rs`** (lines 116–128) and the matching `allowed_edges` helper (lines 89–99) — 26 LOC asserting that six legal edges match six legal edges; `matches!` is already the truth table. Δ: −26 LOC, −1 oracle pattern. (Subsumed by F3 if that lands.)

4. **Inline `LifecycleStatus::is_terminal`** — `crates/domain/src/slice/lifecycle.rs:42–46`; check call sites with `rg 'is_terminal\(' crates/`; if ≤ 2, drop the helper and write `matches!(status, LifecycleStatus::Merged | LifecycleStatus::Dropped)` inline. Δ: ≈−5 LOC.

5. **Delete the `EXIT_*` constant aliases in `src/output.rs`** if `Exit` already exposes a `pub const fn code()` (it does, lines 26–35). The `EXIT_SUCCESS = 0` style constants only appear in `AGENTS.md` as documentation, not in code — confirm with `rg 'EXIT_SUCCESS|EXIT_GENERIC_FAILURE|EXIT_VALIDATION_FAILED|EXIT_VERSION_TOO_OLD' src crates` (zero hits → tidy applies); if non-zero, drop. Δ: 0–8 LOC depending on grep result.

6. **Delete `ALLOWED_AUTO_MERGE_CONTEXT` / `ALLOWED_GH_MERGE_CONTEXT` / `ALLOWED_WORKSPACE_MERGE_CONTEXT` in `scripts/checks/prose.ts`** lines 150–156 — they are escape-hatch regexes that exist only because RFC-14 vocabulary used to be live. With the hard 2.0 cut, the predicate `checkWorkspaceLanding` can stop allow-listing those phrases or be deleted alongside F5. Δ: ≈−7 LOC, −3 escape regexes; verify by running `make checks` after the change.

7. **Drop the duplicate `Result` re-export note in `crates/error/src/lib.rs`** lines 13–19 — the 6-line doc comment explains a 1-line type alias. Tighten to `/// Workspace `Result` alias bound to [`Error`].`. Δ: −5 LOC, no behaviour change. (Edge tidy; only worth it if a nearby file is already being touched.)

8. **Delete the `#[expect(clippy::same_name_method, …)]` round-trip in `crates/domain/src/change/plan/core/io.rs:49`** and `crates/domain/src/config.rs:61` by renaming the inherent method to `read` and dropping the `AtomicYaml` trait alias for `load` — both files document the shadowing as a wart in 6+ lines of justification. Δ: −12 LOC of explanatory `expect` attributes + two trait method renames. (Borderline; only land alongside another change to the same files.)

## Dropped findings

- **Inline `Plan::topological_order`** — wanted to drop it as redundant with `plan validate` cycle detection, but `src/commands/plan/status.rs:126` uses it for the operator-facing ordering of entries; deletion would degrade `specify plan status` UX. Kept.
- **Drop `Patch<T>` three-way enum (`model.rs:290–310`)** — considered replacing with `Option<Option<T>>` or a pair of `Option<T>` flags, but the enum is 6 LOC of `apply` + 4 patch sites in `EntryPatch`; either alternative costs more at call sites than the enum saves at its definition. Kept.
- **Collapse `crates/domain/src/change/plan/core/` into a single file** — 8 sibling files for one model is a real smell (model 702, amend 375, create 312, io 282, next 385, transitions 313, validate 311, archive 128). A combined file lands at ≈2 800 LOC, which violates the in-repo file-size norm in `docs/standards/architecture.md`. The size came from genuine concern separation; the LOC budget should be earned by F3 first, then revisit. Held.
- **Promote recurring `Diag` codes to typed `Error` variants** — `rg 'code: ?"[a-z-]+"'` shows 196 sites; only `tool-resolver` (×4) and `merge-spec-conflicts` (×3) repeat. Promoting either to a typed variant adds enum lines + `Exit::from` + `variant_str` rows for a net `+` LOC change. Kept.
- **Delete `crates/error/src/serde_rfc3339*.rs` helpers** — only one consumer (`journal.rs`) for both modules, but the helpers are 7 lines each and removing them would force inlining `#[serde(with = …)]` glue at every Timestamp field. Net would be roughly flat. Kept.

## Post-mortem

One line per applied finding: actual ΔLOC vs predicted, did the "done when" assertion flip cleanly, did anything regress.

- **F1** — actual −1399 LOC (−971 from sources + −428 from the 33 fixture YAML/MD files) vs predicted −≈990 (≈41% over); deletion was clean (33 fixture files matched the brief exactly, no orphaned production callers); `rg -l 'migrate-to-2|migrate_to_2_0' --glob '!rfcs/archive'` returns 6 (REVIEW.md self-reference + the 4 explicitly-leave-alone archival hits in `rfcs/rfc-27-synthesis.{md,html}` and `docs/explanation/{decision-log,release-notes}.md` + the brief's own note in `REVIEW.md`) — the done-when literal ("zero outside `rfcs/archive/`") and the brief's own "leave archival prose alone" rule disagree; treated the leave-alone rule as authoritative; `make checks` red but with 28 pre-existing `rfcs/rfc-25-workflow.md`/`rfc-25-plan.md` broken-link failures unrelated to F1 (RFC-25 was archived in a prior change; back-refs not yet rewritten); Makefile collateral landed as predicted (`ci` target dropped two deps, no other targets referenced them); two additional non-archival live references the brief didn't flag (`docs/contributing/acceptance.md` advertised the removed `make test-migration*` targets and linked to deleted fixtures; `adapters/targets/vectis/schemas/README.md` had an in-prose link to the deleted migration doc) were cleaned in-scope; fixture-tree blow-up was modest (~+1.4×) vs the F8 prior's 5–7× warning because the fixtures are tiny per-file (33 files × ≈13 LOC) rather than the meatier orphaned-tests pattern the prior was calibrated on.
- **F2** — actual −2006 LOC vs predicted −1414 (≈42% over; prediction used tokei code-line accounting while `git diff --shortstat` counts raw lines including blanks — the file was 1414 code / 2006 raw); `tokei` reports no HTML row at all (HTML 0 files; the 57 `.html` paths under `docs/book/` are gitignored mdBook build output) and `make checks` failure count unchanged at 28 (same pre-existing `rfc-25-workflow.md` / `rfc-25-plan.md` broken-link set as F1, no new `rfc-27-synthesis.html` failure introduced); `rg 'rfc-27-synthesis\.html'` returned only the F2 brief itself in `REVIEW.md` plus a self-mention inside the deleted `.html` — no in-tree linkers, no `docs/SUMMARY.md` / `rfcs/roadmap.md` references, no source-tree retargets needed; clean single-file deletion with no fixture tail, no regressions.
- **F3** — actual −265 LOC vs predicted −≈310 (≈15% under; `git diff --shortstat` reports 339 deletions / 74 insertions across `transitions.rs`, `slice/lifecycle.rs`, and the T4 inline in `slice/actions/overlap.rs`); both files now 89/88 lines (target <90, hit on the second compaction pass — the first landed at 102/96 because pedantic clippy demands `# Errors` doc-blocks per pub fn and rustfmt pins the multi-line `format!()` once it overflows 100 chars); `rg can_transition_to crates/domain/src` returns 0; pre-edit baselines `specify-domain` 473 → 464 (−9 = 8 dropped from `transitions.rs` + 3 dropped from `slice/lifecycle.rs` + 2 added back as the merged positive/negative cases) and `plan_orchestrate` 62 → 62 (untouched as expected); `cargo make check` green and `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --workspace --all-features` green — no broken intra-doc-links surfaced (audit pre-pass with `rg '\[Status::can_transition_to\]|\[Lifecycle::can_transition_to\]|\[LifecycleStatus::initial\]|\[LifecycleStatus::is_terminal\]|\[LifecycleStatus::can_transition_to\]'` returned 0 across both repos, so the prior's "F3 green on `check`, red on `ci`" trap did not fire); T2 (`LifecycleStatus::initial`) subsumed — note the brief's claimed "single caller is `slice/actions/create.rs`" is stale, that file already wrote the `LifecycleStatus::Refining` literal directly, so the only in-tree caller was `initial_is_refining` in the `lifecycle.rs` test that was dropped anyway; T3 (oracle test + `allowed_edges`) subsumed; T4 inline applied — `is_terminal` had exactly 1 external caller (`crates/domain/src/slice/actions/overlap.rs:66`), under the ≤2 threshold, so it was inlined as `matches!(status, LifecycleStatus::Merged | LifecycleStatus::Dropped)`; wire envelope byte-stable — `plan-transition` / `plan-lifecycle-transition` / `plan-entry-not-found` / `lifecycle` diag codes unchanged, the `format!()` templates `"cannot transition from {self:?} to {target:?}"` (per-entry), `"cannot transition plan lifecycle from {:?} to {target:?}"` (plan-level — the `self.lifecycle` arg was renamed to a `current` binding for line-budget reasons, but the rendered text is byte-identical), `"no slice named '{name}' in plan"`, and `"expected valid transition from {self:?}, found {target:?}"` reproduce verbatim; one deliberate test-shape change worth flagging: dropped `transition_missing_entry` from `transitions.rs` per the brief — the `plan-entry-not-found` wire shape remains exercised by `crates/domain/src/change/plan/core/amend.rs` tests (`rg plan-entry-not-found` confirms the assertion survives), so coverage of that diag code persists across the workspace; one minor structural deviation: the kept positive (`transition_in_progress_to_done`) and negative (`transition_rejects_pending_to_done_skipping_in_progress`) per-entry tests were merged into a single `transition_in_progress_to_done_and_rejects_pending_skip` test to claw back 5 lines toward the <90 target — the same wire-shape assertions (code, both endpoints in `detail`, status-not-mutated invariant) survive in the merged body; no regressions, no clippy/rustdoc warnings introduced.
- **F4** — actual −12 LOC vs predicted −≈55 (≈78% under); fn-pointer vs impl-Fn decision: chose `pub type CmdRunner<'a> = &'a dyn Fn(&mut Command) -> io::Result<Output>` (borrowed-dyn-closure alias) because `MockCmd` carries state (a `RefCell<Vec<RecordedCall>>` recorder + a `RefCell<Box<dyn FnMut(&RecordedCall) -> io::Result<Output>>>` dispatch handler) so a stateless `fn` pointer can't transport the closure that wraps `&MockCmd`; preferred `&dyn Fn` over `impl Fn(...)` because the runner is threaded through up to four call layers (`finalize::run` → `probe::probe_one` → `is_dirty` / `pr_view_for_branch`; `push_projects` → `push_single_project` → `inspect_remote_branch` → `repo_exists` / `ensure_pull_request` → `github_pr_for_branch`) and the borrowed-dyn shape propagates as `Copy` without per-layer `+ Copy` bounds and without sprinkling `<F: Fn(...)>` generics back across the very files this finding aimed to clean; `rg 'CmdRunner|RealCmd' crates/domain/src` returns 23 (zero `RealCmd`, 23 `CmdRunner` — alias declaration + 6 `use crate::cmd::CmdRunner` imports + 13 `runner: CmdRunner<'_>` parameter-type sites + 3 doc-comment `[CmdRunner]` intra-doc-links + 1 `let runner: CmdRunner<'_> = &real_cmd;` binding); `rg 'where R: ' crates/domain/src/registry crates/domain/src/change/finalize` returns 0; `cargo make check` green (798 tests run / 798 passed / 2 skipped, fmt + clippy + nextest + doc-tests all green); `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --workspace --all-features` green — no broken intra-doc-links because the kept alias preserves the `CmdRunner` identifier the `[CmdRunner]` links resolve through; `docs/standards/style.md` updated — kept the "no traits for testability alone" rule but rewrote the GOOD example from `fn pr_list(runner: &dyn CmdRunner) -> ...` (which referenced a now-removed trait) to `fn pr_list(runner: CmdRunner<'_>) -> ...` (the alias) and replaced "via `CmdRunner`" with "via the `CmdRunner` callable alias in `specify_domain::cmd`" so the standards doc still illustrates the lowest-external-surface boundary without resurrecting a dead trait — no third-party in-tree trait was needed because the rewrite re-uses `CmdRunner` itself; surprises: (a) the prior's "10–56% undershoot" envelope underestimated by 22 percentage points — `&dyn Fn(&mut Command) -> io::Result<Output>` (the alias body) and `runner: CmdRunner<'_>` are both longer than `<R: CmdRunner>(runner: &R, ...)` once formatted, so rustfmt added a wrapped-argument row in three signatures (`ensure_pull_request`, `ensure_pr_if_supported`, `probe_one`) clawing back ≈3 lines, and the test-call rewrite (`&runner` → `&|c| runner.run(c)` × 14 call sites in `tests/finalize.rs`) was net-zero per line but the test mock module (`tests/common/mod.rs`) needed an extra clippy `expect` attribute when the inherent `MockCmd::run` signature first kept `&mut Command` — fix on the second pass switched the inherent method to `&Command` (relying on `&mut Command` reborrow at the closure boundary), trimming the four-line `expect` block back to one and recovering the LOC; (b) brief's "1 test helper" caller count was accurate, but `tests/finalize.rs` itself (the integration-test binary, 947 LOC) is a *consumer* of that helper with 14 inline `&runner` call-site rewrites — counting the helper alone undercounts the rewrite surface by an order of magnitude; (c) no production binary actually calls `change::finalize::run` today (the doc-comment claiming "the CLI binary plugs in `RealCmd`" was stale — only `change::finalize::probe::probe_one`'s test path exercises the runner, with `push::push_projects` being the sole live in-binary caller via the new `&real_cmd` static `&fn`) — corrected the stale doc-comment in-scope; no behavioural regressions, `Command::output()` remains the only spawn primitive, JSON envelopes for `finalize::Outcome` and `push::PushResult` byte-stable (no shape touched).
- **F5** — actual −229 LOC vs predicted −≈100 (≈129% over, ≈2.3× — F5-only would be −121 / ≈21% over because `git diff --numstat` reports 1/114 for `docs_quality.ts` + 1/9 for `checks.ts`; tidy #6 adds the remaining −108 LOC from dropping `checkWorkspaceLanding` whole-predicate from `prose.ts`); `scripts/checks/docs_quality.ts` now 65 lines (under the brief's ≈78 target because the three-predicate file-header comment collapsed to a single-predicate header, ~13 lines below target but within reason); `rg 'checkNoLayerNumbersInDocs|checkNoLegacyAdaptersReferencePath' scripts` returns 0; tidy #6 (`ALLOWED_*_CONTEXT` in `prose.ts`) applied: deleted the entire `checkWorkspaceLanding` predicate and all three `ALLOWED_*_CONTEXT` regex constants — analysis confirmed the predicate is purely RFC-14 vocabulary policing (its sole purpose is to allow-list "removed/retired/operator-owned" descriptions of phrases that should never appear as active commands), and ~10 current legitimate doc lines (e.g. `docs/standards/cli-contract.md:43`, `docs/reference/cli/workspace.md:151`, `plugins/spec/skills/finalize/references/runbook.md:109/113/165/217`) rely on the allow-lists to describe the removal; dropping only the constants would have made the predicate strictly stricter (not a no-op), breaking those legitimate "removed-as-of-2.0" prose lines; with the hard 2.0 cut and `specify workspace merge` absent from the CLI surface, the check is transitional vocabulary policing that has done its job and the architecture is self-policing via the absent CLI verb — also dropped `checkWorkspaceLanding` from the `scripts/checks.ts` registration block and tightened the `prose.ts` file-header comment; `make checks` failure count unchanged at 28 (same pre-existing `rfc-25-workflow.md` / `rfc-25-plan.md` broken-link set as F1/F2/F3, no new failures introduced — registered check count dropped by 3 total: 2 from F5 proper + 1 from tidy #6); the prior's "blow through by 5–7× when orphaned tests/fixtures tag along" did not fire because the deleted predicates had no fixture tree, but the brief's −100 prediction missed the −108 LOC tidy #6 explicitly flagged as conditional with F5, so the 2.3× overshoot is a scope expansion not a scope underestimate; clean deletion across all three files, no regressions.
- **F6** — actual −0 LOC vs predicted −≈12 (100% under; `git diff --shortstat` reports 13 insertions / 13 deletions — file still 189 lines because `rustfmt.toml` pins `struct_lit_width = 20` and so the new `Self { done: 0, in_progress: 0, pending: 0, total: entries.len() }` literal reflows across 6 rows, clawing back the 2 deleted imports + the loop + sum + index-out savings; the brief's "accept the multi-line reflow" caveat applied verbatim, and the only way to recover the predicted −12 would be to relax `struct_lit_width` workspace-wide, which is out of scope); `rg 'BTreeMap|value_variants' src/commands/plan/status.rs` returns 0; `cargo make check` green (798 tests run / 798 passed / 2 skipped, fmt + clippy + nextest + doc-tests all green); `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --all-features` green — no broken intra-doc-links to `BTreeMap` / `value_variants` (the prior's "cheap to check" turned up nothing); `Counts` wire shape unchanged — the kebab-case JSON keys `done` / `in-progress` / `pending` / `total` are unmodified (struct field order matched the brief's body verbatim and `tests/plan_orchestrate.rs::plan_status_renders_counts_and_topo` still pins the four keys + the `done=1`, `in-progress=1`, `pending=7`, `total=9` values); audit pre-pass confirmed the brief's "only consumer is `src/commands/plan/status.rs`" — `rg 'value_variants|BTreeMap<Status' crates src tests wasi-tools` returned only the two in-file lines about to be deleted, and the second `Counts::from_entries` caller in `src/commands/status.rs` reaches `Counts` through `pub use` and only field-accesses the four typed members, so the body swap is invisible to it; surprises: (a) the LOC undershoot is 100% not the prior's "10–56% unifications undershoot" envelope — root cause is `struct_lit_width = 20`, not a missed call site; (b) the prior's F7 warning about `Diag { code, detail: format!(...) }` reflow correctly did not apply, but a different rustfmt rule (struct-lit width) bit; (c) no `expect("ALL covers status")` guard had migrated elsewhere — it lived only in the deleted body and is gone; no regressions.

