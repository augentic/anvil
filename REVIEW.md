# Specify / Specify-CLI — Subtraction Pass

## Summary

1. **Top three by sort key**:
   (a) **F1** — `make check` is red on `main`; two stale rfc links in `rfcs/roadmap.md` (verified defect, ΔLOC ≈ 0).
   (b) **F2** — Three hand-rolled `Ord`/`PartialOrd` impls (`SourceOperation`, `TargetOperation`, `AuthorityOverrideAction`) all replaceable with `#[derive]` (~−68 LOC, idiom).
   (c) **F3** — Drop redundant `root_dir: PathBuf` fields from `ResolvedSourceAdapter` / `ResolvedTargetAdapter` (already exposed via `location.path()`) and the duplicate clone in `locate_axis` (~−18 LOC, −2 fields).
2. **Total ΔLOC if all land**: **−131 LOC** across 4 structural findings + 2 tidies.
3. **Primary non-LOC axes moved**: −2 public struct fields, −7 hand-rolled `impl` blocks, +3 `#[derive]` lines, −1 failing CI predicate, −1 transitive `serde_json::to_string` allocation per BTreeMap key compare on the cache-index hot path.
4. **Verified defects closed**: 1 — `make check`'s `links.unresolved` predicate (×2 lines, both in the same file). Net +ΔLOC from defect-only findings: **0** (well under the +30 cap). No CI predicate failures remain after F1 lands; clippy / `cargo make check` already green on `specify-cli`.
5. **Most likely to break in remediation**: **F2** — variant-reorder of `TargetOperation` (`Shape, Build, Merge` → `Build, Merge, Shape`) lets `derive(Ord)` reproduce the existing kebab-alphabetical iteration order; if any caller `match`es by integer discriminant or `as u8` casts (none found, but check), it will silently shift.

---

## Reconnaissance numbers (current state)

```text
tokei (both repos, totals):  1164 files, 162427 lines, 83159 code
specify-cli Rust LOC:        302 files / 49504 code lines
specify-cli mod.rs (non-test): 0 (only 3 in tests/common/)
specify         standards:  716 LOC across 5 files (cli-contract.md, doc-authoring.md, skill-authoring.md, skill-guardrails.md)
specify-cli     standards:  878 LOC across 6 files (style/coding/handler/architecture/testing/workflow)
files > 500 LOC (specify-cli, non-test):
  890  crates/domain/src/discovery/document.rs
  742  crates/domain/src/adapter/core.rs
  700  crates/domain/src/change/plan/core/model.rs
  667  crates/domain/src/slice/fusion.rs
  641  crates/domain/src/journal.rs
  607  crates/domain/src/spec/provenance.rs
  520  crates/tool/src/validate.rs
  514  src/commands/plan/lifecycle.rs
  509  crates/domain/src/adapter/cache/io.rs
make check (specify):     FAIL — 2 × links.unresolved in rfcs/roadmap.md
cargo clippy --workspace --all-targets -D warnings (specify-cli): PASS
unwrap/expect on non-test paths under crates/ + src/: ~190 hits, all reviewed; every reachable hit
  is regex-static, schema-static, or in `#[cfg(test)]` blocks (greps below).
panic!/unreachable! on non-test paths: 16 hits, every one inside a `#[cfg(test)]` mod block
  (false positive — test glob predicate excluded `tests.rs` only, not `mod tests {…}`).
```

No verified panic-on-operator-path or wire-contract finding turned up beyond F1.

---

## Structural findings

### F1 — Fix broken `rfcs/roadmap.md` links so `make check` is green again [defect closure]

**Evidence (current state)**:

```text
$ cd specify && make check
FAIL: links.unresolved: Broken link in rfcs/roadmap.md: next/rfc-28-codex-rules.md
  at rfcs/roadmap.md:1
FAIL: links.unresolved: Broken link in rfcs/roadmap.md: rfc-5-tooling.md
  at rfcs/roadmap.md:1
2 check failure(s).
```

The matching `git status` confirms RFC-5 was moved to `rfcs/done/rfc-5-tooling.md` and RFC-28 was promoted out of `rfcs/next/` to `rfcs/rfc-28-codex-rules.md`; nothing updated `rfcs/roadmap.md`:

```55:55:rfcs/roadmap.md
**Consumes:** [RFC-28](next/rfc-28-codex-rules.md)'s resolved codex export and structured finding schema.
```

```120:120:rfcs/roadmap.md
**Goal:** Land the framework dev-tooling workspace at `augentic/specify/tooling/` per [RFC-5](rfc-5-tooling.md) — schema-first authoring feedback in Cursor, a single `tooling` binary with `check` and `docgen` subcommands for CI and local use, integration tests that replace `tests/cross_repo.ts`, and full Deno retirement.
```

**Action**:

1. In `rfcs/roadmap.md` line 55, replace `next/rfc-28-codex-rules.md` with `rfc-28-codex-rules.md`.
2. In `rfcs/roadmap.md` line 120, replace `rfc-5-tooling.md` with `done/rfc-5-tooling.md`.
3. Re-run `make check`.

**Quality delta**: `−1 defect, 0 LOC`. Two character-level path edits; no axes regressed.
**Net LOC**: 392 → 392 (two characters changed in two lines).
**Done when**: `make check` exits 0 and `tooling/target/release/tooling check 2>&1 | rg -c links.unresolved` returns `0`.
**Rule?**: no — `tooling check` already enforces this; the rename slipped through because the move pre-dated this pass.
**Counter-argument**: "Re-run `tooling check` in pre-commit instead." — Loses: pre-commit isn't part of this repo's policy and would add infrastructure; the predicate already exists.
**Depends on**: none.

---

### F2 — Replace three hand-rolled `Ord` / `PartialOrd` impls with `#[derive]`

**Evidence (current state)**:

```text
$ cd specify-cli && rg -n 'fn cmp\(&self|impl Ord for|impl PartialOrd for' \
    crates/ src/ --glob '!**/tests/**' --glob '!**/tests.rs'
crates/domain/src/journal.rs:316:impl PartialOrd for AuthorityOverrideAction {
crates/domain/src/journal.rs:322:impl Ord for AuthorityOverrideAction {
crates/domain/src/journal.rs:323:    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
crates/domain/src/adapter/operation.rs:70:impl Ord for SourceOperation {
crates/domain/src/adapter/operation.rs:71:    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
crates/domain/src/adapter/operation.rs:76:impl PartialOrd for SourceOperation {
crates/domain/src/adapter/operation.rs:140:impl Ord for TargetOperation {
crates/domain/src/adapter/operation.rs:141:    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
crates/domain/src/adapter/operation.rs:146:impl PartialOrd for TargetOperation {
```

All three impls exist solely to *override the derive's variant-declaration order*. Each ships with a paragraph-long doc comment claiming the manual impl "decouples the wire iteration order from variant order against future reshuffles". The same enums already carry a unit test that pins the wire order; the test (not the impl) is what protects the invariant.

For `SourceOperation { Enumerate, Extract }`, declaration order already matches kebab-alphabetical order, so the manual `to_string().cmp(&to_string())` is dead code that allocates twice per compare:

```70:80:crates/domain/src/adapter/operation.rs
impl Ord for SourceOperation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_string().cmp(&other.to_string())
    }
}
```

For `TargetOperation { Shape, Build, Merge }`, declaration order disagrees with kebab-alphabetical order; reorder variants to `Build, Merge, Shape` and the derive matches the existing wire order.

For `AuthorityOverrideAction { Set, Clear }`, declaration order already gives `Set < Clear`; the existing `set_sorts_before_clear` test (lines 332–344) is the load-bearing guard, not the manual impl.

**Action** (all in `crates/domain/src/`):

1. `adapter/operation.rs`:
   - Add `PartialOrd, Ord` to both `derive(...)` lists (1 line each).
   - Reorder `TargetOperation` variants to `Build, Merge, Shape` (and move their doc comments with them).
   - Delete `impl Ord for SourceOperation` (lines 70–74), `impl PartialOrd for SourceOperation` (76–80), `impl Ord for TargetOperation` (140–144), `impl PartialOrd for TargetOperation` (146–150) — 4 impl blocks, ~22 LOC.
   - Trim the four-paragraph doc preamble that justifies the manual impls down to one sentence ("Variants declared in kebab-alphabetical order so `BTreeMap` iteration matches the wire envelope.") — ~16 LOC.
   - Add or extend the existing `target_operation_round_trips_kebab_case` test with `assert!(TargetOperation::Build < TargetOperation::Merge && TargetOperation::Merge < TargetOperation::Shape)` (3 lines).
2. `journal.rs`:
   - Add `PartialOrd, Ord` to `AuthorityOverrideAction`'s `derive(...)` list.
   - Delete `const fn sort_key` (lines 308–313, 6 LOC), `impl PartialOrd` (316–320, 5 LOC), `impl Ord` (322–326, 5 LOC), and the 12-line "Ord is implemented by hand…" rationale paragraph above the enum (~28 LOC total). The existing `set_sorts_before_clear` test continues to guard variant-order drift.

**Quality delta**: `−68 LOC, −7 impl blocks, −1 hand-rolled `sort_key` method, −1 per-cmp `String` heap alloc on cache-index sort hot path` (axes: LOC, types, idiom — `derive(Ord)` matches stdlib / clap / serde idiom).
**Net LOC**: 204 (operation.rs) + 641 (journal.rs) = 845 → ~777.
**Done when**: `rg -n 'impl (Partial)?Ord for (SourceOperation|TargetOperation|AuthorityOverrideAction)' crates/ src/` returns no matches; `cargo nextest run -p specify-domain operation::tests authority_override` passes.
**Rule?**: no — `clippy::derive_ord_xor_partial_ord` plus the existing tests already provide negative coverage; a project-specific lint would be three duplicate hits' worth of value.
**Counter-argument**: "The manual impl is documented as a deliberate decoupling from variant order." — Loses: the unit tests are the actual defence; the impl is redundant ceremony, and `to_string()` per cmp is a measurable inefficiency on the cache-index sort path. Reorderings are caught by the existing tests.
**Depends on**: none.

---

### F3 — Drop redundant `root_dir: PathBuf` from `Resolved{Source,Target}Adapter`

**Evidence (current state)**:

```280:304:crates/domain/src/adapter/core.rs
pub struct ResolvedSourceAdapter {
    pub manifest: SourceAdapter,
    pub root_dir: PathBuf,
    pub location: AdapterLocation,
}
…
pub struct ResolvedTargetAdapter {
    pub manifest: TargetAdapter,
    pub root_dir: PathBuf,
    pub location: AdapterLocation,
}
```

`AdapterLocation` already exposes the path:

```162:179:crates/domain/src/adapter/core.rs
impl AdapterLocation {
    pub const fn label(&self) -> &'static str { … }
    pub const fn path(&self) -> &PathBuf {
        match self { Self::Local(p) | Self::Cached(p) => p }
    }
}
```

And `locate_axis` literally clones the path back out solely to satisfy the `root_dir` field:

```478:481:crates/domain/src/adapter/core.rs
    check_axis_unique_for_name_memo(axis, name, project_dir, location.path())?;
    let path = location.path().clone();
    Ok((path, location))
```

Across both repos `root_dir` is read in 4 places:

```text
src/commands.rs:176,188      resolved.root_dir.display().to_string()  (×2)
src/commands/context/assemble.rs:72,77    adapter.root_dir.join(...)  (×2)
src/commands/tool.rs:38,42                plugin.root_dir(.clone())   (×2)
crates/domain/tests/adapter.rs:74,90,295   resolved.root_dir.ends_with(…)
```

Every call site can swap `.root_dir` for `.location.path()` (or `.location.path().clone()` for the one tool.rs `clone` site) verbatim.

**Action** (`crates/domain/src/adapter/core.rs`):

1. Delete the `root_dir: PathBuf` field + its 2-line doc comment from both `ResolvedSourceAdapter` and `ResolvedTargetAdapter` (~6 LOC).
2. In both `*::resolve` methods, drop the `root_dir,` line from the struct construction (2 LOC).
3. Change `load_validated`'s return type from `(PathBuf, AdapterLocation, PathBuf, serde_json::Value)` to `(AdapterLocation, PathBuf, serde_json::Value)`, dropping the leading `PathBuf` (and the matching destructuring patterns at the two call sites) — ~3 LOC.
4. Change `locate_axis`'s return type from `Result<(PathBuf, AdapterLocation), Error>` to `Result<AdapterLocation, Error>`; delete the trailing two-line `let path = location.path().clone(); Ok((path, location))` and return `Ok(location)` directly (~3 LOC).
5. Update the 8 call sites listed above (rename only — same character count).

**Quality delta**: `−18 LOC, −2 public struct fields, −1 PathBuf clone per resolve`. (axes: LOC, types, call-site burden when exposing path — callers gain nothing extra).
**Net LOC**: 742 → ~724 in `adapter/core.rs`; touched files compile and pass tests with no logic change.
**Done when**: `rg -n 'root_dir' crates/domain/src/adapter/ src/ crates/domain/tests/` finds no matches in struct fields or destructure patterns; `cargo nextest run -p specify-domain adapter` passes.
**Rule?**: no — single occurrence in this codebase.
**Counter-argument**: "Two public fields is fine; tests use both." — Loses: tests use exactly one (assert against the `ends_with`), `.location.path().ends_with(...)` is the same number of characters and removes the duplicated state.
**Depends on**: none.

---

### F4 — Replace `RequirementStatus` / `RequirementTag` hand-rolled `as_str` / `parse` with strum derives

**Evidence (current state)**:

```70:93:crates/domain/src/spec/provenance.rs
impl RequirementStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Agreed => "agreed",
            Self::Unknown => "unknown",
            Self::Conflict => "conflict",
            Self::Divergence => "divergence",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "agreed" => Some(Self::Agreed),
            "unknown" => Some(Self::Unknown),
            "conflict" => Some(Self::Conflict),
            "divergence" => Some(Self::Divergence),
            _ => None,
        }
    }
}
```

```107:137:crates/domain/src/spec/provenance.rs
impl RequirementTag {
    pub const fn as_str(self) -> &'static str { … }
    pub const fn expected_status(self) -> RequirementStatus { … }
    fn parse(s: &str) -> Option<Self> { … }
}
```

Both enums already carry `#[serde(rename_all = "kebab-case")]` and the workspace already pulls in `strum = { version = "0.28", features = ["derive"] }`. `strum::Display`, `strum::EnumString`, and `strum::IntoStaticStr` (already used elsewhere — see `Axis`, `CacheMode`, `CacheMissReason`) cover both directions of the mapping for free.

External callers of `as_str` (4 hits, all in `provenance.rs::check_status` lines 445–459) feed a `format!("{}", x.as_str())` pattern — `Display` makes them `format!("{}", x)`, smaller still. External callers of `parse` (1 hit, line 527 inside the same file) become `s.parse().ok()` via `FromStr`.

**Action** (`crates/domain/src/spec/provenance.rs`):

1. Add `strum::Display, strum::EnumString, strum::IntoStaticStr` to the `derive` list on `RequirementStatus` and `RequirementTag`. Keep `#[strum(serialize_all = "kebab-case")]` (matches the existing serde rule).
2. Delete `impl RequirementStatus { as_str, parse }` (~22 LOC) and `impl RequirementTag { as_str, parse }` (~13 LOC), keeping `expected_status` (still load-bearing).
3. Update the four call sites at lines 445–459 to use `{tag}` / `{status}` format-args directly; update line 527 to `status_raw.as_deref().and_then(|s| s.parse().ok())`.

**Quality delta**: `−25 LOC, −4 hand-rolled methods, +3 derive entries, idiom (matches Axis/CacheMode/CacheMissReason already in the same crate)`.
**Net LOC**: 607 → ~582 in `spec/provenance.rs`.
**Done when**: `rg -n 'fn (as_str|parse)\(' crates/domain/src/spec/provenance.rs` finds zero hand-rolled methods on `RequirementStatus` / `RequirementTag`; `cargo nextest run -p specify-domain spec::provenance` passes.
**Rule?**: no.
**Counter-argument**: "`as_str` is `const fn`; `Display` isn't." — Loses: every existing call site is inside `format!`/`writeln!` (non-const context), and no caller uses `as_str` in a `const` position. The const-ness was never reached.
**Depends on**: none.

---

## One-touch tidies

### T1 — `locate_axis` reuses `cached` / `local` instead of recomputing them in the not-found branch

**Evidence**:

```444:481:crates/domain/src/adapter/core.rs
fn locate_axis(...) -> Result<(PathBuf, AdapterLocation), Error> {
    let cached = cache_dir(project_dir, axis, name);
    let location = if cached.is_dir() {
        AdapterLocation::Cached(cached)
    } else {
        let local = adapter_axis_dir(project_dir, axis).join(name);
        if local.is_dir() {
            AdapterLocation::Local(local)
        } else {
            return Err(Error::Diag {
                code: "adapter-not-found",
                detail: format!(
                    "adapter `{name}` (axis `{axis}`) not found at {} or {}",
                    cache_dir(project_dir, axis, name).display(),    // re-walks
                    adapter_axis_dir(project_dir, axis).join(name).display(),  // re-walks
                ),
            });
        }
    };
    …
}
```

Both `cache_dir(...)` and `adapter_axis_dir(...).join(name)` are recomputed inside the error literal even though the matching `PathBuf`s are still in scope (`cached` was moved out of the `if`-cond branch, but the not-found branch never reads it; `local` is the local in scope).

**Action**: rewrite the function body so both `PathBuf`s are computed once and named (`cached`, `local`) and the not-found branch references them:

```rust
let cached = cache_dir(project_dir, axis, name);
let local = adapter_axis_dir(project_dir, axis).join(name);
let location = if cached.is_dir() {
    AdapterLocation::Cached(cached)
} else if local.is_dir() {
    AdapterLocation::Local(local)
} else {
    return Err(Error::Diag {
        code: "adapter-not-found",
        detail: format!(
            "adapter `{name}` (axis `{axis}`) not found at {} or {}",
            cached.display(), local.display(),
        ),
    });
};
```

**Quality delta**: `−4 LOC, −2 redundant filesystem-path constructions per missing-adapter error`.
**Net LOC**: ~742 → ~738 in `adapter/core.rs`.
**Done when**: `rg -nC1 'cache_dir\(project_dir, axis, name\)\.display' crates/domain/src/adapter/core.rs` finds zero hits inside `locate_axis`; tests pass.
**Rule?**: no.
**Counter-argument**: "The not-found branch is cold; the duplication doesn't matter." — Loses: −LOC and the variant pair already exists in scope; deleting redundancy is the priority of this pass.
**Depends on**: F3 (or land standalone — both edits live in the same function).

---

### T2 — Drop the `tooling check`'s 31-element `&[&dyn Check; 31]` array length annotation

**Evidence**:

```50:83:tooling/src/check/mod.rs
pub fn run(ctx: &Context) -> Vec<Finding> {
    let checks: [&dyn Check; 31] = [
        &AdapterCheck,
        …
    ];
```

The `; 31` length is hand-counted and silently goes stale every time a check is added or removed. Rust infers the array length from the literal; the explicit `; 31` annotation only exists to be wrong on the next add.

**Action**: change `let checks: [&dyn Check; 31] = [...]` to `let checks: [&dyn Check; _] = [...]` (`feature(generic_arg_infer)` is stable since 1.79) or simply `let checks: &[&dyn Check] = &[...]`. Pick whichever requires zero feature pin — the `&[&dyn Check]` slice form needs no MSRV bump.

**Quality delta**: `−1 brittle constant, 0 LOC`.
**Net LOC**: 100 → 100 in `tooling/src/check/mod.rs`.
**Done when**: `rg -n '\[&dyn Check; \d+\]' tooling/src/check/` returns no hits; `make check` (or `cargo run --release --manifest-path tooling/Cargo.toml -- check`) still loads the same 31 checks.
**Rule?**: no.
**Counter-argument**: "Explicit length documents the count." — Loses: the count drifts on every add/delete and no other workspace check site uses this pattern.
**Depends on**: none.

---

## Findings considered and dropped

- **Add a clippy lint preventing future hand-rolled `Ord` impls.** Forbidden by the "no new mechanical enforcement" rule; F2's deletion + tests are sufficient.
- **Rewrite `crates/domain/src/discovery/document.rs` (890 LOC) on top of a Markdown crate.** The hand-rolled parser is documented as deliberately scoped to the `## Candidate inventory` section grammar; pulling in pulldown-cmark or comrak would *add* a dependency for a net-positive LOC delta. Drop.
- **Collapse the duplicated `ResolveBody` arms in `src/commands.rs` (lines 169–197).** The two arms differ only by `SourceAdapter` vs `TargetAdapter`. A `trait ResolvedAdapter` abstraction would touch ≥ 2 existing types but only deletes ~10 LOC of literal duplication; the trait itself spends 8+ LOC. Net wash. Drop.
- **Delete `is_kebab_target_name` (`crates/domain/src/change/plan/core/model.rs:389`) in favour of a regex.** `regex` is already in workspace deps but pulling it into this hot parse path adds compile-time regex setup for a 17-line function. No net gain. Drop.
- **Migrate `RequirementStatus::as_str` to `IntoStaticStr` even though `Display` does the same job.** `IntoStaticStr` would preserve `&'static str` callers; F4 already shows there are none. Drop in favour of plain `Display`.
- **Replace 16 `panic!`/`unreachable!` hits on the "non-test" recon path.** Every hit is inside a `#[cfg(test)] mod tests {…}` block (the recon `--glob '!**/tests.rs'` was a false-positive filter). Verified by reading each hit; no operator-reachable panic surface to close.

---

## Threshold assertion

- 4 structural findings (F1: defect closure with severity ≥ CI predicate; F2: −68 LOC ≥ 30; F3: −18 LOC + −2 fields = ≥ 2 axes; F4: −25 LOC + −4 methods + idiom = ≥ 2 axes).
- 2 one-touch tidies (T1: −4 LOC single axis; T2: 0 LOC, single axis — qualifies because it removes a verifiably wrong constant).
- Subtraction-only ΔLOC: **−131 LOC**. Defect-only ΔLOC: **0** (well under the +30 cap).
- No formatting-only, rename-only, comment-only, or "abstract over" findings included.
- No new dependencies, modules, files, traits, types, predicates, or rule docs proposed.

---

## Post-mortem

<!-- One line per applied finding: actual ΔLOC vs predicted, did the "done when" assertion flip cleanly, did anything regress. -->
- F1 (rfcs/roadmap.md broken links): actual ΔLOC 0 vs predicted 0; done when clean; regressions none.
- F2 (derive Ord for SourceOperation/TargetOperation/AuthorityOverrideAction): actual ΔLOC -60 vs predicted -68; done when clean; regressions none.
- F3 (drop root_dir from Resolved{Source,Target}Adapter): actual ΔLOC -12 vs predicted -18; done when clean; regressions none.
- F4 (strum derives for RequirementStatus/RequirementTag): actual ΔLOC -33 vs predicted -25; done when clean; regressions none.
- T1 (locate_axis cached/local dedup): actual ΔLOC -2 vs predicted -4; done when clean; regressions none.
- T2 (drop [&dyn Check; 31] length annotation): actual ΔLOC 0 vs predicted 0; done when clean; regressions none.

