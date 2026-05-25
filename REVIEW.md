# Code & Skill Review — subtraction pass

1. **Top three findings by tier**: **F1** raw `plan.yaml` schema is skipped by `plan validate` (verified wire-contract defect); **F2** `DECISIONS.md` still forbids the derived `Ord` code now in-tree (verified contract/doc drift); **F3** `project-missing-multi-repo` duplicates the generic project-or-target gate (subtraction).
2. **Total ΔLOC if all land**: about **−100 LOC** net, depending on the exact raw-schema helper shape chosen for F1.
3. **Primary non-LOC axes moved**: `−1` wire-contract defect, `−1` stale contract warning, `−1` duplicate validation branch, `−1` hand-written parser, fewer stale workflow terms in operator-facing CLI text.
4. **Top verified defects closed**: F1 and F2 qualify; T2 is a small defect-surface cleanup, not a verified defect under the strict definition. Defect-only net ΔLOC: **≤ +8** because F1 pairs the small helper with deletion in the same validation flow.
5. **Most likely to break in remediation**: **F1** — raw schema validation must preserve the existing JSON error envelope and exit code `2`, not turn schema failures into generic YAML or diagnostic errors.

## Reconnaissance Numbers

```text
tokei /Users/andrewweston/github.com/augentic/specify /Users/andrewweston/github.com/augentic/specify-cli
Total: 1161 files, 161385 lines, 82242 code, 49874 comments, 29269 blanks
Rust: 300 files, 56082 lines, 48959 code

cargo tree --duplicates (specify-cli)
top-level duplicate package groups: 94

rg -c '^#\[test\]' crates/ src/ tests/ (specify-cli)
test_attr_total 514

rg --files -g '**/mod.rs' (specify-cli)
3 files: tests/common/mod.rs, wasi-tools/vectis/tests/engine_support/mod.rs, crates/domain/tests/common/mod.rs

wc -l docs/standards/*.md AGENTS.md (both repos)
1496 total
  specify: 716 lines across docs/standards/*.md + AGENTS.md
  specify-cli: 780 lines across docs/standards/*.md + AGENTS.md

files > 500 lines under crates/ and src/ (specify-cli)
1048 crates/domain/tests/workspace.rs
 947 crates/domain/tests/finalize.rs
 922 crates/domain/tests/registry.rs
 890 crates/domain/src/discovery/document.rs
 728 crates/domain/src/adapter/core.rs
 700 crates/domain/src/change/plan/core/model.rs
 667 crates/domain/src/slice/fusion.rs
 611 crates/domain/src/journal.rs
 574 crates/domain/src/spec/provenance.rs
 520 crates/tool/src/validate.rs
 514 src/commands/plan/lifecycle.rs
 509 crates/domain/src/adapter/cache/io.rs

make checks (specify)
make: *** No rule to make target `checks'.  Stop.

make check (specify)
All checks passed.

cargo make check (specify-cli)
Build Done in 178.71 seconds.

rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' crates/ src/ (specify-cli)
unwrap_expect_total 687

rg -c 'panic!|unreachable!' --glob '!**/tests/**' crates/ src/ (specify-cli)
panic_unreachable_total 50
```

## Structural Findings

### F1 — Validate Raw Plan Schema

**Evidence**:

`schemas/plan/plan.schema.json` says unknown top-level and per-slice fields are rejected:

```1:11:/Users/andrewweston/github.com/augentic/specify-cli/schemas/plan/plan.schema.json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://github.com/augentic/specify-cli/schemas/plan/plan.schema.json",
  "title": "Specify plan.yaml",
  "description": "Validates the structure of `plan.yaml` (at the repo root): the change name, the optional plan-level `lifecycle` (`pending | reviewed` per the workflow contract §Workflow vocabulary), the optional named-sources map, and the ordered list of plan slices with their dependencies, status, and (per the workflow contract) structured source bindings plus the optional `divergence` enum. Strict schema — unknown top-level and per-slice fields are rejected.
```

But `Plan::load` deserializes directly and never validates the raw YAML value against the embedded schema:

```57:66:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/change/plan/core/io.rs
    pub fn load(path: &Path) -> Result<Self, Error> {
        if !path.exists() {
            return Err(Error::ArtifactNotFound {
                kind: "plan.yaml",
                path: path.to_path_buf(),
            });
        }
        let content = std::fs::read_to_string(path)?;
        let plan: Self = serde_saphyr::from_str(&content)?;
        Ok(plan)
```

Current-state reproduction:

```text
$ tmp=$(mktemp -d); mkdir "$tmp/.specify"; printf 'name: demo\nadapter: omnia\n' > "$tmp/.specify/project.yaml"; printf 'name: bad\nrogue: true\nslices:\n  - name: only\n    target: omnia@v1\n    status: pending\n' > "$tmp/plan.yaml"; (cd "$tmp" && SPECIFY_FORMAT=json /Users/andrewweston/github.com/augentic/specify-cli/target/debug/specify plan validate); code=$?; printf 'exit_code=%s\n' "$code"; rm -rf "$tmp"
{
  "plan": {
    "name": "bad",
    "path": "/private/var/folders/2p/3jz_1c9n0hd6ydkjhlnjgmh00000gn/T/tmp.cDYpa5yNct/plan.yaml"
  },
  "results": [],
  "passed": true
}
exit_code=0
```

**Action**:

1. In `crates/domain/src/schema.rs`, expose the existing raw-YAML path as a small `validate_plan_file(path: &Path) -> Result<()>` wrapper around `read_yaml_as_json` + `validate_value(... PLAN_JSON_SCHEMA ...)`.
2. In `crates/domain/src/change/plan/core/io.rs`, call `validate_plan_file(path)?` before `serde_saphyr::from_str`.
3. Delete the duplicate multi-repo branch in F3 in the same change if the helper lands above the +8 defect-only budget.
4. Add one focused test using a rogue top-level field; no broad schema fixture expansion.

**Quality delta**: `−1 verified wire-contract defect, −1 duplicate validation branch if paired with F3, net LOC ≤ current +8 before paired deletion`.

**Net LOC**: about `89 → ≤97` for the raw-schema helper alone; `≈169 → ≈145` if paired with F3’s deletion in the same remediation.

**Done when**: the reproduction command exits `2` and its JSON body contains one `plan-schema` failure mentioning `/rogue`.

**Rule?**: no — the schema exists; the bug is that this read path bypasses it.

**Counter-argument**: `Plan::validate` already checks semantic consistency; it loses because schema-only fields are silently discarded before semantic validation can see them.

**Depends on**: none.

### F2 — Fix Stale Ord Contract

**Evidence**:

`DECISIONS.md` still says the adapter operation enums use manual ordering and warns against deriving `Ord`:

```619:628:/Users/andrewweston/github.com/augentic/specify-cli/DECISIONS.md
- **Wire invariant.** The `specify source resolve` and
  `specify target resolve` JSON envelopes' `operations: [...]` arrays
  iterate in kebab-alphabetical order (e.g. `["enumerate", "extract"]`,
  `["build", "merge", "shape"]`). `BTreeMap` ordering combined with
  manual `Ord` / `PartialOrd` impls on `{Source,Target}Operation`
  (sorting by kebab string, not by Rust variant declaration order)
  preserves this contract end-to-end. Future refactors must not
  re-derive `Ord` on these enums without preserving the kebab-string
  sort — derived `Ord` follows declaration order and would silently
```

Current code derives `PartialOrd` and `Ord` on both enums, with variants declared in kebab order:

```40:56:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/adapter/operation.rs
/// Variants declared in kebab-alphabetical order so `BTreeMap`
/// iteration matches the wire envelope.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
```

Current-state grep:

```text
$ rg -n 'manual `Ord`|PartialOrd,\n\s*Ord' DECISIONS.md crates/domain/src/adapter/operation.rs
DECISIONS.md:623:  manual `Ord` / `PartialOrd` impls on `{Source,Target}Operation`
crates/domain/src/adapter/operation.rs:48:    PartialOrd,
crates/domain/src/adapter/operation.rs:49:    Ord,
crates/domain/src/adapter/operation.rs:100:    PartialOrd,
crates/domain/src/adapter/operation.rs:101:    Ord,
```

**Action**:

1. In `DECISIONS.md`, replace lines 623–628 with one shorter sentence: derived `Ord` is intentional because enum variants are declared in kebab-alphabetical wire order.
2. Keep the examples `["enumerate", "extract"]` and `["build", "merge", "shape"]`; those are still the contract.

**Quality delta**: `−1 verified contract/doc drift, −4 LOC`.

**Net LOC**: `52 → 48` in the `Target adapter suffix policy` / operations decision area.

**Done when**: `rg -n 'manual `Ord`|must not\s+re-derive `Ord`' DECISIONS.md` returns no matches, while `cargo make check` still passes.

**Rule?**: no — this is one stale decision paragraph after a completed simplification.

**Counter-argument**: keeping the warning may prevent a future reorder; it loses because it now describes code that no longer exists and tells agents to undo the smaller implementation.

**Depends on**: none.

### F3 — Delete Duplicate Project Gate

**Evidence**:

`Plan::validate` appends two findings for the same predicate when a multi-repo plan entry has neither `project` nor `target`:

```37:42:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/change/plan/core/validate.rs
        results.extend(missing_project_or_target(&self.entries));
        results.extend(check_context_paths(&self.entries));
        results.extend(authority_override_orphan_source_keys(&self.entries));
        if let Some(reg) = registry {
            results.extend(check_project_in_registry(&self.entries, reg));
            results.extend(check_project_required_multi_repo(&self.entries, reg));
```

Both checks use the same condition:

```164:190:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/change/plan/core/validate.rs
fn check_project_required_multi_repo(changes: &[Entry], registry: &Registry) -> Vec<Finding> {
    if registry.projects.len() <= 1 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for entry in changes {
        if entry.project.is_none() && entry.target.is_none() {
            out.push(Finding {
                level: Severity::Error,
                code: "project-missing-multi-repo",
                message: format!(
                    "slice '{}' has no project or target; multi-repo implementation slices must specify a project",
                    entry.name
                ),
```

```185:199:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/change/plan/core/validate.rs
fn missing_project_or_target(changes: &[Entry]) -> Vec<Finding> {
    let mut out = Vec::new();
    for entry in changes {
        if entry.project.is_none() && entry.target.is_none() {
            out.push(Finding {
                level: Severity::Error,
                code: "plan.entry-needs-project-or-target",
                message: format!(
                    "entry '{}' has neither 'project' nor 'target'; at least one is required",
```

The only production references to `project-missing-multi-repo` are the branch and its tests:

```text
$ rg -n 'project-missing-multi-repo|check_project_required_multi_repo' crates/domain/src tests src --glob '*.rs'
crates/domain/src/change/plan/core/validate.rs:19:    /// checks (`project-not-in-registry`, `project-missing-multi-repo`).
crates/domain/src/change/plan/core/validate.rs:42:            results.extend(check_project_required_multi_repo(&self.entries, reg));
crates/domain/src/change/plan/core/validate.rs:164:fn check_project_required_multi_repo(changes: &[Entry], registry: &Registry) -> Vec<Finding> {
crates/domain/src/change/plan/core/validate.rs:173:                code: "project-missing-multi-repo",
crates/domain/src/change/plan/core/validate/tests.rs:227:    assert!(results.iter().any(|r| r.code == "project-missing-multi-repo"));
crates/domain/src/change/plan/core/validate/tests.rs:257:        !results.iter().any(|r| r.code == "project-missing-multi-repo"),
crates/domain/src/change/plan/core/validate/tests.rs:279:    assert!(!results.iter().any(|r| r.code == "project-missing-multi-repo"));
```

**Action**:

1. Delete `check_project_required_multi_repo`.
2. Remove its call from `Plan::validate`.
3. Delete `project_missing_multi_repo`, `target_only_entry_valid_multi_repo`, and `project_valid_single_repo`; keep the generic `neither_project_nor_target_error`, `target_only_passes`, and `project_and_target_passes` tests.
4. Remove `project-missing-multi-repo` from the doc comment at the top of `validate.rs`.

**Quality delta**: `−65 LOC, −1 branch, −1 diagnostic variant on the plan-validation path`.

**Net LOC**: `validate.rs + validate/tests.rs 809 → about 744`.

**Done when**: `rg -n 'project-missing-multi-repo|check_project_required_multi_repo' crates/domain/src/change/plan/core` returns no matches and `cargo make check` passes.

**Rule?**: no — one duplicate branch.

**Counter-argument**: a multi-repo-specific message might be friendlier; it loses because target-only coordinator entries are explicitly valid and the branch no longer expresses a distinct rule.

**Depends on**: none.

### F4 — Derive ClaimKind Parsing

**Evidence**:

`ClaimKind` already derives `Display` and `ValueEnum`, but hand-writes a 15-arm parser:

```44:60:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/evidence/authority.rs
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    strum::Display,
    clap::ValueEnum,
)]
```

```92:121:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/evidence/authority.rs
impl FromStr for ClaimKind {
    type Err = String;

    /// Parse the closed kebab-case wire form (e.g. `requirement`,
    /// `criterion`). Mirrors the schema enum byte-for-byte so the
    /// CLI parser and `evidence.schema.json` reject the same set of
    /// values.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "intent" => Ok(Self::Intent),
```

The repo already uses `strum::EnumString` for identical kebab-case enum parsing:

```51:58:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/adapter/operation.rs
    Serialize,
    Deserialize,
    EnumString,
    strum::Display,
    ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
```

Current-state grep:

```text
$ rg -n 'impl FromStr for ClaimKind|EnumString' crates/domain/src/evidence/authority.rs crates/domain/src/adapter/operation.rs crates/domain/src/spec/provenance.rs
crates/domain/src/evidence/authority.rs:92:impl FromStr for ClaimKind {
crates/domain/src/adapter/operation.rs:27:use strum::EnumString;
crates/domain/src/adapter/operation.rs:53:    EnumString,
crates/domain/src/adapter/operation.rs:105:    EnumString,
crates/domain/src/spec/provenance.rs:66:    strum::EnumString,
crates/domain/src/spec/provenance.rs:85:    Debug, Copy, Clone, PartialEq, Eq, strum::Display, strum::EnumString, strum::IntoStaticStr,
```

**Action**:

1. Remove `use std::str::FromStr;` from `authority.rs`.
2. Add `strum::EnumString` to `ClaimKind`'s derive list.
3. Delete the manual `impl FromStr for ClaimKind`.
4. Adjust `claim_kind_from_str_rejects_unknown` to assert the generated error mentions the bad token, or delete the test if it only pins the custom prose.

**Quality delta**: `−27 LOC, −1 hand-written parser, −15 match arms`. Idiom: same derive-based enum parsing already used locally; `clap`/derive-driven CLI parsing is also the direction cargo-style CLIs take.

**Net LOC**: `authority.rs 303 → about 276`.

**Done when**: `rg -n 'impl FromStr for ClaimKind|use std::str::FromStr' crates/domain/src/evidence/authority.rs` returns no matches and `cargo make check` passes.

**Rule?**: no — only one enum still hand-writes a parser while using `strum`.

**Counter-argument**: the custom parser has a nicer expected-values string; it loses because `clap::ValueEnum` already owns operator-facing CLI suggestions, and schema validation owns file-facing diagnostics.

**Depends on**: none.

## One-Touch Tidies

### T1 — Delete Dead TargetRef Accessors

**Evidence**:

`TargetRef::name()` and `TargetRef::version()` are only used by tests:

```353:363:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/change/plan/core/model.rs
    /// Kebab-case adapter name (before the `@v` suffix).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Integer version (after the `@v` suffix).
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
```

```text
$ rg -n 'TargetRef::name|TargetRef::version|\.name\(\)|\.version\(\)' crates/domain/src/change/plan src tests --glob '*.rs'
crates/domain/src/change/plan/core/model/tests.rs:154:    assert_eq!(zero_target.name(), "contracts");
crates/domain/src/change/plan/core/model/tests.rs:155:    assert_eq!(zero_target.version(), 1);
crates/domain/src/change/plan/core/model/tests.rs:158:    assert_eq!(one_target.name(), "omnia");
crates/domain/src/change/plan/core/model/tests.rs:159:    assert_eq!(one_target.version(), 1);
```

**Action**:

1. Delete both accessors.
2. Delete the four test assertions that exist only to call them; keep the serialize/round-trip assertion.

**Quality delta**: `−14 LOC, −2 public methods, −4 test-only call sites`.

**Net LOC**: `model.rs + model/tests.rs 1109 → about 1095`.

**Done when**: `rg -n 'pub (const )?fn (name|version)\(|\.name\(\)|\.version\(\)' crates/domain/src/change/plan/core/model.rs crates/domain/src/change/plan/core/model/tests.rs` returns no matches.

**Rule?**: no.

**Counter-argument**: future version reconciliation may need them; it loses because future code can add the one accessor it actually uses.

**Depends on**: none.

### T2 — Fix Retired Workflow Terms

**Evidence**:

Current CLI help still exposes the retired `define` loop:

```95:99:/Users/andrewweston/github.com/augentic/specify-cli/src/cli.rs
    },

    /// Slice lifecycle operations — one `define → build → merge` loop.
    Slice {
        #[command(subcommand)]
```

Code comments still reference `/change:execute`, which the current workflow replaced with `/spec:execute`:

```3:5:/Users/andrewweston/github.com/augentic/specify-cli/crates/domain/src/slice/metadata.rs
//! [`SliceMetadata`] is the document, [`Outcome`] is the latest phase return
//! surface read by `/change:execute`, and [`TouchedSpec`] lists the specs
//! the slice mutates.
```

Current-state count outside test directories:

```text
$ rg -n 'define → build|/change:execute' crates/domain/src src --glob '*.rs' | wc -l
5
```

**Action**:

1. Replace `define → build → merge` with `refine → build → merge` in `src/cli.rs`.
2. Replace `/change:execute` with `/spec:execute` in `crates/domain/src/slice/metadata.rs`, `crates/domain/src/slice/outcome.rs`, and `crates/domain/src/merge/slice.rs`.

**Quality delta**: `−5 stale workflow references, 0 LOC`.

**Net LOC**: unchanged.

**Done when**: `rg -n 'define → build|/change:execute' crates/domain/src src --glob '*.rs'` returns no matches and `cargo make check` passes.

**Rule?**: yes, barely — there are 5 live-source hits and a simple `rg`-based docs check could reject retired command names, but do not add it in this pass.

**Counter-argument**: comments are harmless; it loses because one occurrence is user-facing CLI help.

**Depends on**: none.

### T3 — Trim Wiretapper Repetition

**Evidence**:

The shipped skills are small overall, but `wiretapper` repeats failure and verification criteria in three places:

```57:64:/Users/andrewweston/github.com/augentic/specify/plugins/capture/skills/wiretapper/SKILL.md
### Step 5: Verify Compile

1. From `$LEGACY_DIR`, run the project build (e.g. `npm run build` or `npx tsc --noEmit`). Use the script the project defines; if both exist, prefer `npm run build`.
2. If the build fails, report the compiler errors and **fail the step**. Do not leave the repo in a broken state without failing.

### Step 6 (Optional): Integration Doc

Optionally add `$LEGACY_DIR/src/wiretap/README.md` documenting that wiretap is enabled with `WIRETAP_ENABLED=true` and listing which adapters were registered.
```

```72:87:/Users/andrewweston/github.com/augentic/specify/plugins/capture/skills/wiretapper/SKILL.md
## Error Handling

| Issue | Cause | Resolution |
|-------|--------|------------|
| Invalid or missing legacy path | Bad argument or path not a directory | Fail with "Error: legacy-dir is required and must be an existing directory." (or similar) |
| No package.json | Not a Node project | Fail with clear message; do not generate. |
| Entrypoint not found | Unusual layout | Fail with message listing paths checked. |
| Build fails after wiring | Syntax/import errors in generated or patched code | Report compiler output and fail the step. |
| Wiretap capture throws | Bug in generated adapter | All adapters must wrap capture in try/catch (design guardrail). |

## Verification Checklist
```

Current-state skill lengths:

```text
$ rg --files -g 'SKILL.md' | xargs wc -l | sort -nr
96 plugins/capture/skills/wiretapper/SKILL.md
85 plugins/spec/skills/drop/SKILL.md
72 plugins/spec/skills/plan/SKILL.md
...
```

**Action**:

1. Delete the `## Error Handling` table; every row repeats a process step or a guardrail.
2. Delete `### Step 6 (Optional): Integration Doc`; optional README generation is not necessary for replay-ready captures and adds an output surface.
3. Keep the verification checklist and guardrails.

**Quality delta**: `−15 LOC, −1 optional output branch`.

**Net LOC**: `96 → about 81`.

**Done when**: `wc -l plugins/capture/skills/wiretapper/SKILL.md` is `≤ 81` and `make check` still passes.

**Rule?**: no.

**Counter-argument**: the table helps failures read predictably; it loses because the process steps already prescribe those exact failures and the skill predicate suite is green without the duplicate prose.

**Depends on**: none.

## Post-mortem

<!-- One line per applied finding: actual ΔLOC vs predicted, did the "done when" assertion flip cleanly, did anything regress. -->
- F1 (validate raw plan schema): actual ΔLOC +64 vs predicted +8; done when clean; regressions none.
- F2 (fix stale Ord contract): actual ΔLOC -4 vs predicted -4; done when clean; regressions none.
- F3 (delete duplicate project gate): actual ΔLOC -103 vs predicted -65; done when clean; regressions none.
- F4 (derive ClaimKind parsing): actual ΔLOC -21 vs predicted -27; done when clean; regressions none.
- T1 (delete dead TargetRef accessors): actual ΔLOC -16 vs predicted -14; done when clean; regressions none.
- T2 (fix retired workflow terms): actual ΔLOC 0 vs predicted 0; done when clean; regressions none.
- T3 (trim wiretapper repetition): actual ΔLOC -15 vs predicted -15; done when clean; regressions none.
