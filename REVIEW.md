# Code & Skill Review — specify + specify-cli

Pre-1.0 pass; quality-biased; subtraction-first. Reconnaissance run 2026-05-23.

## Summary

1. **Top three (by sort key):** (a) **Shared plan dependency graph** — one builder replaces three copies in `validate` / `doctor/cycle` / `next` (−28 LOC); (b) **Synthesis journal belongs in `slice validate`** — closes DECISIONS.md wire-contract drift (+10 net LOC); (c) **`plan add`/`amend` `.expect` on operator path** — replace with existing `entry_mut` (0 LOC, −3 panic sites).
2. **Total ΔLOC if all findings land:** approximately **−95 LOC** (structural subtractions −105; defect-only portfolio **+10** after paired skill trim).
3. **Primary non-LOC axes:** panic surface (−4 operator-adjacent sites), types (−1 dead enum variant), defect surface (−2 wire/doc mismatches), call-site burden (agents stop hand-appending synthesis journal lines).
4. **Verified defects closed:** 2 qualified (`plan/create` expect triplet; DECISIONS.md `slice.synthesis.*` emitter vs `slice validate` implementation). **1 still open** if graph-sharing finding is skipped (non-defect). Defect-only net ΔLOC: **+10** (≤ +30 cap).
5. **Most likely to break in remediation:** **F1 (shared dependency graph)** — cycle message formatting and self-loop handling differ slightly across validate vs doctor today; a rushed merge can regress golden cycle strings.

---

## Reconnaissance

| Metric | specify | specify-cli |
|---|---|---|
| **tokei total** | 86,773 lines / 647 files | 66,138 lines / 454 files (Rust: 48,640) |
| **make checks / cargo make check** | All checks passed (0 failures) | 853 passed, 2 skipped |
| **`#[test]` count** | — | 515 |
| **`mod.rs` files** | — | 3 |
| **docs/standards + AGENTS.md** | 534 lines | 639 lines |
| **unwrap/expect** (crates/ + src/, incl. co-located tests) | — | 738 |
| **panic!/unreachable!** (same scope) | — | 50 |
| **Files >500 LOC** (crates/, src/) | — | 12 (largest: `workspace.rs` test 1041, `plan/create.rs` 974, `fusion.rs` 906) |
| **cargo tree --duplicates** | — | base64 0.21/0.22, bitflags 2.x, rustix 0.38/1.1, reqwest 0.12/0.13, lazy_static 0.2/1.5 (frozen — no action) |

---

## Structural findings

### F1 — Share plan dependency graph

**Evidence:** Three hand-rolled `DiGraph` builders with identical node/edge insertion: `crates/domain/src/change/plan/core/validate.rs:76–94` (`detect_cycles`), `crates/domain/src/change/plan/doctor/cycle.rs:53–68` (`build_graph`), `crates/domain/src/change/plan/core/next.rs:100–113` (`topological_order`). Recon: validate.rs 409 LOC, cycle.rs 69 LOC, next.rs 385 LOC.

**Action:**
1. Add one `pub(super) fn entry_dependency_graph(entries: &[Entry]) -> DiGraph<&str, ()>` in `validate.rs` (or `model.rs` if doctor cannot import validate — prefer `core/model.rs` adjacent to `Entry`).
2. Replace inline graph construction in all three call sites with one call.
3. Delete `build_graph` from `doctor/cycle.rs`.
4. Run `cargo make check`; cycle-related tests in `validate/tests.rs` and `doctor/tests.rs` must stay green.

**Quality delta:** −28 LOC, −2 branches (duplicate edge loops), −2 module edges (doctor stops owning graph copy).

**Net LOC:** ~55 duplicated insert lines → ~27 shared helper (**974→946** across touched files).

**Done when:** `rg 'graph.add_node\(entry.name' crates/domain/src/change/plan` returns **1** hit (inside the shared helper only).

**Rule?** no — one-off triplication, not a recurring pattern elsewhere.

**Counter-argument:** validate uses a toposort fast-path before SCC enumeration; doctor always runs tarjan. **Rebuttal:** shared builder does not force shared enumeration; only the 15-line insert loop is duplicated today.

**Depends on:** none

---

### F2 — Emit synthesis journal in slice validate

**Evidence:** Wire contract — `specify-cli/DECISIONS.md` journal table row: `slice.synthesis.conflict / .divergence / .unknown` → **Emitted by `specify slice validate`**. Implementation: `src/commands/slice/validate.rs` never calls `journal::append_batch`. Skill drift: `plugins/spec/skills/refine/SKILL.md:58` instructs agents to append synthesis events manually. Test comment at `tests/journal.rs:347` still documents agent emit for synthesis events.

**Action:**
1. In `validate_spec_provenance` (or immediately after it succeeds inside `run`), scan parsed requirements for `RequirementTag` (`crates/domain/src/spec/provenance.rs:98–105`) and append one `journal::Event` per tagged requirement via `journal::append_batch`.
2. Map `Unknown|Conflict|Divergence` → `EventKind::SliceSynthesis*` (`journal.rs:121–142`).
3. Delete refine skill step 4 journal paragraph (line 58) and the synthesis bullet in the step 3 extract journal example if it references synthesis (keep `slice.extract.completed` — still skill-owned per DECISIONS).
4. Add/adjust integration test in `tests/journal.rs` so `specify slice validate` on a fixture spec with tags appends events (replace agent-emit-only coverage for synthesis).

**Quality delta:** −1 defect (wire-contract), −1 call-site burden (agents), +1 defect surface closed in tests; skill body −6 LOC.

**Net LOC:** validate.rs **+22**, provenance helper **+6**, refine SKILL **−6**, tests **±0** → **+22** CLI, **−6** skill (defect portfolio **+16** before test trim; target **+10** with compact helper).

**Done when:** `rg 'slice.synthesis' src/commands/slice/validate.rs` ≥1 match **and** `rg 'slice.synthesis.*journal' plugins/spec/skills/refine/SKILL.md` returns **0** matches **and** `make checks` passes.

**Rule?** no

**Counter-argument:** Agent emit avoids validate-time side effects on dry validation runs. **Rebuttal:** DECISIONS already assigns ownership to validate; idempotent append on successful validate matches `slice.transition.refined` pattern.

**Depends on:** none

---

### F3 — Replace plan handler `.expect` with `entry_mut`

**Evidence:** Operator-reachable CLI handler panics — `src/commands/plan/create.rs:712,850,894` use `.expect(...)` inside `with_state` closures for `plan add` / `plan amend`. Recon panic-adjacent count: **738** total (includes tests); these **3** are on the non-test hot path (`rg '\.expect\(' src/commands/plan/create.rs` → 3, file has no `#[cfg(test)]`). Existing helper: `entry_mut` at `:337–342` already returns `Result` with `unknown_slice_err`.

**Action:**
1. Line 712 (`add`): after `plan.create(entry)?; validate_plan(plan)?;` replace `.last().expect(...)` with `let created = entry_mut(plan, &plan.name, name)?;`
2. Line 850 (`amend`): replace `.find(...).expect("amended entry present")` with `entry_mut(plan, &plan.name, &name)?`
3. Line 894: replace `.find(...).expect("amended entry present")` with read-only `plan.entries.iter().find(...).ok_or_else(|| unknown_slice_err(&plan_name, &name))?`

**Before (712):**
```rust
let created =
    plan.entries.last().expect("Plan::create appended an entry that is now missing");
```

**After:**
```rust
let created = entry_mut(plan, &plan.name, name)?;
```

**Quality delta:** −3 panic surface, −1 defect (operator panic path).

**Net LOC:** 974 → **971** (`create.rs`).

**Done when:** `rg '\.(unwrap|expect)\(' src/commands/plan/create.rs` returns **0** (was **3**).

**Rule?** no

**Counter-argument:** `expect` documents an internal invariant after `Plan::create`. **Rebuttal:** `entry_mut` encodes the same invariant as `Error::Diag` — ripgrep/cargo pattern; panics on CLI paths fail the review gate.

**Depends on:** none

---

### F4 — Deduplicate composition rule YAML parse

**Evidence:** `crates/domain/src/validate/registry/composition.rs` parses `ctx.content` independently in four check fns (`:6, :15, :35, :59`) with identical `Err(_) => RuleOutcome::Fail { detail: "not valid YAML" }` arms (131 LOC file).

**Action:**
1. Add `fn parse_composition(ctx: &BriefContext<'_>) -> Result<serde_json::Value, RuleOutcome>` once at top of file.
2. Replace four inline `serde_saphyr::from_str` blocks with `parse_composition(ctx)?` or early-return on `composition_valid_yaml` only.
3. Keep rule IDs unchanged.

**Quality delta:** −18 LOC, −3 branches (duplicate error arms).

**Net LOC:** 131 → **113** (`composition.rs`).

**Done when:** `rg 'not valid YAML' crates/domain/src/validate/registry/composition.rs` returns **1** match (inside helper only).

**Rule?** no

**Counter-argument:** Per-rule isolation keeps failures independent. **Rebuttal:** rules already run sequentially on the same bytes; one parser does not couple outcomes.

**Depends on:** none

---

### F5 — Delete dead stale-clone reason variant — **RESOLVED**

**Evidence:** `crates/domain/src/change/plan/doctor.rs` — unused `StaleReason` variant retained for old JSON consumers. Doc drift in `specify/docs/reference/cli/plan.md:51`.

**Action:** Removed dead enum variant; aligned plan.md with live `signature-changed` / `slot-mismatch` reasons.

**Quality delta:** −5 LOC, −1 type, −1 defect (doc/code drift).

**Rule?** no — pre-1.0, user waived back-compat.

**Depends on:** none

---

## One-touch tidies

### T1 — OnceLock kebab slug regex in composition rules

**Evidence:** `composition.rs:67` — `Regex::new(...).unwrap()` on every rule invocation. Elsewhere the domain crate uses `OnceLock` (`validate/primitives.rs:21–25`, `merge/validate.rs:21–24`, `task.rs:53`).

**Action:** Add `fn slug_re() -> &'static Regex` with `OnceLock`, matching `task.rs` pattern; replace line 67.

**Quality delta:** −1 panic surface (operator validation path), +1 idiomatic (cargo-style OnceLock).

**Net LOC:** 131 → **133** (+2 — allowed: paired with panic reduction).

**Done when:** `rg 'Regex::new' crates/domain/src/validate/registry/composition.rs` returns **0**.

**Rule?** no

**Counter-argument:** Compile cost is negligible. **Rebuttal:** crate already standardized on OnceLock; one call site is the outlier.

**Depends on:** F4 (same file; land together)

---

### T2 — Consolidate RFC3339 test fixture helper

**Evidence:** Identical test helper duplicated 5×: `adapter/cache.rs:261`, `adapter/cache/io.rs:340`, `journal.rs:388`, `slice/fusion.rs:498`, `slice/metadata.rs:188` — all `raw.parse().expect("valid rfc3339 timestamp in test fixture")`.

**Action:** Add `pub(super) fn test_timestamp(raw: &str) -> jiff::Timestamp` to `crates/domain/src/change/plan/core/test_support.rs` (already exists for plan fixtures) **or** a single `crates/domain/src/test_util.rs` is forbidden (new file). **Preferred:** add `test_timestamp` to `journal.rs` as `pub(crate)` and import from sibling test modules.

**Quality delta:** −12 LOC, −4 duplicate helpers.

**Net LOC:** 5×4 lines → 1×4 + 5 imports ≈ **−7**.

**Done when:** `rg 'valid rfc3339 timestamp in test fixture' crates/domain/src` returns **1** match.

**Rule?** no

**Counter-argument:** Copy-paste keeps test modules self-contained. **Rebuttal:** jj/cargo consolidate time fixtures once test count >3.

**Depends on:** none

---

### T3 — Trim guardrail restatements in phase skills

**Evidence:** `docs/standards/skill-authoring.md` + `plugins/references/guardrails.md:3` — skills should **link** guardrails, not restate. Repeated "Never hand-edit `.metadata.yaml`" bullets: `refine/SKILL.md:113`, `build/SKILL.md:56`, `merge/SKILL.md:56` (each ~2 lines).

**Action:** Replace each bullet with one line: `- **Lifecycle single-writer:** [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state).` Keep merge-only bullets (post-merge validator, `--check-only`).

**Quality delta:** −6 LOC (skill bodies), −1 call-site burden (less token noise per invoke).

**Net LOC:** refine 113→**112**, build 59→**58**, merge 61→**59**.

**Done when:** `rg 'Never hand-edit .metadata.yaml' plugins/spec/skills/{refine,build,merge}/SKILL.md` returns **0**.

**Rule?** yes — already partially enforced by `skill_body.ts` duplicate detection; extend only if new violations appear after edit.

**Counter-argument:** Models skip linked docs. **Rebuttal:** Critical Path already carries must-not-break rules; guardrails H2 becomes a pointer.

**Depends on:** none

---

### T4 — Collapse plan-lock prose in build/merge skills

**Evidence:** `build/SKILL.md:48–50` and `merge/SKILL.md:50–52` duplicate `plugins/spec/skills/execute/references/plan-lock.md` (also cited in Critical Path step 2).

**Action:** Replace each `## Plan-lock semantics` section (~3 lines) with: `Plan-lock acquisition follows [plan-lock.md](../../references/plan-lock.md); env var \`SPECIFY_PLAN_LOCK_HELD=1\` suppresses re-acquire.`

**Quality delta:** −10 LOC, −1 axis (skill body cap headroom).

**Net LOC:** build 59→**54**, merge 61→**56**.

**Done when:** `wc -l plugins/spec/skills/{build,merge}/SKILL.md` drops by ≥5 each vs current 59/61.

**Rule?** no

**Counter-argument:** Standalone invocations need inline snippet. **Rebuttal:** Critical Path step 2 already points at plan-lock.md; duplication fails house skill-authoring discipline.

**Depends on:** none

---

### T5 — Use `is_kebab` in one place for plan names

**Evidence:** `src/commands/plan/create.rs:53–64` defines `require_kebab_change_name` with code `change-name-not-kebab`; domain slice create uses `invalid-name` (`slice/actions/create.rs:66`). Two kebab gates, two codes — not a bug, but CLI duplicates `specify_error::is_kebab` logic already imported at `:16`.

**Action:** Inline `require_kebab_change_name` body to 4 lines using `is_kebab` (already imported); do **not** merge error codes (tests lock `change-name-not-kebab` at `tests/plan_orchestrate.rs:919`).

**Quality delta:** −8 LOC, −1 branch (redundant function wrapper).

**Net LOC:** create.rs 974→**966** (if F3 not yet landed, cumulative).

**Done when:** `rg 'fn require_kebab_change_name' src/commands/plan/create.rs` returns **0** and `change-name-not-kebab` tests still pass.

**Rule?** no

**Counter-argument:** Named helper reads clearer at call site. **Rebuttal:** Single call site (`:559`); named function buys nothing.

**Depends on:** none

---

### T6 — Drop redundant `composition_valid_yaml` when later rules run

**Evidence:** Not recommended — would change rule granularity. **Dropped** (fails "improves readability" bar).

---

### T7 — `path_hint` uses lossy join (keep)

**Evidence:** `slice/validate.rs:248–254` — already non-panicking. **Dropped.**

---

### T8 — Refine skill step bodies vs Critical Path

**Evidence:** `make checks` passed; `skill_body.ts` duplicate predicate did not fire on refine (113 body lines). **No action** — predicate clean.

---

### T9 — Adapter cache `canonical_bytes` expect

**Evidence:** `adapter/cache.rs:84` — documented `# Panics` on closed serde type; not operator path without digest call. **Dropped** (invariant panic, not defect).

---

### T10 — `topological_order` indegree expect

**Evidence:** `next.rs:141` — post-`toposort` DAG walk; only reachable from tests/diagnostics, not `plan next` hot path (`advance_next` uses `next_eligible`). **Dropped.**

---

## Findings dropped (burden of proof)

| Idea | Why dropped |
|---|---|
| Merge `tool/dto.rs` into handlers | DTOs carry serde shapes; deletion breaks handler-shape pattern |
| Split `plan/create.rs` into new module | Violates "no new modules" rule; file long but cohesive |
| Dedupe `cargo tree` duplicate deps | Requires `Cargo.toml` dependency surgery (frozen) |
| Add `specify slice journal append` verb | Adds CLI surface (+LOC) to fix agent hand-roll |
| Move authority-override journal helpers to domain | Net +LOC; CLI-specific event batching |
| New xtask/clippy for `.expect` in handlers | User forbids new mechanical enforcement |

---

## Execution order (suggested)

1. **F3** — smallest defect, zero risk (−3 expect).
2. **F4 + T1** — same file.
3. **F1** — run cycle tests after graph extract.
4. **F2** — wire contract; run `tests/journal.rs`.
5. **F5** — enum + doc.
6. **T2–T5** — skills/docs tidies; `make checks` after skill edits.

---

## Verification commands (post-remediation)

```bash
# specify plugin repo
cd /Users/andrewweston/github.com/augentic/specify && make checks

# specify-cli
cd /Users/andrewweston/github.com/augentic/specify-cli && cargo make check

# Panic regression on plan handler
rg '\.(unwrap|expect)\(' src/commands/plan/create.rs  # expect 0

# Graph dedup
rg 'graph.add_node\(entry.name' crates/domain/src/change/plan  # expect 1

# Wire contract
rg 'slice.synthesis' src/commands/slice/validate.rs  # expect ≥1
```

---

## Post-mortem

| Finding | Predicted ΔLOC | Actual ΔLOC | Done-when | Regressions |
|---|---|---|---|---|
| F3 | −3 (`create.rs` 974→971) | −2 (975→973); borrow-scoping block in `add` | ✓ `rg expect` → 0 | None; `cargo make check` green |
| F4+T1 | F4 −18 (131→113); T1 +2 | F4 Δ0 (131→131); T1 OnceLock in `primitives.rs` (+7); `composition.valid-yaml` detail now generic `"not valid YAML"` | ✓ both `rg` assertions | None; `cargo make check` green |
| F1 | −28 | −24 net (−49/+25 across validate, next, cycle); helper in `validate.rs` | ✓ `rg graph.add_node` → 1 | None; 16 cycle golden tests unchanged |
| F2 | +22 CLI, −6 skill | validate +65, provenance +8, journal tests +68 net; skill −2 net | ✓ all `rg` + both test suites | None; journal only on full validate pass |
| F5 | −5 doctor, −1 plan.md | −4 doctor (181→177); plan.md in-place swap | ✓ repo-wide `rg` → 0 | None; `cargo make check` green |
| T2 | ~−7 | −11 net; helper in `journal.rs` | ✓ `rg` fixture string → 1 | None; fixed accidental `.exists()` corruption during rename |
| T3 | −6 skill bodies | +1 net (merge kept sole-writer bullet + link); refine/build 1:1 swap | ✓ `rg Never hand-edit` → 0; `make checks` green | None |
| T4 | build −5, merge −5 | 64→59, 67→62 (−5 each); also trimmed redundant guardrail/References dupes | ✓ `wc -l` drop ≥5 each; `make checks` green | None |
| T5 | −8 (`create.rs` 974→966) | −7 (973→966); inlined 4-line `is_kebab` guard | ✓ `rg fn require_kebab` → 0; `change-name-not-kebab` test green | None; `cargo make check` green |
