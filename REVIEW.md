# Code & Skill Review — single pass, quality-biased

Scope: `specify` + `specify-cli`, including shipped Skills. Pre-1.0, no back-compat.

## Summary

Top three by LOC removed: **(1)** delete the legacy `<slice>/journal.yaml` apparatus (~640 LOC across crate+bin+tests, RFC-25 already moved to `.specify/journal.jsonl`); **(2)** delete `ChangeBrief` parser/template plus its test block (~485 LOC, only `path()` is called and `Layout::change_brief_path()` already exists); **(3)** unify the parallel `ValidationResult` (domain) and `ValidationSummary` (error) types plus their three near-identical schema-validator helpers (~120 LOC, −1 enum, −2 functions). If every structural finding lands, ΔLOC is roughly **−1700 LOC** (~3.3% of all Rust). Non-LOC axes moved: **−5 types**, **−4 enums**, **−6 schema/validator helpers**, **−2 source files**, **−1 module crate-edge** (`slice::journal`). Most likely to break in remediation: **F3 (validation type unification)** — the `serde` wire shape for `ValidationResult` (`{status, rule-id, rule, …}`) differs from `ValidationSummary` (`{status, rule-id, rule, detail}`); the `validate slice` JSON envelope is downstream-consumed by skill drivers, so the merge must keep that envelope byte-stable.

---

## Structural findings

### F1 — Delete the per-slice `journal.yaml` apparatus

**Evidence.**
- `rg "slice journal" plugins/spec/skills` in the specify repo returns **zero** matches — no SKILL invokes `specify slice journal append|show`.
- `rg --files-with-matches Journal::append|Journal::load` in `specify-cli/` outside the journal files themselves: only `src/commands/slice/journal.rs`.
- `wc -l` of the surface: `crates/domain/src/slice/journal.rs` = **232**, `src/commands/slice/journal.rs` = **113**, `tests/slice.rs:1017‑1280` (the `journal_*` test block) = **264**, plus `JournalAction` + dispatcher arm in `src/commands/slice/cli.rs:48‑51, 232‑257` + `src/commands/slice.rs:70‑79` ≈ **35**.
- RFC-25 §Observability nominates `.specify/journal.jsonl` (the new `specify_domain::journal` module) as the single observability surface; nothing reads `<slice>/journal.yaml`.

**Action.**
1. Delete `crates/domain/src/slice/journal.rs` and remove `pub mod journal;` + the re-exports from `crates/domain/src/slice.rs`.
2. Delete `src/commands/slice/journal.rs` and remove `mod journal;` + the `JournalAction` arm from `src/commands/slice.rs`.
3. Drop `Journal { … }` from `SliceAction` and the `JournalAction` enum from `src/commands/slice/cli.rs`.
4. Delete the seven `journal_*` test fns in `tests/slice.rs` and the standalone `tests/journal.rs` references.

**Quality delta.** `−640 LOC, −2 types (Journal, JournalEntry), −1 enum (EntryKind), −1 module edge (slice::journal), −4 clap subcommands`.

**Net LOC.** ≈ 644 → 0 in touched files (one extra `JournalAction` variant deletion in `cli.rs`).

**Done when.** `rg -c 'journal\.yaml' crates/ src/` returns no hits and `cargo make ci` passes.

**Rule?** No — single-touch deletion.

**Counter-argument.** "Operators may grep `journal.yaml` to debug a slice locally." Loses: nothing in the RFC-25 workflow writes it; debug surface is `.specify/journal.jsonl`. *jj* shipped the same way when it dropped Mercurial's `.hg/journal`.

**Depends on.** none.

---

### F2 — Delete `ChangeBrief` parser, template, and its 280-line test block

**Evidence.**
- `rg "ChangeBrief::(load|parse_str|template)" specify-cli/{src,crates}` returns only matches under `crates/domain/tests/adapter.rs` (tests of itself).
- The only production call is `ChangeBrief::path(&ctx.project_dir)` at `src/commands/plan/lifecycle.rs:263`, returning `project_dir.join("change.md")` — identical to the already-present `Layout::change_brief_path()` at `crates/domain/src/config.rs:201‑203`.
- `wc -l crates/domain/src/adapter/change_brief.rs` = **205**.
- `crates/domain/tests/adapter.rs` lines 883‑1180 (every `ChangeBrief::*` test, byte-golden, parse, load, template) = **~280 LOC**.

**Action.**
1. Rewrite the one site at `src/commands/plan/lifecycle.rs:263` from `let brief_path = ChangeBrief::path(&ctx.project_dir);` to `let brief_path = ctx.layout().change_brief_path();`.
2. Delete `crates/domain/src/adapter/change_brief.rs` and its `mod change_brief;` + re-exports in `crates/domain/src/adapter.rs:28‑30`.
3. Delete the `ChangeBrief::*` test cluster (lines 883‑1180) in `crates/domain/tests/adapter.rs`.
4. Drop the now-unused `is_kebab` re-export check from `crates/domain/src/adapter/change_brief.rs:8` consumers.

**Quality delta.** `−485 LOC, −3 types (ChangeBrief, ChangeFrontmatter, ChangeInput), −1 enum (InputKind), −1 file, −1 module edge`.

**Net LOC.** ≈ 485 → 1 (the single `let brief_path = …` line).

**Done when.** `rg -c 'ChangeBrief' crates/ src/` returns 0; `make test` passes.

**Rule?** No.

**Counter-argument.** "Operators or future skills will need to parse `change.md` frontmatter." Loses: RFC-25 §Plan-driven loop says `change.md` is operator prose with no structured frontmatter contract; nothing reads it. *ripgrep* drops its `.ignore` parser when it stops being called.

**Depends on.** none.

---

### F3 — Unify `ValidationResult` (domain) with `ValidationSummary` (error)

**Evidence.**
- Two structurally-equivalent shapes:
  - `crates/domain/src/adapter.rs:46‑74` defines `ValidationResult::{Pass, Fail, Deferred}` carrying `rule_id: Cow<'static, str>`, `rule: Cow<'static, str>`, plus per-arm `detail`/`reason`.
  - `crates/error/src/validation.rs:18‑29` defines `ValidationSummary { status: Status (Pass|Fail|Deferred), rule_id: String, rule: String, detail: Option<String> }`.
- Manual adapter `validation_failures` at `crates/domain/src/adapter/codex.rs:207‑224` converts one into the other (**18 LOC** of bridge that exists only because both shapes exist).
- Three schema-validator helpers do the same job, returning whichever shape the caller wants:
  - `validate_against_schema` (adapter/adapter.rs:336‑376) → `Vec<ValidationResult>` (**~40 LOC**).
  - `validate_value` (schema.rs:147‑165) → `Result<(), Error::Validation>` (**~20 LOC**).
  - `run_schema` (plugin/core.rs:289‑315) → `Result<(), Error::Diag>` (**~27 LOC**).
- `Finding::into_summary` (spec/provenance.rs:169‑188) is another manual lift over the same data (**~20 LOC**).

**Action.**
1. Delete `ValidationResult` and its imports in `crates/domain/src/adapter.rs`, `adapter/adapter.rs`, `adapter/cache.rs`, `adapter/codex.rs`, `merge/slice/read.rs`, `merge/validate.rs`, `validate.rs`, `validate/run.rs`.
2. Make every callsite use `ValidationSummary` directly (`detail = Some(reason)` for the former `Deferred.reason`).
3. Collapse the three schema helpers into one helper `validate_value` in `crates/domain/src/schema.rs` returning `Vec<ValidationSummary>`; rewrite the seven callers to use it (5 in adapter/, 2 in plugin/, 1 in schema/).
4. Delete `validation_failures` (codex.rs) and `Finding::into_summary` collapses into a single `Finding -> ValidationSummary` `From`.

**Quality delta.** `−120 LOC, −1 enum, −3 schema-validator functions, −1 module's-worth of Cow<'static, str> plumbing, −1 cross-crate type duplication`.

**Net LOC.** ≈ 280 → 160 (collapses two schema-validator bodies into one).

**Done when.** `rg -c 'ValidationResult' crates/` returns 0; `serde_json::to_value(&report)["brief-results"]["proposal"][0]["status"]` still serialises as `"pass"` (test at `crates/domain/src/validate.rs:159‑195` passes after rewrite).

**Rule?** No.

**Counter-argument.** "`Cow<'static, str>` lets us cite a `rule.description` directly without `.to_string()` on the hot path." Loses: every actual call site already constructs the `Cow` from `.into()` of a `&'static str`, so the win is paper — `ValidationSummary::rule: String` adds one heap allocation per finding (≤ dozens per run) and removes a whole type. *cargo's* `cargo_diagnostic::Diagnostic` carries `String` rule labels for the same reason.

**Depends on.** none.

---

### F4 — Delete dead `to_string() == "refined"` branch

**Evidence.** `src/commands/slice/lifecycle.rs:55‑64`:

```55:64:src/commands/slice/lifecycle.rs
    if metadata.status.to_string() == "refined" {
        let event = specify_domain::journal::Event::new(
            Timestamp::now(),
            specify_domain::journal::EventKind::SliceTransitionRefined {
                slice_name: name.clone(),
            },
        );
        specify_domain::journal::append(ctx.layout(), &event)?;
    }
```

- `LifecycleStatus` (crates/domain/src/slice/lifecycle.rs:23‑36) variants are `Defining | Defined | Building | Complete | Merged | Dropped` — no `Refined`.
- `#[strum(serialize_all = "kebab-case")]` therefore never yields `"refined"`; `rg 'Refined|Refining|"refined"' crates/domain/src` finds matches only inside `journal.rs` (the event type).
- Net effect: `SliceTransitionRefined` is provably never emitted from this site. The branch is unreachable.

**Action.** Delete lines 53‑64 of `src/commands/slice/lifecycle.rs` (the comment plus the `if` block). The `Timestamp::now()` import survives because the `TransitionBody` write still uses timestamps via `metadata.*_at`.

**Quality delta.** `−12 LOC, −1 unreachable branch, −1 misleading comment ("Other lifecycle transitions are not in the v1 event set")`.

**Net LOC.** 12 → 0.

**Done when.** `rg -n '"refined"' src/` returns 0.

**Rule?** No — single dead branch.

**Counter-argument.** "Maybe `LifecycleStatus::Refined` is coming." Loses: if it is, the typed `match` arm will be a separate landing; the stringly-typed `to_string() == "refined"` is fragile regardless, and it currently does nothing. Restoring it later is one line.

**Depends on.** none.

---

### F5 — Collapse the source/target resolve command pair

**Evidence.**
- `src/commands/source.rs` (60 LOC) and `src/commands/target.rs` (62 LOC) are byte-identical apart from `Axis::Source` ↔ `Axis::Target` and the `value.split_once('@')` stripping in target — the entire `ResolveBody`, `write_resolve_text`, and `resolve` body duplicate.
- `src/commands/source/cli.rs` (25 LOC) and `src/commands/target/cli.rs` (28 LOC) each declare a one-variant `*Action::Resolve { name|value, project_dir }` enum.
- `Axis` already carries `dir_segment()` + `Display` so the rendering branches by axis with no further dispatch.

**Action.**
1. Move the `ResolveBody` + `write_resolve_text` + `resolve` body into `src/commands.rs` as a private free function `resolve_plugin(format, axis, value, project_dir)` (the rule allows it because the same finding deletes two identical impls).
2. In the new helper, do `let name = if matches!(axis, Axis::Target) { value.split_once('@').map_or(value, |(n,_)| n) } else { value };` — covers both call sites.
3. Delete `src/commands/source.rs` and `src/commands/target.rs`; have `Commands::Source { … } | Commands::Target { … }` dispatch directly to `resolve_plugin`.
4. Keep `source/cli.rs` and `target/cli.rs` (they own clap surface text); only the run-side collapses.

**Quality delta.** `−50 LOC, −1 duplicate impl pair (resolve() bodies), −2 duplicate ResolveBody structs, −1 duplicate write_resolve_text`.

**Net LOC.** 122 → ~70 (one shared helper + the dispatch arms).

**Done when.** `rg -c 'fn write_resolve_text' src/` returns 1 (was 2); `cargo make ci` passes.

**Rule?** No.

**Counter-argument.** "Source vs target may diverge later." Loses: they share `axis: source|target`, `operations`, and `description` by RFC-25 §Adapter implementation shape — divergence would land back-compatibly as new arms, not as cloned files. *clap-rs* shares the `*Action::Resolve` shape across its `bin`/`lib` modes the same way.

**Depends on.** none.

---

### F6 — Collapse `Divergence` + `DivergenceState` into one enum

**Evidence.**
- `crates/domain/src/change/plan/core/model.rs:177‑191` declares `Divergence::{Likely, Accepted, Rejected}` (3 variants; absence on disk = "none").
- `crates/domain/src/journal.rs:146‑170` declares `DivergenceState::{None, Likely, Accepted, Rejected}` (4 variants) plus a hand-written `From<Option<Divergence>>` impl (~10 LOC) that exists *only* to bridge the two encodings.
- Two enums + one `From` impl + two test fns (`crates/domain/src/journal.rs:289‑296`, +1 in journal tests) for the same closed taxonomy.

**Action.**
1. Add `None` to `Divergence` with `#[serde(other)]` or keep `Option<Divergence>` on disk and have the journal carry `Option<Divergence>` directly with `#[serde(skip_serializing_if = "Option::is_none")]` on `from`/`to`. Either way, delete `DivergenceState`.
2. Delete `From<Option<Divergence>> for DivergenceState` and `divergence_state_from_option_divergence_round_trip`.
3. Update `PlanAmendDivergence { from, to }` fields to `Option<Divergence>` (or `Divergence` with the new variant), update payload serde tests at journal.rs:268‑287.

**Quality delta.** `−45 LOC, −1 enum, −1 From impl, −1 round-trip test`.

**Net LOC.** ≈ 50 → 5.

**Done when.** `rg -c 'DivergenceState' crates/ src/` returns 0.

**Rule?** No.

**Counter-argument.** "The journal wire form pins `none|likely|accepted|rejected`, and `Option` would serialise as `null` instead." Loses: with `#[serde(rename_all = "kebab-case")]` on `Divergence` plus an explicit `None → "none"` rename (or a single custom `Serialize` impl that's still net-cheaper than the second enum), the wire shape is byte-identical and one type carries it.

**Depends on.** none.

---

### F7 — Trim `ToolError` to its load-bearing variants

**Evidence.**
- `crates/tool/src/error.rs` = **272 LOC**.
- `impl From<ToolError> for specify_error::Error` (lines 246‑272) shows the wire collapse: every variant except `ToolNotDeclared` / `InvalidPermission` / `PermissionDenied` lowers to `Error::Diag { code: "tool-…", detail }`. Of the 9 non-Diag variants, 6 (`InvalidCacheSegment`, `Runtime`, `InvalidSource`, `EmptySource`, `DigestMismatch`, `InvalidPermission` *as Diag*) are converted to `Diag` at the boundary with their `.to_string()` payload.
- The same crate has 13 named helper constructors (`cache_io`, `cache_root`, `manifest_read`, `manifest_parse`, `atomic_move_failed`, `package`, `package_label`, `sidecar_parse`, `sidecar_schema`, `network_status`, `network_timeout`, `network_malformed`, `network_too_large`, `network_other`) — every one already builds a `ToolError::Diag` directly (~85 LOC).

**Action.**
1. Delete the variants that round-trip to `Error::Diag` at the boundary: `InvalidCacheSegment`, `Runtime`, `InvalidSource`, `EmptySource`, `DigestMismatch`. Move call sites to construct `Self::Diag { code: "tool-…", detail }` inline (the 13 helpers already do this).
2. Keep only `Diag`, `ToolNotDeclared`, `InvalidPermission`, `PermissionDenied` — the three the boundary routes off of via `From<ToolError> for Error`.
3. Drop the now-unused `runtime` / `invalid_source` helpers (`pub fn runtime`, `pub fn invalid_source`) since their callers move to `Diag`.

**Quality delta.** `−100 LOC, −5 enum variants, −2 redundant constructors`.

**Net LOC.** ≈ 130 → 30 (variants + helpers).

**Done when.** `rg -c 'ToolError::(Runtime|InvalidCacheSegment|InvalidSource|EmptySource|DigestMismatch)' crates/tool/src/` returns 0; `cargo nextest run -p specify-tool` green.

**Rule?** No.

**Counter-argument.** "`DigestMismatch { expected, actual }` is testable by destructure." Loses: `rg 'ToolError::DigestMismatch' crates/` returns matches only inside `error.rs` itself and `package.rs` where it is constructed — no test or skill destructures it; the `expected`/`actual` already appear in the `Display` body, which is what every consumer reads.

**Depends on.** none.

---

### F8 — Drop `Pipeline.plan` + `Phase::Plan` arm

**Evidence.**
- `crates/domain/src/adapter/adapter.rs:33‑35` declares `Pipeline.plan: Vec<PipelineEntry>` "Optional Layer 2 authoring-phase briefs for `/change:draft`" — the `/change:draft` skill is gone per `.cursor/rules/project.mdc` ("`change` is on-disk vocabulary, not a slash-command namespace") and RFC-25 §"Hard cut from 1.x".
- `Adapter::plan_entries()` (lines 218‑220) is called only by `PipelineView::load` (pipeline.rs:45) to fold plan-phase briefs into a fake "pre-define" ordering — every consumer (slice `validate`, status, completion) then filters by `Phase::{Define,Build,Merge}` anyway.
- `rg "Phase::Plan" crates/ src/` returns matches in `adapter/adapter.rs` and `adapter/pipeline.rs` only; `validate/run.rs:31` iterates briefs without dispatching on `Plan`.

**Action.**
1. Delete `Pipeline.plan` field, `plan_entries()` method, the chained iteration in `entry()` (adapter.rs:227‑232), and the `Phase::Plan` arm.
2. Inline the iteration in `PipelineView::load` (pipeline.rs:45‑46) to drop the `plan_iter.chain(…)` prefix.
3. Drop `merge_phase(parent.pipeline.plan, child.pipeline.plan)` from `Adapter::merge`.
4. Update the embedded `adapter.schema.json` to remove the `plan` array.

**Quality delta.** `−30 LOC, −1 enum variant, −1 field, −1 method`.

**Net LOC.** ≈ 35 → 5.

**Done when.** `rg -c 'Phase::Plan|pipeline\.plan\b' crates/ src/` returns 0.

**Rule?** No.

**Counter-argument.** "Tier-2 authoring may come back." Loses: it shipped once and was retired in RFC-25; if it returns, it'll be source `enumerate` + target `shape`, not a fourth pipeline phase. *helix*'s editor commands collapsed the analogous "authoring mode" the same way.

**Depends on.** none.

---

### F9 — Flag (do not fix yet): `crates/domain/src/adapter/` ↔ `crates/domain/src/plugin/` shape duplication

**Evidence.**
- `wc -l crates/domain/src/adapter/*.rs crates/domain/src/plugin/*.rs` totals **~1675 LOC** of legacy axis-agnostic loader against **~340 LOC** of RFC-25 axis-aware loader.
- `Adapter::resolve` + `Adapter::locate` + `AdapterSource` + `ResolvedAdapter` (adapter/adapter.rs:129‑162) duplicate `Plugin::resolve` + `Plugin::locate` + `PluginLocation` + `ResolvedPlugin` (plugin/core.rs:173‑256) with `axis: source|target` injected.
- `PipelineView::load` is the only thing still routing through the old `Adapter` shape; per RFC-25 §"`refine → build → merge`" the `Pipeline.{define,build,merge}` chain itself is legacy.

**Action.** Not actionable in one finding — call this out as the largest open subtraction. Resolution path: migrate `PipelineView`, `Codex*`, `slice validate`, `validate/run.rs`, and `init/regular.rs` onto `Plugin` + per-axis `briefs.<op>` lookups, then delete `crates/domain/src/adapter/` wholesale. Expected savings if done in a follow-up: ≥ 1000 LOC, −5 types (`Adapter`, `ResolvedAdapter`, `AdapterSource`, `Pipeline`, `PipelineEntry`), −1 enum (`Phase`).

**Quality delta.** flag-only — no edits in this pass.

**Net LOC.** n/a (signpost).

**Done when.** Future PR: `rg -c 'crate::adapter::Adapter\b' crates/ src/` returns 0.

**Rule?** No.

**Counter-argument.** "Migration is risky." Loses: the `Plugin` loader already runs in `specify source resolve` / `specify target resolve` / `specify tool run` paths; the open question is the brief-ordering contract (`needs` / `tracks`), which RFC-25 §"Adapter implementation shape" already replaces with operation-set ordering.

**Depends on.** F8 (drop `Pipeline.plan` first to shrink the migration surface).

---

## One-touch tidies

### T1 — Drop the SKILL's references to dead `plan transition` targets

**Evidence.** `plugins/spec/skills/drop/SKILL.md:22‑23` documents `specify plan transition <name> failed` / `… blocked`; `src/commands/plan/lifecycle.rs:240‑244` returns `Error::Argument` for `"blocked" | "failed" | "skipped"` with the message *"per-entry `<x>` is not a v1 state — RFC-25 collapsed the per-entry enum to `pending | in-progress | done`."* The operator copy-pasting from the skill hits exit 2.

**Action.** Delete `plugins/spec/skills/drop/SKILL.md:19‑26` (the advisory `failed`/`blocked` block); replace with one sentence pointing at `specify plan transition <name> done` or `specify plan amend` for re-binding sources.

**Quality delta.** `−8 LOC, −1 skill/CLI drift surface`.

**Done when.** `rg -n 'plan transition.*(failed|blocked)' plugins/spec/skills/` returns 0.

### T2 — Inline `commands/slice.rs::artifact_classes`

**Evidence.** `src/commands/slice.rs:31‑46` hardcodes the two-class `specs` + `contracts` set; `rg artifact_classes src/` shows four calls inside `commands/slice/`. Pre-1.0 with one target writer per slice, the per-call cost of inlining is zero.

**Action.** Inline at the four sites (`merge::{run, preview, conflicts}` + `touched::specs`) — each call expands to a 12-line `vec![…]`. Net wash, but the lookup-then-inline cost is paid back by deleting the module's only `pub(super)` export.

**Quality delta.** wash-LOC, `−1 inter-module API edge`. Drop unless F1/F2 land.

### T3 — Replace `ChangeBrief::path` with `Layout::change_brief_path`

**Evidence.** Both functions return `project_dir.join("change.md")`. Even without F2, the production call at `src/commands/plan/lifecycle.rs:263` is the duplicate.

**Action.** One-line site swap as part of F2; standalone if F2 is parked.

**Quality delta.** `−4 LOC, −1 duplicate accessor`.

### T4 — Drop `Patch<T>` from `EntryPatch::status`

**Evidence.** `crates/domain/src/change/plan/core/model.rs:332‑352, 364‑386` already calls out that `status` is deliberately absent. The `Patch<T>` enum carries three variants (`Keep`, `Clear`, `Set`); only `project` / `target` / `description` use the three-way form. `status_omits_field` test (line 607) is the only consumer of the negative invariant. Patch could be `Option<Option<T>>` (or two bool flags) with zero loss, but the rule forbids rename-only changes; skip unless `Patch<T>` deletes alongside something else.

### T5 — Delete duplicate atomic-write `pre_post` test pre-condition assertion in slice/journal.rs

**Evidence.** `crates/domain/src/slice/journal.rs:171‑231` test `append_never_truncates_on_mid_write_error` does ~60 LOC of byte-level "prefix is intact" assertions — duplicated by `crates/domain/src/config/atomic.rs::yaml_write` invariants which are already covered there. If F1 lands this test is deleted automatically; if not, it is the lowest-value test in the crate to keep.

### T6 — Collapse three `validate_*` helpers

**Evidence.** Even without F3, three helpers (`validate_against_schema` adapter/adapter.rs:336, `validate_value` schema.rs:147, `run_schema` plugin/core.rs:289) compile and run a JSON schema. They return different result shapes only because of F3's two-type split.

**Action.** Lift one private `compile_schema(&str) -> Result<Validator, Error>` into `crates/domain/src/schema.rs` (it already exists at line 167). The three call sites then call it directly; the per-helper wrappers shrink by their `compile + iter_errors` boilerplate (~15 LOC each).

**Quality delta.** `−30 LOC, −1 schema-compile helper duplication`.

### T7 — Delete dead `let last = first;` in `is_valid_source_key`

**Evidence.** `crates/domain/src/spec/provenance.rs:615` assigns `let mut last = first;` and `last = b;` inside the loop, used only at line 629 (`last != b'-'`). The `prev_dash` flag already records the same fact (trailing dash → loop exited with `prev_dash == true`). The `last` variable is redundant — return `!prev_dash`.

**Action.** Replace the three `last`-related lines with `Ok(!prev_dash)` in the return position; ~5 LOC.

**Quality delta.** `−5 LOC, −1 redundant local`.

### T8 — Drop `Patch::Default` derive from `Patch<T>`

**Evidence.** `crates/domain/src/change/plan/core/model.rs:332` derives `Default`. The only construction site is `EntryPatch::default()` which currently relies on the field-by-field derive of `EntryPatch` itself. Inlining `Patch::Keep` everywhere drops the derive and the `#[default]` attribute (~3 LOC). Drop unless paired.

### T9 — `Adapter::probe_dir` returns `Option<PathBuf>` that only tests use

**Evidence.** `rg 'Adapter::probe_dir' crates/ src/` shows hits only in `adapter.rs:131` (self) and `crates/domain/tests/adapter.rs` (test). Inline at the one production site, delete the method (~10 LOC).

### T10 — Drop the `ChangeBrief` re-export comment block in adapter.rs even if F2 is parked

**Evidence.** `crates/domain/src/adapter.rs:28‑30` re-exports `ChangeBrief, ChangeFrontmatter, ChangeInput, FILENAME as CHANGE_BRIEF_FILENAME, InputKind` — five names, four of which are unused outside the file itself (per F2 grep). Even without deletion, the four-of-five re-export shrinks to one (`CHANGE_BRIEF_FILENAME`). `−4 LOC, −4 cross-module use edges`.

---

## Ranking and dependencies

The structural findings rank cleanly by LOC: **F1 (640) > F2 (485) > F3 (120) > F7 (100) > F5 (50) > F6 (45) > F8 (30) > F4 (12)**. F9 is the open architectural call-out. All findings stand alone; F9 needs F8 first, and several tidies (T2/T3/T6/T10) collapse into F1/F2/F3 if those land. No new modules, no new traits, no new dependencies, no `Cargo.toml` edits, no clippy ratchets — every saving is a deletion.

---

## Post-mortem

One line per applied finding: actual ΔLOC vs predicted, did the "done when" assertion flip cleanly, did anything regress.

- **F1** — actual −643 vs predicted −640 (0.5% under-estimate from doc/comment retargets); `rg -c 'journal\.yaml' crates/ src/` returns 0 and `cargo make ci` green; no regressions.
- **F2** — actual −518 vs predicted −485 (+6.8% from AGENTS/DECISIONS/architecture doc retargets, same calibration shape as F1); `rg -c 'ChangeBrief' crates/ src/` returns 0 and `cargo make ci` green; T3/T10 subsumed; no regressions.
- **F3** — actual −53 vs predicted −120 (~56% under; unification undershoots pure-deletion because per-arm constructors and unified-helper bodies live where the enum used to); `rg -c 'ValidationResult' crates/` returns 0 and `cargo make check` green; T6 subsumed; wire envelope byte-stable for Pass/Fail, `Deferred.reason` → `detail` (no skill driver reads the old key); two clippy nits fixed in place; goldens regenerated (4 e2e + 4 domain), diff is exactly the documented key swap.
- **F4** — actual −13 vs predicted −12 (+8%, accounting noise on the closing brace); `rg -n '"refined"' src/` returns 0 and `cargo make check` green; no regressions. Side observation: `EventKind::SliceTransitionRefined` is now production-unused — every remaining reference is the variant declaration or a self-test. Flagged for a future finding (restore emitter, gate `#[cfg(test)]`, or remove + RFC-19 update).
- **F5** — actual −45 vs predicted −50 (~10% under, tighter than F3 because the `<module>.rs` + `<module>/` architecture rule pinned a ~9-LOC `pub mod cli;` shell on each side); `rg -c 'fn write_resolve_text' src/` returns 1 (was 2) and `cargo make check` green; envelope byte-stable including the `target@v1` strip; no regressions.
- **F6** — actual −28 vs predicted −45 (~38% under; shape A — `Divergence` gains `None` variant + `#[serde(rename = "none")]`, journal carries `Divergence` directly, on-disk stays `Option<Divergence>`); `rg -c 'DivergenceState' crates/ src/` returns 0 and `cargo make check` green; all four `plan-amend-divergence-*` goldens byte-stable; no schema diff (plan.schema.json's `Divergence` enum is journal-orthogonal); no regressions. Unification cost lives in the merged type's expanded doc-block (~12 LOC).
- **F7** — actual **+28** vs predicted **−100** (sign flip; brief mis-sized the inline-rewrite cost); structural wins still landed: −5 variants, −2 helpers, −1 cross-crate duplication, byte-identical wire for `DigestMismatch`/`Runtime`; `rg -c 'ToolError::(Runtime|InvalidCacheSegment|InvalidSource|EmptySource|DigestMismatch)' crates/tool/src/` returns 0 and `cargo make check` green; no regressions. Calibration learning: inlining typed variants into `Diag { code, detail: format!(...) }` adds +2–3 LOC per callsite under `rustfmt`, and net-positive when callsites > helpers — a counter to the F1/F2 pure-deletion prior.
- **F8** — actual **−220** vs predicted **−30** (~7× over; source-only delete was −44 LOC near the prior, but the deletion surfaced 114 LOC of dead tests and 64 LOC of orphaned fixtures whose only purpose was exercising the removed surface); `rg -c 'Phase::Plan|pipeline\.plan\b' crates/ src/` returns 0 and `cargo make check` green; schema step was a no-op (already cleaned in a prior pass); no cross-repo touches; no regressions. Calibration learning: pure deletions can have long fixture/test tails when the removed surface had its own happy-path tests.
- **T1** — actual −7 vs predicted −8 (+1 LOC from the replacement sentence in `drop/SKILL.md`, plus a sibling sweep in `execute/references/stop-conditions.md`); `rg -n 'plan transition.*(failed|blocked)' plugins/spec/skills/` returns 0 and `make checks` shows only pre-existing failures (unrelated broken link in `tests/fixtures/skills/execute/README.md`). Two upstream drift sites flagged for follow-up tidies: `plugins/spec/references/phase-outcome-contract.md` (descriptive `failed`/`blocked` prose + outcome-translation table — needs a deliberate answer for `deferred` under the v1 enum) and `docs/reference/cli/plan.md` `unreachable-entry` recovery hint (recommends a transition the CLI now rejects).
- **T2** — actual 0 vs predicted ~0; chose **private `fn`** over the brief's literal 4× inline (which would have been +36 LOC and 4× duplication) and over `const` slice (unreachable: `ArtifactClass` carries `String` + `PathBuf` with no `const` ctors). Demoted `pub(super)` → private; one canonical declaration retained; `cargo make check` green; no regressions. Brief's "wash-LOC" framing was incorrect for its own prescribed shape — flagged so future audits don't repeat the calculation.
- **T7** — actual −2 vs predicted −5 (brief over-counted; the return line was an in-place edit, not a deletion); `cargo make check` green and all 9 `is_valid_source_key` tests pass; equivalence proof recorded (`prev_dash == (last == b'-')` invariant holds at every loop exit because the `!ascii_alnum` branch exits before either assignment); no regressions.
- **T9** — actual −1 vs predicted −10 (audit miscounted callers: claimed 1 prod + 1 test, reality 3 prod + 0 test; inlining at three sites converted clean deletion into a near-wash); `rg -c 'probe_dir' crates/ src/ tests/` returns 0 and `cargo make check` green; no test churn; no regressions. Calibration learning: audits citing `Self::method` references should always also `rg` the bare method name to catch external callers.

### Final deep validation

`cargo make ci` in `specify-cli` initially flagged two rustdoc broken intra-doc-links introduced by F3 (`[ValidationSummary::Pass]` on `schema.rs:153`; bare `[ValidationStatus]` on `validate.rs:5`). Both came from `-Dwarnings` on the `doc` stage, which `cargo make check` does not run. Fixed in place (5 net characters changed across two lines: `Summary::Pass` → `Summary` and brackets removed from one bare link). `cargo make ci` then green end-to-end (lint + file-size + test + test-docs + doc + vet + outdated + deny + fmt). `make checks` in the parent repo reports two pre-existing failures in `tests/fixtures/skills/execute/README.md` (broken link to `rfcs/rfc-25-plan.md`); the file is untouched this session (`git status` clean, last commit `4e0b69f wave 3 & 4`) — flagged for a future tidy, not a regression. Calibration learning: per-item `cargo make check` is faster but lacks the `doc` stage; for findings that touch doc-comments (notably structural unifications), the `check` → `ci` delta surfaces broken intra-doc-links that only the full suite catches.

### Totals across this session

Structural findings (F1–F8): predicted −1482 LOC, actual −1502 LOC (−1.4% under as a sum, but with wide per-finding variance from −56% to +7×). Tidies applied (T1, T2, T7, T9): predicted −23 LOC, actual −10 LOC. Tidies subsumed by structural findings (T3, T5, T6, T10): no extra work. Tidies skipped per their own rubric (T4, T8). F9 stays flagged.

Calibration shape:
- **Pure deletions of dead surface** (F1, F4, F8): hit prediction ±10% for the source delta, but can blow through by 5–7× when the deletion unblocks orphaned tests/fixtures (F8: −220 actual vs −30 predicted).
- **Unifications that fold types into a single carrier** (F3, F5, F6): undershoot by 10–56% because helper bodies, expanded doc-blocks, and module-carrier shells absorb LOC that the deleted type's `wc -l` did not account for.
- **Enum-trim with inline `Diag` rewrites** (F7): can invert the sign (+28 vs −100) when `rustfmt` reflow on `Diag { code, detail: format!(...) }` literals exceeds the helper savings; net-positive only when helpers > callsites.
- **Audit miscounts on callers** (T9): predicted −10, actual −1, because the audit's grep didn't catch all callers; always cross-check with the bare method name, not just the type-qualified form.

