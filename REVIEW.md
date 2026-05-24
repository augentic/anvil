# Code & Skill Review — specify + specify-cli

Top three findings by tier: **F1 Inline plan-add overrides through `emit_override_events`** (~−10 LOC, −1 duplicate event-build site), **F2 Collapse triplicate `Patch::from(Option<String>)` matches in `amend`** (~−9 LOC, −2 branches), **F3 Share `discovery-candidate-unknown` between add/remove_alias** (~−7 LOC, −1 duplicate diag).
Total ΔLOC if all land: **approximately −38 LOC**.
Primary non-LOC axes moved: fewer duplicate event-build / error-build sites, fewer branch clusters, lower call-site burden on the amend path.
Top verified defects closed: **none qualified** (0 open from this pass); defect-only net ΔLOC: **0** (portfolio cap unused).
Most likely to break in remediation: **F1** — the `add` event order must remain BTreeMap-stable (`(slice, kind, Set)`) so the prior happy-path test `tests/plan_orchestrate.rs` keeps asserting the same wire bytes.

## Reconnaissance

- `tokei`: **specify** **593** Markdown files / **52,608** lines; **specify-cli** **248** Rust files / **47,451** lines (workspace total **1,094** files / **151,505** lines).
- `cargo tree --duplicates` (`specify-cli`): non-empty — `base64 v0.21.7 / v0.22.1`, `reqwest v0.12.28 / v0.13.3`, multi-version `wasmtime` / `wasm-pkg-client` transitives. `Cargo.toml` frozen for this pass.
- `rg -c '^#\[test\]' crates/ src/ tests/` (`specify-cli`): summed **552** `#[test]` declarations (e.g. `tests/plan_orchestrate.rs` **70**, `crates/domain/tests/registry.rs` **50**, `crates/domain/tests/workspace.rs` **38**).
- `rg --files -g '**/mod.rs'` (`specify-cli`): **3** files — `tests/common/mod.rs`, `crates/domain/tests/common/mod.rs`, `wasi-tools/vectis/tests/engine_support/mod.rs`. All test-fixture roots; coding-standards-compliant.
- `wc -l docs/standards/*.md AGENTS.md`:
  - `specify`: **555 total**.
  - `specify-cli`: **638 total**.
- Files >500 lines under `crates/` and `src/` (`specify-cli`):
  - Tests: `crates/domain/tests/workspace.rs` **1048**, `crates/domain/tests/finalize.rs` **947**, `crates/domain/tests/registry.rs` **922**.
  - Source: `src/commands/plan/create.rs` **895**, `crates/domain/src/discovery/document.rs` **891**, `crates/domain/src/slice/fusion.rs` **839**, `crates/domain/src/adapter/core.rs` **709**, `crates/domain/src/change/plan/core/model.rs` **629**, `crates/domain/src/spec/provenance.rs` **607**, `crates/domain/src/journal.rs` **595**, `crates/tool/src/validate.rs` **520**, `crates/domain/src/adapter/cache/io.rs` **509**.
- `make checks` (`specify`): **passed** — `All checks passed.` Total failures: **0**.
- `cargo make ci` (`specify-cli`): **passed** — `[cargo-make] INFO - Build Done in 167.86 seconds.` First error: **none**. (`lint + file-size + test + test-docs + doc + vet + outdated + deny + fmt` all green; `Vetting Succeeded (285 fully audited, 35 partially audited, 313 exempted)`; `cargo deny` `advisories ok, bans ok, licenses ok, sources ok`.)
- `rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' --glob '!**/tests.rs' --glob '!**/test_support*' crates/ src/` (`specify-cli`): summed **518** matching lines (still inflated — the largest single contributors are inline `#[cfg(test)] mod tests { … }` blocks inside source files such as `crates/domain/src/init/regular.rs`; the operator-path count is materially smaller and no panic-on-operator-path defect surfaced).
- `rg -c 'panic!|unreachable!|todo!' --glob '!**/tests/**' --glob '!**/tests.rs' --glob '!**/test_support*' crates/ src/` (`specify-cli`): summed **41** matching lines, again dominated by inline `#[cfg(test)]` modules.

## Structural Findings

### F1 — Inline `add` override events via `emit_override_events`

**Evidence:** `src/commands/plan/create.rs:643-663` hand-builds `PlanAmendAuthorityOverride` events in the `add` handler (one per `created.authority_override.by_kind` entry) while the sibling `create` and `amend` handlers route through `mutate_authority_overrides` → `emit_override_events` (`:299-348`). The hand-built block reconstructs the same `(plan_name, slice_name, Set, claim_kind=Some(kind.to_string()), source_key=Some(key.clone()))` shape:

```text
src/commands/plan/create.rs:645:                let events: Vec<journal::Event> = created
src/commands/plan/create.rs:649:                    .map(|(kind, key)| {
src/commands/plan/create.rs:650:                        journal::Event::new(
src/commands/plan/create.rs:652:                            journal::EventKind::PlanAmendAuthorityOverride {
src/commands/plan/create.rs:299:fn emit_override_events(
```

Both call sites iterate in `BTreeMap<ClaimKind, _>` order so the on-disk batched-append bytes are identical when set events are the only contribution. `rg -c 'PlanAmendAuthorityOverride' src/commands/plan/create.rs` returns **3** (the `EventKind::` literal appears once in `add`, once in `emit_override_events`, plus the doc reference).

**Action:**
1. In `add()`, replace the `let (events, created_entry) = { … by_kind.iter().map(...).collect()` block (`:643-663`) with: build a `BTreeMap<(String, ClaimKind), String>` from `created.authority_override.by_kind` (keyed on `(created.name.clone(), *kind)`) and call `emit_override_events(&plan_name, &set_map, &BTreeSet::new(), &BTreeSet::new(), &BTreeMap::new(), now)`. Clone `created` before the call for `created_entry`.
2. Mark `emit_override_events` `pub(super)` (it already lives in the same module; no visibility widening needed).
3. Keep the `journal::append_batch(ctx.layout(), &override_events)?` call outside `with_state` unchanged.

**Quality delta:** `−10 LOC, −1 duplicate PlanAmendAuthorityOverride literal, lower call-site burden in add()`.

**Net LOC:** `src/commands/plan/create.rs` **895 → ~885**.

**Done when:** `rg -c 'EventKind::PlanAmendAuthorityOverride' src/commands/plan/create.rs` drops from **3** to **2** (one in `emit_override_events`, one in the rustdoc list), and `cargo make check` passes.

**Rule?** no — single handler reproducing one shared helper's output.

**Counter-argument:** The hand-built block reads top-down inside the `with_state` closure without forward references. It loses because `create()` already takes the indirection and the wire output must stay byte-identical; one shared writer is the whole point of `emit_override_events`.

**Depends on:** none.

### F2 — Collapse `Patch::from(Option<String>)` triple

**Evidence:** `src/commands/plan/create.rs:745-759` in `amend()` carries three near-identical `match Option<String> { None => Patch::Keep, Some(s) if s.is_empty() => Patch::Clear, Some(s) => Patch::Set(s) }` blocks, one each for `project`, `target`, `description`. Only `target` maps the `Set` branch through `parse_target_flag(&s)?`. `rg -nC2 'match (project|target|description)\.clone\(\)' src/commands/plan/create.rs` returns **3** identical-shape matches × 4 lines + the field name line each (15 LOC total under the `let patch = EntryPatch { … }` literal).

**Action:**
1. Add an inherent helper to `Patch<String>` in `crates/domain/src/change/plan/core/model.rs` (next to `Patch::apply` at `:552-561`):
   ```rust
   impl Patch<String> {
       #[must_use]
       pub fn from_string_option(value: Option<String>) -> Self {
           match value {
               None => Self::Keep,
               Some(s) if s.is_empty() => Self::Clear,
               Some(s) => Self::Set(s),
           }
       }
   }
   ```
2. Replace the `project:` and `description:` matches with `Patch::from_string_option(project.clone())` and `Patch::from_string_option(description.clone())`.
3. Replace `target:` with `match target.clone() { None => Patch::Keep, Some(s) if s.is_empty() => Patch::Clear, Some(s) => Patch::Set(parse_target_flag(&s)?) }` collapsed onto the same line (or leave as the one remaining match — only one site for the fallible variant).

**Quality delta:** `−9 LOC net (+7 helper / −16 call-site), −2 branch clusters, lower call-site burden`.

**Net LOC:** `model.rs` **629 → 636**, `create.rs` **895 → 879** ≈ combined **−9**.

**Done when:** `rg -c 'Some\(s\) if s\.is_empty\(\) => Patch::Clear' src/commands/plan/create.rs` drops from **3** to **1** (the remaining `target` branch), `rg -n 'fn from_string_option' crates/domain/src/change/plan/core/model.rs` returns **≥ 1**, and `cargo make check` passes.

**Rule?** no — single handler, three sites only.

**Counter-argument:** The repeated match is explicit about the empty-string-as-clear convention at each field. It loses because the convention is wire-stable across every `Patch<String>` site and the helper keeps the convention named in one place.

**Depends on:** none.

### F3 — Share `discovery-candidate-unknown` builder

**Evidence:** `crates/domain/src/discovery/document.rs:213-222` (`add_alias`) and `:257-266` (`remove_alias`) each open with the same eight-line `let Some(candidate) = self.candidate_mut(candidate_id) else { return Err(Error::Diag { code: "discovery-candidate-unknown", detail: format!("no candidate `{candidate_id}` in discovery.md; --{add|remove}-alias must reference an existing candidate id") }) };` shape. `rg -n '"discovery-candidate-unknown"' crates/domain/src/discovery/document.rs` returns **2** (one per handler) plus three doc-comment / test references.

**Action:**
1. Add a small inherent helper on `Discovery` next to `candidate_mut` (`:126`):
   ```rust
   fn candidate_mut_or_unknown(&mut self, id: &str, flag: &str) -> Result<&mut Candidate> {
       self.candidate_mut(id).ok_or_else(|| Error::Diag {
           code: "discovery-candidate-unknown",
           detail: format!(
               "no candidate `{id}` in discovery.md; {flag} must reference an existing candidate id"
           ),
       })
   }
   ```
2. Replace the `let Some(candidate) = self.candidate_mut(candidate_id) else { … };` blocks in `add_alias` and `remove_alias` with `let candidate = self.candidate_mut_or_unknown(candidate_id, "--add-alias")?;` (and `"--remove-alias"`).
3. Keep `Error::Diag { code: "discovery-candidate-unknown", … }` literal-free outside the helper — the rollback path inside `add_alias` (`:240`) already re-uses `candidate_mut(candidate_id)` for cleanup, no change needed there.

**Quality delta:** `−7 LOC, −1 duplicate Error::Diag literal, lower call-site burden`.

**Net LOC:** `discovery/document.rs` **891 → ~884**.

**Done when:** `rg -c '"discovery-candidate-unknown"' crates/domain/src/discovery/document.rs` (excluding test + doc lines) drops from **2** to **1** (helper body only), and `cargo make check` passes.

**Rule?** no — two callers, one diagnostic.

**Counter-argument:** Each callsite is locally readable as a flag-specific guard. It loses because the flag name is the only varying token; the helper keeps the readability with one parameter.

**Depends on:** none.

## One-Touch Tidies

### T1 — Helper for `parse_slice_pair_args` → `(String, ClaimKind, String)` flatten

**Evidence:** `src/commands/plan/create.rs:502-509` (`create`) and `:702-709` (`amend`) carry an identical 8-line chunk:

```text
src/commands/plan/create.rs:503:        parse_slice_pair_args::<AuthorityOverrideKindAssign>(
src/commands/plan/create.rs:504:            authority_override,
src/commands/plan/create.rs:505:            "--authority-override",
src/commands/plan/create.rs:506:        )?
src/commands/plan/create.rs:507:        .into_iter()
src/commands/plan/create.rs:508:        .map(|(slice, assign)| (slice, assign.kind, assign.source_key))
src/commands/plan/create.rs:509:        .collect();
```

**Action:**
1. Add a private helper next to `parse_slice_pair_args` (`:244`):
   ```rust
   fn parse_authority_override_assigns(raw: &[String]) -> Result<Vec<(String, ClaimKind, String)>> {
       Ok(parse_slice_pair_args::<AuthorityOverrideKindAssign>(raw, "--authority-override")?
           .into_iter()
           .map(|(slice, a)| (slice, a.kind, a.source_key))
           .collect())
   }
   ```
2. Replace both call sites with `let override_assigns = parse_authority_override_assigns(authority_override)?;` / `let override_sets = parse_authority_override_assigns(authority_override)?;`.

**Quality delta:** `−7 LOC, −1 duplicate flatten-chain`.

**Net LOC:** `create.rs` **895 → ~888**.

**Done when:** `rg -c '\.map\(\|\(slice, assign\)\| \(slice, assign\.kind' src/commands/plan/create.rs` drops from **2** to **0**, and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** Two call sites in one file is the borderline case. It loses because the chain is the longest repeated literal in the handler and the helper has one parameter only.

**Depends on:** none.

### T2 — Drop duplicate "Finding-ID conventions" boilerplate across vectis review briefs

**Evidence:** Three vectis review briefs carry near-identical four-bullet `## Finding-ID conventions` blocks, differing only in the platform-prefix tokens on bullet 1 (`IOS-1/SWF-1`, `AND-1/KTL-1`, `CRX-1/LOG-1`) and the `VECTIS-IOS/AND/CORE-001` example on bullet 2:

```text
adapters/targets/vectis/briefs/build/ios/review.md:22-25
adapters/targets/vectis/briefs/build/android/review.md:22-25
adapters/targets/vectis/briefs/build/core/review.md:26-29
```

Bullets 3 (`Severity reflects antagonist adjustments …`) and 4 (`Every finding carries a file:line reference …`) are byte-identical across all three.

**Action:**
1. Move the two shared bullets (severity + file:line) into `adapters/targets/vectis/references/review/iteration-report.md` under a new `## Finding-ID conventions` section (referenced already from each brief's Pipeline step 5).
2. In each of the three review briefs, replace the four-bullet block with the two platform-specific bullets only and append one line: `See [iteration-report.md](../../../references/review/iteration-report.md) § Finding-ID conventions for severity and `file:line` rules.`

**Quality delta:** `−4 LOC (six removed, two added per file × 3 files; reference doc gains four lines), −2 duplicate prose blocks`.

**Net LOC:** ios + android + core review briefs **25 + 25 + 30 = 80 → ~72** combined; reference doc gains **~4 LOC**.

**Done when:** `rg -nF 'Severity reflects antagonist adjustments' adapters/targets/vectis/briefs/` returns **0** (all hits live under `references/` now), and `make checks` passes.

**Rule?** no.

**Counter-argument:** Each platform brief is meant to be standalone. It loses because every brief already references the shared `iteration-report.md` for the report shape; the severity / `file:line` rules belong with the report shape, not duplicated in the per-platform pipelines.

**Depends on:** none.

## Post-mortem

- F1 (emit_override_events for add): actual ΔLOC -3 vs predicted -10; done when clean; regressions none.
- F2 (Patch::from_string_option): actual ΔLOC +7 vs predicted -9; done when clean; regressions none.
- F3 (candidate_mut_or_unknown helper): actual ΔLOC +3 vs predicted -7; done when clean; regressions none.
