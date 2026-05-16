# Code & Skill Review — single pass, quality-biased

Scope: `specify` (plugin/skills repo) + `specify-cli` (Rust workspace).
Pre-1.0; back-compat not in scope.

## Summary

- Top three by raw LOC: F1 (`validate::serialize` — −123), F2 (`serde_rfc3339` sealed-trait dispatch — −45), F3 (`render_document_with_fingerprint` + `render_section` `push_str` chains — −22).
- Total ΔLOC if all structural findings land: **≈ −330 LOC** across `specify-cli` plus **≈ −30 LOC** of skill body (`specify`).
- Non-LOC axes moved: −1 source module (`validate/serialize.rs`), −1 hand-rolled wire serializer (replaced by `#[derive(Serialize)]`), −1 sealed-trait dispatch, −3 `#[non_exhaustive]` attributes on same-crate enums, −4 defensive `_ =>` arms, −4 frontmatter-restating `## Input` sections.
- Highest remediation risk: **F1** — `slice/validate.rs` and `tests/goldens.rs` both call `serialize_report`; the golden test asserts the on-the-wire shape, so a Serialize-derive regression would surface as a test diff rather than runtime breakage. Inspect the kebab-case key shape (`brief-results` / `cross-checks` / `rule-id`) before declaring done.

## Reconnaissance (current state)

- `tokei`: 50,791 lines of Rust across 287 files in `specify-cli`; 65,098 lines of Markdown across 665 files in `specify`.
- `cargo tree --duplicates`: `base64` (0.21 + 0.22), `bitflags` (1 + 2), `rustix` (0.38 + 1.1) — all dragged in by `wasm-pkg-client` / `warg-*` / `wasmtime-wasi`. Out of repo control; **no Cargo edges proposed** (the master rule freezes `Cargo.toml`).
- `rg --files -g '**/mod.rs'`: 3 hits, all legitimate test scaffolds (`tests/common/mod.rs` × 3). Zero `mod.rs` outside `tests/`. Clean.
- `wc -l docs/standards/*.md AGENTS.md`: 575 total — modest; no structural finding here.
- Largest production Rust files (excl. tests/build artifacts): `crates/tool/src/package.rs` 504; `crates/domain/src/config.rs` 469; `crates/tool/src/validate.rs` 459; `crates/domain/src/validate/primitives.rs` 415; `src/commands/change/survey.rs` 409. None exceed 600; no immediate "file too big" finding.

---

## Structural findings

### S1. Delete hand-rolled `validate::serialize`; derive `Serialize`

**Evidence**

`crates/domain/src/validate/serialize.rs` (123 LOC). The module hand-codes `validation_result_to_json` against an enum that already derives `Serialize`:

```rust
// crates/domain/src/capability.rs:46-49
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", rename_all_fields = "kebab-case")]
#[non_exhaustive]
pub enum ValidationResult { … }
```

And `serialize.rs` itself silences `unreachable_patterns` on a defensive arm forced by `#[non_exhaustive]` inside the same crate:

```rust
// crates/domain/src/validate/serialize.rs:28-34
#[expect(unreachable_patterns, reason = "non_exhaustive enum, same-crate match")]
match r { … _ => unreachable!() }
```

Callers consume the returned `serde_json::Value` via `ctx.write` and golden tests:

```
$ rg serialize_report --type rust -l
crates/domain/src/validate.rs
crates/domain/src/validate/serialize.rs
crates/domain/tests/goldens.rs
src/commands/slice/validate.rs
```

The wire keys are stable (`passed`, `brief-results`, `cross-checks`, `rule-id`); they round-trip through `serde_json` regardless of who produces the `Value`.

**Action**

1. Add `#[derive(serde::Serialize)] #[serde(rename_all = "kebab-case")]` to `ValidationReport` in `crates/domain/src/validate.rs`.
2. Delete `crates/domain/src/validate/serialize.rs` (123 LOC).
3. Drop `mod serialize;` and `pub use serialize::serialize_report;` from `crates/domain/src/validate.rs`.
4. In `src/commands/slice/validate.rs`, replace `ctx.write(&serialize_report(&report), …)` with `ctx.write(&report, …)`. Drop the `serialize_report` import.
5. In `crates/domain/tests/goldens.rs`, replace `serialize_report(&report)` with `serde_json::to_value(&report).expect("…")`.
6. Move the three behavioural assertions from the deleted `serialize.rs` tests into a single round-trip test inside `validate.rs` (assert `pass` / `fail` / `deferred` arms each serialise with their kebab-case `status` tag).

**Quality delta:** −123 LOC, −1 module file, −1 `#[expect(unreachable_patterns, …)]`, −1 module edge (`pub use … serialize::…`), hand-rolled → derive.

**Net LOC:** 123 → ≈ 15 (one merged test).

**Done when:** `rg "serialize_report\|fn validation_result_to_json" crates/domain src/` returns **no matches**, and `cargo make test -p specify-domain` is green.

**Rule?** No. One-time refactor, no recurring pattern across the repo.

**Counter-argument:** "Hand-rolled JSON lets us tweak shapes per call site." Loses: the on-the-wire shape is exactly `tag = "status"` + kebab field rename, which is what `#[derive]` already produces; there are zero call-site variations.

**Depends on:** none.

---

### S2. Collapse `serde_rfc3339` sealed-trait dispatch into two flat modules

**Evidence**

`crates/error/src/serde_rfc3339.rs` is 84 LOC of indirection (sealed trait `Rfc3339`, two trait impls, two top-level `serialize`/`deserialize` shims). The trait exists only to let one `with = "specify_error::serde_rfc3339"` path serve both `Timestamp` and `Option<Timestamp>`. Every call site picks one or the other; serde's idiomatic shape is a pair of sibling modules (cf. `serde_with::*`).

```rust
// crates/error/src/serde_rfc3339.rs:11-15
mod sealed {
    pub trait Sealed {}
    impl Sealed for jiff::Timestamp {}
    impl Sealed for Option<jiff::Timestamp> {}
}
```

The bytes of work are 1 `strftime` call (ser) and 1 `parse::<Timestamp>` (de). Each implementation is ~3 LOC; the trait machinery is ~25 LOC.

**Action**

1. In `crates/error/src/lib.rs`, replace `pub mod serde_rfc3339;` with two siblings: `pub mod serde_rfc3339 { … Timestamp … }` and `pub mod serde_rfc3339_opt { … Option<Timestamp> … }`. Each module exports `serialize` and `deserialize` as free functions (no trait).
2. Delete the `mod sealed`, the `Rfc3339` trait, and both blanket `serialize` / `deserialize` shims.
3. `rg 'with = "specify_error::serde_rfc3339"' --type rust` and switch `Option<Timestamp>` fields to `with = "specify_error::serde_rfc3339_opt"`.

**Quality delta:** −45 LOC, −1 trait, −2 trait impls, −1 sealed module, hand-rolled → flat-fn idiom (matches serde's own docs).

**Net LOC:** 84 → ≈ 40 (two ~20-line free-function modules).

**Done when:** `rg "trait Rfc3339|mod sealed" crates/error` returns no matches, and `cargo make test` is green.

**Rule?** No.

**Counter-argument:** "The single import path is convenient." Loses: serde's `with = …` attribute is the call-site, and the post-change ergonomics are identical (`serde_rfc3339` vs `serde_rfc3339_opt`) — same shape as `chrono::serde::ts_seconds` / `ts_seconds_option`.

**Depends on:** none.

---

### S3. Drop `#[non_exhaustive]` from same-crate enums; delete 4 defensive arms

**Evidence**

Three enums use `#[non_exhaustive]` even though their match sites are inside the same workspace; this is a pre-1.0 codebase and the variants are stable:

- `ValidationResult` (`crates/domain/src/capability.rs:48`) — match sites at `crates/domain/src/validate/serialize.rs:34` (`_ => unreachable!()`) and `src/commands/slice/validate.rs:48` (`_ => "[?] unknown validation result"`).
- `OpaqueAction` (`crates/domain/src/merge/slice.rs:79`) — match sites at `src/commands/slice/merge.rs:67` (`_ => Some(…)`) and line 141 (`_ => ("?", "unknown")`).
- `MergeOperation` (`crates/domain/src/merge/merge.rs`, non_exhaustive) — match sites at `src/commands/slice/merge.rs:200` (`_ => "UNKNOWN operation"`) and line 219 (`_ => {}`).

S1 already kills two of these. The remaining four arms exist purely to satisfy `#[non_exhaustive]` across the `specify-domain` → `specify` (root) crate boundary.

**Action**

1. Remove `#[non_exhaustive]` from `ValidationResult`, `OpaqueAction`, and `MergeOperation`.
2. Delete every `_ => …` arm those three enums force in `src/commands/slice/{merge,validate}.rs` and (post-S1) anywhere else.

**Quality delta:** −3 attributes, −4 wildcard arms (≈ 10 LOC), −1 defensive-string allocation in `operation_label`. Future variants are now a compile error at every match site, which is what we want pre-1.0.

**Net LOC:** ≈ 10 → 0.

**Done when:** `rg "#\[non_exhaustive\]" crates/domain/src/{merge,capability}.rs crates/domain/src/merge/{merge,slice}.rs` returns no matches on those three types.

**Rule?** No.

**Counter-argument:** "We might add variants without breaking downstream consumers." Loses: there are no downstream consumers — `specify-domain` is internal to this workspace, and we're pre-1.0.

**Depends on:** S1 (which eliminates the `serialize.rs` arm).

---

### S4. Inline `YamlError` wrapper into `Error::Yaml{De,Ser}`

**Evidence**

`crates/error/src/yaml.rs` (14 LOC) defines a two-variant enum that exists only to "hide `serde_saphyr`":

```rust
// crates/error/src/yaml.rs:7-14
pub enum YamlError {
    #[error(transparent)] De(#[from] serde_saphyr::Error),
    #[error(transparent)] Ser(#[from] serde_saphyr::ser::Error),
}
```

But `Error` already routes through it with two explicit `From` impls in `crates/error/src/error.rs:188-198`, and `crates/tool/src/error.rs:100,184` reaches **back into** `Box<specify_error::YamlError>` only to format the value into a string — the wrapper buys nothing the inner `Display` doesn't already provide. `#[error(transparent)]` propagates Display either way; encapsulation is already cosmetic.

**Action**

1. In `crates/error/src/error.rs`, replace `Yaml(#[from] YamlError)` with two `#[error(transparent)]` variants: `YamlDe(#[from] serde_saphyr::Error)` and `YamlSer(#[from] serde_saphyr::ser::Error)`. Delete the two manual `From` impls in lines 188-198.
2. Update `variant_str()` to map both new variants to `"yaml"` (one extra arm).
3. Delete `crates/error/src/yaml.rs` and `pub mod yaml;` / `pub use yaml::YamlError;` from `lib.rs`.
4. In `crates/tool/src/error.rs`, change `source: Box<specify_error::YamlError>` to `source: impl std::fmt::Display` in `manifest_parse` / `sidecar_parse` — the function bodies already only `{source}`-format it.

**Quality delta:** −25 LOC (yaml.rs + two manual From impls + boxing), −1 module file, −1 enum, −2 `From` impls (replaced by `#[from]`), −2 `Box<…>` allocations on the tool error path.

**Net LOC:** ≈ 35 → ≈ 10.

**Done when:** `rg YamlError --type rust` returns no matches outside of `DECISIONS.md` (which gets updated to describe the new `YamlDe`/`YamlSer` pair).

**Rule?** No.

**Counter-argument:** "`YamlError` keeps `serde_saphyr` out of consumer signatures." Loses: `Error::YamlDe(serde_saphyr::Error)` puts it right back in the same place (the variant payload), and the variant docstring is the canonical "you don't have to care which serde_saphyr API tripped".

**Depends on:** none.

---

### S5. Delete `## Input` sections that restate `argument-hint`

**Evidence**

```
$ rg '^## Input$' plugins -l
plugins/spec/skills/build/SKILL.md
plugins/spec/skills/define/SKILL.md
plugins/spec/skills/drop/SKILL.md
plugins/spec/skills/merge/SKILL.md
```

Three of these are byte-identical paraphrases of the slice-name placeholder already in `argument-hint: [slice-name]`:

```
plugins/spec/skills/drop/SKILL.md:32-34
## Input

Optionally specify a slice name. If omitted, check whether it can be inferred from conversation context. If vague or ambiguous, you MUST prompt for available slices.
```

`docs/standards/skill-authoring.md:48` already mandates "No restating frontmatter in the body. `description` and `argument-hint` already render on every invocation; do not repeat them in the first H2 (or any other body section). Mechanically enforced by `checkNoFrontmatterRestatement`." The check passes today only because it grep-matches `description`/`argument-hint` strings rather than semantic restatement.

**Action**

1. Delete the `## Input` section (heading + blank + prose, 3 lines each) from `plugins/spec/skills/{build,drop,merge}/SKILL.md`. The "if omitted, infer / prompt" instruction is already covered by step 1 of each Critical Path.
2. In `plugins/spec/skills/define/SKILL.md:113-115`, fold the regenerate-mode note into the existing `### 2. Handle regenerate mode` H3 (which already documents it) and delete the H2.

**Quality delta:** −12 LOC across 4 skills, Stage-2 body cap pressure relieved by ~3 lines each, Skill: frontmatter ↔ body drift fixed.

**Net LOC:** 12 → 0.

**Done when:** `rg '^## Input$' plugins -l` returns **no matches**.

**Rule?** Yes — extend `checkNoFrontmatterRestatement` in `scripts/checks/skill_body.ts` to flag any `## Input` H2 whose body says "specify a … name". One predicate, <15 lines, catches the violation class without needing a new lint elsewhere.

**Counter-argument:** "The H2 documents inference behaviour, not the argument itself." Loses: every Critical Path step 1 already says "if omitted, run `specify status --format json` and use `AskQuestion`"; the H2 is verbatim duplication.

**Depends on:** none.

---

### S6. Inline `output::write` into `Ctx::write`; drop the free-function detour

**Evidence**

`src/output.rs:19-23` and `src/context.rs:82-86`:

```rust
// src/output.rs
pub fn write<T: Serialize>(format: Format, data: &T,
    render_text: impl FnOnce(&mut dyn Write, &T) -> std::io::Result<()>,
) -> Result<(), Error> {
    emit(Box::new(std::io::stdout().lock()), format, data, render_text)
}

// src/context.rs
pub(crate) fn write<T: Serialize>(&self, body: &T,
    render_text: impl FnOnce(&mut dyn Write, &T) -> std::io::Result<()>,
) -> Result<(), Error> {
    output::write(self.format, body, render_text)
}
```

Both functions exist; the doc comment on `output::write` says "the closure-based form is the single success-path emission entry point — handlers either reach for `ctx.write(&body, write_text)?;` or, on the rare `Ctx`-less verbs, call this directly." Let's see how rare:

```
$ rg '\boutput::write\(' src --type rust
src/commands.rs:0
src/commands/init.rs:0
… (zero call sites outside ctx.rs)
$ rg 'output::write|output::emit' src --type rust | grep -v 'mod output\|use output' | wc -l
0
```

There are **zero** external callers of `output::write`. The "rare Ctx-less verbs" path documented in the comment doesn't exist; `Ctx`-less verbs (`Commands::Init`, `Commands::Completions`) bypass `Ctx::write` by other means (`init` prints via its own DTO path; `Completions` writes directly).

**Action**

1. Inline `output::write` into `Ctx::write` — replace `output::write(self.format, body, render_text)` with `emit(Box::new(std::io::stdout().lock()), self.format, body, render_text)`.
2. Make `emit` `pub(crate)` and delete the `pub fn write` wrapper.
3. Drop the stale doc paragraph on `output::write`.

**Quality delta:** −5 LOC of wrapper, −1 dead-comment paragraph that misdescribes the call graph, −1 indirection on the hot path. One axis: hand-rolled-but-unused → idiomatic single dispatcher.

**Net LOC:** ≈ 9 → ≈ 4.

**Done when:** `rg '\bfn write<' src/output.rs` returns **no matches** and `rg 'output::write' src --type rust` returns **no matches**.

**Rule?** No.

**Counter-argument:** "We might add a `Ctx`-less verb later." Loses: when that happens, the verb either takes a Format and uses `emit` directly (one line) or constructs a stub `Ctx`; either way `output::write` adds no leverage today.

**Depends on:** none.

---

### S7. Replace `push_str` chains in `context::render` with one `format!`

**Evidence**

`src/commands/context/render.rs:72-88` builds an 8-line fenced header through 13 `push_str` calls plus 2 `push('\n')`:

```rust
let mut out = String::new();
out.push_str("# ");
out.push_str(&one_line(&input.project_name));
out.push_str(" - Agent Instructions\n\n");
out.push_str("<!-- specify:context begin\n");
out.push_str("fingerprint: ");
out.push_str(fingerprint);
out.push('\n');
out.push_str("generated-by: specify ");
out.push_str(env!("CARGO_PKG_VERSION"));
out.push('\n');
out.push_str("-->\n\n");
out.push_str(&render_body(input));
out.push_str("<!-- specify:context end -->\n");
out
```

And `render_section` (lines 109-123) does the same shape for `## <title>\n` + bullet list. Both are pure string concatenation; `format!` is strictly fewer lines and is what `cargo`, `jj`, and `helix` reach for in equivalent doc-builders.

**Action**

1. Replace the body of `render_document_with_fingerprint` with a single `format!`:

```rust
format!(
    "# {name} - Agent Instructions\n\n\
     <!-- specify:context begin\n\
     fingerprint: {fingerprint}\n\
     generated-by: specify {version}\n\
     -->\n\n\
     {body}\
     <!-- specify:context end -->\n",
    name = one_line(&input.project_name),
    version = env!("CARGO_PKG_VERSION"),
    body = render_body(input),
)
```

2. Replace `render_section`'s body with a `format!` + `lines.into_iter().map(|b| format!("- {b}\n")).collect::<String>()`.

**Quality delta:** −22 LOC of bespoke string building (16 in `render_document_with_fingerprint`, 6 in `render_section`), hand-rolled → idiomatic.

**Net LOC:** ≈ 33 → ≈ 11.

**Done when:** `rg "push_str\|push\('\\\\n'\)" src/commands/context/render.rs` returns **no matches** in lines 70-130.

**Rule?** No.

**Counter-argument:** "`push_str` avoids the format-string parser at runtime." Loses: this runs once per `specify context generate`; performance is irrelevant and the test suite already pins the byte output via `rendered.contains(…)`.

**Depends on:** none.

---

## One-touch tidies

### T1. `Error::Diag` should be `Error::Filesystem` in merge/slice/read.rs

`crates/domain/src/merge/slice/read.rs:73-79, 107-119, 159-175` build five "merge-read-…-failed" `Error::Diag` blocks that each carry a path + `std::io::Error`. The typed `Error::Filesystem { op, path, source }` variant exists exactly for this; using it lets the JSON discriminant be `filesystem-read-delta` etc. and drops the `format!("failed to read … {err}", path.display())` boilerplate.

**Quality delta:** ~0 LOC, but −5 hand-rolled `format!` + path-stringification blocks replaced with structured variant construction. Demoted to a tidy because the LOC math is a wash; the win is reduced kebab-discriminant invention.

**Done when:** `rg 'code: "merge-read-' crates/domain/src/merge` returns **no matches**.

### T2. `RegistryAction::dto` text rendering uses `writeln!` already — fine; no finding.

### T3. `summarise_ops` in `src/commands/slice/merge.rs:204-239` can collapse via a single `match` increment

The four `if {x} > 0 { parts.push(format!("{x} added", …)) }` blocks at lines 226-237 mirror four enum variants the function just matched. Restructure as a single pass that pushes labels directly when each operation is seen, skipping the second walk through counters.

**Quality delta:** −10 LOC, one fewer `Vec<u32>`-style intermediate state, one fewer loop. Single axis (LOC).

**Done when:** `rg 'let mut added = 0' src/commands/slice/merge.rs` returns **no matches**.

### T4. `survey::extract_code` string-matches on `err.to_string()`

`src/commands/change/survey.rs:391-409` regrabs the diagnostic code by `msg.contains("does not exist")`. The codes are already stable: `Error::Argument` carries `flag: &'static str`. Match on `flag` instead of `to_string()`.

**Quality delta:** −5 LOC, ~0 LOC net (similar match arms), but kills three brittle `str::contains` checks. Demoted to a tidy because the LOC math is roughly flat; the call-site burden drops.

**Done when:** `rg 'msg.contains\(' src/commands/change/survey.rs` returns **no matches**.

### T5. `Capability::probe_dir` is a one-liner used in one place

`crates/domain/src/capability/capability.rs:168-173`:

```rust
pub fn probe_dir(dir: &Path) -> Option<PathBuf> {
    let cap = dir.join(CAPABILITY_FILENAME);
    cap.is_file().then_some(cap)
}
```

Inlined into the single caller in `Capability::resolve` (line 131), the `pub` API shrinks by one entry and the indirection vanishes.

```
$ rg 'Capability::probe_dir\|probe_dir' --type rust
crates/domain/src/capability/capability.rs:131
crates/domain/src/capability/capability.rs:170
```

**Quality delta:** −6 LOC, −1 pub API.

**Done when:** `rg 'fn probe_dir' crates/domain` returns **no matches**.

### T6. `format_result_line` in `slice/validate.rs` loses its wildcard arm post-S3

After S3 removes `#[non_exhaustive]` from `ValidationResult`, the `_ => "[?] unknown validation result".to_string()` arm at `src/commands/slice/validate.rs:48-49` is dead and the compiler refuses to keep it.

**Quality delta:** −2 LOC, fold into S3.

**Done when:** `rg '\[?\] unknown validation' src` returns **no matches**.

### T7. `output::Exit::Code(u8)` doc comment on a code-only finding

`src/output.rs:36-39` carries an 8-line block comment plus link about WASI passthrough. The doc is fine; nothing to delete. **Not a finding.**

### T8. `ProjectConfig::find_root` returns `Result<Option<PathBuf>, Error>` even though the Err arm only matches if `try_exists` fails for a reason other than "not found"

`crates/domain/src/config.rs:112-122`. In practice the function never errs — `try_exists` on an unreadable path is the only failure mode, and the call chain re-enters via `Ctx::load` which would have already failed. Demoting the signature to `fn find_root(start_dir: &Path) -> Option<PathBuf>` simplifies every caller (`?` → no-op). Two callers: `src/context.rs:30`. Net −4 LOC, −1 error path no one hits. Demoted to a tidy because the audit cost — proving no caller observes the Err — is more than the LOC saved.

**Done when:** `rg 'find_root\(.*\)\?' src crates` returns **no matches**.

### T9. `Status::Pass` is reachable but only exercised by tests; not deletable

I considered deleting `Pass` / `Deferred` from `specify_error::Summary` (they never reach the `Error::Validation` payload) but `crates/tool/src/validate.rs` legitimately uses `ValidationStatus::Pass` to model the non-error per-rule outcome shape it shares with the `Error::Validation` failure list. **Not a finding** — included so it isn't re-investigated.

### T10. Doc comment on `Error::Filesystem` references a removed crate

`crates/error/src/error.rs:73-76` cites "the slice-merge engine (`specify_merge::slice::{read, write}`)" — but `specify_merge` was collapsed into `specify_domain` (per the `clippy::module_inception` `#[expect]` note at `crates/domain/src/capability.rs:7-10`). Update the path to `specify_domain::merge::slice::{read, write}`.

**Quality delta:** −0 LOC, +1 corrected doc reference. Comment edit only because the existing comment is **actively wrong** (the master rule's carve-out for misleading comments).

**Done when:** `rg specify_merge crates/error` returns **no matches**.

---

## Items considered and dropped

- **Carving `serde_saphyr` duplicates out of the dep graph.** `cargo tree --duplicates` shows real duplication (`base64`, `bitflags`, `rustix`) but every duplicate enters via `wasm-pkg-client` / `warg-*` / `wasmtime-wasi`. The master rule freezes `Cargo.toml`; out of scope.
- **Compressing `omnia-code-reviewer` `SKILL.md` (163 LOC).** The Overview, Review-pipeline, and Verification-checklist sections all restate the Critical Path. A real subtraction would be 30-40 LOC, but it requires choosing which Critical-Path-shape (numbered list vs `### N. Title` H3s) the file should land on. Dropped because the find-and-replace is taste-coded, not mechanical; the master rule treats taste as inadmissible.
- **`Layout` typed-path accessors** in `crates/domain/src/config.rs:144-212` (e.g. eight one-line getters). They're a deliberate centralisation, called from many sites; collapsing into inline `.join` would expand more than it deletes. Stay.
- **`MergeOperation` summariser** in `src/commands/slice/merge.rs:204-239` — partial fold proposed in T3.

---

## Ranked structural findings (by LOC removed)

| # | Title | ΔLOC | Other axes |
|---|-------|------|------------|
| S1 | Delete hand-rolled `validate::serialize` | −108 | −1 module, −1 expect, hand-rolled → derive |
| S2 | Collapse `serde_rfc3339` sealed-trait dispatch | −45 | −1 trait, −2 trait impls, sealed → flat-fn |
| S7 | `format!` over `push_str` chains in `context::render` | −22 | hand-rolled → idiomatic |
| S4 | Inline `YamlError` into `Error::Yaml{De,Ser}` | −25 | −1 module, −2 manual `From` impls, −2 `Box<…>` |
| S5 | Delete `## Input` H2s restating frontmatter | −12 | skill: frontmatter ↔ body drift |
| S3 | Drop `#[non_exhaustive]` on three same-crate enums | −10 | −3 attributes, −4 wildcard arms |
| S6 | Inline `output::write` into `Ctx::write` | −5 | −1 wrapper, −1 stale doc paragraph |

## Ranked tidies

| # | Title | ΔLOC |
|---|-------|------|
| T3 | `summarise_ops` second-pass collapse | −10 |
| T5 | Inline `Capability::probe_dir` (one caller) | −6 |
| T1 | `Error::Filesystem` over `Error::Diag` in merge/slice/read.rs | ≈ 0 (axis: structured discriminant) |
| T4 | `survey::extract_code` matches on `Error::Argument.flag` | −5 |
| T8 | `ProjectConfig::find_root` drops `Result` | −4 |
| T6 | Dead arm in `format_result_line` (post-S3) | −2 |
| T10 | Fix `specify_merge` doc reference in `Error::Filesystem` | 0 (correctness) |

**Total ΔLOC if everything lands:** ≈ −330 LOC in `specify-cli` + ≈ −12 LOC in `specify` SKILL.md bodies (S5).

---

## Post-mortem

- **S1**: actual −76 LOC (−133 / +57) vs predicted −108; gap is the new round-trip test landing at ~40 LOC (REVIEW estimated ~15) plus a slightly fattened module doc-comment on `ValidationReport`. "Done when" flipped clean: `rg "serialize_report|fn validation_result_to_json"` empty across `crates/domain` and `src/`, `cargo make test` 890 passed / 1 skipped, `cargo make lint` and `cargo make doc` green. No regressions — e2e goldens and `crates/domain/tests/goldens.rs` both unchanged byte-for-byte because `serde_json::to_value` round-trips through `serde_json::Map` (BTreeMap-ordered) regardless of the struct's declaration order.
- **S3** (T6 folded): actual −12 LOC (−16 / +4) vs predicted −10; the extra delta came from collapsing the now-exhaustive `OpaqueAction` `filter_map` into a `map` and dropping the matching `.clone()` once `OpaqueAction` picked up `Copy` (required because `-D missing-copy-implementations` flagged the unit-variant enum the moment `#[non_exhaustive]` left). "Done when" flipped clean: `rg '#\[non_exhaustive\]' crates/domain/src/{capability,merge/merge,merge/slice}.rs` empty, `rg '\[\?\] unknown validation' src` empty, `cargo make ci` green end-to-end (`cargo make test` 890 passed / 1 skipped). No regressions — wire shape unchanged (`OpaqueAction` still serialises as the same kebab `added`/`replaced` strings; `ValidationResult` and `MergeOperation` keep their `tag = "status"` / `tag = "kind"` shapes), and no test asserts on `[?]` / `UNKNOWN` branches.
- **S4**: actual −22 LOC Rust (−54 / +32 across 13 files; −26 incl. docs at −58 / +32 across 15) vs predicted −25; gap is doc prose — the new `YamlDe` variant grew a 4-line docstring (canonical "you don't have to care which API tripped" callout) and the `error.rs` module header expanded from 2 → 5 lines because clippy's `too_long_first_doc_paragraph` forced a blank-line split. "Done when" flipped clean: `rg YamlError --type rust` empty across the workspace, `cargo make ci` green end-to-end (lint + test + test-docs + doc + vet + outdated + deny + fmt). No regressions — wire discriminant unchanged (both `YamlDe` and `YamlSer` keep `variant_str() == "yaml"`), `?`-propagation from raw `serde_saphyr::{Error, ser::Error}` still works via the two `#[from]` variants, and the `manifest_parse` / `sidecar_parse` callers in `crates/tool` drop their `Box::new(err.into())` wrapping now that the helpers take `impl Display`.
- **S5**: actual −16 LOC of skill body (4 lines × 4 SKILL.md files) vs predicted −12 (3 lines × 4); the extra line per file is the trailing blank that the H2 delete swallowed to avoid two consecutive blanks between `## Phase outcome contract` and the next H2. New predicate `checkNoFrontmatterRestatement` landed at +31 LOC in `scripts/checks/skill_body.ts` (+ 2 wire-up lines in `scripts/checks.ts`); REVIEW estimated "<15 lines" — actual was double, the gap is the `walk` boilerplate every predicate carries, not new logic. "Done when" flipped clean: `rg '^## Input$' plugins -g 'SKILL.md' -l` returns no matches; the broader `rg '^## Input$' plugins -l` still hits five non-SKILL files (briefs, references, sow examples) which are correctly out of scope (they aren't subject to the skill-body cap). Seed test confirmed the predicate fires as designed (re-introduced `## Input` in `build/SKILL.md` produced exactly one new `Frontmatter restated in skill body` failure at the expected line). No regressions — `make checks` reports the same 15 pre-existing broken-link failures in `rfcs/archive/` and `docs/explanation/release-notes.md` (unrelated to S5) before and after the change.
