# Code & Skill Review — May 2026

## Summary

1. **Top three:** (1) Delete test-only `change::finalize` domain module (−1527 LOC); (2) fix stale vectis WASM breaking `cargo make check` (2 failing tests); (3) replace phantom `specrun plan finalize` docs with the shipped `specrun plan archive` verb (wire-contract drift, ~28 live references).
2. **Total ΔLOC if all land:** approximately **−1560** net (−1527 finalize module, −~35 CLI/skill/doc tidies, +~5 Makefile/test precondition lines for vectis).
3. **Primary non-LOC axes:** module/crate edges (−1 dead domain subtree), defect surface (−2 CI failures, −1 wire-contract mismatch class), branches (−1 duplicate CLI exit handler).
4. **Verified defects closed:** 2 CI test failures (`schema_vectis_*`); 1 wire-contract class (`specrun plan finalize` documented but not registered in clap). Defect-only ΔLOC: **+5** (vectis rebuild precondition only; doc fixes are neutral).
5. **Most likely to break in remediation:** deleting `crates/domain/src/change/finalize/` — probe/classification logic is well-tested but never wired; confirm `/spec:finalize` skill runbook stays the canonical orchestrator before removal.

---

## Reconnaissance

| Signal | specify-cli | specify (plugins/docs) |
|--------|-------------|------------------------|
| **tokei Rust LOC** | 52,844 code lines (69,306 incl. markdown in `.rs`) | 19,510 code lines (80,003 incl. embedded langs in `.md`) |
| **cargo tree --duplicates** | `base64` 0.21.7 / 0.22.1; `bitflags` via `ron`/`rustix`; transitive via `wasm-pkg-client` → `specify-tool` | — |
| **`#[test]` count** | 467 matches across `crates/` `src/` `tests/` (per-file `rg -c`) | — |
| **`mod.rs` files** | 4 (`tests/common`, `crates/domain/tests/common`, `crates/authoring/src/check`, `wasi-tools/vectis/tests/engine_support`) | — |
| **docs/standards + AGENTS.md** | 790 lines | 731 lines |
| **Rust files >500 LOC** | 19 under `crates/` + `src/` (largest: `crates/domain/tests/workspace.rs` 1048) | — |
| **CI** | `cargo make check` **FAIL** — 1078/1080 pass; 2 fail in `specify::tool_schema` | `make check` **PASS** — "All checks passed." |
| **First CI failure** | `schema_vectis_unknown_name_exits_nonzero` — stdout not JSON; guest stderr: `error: unrecognized subcommand 'schema'` | — |
| **unwrap/expect (non-test hot path)** | 907 total matches under `crates/` + `src/` excluding `tests/`; **0** on CLI handler paths outside `#[cfg(test)]` | — |
| **panic!/unreachable!** | 76 matches (non-test `crates/` + `src/`) | — |

---

## Structural findings

### F1 — Delete unshipped `change::finalize` module

**Evidence:** `crates/domain/src/change/finalize.rs` (279) + submodules (301) + `crates/domain/tests/finalize.rs` (947) = **1527 LOC**. `rg 'change::finalize' crates/ src/ tests/` → hits **only** `crates/domain/tests/finalize.rs`. CLI exposes `PlanAction::Archive` only (`src/runtime/commands/plan/cli.rs:277–283`); no `Finalize` variant. `/spec:finalize` skill runbook already orchestrates `workspace push` → `gh pr view` → `specrun plan archive` (`plugins/spec/skills/finalize/references/runbook.md:14–15`).

**Action:**
1. Delete `crates/domain/src/change/finalize.rs`, `crates/domain/src/change/finalize/`, `crates/domain/tests/finalize.rs`.
2. Remove `pub mod finalize;` from `crates/domain/src/change.rs`.
3. `rg 'change::finalize|specify change finalize' crates/ docs/ DECISIONS.md` → zero hits outside archived RFCs.

**Quality delta:** −1527 LOC, −4 module files, −1 crate edge, −947 lines of test-only harness.

**Net LOC:** 1527 → 0.

**Done when:** `rg 'mod finalize|change::finalize' crates/domain/` → no matches; `cargo make check` passes without `specify-domain::finalize` test binary.

**Rule?** no — one-off dead subtree, not a repeated pattern.

**Counter-argument:** "Future `specrun plan finalize` will wire this." Loses: pre-1.0, skill already owns orchestration, and `plan archive` is the only shipped archive writer — keeping both paths guarantees drift (already present in docs).

**Depends on:** F3 (doc verb alignment) should land in the same PR if finalize module prose is deleted from `DECISIONS.md`.

---

### F2 — Rebuild vectis WASM before schema integration tests

**Evidence:** `cargo make check` summary: `FAIL specify::tool_schema schema_vectis_tokens_returns_valid_json` and `schema_vectis_unknown_name_exits_nonzero`. stderr: `error: unrecognized subcommand 'schema'` from guest vectis. Test reads stale artifact at `wasi-tools/target/wasm32-wasip2/release/vectis.wasm` (`tests/tool_schema.rs:15–17`); `cargo build` there does **not** run `scripts/build-vectis-local.sh`. Fresh build writes `target/vectis-wasi-tools/release/vectis.wasm` (`scripts/build-vectis-local.sh:19–26`). Skip guard only checks `is_file()` — stale file exists, skip never fires.

**Action:**
1. In `Makefile.toml`, add `vectis-wasm` to `[tasks.test] dependencies` (mirror comment block for `contract-wasm` at lines 112–115).
2. Point `vectis_wasm()` at `repo_root().join("target/vectis-wasi-tools/release/vectis.wasm")`.
3. Keep early-return skip when that path is absent (local dev without WASI target installed).

**Before:**
```rust
fn vectis_wasm() -> PathBuf {
    repo_root().join("wasi-tools/target/wasm32-wasip2/release/vectis.wasm")
}
```

**After:**
```rust
fn vectis_wasm() -> PathBuf {
    repo_root().join("target/vectis-wasi-tools/release/vectis.wasm")
}
```

**Quality delta:** −2 defects, +0 branches (CI green).

**Net LOC:** tests 180 → 180; Makefile 130 → 131 (+1 dependency line).

**Done when:** `cargo make check` summary shows 1080/1080 pass (0 failed in `specify::tool_schema`).

**Rule?** no.

**Counter-argument:** "Developers can run `cargo make vectis-wasm` manually." Loses: CI currently red on clean machines with an old cached wasm present — exactly the failure mode observed.

**Depends on:** none.

---

### F3 — Retire phantom `specrun plan finalize` verb in live docs

**Evidence:** `rg 'specrun plan finalize' --glob '!rfcs/**' specify/` → **28** hits (`AGENTS.md:69`, `docs/standards/cli-contract.md`, `plugins/spec/skills/execute/references/stop-conditions.md:50`, test fixtures, etc.). `src/runtime/commands/plan/cli.rs` registers **`Archive` only** — `rg 'plan finalize|PlanAction::Finalize' specify-cli/src` → **0**. Skill body correctly uses `specrun plan archive` (`plugins/spec/skills/finalize/SKILL.md:17`). DECISIONS.md still says `specify plan finalize` (`specify-cli/DECISIONS.md:333–334`).

**Action:**
1. Replace `specrun plan finalize` → `specrun plan archive` in every live doc/skill/fixture under `specify/` and `specify-cli/{AGENTS.md,DECISIONS.md,docs/}` (exclude `rfcs/`).
2. Where prose conflates skill and verb, use: "`/spec:finalize` skill" vs "`specrun plan archive` CLI".
3. Update `tests/fixtures/skills/finalize/*/transcript.md` command lines to `specrun plan archive <name>`.

**Quality delta:** −1 wire-contract defect class, −0 LOC (mostly 1:1 substitution).

**Net LOC:** ~28 edited lines, ΔLOC ≈ 0.

**Done when:** `rg 'specrun plan finalize' --glob '!rfcs/**' /Users/andrewweston/github.com/augentic/specify /Users/andrewweston/github.com/augentic/specify-cli` → 0; `specrun plan --help` still lists `archive` only.

**Rule?** no — one-time vocabulary alignment.

**Counter-argument:** "Add `plan finalize` as a clap alias." Loses: +handler +tests +duplicate noun; skill already composes smaller verbs (jj-style thin commands).

**Depends on:** none (orthogonal to F1; land together if F1 deletes finalize module docs).

---

### F4 — Collapse duplicate tool exit handlers

**Evidence:** `run_tool` and `run_tool_schema` in `src/runtime/commands.rs:167–191` are byte-identical except the `tool::run` vs `tool::schema` call — duplicate `Ctx::load` + `Exit::Code` mapping (cargo/clap pattern: one helper, ripgrep-style).

**Action:**
1. Replace both with:
```rust
fn run_tool_with(format: Format, name: &str, args: Vec<String>) -> Exit {
    let ctx = match Ctx::load(format) {
        Ok(ctx) => ctx,
        Err(err) => return report(format, &err),
    };
    match tool::run(&ctx, name, args) {
        Ok(0) => Exit::Success,
        Ok(code) => Exit::Code(code),
        Err(err) => report(format, &err),
    }
}
```
2. Dispatch: `ToolAction::Run { name, args }` → `run_tool_with(format, &name, args)`; `ToolAction::Schema { name, schema }` → `run_tool_with(format, &name, vec!["schema".into(), schema.into()])`.
3. Delete `src/runtime/commands/tool/schema.rs`; export `run` only from `tool.rs`; remove `mod schema` and `pub(super) use schema::schema`.

**Quality delta:** −~20 LOC, −1 module file, −1 branch duplicate, −1 call-site type (`schema` handler).

**Net LOC:** `commands.rs` 255 → ~245; delete `schema.rs` 13 lines.

**Done when:** `rg 'run_tool_schema|mod schema' specify-cli/src` → 0; `specrun tool schema vectis tokens` still exits 0 with JSON when F2 precondition met.

**Rule?** no.

**Counter-argument:** "Separate handlers document schema passthrough intent." Loses: comment on `run_tool_with` suffices; DECISIONS already documents `Exit::Code` passthrough once.

**Depends on:** F2 (for green integration test).

---

### F5 — Delete retired change-skills stub page

**Evidence:** `docs/reference/change-skills/draft.md` is 3 lines ("`/change:draft` (retired)"). RFC-26 step 5 removed `plugins/change/`; marketplace registers only `spec` plugin. Page adds navigation dead-end; index already documents `/spec:plan`.

**Action:**
1. Delete `docs/reference/change-skills/draft.md`.
2. Remove any link to `draft.md` from `docs/reference/change-skills/index.md` or `docs/SUMMARY.md` if present (`rg 'change-skills/draft'`).

**Quality delta:** −3 LOC, −1 doc file.

**Net LOC:** 3 → 0.

**Done when:** `test ! -f docs/reference/change-skills/draft.md`.

**Rule?** no.

**Counter-argument:** "Stub helps operators migrating from 1.x." Loses: pre-1.0 hard cut per AGENTS; release-notes already state `/change:*` retires.

**Depends on:** none.

---

## One-touch tidies

### T1 — One-line SHA-256 in codex schema drift check

**Evidence:** Hand-rolled loop `crates/authoring/src/check/codex_schema_drift.rs:108–115` (8 lines). Same crate already depends on `sha2`. std-style: `format!("{:x}", Sha256::digest(bytes))` (used elsewhere via `specify_tool::sha256_hex` in runtime; authoring stays dependency-light).

**Action:** Replace `sha256_hex` body with `format!("{:x}", Sha256::digest(bytes))`; delete loop.

**Quality delta:** −5 LOC, −1 hand-rolled helper pattern.

**Net LOC:** 128 → 123.

**Done when:** `wc -l crates/authoring/src/check/codex_schema_drift.rs` ≤ 123; `make check` pass.

**Rule?** no.

**Depends on:** none.

---

### T2 — Remove orphan `Phase outcome contract` section in drop skill

**Evidence:** Only `plugins/spec/skills/drop/SKILL.md` carries `## Phase outcome contract` linking `phase-outcome-contract.md` (lines 17–19). Merge/build/refine link guardrails inline; drop is the outlier. Section adds 3 lines the model reads from shared guardrails anyway.

**Action:** Delete lines 17–19 (heading + blockquote). Keep guardrails bullet at line 85.

**Quality delta:** −3 LOC, −1 skill section.

**Net LOC:** 85 → 82 body lines.

**Done when:** `rg 'Phase outcome contract' plugins/spec/skills/drop/SKILL.md` → 0; `make check` pass.

**Rule?** no.

**Depends on:** none.

---

### T3 — Drop skill empty step 5 header

**Evidence:** `plugins/spec/skills/drop/SKILL.md:65–67` — `5. **Display summary**` immediately followed by `## Output On Success` with no step body (structural gap / frontmatter-body drift risk).

**Action:** Renumber: merge step 5 into `## Output On Success` (delete standalone step 5 header).

**Quality delta:** −2 LOC, −1 empty section.

**Net LOC:** 85 → 83.

**Done when:** no `5. **Display summary**` line in drop SKILL; section flows step 4 → Output.

**Rule?** no.

**Depends on:** T2 (same file; batch edit).

---

### T4 — Inline `prefixed_sha256` wrapper in agents fingerprint

**Evidence:** `src/runtime/commands/agents/fingerprint.rs:124–130` — `sha256_hex` is a one-line forward to `specify_tool::sha256_hex`; only used by `prefixed_sha256`.

**Action:** In `prefixed_sha256`, call `specify_tool::sha256_hex(bytes)` directly; delete local `sha256_hex` fn.

**Quality delta:** −4 LOC, −1 function.

**Net LOC:** 207 → 203.

**Done when:** `rg 'fn sha256_hex' src/runtime/commands/agents/fingerprint.rs` → 0.

**Rule?** no.

**Depends on:** none.

---

### T5 — Fix broken plan-lock link target in build/merge skills

**Evidence:** Build/merge SKILL.md cite `[plan-lock.md](../../references/plan-lock.md)` but canonical file lives at `plugins/spec/skills/execute/references/plan-lock.md` (78 lines). Shared `plugins/spec/references/plan-lock.md` **does not exist** (`ls plugins/spec/references/plan-lock.md` → missing). Execute skill references work via `execute/references/plan-lock.md`.

**Action:** Point build + merge skills at `../execute/references/plan-lock.md` (same relative depth as execute loop uses).

**Quality delta:** −1 skill integrity broken-link class (manual verify; not yet a specdev predicate on this path).

**Net LOC:** 0 (path fix only).

**Done when:** `test -f` resolved target from `plugins/spec/skills/build/SKILL.md` link; link preview opens existing file.

**Rule?** yes — extend existing `links.*` predicate to resolve `../../references/plan-lock.md` from shipped skills (≤30 lines in link walker).

**Depends on:** none.

---

### T6 — Deduplicate `scaffold_project_with_tool` vs contract fixture

**Evidence:** `tests/tool_schema.rs:27–75` (49 lines) duplicates adapter scaffold pattern from `tests/contract_tool.rs:22–67` (adapter yaml, briefs, tools.yaml wiring). Only used by `tool_schema.rs`.

**Action:** Hoist generic `scaffold_tool_project(tmp, tool_name, wasm_path) -> (project, cache)` into `tests/common/mod.rs`; delete local struct/fixture in `tool_schema.rs`.

**Quality delta:** −~25 LOC net (after common helper), −1 duplicate scaffold.

**Net LOC:** tool_schema 180 → ~130; common +55.

**Done when:** `rg 'scaffold_project_with_tool|SchemaFixture' tests/` → hits only `common/mod.rs`.

**Rule?** no.

**Depends on:** F2.

---

### T7 — Trim finalize runbook duplicate overview table

**Evidence:** `plugins/spec/skills/finalize/references/runbook.md` lines 7–16 restate SKILL.md critical path as a table; SKILL.md lines 11–17 already list the same five steps. 10 lines of restatement (skill.frontmatter-restatement pattern at section level).

**Action:** Delete `## Overview` table (lines 5–18); open runbook at `## Invocation`.

**Quality delta:** −13 LOC, −1 duplicated section.

**Net LOC:** 227 → 214.

**Done when:** `wc -l plugins/spec/skills/finalize/references/runbook.md` = 214.

**Rule?** no.

**Depends on:** F3 (verb names in runbook already correct).

---

### T8 — Remove `CacheKey` type alias

**Evidence:** `src/runtime/commands/tool/dto.rs:10` — `type CacheKey = (String, String, String);` used once in `tool.rs:97–105`. Alias adds indirection without semantic weight.

**Action:** Inline `(String, String, String)` at `kept_by_scope`; delete alias line.

**Quality delta:** −1 type alias, −1 LOC.

**Net LOC:** dto 137 → 136.

**Done when:** `rg 'CacheKey' specify-cli/` → 0.

**Rule?** no.

**Depends on:** none.

---

### T9 — DECISIONS exit-code comment points at wrong path

**Evidence:** `src/runtime/output.rs:21` cites `../../DECISIONS.md#exit-codes` from `src/runtime/` — resolves outside repo root. DECISIONS lives at repo root (`specify-cli/DECISIONS.md`).

**Action:** Fix comment to `../../../DECISIONS.md#exit-codes` (or repo-relative doc link used elsewhere in `src/runtime/commands.rs:163`).

**Quality delta:** −1 broken doc reference (comment-only; wrong comment misleads agents).

**Net LOC:** 0.

**Done when:** comment path resolves to existing file from `src/runtime/output.rs` location.

**Rule?** no.

**Depends on:** none.

---

### T10 — Plan skill closing-hint duplication in body

**Evidence:** `plugins/spec/skills/plan/SKILL.md` lines 49–55 (`## Closing hint`) repeat step 8 from Critical Path line 22 almost verbatim (same literal `specrun plan transition` command). Predicate `skill.step-body-duplicates-critical-path` may not fire on cross-section duplication.

**Action:** Replace `## Closing hint` section with one line: "Emit the closing hint from step 8 verbatim."

**Quality delta:** −6 LOC, −1 duplicated prose block.

**Net LOC:** 69 → 63 body lines.

**Done when:** `rg '## Closing hint' plugins/spec/skills/plan/SKILL.md` → 0; step 8 still contains full hint block.

**Rule?** no.

**Depends on:** none.

---

## Findings not promoted

| Candidate | Why dropped |
|-----------|-------------|
| Delete `plugins/change/` | Directory already removed on disk; glob index was stale |
| Dedupe `plugins/spec/skills/*/references/` trees | Symlinks to `plugins/spec/references/` — not copies |
| Add `specrun plan finalize` CLI verb | Adds LOC/types; contradicts subtraction default |
| Delete vectis schema integration tests | Loses host→WASI envelope coverage; F2 fixes root cause |
| Collapse `Ctx::slices_dir` / `archive_dir` | Net +LOC at call sites for −8 lines in `context.rs` |
| Update `decision-log.md` 1.x verb history | Historical decision record; not operator-facing contract |

---

## Verification checklist (post-remediation)

```bash
# specify plugins/docs
cd specify && make check

# specify-cli
cd specify-cli && cargo make check

# Wire contract
rg 'specrun plan finalize' --glob '!rfcs/**' ../specify ../specify-cli
specrun plan --help | rg 'archive'

# Dead finalize module
rg 'change::finalize' specify-cli/crates specify-cli/tests

# Schema tests
cd specify-cli && cargo nextest run -p specify --test tool_schema
```
