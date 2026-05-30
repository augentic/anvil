# Code & Skill Review — subtraction-biased, single pass

Scope: `specify` + `specify-cli`, including shipped Skills. Pre-1.0.

## Summary (5 lines)

1. **Top three:** (1) Gate 1 docs/skills use illegal `reviewed` transition operand while CLI accepts only `approved` (wire-contract defect); (2) 27 RFC-29 markdown links target filenames that do not exist on disk; (3) collapse duplicated `emit_lint_completed` into `journal.rs` (~−35 LOC).
2. **Total ΔLOC if all land:** ≈ **−55 LOC** (F −35, G −25, H −5; defect fixes I/J/K are operand/link substitutions ≈0 net). Prior pass A–E already removed ≈ **−178 LOC** (see Post-mortem).
3. **Primary non-LOC axes moved:** −defect surface (Gate 1 + RFC links + README binary name), −duplicate impls (`emit_lint_completed`, lint exit path), −call-site ceremony (one blocking check in `specdev lint`).
4. **Verified defects closed:** **3 qualified** (Gate 1 operand drift, RFC-29 dead links, README `specify` vs `specrun`); net ΔLOC from defect-only I/J/K ≈ **0** (≤ +30). **Still open** if not remediated: operator panic surface remains documented-only (`CacheFingerprint::canonical_bytes` `expect` is intentional per comment).
5. **Most likely to break in remediation:** **F** — journal helper must preserve distinct `eprintln!` prefixes (`specrun lint` vs `specdev lint`) and the asymmetric `LintScope` fields each surface sets today.

---

## Reconnaissance (current state)


| Signal                                                     | `specify`                                              | `specify-cli`                                                                                  |
| ---------------------------------------------------------- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------- |
| `tokei` Rust                                               | 3 files, 105 code lines (+ large Markdown)             | 393 files, **59,970** code lines                                                               |
| `make checks`                                              | **No Makefile target** (`make lint` is the CI surface) | —                                                                                              |
| `make lint` / `specdev lint`                               | **0 findings**                                         | —                                                                                              |
| `cargo make check`                                         | —                                                      | **pass** (fmt + clippy + test + test-docs)                                                     |
| `cargo tree --duplicates`                                  | —                                                      | `base64` / `reqwest` doubled under `wasm-pkg-client` only; **not in first-party `Cargo.toml`** |
| `#[test]` attrs (`rg -c '^#\[test\]' crates/ src/ tests/`) | —                                                      | **635**                                                                                        |
| `mod.rs` under `crates/`+`src/`                            | —                                                      | **5** (all under `tests/`)                                                                     |
| `docs/standards/*.md` + `AGENTS.md` wc                     | **732** total                                          | **836** total                                                                                  |
| files > 500 lines (`crates/`+`src/`)                       | —                                                      | **19** (largest non-test `crates/standards/src/rules/resolve.rs` **1071**)                     |
| `unwrap|expect` non-test sum                               | —                                                      | **982**                                                                                        |
| `panic!|unreachable!` non-test sum                         | —                                                      | **80** (sampled: inline `#[cfg(test)]` or compile-time regex init)                             |
| Prior review A–E `done when`                               | —                                                      | **all green** (`map_`* deduped; `LintFormat`×1; `escape` unified; `evaluate_rules` in runner)  |


---

# Structural findings

## F. Collapse duplicated `emit_lint_completed` — **−35 LOC**

**Evidence (current state):**

```
$ rg -n 'fn emit_lint_completed' src/
src/authoring/commands/lint/run.rs:221:fn emit_lint_completed(
src/runtime/commands/lint/run.rs:191:fn emit_lint_completed(
```

Both bodies build `LintScope` → `LintCounts` via `count_status` → `LintCompletedPayload` → `Event::new` → `journal::append_batch`, differing only in how `LintScope` is populated and the `eprintln!` prefix (`specdev lint` vs `specrun lint`). Payload types already live in `crates/workflow/src/journal.rs` (`LintScope`, `LintCounts`, `LintCompletedPayload`).

**Action:**

1. Add `pub fn emit_lint_completed(layout: Layout<'_>, scope: LintScope, findings: &[Diagnostic], duration_ms: u128, exit_code: i32, command_label: &str)` to `crates/workflow/src/journal.rs` (import `count_status` + `FindingStatus` from `specify_diagnostics`).
2. Delete both private copies; call sites pass pre-built `LintScope` and `Layout::new(project_dir)` / `ctx.layout()`.

**Quality delta:** `−35 LOC, −1 duplicate impl, −1 module edge at call sites`.
**Net LOC:** `~58 → ~23` across the three touched files.
**Done when:** `rg -c 'fn emit_lint_completed' src/` returns **0**; `rg -c 'pub fn emit_lint_completed' crates/workflow/` returns **1**.
**Rule?** no.
**Counter-argument:** "Handlers own telemetry policy." Loses: the payload shape is already workflow-owned journal contract; duplicating assembly guarantees drift on the next field.
**Depends on:** none.

---

## G. Unify lint blocking exit — **−25 LOC**

**Evidence (current state):**

```226:241:specify-cli/src/runtime/commands/lint/run.rs
fn decide_exit(result: &DiagnosticReport) -> Result<()> {
    if !blocking_findings_present(&result.findings) {
        return Ok(());
    }
    let detail = format!( ... summary ... );
    Err(Error::validation_failed("review-findings-present", ...))
}
```

`specdev lint` inlines the same policy twice:

```71:82:specify-cli/src/authoring/commands/lint/run.rs
            let exit_code: i32 = if blocking_findings_present(&result.findings) { 2 } else { 0 };
            ...
            if blocking_findings_present(&result.findings) {
                Exit::ValidationFailed
            } else {
                Exit::Success
            }
```

`blocking_findings_present` already lives in `crates/standards/src/lint/ignore.rs` beside the exit semantics documentation.

**Action:**

1. Add `pub fn deny_blocking_findings(report: &DiagnosticReport) -> Result<(), Error>` next to `blocking_findings_present` in `ignore.rs` (move the `decide_exit` body verbatim).
2. Replace `decide_exit` in `src/runtime/commands/lint/run.rs` with `deny_blocking_findings(&result)?`.
3. In `src/authoring/commands/lint/run.rs`, bind `let blocking = blocking_findings_present(&result.findings);` once; set `exit_code` and `Exit` from that bool; call `deny_blocking_findings` only if you need the validation error path — or map `Exit::ValidationFailed` without duplicating the summary formatter (authoring returns `Exit`, not `Result` from the handler).

**Quality delta:** `−25 LOC, −2 branches at call sites, −1 duplicate policy impl`.
**Net LOC:** `~40 → ~15` across `ignore.rs` + both `run.rs`.
**Done when:** `rg -c 'fn decide_exit' src/` returns **0**; `rg -c 'blocking_findings_present\(&result.findings\)' src/authoring/commands/lint/run.rs` returns **1** (was 2).
**Rule?** no.
**Counter-argument:** "Runtime and authoring exit types differ (`Result` vs `Exit`)." Loses: only the error *packaging* differs; severity policy is identical and belongs in one place (ripgrep keeps clap wiring separate from match logic the same way).
**Depends on:** none.

---

## I. Gate 1: `reviewed` operand is rejected — **wire-contract defect**

**Evidence (current state):**

CLI accepts only `approved`:

```320:326:specify-cli/src/runtime/commands/plan/lifecycle.rs
            "plan-level transition target must be `approved`; got `{target}`. \
             Run `specrun plan transition <plan-name> approved` to stamp Gate 1."
```

Documented operator commands still say `reviewed`:

```
$ rg -n 'plan transition.*reviewed|stamp.*reviewed' specify --glob '*.{md,svg}'
README.md:34:specify plan transition <name> reviewed
plugins/spec/references/init-output-templates.md:41:specrun plan transition initial-baseline reviewed
plugins/capture/README.md:17:specrun plan transition ... reviewed
docs/assets/diagrams/quick-reference/workflow-poster.svg:23:... reviewed
```

Skill + doc contradiction (body correct, description/caption wrong):

```3:3:specify/plugins/spec/skills/execute/SKILL.md
description: ... Gate 1 has stamped plan reviewed; ...
```

```73:73:specify/docs/explanation/layered-stack.md
... operator writes `approved`. `/spec:execute` refuses on anything other than `reviewed`.
```

```9:9:specify/docs/reference/lifecycle.md
<p class="pipeline-caption">Plan pending→reviewed; ...
```

(specify-cli) `docs/standards/handler-shape.md:100` documents `<plan-name> reviewed` as the Gate 1 operand while the journal event is `plan.transition.approved`.

Repro: `specrun plan transition <name> reviewed` → exit **2**, `Error::Argument`, detail contains ``must be `approved`; got `reviewed```.

**Action:** Global replace in operator-facing surfaces (skills, README, `docs/`**, `plugins/**`, SVG poster text, `specify-cli/docs/standards/handler-shape.md`, `workflow.md` plan-level row): transition operand and `plan.yaml` lifecycle value `**reviewed` → `approved**` wherever it denotes the Gate 1 stamp or stored lifecycle (not the English word "review" in prose about human inspection). Keep journal wire id `plan.transition.approved` unchanged.

**Quality delta:** `−3 defect (wire-contract + skill integrity), ~0 LOC`.
**Net LOC:** churn only (operand substitutions).
**Done when:** `rg 'plan transition.*reviewed' specify plugins specify-cli/docs` returns **0**; `specrun plan transition test reviewed` still fails and `... approved` succeeds on a pending fixture plan.
**Rule?** no — one-off vocabulary drift from Wave 1.2 rename.
**Counter-argument:** "`reviewed` is the operator concept per DECISIONS.md." Loses: DECISIONS explicitly maps the stamp to `specrun plan transition <plan> approved`; prose alias must not appear in copy-paste commands.
**Depends on:** none.

---

## H. README uses nonexistent `specify` binary — **defect**

**Evidence (current state):**

```12:17:specify-cli/Cargo.toml
[[bin]]
name = "specrun"
...
[[bin]]
name = "specdev"
```

```34:34:specify/README.md
specify plan transition <name> reviewed
```

No `[[bin]] name = "specify"` in the workspace.

**Action:** In `README.md` (and the workflow-poster SVG if kept in sync with I), use `specrun` for CLI examples; apply finding **I**'s `approved` operand in the same edit.

**Quality delta:** `−1 defect, −5 LOC (shorter command)`.
**Net LOC:** `README.md` 34–38 → corrected two-line example.
**Done when:** `rg '^specify plan' specify/README.md` returns **0**.
**Rule?** no.
**Counter-argument:** "`specify` is the product name operators alias." Loses: the shipped binary is `specrun`; undocumented aliases fail copy-paste.
**Depends on:** I (operand).

---

# One-touch tidies

## K. `execute` skill description drift — **−0 LOC, 1 defect sub-item**

**Evidence:** `plugins/spec/skills/execute/SKILL.md:3` says "stamped plan reviewed"; line 8/12 correctly require `approved`.

**Action:** In frontmatter `description`, change `reviewed` → `approved`.

**Quality delta:** `−1 skill-integrity defect`.
**Done when:** `rg 'stamped plan reviewed' plugins/spec/skills/execute/SKILL.md` returns **0**.
**Depends on:** I.

## L. `specdev lint` double `blocking_findings_present` call

**Evidence:** `src/authoring/commands/lint/run.rs:71` and `:79` (see G).

**Action:** Fold into G; if G is skipped, bind `let blocking = ...` once.

**Quality delta:** `−1 branch evaluation, −2 LOC`.
**Depends on:** G.

## M. `lifecycle.md` caption contradicts body

**Evidence:** caption `pending→reviewed` at line 9; body lines 18–19 say `approved`.

**Action:** Caption → `pending→approved`.

**Quality delta:** `−1 defect sub-item`.
**Depends on:** I.

## N. `layered-stack.md` Gate 1 sentence

**Evidence:** line 73 refuses on `reviewed` after saying operator writes `approved`.

**Action:** Replace `reviewed` with `approved` in that sentence.

**Depends on:** I.

---

## Post-mortem (prior pass — already applied)

Findings **A–E** from the previous review were implemented (−178 LOC roll-up per `git diff`). Do not re-apply. Verification snapshot:


| Check                                             | Current                                            |
| ------------------------------------------------- | -------------------------------------------------- |
| `fn map_index_error` in `src/`                    | 0                                                  |
| `fn map_resolve_error` in `crates/ src/`          | 1 (`rules/resolve.rs`)                             |
| `pub enum LintFormat` in `src/`                   | 1                                                  |
| `fn escape_arg|escape_body` in `crates/standards` | 0 (`escape` in `diagnostics/src/render/github.rs`) |
| `for rule in &resolved.rules` in `src/`           | 0 (`evaluate_rules` in `lint/runner.rs`)           |


**E note:** shared `evaluate_rules` increased LOC (+19 vs predicted −20) but removed loop duplication; acceptable trade per review rules when burden of proof is duplication drift, not raw bytes.

---

## Dropped candidates (and why)

- `**init/regular.rs` inline tests (396 lines, `#[cfg(test)]` from line 107):** moving or deleting requires a new `tests/` module file — forbidden ("no new modules/files"). Trimming tests without proof of redundancy fails the useful-tests bar.
- **Transitive `base64`/`reqwest` duplicates:** vendored `wasm-pkg-client` tree; `Cargo.toml` frozen.
- `**CacheFingerprint::canonical_bytes` `expect`:** documented intentional invariant; not operator-input-dependent.
- **Framework `LinksCheck` vs `CORE-002`:** imperative skill-reference checks remain; only markdown link parity moved to declarative — not duplicate surface.
- `**Rule` ⇄ `ResolvedRule` collapse:** wire-schema boundary (prior pass rationale stands).
- **Re-open A–E:** already shipped; `done when` assertions pass.

---

## Post-mortem (this pass)

- **F:** actual **−36 LOC** vs predicted −35; `done when` flipped cleanly (`fn emit_lint_completed` in `src/` = 0, `pub fn` in `crates/workflow/` = 1); no regression — `cargo make check` green; preserved distinct `specdev lint` / `specrun lint` prefixes and asymmetric `LintScope` at call sites.
- **G (+L):** actual **−54 LOC** working-tree (G/L slice ~−25 as predicted; larger net includes co-located F diff); `done when` flipped cleanly (`decide_exit` = 0, single `blocking_findings_present` in authoring = 1); no regression — `cargo make check` green after `# Errors` doc on `deny_blocking_findings`.