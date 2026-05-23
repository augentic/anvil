# Code & Skill Review — single pass

1. **Top three:** S2 Delete orphan init sidecars; S1 Delete retired adapter reference; S3 Inline plan entry JSON detour.
2. **Total ΔLOC if all land:** about **−147 LOC**.
3. **Primary non-LOC axes moved:** −5 operator-panic sites, −3 stale reference/module edges, −1 wire-contract defect, less JSON plumbing at plan call sites.
4. **Top verified defects closed:** stale `specify adapter *` wire docs, 2 plan-entry serialization `expect`s, 2 journal/cache serialization `expect`s; defect-only positive LOC is **+14**.
5. **Most likely remediation break:** S5, because changing journal/cache serialization errors touches append paths that many commands share.

## Reconnaissance

- `tokei` from `specify`: **658 files / 90,281 lines**; Markdown is **524 files / 50,577 lines**.
- `tokei` from `specify-cli`: **485 files / 71,325 lines**; Rust is **277 files / 53,915 lines**.
- `cargo tree --duplicates` from `specify-cli`: duplicates are transitive and heavy (`base64 0.21/0.22`, `reqwest 0.12/0.13`, `rustix 0.38/1.1`, `thiserror 1/2`, `wasmparser 0.121/0.244/0.246`, etc.); frozen for this pass.
- `rg -c '^#\[test\]' crates/ src/ tests/` from `specify-cli` returns per-file counts totaling **604** test attributes.
- `rg --files -g '**/mod.rs'` from `specify-cli`: `tests/common/mod.rs`, `wasi-tools/vectis/tests/engine_support/mod.rs`, `crates/domain/tests/common/mod.rs`.
- `wc -l docs/standards/*.md AGENTS.md`: `specify` **534 total**; `specify-cli` **637 total**.
- Files >500 lines under `crates/` and `src/` in `specify-cli`: `src/commands/plan/create.rs` 1025; `crates/domain/src/slice/fusion.rs` 906; `crates/domain/src/journal.rs` 619; `crates/tool/src/validate.rs` 520; `crates/domain/src/discovery/document.rs` 915; `crates/domain/src/spec/provenance.rs` 599; `crates/domain/src/adapter/cache.rs` 503; `crates/domain/src/adapter/cache/io.rs` 508.
- `make checks` from `specify`: `All checks passed.` Total failures: **0**. First 5 predicate ids: none.
- `cargo make check` from `specify-cli`: failed in `nextest` dep-info generation after its own `cargo clean`; first error: `error: could not parse/generate dep info at: .../target/debug/deps/equivalent-238f08d9fcb31728.d` followed by `No such file or directory (os error 2)`. A direct `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' crates/ src/` from `specify-cli`: per-file counts total **763** occurrences.
- `rg -c 'panic!|unreachable!' --glob '!**/tests/**' crates/ src/` from `specify-cli`: per-file counts total **50** occurrences.

## Structural Findings

### S1 — Delete Retired Adapter Reference

**Evidence:** `plugins/spec/references/adapter-resolution.md:5-7` documents `specify adapter resolve` and `specify adapter pipeline`; `rfcs/archive/rfc-25-workflow.md:730` says `specify source resolve` / `specify target resolve` replaced `adapter resolve` and `specify adapter pipeline` retired. Current CLI code agrees: `specify-cli/src/cli.rs:82-98` exposes `Source` and `Target` subcommands, not `Adapter`. Current line count: `wc -l plugins/spec/references/adapter-resolution.md plugins/spec/README.md` contributes `32` + `22` lines. Recon: `make checks` passes, so this stale wire doc is not caught by predicates.

Current bad span:

```md
- `specify adapter resolve <adapter-value> --format json` → returns the resolved directory path plus a `source` flag (`local` | `cached`).
- `specify adapter pipeline <phase> [--change <dir>] --format json` → returns the brief topology for a phase plus absolute paths to every brief markdown file.
```

**Action:**
1. Delete `plugins/spec/references/adapter-resolution.md`.
2. Delete the `Adapter Resolution` row from `plugins/spec/README.md`.

**Quality delta:** `−33 LOC, −1 wire-contract defect, −1 stale reference edge, −1 false operator command path`.

**Net LOC:** `54 → 21` across touched files.

**Architectural impact:** This removes the retired 1.x command surface instead of teaching agents both the old axis-free and current source/target resolver vocabulary.

**Done when:** `test ! -e plugins/spec/references/adapter-resolution.md && ! rg -q 'Adapter Resolution' plugins/spec/README.md`

**Rule?** no.

**Counter-argument:** A historical reference might help migration archaeology; it loses because pre-1.0 explicitly does not preserve retired command docs in shipped skill references.

**Depends on:** none.

### S2 — Delete Orphan Init Sidecars

**Evidence:** `wc -l plugins/spec/references/topology-flow.md plugins/spec/references/baseline-detection.md plugins/spec/skills/init/SKILL.md` reports `89`, `35`, and `35` lines. `rg 'topology-flow|baseline-detection|init-output-templates|init-runbook|adapter-resolution'` shows `topology-flow.md` is only self-referenced, and `baseline-detection.md` is only referenced by that orphan. The live init skill already carries the same critical facts in `plugins/spec/skills/init/SKILL.md:11-29`, and `plugins/spec/references/init-runbook.md:143-173` duplicates the baseline-detection procedure.

Current redundant span:

```md
After a regular init, the skill optionally detects existing code indicators (`Cargo.toml`, `package.json`, `src/`, etc.) and offers to create an `initial-baseline` slice via `specify slice create`.
```

**Action:**
1. Delete `plugins/spec/references/topology-flow.md`.
2. Delete `plugins/spec/references/baseline-detection.md`.
3. Shorten `plugins/spec/skills/init/SKILL.md:29` so it points only at `references/init-runbook.md` and `../../references/init-output-templates.md`; remove the claim that the runbook links topology flow, adapter resolution, and baseline detection.

**Quality delta:** `−124 LOC, −2 reference/module edges, −2 duplicate procedural surfaces`.

**Net LOC:** `159 → 35` across touched files.

**Architectural impact:** `/spec:init` already has three layers of instruction; deleting the two unreachable sidecars leaves one operational runbook plus the short skill body.

**Done when:** `test ! -e plugins/spec/references/topology-flow.md && test ! -e plugins/spec/references/baseline-detection.md && ! rg -q 'topology flow|baseline detection|adapter resolution' plugins/spec/skills/init/SKILL.md`

**Rule?** no.

**Counter-argument:** Smaller sidecars are easier to skim than the runbook; it loses because the sidecars are not actually linked from the skill path and already disagree by omission with the runbook.

**Depends on:** none.

### S3 — Inline Plan Entry JSON

**Evidence:** Recon panic-adjacent surface: `rg -o '\.(unwrap|expect)\(' --glob '!**/tests/**' crates/ src/ | wc -l` returns **763**. `specify-cli/src/commands/plan/create.rs:793` and `:969` call `serde_json::to_value(...).expect("plan Entry serialises as JSON")` on the CLI `plan add` / `plan amend` path. The only reason is `EntryBody.entry: Value` at `create.rs:1013-1017`, then `write_entry_text` reads the name back out with `Value::get` at `create.rs:1019-1023`. The domain type is already serializable and clonable at `crates/domain/src/change/plan/core/model.rs:113-116`.

Current shape:

```rust
entry: serde_json::to_value(created).expect("plan Entry serialises as JSON"),
// ...
entry: Value,
// ...
let name = body.entry.get("name").and_then(Value::as_str).unwrap_or("");
```

**Action:**
1. Remove `use serde_json::Value` from `src/commands/plan/create.rs`.
2. Store `Entry` directly in `EntryBody` (`entry: Entry`).
3. Replace both `serde_json::to_value(...).expect(...)` call sites with `created.clone()` / `amended.clone()`.
4. Replace the text writer lookup with `let name = &body.entry.name;`.

**Quality delta:** `−5 LOC, −2 panic surface, −1 module edge, −2 serialization call-site burdens`.

**Net LOC:** `1025 → about 1020`.

**Done when:** `! rg -q 'plan Entry serialises|entry: Value|use serde_json::Value' src/commands/plan/create.rs`

**Rule?** no.

**Counter-argument:** JSON `Value` keeps the output DTO decoupled from the domain model; it loses because the DTO only needs the already-public `Entry` shape and the current detour adds two production `expect`s to print a name.

**Depends on:** none.

### S4 — Cast Clamped Tool Exit

**Evidence:** Recon panic-adjacent surface includes `src/commands/tool/run.rs:29`: `u8::try_from(exit.clamp(0, 255)).expect("tool exit code is clamped to u8 range")`. This is reachable from `specify tool run`, a CLI handler that returns the guest exit byte. The value has already been clamped into the `u8` range, so the checked conversion plus `expect` buys no operator behavior.

Current shape:

```rust
Ok(u8::try_from(exit.clamp(0, 255)).expect("tool exit code is clamped to u8 range"))
```

**Action:**
1. Replace it with `Ok(exit.clamp(0, 255) as u8)`.

**Quality delta:** `±0 LOC, −1 panic surface, −1 conversion branch, −1 call-site burden`.

**Net LOC:** `30 → 30`.

**Done when:** `! rg -q 'clamped to u8 range|try_from\\(exit\\.clamp' src/commands/tool/run.rs`

**Rule?** no.

**Counter-argument:** `TryFrom` documents the range invariant; it loses because the `clamp` already documents and enforces the invariant, and `cargo`-tier CLIs should not carry an abort path for a normalized process status.

**Depends on:** none.

### S5 — Return Serialization Errors

**Evidence:** Recon panic-adjacent surface includes `crates/domain/src/journal.rs:336` (`serde_json::to_string(event).expect("Event serialises as JSON")`) and `crates/domain/src/adapter/cache/io.rs:282` (`serde_json::to_string(entry).expect("CacheIndexEntry serialises as JSON")`). Both are phase-critical append paths: `journal::append_batch` is called by plan/slice handlers, and `append_index` is called by source cache writes. `crates/error/src/error.rs:106-120` only has `Io`, `YamlDe`, and `YamlSer` transparent conversions, so bare `?` cannot replace these expects without adding a new error variant.

Current shape:

```rust
let line = serde_json::to_string(event).expect("Event serialises as JSON");
let line = serde_json::to_string(entry).expect("CacheIndexEntry serialises as JSON");
```

**Action:**
1. In `journal::append_batch`, replace the `expect` with `map_err(|err| Error::Diag { code: "journal-event-serialise-failed", detail: format!("failed to serialise journal event: {err}") })?`.
2. In `adapter::cache::io::append_index`, replace the `expect` with `map_err(|err| Error::Diag { code: "cache-index-entry-serialise-failed", detail: format!("failed to serialise cache index entry: {err}") })?`.

**Quality delta:** `+8 LOC, −2 panic surface, −2 aborting phase-critical paths, +2 structured defect reports`.

**Net LOC:** `1127 → about 1135` across the two files.

**Done when:** `! rg -q 'Event serialises as JSON|CacheIndexEntry serialises as JSON' crates/domain/src/journal.rs crates/domain/src/adapter/cache/io.rs`

**Rule?** no.

**Counter-argument:** These serde failures are unreachable with closed derives; it loses because the fix is the smallest structured error path, stays within the +8 defect budget, and removes two CLI-reachable aborts.

**Depends on:** none.

## One-Touch Tidies

### T1 — Soften Alias Rollback Expect

**Evidence:** `crates/domain/src/discovery/document.rs:220-254` documents and uses `self.candidate_mut(candidate_id).expect("candidate located above")` after adding an alias, then finding a whole-document collision. This is reachable from `specify plan amend --add-alias`. Recon panic-adjacent total is **763** `unwrap`/`expect` occurrences.

**Action:** Replace the `expect` rollback with an `if let Some(candidate) = self.candidate_mut(candidate_id) { candidate.remove_alias(alias); }` block; keep returning `Self::collision_error(&collisions)`.

**Quality delta:** `+2 LOC, −1 panic surface`.

**Net LOC:** `915 → 917`.

**Done when:** `! rg -q 'candidate located above' crates/domain/src/discovery/document.rs`

**Rule?** no.

**Counter-argument:** The panic protects an owned-state invariant; it loses because the rollback is already on an operator error path and a missing rollback target can still return the collision cleanly.

**Depends on:** none.

### T2 — Avoid Transition Re-Find Expect

**Evidence:** `src/commands/plan/lifecycle.rs:232-244` finds the entry to capture `previous`, calls `plan.transition`, then re-finds it with `expect("just transitioned")`. This is reachable through `specify plan transition <entry> done`.

**Action:** Capture the entry index with `position` before calling `plan.transition`, read `previous` from `plan.entries[idx]`, call `plan.transition`, then read the post-transition entry via the same index.

**Quality delta:** `+3 LOC, −1 panic surface, −1 duplicate lookup`.

**Net LOC:** `433 → 436`.

**Done when:** `! rg -q 'just transitioned' src/commands/plan/lifecycle.rs`

**Rule?** no.

**Counter-argument:** Re-finding after mutation is obvious and should be impossible to fail; it loses because the index version is still local and removes one production `expect` plus one full scan.

**Depends on:** none.

### T3 — Pattern-Match Context Bytes

**Evidence:** Retired with the public context-check surface; init-time context generation no longer uses this read path.

**Action:** Replace the second `expect` with a `let Some(agents) = agents.as_deref() else { ...same context-not-generated return... };` pattern and reuse the existing no-generated body.

**Quality delta:** `+1 LOC, −1 panic surface`.

**Net LOC:** `143 → 144`.

**Done when:** `! rg -q 'agents bytes present' src/commands/context/check.rs`

**Rule?** no.

**Counter-argument:** The early return already proves the option is `Some`; it loses because `let-else` states that proof in the type flow without a CLI abort surface.

**Depends on:** none.

## Dropped

- Do not chase `cargo tree --duplicates`: the worst duplicates come through Wasmtime / Warg / reqwest stacks and `Cargo.toml` / `Cargo.lock` are frozen for this pass.
- Do not make a finding from the `cargo make check` failure: direct clippy is green, and the failure is a dep-info write race after `cargo clean`, not a code span.
- Do not split `src/commands/plan/create.rs`: it is 1025 lines, but the current `#[expect(clippy::too_many_lines)]` documents the atomic `with_state` trade; a file split would add module edges without deleting duplicated logic.
- Do not change `CacheFingerprint::digest`: removing its `expect` requires returning `Result<String, Error>` through cache lookup/write callers and tests, which exceeds the +8 defect-only budget without paired deletion.
# Code & Skill Review — quality-balanced pass

## Summary

1. **Top three by impact score:** (S1) `slice merge run` never stamps plan entry `done` despite RFC/skills claiming merge owns it — **9**; (S5) plan validate workspace check parses slot `project.yaml` as untyped `serde_json::Value` instead of `ProjectConfig` — **7**; (S2) plan skill denies `specify plan validate` while the CLI ships it — **4**.
2. **Total ΔLOC if all land:** ≈ **+35 net** (quality fixes dominate; largest subtraction is S7 at −407 in `model.rs` with +407 relocated to `model/tests.rs`).
3. **Primary quality axes moved (8–14):** Correctness, Error fidelity, Invariant encoding, Skill contract fidelity, Test signal.
4. **Primary reduction axes moved (1–7):** LOC (S7 relocation), Types (S5), Branches (S6), Call-site burden (S1 collapses two-step merge+transition), Module edges (S7).
5. **Highest regression risk:** **S1** — wiring `plan.yaml` entry `done` inside `slice merge run` touches the plan/slice boundary in workspace mode and must refuse sensibly when no plan entry exists.

## Reconnaissance

- `tokei`: specify **89 461** lines / 628 files (Markdown 50 286; Rust 105 + 12 262 embedded); specify-cli **71 228** lines / 484 files (Rust 53 809 code).
- `cargo tree --duplicates` (specify-cli): transitive only (`base64 0.21/0.22`, `reqwest 0.12/0.13`, `bitflags 2.x`, `rustix 0.38/1.x` via `wasm-pkg-client`); frozen for this pass.
- `rg -c '^#\[test\]' crates/ src/ tests/ wasi-tools/` (specify-cli): **675** `#[test]` functions.
- `rg --files -g '**/mod.rs'` (specify-cli): **3** (all test shims).
- `wc -l docs/standards/*.md AGENTS.md`: specify **398**, specify-cli **637**, combined **1035**.
- Files > 500 lines under `crates/` and `src/` (specify-cli): `src/commands/plan/create.rs` **1024**; `crates/domain/src/discovery/document.rs` **915**; `crates/domain/src/slice/fusion.rs` **906**; `crates/domain/src/change/plan/core/model.rs` **811**; `crates/domain/src/journal.rs` **619**; `crates/domain/src/spec/provenance.rs` **599**; `crates/domain/src/change/plan/core/validate/tests.rs` **593**; `crates/tool/src/validate.rs` **520**; `crates/domain/src/adapter/cache/io.rs` **508**; `crates/domain/src/adapter/cache.rs` **503**.
- `make checks` (specify): **All checks passed.**
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` (specify-cli): **clean** (44s).
- Production hot-path `unwrap`/`expect`/`panic!`/`unreachable!` in `plan/create.rs`, `plan/lifecycle.rs`, `slice/merge.rs`, `registry/add.rs`: **15** sites (6 in `create.rs` including 1 `panic!` + 1 `unreachable!`).
- `rg -n 'clone\(\)' crates/ src/ --glob '!**/tests/**'` (specify-cli): **318** (context-specific; no blanket-delete finding).
- `todo!` / `unimplemented!` in production Rust: **0**.
- `unsafe` in production Rust: **1** intentional site — `crates/domain/src/change/plan/lock.rs:227` (`libc::kill(pid, 0)` liveness probe, documented).
- Shipped skills: **11** `SKILL.md` files; longest `refine` **118** lines (under 200 cap); `make checks` skill predicates green.

## Structural Findings

### S1 — Wire plan entry `done` into `slice merge run`

- **Evidence**: RFC-25 §Workflow: "`/spec:merge` is the only writer of per-entry `done`" ([`rfcs/archive/rfc-25-workflow.md:126`](rfcs/archive/rfc-25-workflow.md)). `src/commands/slice/merge.rs:20–44` — `run()` calls `slice::commit` and returns; **no** `Plan` load or `Status::Done` write. `plugins/spec/skills/merge/SKILL.md:34,56` claims "`specify slice merge` … stamps the plan entry's per-entry status to `done`". `tests/e2e.rs:130–137` — merge e2e asserts baselines/archive only; **never** reads `plan.yaml`. Per-entry close today requires a separate `specify plan transition <entry> done` (`src/commands/plan/lifecycle.rs:226`, tested at `tests/plan_orchestrate.rs:727`).
- **Action**:
  1. After successful `slice::commit` in `src/commands/slice/merge.rs::run`, when `ctx.layout().plan_path()` exists, `with_state::<Plan, _, _>` find `entries.iter_mut().find(|e| e.name == name)` and call `plan.transition(name, Status::Done)?` (same transition table as lifecycle handler).
  2. Skip silently when no plan on disk (standalone merge fixtures) or entry absent (return diagnostic only if plan exists but slice name missing — mirror `plan-entry-not-found`).
  3. Update `plugins/spec/skills/merge/SKILL.md` step 6 to match reality **or** delete the extra `specify plan transition` step from `plugins/spec/skills/execute/SKILL.md:48` once CLI owns it.
  4. Extend `tests/e2e.rs::merge_two_spec_slice_produces_baselines` with a staged `plan.yaml` row for `my-slice` at `in-progress`; assert `status: done` after merge.
- **Quality delta**: +1 correctness, +1 invariant encoding, +1 skill contract fidelity, +1 API misuse resistance, −1 call-site burden.
- **Net LOC**: `merge.rs` **~45 → ~65**; skills **±5**; test **+12**.
- **Done when**: `rg 'plan transition.*done' plugins/spec/skills/merge` returns **0** (skill no longer documents a second step) **and** `cargo test merge_two_spec_slice_produces_baselines -- --exact` asserts plan entry `done`.
- **Rule?**: no.
- **Counter-argument**: "Keeping `plan transition` separate preserves orthogonality" — loses because RFC-25 and every skill body already treat merge as the sole `done` writer; today's two-step contract is undocumented in tests and breaks `/spec:execute` drain unless the agent remembers a second verb (cargo separates `cargo test` from `cargo publish`, but merge+done is one operator action).
- **Depends on**: none.

### S2 — Plan skill denies shipped `specify plan validate`

- **Evidence**: `plugins/spec/skills/plan/SKILL.md:62` — "there is no `specify plan validate`". `src/commands/plan.rs:34` dispatches `PlanAction::Validate`; `src/commands/plan/lifecycle.rs:15–83` implements it; `plugins/references/cli-output-shapes.md:417` documents fixtures; `tests/plan_orchestrate.rs:1677+` pins health diagnostics. `make checks` does not catch the contradiction (skill body guardrails are not cross-checked against CLI surface).
- **Action**:
  1. Replace line 62 in `plan/SKILL.md` with: validation after writes may call `specify plan validate` for structural/health diagnostics; creation/amend paths still fold schema validation into `plan add` / `plan amend`.
  2. Add one Critical Path bullet (Gate 1 optional): `specify plan validate --format json` before printing the closing hint when multi-slice or workspace plans need doctor output.
- **Quality delta**: +1 correctness, +1 skill contract fidelity.
- **Net LOC**: **70 → 72** (`plan/SKILL.md`).
- **Done when**: `rg 'no `specify plan validate`' plugins/spec/skills/plan/SKILL.md` returns **0** and `rg 'specify plan validate' plugins/spec/skills/plan/SKILL.md` returns **≥1**.
- **Rule?**: no (one-off stale guardrail).
- **Counter-argument**: "Agents might over-call validate" — loses because the skill already tells operators to re-read `plan.yaml`; a documented optional doctor call is strictly more accurate than denying a shipped verb.
- **Depends on**: none.

### S3 — Refine skill omits `slice.fusion.written` journal event

- **Evidence**: `plugins/spec/references/synthesis/fusion.md:7` — "After the atomic rename succeeds, emit the `slice.fusion.written` journal event." `plugins/spec/skills/refine/SKILL.md` step 5 (lines 60–64) writes `fusion.yaml` but **never** mentions the event. `crates/domain/src/journal.rs:185–193` defines `EventKind::SliceFusionWritten { slice_name, generator, requirement_count }`; wire test at `journal.rs:497–505`.
- **Action**:
  1. Append to refine step 5 (after atomic rename): emit one `slice.fusion.written` line with payload `{ slice-name, generator: specify@<version>, requirement-count: <N> }` per the closed shape in `journal.rs`.
  2. Point at `plugins/spec/references/synthesis/fusion.md` for the event (do not duplicate payload prose).
- **Quality delta**: +1 correctness, +1 skill contract fidelity.
- **Net LOC**: **118 → 123** (`refine/SKILL.md`).
- **Done when**: `rg 'slice\.fusion\.written' plugins/spec/skills/refine/SKILL.md` returns **≥1**.
- **Rule?**: no.
- **Counter-argument**: "Journal is best-effort telemetry" — loses because RFC-27 §D4 lists this event in the closed set and fusion.md already makes it mandatory for refine step 5.
- **Depends on**: S4 (shared journal emit guidance).

### S4 — Journal hand-append lacks wire-format contract

- **Evidence**: `plugins/spec/skills/refine/SKILL.md:45,58` — "by appending to `.specify/journal.jsonl`" with no adjacency-tagged shape. `crates/domain/src/journal.rs:58–59` — wire is `{ timestamp, event, payload }` kebab-case; `append_batch` fsyncs (`journal.rs:328–343`). `tests/journal.rs:343–407` + `tests/fixtures/journal/agent-emit-helper.json` golden the agent-driven events. Hand-written lines routinely drift on `event` id spelling (grep history: snake_case leaks caught by `no_snake_case_fields_or_values_leak_to_wire`).
- **Action**:
  1. In refine step 3 sub-step 4 and step 4 tag loop, replace "append to journal.jsonl" with: "append one NDJSON line per event using the adjacency-tagged `{ timestamp, event, payload }` shape; field names are kebab-case per [`plugins/spec/references/synthesis/tags.md`](plugins/spec/references/synthesis/tags.md) §Journal-event hand-off and the worked line in [`plugins/spec/skills/plan/fixtures/divergence-journal/journal.jsonl`](plugins/spec/skills/plan/fixtures/divergence-journal/journal.jsonl)."
  2. In `plugins/spec/references/synthesis/tags.md` §Journal-event hand-off, add one sentence: each line must be one JSON object, newline-terminated, no snake_case keys.
- **Quality delta**: +1 error fidelity, +1 skill contract fidelity, +1 correctness.
- **Net LOC**: **+8** across refine skill + tags reference.
- **Done when**: `rg 'divergence-journal/journal\.jsonl' plugins/spec/skills/refine/SKILL.md` returns **≥1**.
- **Rule?**: no.
- **Counter-argument**: "Agents can infer JSON shape" — loses because integration tests already golden the shape and production `append_batch` rejects malformed consumers silently until tail-parse fails.
- **Depends on**: none.

### S5 — Plan validate workspace check bypasses `ProjectConfig`

- **Evidence**: `src/commands/plan/lifecycle.rs:42–46` — `serde_saphyr::from_str::<serde_json::Value>(&content)` on workspace slot `project.yaml`, then `.get("adapter")`. Typed loader exists at `crates/domain/src/config.rs:14–79` (`ProjectConfig::load` enforces semver floor, hub/adapter invariant). Untyped parse accepts `{ adapter: 42 }` (no warning) and skips hub slots correctly only by accident. Same smell in `crates/domain/src/validate/registry/composition.rs:6` (YAML validity only — acceptable there).
- **Action**:
  1. Replace lines 42–46 with `ProjectConfig::load(slot_project_dir)` (parent of `.specify/`); on `Ok(cfg)`, compare `cfg.adapter.as_deref()` to `rp.adapter`; on `Err`, push a `PlanDoctorDiagnostic` with code `workspace-slot-config-unreadable` instead of silent skip.
  2. Add one integration assertion in `tests/plan_orchestrate.rs` workspace adapter-mismatch block.
- **Quality delta**: +1 correctness, +1 invariant encoding, +1 error fidelity.
- **Net LOC**: `lifecycle.rs` **418 → 425**.
- **Done when**: `rg 'from_str::<serde_json::Value>' src/commands/plan/lifecycle.rs` returns **0**.
- **Rule?**: no.
- **Counter-argument**: "Value parse is shorter" — loses because `ProjectConfig` already centralises hub/adapter rules; ripgrep uses typed config at boundaries, not `serde_json::Value` for YAML files.
- **Depends on**: none.

### S6 — Replace `authority_override_event_key` panic with typed extract

- **Evidence**: `src/commands/plan/create.rs:514` sorts journal events via `authority_override_event_key`; `create.rs:542` — `panic!("…non-PlanAmendAuthorityOverride…")` on misuse. `emit_override_events` (`create.rs:411–463`) only constructs `PlanAmendAuthorityOverride` variants — panic is documentation, not recovery. Production CLI must not panic on internal sorting (cargo uses `debug_assert!` / `unreachable!` only for truly dead match arms fed by literals).
- **Action**:
  1. Change `authority_override_event_key` to return `Result<(String, Option<String>, AuthorityOverrideAction), Error>` with `Error::Diag { code: "internal-journal-sort", … }`, **or** make it take `&EventKind::PlanAmendAuthorityOverride` by building a typed intermediate vec in `emit_override_events` (preferred — deletes the helper entirely).
  2. Delete `create.rs:1022` `unreachable!` arm by typing `EntryBody.action` as `enum Action { Create, Amend }` (two variants — no third arm).
- **Quality delta**: +1 error fidelity, −1 branches (removes panic path), −1 type (drops helper if typed vec built at source).
- **Net LOC**: `create.rs` **1024 → ~1010**.
- **Done when**: `rg 'panic!|unreachable!' src/commands/plan/create.rs` returns **0**.
- **Rule?**: no.
- **Counter-argument**: "Panic catches developer mistakes faster" — loses in a CLI binary where the sort input is closed-construction; jj returns `Bug`/`InternalError` for impossible states, not process abort.
- **Depends on**: none.

### S7 — Move `model.rs` inline tests to sibling `tests.rs`

- **Evidence**: `crates/domain/src/change/plan/core/model.rs` **811** lines; `#[cfg(test)] mod tests` starts at **404** (~407 lines). Sibling pattern already used: `validate.rs:350–351` → `validate/tests.rs` (587 lines). Production `model.rs` body is **403** lines — under cap after split.
- **Action**:
  1. Cut `model.rs:403–811` into `crates/domain/src/change/plan/core/model/tests.rs`.
  2. Replace with `#[cfg(test)] mod tests;` + `mod tests` file header `use super::*;`.
- **Quality delta**: −407 LOC in hot file, +1 module edge clarity (matches house pattern).
- **Net LOC**: **811 + 0** (relocate only).
- **Done when**: `wc -l crates/domain/src/change/plan/core/model.rs` reports **≤450** and `test -f crates/domain/src/change/plan/core/model/tests.rs`.
- **Rule?**: no.
- **Counter-argument**: "One file keeps round-trip tests near types" — loses because four sibling plan modules already externalised tests and `model.rs` is the only outlier >500 lines.
- **Depends on**: none.

### S8 — Hub init doc still names retired `specify change draft`

- **Evidence**: `docs/reference/cli/init.md:17` — "`change.md` and `plan.yaml` are operator artifacts minted later by `specify change draft`". 2.0 surface is `/spec:plan` → `specify plan create` (see `plugins/spec/skills/plan/SKILL.md:16`). `scripts/checks/prose.ts:42–47` forbids `specify change plan` but **not** `specify change draft`. `make checks` passes with the stale line.
- **Action**:
  1. Replace "`specify change draft`" with "`/spec:plan` (via `specify plan create`)" in `docs/reference/cli/init.md:17`.
  2. Regenerate mdBook if `docs/book/` is committed output (`make docs` or project recipe).
- **Quality delta**: +1 correctness, +1 skill contract fidelity (operator docs match shipped skills).
- **Net LOC**: **±0**.
- **Done when**: `rg 'specify change draft' docs/reference/cli/init.md` returns **0**.
- **Rule?**: yes — add `[/\bspecify change draft\b/, "use `/spec:plan` or `specify plan create`"]` to `scripts/checks/prose.ts` FORBIDDEN (≤1 line); `make checks` enforces.
- **Counter-argument**: "decision-log mentions draft for history" — loses because `ALLOWED_PREFIXES` already exempts `docs/explanation/decision-log.md`.
- **Depends on**: none.

## One-touch tidies

1. **Use `ok_or_else` in `entry_mut`** — `src/commands/plan/create.rs:367–368` `expect("slice presence pre-checked above")` → return `unknown_slice_err` already defined at `:578`. Δ: −1 expect, +2 lines. Done when: `rg 'slice presence pre-checked' src/` → 0.

2. **Drop `registry/add` post-push `expect`** — `src/commands/registry/add.rs:65–68` — bind `let added = candidate.clone()` before `push` instead of `last().expect`. Δ: −1 expect. Pattern: std `Vec` push then use moved value (same as `cargo metadata` handlers).

3. **Archive metadata pick without `expect`** — `src/commands/slice/outcome.rs:88–91` — `match candidates.into_iter().max_by_key(...)` with explicit `None` arm returning `slice-not-found` (unreachable today but removes prod `expect`). Δ: +3/−1 lines.

4. **Align journal golden requirement ids** — `tests/journal.rs:373–387` uses `"R-01"`…`"R-03"` while `plugins/spec/references/synthesis/tags.md` and provenance parser require `REQ-NNN`. Update golden + fixture to `REQ-001` etc. Δ: test-only, +1 test signal. Done when: `rg '"R-0' tests/fixtures/journal/` → 0.

5. **Fix execute stop-condition drop hint** — `plugins/spec/skills/execute/references/stop-conditions.md:40` suggests `specify plan transition <slice> done` after drop; dropping abandons the slice — should be `specify plan amend` / re-`plan next`, not `done`. Δ: 1 sentence. +1 skill contract fidelity.

6. **Delete stale comment on `SliceFusionWritten`** — `crates/domain/src/journal.rs:183–184` says "CLI-driven once the atomic writer lands in Change 2.6" but agent refine is the writer today. Replace with "Agent-driven from `/spec:refine` step 5." Δ: −0 LOC, comment only (actively wrong today).

7. **Tighten `binding_from_arg` legacy comment** — `src/commands/plan/create.rs:63–64` "legacy behaviour" when `discovery` is `None` — rename to "discovery-absent passthrough" (pre-1.0, not legacy compat). Δ: comment only.

8. **Init skill description shortens past 512?** — `plugins/spec/skills/init/SKILL.md` description is **393** chars (under cap); no action. Listed to document recon — **drop**.

## Dropped findings

- **Collapse `RequirementTag` into `RequirementStatus`** — semantic distinction between absent tag vs `Agreed` still used by validator (`provenance.rs:443`); net axes neutral.
- **Add `specify journal append` CLI verb** — RFC-25 explicitly cut it; S4 fixes the skill gap without new surface.
- **Dedupe `base64` / `reqwest` in lockfile** — dependency freeze; transitive from `wasm-pkg-client` only.
- **Split `plan/create.rs` (1024 lines)** — clippy already `expect`s `too_many_lines` with RFC-27 justification; split would add module edge without deleting duplication.
- **Delete omnia/vectis specialist skills from Cursor cache** — not in this repo's `plugins/` tree (references-only per 2.0); out of scope.
- **Promote hundreds of `Error::Diag` codes to enum variants** — net +LOC, no axis win.

## Post-mortem

- **S1** — ΔLOC +42 vs predicted ~+37 (within ±10%); done-when flipped cleanly (`rg` 0, e2e asserts `done`, `cargo make check` pass); no regressions; stop-conditions.md drop hint still stale (T5 scope).
- **S2** — ΔLOC +1 vs predicted +2; done-when rg checks pass; `cargo make check` pass; `make checks` failed on pre-existing broken links in REVIEW.md (S3/S4 paths cited but not yet landed), not on plan skill edit.
- **S3** — ΔLOC +0 net (extended existing line, file stays 118 lines) vs predicted +5; `rg 'slice\.fusion\.written'` → 1; `cargo make check` pass; no regressions.
- **S4** — ΔLOC +6 vs predicted +8; done-when `rg divergence-journal` → 2; `make checks` pass after REVIEW.md link fix; `cargo make check` pass; no regressions.
- **S5** — lifecycle.rs +15 vs predicted +7 (test +52 extra); done-when `rg from_str::<serde_json::Value>` → 0; `cargo make check` pass; no regressions.
- **S6** — create.rs −8 vs predicted −14; done-when `rg 'panic!|unreachable!'` → 0; `cargo make check` pass; no regressions.
- **S7** — relocate-only 811→404+406; done-when line count ≤450 and tests.rs exists; `cargo make check` pass; no regressions.
- **S8** — ΔLOC −1 net (init.md + prose.ts + cli-output-shapes heading); done-when rg + `make checks` pass; mdBook skipped (gitignored); no regressions.
- **T1** — ΔLOC ~+12 in create.rs helpers vs predicted +2; done-when `rg 'slice presence pre-checked'` → 0; `cargo make check` pass; no regressions.
- **T2** — ΔLOC +1/−5; done-when clean; no regressions.
- **T3** — ΔLOC +4/−9; done-when clean; no regressions.
- **T4** — ΔLOC +6/−6 test-only; done-when `rg '"R-0'` → 0; no regressions.
- **T5** — 1 sentence fix; closes S1 follow-up on stop-conditions drop hint; `make checks` pass; no regressions.
- **T6–T7** — comment-only; no LOC delta; `cargo make check` pass; no regressions.
- **Final validation** — `make checks` (specify) pass; `cargo make check` pass per item; `cargo make ci` fails on pre-existing `RUSTSEC-2026-0149` (`wasmtime-wasi` 44.0.1 advisory in `cargo deny`) — unrelated to review remediation.
