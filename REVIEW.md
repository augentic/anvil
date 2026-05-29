# Code & Skill Review — subtraction-biased, single pass

Scope: `specify` + `specify-cli`, including shipped Skills. Pre-1.0.

## Summary (5 lines)

1. **Top three:** (A) lift the byte-identical `specdev`/`specrun` lint error mappers + helpers into `specify-lints` (~−125 LOC); (B) collapse the duplicated `map_resolve_error` (~−42 LOC); (C) collapse the verbatim-duplicated `LintFormat` enum + `From` (−22 LOC). All are subtraction; no verified defect outranks them.
2. **Total ΔLOC if all land:** ≈ **−223 LOC** (A −125, B −42, C −22, D −14, E −20).
3. **Primary non-LOC axes moved:** −2 types (one `LintFormat` mirror, one `escape_*` fn pair), −module-edge churn (two CLI trees stop carrying private copies of the same mappers), and one *latent* defect retired (the two `map_hint_error` copies have already drifted — runtime binds `op` then discards it, authoring uses `..`).
4. **Verified defects:** **none qualified.** `make lint` (specify) = "0 finding(s)"; `cargo clippy --workspace --all-targets --all-features -- -D warnings` = clean (exit 0). Non-test panic surface (`rg -c '\.(unwrap|expect)\('` = 935; `panic!|unreachable!` = 79) is almost entirely inside inline `#[cfg(test)]` modules; no operator-reachable handler panic found. Net ΔLOC from defect-only findings = **0** (≤ +30, trivially).
5. **Most likely to break in remediation:** Finding A — moving the mappers to `specify-lints` must not pull a `specify-workflow` edge (`emit_lint_completed` stays behind because it touches `specify_workflow::journal`; the sibling-crate invariant in `specify-cli/AGENTS.md` forbids `specify-lints → specify-workflow`).

---

## Reconnaissance (current state)

- `tokei`: Rust 385 files, **60,409 code lines**; Markdown 117 files.
- `cargo tree --duplicates`: `base64` v0.21/v0.22 and `reqwest` v0.12/v0.13 doubled — **all transitive under `wasm-pkg-client`/`oci-client`/`warg-*`**, none in first-party `Cargo.toml`. `Cargo.toml` is frozen for this pass; not actionable.
- test fns: **1,187**. `mod.rs` files: 5, **all under `tests/`** (allowed by `coding-standards.md`).
- files > 500 lines under `crates/`+`src/`: 24 (largest `crates/workflow/tests/workspace.rs` 1048; largest non-test `crates/specify-lints/src/rules.rs` 1016).
- `make lint` (specify): **0 findings** (0 critical/important/suggestion/optional) → no skill-predicate defects.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: **pass (exit 0)**.
- panic-adjacent: `unwrap|expect` non-test = 935; `panic!|unreachable!` non-test = 79 — sampled and found test-bound.
- `#[allow(dead_code)]` / `#[allow(unused …)]`: **0**. `TODO|FIXME|XXX|HACK`: **0**.

Net read: clean codebase. The one real structural debt is the `specdev lint` ⇄ `specrun lint` runner duplication; everything else is wire-bound or deliberately split.

---

# Structural findings

## A. Lift duplicated lint mappers into `specify-lints` — **−125 LOC**

**Evidence (current state):**

```
$ rg -n 'fn map_index_error|fn map_render_error|fn map_hint_error|fn count_status|fn emit_dump_model' src/
src/authoring/commands/lint/run.rs:374:fn map_index_error(err: IndexError) -> Error {
src/runtime/commands/lint/run.rs:346:fn map_index_error(err: IndexError) -> Error {
src/authoring/commands/lint/run.rs:518:fn map_render_error(err: RenderError) -> Error {
src/runtime/commands/lint/run.rs:458:fn map_render_error(err: RenderError) -> Error {
src/authoring/commands/lint/run.rs:403:fn map_hint_error(rule: &ResolvedRule, err: HintError) -> Error {
src/runtime/commands/lint/run.rs:386:fn map_hint_error(rule: &ResolvedRule, err: HintError) -> Error {
src/authoring/commands/lint/run.rs:361:fn count_status(...) -> u32 {
src/runtime/commands/lint/run.rs:299:fn count_status(...) -> u32 {
src/authoring/commands/lint/run.rs:313:fn emit_dump_model(model: &WorkspaceModel) -> Result<()> {
src/runtime/commands/lint/run.rs:239:fn emit_dump_model(model: &WorkspaceModel) -> Result<()> {
```

`map_index_error` (runtime 346–373 ≡ authoring 374–401), `map_render_error` (458–473 ≡ 518–533), `count_status` (299–310 ≡ 361–372), and `emit_dump_model` (239–254 ≡ 313–328) are **byte-identical**. `map_hint_error` is identical except a drifted `HintError::Filesystem` arm (runtime `{ op, path, source }` + `let _ = op;` at 438–445; authoring `{ path, source, .. }` at 455–459) — same output, proving the copies do not stay in sync.

Every one of these maps a type **owned by `specify-lints`** (`IndexError`, `HintError`, `RenderError`, `FindingStatus`, `WorkspaceModel`) to `specify_error::Error` — and `specify-lints` already depends on `specify-error` and `specify-schema` (`specify-cli/AGENTS.md` crate graph). Their natural home is `specify-lints`, not two private copies in the binary crate.

**Action:**
1. In `crates/specify-lints/src/lint/diagnostics.rs` (already the home of `RenderError`/`render`), add `pub fn map_index_error`, `pub fn map_hint_error`, `pub fn map_render_error`, `pub fn count_status`, `pub fn emit_dump_model` — paste one copy verbatim (keep the runtime copy's richer `///` mapping tables).
2. Delete all five fns from `src/runtime/commands/lint/run.rs` and `src/authoring/commands/lint/run.rs`.
3. Add `map_index_error, map_hint_error, map_render_error, count_status, emit_dump_model` to the existing `use specify_lints::lint::diagnostics::{…}` import in both files.
4. Leave `emit_lint_completed` where it is — it calls `specify_workflow::journal` and must not move (sibling-crate invariant).

**Quality delta:** `−125 LOC, −5 duplicate impls, −1 latent drift defect (map_hint_error)`.
**Net LOC:** two files ~169 + ~131 dup lines → one ~169-line home: `~470 → ~345` across touched files.
**Done when:** `rg -c 'fn map_index_error' src/` returns **0** (was 2) and `rg -c 'pub fn map_index_error' crates/specify-lints/` returns **1**.
**Rule?** no — three call sites, enforced by the dedup itself.
**Counter-argument:** "`src/.../lint/cli.rs` comments say keep `specify-lints` runtime-agnostic." Loses: that note is about *presentation* enums (`LintFormat`); mapping `specify-lints`' own error enums onto the shared `specify_error::Error` (an existing dep) is not runtime-specific, and the already-drifted `map_hint_error` shows the copies are a maintenance hazard.
**Depends on:** none.

---

## B. Collapse the documented `map_resolve_error` mirror — **−42 LOC**

**Evidence (current state):**

```
$ rg -n 'fn map_resolve_error' crates/ src/
src/authoring/commands/lint/run.rs:478:fn map_resolve_error(err: ResolveError) -> Error {
src/runtime/commands/rules/export.rs:74:pub fn map_resolve_error(err: ResolveError) -> Error {
```

The authoring copy is an explicit, self-documented duplicate:

```475:477:src/authoring/commands/lint/run.rs
/// Mirror of `src/runtime/commands/rules/export.rs::map_resolve_error`
/// kept local so the authoring tree does not depend on the runtime
```

Both bodies map `ResolveError` (a `specify-lints` type) to `specify_error::Error`, 1:1 on all four arms.

**Action:**
1. Move the single `pub fn map_resolve_error` into `crates/specify-lints/src/rules.rs` (or `rules/resolve.rs`, beside `ResolveError`).
2. Delete the copy at `src/authoring/commands/lint/run.rs:478–516` and the definition at `src/runtime/commands/rules/export.rs:74–112`.
3. Both `run.rs` files and `rules/export.rs` import it from `specify_lints` (the crate they already import).

**Quality delta:** `−42 LOC, −1 duplicate impl, −1 cross-module `use` (export.rs no longer re-exported through runtime)`.
**Net LOC:** `~81 → ~39` across touched files.
**Done when:** `rg -c 'fn map_resolve_error' crates/ src/` returns **1** (was 2).
**Rule?** no.
**Counter-argument:** "the comment deliberately kept authoring independent of the runtime module." Loses: relocating to `specify-lints` (where `ResolveError` lives) satisfies the independence goal *better* than the copy, and deletes the copy outright.
**Depends on:** none (independent of A; pairs naturally with it if both target `specify-lints`).

---

# One-touch tidies

## C. De-duplicate the `LintFormat` enum + `From` impl — **−22 LOC**

**Evidence (current state):**

```
$ rg -n 'pub enum LintFormat' src/
src/authoring/commands/lint/cli.rs:104:pub enum LintFormat {
src/runtime/commands/lint/cli.rs:102:pub enum LintFormat {
```

The enum (4 variants) and its `impl From<LintFormat> for DiagnosticsFormat` are **byte-identical** (runtime `cli.rs` 101–122 ≡ authoring `cli.rs` 103–124).

**Action:** keep the definition in `src/runtime/commands/lint/cli.rs`; in `src/authoring/commands/lint/cli.rs` delete lines 98–124 and `pub use crate::runtime::commands::lint::cli::LintFormat;`.
**Quality delta:** `−22 LOC, −1 type, −1 From impl`.
**Net LOC:** `~50 → ~28` across the two files.
**Done when:** `rg -c 'pub enum LintFormat' src/` returns **1** (was 2).
**Rule?** no.
**Counter-argument:** "the doc says kept distinct so `specify-lints` stays runtime-agnostic." Loses: both copies already live in the **binary** crate, not `specify-lints`; defining once and re-using keeps the standards crate equally untouched.
**Depends on:** none.

## D. Collapse `escape_arg` / `escape_body` into one fn — **−14 LOC**

**Evidence (current state):**

```
$ rg -n 'fn escape_arg|fn escape_body' crates/specify-lints
crates/specify-lints/src/lint/diagnostics/github.rs:66:fn escape_arg(s: &str) -> String {
crates/specify-lints/src/lint/diagnostics/github.rs:81:fn escape_body(s: &str) -> String {
```

`escape_body` (81–92) is `escape_arg` (66–79) minus the `','`/`':'` arms — same char-walk scaffolding twice.

**Action:** replace both with one fn:

```rust
fn escape(s: &str, in_arg: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '%' => out.push_str("%25"),
            '\r' => out.push_str("%0D"),
            '\n' => out.push_str("%0A"),
            ',' if in_arg => out.push_str("%2C"),
            ':' if in_arg => out.push_str("%3A"),
            other => out.push(other),
        }
    }
    out
}
```

Call sites: `escape_arg(x)` → `escape(x, true)`, `escape_body(x)` → `escape(x, false)`.
**Quality delta:** `−14 LOC, −1 fn`.
**Net LOC:** `27 → 13` for the two fns.
**Done when:** `rg -c 'fn escape_arg|fn escape_body' crates/specify-lints` returns **0**; `rg -c 'fn escape\(' crates/specify-lints/src/lint/diagnostics/github.rs` returns **1**.
**Rule?** no.
**Counter-argument:** "two named fns read clearer." Loses: readability is not an axis; the bodies are one parameterised loop.
**Depends on:** none.

## E. Share the hint-eval loop across both lint runners — **−20 LOC**

**Evidence (current state):** the per-rule deterministic-hint loop is duplicated:

```95:110:src/runtime/commands/lint/run.rs
    for rule in &resolved.rules {
        if matches!(rule.lint_mode, Some(LintMode::ModelAssisted)) {
            continue;
        }
        let Some(hints) = rule.deterministic_hints.as_deref() else {
            continue;
        };
        if hints.is_empty() {
            continue;
        }
        let outcome = evaluate(rule, hints, &model, &ctx.project_dir, &runner, next_id)
            .map_err(|err| map_hint_error(rule, err))?;
```

The authoring copy (`src/authoring/commands/lint/run.rs:181–199`) is identical except a leading `rule_filter_set` gate (182–184).

**Action:** add `pub fn evaluate_rules(rules: &[ResolvedRule], model, project_dir, runner, start_id, rule_filter: &[&str]) -> Result<(Vec<LintFinding>, Vec<ReservedSkipped>, u64), HintError>` to `crates/specify-lints/src/lint/eval.rs`; both callers replace their loop with one call (runtime passes `&[]` as `rule_filter`). Deletes the loop from both run.rs.
**Quality delta:** `−20 LOC, −1 duplicate loop`.
**Net LOC:** `~16 + ~22 → ~25` (one fn).
**Done when:** `rg -c 'for rule in &resolved.rules' src/` returns **0** (was 2).
**Rule?** no.
**Counter-argument:** "extract-function findings are discouraged." Loses: the exception explicitly applies — exactly 2 call sites delete duplicate code as a result.
**Depends on:** none (composes with A).

---

## Post-mortem

One line per applied finding: actual ΔLOC vs predicted, did the "done when" assertion flip cleanly, did anything regress.

- **A:** ΔLOC −133 (−130 incl. test-import fix) vs predicted −125 — modest overshoot (moved copies carried more shared doc/blank lines than the single home). Done-when flipped cleanly (`fn map_index_error` in `src/` 2→0; `pub fn map_index_error` in `crates/specify-lints/` =1). No regression; `cargo make check` green. Removed two now-unreachable `other =>` wildcards (same-crate `#[non_exhaustive]` no longer applies) and split two doc paragraphs for `clippy::too_long_first_doc_paragraph`.
- **B:** ΔLOC −36 (B-only, hand-isolated from A's prior edits to the two `run.rs` files) vs predicted −42 — undershoot, because only the authoring copy is a true deletion; relocating the canonical fn + its three tests into `specify-lints` is net-neutral and import-block reflows add ~+7 back. Done-when flipped cleanly (`fn map_resolve_error` in `crates/ src/` 2→1). No regression; `cargo make check` green. Same-crate clippy needed `#[must_use]` + one doc-paragraph split.
- **C:** ΔLOC −20 on the two prescribed `cli.rs` files vs predicted −22. Done-when flipped cleanly (`pub enum LintFormat` in `src/` 2→1). No regression; `cargo make check` green. One unforeseen step: the re-export path didn't resolve until `mod commands;` in `src/runtime.rs` was widened to `pub(crate) mod commands;` (net 0); also dropped now-unused `DiagnosticsFormat`/`ValueEnum` imports in authoring cli.rs.
- **D:** ΔLOC −13 (6 ins / 19 del) vs predicted −14. Done-when flipped cleanly (`fn escape_arg|fn escape_body` →0; `fn escape(` →1). Merged fn verified behaviorally identical to both originals; github diagnostics formatter tests pass. No regression; `cargo make check` green (after a one-off transient `cargo clean` filesystem race, unrelated to the edit).
- **E:** ΔLOC **+19** (E-isolated; `eval.rs` E-exclusive +52/−1, offset by ~−33 removed from the two `run.rs` loops) vs predicted −20 — wrong-direction miss. The single `evaluate_rules` fn (filter param, ModelAssisted/empty-hints skips, internal `map_hint_error` → `specify_error::Error`, tuple return, doc) is larger than the two duplicated loops, so the dedup wins single-source-of-truth but loses raw LOC. Done-when flipped cleanly (`for rule in &resolved.rules` in `src/` 2→0). Authoring `--rules` allow-list semantics preserved (runtime passes empty filter); no `specify-workflow` edge; lint runner tests pass; `cargo make check` green.

**Roll-up:** total across A–E = **−178 LOC** (`git diff --shortstat`: 385 ins / 563 del across 11 files) vs predicted ≈ −223. Shortfall driven by E reversing direction (+19 vs −20) plus modest B/C/D undershoots; A overshot. Five duplicate impls + one drifted-mapper latent defect retired; sibling-crate invariant (`specify-lints` ⊥ `specify-workflow`) held throughout.

---

## Dropped candidates (and why)

- **`Rule` ⇄ `ResolvedRule` collapse** (`crates/specify-lints/src/rules.rs` 376–405 / 434–468, bridge in `rules/resolve/sort.rs:86–120`): the `id`↔`rule-id` rename and the extra `origin`/`path-root`/`path` fields are a **deliberate wire boundary** (two separate `deny_unknown_fields` JSON schemas). ~34-line bridge, high wire-contract risk — burden of proof not met.
- **Plan `Finding` ⇄ `Diagnostic`** (`change/plan/core/model.rs:673` / `change/plan/doctor.rs:46`): `Diagnostic` legitimately adds `data: Option<DiagnosticPayload>` + `code: String`; not a 1:1 mirror. 9-line bridge — too small to justify the risk.
- **`SourceAdapter` ⇄ `TargetAdapter` twins** (`adapter/core.rs:214/251`): collapsing fights the documented F9 "operations typed at parse boundary" split (`specify-cli/AGENTS.md`). Explicit architectural decision.
- **`LintResultVersion` ⇄ `WorkspaceModelVersion`** (`lint/diagnostics.rs:32` / `lint/model.rs:48`): a shared deserialise helper saves ~6 lines but adds a cross-module `use` + a helper — net ≈ 0, adds a module edge. Dropped.
- **Transitive `base64`/`reqwest` duplicate deps:** all under vendored `wasm-pkg-client`/`warg-*`; `Cargo.toml` frozen. Not actionable.
- **`specify` skills/docs:** `make lint` returns 0 findings; no skill-integrity or frontmatter/body-cap defect to close, and no taste-only edits proposed.
