# Code & Skill Review — subtraction + defect pass

Persona: senior CLI engineer who defaults to subtraction. Scope: `specify` + `specify-cli`, including shipped Skills. Pre-1.0.

Baseline (re-verified): `specify` @ `1b005aa4`, `specify-cli` @ `43328053`. The tree advanced mid-review; every finding below was re-checked against these HEADs.

## Summary

1. **Top three (sort key):** F2 delete the always-firing `adapter.execution-agent` check (cross-repo, ~−108 LOC, −1 predicate); F3 delete the orphaned `merge-runbook.md` (−78 LOC); then the two one-touch tidies. No structural defect remains open.
2. **Total ΔLOC if all land:** ≈ **−186** (F2 −108, F3 −78, tidies ~0).
3. **Primary non-LOC axes:** −1 check/predicate, −8 perpetual SUGGESTION findings (noise surface), −1 DTO (`ExecutionProbe`), −2 hot-path branches, −1 duplicated dir-walk, public-API surface trim.
4. **Verified defects:** **none open.** The prior red `make lint` (`CORE-016` ×6 in `docs/contributing/checks.md`) was resolved upstream at `1b005aa4` — `make lint` now exits 0 (`0 critical, 0 important, 8 suggestion`). Operator-panic surface: **zero** — all 361 `unwrap`/`expect` under `crates/`+`src/` are in `#[cfg(test)]` modules (`prod_unwraps=0` across the five hottest files). Defect-only net ΔLOC = 0.
5. **Most likely to break in remediation:** F2 — it spans 6 files across both repos (predicate, re-export, severity table, predicate-bridge match, tests, rule file); miss one wiring site and `cargo make test` or `make lint` breaks.

Reconnaissance: Rust 90,688 lines / 599 files (tokei); `cargo clippy --workspace --all-targets --all-features -- -D warnings` = **pass** (re-run @ `43328053`); `make lint` = **pass** (exit 0); no `#[allow(dead_code)]` under `crates/`/`src/`; `panic!`/`unreachable!` non-test reachable from handlers = 0.

---

## Structural findings

### F2 — Delete always-firing adapter execution check

**Evidence (current state).** `make lint` emits 8 identical `[SUGGESTION] CORE-051 [adapter.execution-agent]` findings — the entire `suggestion` count — one per first-party manifest:

```
rg -l 'execution: agent' adapters/sources/*/adapter.yaml adapters/targets/*/adapter.yaml  →  8 files
make lint 2>&1 | rg -c 'adapter.execution-agent'                                           →  8
```

The predicate is `check_execution_agent` (`crates/standards/src/framework/check/adapter.rs:72`); it fires for any manifest with `execution: agent`. All 8 first-party adapters (`intent`, `documentation`, `code-typescript`, `captures`, `screenshots`, `omnia`, `vectis`, `contracts`) declare it, and the printed remediation — `switch to execution: tool once a deterministic dispatch path exists` (`adapter.rs:100`) — is impossible for inherently agent-run sources. A check firing on 100% of valid inputs with an unsatisfiable fix is a constant, not a lint. No parity test references it (`rg execution.agent crates/standards/tests/` → empty).

**Action (ordered; line numbers @ `43328053`).**
1. `specify-cli` `crates/standards/src/framework/check/adapter.rs`: delete `RULE_EXECUTION_AGENT` const + doc (`14-20`), `ExecutionProbe` + doc (`24-31`), the two `findings.extend(check_execution_agent(...))` lines (`62-63`), and `check_execution_agent` (`67-107`).
2. `crates/standards/src/framework/check.rs:24`: drop `RULE_EXECUTION_AGENT` from the `pub use adapter::{…}` re-export.
3. `crates/standards/src/framework/builder.rs:73-81`: delete the `"adapter.execution-agent" => Severity::Suggestion` arm + its doc.
4. `crates/standards/src/lint/eval/authoring_predicate.rs`: delete the list entry (`:28`) and the `| "adapter.execution-agent"` alternate (`:129`).
5. `crates/standards/src/framework/check/adapter/tests.rs`: delete `execution_agent_emits_suggestion` and `execution_tool_emits_nothing`.
6. `specify`: delete `adapters/shared/rules/core/CORE-051-adapter-execution-agent.md`.

**Quality delta.** −~108 LOC, −1 check/predicate, −1 DTO, −2 branches, −8 perpetual SUGGESTION findings, −2 tests, −1 duplicated dir-walk (the deleted body re-implements `check_missing_manifests`'s `read_dir`/`is_dir`/`under_symlink` skeleton).
**Net LOC.** ~108 → 0.
**Done when.** `make lint 2>&1 | rg -c 'adapter.execution-agent'` → `0`; `cargo make test` green.
**Architectural impact.** Removes a cross-repo predicate whose declarative migration can never reach hint parity (no native hint expresses "is agent-run *and* should not be"); the framework-check surface keeps only discriminating rules.
**Rule?** No.
**Counter-argument.** "It is informational by design (`severity: suggestion`)." Loses: an always-on constant carries zero signal; ripgrep/clippy ship no lint that fires on every correct input.
**Depends on.** none.

---

### F3 — Delete orphaned merge-runbook reference

**Evidence (current state).** Recursive orphan scan over `plugins/**` + `adapters/**/references/**` found exactly one genuinely unreferenced doc:

```
rg -l 'merge-runbook' . | grep -v -e merge-runbook.md -e REVIEW.md   →  (empty)
wc -l plugins/spec/references/merge-runbook.md                        →  78
```

The merge skill (`plugins/spec/skills/merge/SKILL.md`) links only `plan-lock.md` and `guardrails.md`, never this runbook — contrast `finalize`, which links its own `references/runbook.md`. (The `code-typescript` `examples/*.md` are *not* orphans: `adapters/sources/code-typescript/briefs/extract.md:29` directory-links them; `specialist-usage`, `phase-outcome-contract`, `standards-layer-snippet`, etc. carry 3–51 external refs.)

**Action.** `rm plugins/spec/references/merge-runbook.md`.
**Quality delta.** −78 LOC.
**Net LOC.** 78 → 0.
**Done when.** `ls plugins/spec/references/merge-runbook.md` → "No such file"; `make lint` still exits 0.
**Rule?** No.
**Counter-argument.** "It documents the merge runbook." Loses: nothing reaches it; if needed, content belongs inline in the already-standalone merge skill.
**Depends on.** none.

---

## One-touch tidies

### T1 — Narrow `from_evidence_yaml` to `pub(crate)`

**Evidence.** `rg -n 'from_evidence_yaml' crates/ src/` → defined at `crates/workflow/src/slice/synthesis/wire.rs:172`, called only by `from_evidence_file` (same file) and tests; production (`src/runtime/commands/slice/synthesize.rs`) uses `from_evidence_file`. No external `pub` consumer.
**Action.** `pub fn from_evidence_yaml` → `pub(crate) fn from_evidence_yaml` (`wire.rs:172`).
**Quality delta.** −1 crate-boundary edge (public API), 0 LOC.
**Net LOC.** same.
**Done when.** `rg -c 'pub fn from_evidence_yaml' crates/` → `0`.
**Rule?** No.
**Counter-argument.** "Harmless helper." Loses mildly: an internal step with no external caller; narrowing prevents accidental coupling.
**Depends on.** none.

### T2 — Align drop skill argument-hint with body

**Evidence.** `plugins/spec/skills/drop/SKILL.md:4` declares `argument-hint: "[slice-name]"`, but the body documents a non-interactive `reason` mode and forwards `--reason` (`:15`, `:56`). Hint↔body drift (not predicate-caught — `make lint` does not flag it — hence a tidy).
**Action.** Set `argument-hint: "[slice-name] [reason]"` (one-line edit).
**Quality delta.** −1 doc-drift; ΔLOC 0 (one line).
**Net LOC.** same.
**Done when.** `rg -n 'argument-hint' plugins/spec/skills/drop/SKILL.md` shows `[reason]`.
**Rule?** No.
**Counter-argument.** "argument-hint is optional." Loses: the body actively advertises the `reason` positional; an incomplete hint misleads non-interactive callers.
**Depends on.** none.

---

## Dropped after verification (recorded so they are not re-litigated)

- **`CORE-016` red in `docs/contributing/checks.md`** — was a real CI-predicate defect at the start of the pass; **fixed upstream** at HEAD `1b005aa4` (lines 46/323 reworded to drop the design-history citations). `make lint` now exits 0. No action needed.
- **`ToolSource` `From`/`TryFrom` impls** (`crates/tool/src/manifest.rs`) — *not* dead: consumed by `#[serde(into = "String", try_from = "String")]` on the type.
- **`migrate`/`upgrade` `emit` wrappers** — 3 and 2 call sites; inlining the `stdout().lock()` + `write_text` ceremony would *increase* LOC.
- **`code-typescript` `examples/*.md`** (663 LOC) — directory-linked corpus, named by `briefs/extract.md`.
- **`validate_*_json` family** (`crates/workflow/src/schema.rs`) — already thin wrappers over shared `validate_parsed_json` / `validate_with_ref_validator`; no DRY left.
- **Operator panic surface** — zero; the 361 `unwrap`/`expect` are all in `#[cfg(test)]` modules.
- **`split_frontmatter` duplication** (`crates/standards/.../parse.rs` vs `crates/model/.../decision.rs`) — fix needs a new crate dependency or a moved module; `Cargo.toml` frozen for the pass.
- **`into_diagnostic` lift** (`provenance.rs` vs `decision.rs`) — the two impls differ (line-number handling, distinct `Artifact`); a shared helper would not be strictly smaller.

---

## Post-mortem

One line per applied finding: actual ΔLOC vs predicted, did the "done when" assertion flip cleanly, did anything regress.

- **F2:** actual **−133 LOC** combined (specify-cli −110, specify −23) vs predicted −108; done-when flipped cleanly (`make lint 2>&1 | rg -c 'adapter.execution-agent'` → 0); no regression — `cargo make lint` pass, `cargo make test` 1692/1692 pass (1 skip), `make lint` exit 0. Extra: removed orphaned `use serde::Deserialize;` import and 2 dangling doc refs (`specify-cli/DECISIONS.md`, `specify/docs/contributing/checks.md`).
- **F3:** actual **−78 LOC** vs predicted −78 (exact); done-when flipped cleanly (file gone; `rg -l merge-runbook` → only `REVIEW.md`); no regression — `make lint` exit 0. Orphan status confirmed before deletion (no skill/brief/doc/AGENTS reference).
- **T1:** actual **0 LOC** (1 ins / 1 del) vs predicted 0; done-when flipped cleanly (`rg -c 'pub fn from_evidence_yaml' crates/` → 0); no regression — `cargo make lint` pass, `cargo make test` 1692/1692 pass. All call sites confirmed in-crate/tests before narrowing.
- **T2:** actual **0 LOC** (in-place one-line edit) vs predicted 0; done-when flipped cleanly (`argument-hint: "[slice-name] [reason]"`); no regression — `make lint` exit 0. Confirmed `[slice-name] [reason]` are two valid optional positionals per skill-authoring grammar.
