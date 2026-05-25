# Code & Skill Review — subtraction pass

1. **Top three findings by tier**: **F1** purge stale `plan validate` wire docs (verified wire-contract drift — retired/missing codes still documented); **F2** collapse plan schema validation + single read in `Plan::load` (subtraction); **F3** trim `/spec:execute` skill sections that repeat the Critical Path (subtraction).
2. **Total ΔLOC if all land**: about **−70 LOC** net across `specify-cli` Rust/docs and shipped phase skills.
3. **Primary non-LOC axes moved**: `−2` wire-contract doc defects, `−1` redundant disk read on plan load, `−1` duplicated validation filter branch, `−2` skill instruction surfaces repeating the same gate/lock rules.
4. **Top verified defects closed**: **F1** (wire-contract/doc drift for `unreachable-entry`, `project-missing-multi-repo`, and wrong example codes in the validate-output schema). **None** beyond F1 qualify under strict CI/predicate/panic definitions — CI is green on both repos. Defect-only net ΔLOC from F1: **−18** (pure deletion).
5. **Most likely to break in remediation**: **F2** — `validate_plan_yaml` must preserve the existing `plan-schema` rule id, JSON-pointer detail shape, and exit code `2`; a sloppy merge that routes YAML parse failures through `Error::YamlDe` instead of `Error::Validation` would regress the F1-era load contract.

## Reconnaissance Numbers

```text
tokei (specify + specify-cli combined)
Total: 1161 files, 161385 lines, 82242 code, 49874 comments, 29269 blanks
Rust: 300 files, 56082 lines, 48959 code

cargo tree --duplicates (specify-cli)
duplicate package groups at top level: 94

rg -c '^#\[test\]' crates/ src/ tests/ (specify-cli)
test_attr_total: 511

rg --files -g '**/mod.rs' (specify-cli)
3 files: tests/common/mod.rs, wasi-tools/vectis/tests/engine_support/mod.rs, crates/domain/tests/common/mod.rs

wc -l docs/standards/*.md AGENTS.md (both repos)
1496 total
  specify: 716 lines
  specify-cli: 780 lines

files > 500 lines under crates/ and src/ (specify-cli)
1048 crates/domain/tests/workspace.rs
 947 crates/domain/tests/finalize.rs
 922 crates/domain/tests/registry.rs
 890 crates/domain/src/discovery/document.rs
 728 crates/domain/src/adapter/core.rs
 688 crates/domain/src/change/plan/core/model.rs
 667 crates/domain/src/slice/fusion.rs
 611 crates/domain/src/journal.rs
 574 crates/domain/src/spec/provenance.rs
 520 crates/tool/src/validate.rs
 514 src/commands/plan/lifecycle.rs
 509 crates/domain/src/adapter/cache/io.rs

make check (specify)
All checks passed.

cargo make check (specify-cli)
Build Done in 181.39 seconds.

rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' crates/ src/ (specify-cli)
unwrap_expect_total: 687 (513 excluding inline #[cfg(test)] modules in the same files)

rg -c 'panic!|unreachable!' --glob '!**/tests/**' crates/ src/ (specify-cli)
panic_unreachable_total: 51

production-only unwrap/expect outside test modules: 1 (crates/tool/src/hash.rs write! to String — infallible)
```

## Structural Findings

### F1 — Purge Stale Validate Wire Docs

**Evidence**:

`plan validate` documents codes the binary no longer emits.

Retired diagnostic — commented in tests, absent from `DiagnosticPayload`:

```1401:1405:/Users/andrewweston/github.com/augentic/specify-cli/tests/plan_orchestrate.rs
// `plan validate` carries the three surviving health diagnostics
// (`cycle-in-depends-on`, `orphan-source-key`,
// `stale-workspace-clone`) alongside its base shape rules. The
// `unreachable-entry` diagnostic retired in RFC-25 alongside the
// per-entry `failed`/`skipped` states it relied on.
```

`DiagnosticPayload` has three variants only (`Cycle`, `OrphanSource`, `StaleClone`) — no `unreachable-entry`:

```68:97:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/change/plan/doctor.rs
pub enum DiagnosticPayload {
    Cycle { cycle: Vec<String> },
    OrphanSource { key: String },
    StaleClone { project: String, reason: StaleReason, ... },
}
```

Removed validate code still listed in the wire README:

```41:41:/Users/andrewweston/github.com/augentic/specify-cli/schemas/plan-validate-output/README.md
- `project-missing-multi-repo` (error): when the registry has multiple projects, a slice is missing the required `project` field.
```

Production grep — code gone, doc remains:

```text
$ rg -n 'project-missing-multi-repo|unreachable-entry' crates/ src/ schemas/plan-validate-output/
schemas/plan-validate-output/README.md:41:...project-missing-multi-repo...
schemas/plan-validate-output/README.md:50:...unreachable-entry...
schemas/plan-validate-output/schema.json:30:...unreachable-entry...
schemas/plan-validate-output/schema.json:53:...dependency-cycle...missing-change-dir-for-in-progress...unreachable-entry...
src/commands/plan/cli.rs:69:...unreachable-entry...
```

Live codes use `cycle-in-depends-on` and `missing-slice-dir-for-in-progress`, not the schema.json examples `dependency-cycle` / `missing-change-dir-for-in-progress`:

```135:135:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/change/plan/core/validate.rs
            code: "multiple-in-progress",
```

```277:277:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/change/plan/core/validate.rs
                    code: "missing-slice-dir-for-in-progress",
```

**Action**:

1. Delete the `project-missing-multi-repo` and `unreachable-entry` bullets from `schemas/plan-validate-output/README.md`; fix the producer example on line 21 to drop `unreachable-entry` from the `data.kind` union.
2. In `schemas/plan-validate-output/schema.json`, replace stale example codes in the three `description` strings: drop `unreachable-entry`; rename `dependency-cycle` → `cycle-in-depends-on`; rename `missing-change-dir-for-in-progress` → `missing-slice-dir-for-in-progress`; change “four supplementary doctor checks” → “three”.
3. In `src/commands/plan/cli.rs` `Validate` doc comment, list the three doctor diagnostics only (`cycle-in-depends-on`, `orphan-source-key`, `stale-workspace-clone`); drop `unreachable-entry`.

**Quality delta**: `−2 wire-contract doc defects, −18 LOC, −4 stale code names in operator-facing schema text`.

**Net LOC**: `schemas/plan-validate-output/* + cli.rs 152 → ~134`.

**Done when**: `rg -n 'project-missing-multi-repo|unreachable-entry|dependency-cycle|missing-change-dir-for-in-progress' schemas/plan-validate-output/ src/commands/plan/cli.rs` returns no matches.

**Rule?**: no — one-off doc drift after F3’s duplicate-gate deletion and RFC-25 retirement.

**Counter-argument**: stale docs are harmless if nobody pattern-matches; it loses because skills and JSON-schema consumers treat `schemas/plan-validate-output/` as canonical wire contract.

**Depends on**: none.

### F2 — Collapse Plan Schema Validation

**Evidence**:

`Plan::load` reads `plan.yaml` twice — once for content, again inside `validate_plan_file`:

```59:68:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/change/plan/core/io.rs
    pub fn load(path: &Path) -> Result<Self, Error> {
        ...
        let content = std::fs::read_to_string(path)?;
        validate_plan_file(path)?;
        let plan: Self = serde_saphyr::from_str(&content)?;
```

`validate_plan_file` re-reads the path:

```70:87:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/schema.rs
pub fn validate_plan_file(path: &Path) -> Result<()> {
    let instance = read_yaml_as_json(path).map_err(|err| { ... })?;
    let results = validate_value(...).into_iter().filter(|summary| summary.status == ValidationStatus::Fail).collect::<Vec<_>>();
    if results.is_empty() { Ok(()) } else { Err(Error::Validation { results }) }
}
```

The same filter/collect tail is duplicated in `validate_serialisable`:

```216:222:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/schema.rs
    for summary in validate_value(&instance, schema_source, rule_id, rule) {
        if summary.status == ValidationStatus::Fail {
            results.push(summary);
        }
    }
    if results.is_empty() { Ok(()) } else { Err(Error::Validation { results }) }
```

**Action**:

1. In `crates/domain/src/schema.rs`, add `fn validation_failures(...)` + `fn err_from_failures(...)`; route both `validate_serialisable` and a new `pub fn validate_plan_yaml(content: &str) -> Result<()>` through them.
2. Make `validate_plan_file` a thin `read_to_string` → `validate_plan_yaml` wrapper (keep for external callers/tests).
3. In `Plan::load`, call `validate_plan_yaml(&content)?` instead of `validate_plan_file(path)?`.

**Quality delta**: `−12 LOC, −1 redundant disk read, −1 duplicated filter branch, −1 call-site ceremony`.

**Net LOC**: `schema.rs + io.rs 356 → ~344`.

**Done when**: `rg -n 'validate_plan_file\(path\)' crates/domain/src/change/plan/core/io.rs` returns no matches; `Plan::load` contains exactly one `read_to_string` for the plan body; `cargo make check` passes.

**Rule?**: no.

**Counter-argument**: the double read is cheap for small plans; it loses because the duplication already bit once (schema helper landed without threading the loaded buffer) and the shared tail removes a second copy of the same four-line filter.

**Depends on**: none.

### F3 — Trim Execute Skill Duplication

**Evidence**:

`/spec:execute` states the Gate-1 refusal twice — Critical Path step 1 and the entire Refusal gate section:

```12:12:/Users/andrewweston/github.com/augentic/specify/plugins/spec/skills/execute/SKILL.md
1. Verify `plan.lifecycle == reviewed` via `specify plan next`; refuse with the literal `specify plan transition <name> reviewed` hint when the plan is still `pending`.
```

```19:26:/Users/andrewweston/github.com/augentic/specify/plugins/spec/skills/execute/SKILL.md
## Refusal gate
...
- `error` with discriminant `plan-not-reviewed` — print `specify plan transition <name> reviewed` verbatim and exit non-zero.
```

Plan lock prose repeats Critical Path step 2 and [`references/plan-lock.md`](plugins/spec/skills/execute/references/plan-lock.md):

```13:13:/Users/andrewweston/github.com/augentic/specify/plugins/spec/skills/execute/SKILL.md
2. Acquire the exclusive lock on `.specify/plan.lock` ... using the `flock`-based shell snippet in [`references/plan-lock.md`](references/plan-lock.md)
```

```28:32:/Users/andrewweston/github.com/augentic/specify/plugins/spec/skills/execute/SKILL.md
## Plan lock
The lock is an exclusive non-blocking advisory file lock on `.specify/plan.lock` ...
The full shell snippet ... lives in [`references/plan-lock.md`](references/plan-lock.md).
```

Guardrails repeat intro/critical-path constraints already stated in lines 8 and 15–16:

```8:8:/Users/andrewweston/github.com/augentic/specify/plugins/spec/skills/execute/SKILL.md
No automation flags exist — no `--continue`, no `--one`, ...
```

```56:60:/Users/andrewweston/github.com/augentic/specify/plugins/spec/skills/execute/SKILL.md
- **No automation flags.** `--continue`, `--one`, ...
- **Never write `reviewed`.** ...
- **Stop on the first failure.** ...
```

Current line count: `wc -l plugins/spec/skills/execute/SKILL.md` → **62**.

**Action**:

1. Delete the entire `## Refusal gate` section; fold the `drained` early-exit case into Critical Path step 5 (one bullet: when `plan next` returns drained before lock acquisition, print the finalize hint and exit).
2. Replace `## Plan lock` body with two sentences: reuse the snippet from `references/plan-lock.md`; on `plan-lock-busy`, exit with holder pid. Delete the macOS fallback prose (it already lives in the reference).
3. Delete guardrail bullets **No automation flags**, **Never write `reviewed`**, and **Stop on the first failure**; keep lock/finalize/CLI-writer bullets that add constraints not already in the Critical Path.

**Quality delta**: `−22 LOC, −2 skill instruction surfaces, −3 duplicated branches in agent routing`.

**Net LOC**: `62 → ~40`.

**Done when**: `wc -l plugins/spec/skills/execute/SKILL.md` is `≤ 42`; `rg -n '^## Refusal gate' plugins/spec/skills/execute/SKILL.md` returns no match; `make check` passes.

**Rule?**: no.

**Counter-argument**: repetition helps models on long skills; it loses here because the skill is only 62 lines and the duplicate sections restate the same exit branches verbatim.

**Depends on**: none.

### F4 — Trim Plan Skill Guardrails

**Evidence**:

Gate-1 “never write `reviewed`” appears three times in a 72-line skill:

```9:9:/Users/andrewweston/github.com/augentic/specify/plugins/spec/skills/plan/SKILL.md
... exits at `pending`. The operator stamps Gate 1 ... — the skill never writes `reviewed` itself.
```

```57:57:/Users/andrewweston/github.com/augentic/specify/plugins/spec/skills/plan/SKILL.md
`/spec:plan` never auto-stamps `reviewed`. Re-running ...
```

```63:63:/Users/andrewweston/github.com/augentic/specify/plugins/spec/skills/plan/SKILL.md
- **Never auto-stamp `reviewed`.** The closing hint is the only place ...
```

**Action**:

1. Delete line 57 (closing-hint paragraph duplicate).
2. Delete the **Never auto-stamp `reviewed`** guardrail bullet; keep **Single-writer**, workspace-mode, verb, and sandbox bullets.

**Quality delta**: `−8 LOC, −1 duplicated skill instruction surface`.

**Net LOC**: `72 → ~64`.

**Done when**: `rg -n 'never auto-stamp|never writes `reviewed`' plugins/spec/skills/plan/SKILL.md | wc -l` prints `1`; `make check` passes.

**Rule?**: no.

**Counter-argument**: Gate 1 is load-bearing; it loses because the overview and closing-hint sections already state the rule once each — a third copy adds no new constraint.

**Depends on**: none.

## One-Touch Tidies

### T1 — Delete Fusion Schema Wrapper

**Evidence**:

`fusion_schema_source()` is a pass-through const fn over a private `include_str!`:

```34:37:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/schema.rs
pub const fn fusion_schema_source() -> &'static str {
    FUSION_JSON_SCHEMA
}
```

Single call site in `fusion.rs` line 177.

**Action**:

1. Delete `fusion_schema_source`.
2. In `fusion.rs`, call `validate_serialisable(..., crate::schema::FUSION_JSON_SCHEMA, ...)` — expose `FUSION_JSON_SCHEMA` as `pub(crate) const FUSION_JSON_SCHEMA` (rename from private const) or inline one `include_str!` at the call site. Prefer `pub(crate) const` to avoid a second `include_str!`.

**Quality delta**: `−5 LOC, −1 public function, −1 module edge`.

**Net LOC**: `schema.rs + fusion.rs 384 → ~379`.

**Done when**: `rg -n 'fusion_schema_source' crates/domain/src` returns no matches.

**Rule?**: no.

**Counter-argument**: the wrapper documents intent; it loses because the call site already cites the schema path in module docs.

**Depends on**: none.

### T2 — Delete Stale Rollout Comments

**Evidence**:

Comments describe work that already landed:

```28:33:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/schema.rs
/// Exposed as a `&'static str` so domain modules can validate
/// in-memory `FusionIndex` values (Phase 1) without re-reading the
/// schema from disk. The fusion validator wiring lands in Change 1.1
/// alongside the new `slice/fusion.rs` module.
```

```13:17:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/slice/fusion.rs
//! Change 2.6 wires up the YAML read and validation envelope
//! ([`FusionIndex::load`]) and drift detection
//! ([`FusionIndex::detect_drift`]) consumed by `specify slice
//! validate`. Agent-side authoring of `fusion.yaml` itself lands in
//! Change 3.2; the CLI half owns validation and inspection only.
```

**Action**: delete both stale rollout blocks; keep the workflow §D4 audit-only sentence in `fusion.rs` module docs.

**Quality delta**: `−9 LOC, −2 misleading comment blocks`.

**Net LOC**: `schema.rs + fusion.rs 384 → ~375` ( stacks with T1).

**Done when**: `rg -n 'Change [0-9]|Phase 1' crates/domain/src/schema.rs crates/domain/src/slice/fusion.rs` returns no matches.

**Rule?**: no — comments are actively wrong, not merely verbose.

**Counter-argument**: historical context aids archaeology; it loses because `DECISIONS.md` and `fusion.rs` public API docs already capture the shipped behaviour.

**Depends on**: none.

### T3 — Deduplicate Plan Load Test Read

**Evidence**:

After F2 lands, `io.rs` tests that write rogue fields still call `Plan::load`; no new test needed for the single-read path — but `load_rejects_unknown_top_level_field` (line ~178) already asserts `/rogue` in schema detail. Keep that test; delete nothing unless F2 introduces a separate `validate_plan_yaml` test duplicating it.

**Action**: when implementing F2, extend the existing rogue-field test to assert one `read_to_string` path (optional: spy via temp file mtime only if a duplicate read reappears — otherwise no test change).

**Quality delta**: `0 LOC` — guardrail for F2 implementer only.

**Net LOC**: unchanged.

**Done when**: F2 lands without adding a second rogue-field test.

**Rule?**: no.

**Counter-argument**: n/a — tidy is a constraint on F2, not standalone work.

**Depends on**: F2.

## Post-mortem

<!-- One line per applied finding: actual ΔLOC vs predicted, did the "done when" assertion flip cleanly, did anything regress. -->
- F1 (purge stale validate wire docs): actual ΔLOC -2 vs predicted -18; done when clean; regressions none.
- F2 (collapse plan schema validation): actual ΔLOC +19 vs predicted -12; done when clean; regressions none.
- F3 (trim execute skill duplication): actual ΔLOC -20 vs predicted -22; done when clean; regressions none.
- F4 (trim plan skill guardrails): actual ΔLOC -3 vs predicted -8; done when clean; regressions none.
- T1 (delete fusion schema wrapper): actual ΔLOC -9 vs predicted -5; done when clean; regressions none.
- T2 (delete stale rollout comments): actual ΔLOC -6 vs predicted -9; done when clean; regressions none.
