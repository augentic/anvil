# Code & Skill Review - May 2026

1. Top three: F1 delete the retired `change finalize` domain island; F2 delete the ignored retired cross-repo integration test; F3 replace the phantom `specrun plan finalize` contract with shipped `specrun plan archive`.
2. Total delta if all land: about -2,150 LOC.
3. Primary non-LOC axes moved: fewer dead module edges, fewer stale CLI-contract references, fewer branches/types in the first-party tool check, and fewer operator-path panic sites.
4. Top verified defects closed: `plan finalize` wire-contract drift; finalize runbook claims guards `plan archive` does not implement; two operator-path panic sites. Defect-only positive LOC: +0.
5. Most likely to break in remediation: F3, because fixture transcripts and reference docs must be updated together or `make check` will catch link/golden drift.

## Reconnaissance

- `tokei`: `specify` has 87,127 total lines, including 53,635 Markdown lines; `specify-cli` has 83,648 total lines, including 60,412 Rust lines.
- `cargo tree --duplicates` in `specify-cli`: duplicates exist, led by transitive `base64` 0.21.7 / 0.22.1, multiple `wasmparser` lines, `thiserror` 1.0.69 / 2.0.18, and `reqwest` 0.12.28 / 0.13.3. No Cargo-edge finding qualified because the duplicates are upstream through `wasm-pkg-client` / Wasmtime.
- `rg -c '^#\[test\]' crates/ src/ tests/`: 467 total matches by summing per-file counts; `tests/cross_repo.rs:1` is ignored and retired.
- `rg --files -g '**/mod.rs'`: 4 files: `tests/common/mod.rs`, `wasi-tools/vectis/tests/engine_support/mod.rs`, `crates/authoring/src/check/mod.rs`, `crates/domain/tests/common/mod.rs`.
- `wc -l docs/standards/*.md AGENTS.md` across both repos: 1,521 total lines (`specify`: 731; `specify-cli`: 790).
- Files over 500 lines under `crates/` and `src/`: 21, largest are `crates/domain/tests/workspace.rs` 1048, `crates/domain/tests/finalize.rs` 947, `crates/domain/src/discovery/document.rs` 890, `crates/domain/src/codex/resolve.rs` 829.
- `make checks` in `specify`: no such target (`make: *** No rule to make target 'checks'. Stop.`). Nearest real target `make check`: pass, `All checks passed.`
- `cargo make check` in `specify-cli`: pass, `Build Done in 279.53 seconds.`
- `rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' crates/ src/`: hot files include `crates/tool/src/hash.rs:1`, `crates/authoring/src/check/tools.rs:1`, and many `#[cfg(test)]` modules inside `src/` files.
- `rg -c 'panic!|unreachable!' --glob '!**/tests/**' crates/ src/`: hot files include `crates/authoring/src/check/tools.rs:1`; most other hits are in `#[cfg(test)]` modules or integration tests.

## Structural Findings

### F1 - Delete Dead Finalize Module

**Evidence:** `wc -l crates/domain/src/change/finalize.rs crates/domain/src/change/finalize/*.rs crates/domain/tests/finalize.rs` reports 1,527 lines. `src/runtime/cli.rs` has no `Commands::Change`; `src/runtime/commands/plan/cli.rs` exposes `PlanAction::Archive`, not `Finalize`. Current Rust refs are only the module export and its own tests:

```text
rg -n 'change::finalize|change finalize|mod finalize|pub mod finalize|tests/finalize' crates src tests --glob '*.rs' | wc -l
       9
```

**Action:**
1. Delete `crates/domain/src/change/finalize.rs`.
2. Delete `crates/domain/src/change/finalize/{archive,probe,summary}.rs`.
3. Delete `crates/domain/tests/finalize.rs`.
4. In `crates/domain/src/change.rs`, delete `pub mod finalize;` and trim the module doc from "closure verb" to the plan-driven change model.

Before:

```rust
pub mod finalize;
mod plan;
```

After:

```rust
mod plan;
```

**Quality delta:** -1,528 LOC, -1 module subtree, -many public DTOs/enums, -many branch paths, -28 tests for an unwired verb.

**Net LOC:** 1,528 current -> 0 proposed.

**Done when:** `test ! -e crates/domain/src/change/finalize.rs && test ! -d crates/domain/src/change/finalize && test ! -f crates/domain/tests/finalize.rs && rg 'pub mod finalize|change::finalize' crates src tests` returns no matches.

**Rule?** no.

**Counter-argument:** The module may be useful when `plan finalize` returns. It loses because pre-1.0 code should track shipped surface, and this one is not reachable from clap.

**Depends on:** none.

### F2 - Delete Ignored Retired Cross-Repo Test

**Evidence:** `wc -l tests/cross_repo.rs` reports 586 lines. The sole test is ignored with a retired-verb reason and still shells out to a nonexistent `change finalize` command:

```text
310:#[ignore = "Wave 1.1: drives the retired `change draft` + `change finalize` verbs; \
560:        envs.command().args(["--format", "json", "change", "finalize"]).assert().success();
583:    let second = envs.command().args(["--format", "json", "change", "finalize"]).assert().failure();
```

**Action:** Delete `tests/cross_repo.rs`. Keep the manual scenario pack under `specify/tests/cross-repo/`; it is the live acceptance surface.

**Quality delta:** -586 LOC, -1 ignored test, -1 stale CLI branch surface, -1 false signal in the test count.

**Net LOC:** 586 current -> 0 proposed.

**Done when:** `test ! -f tests/cross_repo.rs && rg 'change\", \"finalize|rm01_replays_cross_repo' tests crates src` returns no matches.

**Rule?** no.

**Counter-argument:** Ignored tests can document intended future behavior. It loses because this one documents a retired command, while the manual scenario pack already documents the current workflow.

**Depends on:** none.

### F3 - Replace Phantom Plan Finalize

**Evidence:** Current non-RFC docs still advertise `plan finalize` 38 times:

```text
rg -n 'specrun plan finalize|specify plan finalize|plan finalize' specify specify-cli --glob '*.md' --glob '!**/rfcs/**' --glob '!REVIEW.md' | wc -l
      38
```

The shipped clap surface has `PlanAction::Archive` only:

```rust
Archive {
    #[arg(long)]
    force: bool,
}
```

`DECISIONS.md` also names the wrong verb at lines 333-336: "`specify plan finalize` moves `change.md` + `plan.yaml`...".

**Action:**
1. In live docs, standards, fixtures, and `DECISIONS.md`, replace `specrun plan finalize` / `specify plan finalize` with `specrun plan archive` / `specify plan archive`.
2. Where text says finalize verifies PR state, assign that behavior to `/spec:finalize` and `gh pr view`, not the archive verb.
3. Leave historical RFC references alone.

Before:

```md
specrun plan finalize
```

After:

```md
specrun plan archive
```

**Quality delta:** -1 wire-contract defect, -38 stale references, -call-site confusion. LOC is expected to stay flat; defect closure justifies the trade.

**Net LOC:** 38 stale refs current -> 0 stale refs proposed; about 0 LOC delta.

**Done when:** `rg -n 'specrun plan finalize|specify plan finalize|plan finalize' specify specify-cli --glob '*.md' --glob '!**/rfcs/**' --glob '!REVIEW.md'` returns no matches.

**Rule?** yes, only if the existing link checker can be extended in under 30 lines to compare documented `specrun <group> <verb>` forms against clap help. Otherwise no new predicate.

**Counter-argument:** A future `plan finalize` verb may return. It loses because pre-1.0 docs should describe the binary that ships today.

**Depends on:** none.

### F4 - Trim Archive Guard Claims

**Evidence:** `plugins/spec/skills/finalize/references/runbook.md` claims `specrun plan archive` runs PR and workspace guards:

```text
111:The verb runs four guards in order: plan presence, plan terminal-state (drained), per-project PR-state (`MERGED` on remote), and workspace-cleanliness (`git status --porcelain` empty).
153:| finalize CLI guard refusal — dirty workspace | step 5 (`specrun plan archive`) | commit / stash the dirty residue, re-run finalize |
154:| finalize CLI guard refusal — unmerged PR | step 5 (`specrun plan archive`) | merge the named PRs externally, re-run finalize |
```

The code path only checks plan presence, terminal entries, target collisions, and moves files:

```rust
pub(super) fn archive(ctx: &Ctx, force: bool) -> Result<()> {
    let layout = ctx.layout();
    let plan_path = layout.plan_path();
    if !plan_path.exists() {
        return Err(Error::ArtifactNotFound {
```

**Action:**
1. In `plugins/spec/skills/finalize/references/runbook.md`, rewrite Step 5 to say `plan archive` performs archive preflight only.
2. Delete the dirty-workspace and unmerged-PR rows from the Step 5 guard-refusal table; those belong to Step 3 / Step 4.
3. Replace the "idempotent `plan-not-found`" claim with the actual re-entry rule: absence means the plan is already archived only after checking the archive path or prior transcript.

**Quality delta:** -1 skill/runtime contract defect, -about 8 LOC, -2 impossible halt branches.

**Net LOC:** 212 current -> about 204 proposed.

**Done when:** `rg -n 'per-project PR-state|workspace-cleanliness|finalize CLI guard refusal — dirty workspace|finalize CLI guard refusal — unmerged PR|plan-not-found' plugins/spec/skills/finalize/references/runbook.md` returns no matches.

**Rule?** no.

**Counter-argument:** Redundant guard language reminds agents to be cautious. It loses because false redundancy makes agents route errors to a CLI branch that cannot produce them.

**Depends on:** F3.

### F5 - Delete First-Party Version Regex

**Evidence:** The first-party tool check already compares exact package pins:

```text
27:        package: "specify:contract@0.3.0",
32:        package: "specify:vectis@0.3.0",
```

It also carries a separate regex and operator-path panic:

```text
86:fn version_re() -> &'static Regex {
88:    RE.get_or_init(|| Regex::new(r"^(\d+\.\d+\.\d+)$").expect("version regex"))
261:        if !version_re().is_match(version) {
```

The shared adapter schema already owns tool-version shape (`schemas/adapter.schema.json:65-68`).

**Action:**
1. Delete `version_re()`.
2. Delete the `if !version_re().is_match(version)` branch in `resolve_adapter_declarations`.
3. Delete `version_re_accepts_semver_triple`.
4. In `crates/authoring/tests/check_tools.rs`, replace the prerelease-message assertion with the existing package-mismatch assertion or remove it if the count still proves the case.

Before:

```rust
if !version_re().is_match(version) {
    shape_findings.push(invalid_declaration(...));
    continue;
}
```

After:

```rust
declarations.insert(name.to_string(), format!("specify:{name}@{version}"));
```

**Quality delta:** -about 20 LOC, -1 branch, -1 helper, -1 operator-path `expect`, -1 duplicate validation owner.

**Net LOC:** 360 current -> about 340 proposed.

**Done when:** `rg -n 'version_re|without prerelease metadata|version regex' crates/authoring/src/check/tools.rs crates/authoring/tests/check_tools.rs` returns no matches and `make check` still passes.

**Rule?** no.

**Counter-argument:** The custom message is nicer for prerelease pins. It loses because exact first-party package pins already reject the value, and the schema owns version grammar.

**Depends on:** none.

### F6 - Remove Hex Formatting Panic

**Evidence:** `crates/tool/src/hash.rs` has an operator-path `expect`:

```text
15:pub fn sha256_output_hex(digest: impl AsRef<[u8]>) -> String {
18:    for byte in bytes {
19:        write!(hex, "{byte:02x}").expect("String accepts formatted hex");
```

The panic-adjacent reconnaissance includes this exact file:

```text
crates/tool/src/hash.rs:19:        write!(hex, "{byte:02x}").expect("String accepts formatted hex");
```

**Action:** Replace the manual loop with the digest type's standard lower-hex formatting, the same idiom used by `sha2` examples and common cargo/ripgrep-style digest code.

Before:

```rust
pub fn sha256_output_hex(digest: impl AsRef<[u8]>) -> String {
    let bytes = digest.as_ref();
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(hex, "{byte:02x}").expect("String accepts formatted hex");
    }
    hex
}
```

After:

```rust
pub fn sha256_output_hex(digest: impl std::fmt::LowerHex) -> String {
    format!("{digest:x}")
}
```

**Quality delta:** -5 LOC, -1 panic surface, -1 hand-rolled formatting loop.

**Net LOC:** 22 current -> 17 proposed.

**Done when:** `rg -n 'String accepts formatted hex|for byte in bytes' crates/tool/src/hash.rs` returns no matches and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** The current code cannot really fail because formatting into `String` is infallible. It loses because `format!` expresses that invariant without a runtime panic call.

**Depends on:** none.

## One-Touch Tidies

### T1 - Use Digest LowerHex

**Evidence:** `crates/authoring/src/check/codex_schema_drift.rs:108-110` hand-rolls lowercase hex:

```rust
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}
```

**Action:** Replace the body with `format!("{:x}", Sha256::digest(bytes))`.

**Quality delta:** LOC flat, -1 hand-rolled formatting loop. LOC-flat trade is justified because `sha2` exposes the lower-hex idiom directly.

**Net LOC:** 123 current -> 123 proposed.

**Done when:** `rg -n 'iter\\(\\).*format!\\(\"\\{byte:02x\\}\"' crates/authoring/src/check/codex_schema_drift.rs` returns no matches.

**Rule?** no.

**Counter-argument:** The current helper is explicit. It loses because the crate already depends on `sha2`, whose digest output formats as lowercase hex directly.

**Depends on:** none.

### T2 - Inline Resolver Digest Shim

**Evidence:** `crates/tool/src/resolver/digest.rs:10-12` is a one-line wrapper around `crate::hash::sha256_hex`, while `crates/tool/src/resolver.rs` calls it only to compute acquired-byte sidecars:

```rust
pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    hash::sha256_hex(bytes)
}
```

**Action:** Import `crate::hash::sha256_hex` where needed and delete the wrapper.

**Quality delta:** -3 LOC, -1 helper function, -1 module edge.

**Net LOC:** 122 current -> about 119 proposed.

**Done when:** `rg -n 'pub\\(super\\) fn sha256_hex|digest::sha256_hex' crates/tool/src/resolver.rs crates/tool/src/resolver/digest.rs` returns no matches.

**Rule?** no.

**Counter-argument:** Keeping digest names under `resolver::digest` localizes resolver code. It loses because the local name only forwards to the public helper and adds no policy.

**Depends on:** none.

## Findings Not Promoted

- No dependency-removal finding qualified from `cargo tree --duplicates`; the duplicate versions are transitive through Wasmtime, Warg, and `wasm-pkg-client`.
- No `mod.rs` finding qualified; the four hits are test support or the established authoring check module.
- No broad skill-body shortening qualified after `make check` passed; the remaining opportunities were mostly taste or would blur critical-path instructions.
- No new predicate or clippy enforcement is recommended.

## Verification Checklist

```bash
cd /Users/andrewweston/github.com/augentic/specify && make check
cd /Users/andrewweston/github.com/augentic/specify-cli && cargo make check
cd /Users/andrewweston/github.com/augentic && rg -n 'specrun plan finalize|specify plan finalize|plan finalize' specify specify-cli --glob '*.md' --glob '!**/rfcs/**' --glob '!REVIEW.md'
cd /Users/andrewweston/github.com/augentic/specify-cli && rg 'pub mod finalize|change::finalize|version_re|String accepts formatted hex' crates src tests
```

## Post-mortem

- F1: actual ΔLOC -1529 vs predicted -1528; done-when flipped cleanly: yes; regressions: none.
- F2: actual ΔLOC -586 vs predicted -586; done-when flipped cleanly: yes; regressions: none.
- F3: actual ΔLOC -22 vs predicted 0; done-when flipped cleanly: yes; regressions: none.
- F4: actual ΔLOC -2 vs predicted -8; done-when flipped cleanly: yes; regressions: none.
- F5: actual ΔLOC -27 vs predicted -20; done-when flipped cleanly: yes; regressions: none.
- F6: actual ΔLOC -4 vs predicted -5; done-when flipped cleanly: yes; regressions: none.
- T1: actual ΔLOC +7 vs predicted 0; done-when flipped cleanly: yes; regressions: none.
- T2: actual ΔLOC -3 vs predicted -3; done-when flipped cleanly: yes; regressions: none.
