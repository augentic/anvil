# RFC-46 Implementation Plan

> **Authority:** [rfc-46-asset-materialization.md](./rfc-46-asset-materialization.md) (read in full before every step).  
> **Repos:** `augentic/specify` (plugins, adapters, evals, framework checks) and `augentic/specify-cli` (CLI, workflow, `wasi-tools/vectis`). Co-develop with a gitignored `Specify.local.toml` overlay (`cli = { path = "../specify-cli" }`) and `make lint` / `cargo make check` respectively.  
> **Ordering contract:** Start step *N* only after step *N−1* is ✅. Each step is sized for one agent conversation.  
> **Test boundary:** RFC-46 work must obey [Hard rule: no host runtime ↔ WASM tests](#hard-rule-no-host-runtime--wasm-tests-rfc-46-scope) — no new cross-boundary tests in any step.

---

## How to use this document

### Progress (update every session)

| Field | Value |
|-------|-------|
| **Active step** | `R46-S12` |
| **Last completed** | `R46-S11` |
| **Last updated** | 2026-06-15 |
| **Blocked on** | — |

**Step status legend:** `⬜ pending` · `🔄 in progress` · `✅ done` · `⏸ blocked` · `↩ superseded`

When a step finishes: mark it ✅, set **Last completed**, advance **Active step**, append any discoveries to [Discovery log](#discovery-log), and fold material changes back into the step list (do not leave silent drift).

### Architectural correction (R46-S01/S02)

Steps **R46-S01** and **R46-S02** landed on `specify-cli` `rfc-46` with the wrong integration model: `vectis_missing_platforms` in `specify-workflow` dispatches the vectis **WASM** component via `WasiRunner`. That couples plan-time `propose` to `cargo make vectis-wasm`, breaks reusable CI (bootstrap tests `assert!` the artifact; `rust-cache` does not share `target/vectis-wasi-tools/` across jobs), and duplicates logic that is already pure Rust filesystem code in `wasi-tools/vectis/src/verify.rs`.

**Correct model** (see [RFC-46 §Phase 0 detect architecture](./rfc-46-asset-materialization.md#detect-architecture-normative)): extract `specify-vectis-shell-detect`; host callers link in-process; `vectis verify --mode detect` is a thin JSON wrapper. **Do not** spawn WASM from `propose` or `specify-workflow`.

**What to keep from the superseded steps:** `--reconcile-platforms` removal and default-on `propose --from` reconciliation (R46-S02 behaviour intent). **What to revert/replace:** WASM dispatch in `crates/workflow/src/platform/detect.rs`; propose/detect integration tests that require `tools.yaml` + `target/vectis-wasi-tools/release/vectis.wasm`; any CI workflow fork added only for propose WASM.

**Recovery sequence:** [R46-S02a](#r46-s02a-shell-detect-shared-library) → [R46-S02b](#r46-s02b-host-in-process-detect) → [R46-S03](#r46-s03-remove-workflow-shell-heuristics) → continue Phase 0.

### Hard rule: no host runtime ↔ WASM tests (RFC-46 scope)

For the **remainder of RFC-46** (all steps from R46-S02a through R46-S30), **do not add or extend tests** that cross the host `specify` binary ↔ WASM component boundary — including mechanisms that already exist elsewhere in the repo.

**Forbidden for new RFC-46 test coverage** (non-exhaustive):

- `specify tool run …` / `tool::run_captured` from `tests/*.rs` or host crate integration tests.
- Declaring `tools.yaml` + `SPECIFY_TOOLS_CACHE` in a host test to load `vectis.wasm`, `contract-*.wasm`, or any other guest component.
- `assert!` / hard-require on `target/vectis-wasi-tools/release/vectis.wasm` or any WASM path that is not committed in git.
- Host tests that drive `specify lint framework` / embedded framework checker WASM for RFC-46 behaviour (framework lint already does this for unrelated checks — **do not add more**).
- Host tests that invoke `specify lint project` `kind: tool` hints that dispatch a real adapter WASM for RFC-46 behaviour.
- CI changes whose sole purpose is building WASM so RFC-46 host tests can run.

**Required pattern:**

| What | Where tests live | How |
|------|------------------|-----|
| Vectis subcommand behaviour (`verify`, `validate`, `materialize`, `scaffold`, …) | `wasi-tools/vectis/` | `cargo test -p specify-vectis` (and `wasi-tools/` CI job) — direct lib/CLI tests inside the carve-out |
| Logic the host must call (`shell-detect`, launcher probe, bootstrap context, materialize scope, …) | Host shared crate (`specify-vectis-shell-detect`, future siblings) | `cargo test -p <crate>` with tempdir fixtures — **no** Wasmtime |
| Workflow / CLI verbs (`propose`, `plan validate`, `slice build prepare` assembly) | `tests/workflow/*.rs`, `crates/workflow/…/tests.rs` | Exercise host paths only; stub or filesystem-fixture side effects; **never** dispatch vectis WASM to assert RFC-46 outcomes |

**Production code** may still call WASM where the RFC requires it (e.g. `slice build prepare` invoking `materialize` via `run_captured` in a later step). **Tests added in RFC-46 must not** — validate guest behaviour in `wasi-tools/vectis` tests; validate host orchestration with mocks, pure helpers, or on-disk fixtures without loading a component.

**Pre-existing host WASM tests** (`tests/tool/run.rs`, `tests/tool/contract.rs`, `tests/lint/framework.rs`, skip-when-absent `catalog_infer`, etc.) are **unchanged by this RFC** and will be unwound in a **separate** effort — do not copy their patterns into new RFC-46 steps.

**Step PR checklist (every `specify-cli` step):** confirm no new files under `tests/` or host `#[test]` that invoke `specify tool run` or `WasiRunner` for RFC-46 coverage; `rg 'tool run|run_captured|vectis_wasm|SPECIFY_TOOLS_CACHE' tests/` touched paths should show no new RFC-46 assertions.

### Execution tracker

| Step | Title | Status | PR / notes |
|------|-------|--------|------------|
| [R46-S00](#r46-s00-baseline-snapshot) | Baseline snapshot | ✅ | Baseline at [`rfcs/rfc-46-baseline/`](./rfc-46-baseline/); all assurance green on `rfc-46` @ specify-cli `1711ebc`, specify `9d3886e` |
| [R46-S01](#r46-s01-vectis-detect-host-helper) | Vectis detect host helper | ↩ | Superseded — WASM dispatch from workflow; see [correction](#architectural-correction-r46-s01s02) |
| [R46-S02](#r46-s02-propose-default-on-reconciliation) | Propose default-on reconciliation | ↩ | Partial — flag removed and propose wired, but via wrong WASM path; see [correction](#architectural-correction-r46-s01s02) |
| [R46-S02a](#r46-s02a-shell-detect-shared-library) | Shell-detect shared library | ✅ | `specify-vectis-shell-detect`; vectis `verify` calls library |
| [R46-S02b](#r46-s02b-host-in-process-detect) | Host in-process detect | ✅ | `vectis_missing_platforms` → `specify-vectis-shell-detect`; propose tests WASM-free |
| [R46-S03](#r46-s03-remove-workflow-shell-heuristics) | Remove workflow shell heuristics | ✅ | Deleted workflow `detect_missing_platforms`; shell probe is `specify-vectis-shell-detect` only |
| [R46-S04](#r46-s04-phase-0-documentation-alignment) | Phase 0 documentation alignment | ✅ | plan skill, plan-propose ref, eval runs, DECISIONS.md, AGENTS.md |
| [R46-S05](#r46-s05-phase-0-assurance-gate) | Phase 0 assurance gate | ✅ | `cargo make ci` + `make lint` green; §10 rows **#2–#4** remediated |
| [R46-S06](#r46-s06-assets-schema-extensions) | `assets.yaml` schema extensions | ✅ | specify-cli — schema + cross-checks in `validate/engine/assets.rs` |
| [R46-S07](#r46-s07-shell-resident-launcher-probe) | Shell-resident launcher probe | ✅ | specify-cli — `shell_resident_app_icon` in `specify-vectis-shell-detect` |
| [R46-S08](#r46-s08-bootstrap-context-helper) | Bootstrap context helper | ✅ | specify-cli — `BootstrapContext` in `platform/bootstrap.rs` |
| [R46-S09](#r46-s09-plan-validate-app-icon-gate) | Plan validate `app-icon` gate | ✅ | specify-cli — `plan-bootstrap-app-icon-missing` in plan doctor |
| [R46-S10](#r46-s10-vectis-validate-app-icon-checks) | Vectis validate `app-icon` checks | ✅ | specify-cli — export layout §4.2/§4.3, materialization-missing, project.yaml platforms |
| [R46-S11](#r46-s11-scaffold-app-icon-skeletons) | Scaffold app-icon skeletons | ✅ | specify-cli + `adapters/targets/vectis/examples/assets.yaml` |
| [R46-S12](#r46-s12-phase-1-documentation-and-inference-policy) | Phase 1 docs & inference policy | ⬜ | specify |
| [R46-S13](#r46-s13-review-rule-render-by-kind) | Review rule: render-by-`kind` | ⬜ | specify |
| [R46-S14](#r46-s14-phase-1-assurance-gate) | Phase 1 assurance gate | ⬜ | |
| [R46-S15](#r46-s15-materialize-subcommand-skeleton) | Materialize subcommand skeleton | ⬜ | specify-cli |
| [R46-S16](#r46-s16-materialize-path-conventions) | Export path conventions | ⬜ | specify-cli |
| [R46-S17](#r46-s17-materialize-icons) | Materialize icons | ⬜ | specify-cli |
| [R46-S18](#r46-s18-materialize-illustrations) | Materialize illustrations | ⬜ | specify-cli |
| [R46-S19](#r46-s19-materialize-app-icon-ios) | Materialize app-icon (iOS) | ⬜ | specify-cli |
| [R46-S20](#r46-s20-materialize-app-icon-android) | Materialize app-icon (Android) | ⬜ | specify-cli |
| [R46-S21](#r46-s21-pin-semantics-and-yaml-auto-write) | Pin semantics & YAML auto-write | ⬜ | specify-cli |
| [R46-S22](#r46-s22-in-scope-asset-resolution) | In-scope asset resolution | ⬜ | specify-cli |
| [R46-S23](#r46-s23-slice-build-prepare-hook) | Slice build prepare hook | ⬜ | specify-cli |
| [R46-S24](#r46-s24-validate-export-presence) | Validate export presence | ⬜ | specify-cli |
| [R46-S25](#r46-s25-acceptance-fixtures-committed-exports) | Acceptance fixtures | ⬜ | both repos |
| [R46-S26](#r46-s26-phase-2-assurance-gate) | Phase 2 assurance gate | ⬜ | |
| [R46-S27](#r46-s27-writer-contract-docs) | Writer contract docs | ⬜ | specify |
| [R46-S28](#r46-s28-vectis-verify-catalog-completeness) | Verify catalog completeness | ⬜ | specify-cli |
| [R46-S29](#r46-s29-rfc-closure-and-stale-reference-sweep) | RFC closure & reference sweep | ⬜ | both repos |
| [R46-S30](#r46-s30-final-assurance-gate) | Final assurance gate | ⬜ | |

### Discovery log

Append-only. When implementation diverges from the RFC or this plan, record the finding and edit the affected step(s) in the same PR.

| Date | Step | Discovery | Plan change |
|------|------|-----------|-------------|
| 2026-06-12 | R46-S01 | WASI dispatch must **omit** the project path argument and rely on host-injected `PROJECT_DIR`; passing a host absolute path breaks preopen reads inside the guest. | Added note under R46-S01 implementation notes; R46-S02 should wire the helper (not `specify tool run … <path>`) from `propose.rs`. |
| 2026-06-12 | R46-S01 | CI failed on `rust_quality`, `rustdoc`, `fmt`, and `clippy` (clock injection, test fn length, doc private links, formatting, `significant_drop_tightening`). | Added [Specify-cli step assurance](#specify-cli-step-assurance); every `specify-cli` step references it. |
| 2026-06-12 | R46-S02 | Propose bootstrap integration tests must declare a `vectis`-named adapter with `tools.yaml` (the in-repo `vectis-stub` fixture name skips detect); `cargo make vectis-wasm` prerequisite unchanged. | Note under R46-S02 assurance; no new steps. |
| 2026-06-12 | R46-S02 | Reusable CI `cargo nextest` does not run `cargo make vectis-wasm`; bootstrap propose tests panicked on missing `target/vectis-wasi-tools/release/vectis.wasm`. | Added `vectis-wasm` prep job to `specify-cli` `.github/workflows/ci.yaml` (`needs` before reusable `ci`); extended [Specify-cli step assurance](#specify-cli-step-assurance) with vectis-dispatch integration-test rules. |
| 2026-06-12 | R46-S02 | Prep job + `needs` is insufficient: `rust-cache` does not share `target/vectis-wasi-tools/` across reusable-workflow job boundaries (prep built WASM; Test job still panicked). | Attempted CI workflow fork; **reverted** (`revert CI workflow fork`). Do not reintroduce CI changes solely for propose WASM — fix architecture instead. |
| 2026-06-12 | R46-S02 | Plan-time detect does not need WASM — heuristics are pure Rust and identical in `verify.rs` and legacy `platforms.rs`. Spawning vectis from `propose` is an anti-pattern (CI, layering, test isolation). | Superseded R46-S01/S02; added [R46-S02a](#r46-s02a-shell-detect-shared-library) + [R46-S02b](#r46-s02b-host-in-process-detect); updated RFC §Phase 0 [detect architecture](./rfc-46-asset-materialization.md#detect-architecture-normative). |
| 2026-06-12 | R46-S02 | Broader repo already has host↔WASM tests (framework lint, `tool run` fixtures, contract dist, optional vectis smoke) — unwinding deferred outside RFC-46. | Added [Hard rule: no host runtime ↔ WASM tests](#hard-rule-no-host-runtime--wasm-tests-rfc-46-scope); RFC-46 steps must not add cross-boundary tests even via those mechanisms. |
| 2026-06-12 | R46-S02b | Propose reconcile integration tests live in `tests/plan.rs` (`mod propose` → `tests/workflow/propose.rs`); there is no `--test propose` binary. | Corrected assurance commands to `cargo nextest run --test plan reconcile`. |
| 2026-06-12 | R46-S04 | `CORE-057` `cli-contract` `invocations` already flags retired `--reconcile-platforms` once docs are updated; no separate regex rule added. | Optional CORE rule in R46-S04 skipped; `wasi-tools/vectis/DECISIONS.md` `detect_missing_platforms` citation deferred to R46-S12 per plan. |
| 2026-06-12 | R46-S05 | Phase 0 assurance gate green on `rfc-46` @ both repos; `plan-single-project` scenario/fixture paths unchanged (Omnia target — pass record already omits `--reconcile-platforms`). | Marked §10 rows **#2–#4** remediated in RFC-46; unblocks Phase 1. |
| 2026-06-12 | R46-S06 | No mirrored `assets.schema.json` under `schemas/` or `specify` — vectis `embedded/assets.schema.json` is the sole source; `crates/schema` has no assets constant. Raster dimension/alpha decode checks deferred to R46-S10 per plan. | No new steps; R46-S07 unchanged. |
| 2026-06-15 | R46-S09 | Plan gate checks path A (source file presence + kind/ext) and path B (pin resolves to file/dir) only; export layout §4.2/§4.3 and raster dimension decode stay in R46-S10 vectis validate. `project_dir` was already forwarded to `plan_doctor`. | No new steps; R46-S10 unchanged. |
| 2026-06-15 | R46-S10 | Split `assets.rs` into `assets/{mod,app_icon,exports,platforms}.rs`; `load_shell_platforms` filters `ios`/`android` only (not `core` from shell-detect). Missing `project.yaml` falls back silently to `["ios","android"]` so lone `assets.yaml` validate keeps working. Raster PNG dimension/alpha decode uses inline IHDR parsing (no `image` dep). | No new steps. |
| 2026-06-15 | R46-S11 | No separate Vectis init design-system template exists — added `adapters/targets/vectis/examples/assets.yaml` (commented `app-icon` field, no placeholder asset file). Android scaffold stubs omit legacy `mipmap-*/ic_launcher.png` (materialize fills in R46-S20; `shell_resident_app_icon` satisfied by `mipmap-anydpi-v26` stubs). | No new steps. |

### Specify-cli step assurance

Every step with **Repo:** `specify-cli` must pass the checks below **in addition to** any step-specific tests, before marking the step ✅. Run from the `specify-cli` repo root. Phase assurance gates (R46-S05, S14, S26, S30) still require full `cargo make ci` as the merge bar; these are the per-step minimum that mirrors CI on push.

**Every step must also satisfy** [Hard rule: no host runtime ↔ WASM tests](#hard-rule-no-host-runtime--wasm-tests-rfc-46-scope).

**Required (CI `Format`, `Clippy`, `Test`, and `Docs` jobs):**

```bash
cargo +nightly fmt --all -- --check
RUSTFLAGS=-Dwarnings cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
RUSTFLAGS=-Dwarnings cargo test --test rust_quality
RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps
```

**When the step changes `wasi-tools/`** (vectis or other carve-out crates), also from `wasi-tools/`:

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

**Host test posture for RFC-46 (see hard rule above):**

| Layer | Example commands | WASM? |
|-------|------------------|-------|
| Shared host libs | `cargo test -p specify-vectis-shell-detect` | No |
| Vectis carve-out | `cd wasi-tools && cargo test -p specify-vectis` | Yes — **inside carve-out only** |
| Host workflow / CLI | `cargo nextest run --test plan reconcile`, `cargo test -p specify-workflow`, `tests/workflow/validate.rs` | **No** — no `tool run`, no `tools.yaml`, no `vectis-wasm` |

**Pre-push for host workflow changes:** `cargo nextest run --test plan reconcile` (or the relevant filter) **without** `cargo make vectis-wasm`.

**Do not** add new host integration tests that dispatch vectis/contract/framework WASM for RFC-46 — including skip-when-absent smoke (that pattern is reserved for pre-existing tests pending repo-wide unwind).

**Discipline enforced by `rust_quality` (fix before push, not in CI triage):**

- **`specify-workflow` clock reads:** library code must not call `Timestamp::now()` — accept `now: Timestamp` from runtime handlers (`docs/standards/architecture.md` §Time injection).
- **Test fn names:** ≤ 40 characters (`docs/standards/testing.md`); narrative belongs in the test body.
- **Lint suppressions:** `#[expect(..., reason = "...")]` only — no bare `#[allow]`.
- **Rustdoc:** public docs must not link to private items (use prose, e.g. `` `vectis` ``, not `` [`VECTIS_ADAPTER`] ``).

---

## Phase 0 — Platform bootstrap inference (prerequisite)

RFC §Implementation phases · Phase 0. **Phase 1 must not merge until R46-S05 is ✅.**

### R46-S00 — Baseline snapshot

**Goal:** Record pre-change behaviour so regressions are obvious.

**Work:**
1. On `specify-cli` `main` (or the branch you are implementing from), run and save outputs:
   - `cargo make check`
   - `cargo test -p specify-workflow propose` (or the `tests/workflow/propose.rs` subset)
   - `cargo test -p specify-vectis` inside `wasi-tools/`
2. On `specify` `main`, run `make lint`.
3. Note current behaviour: `specify plan propose --from` **without** `--reconcile-platforms` does **not** insert bootstrap slices; with the flag it uses workflow `detect_missing_platforms` in `crates/workflow/src/change/plan/core/propose/platforms.rs`, not `vectis verify --mode detect`.

**Assurance:** No code changes. Document baseline in the step PR description or a one-line note under **PR / notes** in the tracker.

**Handoff:** Confirms vectis detect JSON shape (`missing: ["ios", …]`) matches `wasi-tools/vectis/src/verify.rs` `render_detect`.

---

### R46-S01 — Vectis detect host helper

> **↩ Superseded** by [R46-S02a](#r46-s02a-shell-detect-shared-library) + [R46-S02b](#r46-s02b-host-in-process-detect). Do not implement WASM dispatch from workflow. Kept for history.

**Goal:** One reusable host-side function that returns declared-but-absent platforms for a Vectis-bound project directory.

**Repo:** `specify-cli`

**Work:**
1. Add a small module (suggested: `crates/workflow/src/platform/detect.rs` or `src/runtime/platform_detect.rs` — follow existing layout; prefer workflow if plan validate will reuse it without pulling `Ctx`).
2. API sketch:
   ```rust
   pub fn vectis_missing_platforms(
       project_dir: &Path,
       declared: &[Platform],
       now: Timestamp,
   ) -> Result<Vec<Platform>, Error>
   ```
   Runtime dispatchers pass `Timestamp::now()`; tests pin a fixed stamp (see `docs/standards/architecture.md` §Time injection).
3. Implementation: call `tool::run_captured` (see `src/runtime/commands/tool/run.rs`) with `vectis` and args `["verify", "--mode", "detect", "<project-dir>"]`. Parse stdout JSON; map `missing[]` strings → `Platform` via `FromStr`; ignore `web`/`desktop` (vectis already treats them as present).
4. **Vectis-bound gate:** only invoke when the project's target adapter is `vectis` (resolve via `TargetAdapter::resolve` + `project.yaml` / plan topology — mirror `propose.rs` `detect_missing_for_topology`). Non-Vectis → return empty `missing` (RFC: Omnia unaffected).
5. Unit tests with a tempdir fixture: greenfield tree → `missing` contains `core`, `ios`, `android`; tree with `shared/src/app.rs` + `iOS/*.swift` → `ios` absent from `missing`, etc. Reuse layout from `wasi-tools/vectis/src/verify/tests.rs`.

**Implementation notes (not in RFC):**
- `run_captured` already powers `specify catalog infer`; copy that error-mapping posture.
- Wasm must be built for tests: `cargo make contract-wasm` is for contract; vectis uses `cargo make framework-wasm` / vectis dist — follow `AGENTS.md` wasi-tools CI recipes. Document the prerequisite in the PR if CI already builds it.
- Parse failures → `tool-runtime` or a new `vectis-detect-parse` validation error; do not silently fall back to workflow heuristics.
- **WASI path argument:** dispatch `vectis verify --mode detect` with **no** trailing project path — the host sets `PROJECT_DIR` on the guest environment and preopens `$PROJECT_DIR`. A host absolute path argument is not readable inside the sandbox (discovered R46-S01).

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance) (including vectis-dispatch integration-test rules).
- `cargo test -p specify-workflow platform::detect` passes.
- Manual smoke: from a Vectis fixture project root, `specify tool run vectis -- verify --mode detect` (no path arg — uses `PROJECT_DIR`) returns expected `missing`.

**Handoff:** Helper is callable from `propose.rs` and later from plan validate (R46-S09).

---

### R46-S02 — Propose default-on reconciliation

> **↩ Superseded** for detect wiring — flag removal may already be on branch; replace WASM path per [R46-S02b](#r46-s02b-host-in-process-detect).

**Goal:** `specify plan propose --from` always reconciles platforms when `project.yaml.platforms` is non-empty; remove `--reconcile-platforms`.

**Repo:** `specify-cli`

**Prerequisites:** R46-S01 ✅

**Work:**
1. `src/runtime/commands/plan/cli.rs` — delete `--reconcile-platforms` flag and `conflicts_with` wiring.
2. `src/runtime/commands/plan/propose.rs` — replace `detect_missing_platforms` call with `vectis_missing_platforms`; run reconciliation when topology project has non-empty `platforms` (not flag-gated).
3. Map `Vec<Platform>` → `ProjectMissingPlatforms` unchanged; `Plan::reconcile_platforms` stays as-is (DAG insertion only).
4. Update `src/runtime/commands/plan/propose.rs` module docs and any `build_contract()` / CLI help text that mentions the flag.
5. Adjust `tests/workflow/propose.rs` bootstrap tests to call `propose --from` **without** the flag; keep greenfield / incremental bootstrap assertions.

**Implementation notes:**
- Hub/workspace mode: preserve `detect_missing_for_topology` project-dir resolution (`ctx.project_dir` vs `.specify/workspace/<project>/`).
- Journal event `plan.reconcile.completed` unchanged.
- If vectis tool is undeclared for a Vectis project, fail loudly at propose time (do not revert to workflow heuristics).

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance) (including vectis-dispatch integration-test rules).
- `cargo test propose` (workflow propose tests) green; bootstrap fixtures patch `vectis-stub` → `vectis` and declare the WASI tool.
- Greenfield fixture: `propose --from` inserts `app-foundation`.
- Partial shell fixture: inserts `bootstrap-ios` / `bootstrap-android` only.

**Handoff:** Flag removed from CLI; docs still stale until R46-S04.

---

### R46-S02a — Shell-detect shared library

**Goal:** Single source of truth for Crux shell presence heuristics, shared by host CLI and vectis WASI.

**Repo:** `specify-cli`

**Prerequisites:** None (first step of recovery sequence).

**Work:**
1. Add host workspace crate `crates/vectis-shell-detect` (`specify-vectis-shell-detect`) with **stdlib only** (no `specify-workflow` / `specify-tool` deps).
2. API (string-based; callers map to `Platform`):
   ```rust
   pub const SUPPORTED_SHELL_PLATFORMS: &[&str];
   pub fn shell_present(project_dir: &Path, platform: &str) -> bool;
   pub fn missing_shell_platforms(project_dir: &Path, declared: &[&str]) -> Vec<String>;
   ```
   Heuristics (RFC normative): `core` → `shared/src/app.rs`; `ios` → `iOS/**/*.swift`; `android` → `Android/**/*.kt`; `web`/`desktop`/unknown → present.
3. Register crate in root `Cargo.toml` `members` + `[workspace.dependencies]`.
4. Refactor `wasi-tools/vectis/src/verify.rs` to call the library (`check_platform` → `shell_present`; delete inlined duplicates). Add path dependency in `wasi-tools/vectis/Cargo.toml`.
5. Unit tests in `crates/vectis-shell-detect` (tempdir fixtures — copy cases from `wasi-tools/vectis/src/verify/tests.rs`). Existing vectis verify tests must stay green.

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance) Tier A.
- `cargo test -p specify-vectis-shell-detect`
- From `wasi-tools/`: `cargo test -p specify-vectis verify`

**Handoff:** Library exists; vectis WASM CLI still works; host can link without Wasmtime.

---

### R46-S02b — Host in-process detect

**Goal:** Replace workflow WASM dispatch with in-process library calls; restore CI-friendly propose tests.

**Repo:** `specify-cli`

**Prerequisites:** R46-S02a ✅

**Work:**
1. Rewrite `crates/workflow/src/platform/detect.rs`:
   - `vectis_missing_platforms(project_dir, declared) -> Result<Vec<Platform>>` calls `missing_shell_platforms` from `specify-vectis-shell-detect` when Vectis-bound; no `specify-tool`, no `WasiRunner`, no `now: Timestamp` parameter.
   - Keep `is_vectis_bound` gate (non-Vectis → empty `missing`).
2. Add `specify-vectis-shell-detect` to `crates/workflow/Cargo.toml`; remove `specify-tool` from workflow **only if** no other workflow module needs it (init/config still use tool manifest types — likely keep `specify-tool` dep, drop host dispatch only).
3. `src/runtime/commands/plan/propose.rs` — call updated `vectis_missing_platforms` (drop `Timestamp::now()` pass-through if API loses `now`).
4. **Tests — Tier B (no WASM):**
   - `crates/workflow/src/platform/detect/tests.rs` — tempdir + vectis adapter stub; **no** `tools.yaml`, no `SPECIFY_TOOLS_CACHE`, no `vectis_wasm()` assert.
   - `tests/workflow/propose.rs` `platform_project` — patch `vectis-stub` → `vectis` name only; remove WASM/`tools.yaml`/cache wiring; drop `SPECIFY_TOOLS_CACHE` from `propose_reconcile_ok` if unused.
5. Delete error paths only needed for WASM parse (`vectis-detect-parse`, `tool-not-declared` at propose detect) unless still used elsewhere.
6. **Do not** change `.github/workflows/ci.yaml` for vectis-wasm — reusable CI must pass host propose tests as-is ([hard rule](#hard-rule-no-host-runtime--wasm-tests-rfc-46-scope)).

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance) Tier B.
- `cargo nextest run --test plan reconcile` **without** `cargo make vectis-wasm`.
- `cargo test -p specify-workflow platform::detect`

**Handoff:** Propose default-on reconciliation preserved; detect is in-process; CI green on reusable workflow.

---

### R46-S03 — Remove workflow shell heuristics

**Goal:** Delete duplicated Crux detection from `specify-workflow`; vectis-owned library is the sole presence probe.

**Repo:** `specify-cli`

**Prerequisites:** R46-S02b ✅

**Work:**
1. Remove `detect_missing_platforms`, `platform_present`, `walk_dir_recursive` from `crates/workflow/src/change/plan/core/propose/platforms.rs`. Keep `ProjectMissingPlatforms`, `Plan::reconcile_platforms`, and bootstrap entry helpers.
2. Remove `pub use platforms::{..., detect_missing_platforms}` from `propose.rs` / `change.rs` exports.
3. Migrate or delete unit tests in `crates/workflow/src/change/plan/core/propose/tests.rs` that targeted `detect_missing_platforms` — coverage lives in `specify-vectis-shell-detect` + R46-S02b tests + propose integration tests.
4. `rg detect_missing_platforms` across both repos → zero hits (except RFC/plan docs until R46-S04).

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- `rg detect_missing_platforms` in `specify-cli` source (not docs) → empty.

**Handoff:** Workflow kernel is platform-agnostic; all shell presence is vectis-owned. Doc citations (`DECISIONS.md`, `AGENTS.md`, `wasi-tools/vectis/DECISIONS.md`) remain until R46-S04.

---

### R46-S04 — Phase 0 documentation alignment

**Goal:** Close RFC §10 remediations **#2–#4** and prevent prose regression.

**Repos:** both

**Prerequisites:** R46-S03 ✅

**Work — `specify`:**
1. `plugins/spec/skills/plan/SKILL.md` — submit step: `specify plan propose --from …` with **no** `--reconcile-platforms`; describe default-on bootstrap via vectis shell-detect (in-process; not `tool run vectis` from propose).
2. `plugins/spec/references/cli/plan-propose.md` — add bootstrap reconciliation paragraph; document that vectis-owned shell-detect drives insertion.
3. `evals/runs/plan-single-project.pass.md`, `evals/runs/code-multi-slice.pass.md` — drop flag from recorded invocations.
4. Optional but recommended: add `CORE-*` `cli-contract` rule or extend an existing rule config to forbid `--reconcile-platforms` in skills/docs (see `adapters/shared/rules/core/` patterns).

**Work — `specify-cli`:**
1. `DECISIONS.md` §Target platform capability — reconciliation default-on; vectis-owned `specify-vectis-shell-detect` library; no plan-time WASM dispatch.
2. `AGENTS.md` — update `Plan::reconcile_platforms` bullet; remove `detect_missing_platforms` reference.

**Assurance:**
- `make lint` (`specify`) passes.
- `rg --reconcile-platforms` across both repos → only RFC-46, this plan, and intentional historical mentions (if any remain in `rfcs/future/*`, annotate in discovery log).

**Handoff:** Phase 0 code + docs aligned; ready for assurance gate.

---

### R46-S05 — Phase 0 assurance gate

**Goal:** Prove Phase 0 is merge-ready before any `app-icon` work lands.

**Repos:** both

**Prerequisites:** R46-S04 ✅

**Checklist (all required):**
- [x] [Specify-cli step assurance](#specify-cli-step-assurance) (or full `cargo make ci`, which supersedes it).
- [x] [Hard rule](#hard-rule-no-host-runtime--wasm-tests-rfc-46-scope) satisfied for Phase 0 steps.
- [x] `cargo make ci` (`specify-cli`) green.
- [x] `make lint` (`specify`) green.
- [x] `specify plan propose --help` shows no `--reconcile-platforms`.
- [x] Eval scenario spot-check: `plan-single-project` fixture path still valid (re-run or refresh pass summary if your process requires it).
- [x] Cross-repo grep: `detect_missing_platforms` absent from `specify-cli` Rust sources.
- [x] `cargo nextest run --test plan reconcile` passes on reusable CI **without** `cargo make vectis-wasm`.
- [x] `rg WasiRunner` in `crates/workflow/src/platform/` → empty.
- [x] Update RFC-46 §10 table row **#2–#4** status in discovery log or a short comment in tracker **PR / notes**.

**Handoff:** Unblocks Phase 1. Do not start R46-S06 until this step is ✅.

---

## Phase 1 — Policy and gates (no converter yet)

RFC §Implementation phases · Phase 1. Validation and schema land **before** materialize so gates fail meaningfully.

### R46-S06 — `assets.yaml` schema extensions

**Goal:** Encode RFC §3 schema deltas in the embedded assets schema.

**Repo:** `specify-cli`

**Prerequisites:** R46-S05 ✅

**Work:**
1. Edit `wasi-tools/vectis/embedded/assets.schema.json`:
   - Top-level optional `app-icon` → `#/$defs/assetId`.
   - `role` enum add `app-icon`.
   - `symbolEntry`: optional `inferred: boolean` (default false at validate layer if needed).
   - `rasterEntry`: conditional `source` allowed **only** when `role: app-icon`; otherwise reject `source` (use `allOf` / `if-then` or split subschema).
   - `sources.ios` / `sources.android` for `role: app-icon`: accept directory paths (export roots), not only density objects.
2. Add cross-check logic in `wasi-tools/vectis/src/validate/engine/assets.rs` (or a dedicated `app_icon.rs`):
   - `app-icon` id exists and has `role: app-icon`.
   - `kind` vs `source:` extension agreement (`assets-app-icon-kind-source-mismatch`).
   - Raster `source:` constraints for path A (`assets-app-icon-source-invalid`) — dimension/square/alpha rules can be partially deferred to R46-S10 if decoding deps are not yet present; minimum: extension vs `kind` and presence.
3. Run schema embed parity: `crates/schema` if a copy exists; vectis embeds its own — run `cargo test -p specify-vectis` and any `embedded` digest tests.

**Implementation notes:**
- Directory vs file path detection: treat paths without image extensions that exist as directories on disk as export roots during validate; schema alone may use a pattern like `exports/` prefix or `anyOf` file/directory string.
- `schemas/` in CLI repo: sync if there is a mirrored `assets.schema.json` (check `crates/schema/tests/schemas.rs`).

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- `cargo test -p specify-vectis` green.
- Fixture YAML: valid `app-icon` vector + raster examples from RFC §3.1 parse; invalid `source` on `role: icon` raster fails.

**Handoff:** Schema ready for plan validate and materialize.

---

### R46-S07 — Shell-resident launcher probe

**Goal:** Implement RFC §6.3 escape hatch as shared vectis logic.

**Repo:** `specify-cli`

**Prerequisites:** R46-S06 ✅ (soft — can parallel if careful; prefer sequential)

**Work:**
1. Implement `shell_resident_app_icon(project_root, platform)` in a **host-callable shared crate** (extend `specify-vectis-shell-detect` or add a sibling — same pattern as shell-detect). `wasi-tools/vectis` may re-export or call it for validate paths; host `plan validate` links in-process.
2. **iOS:** `iOS/*/Resources/Assets.xcassets/AppIcon.appiconset/Contents.json` exists **and** at least one referenced PNG exists (parse `Contents.json` images array).
3. **Android:** `Android/app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml` **or** any `mipmap-*/ic_launcher.png`.
4. Unit tests in the **shared host crate** with minimal temp trees: skeleton appiconset without PNG → **false**; valid 1024 PNG → **true**. Optional vectis-carve-out smoke only if it calls the same lib without duplicating logic — **no** new `tests/*.rs` `tool run` coverage.

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- `cargo test -p specify-vectis-shell-detect` (or the crate owning launcher) green; `wasi-tools` tests if vectis wires the lib.

**Handoff:** Used by plan validate (R46-S09) and prepare hook (R46-S23).

---

### R46-S08 — Bootstrap context helper

**Goal:** Pure function answering “does RFC §6.1 trigger for this project?” and “which UI platforms in `missing[]` need `app-icon`?”

**Repo:** `specify-cli`

**Prerequisites:** R46-S02b ✅, R46-S07 ✅

**Work:**
1. Host-side or vectis-side helper (prefer **host workflow** so plan validate does not need WASM for logic-only composition):
   ```rust
   pub struct BootstrapContext {
       pub triggers: bool,           // §6.1
       pub missing_ui: Vec<Platform>, // ios/android subset of vectis missing
   }
   ```
2. `triggers` = Vectis-bound project ∧ `missing[]` contains `ios` or `android`.
3. `core`-only missing does **not** trigger (`app-foundation` alone).
4. Reuse `vectis_missing_platforms` / shell-detect library output; do **not** read `plan.yaml` slice names.

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- Table-driven tests: greenfield → trigger + `{ios, android}` (and `core` in missing but filtered out of `missing_ui`); existing shells with launcher → escape handled in R46-S09.

**Handoff:** Single source for §6.1 across plan validate and prepare.

---

### R46-S09 — Plan validate `app-icon` gate

**Goal:** Emit `plan-bootstrap-app-icon-missing` at `specify plan validate` when §6.1 ∧ ¬§6.2.

**Repo:** `specify-cli`

**Prerequisites:** R46-S06 ✅, R46-S07 ✅, R46-S08 ✅

**Work:**
1. Extend plan doctor pipeline (`crates/workflow/src/change/plan/doctor.rs` or new `doctor/bootstrap_app_icon.rs`) — append findings after existing doctor checks.
2. For each platform `π` in `missing_ui`:
   - If `shell_resident_app_icon(project_dir, π)` → pass for `π`.
   - Else evaluate `design-system/assets.yaml`: top-level `app-icon`, entry `role: app-icon`, path A (`source:` materializable — for Phase 1, “materializable” means schema + file existence + square raster decode if PNG; SVG source file exists) or path B (pinned export tree layout §4.2/§4.3 on disk).
3. Diagnostic id `plan-bootstrap-app-icon-missing`; blocking; cite RFC §6.2 bullets in message.
4. Wire `project_dir` into `specify plan validate` handler if not already passed to `doctor()`.
5. Tests in `tests/workflow/validate.rs`: greenfield Vectis project without `app-icon` → exit 2; with valid placeholder `assets.yaml` → pass; shell-resident icon → pass without design-system entry.

**Implementation notes:**
- Path A “materializable” without materialize: Phase 1 checks canonical `source:` file presence + metadata; optional dry decode via `image` crate in host **only** for plan validate if acceptable — otherwise defer strict dimension checks to vectis validate (R46-S10) and keep plan gate on presence + schema.
- Incremental plan (shells present, detect empty) → no gate.

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- `cargo test validate` subset green.
- `specify plan validate --format json` includes finding with `rule-id: plan-bootstrap-app-icon-missing` on negative fixture.

**Handoff:** Gate 1 can block unbootstrappable plans.

---

### R46-S10 — Vectis validate `app-icon` checks

**Goal:** Structural `app-icon` validation in `vectis validate assets` (RFC §7).

**Repo:** `specify-cli`

**Prerequisites:** R46-S06 ✅, R46-S07 ✅

**Work:**
1. Extend `wasi-tools/vectis/src/validate/engine/assets.rs`:
   - `role: app-icon` export tree layout §4.2 / §4.3 (`assets-app-icon-export-invalid`).
   - `sources.ios` ending in `.svg` for `app-icon` → error.
   - `sources.ios` `.svg` for `illustration` → warning (`assets-svg-illustration-on-ios`).
   - Composition-referenced `vector`/`raster` missing `sources.<platform>` **and** no export file → `assets-materialization-missing` (file presence only in Phase 1).
   - Read platforms from `project.yaml` not hardcoded list.
2. Map findings to `diagnostic.schema.json` shape (vectis validate JSON envelope).

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- `cd wasi-tools && cargo test -p specify-vectis` with golden fixtures for invalid appiconset / valid pinned tree — **no** new host `tests/tool run` or `lint project` `kind: tool` tests for RFC-46.

**Handoff:** Operator `specify tool run vectis -- validate assets` enforces inventory shape before materialize exists; coverage stays in the carve-out.

---

### R46-S11 — Scaffold app-icon skeletons

**Goal:** RFC §8 scaffold changes in vectis templates.

**Repo:** `specify-cli`

**Prerequisites:** R46-S06 ✅

**Work:**
1. `templates/vectis/ios/` — add `AppIcon.appiconset/Contents.json` skeleton (empty images or placeholder entry per RFC §4.2); ensure `project.yml` `resources:` includes app Resources path (verify existing).
2. `templates/vectis/android/` — adaptive icon resource stubs referencing materialized layers (`ic_launcher`); align with `AndroidManifest.xml` `android:icon="@mipmap/ic_launcher"`.
3. Update scaffold manifest / `wasi-tools/vectis/src/scaffold/` if paths are generated programmatically.
4. Vectis init / design-system template in `specify` (if separate from CLI templates): document `app-icon` field; **no** default placeholder asset file (RFC §8).

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- `cargo test -p specify-vectis scaffold`; scaffolded iOS tree contains `AppIcon.appiconset`; Android contains `mipmap-anydpi-v26` stubs.

**Handoff:** Bootstrap slices scaffold empty slots materialize fills later.

---

### R46-S12 — Phase 1 documentation and inference policy

**Goal:** RFC §9 docs + §5 inference policy + §10 remediation **#1**.

**Repo:** `specify` (plus `wasi-tools/vectis/DECISIONS.md` in `specify-cli`)

**Prerequisites:** R46-S09 ✅, R46-S10 ✅

**Work:**
1. `adapters/targets/vectis/references/ios/design-system-integration.md` — materialize-before-copy; remove build-time symbol fallback for vector/raster.
2. `adapters/targets/vectis/references/android/design-system-integration.md` — same.
3. `adapters/targets/vectis/briefs/build/ios/write.md` and `android/write.md` — render-by-`kind`; materialize step before copy.
4. `adapters/sources/screenshots/briefs/extract.md` — symbol inference policy (§5).
5. `adapters/targets/vectis/references/layout-inferer-contract.md` — `kind: symbol` + `inferred: true`; branded → `notes.todo`.
6. `wasi-tools/vectis/DECISIONS.md` — replace draft §K/§L stub with §6.1-accurate text (vectis detect only; not plan slice names).

**Assurance:** `make lint` passes; no prose claiming `--reconcile-platforms` or workflow `detect_missing_platforms`.

**Handoff:** Agents building slices see correct writer contract before materialize lands.

---

### R46-S13 — Review rule: render-by-`kind`

**Goal:** `specify lint project` can flag vector/raster ids rendered as platform symbols.

**Repo:** `specify`

**Prerequisites:** R46-S12 ✅

**Work:**
1. Add `adapters/targets/vectis/rules/` entry (e.g. `VECTIS-006-asset-render-by-kind.md`) — forbid `Image(systemName:)` / `Icons.Default.*` for composition-referenced ids that resolve to `vector` or `raster` in `assets.yaml`.
2. Wire into `adapters/targets/vectis/briefs/build/review.md` checklist.
3. If the rule needs mechanical enforcement, add a `specify lint project` predicate or document as review-only for v1 (RFC says SHOULD flag — review-only is acceptable initially; note choice in discovery log).

**Assurance:** `make lint`; rule resolves from vectis review brief.

**Handoff:** Drift detection exists for Phase 2 writer fidelity.

---

### R46-S14 — Phase 1 assurance gate

**Goal:** Schema + gates + docs merge-ready without materialize.

**Prerequisites:** R46-S06 through R46-S13 ✅

**Checklist:**
- [ ] [Specify-cli step assurance](#specify-cli-step-assurance) (or full `cargo make ci`, which supersedes it).
- [ ] [Hard rule](#hard-rule-no-host-runtime--wasm-tests-rfc-46-scope): no new host↔WASM tests added in Phase 1 steps.
- [ ] `cargo make ci` (`specify-cli`).
- [ ] `make lint` (`specify`).
- [ ] Negative: Vectis greenfield project, `specify plan validate` → `plan-bootstrap-app-icon-missing` (host `tests/workflow/validate.rs` — no `tool run`).
- [ ] Positive: valid `design-system/assets.yaml` with `app-icon` + pinned exports (path B) → plan validate passes.
- [ ] `cd wasi-tools && cargo test -p specify-vectis` — `validate assets` + scaffold cases cover guest diagnostics and `AppIcon.appiconset` skeleton (not host `tool run`).

**Handoff:** Unblocks Phase 2 materialize.

---

## Phase 2 — Materialize v1

RFC §Implementation phases · Phase 2.

### R46-S15 — Materialize subcommand skeleton

**Goal:** `specify tool run vectis -- materialize assets` exists and parses args.

**Repo:** `specify-cli`

**Prerequisites:** R46-S14 ✅

**Work:**
1. `wasi-tools/vectis/src/materialize.rs` + `mod materialize` in `lib.rs`.
2. Clap: `materialize assets [path] [--platform <csv>] [--dry-run]`.
3. Subcommand returns JSON summary `{ "materialized": [...], "skipped_pins": [...], "errors": [...] }` (define stable shape; document in `DECISIONS.md` §K).
4. Register in `Args` / `VectisCommand` enum; update `wasi-tools/vectis` CLI tests.
5. Add deps to `wasi-tools/vectis/Cargo.toml`: `usvg`, `resvg`, `image` (and `tiny-skia` if required by resvg) — keep carve-out dep policy.

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- `cargo test -p specify-vectis cli`; dry-run on missing file → structured error.

---

### R46-S16 — Export path conventions

**Goal:** Deterministic default paths for `sources.<platform>` auto-write (RFC §2, Resolved §7).

**Repo:** `specify-cli`

**Prerequisites:** R46-S15 ✅

**Work:**
1. `materialize/paths.rs` — per `role` + `kind` + platform + asset id:
   - `icon` vector: iOS `exports/ios/<id>.imageset/<id>.pdf`; Android `exports/android/drawable/<id_snake>.xml`.
   - `illustration`: iOS imageset PNGs `@2x/@3x`; Android density buckets.
   - `app-icon`: iOS `exports/ios/app-icon/AppIcon.appiconset/`; Android `exports/android/app-icon/`.
2. Snake_case helper for Android drawable ids (match existing writer conventions in design-system-integration docs).
3. Unit tests for path computation (no I/O).

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- Golden path strings for sample ids.

---

### R46-S17 — Materialize icons

**Goal:** SVG → iOS PDF imageset + Android Vector Drawable XML for `role: icon` (and `decorative` by kind).

**Repo:** `specify-cli`

**Prerequisites:** R46-S16 ✅

**Work:**
1. SVG load via `usvg`; lightweight profile check — unsupported features → error naming asset id (RFC §2).
2. iOS: PDF generation into imageset + `Contents.json`.
3. Android: SVG → VD XML pass (dedicated converter module).
4. Respect `--platform` filter and `--dry-run` (log actions only).

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- Unit/integration tests with tiny SVG fixtures; output files exist and are non-empty.

---

### R46-S18 — Materialize illustrations

**Goal:** SVG → rasterized PNG per density for `role: illustration`.

**Repo:** `specify-cli`

**Prerequisites:** R46-S17 ✅

**Work:**
1. `resvg` render at 2x/3x (iOS) and mdpi–xxxhdpi (Android) scales — document scale table in `DECISIONS.md`.
2. Copy-only path for `role: photo` raster masters (per-density `sources` already pinned).

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- PNG dimensions match expected scale; deterministic output for fixed input (hash optional in test).

---

### R46-S19 — Materialize app-icon (iOS)

**Goal:** Path A iOS `AppIcon.appiconset` from 1024×1024 canvas (RFC §4.2).

**Repo:** `specify-cli`

**Prerequisites:** R46-S16 ✅

**Work:**
1. Shared `decode_to_launcher_canvas(source) -> RgbaImage` 1024×1024 — SVG or square raster ≥1024, no upscale, raster alpha rejection for iOS.
2. Write `Contents.json` (`idiom: universal`, `platform: ios`) + single 1024 PNG.
3. Path B: if `sources.ios` pin points at existing export root → skip (silent).

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- actool-friendly layout tests (minimum: valid JSON + PNG present); invalid 512×512 master → `assets-app-icon-source-invalid`.

---

### R46-S20 — Materialize app-icon (Android)

**Goal:** Path A Android adaptive + legacy mipmap tree (RFC §4.3).

**Repo:** `specify-cli`

**Prerequisites:** R46-S19 ✅ (shares canvas decoder)

**Work:**
1. Generate `mipmap-anydpi-v26/ic_launcher.xml`, round variant, foreground drawable/PNG, background color (from `tint` token ref when auto-converting), legacy `mipmap-*/ic_launcher.png`.
2. Safe-zone guidance is operator-facing; enforce central 66% only if feasible without design input — otherwise document as warning not error (note in discovery log if softened).

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- Required XML/PNG artifacts exist; `aapt2`-friendly well-formed XML (best-effort parse in tests).

---

### R46-S21 — Pin semantics and YAML auto-write

**Goal:** Resolved decisions §6–§7: pins win; auto-write absent `sources.<platform>` only.

**Repo:** `specify-cli`

**Prerequisites:** R46-S17 ✅, R46-S19 ✅, R46-S20 ✅

**Work:**
1. Before writing a platform slot: if `sources.<platform>` set **and** path exists on disk → skip slot (no warning).
2. After successful write: merge default path into in-memory `assets.yaml` representation; atomic write back to the same file path read (slice-local or project-level).
3. Use existing atomic YAML writer patterns from workflow/model if callable from WASM tool — **if not**, perform YAML merge in the tool with `serde_saphyr` and write atomically via temp+rename (vectis carve-out owns I/O).
4. `--dry-run` must not write YAML or exports.

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- Pin present → `source:` edit does not overwrite.
- No pin → materialize writes exports **and** updates `sources.ios` / `sources.android` in YAML.
- Idempotent second run → no-op.

---

### R46-S22 — In-scope asset resolution

**Goal:** Pure function implementing RFC §2.1 reference set for prepare hook.

**Repo:** `specify-cli`

**Prerequisites:** R46-S06 ✅

**Work:**
1. `crates/workflow/src/slice/build/materialize_scope.rs` (suggested):
   - Inputs: slice dir, project dir, bootstrap context, effective `assets.yaml` path.
   - Collect ids from: (a) slice `composition.yaml` asset refs if file exists, else (b) parse `spec.md` / `design.md` for asset id references (reuse or extend `evaluate_ui_surface_coherence` patterns), (c) if slice-local `assets.yaml`, all entries with `source:` lacking satisfiable pin.
   - Filter to `vector`/`raster` + bootstrap `role: app-icon` when §6.1 ∧ ¬§6.2 for platform.
2. Heavy unit tests with fixture slice trees.

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- Table tests for design-system bulk pass vs feature slice incremental pass.

---

### R46-S23 — Slice build prepare hook

**Goal:** Auto-run materialize before build brief handoff (RFC §2.1).

**Repo:** `specify-cli`

**Prerequisites:** R46-S21 ✅, R46-S22 ✅, R46-S08 ✅

**Work:**
1. `src/runtime/commands/slice/build.rs` `prepare()` — after `assemble_and_write_request`, before `target.execution.agent`:
   - If target is Vectis: resolve effective `assets.yaml` (slice-local → project `design-system/assets.yaml`).
   - Compute in-scope ids (R46-S22).
   - If any missing export slots: `run_captured vectis materialize assets <path> [--platform …]`.
   - Non-zero guest exit → `target-build-materialize-failed` (new code) abort prepare.
2. Re-run plan-bootstrap app-icon check when bootstrap context applies (same logic as R46-S09 or call shared helper).
3. Document in `docs/standards/workflow.md` if prepare side-effects are listed there.

**Implementation notes:**
- Prepare runs **before** composition regeneration — composition.yaml may be stale; R46-S22 already accounts for this.
- Skills must **not** call materialize directly (RFC §2.1).

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- **Host:** test prepare assembly / error mapping with stubbed materialize outcome or pre-seeded `exports/` on disk — **do not** integration-test `run_captured vectis materialize` from `tests/`.
- **Carve-out:** `cargo test -p specify-vectis materialize` proves guest materialize + YAML auto-write; slice with unpinned SVG → exports created; second run no-op.

---

### R46-S24 — Validate export presence

**Goal:** Post-materialize validation rules (RFC §7) in vectis validate.

**Repo:** `specify-cli`

**Prerequisites:** R46-S21 ✅

**Work:**
1. Extend `assets.rs` validate: composition-referenced `vector`/`raster` must have on-disk export per declared project platform.
2. Distinguish `assets-materialization-missing` vs `assets-app-icon-invalid`.
3. `vectis validate all` fan-out includes new checks.

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- Fixture without exports fails; after materialize passes.

---

### R46-S25 — Acceptance fixtures committed exports

**Goal:** Framework / eval fixtures version-control `design-system/assets/exports/` (Resolved §1).

**Repos:** both

**Prerequisites:** R46-S23 ✅, R46-S24 ✅

**Work:**
1. Pick representative fixture(s) under `evals/fixtures/targets/vectis/` — add `assets.yaml` with `app-icon`, run materialize, **commit** `exports/` tree and auto-written pins.
2. Ensure `.gitignore` does **not** exclude `exports/`.
3. Update fixture README / build brief expectations.
4. Golden layout checks in `wasi-tools/vectis` or fixture docs — **not** a host `tool run materialize` test.

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance) (when adding or changing `specify-cli` Rust in this step).
- CI builds without invoking materialize on every job (exports already committed in fixtures).
- No new host test dispatches vectis WASM to regenerate exports in CI.

---

### R46-S26 — Phase 2 assurance gate

**Prerequisites:** R46-S15 through R46-S25 ✅

**Checklist:**
- [ ] [Specify-cli step assurance](#specify-cli-step-assurance) (or full `cargo make ci`, which supersedes it).
- [ ] [Hard rule](#hard-rule-no-host-runtime--wasm-tests-rfc-46-scope) satisfied for Phase 2 steps.
- [ ] `cargo make ci` (`specify-cli`); `cargo clippy -p specify-vectis` in `wasi-tools/`.
- [ ] `make lint` (`specify`).
- [ ] **Carve-out:** `cargo test -p specify-vectis materialize` — unpinned asset → exports on disk + YAML pins; idempotent second run.
- [ ] **Host:** `slice build --phase prepare` tests assert request assembly / error codes with stubbed or pre-seeded exports — **not** live `run_captured materialize` from `tests/`.
- [ ] Bootstrap context + `app-icon` scope covered in host unit tests (`materialize_scope`, `BootstrapContext`) and carve-out materialize tests.
- [ ] Re-run `specify plan validate` on bootstrappable fixture → pass.
- [ ] iOS `make sim-build` or Android assemble on a fixture project with materialized assets (manual or existing eval) — document result in tracker.

**Handoff:** Unblocks Phase 3 fidelity work.

---

## Phase 3 — Fidelity

RFC §Implementation phases · Phase 3 (partial — `exports.lock` remains deferred per RFC).

### R46-S27 — Writer contract docs

**Goal:** Final alignment of writer-facing docs now that materialize exists.

**Repo:** `specify`

**Prerequisites:** R46-S26 ✅

**Work:**
1. Re-read `design-system-integration.md` (iOS/Android) against actual materialize output paths — fix any drift.
2. `briefs/build/*/write.md` — explicit copy-from-`exports/` steps; remove any remaining “Material icons resolve at call site without copy” for non-symbol assets.
3. `evals/fixtures/targets/vectis/task-list/input/design.md` — update asset narrative if it contradicts render-by-`kind`.

**Assurance:** `make lint`; grep for “fallback” / “without copy” in vectis adapter → only `symbol` entries.

---

### R46-S28 — Vectis verify catalog completeness

**Goal:** RFC §7 — shell tree missing catalog entry for referenced non-symbol asset.

**Repo:** `specify-cli`

**Prerequisites:** R46-S26 ✅

**Work:**
1. Extend `wasi-tools/vectis/src/verify.rs` `VerifyMode::Verify` (or dedicated validate pass):
   - Cross-check composition-referenced `vector`/`raster` ids against `Assets.xcassets` / `res drawable` presence in shell tree.
2. Optional `actool` dry-run behind env flag or deferred if too heavy — document choice in discovery log.

**Assurance:**
- [Specify-cli step assurance](#specify-cli-step-assurance).
- `cd wasi-tools && cargo test -p specify-vectis verify` — `verify --mode verify` fails when shell lacks imageset; passes when exports present on disk (**carve-out only**).

---

### R46-S29 — RFC closure and stale reference sweep

**Repos:** both

**Prerequisites:** R46-S27 ✅, R46-S28 ✅

**Work:**
1. Update `rfcs/rfc-46-asset-materialization.md` status Draft → **Accepted** (or **Implemented**) when all gates pass.
2. `rg` sweep: `--reconcile-platforms`, `detect_missing_platforms`, “fallback to SF Symbols”, draft §K stub.
3. Update `rfcs/roadmap.md` if RFC-46 is listed.
4. `specify-cli` / `specify` `AGENTS.md` cross-links to materialize + app-icon gates.

**Assurance:** No stale contradictions in agent-facing docs.

---

### R46-S30 — Final assurance gate

**Checklist:**
- [ ] [Specify-cli step assurance](#specify-cli-step-assurance) (or full `cargo make ci`, which supersedes it).
- [ ] Full `cargo make ci` + `make lint`.
- [ ] All steps R46-S00–R46-S29 ✅ in tracker.
- [ ] Run one eval scenario end-to-end (`code-multi-slice` or `plan-single-project`) with refreshed pass record if CLI surface changed.
- [ ] [Hard rule](#hard-rule-no-host-runtime--wasm-tests-rfc-46-scope) satisfied across all RFC-46 steps.
- [ ] Operator manual smoke (optional, **not** automated host tests): edit canonical SVG → `materialize assets` → commit exports → build shells.
- [ ] Discovery log reviewed; open items filed as new RFCs (e.g. `exports.lock`, RFC-46a web).

---

## Appendix A — Key file map

| Area | Location |
|------|----------|
| Shell-detect library (authoritative heuristics) | `specify-cli/crates/vectis-shell-detect/` (`shell_present`, `shell_resident_app_icon`) |
| Vectis detect JSON wrapper | `specify-cli/wasi-tools/vectis/src/verify.rs` |
| Host detect gate | `specify-cli/crates/workflow/src/platform/detect.rs` |
| Propose handler | `specify-cli/src/runtime/commands/plan/propose.rs` |
| Bootstrap DAG insertion | `specify-cli/crates/workflow/src/change/plan/core/propose/platforms.rs` |
| Plan validate doctor | `specify-cli/crates/workflow/src/change/plan/doctor.rs` |
| Slice build prepare | `specify-cli/src/runtime/commands/slice/build.rs` |
| Assets schema | `specify-cli/wasi-tools/vectis/embedded/assets.schema.json` |
| Assets validate engine | `specify-cli/wasi-tools/vectis/src/validate/engine/assets.rs` |
| Tool capture helper | `specify-cli/src/runtime/commands/tool/run.rs` (`run_captured`) |
| iOS/Android templates | `specify-cli/templates/vectis/{ios,android}/` |
| Plan skill | `specify/plugins/spec/skills/plan/SKILL.md` |
| Vectis design-system refs | `specify/adapters/targets/vectis/references/{ios,android}/design-system-integration.md` |

## Appendix B — Diagnostic ids (closed set target)

`plan-bootstrap-app-icon-missing`, `assets-app-icon-invalid`, `assets-app-icon-export-invalid`, `assets-app-icon-kind-source-mismatch`, `assets-app-icon-source-invalid`, `assets-materialization-missing`, `assets-svg-illustration-on-ios`, `target-build-materialize-failed` (prepare hook).

## Appendix C — Deferred / out of scope (do not implement in this plan)

- Web asset materialization → [rfc-46a-web-asset-materialization.md](./future/rfc-46a-web-asset-materialization.md)
- `exports.lock` digest sidecar (RFC §2 idempotence — v1 uses file presence)
- Auto-generated brand placeholders (`app-icon: { generated: true }`)
- `plan-bootstrap-slices-missing` structural finding (RFC §6.1 explicitly rejects)
- Repo-wide unwind of **pre-existing** host↔WASM tests (`tests/tool/run.rs`, `tests/tool/contract.rs`, `tests/lint/framework.rs`, skip-when-absent vectis smoke in `catalog_infer`, etc.) — separate effort; RFC-46 must not extend those patterns
