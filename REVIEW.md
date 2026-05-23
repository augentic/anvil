# Code & Skill Review — specify + specify-cli

Top three by tier: **F1 Fix merge SKILL merge-verb wire drift** (wire-contract defect — retired `--dry-run` / `--check-only` flags vs shipped `preview` / `conflict-check` / `run` subcommands), **F2 Delete unwired `check_target_adapter_versions`** (−121 LOC, exported validation never called from `Plan::validate` or `src/`), **F3 Collapse refine Critical Path / Step-body duplication** (−60 LOC, single-axis prose subtraction).
Total ΔLOC if all findings land: **approximately −235 LOC** (defect-only net **+4**, under the +30 cap).
Primary non-LOC axes moved: fewer dead public API names, fewer unreachable skill branches, −2 production panic sites, −1 wire-contract mismatch cluster (merge + finalize + replay-writer).
Top verified defects closed: **4** (merge CLI surface, finalize archive verb, replay-writer retired `outcome set`, composition merge production `unwrap`); **0** CI predicate failures at recon start. Defect-only net ΔLOC: **+4**.
Most likely to break in remediation: **F3** — deleting refine `## Step 1`–`## Step 7` bodies removes WASI preopen detail, journal hand-off examples, and fusion atomic-write instructions that Critical Path step bullets do not carry; migrate any detail not already in Critical Path into [`plugins/spec/references/synthesis/`](plugins/spec/references/synthesis/) before deleting.

## Reconnaissance

- `tokei`:
  - `specify`: 644 files, 87,109 total lines; Markdown 512 files / 49,624 lines (including embedded code blocks).
  - `specify-cli`: 446 files, 65,073 total lines; Rust 245 files / 47,893 lines.
- `cargo tree --duplicates` (`specify-cli`): non-empty — `base64 v0.21.7` / `v0.22.1`, `reqwest v0.12.28` / `v0.13.3`, `bitflags v2.11.1` via `rustix v0.38.44` / `v1.1.4`, `thiserror v1.0.69` / `v2.0.18`; dominated by `wasmtime` / `wasm-pkg-client` chains. `Cargo.toml` frozen for the pass.
- `rg -c '^#\[test\]' crates/ src/ tests/` (`specify-cli`): **512** test functions across matched files.
- `rg --files -g '**/mod.rs'`: **3 files** (`crates/domain/tests/common/mod.rs`, `wasi-tools/vectis/tests/engine_support/mod.rs`, `tests/common/mod.rs`).
- `wc -l docs/standards/*.md AGENTS.md`:
  - `specify`: **534 total**.
  - `specify-cli`: **638 total**.
- Files >500 lines under `crates/` and `src/` (`specify-cli`):
  - Tests: `crates/domain/tests/workspace.rs` 1041, `crates/domain/tests/finalize.rs` 947, `crates/domain/tests/registry.rs` 922, `crates/domain/src/change/plan/core/validate/tests.rs` 695.
  - Source: `src/commands/plan/create.rs` 966, `crates/domain/src/discovery/document.rs` 891, `crates/domain/src/slice/fusion.rs` 839, `crates/domain/src/adapter/core.rs` 709, `crates/domain/src/journal.rs` 595, `crates/domain/src/change/plan/core/model.rs` 629, `crates/domain/src/spec/provenance.rs` 607, `crates/tool/src/validate.rs` 520, `crates/domain/src/adapter/cache/io.rs` 509.
- `make checks` (`specify`): **passed** — `All checks passed.` Total failures: **0**; first 5 predicate ids: **none**.
- `cargo make check` (`specify-cli`): **passed** (`Finished \`dev\` profile […] in 23.43s`). First error: **none**.
- `rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' crates/ src/` (`specify-cli`): **716** matches across 57 files.
- `rg -c 'panic!|unreachable!' --glob '!**/tests/**' crates/ src/` (`specify-cli`): **49** matches across 20 files.
- Production `unwrap`/`expect` in `src/` handlers: **0** outside `#[cfg(test)]` blocks (all seven matching files gate test code).

## Structural Findings

### F1 — Fix merge SKILL merge-verb wire drift

**Evidence:** `plugins/spec/skills/merge/SKILL.md:33` and `:56` instruct `specify slice merge $SLICE --dry-run` and `--check-only`. Shipped CLI groups merge under subcommands (`specify-cli/src/commands/slice/cli.rs:74-91`):

```74:91:specify-cli/src/commands/slice/cli.rs
pub enum SliceMergeAction {
    Run { name: String },
    Preview { name: String },
    ConflictCheck { name: String },
}
```

`rg '--dry-run|--check-only' specify-cli/src` → **0 matches**. `rg 'SliceMergeAction|merge preview|merge conflict-check' specify-cli/src` confirms the subcommand surface.

Current-state grep:

```text
plugins/spec/skills/merge/SKILL.md:33:… Use `--dry-run` first … use `--check-only` for a baseline-conflict probe.
plugins/spec/skills/merge/SKILL.md:56:- **Never treat `--check-only` success …
```

**Action:**
1. In step 6 (`SKILL.md:33`), replace the flag prose with: `specify slice merge run $SLICE --format json` for the apply path; `specify slice merge preview $SLICE` when the operator asks to preview; `specify slice merge conflict-check $SLICE` for baseline-conflict probing.
2. In guardrails (`:56`), replace `--check-only` with `specify slice merge conflict-check`.
3. Align with the sibling reference already correct at `plugins/spec/references/slice-skills/merge.md` (if present) or `docs/reference/slice-skills/merge.md`.

Before:

```text
specify slice merge $SLICE --format json … Use `--dry-run` first … use `--check-only` …
```

After:

```text
specify slice merge run $SLICE --format json
specify slice merge preview $SLICE        # operator asks to preview
specify slice merge conflict-check $SLICE # baseline-conflict probe
```

**Quality delta:** `−2 LOC, −2 retired CLI flags, −1 wire-contract mismatch, −1 unreachable operator command`.

**Net LOC:** `plugins/spec/skills/merge/SKILL.md` **63 → ~61**.

**Done when:** `rg '\-\-dry-run|\-\-check-only' plugins/spec/skills/merge/SKILL.md` returns **0** and `rg 'merge (run|preview|conflict-check)' plugins/spec/skills/merge/SKILL.md` returns **≥3** hits; `make checks` still prints `All checks passed.`

**Rule?** no — `checkOperationalVocabulary` (`scripts/checks/prose.ts`) already maps `specify merge` → `specify slice merge run` but does not flag bare `specify slice merge $SLICE` without a subcommand.

**Counter-argument:** RFC-25 archived tables list `--dry-run` / `--check-only` on `specify slice merge`. It loses because the shipped binary is the wire contract pre-1.0 and the subcommand surface is what `src/commands/slice/cli.rs` parses today.

**Depends on:** none.

### F2 — Delete unwired `check_target_adapter_versions`

**Evidence:** `crates/domain/src/change/plan/core/validate.rs:388-413` exports `check_target_adapter_versions`. `Plan::validate` (`:34-51`) never calls it — the function is dead on every CLI path. Exported at `change.rs:12` and `plan/core.rs:22`; tested only in `validate/tests.rs:596-695` (`mod target_version`).

Current-state grep:

```text
$ rg 'check_target_adapter_versions' specify-cli --glob '*.rs'
crates/domain/src/change/plan/core/validate.rs:388
crates/domain/src/change/plan/core/validate/tests.rs:602,656,665,692
crates/domain/src/change.rs:12
crates/domain/src/change/plan/core.rs:22
crates/domain/src/change/plan/core/model.rs:313   # doc cross-ref only
```

Zero hits under `specify-cli/src/`.

**Action:**
1. Delete `check_target_adapter_versions` (`validate.rs:388-413`).
2. Delete `mod target_version { … }` (`validate/tests.rs:596-695`).
3. Remove the name from `pub use` blocks in `change.rs:12` and `plan/core.rs:22`.
4. Trim the doc cross-reference at `model.rs:313` to name the policy without linking a deleted symbol.
5. Delete the bullet at `specify-cli/DECISIONS.md:587` that points at the removed function.

Before:

```rust
pub fn check_target_adapter_versions(plan: &Plan, project_dir: &Path) -> Result<(), Error> { … }
```

After:

```rust
// No replacement. Wire into Plan::validate when target-version reconciliation
// becomes a product requirement; until then the export is dead weight.
```

**Quality delta:** `−121 LOC, −1 public function, −1 test module, −2 module-edge re-exports, −1 dead validation branch`.

**Net LOC:** `validate.rs` + `validate/tests.rs` + re-export sites **~834 → ~713**.

**Done when:** `rg 'check_target_adapter_versions' specify-cli` returns **0** and `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.

**Rule?** no.

**Counter-argument:** The check is documented policy for target `@vN` reconciliation and may land soon. It loses because an exported, untested-in-production validator misleads operators and agents into believing version mismatch is already enforced; re-adding ~26 lines when wiring is intentional is cheaper than maintaining a phantom API.

**Depends on:** none.

### F3 — Collapse refine Critical Path / Step-body duplication

**Evidence:** `plugins/spec/skills/refine/SKILL.md:11-19` lists a 7-step Critical Path; `## Step 1`–`## Step 7` (`:21-80`, **60 lines**) re-expands every step with near-identical content. Predicates pass because step headings differ from Critical Path list items (`checkNoStepBodyDuplicatesCriticalPath` only flags exact normalized duplicates).

Current-state line count:

```text
$ wc -l plugins/spec/skills/refine/SKILL.md
118
$ sed -n '21,80p' plugins/spec/skills/refine/SKILL.md | wc -l
60
```

**Action:**
1. Before deleting, audit Step 3–5 for detail absent from Critical Path (WASI preopen contract, journal NDJSON examples, fusion atomic-rename rule) and fold any genuinely unique lines into the matching Critical Path bullet or an existing synthesis reference link — do **not** drop operator-critical detail silently.
2. Delete `## Step 1` through `## Step 7` inclusive (`:21-80`).
3. Keep `## Closing hint`, `## References`, and `## Guardrails` unchanged.

Before:

```markdown
## Critical Path
1. **Resolve target and sources** — …
…
## Step 1 — Resolve target and sources
Resolve `$SLICE_NAME` first:
…
```

After:

```markdown
## Critical Path
1. **Resolve target and sources** — …
…
## Closing hint
```

**Quality delta:** `−60 LOC, −7 duplicate sections, −1 maintenance surface (two sources of truth for the same spine)`.

**Net LOC:** `plugins/spec/skills/refine/SKILL.md` **118 → ~58**.

**Done when:** `rg '^## Step [1-7]' plugins/spec/skills/refine/SKILL.md` returns **0**, `wc -l plugins/spec/skills/refine/SKILL.md` is **≤65**, and `make checks` prints `All checks passed.`

**Rule?** no.

**Counter-argument:** Step bodies carry examples (bash blocks, journal lines) Critical Path omits. It loses because those examples belong in [`plugins/spec/references/synthesis/`](plugins/spec/references/synthesis/) or the Critical Path bullets themselves — not duplicated inline; ripgrep-style tools (the house CLI model) keep one authoritative path and link out.

**Depends on:** none.

### F4 — Fix finalize archive verb wire drift

**Evidence:** `plugins/spec/skills/finalize/SKILL.md:3,17` and `finalize/references/runbook.md:15,123` instruct `specify plan finalize <name>`. Shipped CLI exposes `specify plan archive` with no name positional (`specify-cli/src/commands/plan/cli.rs:267-270`, handler at `lifecycle.rs:290`). `rg 'plan finalize' specify-cli/src` → **0 matches**; `rg 'fn archive' specify-cli/src/commands/plan` → **1 match**.

Cross-repo test marks finalize as future work:

```311:311:specify-cli/tests/cross_repo.rs
returns when W3.5's `/spec:finalize` skill + `specify plan finalize` land.
```

Current-state grep:

```text
plugins/spec/skills/finalize/SKILL.md:17:- Archive: run `specify plan finalize <name>`, …
plugins/spec/skills/finalize/references/runbook.md:15:| Finalize | `specify plan finalize <name>` | CLI |
```

**Action:**
1. Replace `specify plan finalize <name>` with `specify plan archive` in Critical Path step 5 (`SKILL.md:17`) and description frontmatter (`:3`).
2. Update runbook table row (`runbook.md:15`) and step 5 body (`runbook.md:123`).
3. Fix archive path prose: CLI archives to `.specify/archive/plans/<name>-<YYYYMMDD>.yaml` (see `PlanAction::Archive` help text), not `.specify/archive/plans/<name>-<YYYYMMDD>/`.

Before:

```text
Archive: run `specify plan finalize <name>`, then print merged PRs …
```

After:

```text
Archive: run `specify plan archive`, then print merged PRs …
```

**Quality delta:** `−0 LOC, −1 wire-contract mismatch, −1 phantom CLI positional`.

**Net LOC:** finalize skill + runbook **~228 → ~228** (word swaps).

**Done when:** `rg 'plan finalize' plugins/spec/skills/finalize/` returns **0**, `rg 'plan archive' plugins/spec/skills/finalize/` returns **≥2**, and `make checks` passes.

**Rule?** no.

**Counter-argument:** Docs and RFCs still say `plan finalize`; skills should stay aspirational. It loses because operators and agents shell out to the binary that exists today; shipping a skill that invokes a missing verb is a reproducible failure mode (`command not found`), which is a verified wire-contract defect under the review rules.

**Depends on:** none.

### F5 — Delete dead `load_plan` / `PlanLoad`

**Evidence:** `crates/domain/src/change/finalize.rs:205-210,298-304` define `PlanLoad` and `load_plan`. `finalize::run` (`:216+`) takes a loaded `Plan` in its inputs and never calls `load_plan`. Doc at `:219-220` still claims callers must call `load_plan` first — stale.

Current-state grep:

```text
$ rg 'load_plan|PlanLoad' specify-cli --glob '*.rs'
crates/domain/src/change/finalize.rs:205,220,292,298,301,303
```

**Action:**
1. Delete `PlanLoad` enum (`:205-210`).
2. Delete `load_plan` fn (`:298-304`) and its doc block (`:291-297`).
3. Reword the `run` doc at `:219-220` to state the caller supplies a loaded `Plan`.

Before:

```rust
pub enum PlanLoad { Present(Plan), Missing }
pub fn load_plan(project_dir: &Path) -> Result<PlanLoad, Error> { … }
```

After:

```rust
// Caller loads plan.yaml before finalize::run.
```

**Quality delta:** `−25 LOC, −1 enum, −1 dead function, −1 stale doc branch`.

**Net LOC:** `finalize.rs` **605 → ~580**.

**Done when:** `rg 'load_plan|PlanLoad' specify-cli` returns **0** and `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.

**Rule?** no.

**Counter-argument:** W3.5 finalize will need a plan-presence guard. It loses because `run` already accepts `Plan` and the guard belongs inside `run` (or the future CLI handler), not as a second load path that nothing calls.

**Depends on:** none.

### F6 — Fix replay-writer retired lifecycle verb

**Status:** **Closed** — `plugins/rt/skills/replay-writer/` retired; replay work moved to `captures` source adapter and Omnia `build/replay.md`. Omnia `build/replay.md` documents journal-only recording (no `specify slice outcome set`).

**Evidence (historical):** `plugins/rt/skills/replay-writer/SKILL.md:35` listed `specify slice outcome set` as a lifecycle owner. RFC-25 retired that verb (`plugins/spec/references/phase-outcome-contract.md:3`).

**Depends on:** none.

### F7 — Replace composition merge production `unwrap`

**Evidence:** `crates/domain/src/merge/composition.rs:56-57` calls `.unwrap()` on a hardcoded fallback YAML literal on a production path (not `#[cfg(test)]`). Comment at `:15-18` admits the panic surface.

Current-state grep:

```text
crates/domain/src/merge/composition.rs:57:        serde_saphyr::from_str("version: 1\nscreens: {}").unwrap()
```

Production panic count before: **716** `unwrap`/`expect` matches repo-wide (includes tests); this file contributes **1** production hit.

**Action:**
1. Replace `.unwrap()` with `.map_err(|e| Error::Diag { code: "composition-baseline-fallback-malformed", detail: format!("…: {e}") })?` (mirror the adjacent baseline parse arm at `:59-62`).
2. Delete the `# Panics` section (`:15-18`) — error propagation replaces it.

Before:

```rust
serde_saphyr::from_str("version: 1\nscreens: {}").unwrap()
```

After:

```rust
serde_saphyr::from_str("version: 1\nscreens: {}").map_err(|e| Error::Diag {
    code: "composition-baseline-fallback-malformed",
    detail: format!("hardcoded empty baseline failed to parse: {e}"),
})?
```

**Quality delta:** `+4 LOC, −1 production panic site, −1 defect`.

**Net LOC:** `composition.rs` **253 → ~257**.

**Done when:** `rg '\.unwrap\(|\.expect\(' crates/domain/src/merge/composition.rs` returns **0** outside `#[cfg(test)]` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.

**Rule?** no — one site; not a repeated pattern.

**Counter-argument:** The literal cannot fail. It loses because `serde_saphyr::from_str` returns `Result` and production CLI paths must not panic on parse — same rule `cargo`/ripgrep apply to infallible-looking literals (propagate or `const` parse at compile time; here `?` is the smaller fix).

**Depends on:** none.

## One-Touch Tidies

### T1 — Delete dead `sha256_file`

**Evidence:** `crates/domain/src/adapter/cache.rs:140-146` defines `pub fn sha256_file`. `rg 'sha256_file' specify-cli` → **definition only**. `sha256_prefixed` remains the internal digest helper.

**Action:** Delete `sha256_file` and its doc block (7 lines).

**Quality delta:** `−12 LOC, −1 unused public function`.

**Done when:** `rg 'sha256_file' specify-cli` returns **0**.

**Rule?** no.

**Depends on:** none.

### T2 — Delete `sync_all` / `push_all` wrappers

**Evidence:** `sync.rs:27-33` and `push.rs:93-99` are thin wrappers. `src/commands/workspace.rs` imports only `sync_projects` / `push_projects`. Wrappers appear only in `workspace.rs` re-exports and `domain/tests/workspace.rs`.

Current-state grep:

```text
src/commands/workspace.rs:14:… sync_projects, push_projects
crates/domain/src/registry/workspace/sync.rs:27:pub fn sync_all(…)
crates/domain/src/registry/workspace/push.rs:93:pub fn push_all(…)
```

**Action:**
1. Delete both wrapper fns.
2. Drop `sync_all` / `push_all` from `registry/workspace.rs` re-exports.
3. In `domain/tests/workspace.rs`, call `sync_projects(project_dir, &registry.select(&[])?)` / `push_projects(…, &registry.select(&[])?, …)` directly.

**Quality delta:** `−14 LOC, −2 public functions, −2 call-site indirections`.

**Done when:** `rg 'sync_all|push_all' specify-cli --glob '*.rs'` returns **0** (excluding `file.sync_all()` stdlib calls).

**Rule?** no.

**Depends on:** none.

### T3 — Drop dead `change::summarise` re-export

**Evidence:** `crates/domain/src/change.rs:8` re-exports `finalize::summarise`. No importer uses `change::summarise`; tests import `finalize::summarise` directly.

**Action:** Delete line 8 (`pub use finalize::summarise;`).

**Quality delta:** `−1 LOC, −1 module-edge re-export`.

**Done when:** `rg 'change::summarise' specify-cli` returns **0**.

**Rule?** no.

**Depends on:** none.

### T4 — Drop duplicate plan-lock tail (build + merge)

**Evidence:** Identical paragraph after Critical Path in both skills:

```text
Plan-lock acquisition follows [plan-lock.md]…; env var `SPECIFY_PLAN_LOCK_HELD=1` suppresses re-acquire.
```

At `plugins/spec/skills/build/SKILL.md:47` and `plugins/spec/skills/merge/SKILL.md:49`. Critical Path step 2 already covers lock acquisition in both files.

**Action:** Delete the standalone paragraph from each skill.

**Quality delta:** `−6 LOC, −2 duplicate paragraphs`.

**Done when:** `rg 'Plan-lock acquisition follows' plugins/spec/skills/{build,merge}/SKILL.md` returns **0**.

**Rule?** no.

**Depends on:** none.

### T5 — Delete finalize execute hand-off contradiction

**Evidence:** `plugins/spec/skills/finalize/SKILL.md:29-33` claims `/spec:execute` prints `Plan drained: every entry is \`done\`. Run /spec:finalize …`. Execute's canonical closing hint (`execute/SKILL.md:16`) is the literal `drained — run /spec:finalize <name>`.

**Action:** Delete lines 29-33 (peer-routing prose that contradicts execute).

**Quality delta:** `−5 LOC, −1 cross-skill drift`.

**Done when:** `rg 'Plan drained: every entry' plugins/spec/skills/finalize/SKILL.md` returns **0**.

**Rule?** no.

**Depends on:** none.

### T6 — Fix stop-conditions partial-build lifecycle claim

**Evidence:** `plugins/spec/skills/execute/references/stop-conditions.md:9` claims build failure may leave the slice at `built` if "a partial pass landed". `build/SKILL.md:32` and Rust lifecycle (`lifecycle.rs:48-49`) require explicit `refined → built` transition — build failure keeps `refined`.

**Action:** Replace line 9 parenthetical with: "the slice stays `refined`; the plan entry stays `in-progress`."

**Quality delta:** `−2 LOC, −1 unreachable lifecycle branch`.

**Done when:** `rg 'partial pass landed|typically \`built\`' plugins/spec/skills/execute/references/stop-conditions.md` returns **0**.

**Rule?** no.

**Depends on:** none.

### T7 — Trim drop non-interactive confirmation repetition

**Evidence:** `plugins/spec/skills/drop/SKILL.md` states non-interactive rules at lines 13-15, 32, 42, 57-62 and repeats lifecycle guidance in guardrails that step 2 already covers.

**Action:**
1. Keep `## Non-interactive mode` (lines 13-15) as the single authority.
2. In steps 1-3, replace repeated "If `reason` was NOT supplied…" blocks with `(non-interactive: skip — see above)`.
3. Delete guardrail line 91 (`Always confirm…`) — it contradicts non-interactive mode already documented.

**Quality delta:** `−15 LOC, −4 duplicate confirmation branches`.

**Done when:** `wc -l plugins/spec/skills/drop/SKILL.md` drops by **≥10** and `rg 'If \`reason\` was NOT supplied' plugins/spec/skills/drop/SKILL.md` returns **≤1** hit.

**Rule?** no.

**Depends on:** none.

### T8 — Drop public re-export of internal validate registry

**Evidence:** `crates/domain/src/validate.rs:23` — `pub use registry::{cross_rules, rules_for};`. Zero importers outside the `validate` module tree; `validate_slice` is the external entry point.

**Action:** Delete the `pub use registry::{cross_rules, rules_for};` line. Leave `registry` module private; in-module tests already reach helpers via `super::`.

**Quality delta:** `−1 LOC, −2 public names on crate API surface`.

**Done when:** `rg 'pub use registry::\{cross_rules' crates/domain/src/validate.rs` returns **0** and `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.

**Rule?** no.

**Depends on:** none.

## Post-mortem

- **F1:** actual ΔLOC **−1** vs predicted **−2** (`merge/SKILL.md` 63→62); done-when flipped cleanly (0 retired flags, all three subcommands present); `make checks` passed; no regressions.
- **F2:** actual ΔLOC **−166** vs predicted **−121** (extra doc/imports removed with function); done-when flipped cleanly (`rg` 0 hits); `cargo make check` passed; no regressions (3 `TargetRef` sub-tests removed with module — behavior still covered elsewhere).
- **F3:** actual ΔLOC **−62** vs predicted **−60** (`refine/SKILL.md` 119→57); done-when flipped cleanly (0 step headers, 57≤65 lines); `make checks` passed; no regressions — operator detail folded into Critical Path bullets, no synthesis file edits needed.
- **F4:** actual ΔLOC **0** vs predicted **0** (word swaps only, 18 lines touched); done-when flipped cleanly (`plan finalize` 0, `plan archive` 14); `make checks` passed; no regressions — archive path corrected to `.yaml` suffix.
- **F5:** actual ΔLOC **−26** vs predicted **−25** (`finalize.rs` 305→279; REVIEW baseline line count was stale); done-when flipped cleanly (`rg` 0 hits); `cargo make check` passed; no regressions.
- **F6:** actual ΔLOC **0** vs predicted **−1** (intra-line edit, no line removed); done-when flipped cleanly (`outcome set` 0 hits); `make checks` passed; no regressions.
- **F7:** actual ΔLOC **−3** vs predicted **+4** (`composition.rs` 253→250; `# Panics` removal offset the `map_err` block); done-when flipped cleanly (0 production unwrap/expect); `cargo make check` passed; no regressions.
- **T1:** actual ΔLOC **−14** vs predicted **−12** (`cache.rs`); done-when flipped cleanly (`sha256_file` 0 hits); `cargo make check` passed; no regressions.
- **T2:** actual ΔLOC **−31** vs predicted **−14** (wrapper + unused imports + test rewrites); done-when flipped cleanly (only stdlib `file.sync_all()` remain); `cargo make check` passed after intra-doc link fix; no regressions.
- **T3:** actual ΔLOC **−1** vs predicted **−1** (`change.rs`); done-when flipped cleanly (`change::summarise` 0 hits); `cargo make check` passed; no regressions.
- **T4:** actual ΔLOC **−2** vs predicted **−6** (plan-lock tail already absent at pass start in one skill; both skills now 0 hits); done-when flipped cleanly; `make checks` passed; no regressions.
- **T5:** actual ΔLOC **−5** vs predicted **−5** (deleted contradictory peer-routing block); done-when flipped cleanly (`Plan drained` 0 hits); `make checks` passed; no regressions.
- **T6:** actual ΔLOC **−2** vs predicted **−2** (lifecycle parenthetical corrected); done-when flipped cleanly; `make checks` passed; no regressions.
- **T7:** actual ΔLOC **−2** vs predicted **−15** (87→85; non-interactive dedup largely pre-trimmed; removed duplicate IMPORTANT confirm in step 1); done-when partially met (`reason` repetition 0 hits; line drop −2 not −10 — baseline already lean); `make checks` passed; no regressions.
- **T8:** actual ΔLOC **0** vs predicted **−1** (moved `cross_rules`/`rules_for` import into test module); done-when flipped cleanly; `cargo make check` passed; no regressions.

## Notes

Deliberately **not** flagged:

- **RFC-27 §D8 extraction cache I/O** (`adapter/cache/io.rs:99-328`) — `lookup` / `write` / `append_index` have no production callers yet but are tested end-to-end and wired for imminent emit sites; deleting would re-implement D8 later. Same rationale as the prior pass.
- **`change/finalize` domain module (~605 LOC)** — no CLI verb yet (`tests/cross_repo.rs` marks W3.5 future); deleting is strategic scope, not a safe subtraction.
- **`ExampleClaim` / `AuthorityClass` scaffolds** — zero external callers but RFC-27 evidence/authority shapes; YAGNI deletion risks re-adding typed claim surfaces when `captures` extract lands.
- **Journal `EventKind` variants without CLI emitters** (`SliceExtractCompleted`, `SliceFusionWritten`, …) — refine/merge/replay-writer skills instruct agents to append matching NDJSON; the enum is the deserialization contract even when the CLI does not construct them.
- **Dependency deduplication** — transitive duplicates under `wasmtime` / `wasm-pkg-client`; workspace has no upgrade authority inside this pass and `Cargo.toml` is frozen.
- **New xtask predicates** for skill wire-contract drift — ruled out by master "Do NOT propose."

`make checks` (`specify`) and `cargo make check` (`specify-cli`) both passed at recon start; no Skill-integrity predicate failure qualified — merge/finalize/replay drift is content-level wire-contract closure, not a failing `make checks` id.
