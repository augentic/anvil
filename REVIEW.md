# Code & Skill Review — single pass, quality-biased

**Date:** 2026-06-14 · **Scope:** `augentic/specify` + `augentic/specify-cli` (incl. shipped Skills) · pre-1.0, no back-compat.
**Persona bias:** subtraction. Every finding below reduces LOC and/or a named quality axis; nothing qualified on defect-parity.

---

## 5-line summary

1. **Top findings (sort key):** F1 delete dead `evaluate`/`evaluate_env` lint wrappers (−42); F2 delete dead diagnostics `validate`/`validate_fingerprint` + the two error variants they alone construct (−55); T1 collapse the two `relabel_*` helpers (−11). All subtraction; no defect tier.
2. **Total ΔLOC if all land:** ≈ **−111** (F1 −42, F2 −55, T1 −11, T2 −3).
3. **Primary non-LOC axes moved:** −4 public functions, −2 enum branches, −1 helper, narrowed crate public surface (module/crate edges).
4. **Verified defects closed:** **none qualified.** `cargo clippy --workspace --all-targets -- -D warnings` = clean; `specify lint framework` = 0 findings; every `unwrap`/`expect`/`panic!` reachable check landed inside `#[cfg(test)]` (operator panic surface in `src/runtime/commands/` = 0 on live paths). Net defect-only ΔLOC = **0**. Open defects remaining: 0 found.
5. **Most likely to break in remediation:** F2 — removing `validate_fingerprint` orphans `use crate::fingerprint::fingerprint;` and the `FingerprintMismatch`/`FingerprintMalformed` variants; miss either deletion and `-D warnings` fails (dead import / arm). Delete all three in the same commit.

### Reconnaissance (current-state numbers)

```
tokei (specify-cli):        Rust 80,077 code / 554 files
cargo clippy --workspace --all-targets -- -D warnings:   PASS (0 warnings)
specify lint framework --framework-root . (specify):     0 findings / 0 critical
#[test] count (crates/ src/ tests/):                     158 files
mod.rs outside tests/:                                   0
unwrap|expect (non-test, crates/ src/):                  1673 across 148 files (handler paths: agents.rs=16 all #[cfg(test)], contract/dump.rs=1)
panic!|unreachable! (non-test):                          104 across 42 files (src/runtime: 1, in #[cfg(test)])
files > 500 LOC (non-test):                              17
```

> The codebase was cleaned recently: the prior review's `DiagnosticProducer`, `ShaResolver`, `tool::hash`, `SkipDeclarative`, and `set_eq.rs` are all **already gone**; `Platform` now derives `strum::Display`/`EnumString`. Two false leads from exploration were discarded after verification: `plugins/spec/skills/init/references/` is a **symlink** to the shared `references/` (so `references/init-runbook.md` resolves), and the `explore/status/define/verify` skill dirs **do not exist** (`ls plugins/spec/skills/` → `build drop execute finalize init merge plan refine`). Both match the green lint run.

---

## Structural findings

### F1 — Delete dead `evaluate` / `evaluate_env` lint wrappers
**Evidence:** `crates/standards/src/lint/eval.rs:133` (`pub fn evaluate`) and `:156` (`pub fn evaluate_env`). The production entry is `evaluate_rules` (`runner.rs:112`), which calls `evaluate_with_cache` **directly** (`eval.rs:350`) — not `evaluate_env`. `evaluate_env` is called from exactly one place: the body of `evaluate` (`eval.rs:137`). Grep for callers:

```
rg 'eval::evaluate\b|evaluate_env\(' -> eval.rs:137 (self), :156/:322 (defs/doc); src/runtime/commands/lint.rs:4 (comment); ignore_directive_pass.rs:3 (comment). No call sites.
```

The per-kind `module::evaluate(...)` symbols (`schema::evaluate`, `regex::evaluate`, …) are unrelated and stay.

**Action:**
1. Delete `pub fn evaluate` (doc + body, `eval.rs:~119–148`).
2. Delete `pub fn evaluate_env` (`eval.rs:150–161`).
3. Fix the two now-stale references: the "standalone `evaluate_env` entry point" sentence in `evaluate_with_cache`'s doc (`eval.rs:163–168`), the `[evaluate_env]` mention in `evaluate_rules`'s doc (`eval.rs:321`), and the `lint::eval::evaluate` comment in `src/runtime/commands/lint.rs:4` → `evaluate_rules`.

**Quality delta:** `−42 LOC, −2 public fn, −1 EvalEnv-inline-construction branch`.
**Net LOC:** ~298 → ~256 in `eval.rs`.
**Done when:** `rg -n 'pub fn evaluate\b|pub fn evaluate_env\b' crates/standards/src/lint/eval.rs` returns nothing **and** `cargo clippy --workspace --all-targets -- -D warnings` stays clean.
**Rule?** No.
**Counter-argument:** "They're a tidy public seam for direct testing." — Loses: zero tests use them (tests call per-kind `evaluate` or `evaluate_rules`), and `evaluate_with_cache` already provides the seam.
**Depends on:** none.

---

### F2 — Delete dead diagnostics `validate` + `validate_fingerprint` (and their sole error variants)
**Evidence:** `crates/diagnostics/src/validate.rs:77` (`pub fn validate`) and `:141` (`pub fn validate_fingerprint`). No production caller exists — grep for `::validate(`/`validate_fingerprint` resolves only to this file's own body (`:80`), its `#[cfg(test)]` tests (`:197`, `:248`, `:263`), and the re-export (`lib.rs:38–41`). Production uses `validate_diagnostic`, `validate_diagnostic_json`, and `validate_evidence_size` individually; build-report ingestion validates against the schema (`workflow/src/schema.rs:196`), never recomputes a fingerprint. The two error variants `FingerprintMismatch` (`validate.rs:57–62`) and `FingerprintMalformed` (`:63–65`) are constructed **only** inside `validate_fingerprint`, and `use crate::fingerprint::fingerprint;` (`:25`) is used **only** by it.

**Action:**
1. Delete `pub fn validate` (`:71–82`) and `pub fn validate_fingerprint` (`:133–157`).
2. Delete the now-unconstructed `DiagnosticError::FingerprintMismatch` and `FingerprintMalformed` variants (`:55–65`).
3. Delete the now-unused `use crate::fingerprint::fingerprint;` (`:25`).
4. Drop `validate` and `validate_fingerprint` from the `pub use validate::{…}` re-export (`lib.rs:38–41`).
5. Delete the `#[cfg(test)]` tests that exercised the deleted fns (`validate.rs` aggregate-validate test `~:197`; fingerprint tamper tests `~:248–263`); trim the module-doc lines describing the removed aggregate/fingerprint checks (`:11–17`).

**Quality delta:** `−55 LOC, −2 public fn, −2 error variants, −1 cross-module import (fingerprint)`.
**Net LOC:** `validate.rs` ~296 → ~245; `lib.rs` −2 names.
**Done when:** `rg -n 'fn validate_fingerprint|pub fn validate\b' crates/diagnostics/src/validate.rs` returns nothing **and** `cargo clippy --workspace --all-targets -- -D warnings` stays clean (proves no orphaned import/variant).
**Rule?** No.
**Counter-argument:** "Fingerprint integrity should be verified on ingested adapter reports." — Loses for *this* pass: it isn't wired anywhere today (dead code), so removal is behavior-preserving; wiring it back is a feature, out of scope for a subtraction pass.
**Depends on:** none.

---

## One-touch tidies

### T1 — Collapse `relabel_with_lead` + `relabel_with_path` into one `relabel`
**Evidence:** `crates/workflow/src/schema.rs:478–486` and `:488–496` are byte-identical except the label expression (`format!("lead \`{lead}\`")` vs `path.display()`). Three call sites: `:463` (lead), `:331` and `:541` (path).

```478:496:crates/workflow/src/schema.rs
fn relabel_with_lead(mut summary: ValidationSummary, lead: &str) -> ValidationSummary {
    let detail = summary.detail.take().unwrap_or_default();
    summary.detail = Some(if detail.is_empty() { format!("lead `{lead}`") } else { format!("lead `{lead}`: {detail}") });
    summary
}
fn relabel_with_path(mut summary: ValidationSummary, path: &Path) -> ValidationSummary { /* identical but path.display() */ }
```

**Action:** Replace both with one `fn relabel(mut summary: ValidationSummary, label: impl std::fmt::Display) -> ValidationSummary`; call sites become `relabel(summary, format_args!("lead \`{}\`", lead.lead))`-style (`relabel(summary, path.display())` for the two path sites). `path.display()` and a `format!`'d lead label both satisfy `Display`.

**Quality delta:** `−11 LOC, −1 function` (2→1, M<N). One call site gains a `format!` (minor +call-site burden) — paid for by the net −11 and the collapsed function.
**Net LOC:** ~19 → ~8 for the helpers.
**Done when:** `rg -c 'fn relabel' crates/workflow/src/schema.rs` → `1`.
**Rule?** No.
**Counter-argument:** "Two named helpers read clearer." — Loses: identical bodies; readability isn't an axis and the lead label is the only differing token.
**Depends on:** none.

### T2 — Narrow the `workflow::schema` re-export to `pub(crate)`
**Evidence:** `crates/workflow/src/schema.rs:25–32` re-exports ~21 `specify_schema` symbols as `pub use`. Audited consumers: every external import via `specify_workflow::schema::` is a **locally-defined** `pub fn validate_*` (`plan/add.rs`, `amend.rs`, `remove.rs`, `propose.rs`, `slice/synthesize.rs`, `slice/build.rs`); the only internal re-export consumer is `crate::schema::validate_value_cached` (`adapter/validate_manifest.rs:18`). No code anywhere imports the constants / `compile_schema` / `validate_value` through `workflow::schema` — they import from `specify_schema::` directly.

**Action:** Change `pub use specify_schema::{…}` → `pub(crate) use specify_schema::{…}` and drop names not referenced inside `schema.rs` or via `crate::schema::` (e.g. `compile_schema`, `RULE_JSON_SCHEMA`, `RESOLVED_RULES_JSON_SCHEMA` — confirmed used only through direct `specify_schema::` imports elsewhere).
**Quality delta:** `−3 LOC (dropped dead names), −~18 names off the crate's public surface (module/crate edges)`. LOC roughly flat; justified by the strict reduction in public API surface.
**Net LOC:** `:25–32` 8 → ~5 lines.
**Done when:** `rg -n 'pub use specify_schema' crates/workflow/src/schema.rs` returns nothing; workspace still compiles under `-D warnings`.
**Rule?** No.
**Counter-argument:** "Flat LOC isn't subtraction." — Survives only on the public-surface axis; if you disallow flat-LOC tidies, drop it. Lowest-ranked finding for that reason.
**Depends on:** none.

---

## Considered and dropped (with reason)

| Candidate | Why dropped |
|---|---|
| Collapse triplicated evidence walkers (`synthesize.rs:270`, `slice/model.rs:350`, `slice/validate/model_drift.rs`) | Three sites produce **different** outputs (`ClaimKind` map / `ClaimBody` / `EvidenceFacts`) over different inputs (bound sources vs all files vs parsed `EvidenceDoc`). A shared walker needs closure plumbing — adds call-site burden, LOC saving unproven. Fails the "extract function only if ≥2 sites delete duplicate code" bar. |
| Collapse the `validate_*_json/_yaml` thin wrappers in `schema.rs` into a data table | Each is a documented named entry point with a distinct wire `code`/rule string; a table loses the doc comments and adds dispatch. No net LOC win. |
| Privatize `plan_lock::probe` | No non-test caller, but privatizing saves 0 LOC; not worth a touch. |
| Remove `tool/src/package.rs` async OCI stack (`tokio`/`wasm-pkg-client`) | Real YAGNI, but it requires editing `Cargo.toml` deps — **frozen** for this pass. |
| Delete `branding/*.md` scratch (`names.md` 213, `names-2.md` 51, `names-3.md` 33, `pitch.md` 89) | ~386 LOC of non-framework scratch, but outside "code & skills" scope and plausibly intentional product material — defer to the owner. |
| Skill frontmatter↔body "drift" (e.g. `init` description listing `workspace sync`) | Taste/clarity only; the description↔body lint predicate passes. "Readability" is not an axis. |

---

## Verdict

The repo is in good shape — the previous pass's dead abstractions are gone, CI is green on every gate, and there is **no verified defect to close**. The honest yield of a subtraction pass at this maturity is small and precise: **~111 LOC and four dead public functions**, dominated by two confirmed-dead validator/evaluator wrappers (F1, F2). Everything larger either touches frozen `Cargo.toml`, sits outside scope, or trades call-site clarity for no real LOC win — so it is dropped rather than dressed up.
