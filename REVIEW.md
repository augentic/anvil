# Specify + Specify-CLI — improve & optimize review

**Mode:** improve / optimize / subtract — *not* new features. **Constraint update (2026-06-12):** backwards compatibility is *not* required — neither for downstream projects' `.specify/` trees across major versions nor for external consumers of published crates. (Lockstep compatibility with the sibling repo at HEAD is a *current* contract and still binds.) This flips three verdicts, marked **[no-backcompat]** below.

**Baseline:** `specify` @ `32f0e2b6`, `specify-cli` @ `ec6cde0f` (2026-06-12, post "Code review (2026-06-12)" merges). Supersedes the earlier 2026-06-12 review — that round is **verified executed** (11/11 items done or consciously partial; see §0).

**Method:** five parallel breadth dives (CLI architecture, CLI test suite, archaeology, specify docs repo, lint pipeline/wasi-tools) + one instrumented `cargo nextest` run (1,781 tests, per-test timings) + two deep recall passes (workflow-crate dead-code/duplication sweep; adversarial "what did the review miss" counter-review). Every headline finding hand-verified against the live tree. The counter-review **vetoed one earlier recommendation** (`SliceReplayCompleted` deletion — it's agent-emitted by documented contract) and corrected one detail (rfc-45 link direction); both fixes are folded in below.

---

## TL;DR — where to focus, ranked

| # | Finding | Repo | Effort | Payoff |
|---|---------|------|--------|--------|
| **1** | **No wasmtime compilation cache — ~9–10 CPU-minutes per test run re-JIT-ing the same vectis.wasm**; 11 tests at 38–73s, everything else ≤3.4s. Three deployment caveats in §1 | specify-cli | S–M | Biggest CPU win; also production cold-starts |
| **2** | **Release & CI-gate integrity:** crates.io publish job fails by construction (path-deps without versions, 5-of-11 crate order) — **[no-backcompat]: delete it outright**; `cargo make vet` regenerates exemptions before checking so it can never fail; `fmt` task mutates instead of checking inside `ci`; `outdated --exit-code 1` is a time bomb | specify-cli | S | Gates that look strong but aren't |
| **3** | **Archaeology wave A+:** Road-B-as-WASI falsehoods in `AGENTS.md`/`DECISIONS.md`, six deleted-`framework::check` citations in `skill-authoring.md`, broken RFC-42/43 links, false crate-graph claims (`specify-tool` ⊄ diagnostics; phantom `regex` dep), 18 dead `REVIEW.md <item-id>` citations | both | S–M | Doc trust; agents act on these files |
| **4** | **Workflow crate surface debt (~1,100+ lines with [no-backcompat]):** never-shipped `workspace status` kernel (~220 lines test-only), dead forge PR-view (~90), journal emission forked 4 ways with a silent dropped-event gap, ~30 `Timestamp::now()` violations of the documented `ctx.now()` seam, platform-validation rules implemented twice, **plus the entire dormant migration framework (§2.7)** | specify-cli | S–M | Dead code + two consistency contracts restored |
| **5** | **Scenarios checker runs all 5 sub-checks then filters, ×5 rules** — plus scenario discovery implemented three times | specify-cli | M | `lint framework` speed + dedupe |
| **6** | **RFC orphanage + ~120 RFC-tagged code comments** (RFC-40/44 implemented but unarchived; `infer.rs` 14, `end_to_end.rs` 15, `synthesize.rs` 12) | both | S–M | Forward-looking hygiene, mechanical |
| **7** | **Dead wire surface on v1 schemas** — `text_matches` never populated, `marketplace_entries`/`rule_index` built-but-unread, `baseline_present` hardcoded false. (**Keep** `SliceReplayCompleted` — agent-emitted replay hook) | specify-cli | S | Shrink contracts before they calcify |
| **8** | **Test layer duplication** — schema triple stack, journal/init overlap, framework text-path; ~60–100 tests foldable. lint_hint fold **gated** on writing kernel coverage first (13/17 eval modules have zero unit tests) | specify-cli | M | Suite size / maintenance |
| **9** | **specify-repo hygiene** — version skew (VERSION 0.26.0 vs marketplace 0.27.0 vs client 0.26.0), `Specify.toml branch = "main"` moving target, CORE-025/050 hint fan-out, duplicated skill prose, `names.md`/`pitch.md` at root, 7.4G `.sandbox` | specify | S | Signal-to-noise + reproducible lint |

---

## Part 0 — prior round: verified executed

All Day-1/Week-1 items from the previous review landed: `composition` validate namespace resolved, B-2 exit executed (six checkers in-process; `framework-wire`, blobs, sidecars, `framework-wasm` pipeline fully gone from git), `specify-tool-manifest` extracted, `workflow/schema.rs` deduped, vestigial `Check` substrate deleted, vectis `composition.rs` split with typed `Finding`, orphan plugins deleted, CORE-008/012/058 retired, RFC-45 archived, evals parked coherently, `make lint` binary caching landed, and 9 of 11 test-suite cleanups (schema smokes table-driven, `LazyLock` contract dump, `OnceLock` tool fixtures, shared `scaffold_framework`, plan-lock parameterized, platform serde consolidated, help via contract dump, `rust_quality` single scan, `tests/plan/schema.rs` relocated).

**Conscious partials:** `crates/workflow/tests/workspace.rs` was scoped-by-documentation (lines 4–13) rather than line-shrunk — 943 lines / 26 tests remain but assert different things than the binary layer; the residue is ~120 lines of duplicated `run_git`/`GIT_ENV` helpers. CORE-025 collapsed 8 regex → 1 alternation but still carries 7 `path-pattern` hints.

---

## Part 1 — CPU: the test suite's hot spot is wasmtime, not test count

Instrumented run: **1,781 tests, 76.4s wall**, and the tail is brutal —

```text
PASS [ 73.4s] specify::catalog_infer bind_operator_part_wins_slug_over_skill
PASS [ 71.5s] specify::plan end_to_end::rfc40_composition_inference_capstone
PASS [ 70.8s] specify::catalog_infer report_clusters_with_candidate_cache   (+5 more 40–71s)
PASS [ 39.1s] specify::tool schema::schema_vectis_* (×2)
-- next slowest test in the entire suite: 3.3s --
```

Root cause (verified): `Host::new` builds the engine with bare `Config::new()` + `wasm_component_model(true)` — **no compilation cache** (`crates/tool/src/host.rs:66`), and the workspace `wasmtime` dep doesn't enable the `cache` feature (`Cargo.toml:192`). Every spawned `specify` process Cranelift-compiles the full vectis component from scratch; in tests that binary is a **debug** build, so each compile costs 40–70s. `[tasks.test]` depends on `vectis-wasm` (`Makefile.toml:74–77`), so every `cargo make test` / CI run pays ~9.5 CPU-minutes recompiling the **same bytes** eleven times.

Fixes, in order:

1. **S — enable the wasmtime `cache` feature + engine cache** in `Host::new`, **with three caveats** from the counter-review: (a) nextest runs the 11 tests as concurrent processes, and the disk cache only dedupes *completed* compiles — pre-warm with one compile (e.g. in the `vectis-wasm` task) or put the wasm tests in a serial nextest group, or the first wave still pays full price; (b) point the cache dir under the existing `SPECIFY_TOOLS_CACHE` root, not default `~/.cache/wasmtime` — integration tests inherit `HOME` (`tests/common/mod.rs:54–55` strips only two vars) and would pollute developer machines; (c) `Swatinem/rust-cache` does not persist `~/.cache/wasmtime`, so CI needs an explicit cache step, and the `cache` feature's new transitive deps mean one-time cargo-vet/deny churn.
2. **S — `[profile.dev.package]` opt-level bump for `wasmtime-cranelift`/`cranelift-*`** so debug test binaries compile WASM at usable speed. Safe; sole cost is one cold rebuild.
3. **M (optional) — precompile to `.cwasm`** keyed by the existing sha256 in the digest-addressed tool cache. Only if production cold-start matters; do 1+2 first.

Secondary CPU sinks (an order of magnitude smaller): `tests/journal.rs` (26 × fresh init+CLI), `tests/init/base.rs` (15 × cold scaffold), 13 full framework-lint walks across `tests/lint/framework{,_json}.rs`, `review_run_byte_stable` running the full lint twice (`tests/lint/project.rs:246–253`). Also: **no `.config/nextest.toml`** — no slow-timeout (the 73s JIT tests just sit), no retries, no CI profile; `cargo make ci` recompiles the workspace under up to three flag universes because `RUSTFLAGS=-Dwarnings` is set only on `[tasks.test]`.

---

## Part 2 — specify-cli architecture

### 2.1 Scenarios checker: run-all-then-filter ×5 (verified)

`framework_tools/scenarios.rs:42–51` runs `run_with_config` (frontmatter + traces + catalog, full tree walks of `evals/`, `adapters/targets/`, `plugins/`) and *then* filters by the scoped rule — with 5 `kind: tool` scenario rules, one `specify lint framework` does **5 full passes** of identical work. `skill_body` (`:63–76`) and `rules` (`:43–55`) already dispatch per-rule; make scenarios match. Scenario discovery also exists in **three places** (`index/scenario.rs:95–151`, `scenarios.rs:380–439`, the catalog module's walk) — extract one `discover_scenario_candidates`. Effort M, payoff M-H (this is `make lint` latency in the specify repo).

### 2.2 Dead wire surface — shrink the v1 schemas now (verified, one correction)

- `WorkspaceModel.text_matches` (`model.rs:170`): **never populated** — both indexer profiles hard-code `Vec::new()` (`index.rs:183,311`). Delete field + schema property.
- `marketplace_entries` + `rule_index`: built every framework index but **no evaluator reads them**. Either stop building them or migrate the marketplace checker to Road A over the facts (preferred — deletes an in-process checker).
- `LintCompletedPayload.baseline_present`: hardcoded `false` by the only producer (`journal/emit.rs:53`), no reader branches on it. Speculative wire field — prune or wire it.
- In-process checkers still serialize a JSON `DiagnosticReport` that `eval/tool.rs:149–159` immediately reparses (`framework_tools.rs:85–86`). Add a typed `run_diagnostics()` path on `ToolRunner` for in-process tools; keep JSON for WASI.
- **Correction — keep `EventKind::SliceReplayCompleted`.** The earlier draft recommended deletion ("zero emitters"); that grep was scoped to Rust call sites and missed the agent surface: the variant is emitted via `specify journal emit slice.replay.completed` by the documented replay target hook (`adapters/shared/target-hooks/replay/hook-contract.md:33`, `journal-payload.md:7`, `docs/standards/cli-contract.md:111`, omnia `briefs/build.md` replay phase). Deleting it would turn the Omnia replay step into a `journal-emit-unknown-event` exit-2 failure. It is production surface.

### 2.3 Journal emission: one contract, four implementations (verified)

The canonical pair is `journal::emit_best_effort` (records dropped events in the `.specify/journal.dropped` sidecar) and the generic `slice.rs::bracket` (56–79, `ctx.now()`). Divergent copies: `slice/build.rs:164–195,342` (hand-rolled bracket, `Timestamp::now()`, `eprintln!` on failure — **no sidecar record**, so a dropped `slice.build.*` event vanishes silently while a dropped `slice.merge.started` does not), `slice/merge.rs::emit_archive_created:104–129` (same sidecar gap), `synthesize.rs:359–362` (fatal `append_batch`). Collapse all three onto `bracket`/`emit_best_effort` — ~50–60 lines and one unified dropped-event contract.

Related: `context.rs:86` documents that handlers "never call `Timestamp::now()`", yet ~30 call sites across `src/runtime/commands/**` do exactly that (plan/lifecycle.rs ×4, source/op.rs ×2, slice/build.rs ×2, tool/run.rs ×2, init.rs, migrate.rs ×2, workspace/{sync,push}.rs, lint/{project,framework}.rs, …) versus 4 files using `ctx.now()`. Either sweep them onto `ctx.now()` (mechanical) or amend the doc — today the stated seam is fiction for most verbs.

### 2.4 Workflow dead code (deep sweep — all verified by caller census)

- **`registry/workspace/status.rs` (~220 lines): test-only.** `SlotStatus`/`SlotKind`/`status()`/`status_projects()` are pub and re-exported (`workspace.rs:20`) but consumed only by `crates/workflow/tests/workspace.rs` and `slot_problem.rs` (which uses only `SlotKind`). There is no `workspace status` CLI verb (`WorkspaceAction` = Sync | Prepare | Push) — a planned verb that never shipped. Delete (move `SlotKind` into `slot_problem.rs`), or ship the verb; don't keep the limbo.
- **`registry/forge.rs` PR-view surface (~90 of 113 lines): dead.** `PrView`/`PrState`/`pr_view_for_branch` have zero callers anywhere — `/spec:finalize` observes PRs agent-side via `gh`. Fold the one live item (`project_path`) into its caller, delete the file.
- **Small demotions:** `ProjectConfig::rule_path` (test-only), `design_system::{from_yaml,confirmed_slugs,rejected_slugs}` (test-only pub), `decisions::dec_number` + `init/adapter_uri::ensure_adapter_dir` (over-visible), `agents/src/lock.rs:40–46` `#[cfg(test)]` struct in production source, stale `dispatch_survey_tool` comment at `slice/build.rs:15` (symbol exists nowhere — violates AGENTS.md rule 4).

### 2.5 Duplicated kernels (verified)

- **Platform-validation rules ×2:** `init.rs:223–271` (`project-platforms-*`) vs `registry/topology.rs:252–303` (`topology-cache-project-platforms-*`) — same three rules, two encodings, the comment admits the mirror. Extract one kernel returning typed violations each caller maps to its code family (~35 lines + drift risk).
- **Diagnostic-report render + gate error ×2:** `slice/validate.rs:59–88` vs `plan/lifecycle.rs:~363–389` — same renumber → report → PASS/FAIL banner → `Error::validation_failed` shape; only the per-finding formatter differs (~25–30 lines).
- **Kebab validators ×3:** `specify_error::is_kebab` vs `catalog/infer.rs:411` vs `change/plan/core/model.rs:327` (the locals also require a leading alpha — consolidate via a 2-line wrapper, not drop-in).
- **Git wrapper families:** classification/error-mapping re-rolled per site over the shared `cmd.rs` spawn boundary (`registry/workspace/git.rs`, `init/git.rs`, `branch.rs`; `merge/clone_commit.rs` justified). A classified-run helper in `cmd.rs` collapses ~60–80 lines.
- **Fine:** the `source survey`/`extract` two-phase `op::Flow` kernel is shared and exemplary; `slice build`'s schema machinery is justifiedly its own; `catalog infer` is a different domain shape.

### 2.6 Seam verdict: no structural action

The deep pass looked for extract-worthy kernels in `registry/` (4.2k) and `change/plan/` (4.0k) and concluded **against** extraction: both trees decompose cleanly on inspection (`core/` per-verb files at healthy sizes; `propose/` already kernel-ized; `model.rs`/`status.rs` exceed the 400-line soft cap but are cohesive). What they've accumulated is the surface debt in §2.3–2.5 and §2.7, shippable in a week, not rot needing new seams. The journal taxonomy is **justified**: it's an external wire contract (`journal emit` deserializes the full closed set; parent-repo evals and skills assert on journal lines) — prune only `baseline_present` (and the two migration variants per §2.7).

### 2.7 Migration framework: delete **[no-backcompat]** (was "frozen, keep")

The machinery exists solely to migrate *existing* consumer projects across majors — the one concession in an otherwise hard-cut-at-majors posture ("no silent compatibility aliases"). With backwards compatibility dropped, a major mismatch becomes "re-init with a clear error" and the whole apparatus goes. Deletion inventory (footprint verified across 15 files):

- `crates/workflow/src/migrate.rs` (~500 lines: empty `MigrationKind {}`, `Migrator` trait, `MigrationPlan`/`Report`/`Action`, `apply_staged`, `migrator_for` that matches nothing) + its tests.
- `specify migrate` verb (`src/runtime/cli.rs`, `src/runtime/commands/migrate.rs`) and `init --check-migration` (`commands/init.rs`, `init/upgrade.rs` probe path); `tests/bootstrap/migrate.rs`, the `--check-migration` cases in `tests/init/base.rs`.
- `Error::ProjectNeedsMigration` + **exit code 4** (`crates/error`, `src/runtime/output.rs`, `tests/cli/errors.rs`) — the exit table shrinks to 0–3. Exit 3 (`CliTooOld`) stays: that's a forward guard, not backcompat.
- Journal variants `migration.applied` / `migration.skipped` (`journal/event.rs` + wire-shape tests) — dead once no migrator can run.
- `ProjectConfig::load_for_migration` **shrinks but survives** — `upgrade` and `plugins` still need version-tolerant config loads; rename to match its remaining consumers.
- Same-PR doc sweep: DECISIONS.md "Bootstrap, upgrade, and migration lifecycle" rewritten to the re-init posture; the "registered `MigrationKind` before `specify_version` rolls" gate removed from both repos' AGENTS.md (cli gotchas + parent "Crossing a major" gotcha); add one DECISIONS line pinning the new posture ("pre-1.0: no compat shims, no versioned parsing, majors are re-init") so it stops being re-litigated.

Two more **[no-backcompat]** unlocks land elsewhere: the publish job becomes an unambiguous delete (§5.1), and all `pub` demotions/renames in §2.4 are SemVer-free — the deep pass's hedge about crates.io publishing evaporates.

### 2.8 Explicitly fine (re-verified — don't spend budget here)

Crate graph + dependency-direction invariants; handler shape (no `match ctx.format`); error enum at 13 variants; all four diagnostic renderers; `plan_lock.rs`; `cli_contract.rs` (4 selectors, real consumers); `set-eq` single-consumer evaluator; `crates/tool` cache/permissions machinery; vectis composition split; `agents` crate proportionality; `decisions.rs`/`upgrade.rs`/`init --upgrade`/topology lock (all consumer-verified); `model/atomic.rs`, `spec/provenance.rs`, diagnostics fingerprint (golden + proptests); tool permissions/host hardening; zero production TODO/FIXME; no `#[allow]` without reason.

---

## Part 3 — Testing

### 3.1 Remaining layer duplication (fold candidates, ~60–100 tests)

| Area | Duplication | Action |
|------|-------------|--------|
| **Schema triple stack** | `crates/schema/tests/schemas.rs` (keep) + `workflow/tests/goldens/schemas.rs` (~5–8 re-compile smokes) + `workflow/src/schema/tests.rs` (overlap) | Schema crate = single compile/parity home; keep only behavior edges in workflow |
| **Journal** | `journal/tests.rs` (947 lines) wire round-trips vs `tests/journal.rs` (26) binary goldens | Trim CLI-emitted round-trips covered by goldens; **keep agent-emitted rows** (`slice.replay.completed` has no binary-side replacement unless goldens drive `journal emit`) |
| **Init workspace shape** | `init/workspace/tests.rs::canonical_shape` vs `tests/init/base.rs::workspace_writes_canonical_shape` | Keep binary, fold unit |
| **lint_hint middle layer** | 74 crate-integration tests; **but** only 4 of 17 eval modules have any unit tests (field_grammar 1, finding 6, path_pattern 2, regex 1 — the other 13 incl. schema/tool/unique/presence have **zero**) | **Gated:** write the kernel matrix first, then fold; deleting now loses the only per-kind behavioral coverage |
| **lint framework text path** | `tests/lint/framework.rs` (9 full walks incl. pretty/text) vs `framework_json.rs` goldens | Keep JSON goldens + 1 smoke |
| **Model round-trips** | ~6–10 of 20 in `change/plan/core/model/tests.rs` assert serialize→parse identity only | Delete the pure-identity ones |

Also: extract the duplicated `run_git`/`GIT_ENV`/`copy_dir` helpers shared between `tests/common/mod.rs` and `crates/workflow/tests/` (~120 lines); add a `.config/nextest.toml` (slow-timeout, retries=0 explicit, CI profile); note `ignore_directive_pass.rs:316–322`'s env-mutation SAFETY argument holds only under nextest's process-per-test, not plain `cargo test`.

**[no-backcompat] Python-parity goldens — reframe or delete.** `crates/workflow/tests/goldens/parity.rs` (11 tests + `tests/fixtures/parity/`) exists by charter to stay "byte-for-byte" with "the archived Python reference implementation (now retired)" — backwards compatibility with a dead program, and archaeology under the forward-looking rule. Either delete (after confirming `merge_slice.rs` + `tests/slice/merge.rs` cover the merge edges these pin) or keep the useful subset as plain merge goldens with the Python framing scrubbed.

### 3.2 Acceptance tests — proportionate, keep

`tests/plan/end_to_end.rs` (5 tests, 33 binary invocations) explicitly delegates per-verb edges to per-verb suites (`:36–49`) and asserts composed behavior only — policy working as documented. The 71s capstone pain dissolves with Part 1.

### 3.3 Keep as-is

Three-layer pyramid doc; `plan_lock` unit/binary split; propose kernel matrix + integration; journal goldens; the 574 inline kernel unit tests in workflow; fixtures (all referenced, ~2 MB). (`goldens/parity.rs` moved out of this list — see the [no-backcompat] note in §3.1.)

---

## Part 4 — Archaeology (forward-looking cleanup plan)

Counts after the deep passes: **A = 23 actively-misleading, B ≈ 76 noisy, C (keep) ≈ 32.**

### Wave A — present-tense falsehoods (1–2 small PRs, do first)

| Where | Problem | Fix |
|-------|---------|-----|
| `specify-cli/AGENTS.md:40–41` | Road B described as "name-resolved WASI tool (`wasi-tools/<name>/`)" — contradicts line 44 in the same file | Rewrite to in-process checkers under `framework_tools/` |
| `specify-cli/AGENTS.md` crate graph + `architecture.md:17` | Claim `specify-tool` depends on `specify-diagnostics` — it doesn't (`crates/tool/Cargo.toml:23–26`); architecture.md also omits the real `specify-schema` dep | Correct the graph |
| `specify-cli/AGENTS.md:44` | wasi-tools deps claimed "serde / serde-saphyr / jsonschema / regex only" — no `regex` exists; `clap`/`thiserror`/`toml`/`sha2`/`semver` do | Correct the list |
| `specify-cli/DECISIONS.md:61` | "No framework `CORE-*` rule runs as an in-process check" — false post-B-2 (§89 of the same file) | Reword to "no rule uses the deleted `Check` substrate" |
| `specify-cli/src/runtime/commands/lint/framework.rs:11–12,80–82` + `docs/contributing/checks.md:85,404` (both repos) | "`rules` WASI tool" prose | "in-process checker" |
| `specify-cli/docs/standards/architecture.md:46` + `:24` | Lists 4 eval kinds (15+ exist); "reserved kinds" claim contradicted by `rule.schema.json:125`; "WASI tools" wording | Sync with reality |
| `specify-cli/docs/standards/testing.md:13` | Phantom `cache` test binary listed; real `catalog_infer`/`cli_contract` binaries omitted | Sync |
| `specify-cli/docs/release.md:38,71,100–108` | Documents `Formula/specify.rb` and a root `install.sh` — **neither exists**; 3-crate publish order contradicts both the workflow (5) and the real graph (11) | Fix alongside §5.1 |
| `specify/docs/standards/skill-authoring.md:11,15,19,34,76–77` | Six citations of deleted `specify_standards::framework::check::*` modules as live enforcement | Cite CORE rule ids / `framework_tools/` modules |
| `specify/evals/scenarios/intent-only.md:59` + `evals/shared/assertions.md:8` | Links to deleted `rfcs/rfc-42-acceptance.md` / `rfc-43-release-proving.md` | Archive those RFCs or rewrite self-contained |
| `specify/rfcs/roadmap.md:28` | Present-tense `specdev lint` / `specrun lint` | `specify lint framework` / `specify lint project` |
| `specify-cli` workflow §-anchor drift | `§Adapter axis` (`adapter/core.rs:54`, `tool-manifest/lib.rs:294`), `§The plan gate` (`transitions.rs:6`, `tests/workflow/transition.rs:132`), `§Time injection` cited against the wrong doc (`journal/emit.rs:12`) | Retarget (sampled ~10 others: they resolve) |
| `specify-cli` ×18 | Stale `REVIEW.md <item-id>` citations (A3, A6, B4, B5, A19…) in `Makefile.toml:69`, `src/output.rs:64`, `fingerprint.rs:106`, `source/op.rs:2,36`, `tests/workflow/*`, … — ids from a superseded REVIEW.md revision in the *other repo*; unresolvable | Replace with self-contained rationale or DECISIONS links |

### Wave B — RFC orphanage + comment scrub (mechanical)

1. **Archive implemented RFCs with the RFC-45 pattern:** `rfcs/rfc-44-architecture-seams.md` and `rfcs/RFC-40-composition-accumulation-and-component-inference.md` → `rfcs/archive/`, outcomes recorded in `specify-cli/DECISIONS.md`. **When moving rfc-44, update rfc-45's two `../rfc-44-architecture-seams.md` links (lines 3, 68) to `./`** — the move breaks them otherwise. Archive RFC-44 verbatim (it cites old REVIEW item-ids; it's history, don't chase them).
2. **Strip RFC/§ tags from Rust comments** per `style.md` (~120 hits / ~35 files). Worst: `wasi-tools/vectis/src/infer.rs` (14), `tests/plan/end_to_end.rs` (15), `tests/slice/synthesize.rs` (12), `tests/catalog_infer.rs` (6), `src/runtime/commands/catalog/infer.rs` (5). Replace `RFC-40 §B6` with behavior names; one DECISIONS link per module header max.
3. **Era-framing in shipping surfaces:** `components.schema.json:30` ("pre-RFC-40"), `design_system.rs:9–10`, `index.rs:4` ("Phase 2"), `index/scenario.rs:12`, `tests/common/mod.rs:27,237`, ~15 one-liners in `specify/docs/reference/cli/*.md` ("renamed from", "retired verb family") — state what *is*.
4. `docs/quality-debt.md` now meets its own deletion criterion (every T1 row "Done") — delete or restate as empty.

### Keep (category C — not archaeology)

`DECISIONS.md` as the history home (fix only §61); `workflow.md` RFC-tagged spec language; `rfcs/` + `roadmap.md` forward material; CORE-016 (already exempts `REVIEW.md` and `adapters/shared/rules/**` — this cleanup is what it enforces going forward); RFC-5322 (IETF, not history); Road A/B vocabulary; `legacy-monolith` fixture names; the post-bridge regression tests in `tests/lint/framework.rs:146–171`.

---

## Part 5 — Build, release & supply-chain (new this round, all verified)

1. **`publish-crates-io` fails by construction — [no-backcompat]: delete it.** `release.yaml:226–247` publishes 5 of 11 crates in an order that skips `specify-diagnostics`/`-schema`/`-digest` (which `specify-model` needs), and every workspace dep is **path-only with no `version`** (`Cargo.toml:174–184`) — `cargo publish` rejects that at packaging. Only `specify-error` can succeed; the job dies at step 2 of every tagged release. With no external crate consumers to support, the fix-properly branch (add `version =` to 11 path-deps, publish the full graph) has no audience: **delete the job**, declare the binary + WASI packages the only release artifacts, and excise the corresponding `docs/release.md` sections (already false — see §4 Wave A). (S/H)
2. **`cargo make vet` can never fail:** `Makefile.toml:99–106` runs `cargo vet regenerate imports/exemptions/unpublished` *before* `cargo vet --locked` — regeneration auto-exempts anything unaudited (`config.toml` carries 361 exemptions, `[audits]` is empty). Split: regenerate = manual on dep-add (as `architecture.md:117` already describes); check = CI. (S/M-H)
3. **`[tasks.fmt]` mutates inside `ci`/`check`** (`cargo +nightly fmt --all`, no `--check`) — violations are silently rewritten, never failed. Add a `fmt-check` task for `ci`. (S/M)
4. **`[tasks.outdated]` uses `--exit-code 1`** — any upstream release fails `ci` for unrelated changes. Demote to advisory or scheduled job. (S/M)
5. **Dep hygiene:** `walkdir` declared in `[workspace.dependencies]` with zero consumers; host↔wasi-tools pin skew (`serde-saphyr` 0.0.26 vs 0.0.25 — semver-incompatible 0.0.x pins across a shared YAML wire boundary; `sha2` 0.11 vs 0.10; `toml` 1 vs 0.8 — verify direction per workspace and align). (S/L-M)
6. **specify-repo CI runs the framework lint via a debug `cargo run`** while `DIAGNOSTICS.md:63` itself warns debug is "many times slower and not representative" — build `--release` once and reuse. (S/L)

---

## Part 6 — specify repo

1. **Version skew across release surfaces (verified):** `VERSION` = 0.26.0, `marketplace.json` = 0.27.0, `plugins/spec` = 0.27.0, `plugins/client` = 0.26.0 — a partial hand-bump bypassed the release workflow, and no lint checks version coherence. Re-sync now; optionally add the check to CORE-022's checker while touching it. (S/M)
2. **`Specify.toml` pins `branch = "main"`** — an unpinned moving target: `scripts/specify.rs:183–189` resolves via `git ls-remote` on every `make lint`, and any specify-cli push invalidates `.cli/` → minutes-long wasmtime rebuild for every contributor. Pin a `rev`/`tag` (CORE-055 requires *fetchable*, not *unpinned*); CI's branch-matching checkout stays as the co-dev path. (S/M)
3. **Hint fan-out:** CORE-025 still carries 7 `path-pattern` hints; CORE-050 carries 8 `regex` + 2 `path-pattern`. Collapse each to the CORE-025-regex pattern. (S)
4. **Skill prose duplication:** plan-lock acquisition snippet in 4 phase skills; workspace routing in 3; "never hand-edit" guardrails across most. Move to `plugins/spec/references/` and link. (S)
5. **Root hygiene:** `names.md` (172 lines of trademark shortlists) and `pitch.md` are off-mission at the repo root — relocate. `docs/reference/change-component.md` is a 3-line retired stub with only a book.toml redirect — delete both ends. (S)
6. **Evals:** add a `.sandbox` prune note (7.4G gitignored); spot-check `capture-wiretapper`'s 46-line `## Critical Path` against CORE-045's 45 cap with `make lint`. (S)
7. **Reference corpora — defer but monitor:** omnia (16.6k) + vectis (12.1k) references = 65% of adapter lines; act only if agent context cost demonstrably hurts, and then by tiering `examples/` into an archive layer. (L — defer)
8. **Verified healthy:** marketplace ↔ plugins coherent; all 10 skills within caps; all 30 symlinks resolve; references zero orphans; 9 passed scenarios ↔ 9 run records, 4 parked coherently; CLI-verb drift across 15 sampled invocations: none; mdBook genuinely published (Cloudflare Pages, linkcheck error-policy); source briefs consistent (~920 lines); capture/client plugins proportionate; `scripts/specify.rs` robust.

---

## Suggested execution order

1. **Day 1:** wasmtime cache + cranelift dev-profile opt with the §1 caveats; archaeology Wave A (both repos, incl. crate-graph and release.md falsehoods); re-sync specify version skew.
2. **Week 1:** release/CI-gate integrity — delete the publish job, split vet, `fmt-check`, demote `outdated` (§5); workflow surface-debt batch — delete `status.rs` + `forge.rs` PR surface + **migration framework (§2.7, incl. exit code 4 and both repos' doc gates)**, unify journal emission, time-injection sweep, shared platform/render/kebab kernels (§2.3–2.5 + §2.7, ~1,100+ lines); pin `Specify.toml` to a rev.
3. **Week 2:** scenarios per-rule dispatch + single discovery (§2.1); dead wire surface (§2.2 — keeping `SliceReplayCompleted`); RFC archive + comment-scrub waves (§4B); test-layer diet incl. nextest.toml, parity-goldens reframe/delete, **lint_hint kernel matrix before any fold** (§3.1); CORE fan-out + skill prose dedupe + root hygiene (§6).
4. **Monitor, don't act:** workflow seam extraction (deep pass concluded against), `plugins.rs` re-home, reference-corpora tiering, `crates/tool` fleet machinery, journal taxonomy (external contract — keep).

---

**Caveats:** timings are from one warm-cache local run at the debug profile `cargo make test` uses. Hand-verified this round: wasmtime cache absence + `[tasks.test]`→`vectis-wasm`, scenarios filter-after-run, `text_matches`/`rule_index`/`marketplace_entries` reader absence, `baseline_present` hardcoding, the `SliceReplayCompleted` agent-emission contract (veto), `status.rs`/`forge.rs` caller censuses, publish-job path-dep gap, vet/fmt/outdated task definitions, version skew, eval-module test counts (13/17 at zero), `specify-tool` dep list, wasi-tools dep list, stale REVIEW item-id citations, CORE-025/050 hint counts, broken rfc-42/43 links, the migration-framework footprint (15 files), and `parity.rs`'s retired-Python charter. Remaining counts come from the seven sub-audits, spot-checked but not exhaustively re-derived. The **[no-backcompat]** markers assume lockstep releases with the sibling repo remain coordinated — they license breaking *old* consumers, not *current* ones.
