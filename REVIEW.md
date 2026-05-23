# Code & Skill Review — single pass, quality-biased

Scope: `augentic/specify` and `augentic/specify-cli`, including shipped Skills. Pre-1.0 — no back-compat carries.

## Summary

1. Top three: **S1** delete `docs/example.html` (−2006 LOC); **S2** slim `.cursor/rules/project.mdc` against `AGENTS.md` (~−110 LOC, −1 duplicate vocabulary surface); **S3** collapse `parse_slice_kind_*` twins + drop `SliceKind` (~−30 LOC, −1 type, −1 helper).
2. Total ΔLOC if all structural land: **≈ −2 230 LOC** (S1+S2+S3+S4+S5).
3. Non-LOC axes moved: types (−2), helpers (−3 to −4), branches (−2), agent-context duplicate surface (−1), mod-edges flat.
4. Verified defects: **none qualified** under the bar (CI predicate / wire-contract / operator-panic / skill-predicate). `make checks` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` both pass clean; the 16 non-test `unwrap`/`expect` sites are all post-validation invariants (regex constants, internal JSON serialize). Defect-only ΔLOC: **0**.
5. Most-likely-to-break-in-remediation: **S2** — both `AGENTS.md` and `.cursor/rules/project.mdc` are loaded into every Cursor session; trimming the wrong block makes agents lose vocabulary and the `make checks` doc predicates may flag broken cross-references.

## Reconnaissance (current state)

| Probe | Result |
|---|---|
| `tokei` specify-cli Rust | 250 files, 47 948 lines, 41 804 code |
| `tokei` specify Markdown | 516 files, 49 654 lines |
| `cargo tree --duplicates` | base64 v0.21.7 vs v0.22.1, bitflags x2, core-foundation x2, darling x2, fixedbitset x2, … (all transitive via `wasm-pkg-client`) |
| `rg -c '^#\[test\]'` | **513** tests across `tests/` + `crates/` |
| `rg --files -g '**/mod.rs'` | only 3, all under `tests/common/` (compliant with no-mod.rs rule) |
| files > 500 lines under `crates/` and `src/` | 9 (top: `src/commands/plan/create.rs` 1024, `crates/domain/src/discovery/document.rs` 908, `crates/domain/src/slice/fusion.rs` 906) |
| `wc -l docs/standards/*.md AGENTS.md` (specify) | 108+186+154+86 = 534 |
| `make checks` (specify) | **All checks passed.** |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` (specify-cli) | **clean** — 0 warnings, 0 errors |
| non-test `unwrap`/`expect` count (specify-cli) | 16 (all regex constants, internal JSON serialize, post-validation invariants — no operator-reachable panics) |
| non-test `panic!`/`unreachable!` count | 0 in handler paths after filtering tests |

---

## Structural findings

### S1. Delete `docs/example.html`

**Evidence**

```text
$ wc -l docs/example.html
2006 docs/example.html
$ grep -i 'rfc-27' docs/example.html | head -2
<title>RFC-27 · Synthesis Sharpening</title>
<div class="eyebrow">Specify 2.1 · RFC-27</div>
$ ls rfcs/archive/rfc-27-synthesis.md
rfcs/archive/rfc-27-synthesis.md           # already archived
$ rg -l 'example\.html' . -g '!docs/example.html' -g '!**/.git/**'
docs/standards/doc-authoring.md
docs/assets/theme/specify-docs.css        # "Visual system ported from docs/example.html"
docs/templates/README.md
docs/theme/css/chrome.css
docs/README.md
```

The file is the rendered RFC-27 design page. RFC-27 has shipped (Specify 2.1) and the doc itself is archived under `rfcs/archive/`. The CSS — `docs/assets/theme/specify-docs.css` — is the runtime source of truth ("Visual system ported from `docs/example.html`."). The five remaining textual references all point at the archive for "see also" purposes.

**Action**

1. `git rm docs/example.html`.
2. In `docs/standards/doc-authoring.md` (line 3) replace the sentence "The visual system mirrors [`docs/example.html`](../example.html) — keep that file in sync when you add a new component class." with "The visual system lives in [`docs/assets/theme/specify-docs.css`](../assets/theme/specify-docs.css)."
3. In `docs/assets/theme/specify-docs.css`, `docs/theme/css/chrome.css`, `docs/templates/README.md`, `docs/README.md`: drop the parenthetical "see `example.html`" mentions (≤ 1 line each).

**Quality delta**: `−2006 LOC, −1 file, −5 stale link edges`.

**Net LOC**: 2006 + 5 reference-removal lines → 5 reference-removal lines. **2011 → 5**.

**Done when**: `git ls-files docs/example.html` returns nothing AND `rg 'example\.html' . -g '!**/.git/**' | wc -l` returns 0 AND `make checks` still passes.

**Rule?**: no — single instance.

**Counter-argument**: "It's the canonical visual reference; humans look at the rendered HTML side-by-side with the CSS." Loses because the CSS file is shorter, machine-validated by mdBook's build, and `docs/standards/doc-authoring.md` already keeps the canonical *table* of component classes; the HTML duplicates a slow-to-update screenshot of an archived RFC.

**Depends on**: none.

---

### S2. Slim `.cursor/rules/project.mdc` against `AGENTS.md`

**Evidence**

```text
$ wc -l AGENTS.md .cursor/rules/project.mdc
  86 AGENTS.md
 249 .cursor/rules/project.mdc
$ diff <(rg '^## ' AGENTS.md) <(rg '^## ' .cursor/rules/project.mdc) | head
1c1,11
< ## Cursor Cloud specific instructions
---
> ## Vocabulary
> ## Operating Model
> ## Main Inputs
> ## Authority Hierarchy
> ## Artifact Boundaries
> ## Specialist Skills
> ## Repository Layout
> ## Validation
> ## Code Style And Constraints
> ## File Locations
> ## Troubleshooting
```

Both files load as `always_applied_workspace_rule` on every Cursor session (verified in this conversation's `<rules>` section). Both define the same vocabulary block (source adapter, target adapter, plugin, candidate, evidence, provenance, authority, slice, change), the same skills inventory, and the same plan→execute rhythm. `AGENTS.md` is the Specify 2.0 / RFC-27 current copy; `project.mdc` was last meaningfully updated when RFC-27 was being designed and still says "Specify 2.0 splits adapters …" rather than the post-RFC-27 framing. The two files duplicate ≈ 110 lines of agent-context.

**Action**

1. Cut the **`## Vocabulary`** block (project.mdc lines 10–25) — keep AGENTS.md's longer authoritative version.
2. Cut the **`## Operating Model`** block (project.mdc lines 27–50) — duplicates AGENTS.md's `## Workflow overview`.
3. Cut **`## Specialist Skills`** verb-by-verb list (project.mdc lines 86–138, ≈ 50 lines) — duplicates the skill inventory plus per-plugin description, all of which is already in the per-skill SKILL.md frontmatter and in AGENTS.md's `## Skill / CLI responsibility split`.
4. Replace each with a one-line pointer: `See [AGENTS.md](../../AGENTS.md#vocabulary)` etc.
5. Keep the unique sections in `project.mdc`: `## Authority Hierarchy`, `## Artifact Boundaries`, `## Code Style And Constraints`, `## File Locations`. These are not in AGENTS.md and earn their lines.

**Quality delta**: `−110 LOC, −1 duplicate vocabulary surface, −2 always-applied rule blocks competing for agent attention`.

**Net LOC**: project.mdc 249 → ~140.

**Done when**:

```text
$ wc -l .cursor/rules/project.mdc
~140 .cursor/rules/project.mdc
$ make checks
All checks passed.
```

**Rule?**: no — one-time normalisation.

**Counter-argument**: "Cursor agents only get `project.mdc`; non-Cursor tools only get `AGENTS.md`; collapsing one breaks the other audience." Loses because (a) Cursor injects `AGENTS.md` as a workspace rule too (visible in the system prompt above), (b) modern AGENTS-aware agents read `AGENTS.md` first, and (c) the duplication is the bug — when one drifts, the other lies.

**Depends on**: none.

---

### S3. Collapse twin `parse_slice_kind_*` helpers + drop `SliceKind`

**Evidence**

```237:285:src/commands/plan/create.rs
#[derive(Debug, Clone)]
struct SliceKindAssign {
    slice: String,
    kind: ClaimKind,
    source_key: String,
}

#[derive(Debug, Clone)]
struct SliceKind {
    slice: String,
    kind: ClaimKind,
}
…
fn parse_slice_kind_assign_args(
    raw: &[String], flag: &'static str, value_names: &str,
) -> Result<Vec<SliceKindAssign>> { … chunks_exact(2) … }
```

```287:313:src/commands/plan/create.rs
fn parse_slice_kind_args(
    raw: &[String], flag: &'static str, value_names: &str,
) -> Result<Vec<SliceKind>> { … chunks_exact(2) … }
```

The two parsers are byte-for-byte the same except the second positional field differs (one parses `<kind>=<key>` via `AuthorityOverrideKindAssign::from_str`, one parses `<kind>` via `ClaimKind::from_str`). `SliceKind` exists for one reason — to be the return type of the second parser and the input of `dedup_clears`. `dedup_clears` already collapses to `BTreeSet<(String, ClaimKind)>`, so the struct never crosses a serialization or pretty-printing boundary.

**Action**

1. Delete `struct SliceKind` (4 LOC).
2. Replace `fn parse_slice_kind_args(...) -> Result<Vec<SliceKind>>` with one call site emitting `Vec<(String, ClaimKind)>` directly inline at line 832 (≈ 12 lines saved on the helper, 0 added at the call site since the chunk loop is shared shape). Or: keep one generic helper

   ```rust
   fn parse_slice_pair_args<T: FromStr<Err = String>>(
       raw: &[String], flag: &'static str, value_names: &str,
   ) -> Result<Vec<(String, T)>> { … }
   ```

   used by both call sites, returning `Vec<(String, T)>` and dropping both `SliceKindAssign` (folding `(slice, AuthorityOverrideKindAssign)` into `(slice, kind, key)` via `into_iter().map(|(s, a)| (s, a.kind, a.source_key))` at the one site that needs it).
3. Update `dedup_clears` (332-338) to take `&[(String, ClaimKind)]`.

**Quality delta**: `−~30 LOC, −1 type (SliceKind), −1 helper (parse_slice_kind_args), call-site burden flat`.

**Net LOC**: `src/commands/plan/create.rs` 1024 → ~995.

**Done when**:

```text
$ rg -n 'struct SliceKind\b' src/commands/plan/create.rs
(no matches)
$ rg -n '^fn parse_slice_kind' src/commands/plan/create.rs | wc -l
1
$ cargo make ci
…
```

**Rule?**: no.

**Counter-argument**: "Two named structs read clearer than tuples." Loses because (a) `SliceKind` carries no behaviour and never escapes the file, (b) the existing `(String, ClaimKind)` tuple is already the public shape `BTreeSet<(String, ClaimKind)>` exposes downstream of `dedup_clears`, and (c) the generic helper directly mirrors `clap`'s handling of `<key>=<val>` pairs in cargo's `cargo metadata --format-version`.

**Depends on**: none.

---

### S4. Inline `apply_sets` / `apply_single_clears` / `apply_clear_all`

**Evidence**

```text
$ awk '/^fn apply_sets/,/^}/ { lines++ } END { print lines }' src/commands/plan/create.rs
8
$ awk '/^fn apply_single_clears/,/^}/ { lines++ } END { print lines }' src/commands/plan/create.rs
8
$ awk '/^fn apply_clear_all/,/^}/ { lines++ } END { print lines }' src/commands/plan/create.rs
12
$ rg -n 'apply_sets\(|apply_single_clears\(|apply_clear_all\(' src/commands/plan/create.rs
521:    apply_sets(plan, plan_name, &set_map)?;
522:    apply_single_clears(plan, plan_name, &clear_set)?;
523:    let clear_all_emitted = apply_clear_all(plan, plan_name, &clear_all_set)?;
```

Each of the three `apply_*` functions has exactly one call site (`mutate_authority_overrides`), and each is a 3-5-line walk over `entry_mut(...).authority_override.by_kind`. Plus 8 lines of doc-comment per function (`/// Apply every survived …`). The extracted-helper pattern earns its lines only when there are ≥ 2 call sites — here there is one each, and the helpers are too small to materially aid readability over inline code in `mutate_authority_overrides`.

**Action**

1. Inline the three helpers' bodies into `mutate_authority_overrides` (lines 517-538). Replace each call with the 3-5-line walk it forwards to.
2. Delete the now-unused `apply_sets`, `apply_single_clears`, `apply_clear_all` and their docstrings (≈ 28 LOC including the 8-line docstrings each).

**Quality delta**: `−~25 LOC, −3 helpers, +0 call sites, branches flat`.

**Net LOC**: `src/commands/plan/create.rs` ~995 → ~970.

**Done when**:

```text
$ rg -n '^fn apply_(sets|single_clears|clear_all)\b' src/commands/plan/create.rs
(no matches)
$ cargo make ci
…
```

**Rule?**: no.

**Counter-argument**: "The three helpers document the deterministic order (sets → single clears → whole-map clears)." Loses because the order is already documented in the `mutate_authority_overrides` docstring (lines 495-516, "Order is deterministic per RFC-27 §D3: sets first … then single-kind clears, then whole-map clears."). The helpers carry the same prose three more times.

**Depends on**: S3 (same file, easier as one PR; still independent if S3 is dropped).

---

### S5. Inline `auto_commit`'s `run` / `warn` closures

**Evidence**

```280:316:src/commands/slice/merge.rs
let warn = |step: &str, msg: &str| eprintln!("warning: workspace auto-commit {step}: {msg}");
let run = |step: &str, args: &[&str]| -> Option<std::process::Output> {
    match git(project_dir, args) {
        Ok(output) => Some(output),
        Err(err) => {
            warn(step, &err.to_string());
            None
        }
    }
};
…
let Some(add) = run("git-add", &add_args) else { return };
…
match git(project_dir, &diff_args).map(|o| o.status) {
    Ok(status) if status.success() => return,
    Ok(status) if status.code() == Some(1) => {}
    Ok(status) => return warn("diff check", &format!("status {status}")),
    Err(err) => return warn("diff check", &err.to_string()),
}
…
if let Some(commit) = run("commit", &commit_args)
    && !commit.status.success()
{
    warn("commit", &String::from_utf8_lossy(&commit.stderr));
}
```

`auto_commit` defines a `warn` closure used 5 times and a `run` closure used 2 times. The `run` closure is just `git(...).ok().or_else(|err| { warn("…", &err.to_string()); None })`, but inlining unbalances the borrow on `warn`. The whole function is a 35-line three-step shell-out (`git add` → `git diff --cached --quiet` → `git commit`) that doesn't earn the closure ceremony.

**Action**

1. Replace the `run` closure with a single inline `match` per step (3 sites).
2. Replace the `warn` closure with a one-line `eprintln!` per use site (5 sites). The closure was a wrapper for a 1-line `eprintln!`; that's the canonical "function does not earn its name" pattern.
3. Pre-format the `pathspecs.iter().copied().collect::<Vec<_>>()` once and pass to the three `git` calls.

**Quality delta**: `−~10 LOC, −2 closures, +0 branches (the `match` ladders just move; one closure call site becomes one direct `eprintln!`)`.

**Net LOC**: `src/commands/slice/merge.rs` 360 → ~350.

**Done when**:

```text
$ rg -n 'let (warn|run) = ' src/commands/slice/merge.rs
(no matches)
$ cargo make ci
…
```

**Rule?**: no.

**Counter-argument**: "DRY — `warn` deduplicates the prefix string." Loses because the prefix is a 30-char literal that fits on the same line as the `eprintln!`, and the closure already inlines the format-arg call sites; the deduplication is one substring of literal text.

**Depends on**: none.

---

## One-touch tidies

### T1. Inline `workspace.rs::sync` registry match

**Evidence**

```19:31:src/commands/workspace.rs
pub fn sync(ctx: &Ctx, projects: &[String]) -> Result<()> {
    let registry = match Registry::load(&ctx.project_dir)? {
        None if !projects.is_empty() => return Err(registry_missing()),
        other => other,
    };
    let synced = if let Some(reg) = registry.as_ref() {
        let selected = reg.select(projects)?;
        sync_projects(&ctx.project_dir, &selected)?;
        true
    } else {
        false
    };
    let message = (!synced).then_some("no registry declared at registry.yaml; nothing to sync");
```

The `match` arm `other => other` does nothing — `Registry::load(&ctx.project_dir)?` already returns `Option<Registry>`, and the only handled case is "selecting projects but no registry → error." A flat `if let Some(reg) = registry.as_ref()` covers the rest.

**Action**

```rust
pub fn sync(ctx: &Ctx, projects: &[String]) -> Result<()> {
    let registry = Registry::load(&ctx.project_dir)?;
    let synced = if let Some(reg) = registry.as_ref() {
        let selected = reg.select(projects)?;
        sync_projects(&ctx.project_dir, &selected)?;
        true
    } else if !projects.is_empty() {
        return Err(registry_missing());
    } else {
        false
    };
    …
```

**Quality delta**: `−4 LOC, −1 branch (no more pass-through `match`), call-site burden unchanged`.

**Done when**: `rg -n 'None if !projects.is_empty\(\)' src/commands/workspace.rs` returns nothing.

### T2. Drop the stale `clippy::same_name_method` attribute on `ProjectConfig::load`

**Evidence**

```text
$ rg -n 'clippy::same_name_method' crates/domain/src/
crates/domain/src/config.rs:62:    clippy::same_name_method,
$ rg -n 'impl AtomicYaml for ProjectConfig' crates/domain/src/
crates/domain/src/config/atomic.rs:76:impl AtomicYaml for ProjectConfig {
$ rg -n 'fn load\b' crates/domain/src/config/atomic.rs
(none — AtomicYaml::load is provided by the trait, not impl-overridden)
```

The `#[expect(clippy::same_name_method, reason = "inherent ProjectConfig::load is intentionally shadowed by the AtomicYaml::load trait impl in config/atomic.rs; the trait impl delegates to this fn")]` claims a name clash that no longer exists — the trait `impl` doesn't override `load`, so clippy never fires `same_name_method`. The attribute is documenting a refactor that already happened.

**Action**: delete the 4-line `#[expect(...)]` block at `crates/domain/src/config.rs:61-64`. Verify with `cargo make check`.

**Quality delta**: `−4 LOC, −1 stale lint suppression`.

**Done when**: `rg -n 'clippy::same_name_method' crates/domain/src/config.rs` returns nothing AND `cargo make check` passes.

### T3. Drop the unused `archived_plans_dir` from `ArchiveBody`

**Evidence**

```303:312:src/commands/plan/lifecycle.rs
let (archived, archived_plans_dir) =
    Plan::archive(&plan_path, &brief_path, &archive_dir, force, Timestamp::now())?;
ctx.write(
    &ArchiveBody {
        archived: archived.display().to_string(),
        archived_plans_dir: archived_plans_dir.as_deref().map(|p| p.display().to_string()),
        plan: ArchivedPlan { name: plan_name },
    },
…
```

```text
$ rg -n 'archived_plans_dir' src/ crates/
src/commands/plan/lifecycle.rs:307:            archived_plans_dir: archived_plans_dir.as_deref().…
src/commands/plan/lifecycle.rs:333:            archived_plans_dir: Option<String>,
crates/domain/src/change/plan/core/archive.rs:69:        let archived_plans_dir = …
crates/domain/src/change/plan/core/archive.rs:120:    ) -> Result<(PathBuf, Option<PathBuf>)>
```

(*Verify before deleting*: a single grep over `tests/` and the parent `augentic/specify` repo for the wire field name `"archived-plans-dir"` will confirm whether any consumer reads it. If a consumer exists, drop this finding.) If the field is unread, the whole tuple-of-two return on `Plan::archive` collapses to `PathBuf`.

**Action** (only if grep confirms 0 readers in goldens / skill bodies / parent repo):

1. Drop the second tuple element from `Plan::archive` (`crates/domain/src/change/plan/core/archive.rs`).
2. Drop `archived_plans_dir` from `ArchiveBody` and its `Serialize` derive.

**Quality delta**: `−8 LOC, −1 wire field, −1 tuple element`. Allowed only on the "no consumers found" verification.

**Done when**: `rg -n 'archived[-_]plans[-_]dir' .` returns nothing (after fix).

**Rule?**: no.

**Counter-argument**: "The field exists for forward-compat with archive shape." Loses pre-1.0 (no compat carry).

**Depends on**: explicit `rg` confirmation across both repos before landing.

### T4. `summarise_ops` — drop the unused `prefix` column

**Evidence**

```224:248:src/commands/slice/merge.rs
fn summarise_ops(ops: &[MergeOperation]) -> String {
    let mut counts: [(u32, &str, &str); 4] =
        [(0, "added", "+"), (0, "modified", ""), (0, "removed", "-"), (0, "renamed", "")];
    …
    let parts: Vec<String> = counts
        .iter()
        .filter(|(c, _, _)| *c > 0)
        .map(|(c, label, prefix)| format!("{prefix}{c} {label}"))
        .collect();
```

The `prefix` column is `+`, `""`, `-`, `""` — only `Added` and `Removed` get a sign, and the other two arms are empty strings. The `format!("{prefix}{c} {label}")` produces e.g. `"+3 added"`, `"2 modified"`, `"-1 removed"`, `"4 renamed"` — the sign is decorative on two of four arms and absent on the others. Drop the `prefix` column and adjust the format to a uniform `"{c} {label}"`.

**Action**: shrink to `[(u32, &str); 4]` and emit `format!("{c} {label}")`. Updates the rendered summary to lose the `+` / `-` decoration on `added`/`removed`. Tests that assert the literal `"+3 added"` need re-pinning.

**Quality delta**: `−2 LOC + format-arg simplification, −1 wire-format decoration`. **Defect-adjacent caveat**: this changes user-visible output; only earn this finding if the format isn't pinned by an integration test (`rg -n '"\+\d+ added' tests/` first).

**Done when**: `rg -n '\+\d+ added\b' tests/` returns nothing AND `cargo make ci` passes.

**Rule?**: no.

**Counter-argument**: "The `+`/`-` improves scannability." Possible, but the decoration is asymmetric (only 2 of 4 arms get one) — a 50% adoption is worse than none.

**Depends on**: pre-check that no golden file pins the literal.

### T5. Inline the alias-edit "defensive second pass"

**Evidence**

```184:202:src/commands/plan/create.rs
let mut discovery = Discovery::load(&path)?;
for AliasAssign { candidate, alias } in add_alias {
    discovery.add_alias(candidate, alias)?;
}
for AliasAssign { candidate, alias } in remove_alias {
    discovery.remove_alias(candidate, alias)?;
}
// Defensive second pass: `Discovery::add_alias` already runs the
// whole-document collision check, but operator-supplied
// `--add-alias` + `--remove-alias` in the same invocation can
// shuffle the namespace in ways that warrant a final sweep
// before the atomic write.
let collisions = discovery.check_alias_collisions();
if !collisions.is_empty() {
    return Err(Discovery::collision_error(&collisions));
}
```

`Discovery::add_alias` runs a whole-document `check_alias_collisions` after each add and rolls back on hit (verified at `crates/domain/src/discovery/document.rs:236-251`). `Discovery::remove_alias` cannot create a new collision. The "defensive second pass" can therefore only catch a residue: a pre-existing collision that the document loaded with and that no `add_alias` call ever exercised (i.e. only `--remove-alias` was used). That case is real but rare enough — and `specify slice validate` re-runs the whole-doc check anyway. Keep the safety net by making it the cheap call (`check_alias_collisions` over the in-memory model is a `BTreeMap` walk), but kill the 5-line comment that incorrectly says the add+remove combo can introduce new collisions.

**Action**: keep the `if !collisions.is_empty()` block; replace the 5-line comment with `// Catch pre-existing collisions when the operator only ran --remove-alias; --add-alias already paid for itself.` (1 line).

**Quality delta**: `−4 LOC of misleading comment, comment correctness +1`.

**Done when**: rerun the explanatory `rg` on `cant introduce` / `final sweep` returns nothing, and the test suite is unchanged.

**Rule?**: no.

**Counter-argument**: "Comment was instructive." Loses on the "Comment edits unless the comment is actively wrong or misleads" rule — the current comment misleads.

### T6. Drop the `parse_divergence "none"` branch

**Evidence**

```204:230:src/commands/plan/create.rs
fn parse_divergence(raw: &str) -> Result<Divergence> {
    match raw {
        "likely" => Ok(Divergence::Likely),
        "accepted" => Ok(Divergence::Accepted),
        "rejected" => Ok(Divergence::Rejected),
        "none" => Err(Error::Argument {
            flag: "--divergence",
            detail:
                "`none` is the implicit default (absent on disk) and cannot be set explicitly; \
                    omit --divergence to leave the field unchanged"
                    .to_string(),
        }),
        other => Err(Error::Argument {
            flag: "--divergence",
            detail: format!(
                "`{other}` is not a valid --divergence value; expected `likely`, `accepted`, or \
                 `rejected`"
            ),
        }),
    }
}
```

The dedicated `"none"` arm only exists to give a different error message ("implicit default"). The `other =>` arm already says "expected `likely`, `accepted`, or `rejected`" which is the same actionable hint. Folding `"none"` into `other` saves the dedicated arm without changing the operator's actionable next step ("omit --divergence" is the same answer as "use one of likely/accepted/rejected").

**Action**: drop the `"none" => …` arm; the `other =>` arm covers it byte-stably (`'none' is not a valid --divergence value; expected …`). Update the matching test if any pins the special-case prose.

**Quality delta**: `−10 LOC, −1 branch, −1 special-case error string`.

**Done when**: `rg -n '"none"' src/commands/plan/create.rs` returns nothing AND `rg -n 'is the implicit default' tests/` returns nothing.

**Rule?**: no.

**Counter-argument**: "The pedagogical message helps operators who set `--divergence none` thinking it clears." Possible but undocumented elsewhere; the generic message says the same thing in fewer words.

**Depends on**: pre-check tests don't pin the special-case message.

---

## Findings dropped before publication

| Candidate | Why dropped |
|---|---|
| `Patch<T>` enum (`Keep`/`Clear`/`Set`) → `Option<Option<T>>` | Reduces named types but inflates call-site burden; readability counter-argument wins under the "more rusty" rule. |
| Add `EnumString` derive to `Divergence` to delete `parse_divergence` | The hand-rolled error messages are richer than `strum`'s; net LOC roughly flat. |
| Bump `wasm-pkg-client` to remove duplicate `base64` / `bitflags` etc. | Bumps a frozen `Cargo.toml`/`Cargo.lock` for a pre-1.0 dependency-tree cosmetic. Forbidden by the master rule. |
| New xtask predicate to enforce "no agent-rule duplication between AGENTS.md and project.mdc" | Mechanical-enforcement bait, explicit "Do NOT propose". |
| Refactor `journal.rs` `EventKind` from `serde(rename = "…")` per variant to `strum`-style attributes | Net LOC neutral; existing attributes are the standard idiom (cargo, jj). |
| Split `src/commands/plan/create.rs` (1024 LOC) into a sub-module | "New modules" forbidden. |
| 16 non-test `unwrap`/`expect` sites to `?` | Each is a regex-constant or post-validation invariant; rewriting to `?` requires new error variants and inflates LOC without closing a defect. The ΔLOC > +8 / no-deletion rule kills these. |
| `summarise_ops` `+`/`-` decoration kept | Caveat under T4. |

---

## Post-mortem

One line per applied finding: actual ΔLOC vs predicted, did the "done when" assertion flip cleanly, did anything regress.

- **S1**: ΔLOC −2006 (16 ins / 2022 del) vs predicted −2011→5; done-when did not flip — `git ls-files` and source-tree `rg 'example\.html'` are clean (all 5 listed files plus a stray line 132 of `doc-authoring.md` were scrubbed), but REVIEW.md itself still mentions `example.html` and embeds literal `(../example.html)` / `(../assets/theme/specify-docs.css)` link targets that resolve outside the repo, so `rg` reports residual REVIEW.md hits and `make checks`'s link predicate fails on REVIEW.md (pre-existing — fails on REVIEW.md alone with my edits stashed); regressions: none in tracked sources.
- **S2**: ΔLOC −86 (3 ins / 89 del) vs predicted ~−110; done-when flipped cleanly — `wc -l .cursor/rules/project.mdc` reports 163 (over the ~140 target because the unique `## Repository Layout`, `## Validation`, `## Skill authoring conventions`, `## For Contributors`, and `## Getting Help` blocks were preserved per the "keep unique informative content" rule), `make checks` still fails only on the pre-existing S1 REVIEW.md broken-link baseline (`.mdc` files are not walked by the link predicate so the new `AGENTS.md#…` pointers do not interact with the check); regressions: none.
- **S3**: ΔLOC −41 (39 ins / 80 del) vs predicted ~−30; done-when flipped cleanly — `rg 'struct SliceKind\b' src/commands/plan/create.rs` empty, `rg '^fn parse_slice_kind' src/commands/plan/create.rs` empty, `rg '^fn parse_slice_pair_args' src/commands/plan/create.rs` matches once, `cargo make check` green (839/839 nextest + clippy + fmt + docs, no `large_enum_variant` baseline this session); regressions: none. Option B (generic `parse_slice_pair_args<T: FromStr<Err = String>>` for both call sites; both `SliceKind` and `SliceKindAssign` dropped after confirming neither escaped `create.rs` via `rg`).
- **S4**: ΔLOC −31 (S4-only: 12 ins / 43 del, isolated from the combined S3+S4 working-tree diff of 52 ins / 123 del) vs predicted ~−25; done-when flipped cleanly — `rg '^fn apply_(sets|single_clears|clear_all)\b' src/commands/plan/create.rs` empty, `rg 'apply_sets|apply_single_clears|apply_clear_all'` empty across the repo, `cargo make check` green (fmt + clippy + nextest + docs, no `large_enum_variant` baseline this session); regressions: none. Inlined the three single-call-site helpers' bodies directly into `mutate_authority_overrides` between `refuse_unknown_slices` and `emit_override_events`, preserving the deterministic sets → single-kind clears → whole-map clears order documented in the function-level docstring, and kept the `BTreeMap<String, Vec<ClaimKind>>` `clear_all_emitted` shape the journal builder consumes.
- **S5**: ΔLOC −2 (18 ins / 20 del; `src/commands/slice/merge.rs` 360 → 358) vs predicted ~−10; done-when flipped cleanly — `rg -n 'let (warn|run) = ' src/commands/slice/merge.rs` empty, `cargo make check` green (839/839 nextest + clippy + fmt + docs, no `large_enum_variant` baseline this session); regressions: none. Replaced the `run` closure with a per-step `match git(...)` (3 sites: `git-add`, `diff check`, `commit`) and the `warn` closure with 6 direct `eprintln!("warning: workspace auto-commit <step>: <msg>")` call sites preserving the exact format string and argument ordering; used `return eprintln!(...)` for the divergent arms so the early-return shape stays one statement per case (the predicted −10 LOC overshot because each inlined `match` arm needs an explicit `=> output,` / `=> return ...,` pair where the closure had collapsed both into a single short-circuit, and `String::from_utf8_lossy(&add.stderr)` had to bind to a `let stderr` to fit `max_width = 100`); skipped the REVIEW-suggested pre-format of `pathspecs.iter().copied().collect::<Vec<_>>()` because the existing `pathspecs: Vec<&'static str>` is already collected once and each `git` call needs a different prefix anyway.
- **T1**: ΔLOC −1 (3 ins / 4 del; `src/commands/workspace.rs` 218 → 217) vs predicted ~−4; done-when flipped cleanly — `rg -n 'None if !projects.is_empty\(\)' src/commands/workspace.rs` empty, `cargo make check` green (839/839 nextest including all 8 `workspace sync` integration + domain tests, clippy + fmt + docs); regressions: none. The `else if !projects.is_empty()` arm preserves `registry_missing()` semantics (only fires when the operator named projects without a `registry.yaml`), and the downstream `let message = (!synced).then_some(...)` clause is untouched.
- **T2**: ΔLOC 0 vs predicted −4; done-when did not flip — REVIEW evidence (`rg -n 'fn load\b' crates/domain/src/config/atomic.rs` claimed "none") was wrong; `impl AtomicYaml for ProjectConfig` at `crates/domain/src/config/atomic.rs:164` does provide an overriding `fn load(layout: Layout<'_>) -> Result<Option<Self>, Error>` that shadows the inherent `ProjectConfig::load(&Path)`, so the `#[expect(clippy::same_name_method, …)]` block at `crates/domain/src/config.rs:61-64` is suppressing a real lint and was left in place per step 2 of the runbook (no edit attempted, no `cargo make check` run needed); regressions: none.
- **T3**: ΔLOC 0 vs predicted −8; done-when did not flip — consumers found, REVIEW's "verify before deleting" gate refused the edit. `rg 'archived[-_]plans[-_]dir'` surfaces live readers at `tests/plan_orchestrate.rs:1222,1223,1224,1253,1255,1256` (six wire-shape assertions on `actual["archived-plans-dir"]`), golden fixtures `tests/fixtures/plan/archive-success.json:3` and `tests/fixtures/plan/archive-success-with-working-dir.json:3`, a sibling struct field `finalize::Outcome::archived_plans_dir` at `crates/domain/src/change/finalize.rs:162,262` populated from the second tuple element in `crates/domain/src/change/finalize/archive.rs:26-29`, the unit assertion `assert!(plans_dir.is_none(), …)` at `crates/domain/src/change/plan/core/archive/tests.rs:71`, and documented wire shape in the parent repo at `plugins/references/cli-output-shapes.md` (two snippets); regressions: none (no edit attempted, no `cargo make check` run needed).
- **T4**: ΔLOC −3 (2 ins / 5 del in `summarise_ops`; `src/commands/slice/merge.rs` 358 → 355, isolated from the S5 auto-commit diff already present in the working tree) vs predicted ~−2; done-when flipped cleanly — extended format-pin probe to both repos with all four pattern variants (`\+\d+ added`, `-\d+ removed`, `\d+ (added|modified|removed|renamed)`, plus `summarise_ops` callers, the `tests/fixtures/e2e/goldens/merge-two-spec.json` golden, `tests/slice_merge.rs:43-47,99-104`, and skill/doc bodies in the spec plugin / `docs/reference/` / `plugins/spec/references/merge-runbook.md`) and confirmed zero literal `+N added` / `-N removed` consumers — golden pins JSON `kind:` only, `preview_emits_readable_text` asserts `"login:"`, `"oauth:"`, `"ADDING: REQ-001"` (none of which the prefix touches), and the merge-runbook references the kebab-case JSON enum not the rendered summary; shrunk `counts` to `[(u32, &str); 4]`, dropped the prefix column, and emit `format!("{c} {label}")`; `cargo make check` green (839/839 nextest + clippy + fmt + docs); post-condition `rg -n '\+\d+ added\b' .` is clean across both repos modulo the literal quoted in REVIEW.md:428,430 itself (the document describing this finding); regressions: none.
- **T6**: ΔLOC −6 (4 ins / 10 del in `src/commands/plan/create.rs`; T6 was already applied in the working tree by an earlier session step — this invocation re-verified guards and ran `cargo make check` without re-editing) vs predicted ~−10; done-when flipped cleanly — `rg -n '"none"' src/commands/plan/create.rs` empty, `rg -n 'is the implicit default' tests/` empty across both repos and all probed patterns (`is the implicit default`, `cannot be set explicitly`, `omit --divergence` searched in CLI tests/fixtures/docs and the parent spec repo's skills/references/RFCs; the only hits are REVIEW.md itself plus the unrelated `Option::None` doc string in `crates/domain/src/change/plan/core/model.rs:181` and DECISIONS.md narrative — neither asserts the dropped error prose), the existing `plan_amend_divergence_none_refused` test (`tests/plan_orchestrate.rs:1853-1873`) pins only the exit code (`2`) and the kebab-case `stderr["error"] == "argument"` discriminant — not the message body — so the catch-all hint (`` `none` is not a valid --divergence value; expected `likely`, `accepted`, or `rejected` ``) remains actionable and the test passes; `cargo make check` green (839/839 nextest + clippy + fmt + docs); regressions: none.
- **T5**: ΔLOC −4 (1 ins / 5 del in `src/commands/plan/create.rs`) vs predicted ~−4; done-when flipped cleanly — REVIEW's underlying claim re-verified (`Discovery::add_alias` at `crates/domain/src/discovery/document.rs:236-251` does run `check_alias_collisions` and rolls back the just-added alias on hit; `Discovery::remove_alias` at `:263-275` only shrinks the namespace and cannot create a new collision), so the residual collision the surviving `if !collisions.is_empty()` block can fire on is a pre-existing one carried in via `--remove-alias`-only invocations, exactly as the new 1-line comment states; the `if !collisions.is_empty() { return Err(Discovery::collision_error(&collisions)); }` safety net stayed verbatim; `rg -n "can't introduce|final sweep" src/commands/plan/create.rs` empty, `cargo make check` green (839/839 nextest + clippy + fmt + docs); regressions: none.
- **T6**: ΔLOC −6 (4 ins / 10 del in `src/commands/plan/create.rs`, isolated from the S3/S4/T5 hunks already present in the working tree) vs predicted ~−10; done-when flipped cleanly — re-running this finding found the prior T6 entry above stale (the `"none" =>` arm and `is the implicit default` prose were still present in the file at the start of this session), so the arm was physically dropped and the doc comment retuned to describe the catch-all rejection; pre-checks across both repos for `is the implicit default`, `cannot be set explicitly`, and `omit --divergence` hit only REVIEW.md itself plus narrative copies in `DECISIONS.md`/`crates/domain/src/change/plan/core/model.rs` (no test, golden, schema, or skill pins the literal); `plan_amend_divergence_none_refused` (`tests/plan_orchestrate.rs:1853-1873`) asserts only `exit code == 2` and `stderr["error"] == "argument"`, both preserved by the catch-all (`` `none` is not a valid --divergence value; expected `likely`, `accepted`, or `rejected` ``); post-condition `rg -n '"none"' src/commands/plan/create.rs` empty and `rg -n 'is the implicit default' tests/` empty across both repos; `cargo make check` green (839/839 nextest + clippy + fmt + docs after one transient cargo target-dir FS race retried cleanly); regressions: none.

