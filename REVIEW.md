# Specify + Specify-CLI — improve & optimize review

**Mode:** improve / optimize / subtract — *not* new features. Native-hint work, RM-11+, shape-trace work, and feature scenarios stay deferred.

**Baseline:** `specify` @ `5bc95dce`, `specify-cli` @ `b1124be1` (2026-06-10). Supersedes the 2026-06-07 review — every finding from that round was executed (see its post-mortem); this round re-verifies the live tree and focuses on the commits that landed since: cross-repo decoupling (#154/#62), CLI version binding (#155/#63), acceptance simplification (#153/#61), bin/Specify.toml simplification, and the `code-typescript` → `typescript` rename.

**Method:** four parallel deep-dives (CLI code, CLI tests, specify docs/adapters, wasi-tools), with the headline claims re-verified by hand: full `cargo nextest run --all --all-features --no-fail-fast` executed (1707 tests), `make lint` executed (green, 0 findings), `.bin/` tracking confirmed via `git ls-files`, CI workflow read directly.

---

## TL;DR — where to focus, ranked

| # | Focus | Repo | Effort | Payoff |
| - | ----- | ---- | ------ | ------ |
| **0** | **`cargo make check` is RED on tip of main** — one 43-char test fn name trips the `rust_quality` gate | specify-cli | XS | Unblocks CI; 1-line rename |
| **0** | **37.7 MB `specify` binary is tracked in git** (`.bin/bin/specify`) | specify | S | Repo bloat every push; contradicts "no published binary" story |
| **1** | **Committed `Specify.toml` pins `path = "../specify-cli"`** — clean clones can't `make lint`; violates the repo's own CORE-055 policy | specify | S | Restores the documented contributor contract |
| **2** | **`wasi-tools/` (~150 tests, 9 crates, ~11.8k LOC) is never built, linted, or tested in CI** | specify-cli | M | Closes the single biggest test blind spot |
| **3** | **CI docs ≠ CI reality** — AGENTS.md/checks.md describe `make lint` + nightly cargo-script; `ci.yaml` actually uses stable + sibling checkout + direct `cargo run` | specify | M | Operator/contributor-facing contradiction |
| **4** | **Exit code 4 (`EXIT_MIGRATION_REQUIRED`) is structurally untestable and untested** while the binary is `0.x` | specify-cli | M | Wire-contract gap that will bite at 1.0 |
| **5** | **~650–750 lines of copy-pasted DiagnosticReport envelope** across the 7 framework wasi-tools | specify-cli | M | One shared crate, 7 thinner `main.rs` |

**Explicitly de-prioritize (again):** new skills, RM-11+ roadmap, golden-comparing LLM prose, collapsing test binaries (rejected in DECISIONS), `split_frontmatter` consolidation (documented as deliberate).

---

## Health snapshot (what's already good — don't spend budget here)

- **`make lint` is green** (0 findings) and the full CLI test suite is 1706/1707 passing — the sole failure is the P0 name-length gate below, not a behavioural regression.
- **`code-typescript` → `typescript` rename is complete** — zero stragglers across both repos and wasi-tools.
- **All 30 symlinks in `specify` resolve**, including the agent-teams and spec-to-test-mapping overlay chains created last round.
- **Acceptance catalog math is consistent** — 23 scenarios = 14 manual `.md` files + 9 automated catalog entries; README tables match the tree. The uncommitted `acceptance/README.md` / `scenarios/README.md` edits (status `documentation-one-slice: passed`, table formatting) are safe to commit.
- **Framework wasm `dist/` hygiene holds** — all 7 blobs + `.sha256` sidecars consistent, enforced by `dist_digests_pinned`.
- **Production `unwrap`/`expect` posture is clean** in recently-touched paths; zero substantive TODO/FIXME in `crates/` + `src/`; lint-suppression debt unchanged (all T0/T2, tracked).
- **`scripts/specify.rs` is sound** — overlay precedence, hard parse errors, incremental path-mode builds, `exec` handoff on unix. No structural issues found.

---

## Part 0 — Live regressions (fix first)

### 0.1 `rust_quality::no_long_test_fn_names` fails on tip *(P0, XS)*

`cargo make check` / `cargo make ci` fail right now:

```text
[rust.test-fn-name-too-long] test fn `valid_framework_toml_git_only_passes_schema` is 43 chars
  (crates/standards/tests/lint_hint/schema.rs:182)
```

Introduced by `929a393c simplify Specify.toml`. Everything else passes (1706/1707; 3 skips are the `#[ignore]`d networked GitHub smokes). **Action:** rename to ≤ 40 chars, e.g. `framework_toml_git_only_passes_schema` (37).

### 0.2 Tracked 37.7 MB binary at `.bin/bin/specify` *(P0, S — `specify` repo)*

`git ls-files .bin/` → `.bin/.crates.toml`, `.bin/.crates2.json`, `.bin/bin/specify` (37,723,536 bytes). `.gitignore` covers `.cli/` only — `.bin/` looks like an earlier version-binding install-root iteration that got committed. Every doc says "no published binary is downloaded / committed" (`README.md:79`, `docs/contributing/checks.md:48`, CORE-055). **Action:** `git rm -r .bin/`, add `.bin/` to `.gitignore`. (History rewrite to purge the blob is optional; at minimum stop carrying it forward.)

### 0.3 Committed `Specify.toml` pins a local path *(P0–P1, S — `specify` repo)*

`Specify.toml:4` ships `cli = { path = "../specify-cli" }`. The repo's own policy (CORE-055 rule body, `docs/contributing/checks.md:72-79`) says the **committed** `cli` must be fetchable (`version`/`git`), with `path` reserved for the gitignored `Specify.local.toml` overlay — and `docs/contributing/index.md:27` promises markdown-only contributors need no sibling checkout. Today a clean clone's `make lint` dies unless `../specify-cli` exists. **Action:** commit a fetchable pin (e.g. `{ git = "https://github.com/augentic/specify-cli", branch = "main" }` or a version), and move the `path` form into your local overlay.

---

## Part A — Test improvements (unit + integration)

The area you flagged. Ordered by ROI.

### A1. Put `wasi-tools/` under CI *(P1, M)*

The sibling workspace (9 crates, ~11.8k LOC Rust, ~150 `#[test]` fns — vectis alone has 119) is invisible to `cargo make ci`: it's not a workspace member, `ci.yaml` never enters it, and `release.yaml` builds only contract+vectis wasm. Clippy (`-D warnings`, pedantic configured in its `Cargo.toml`) is likewise never enforced. A regression in `scenarios`/`skill-body`/`rules`/etc. ships silently via stale `dist/*.wasm` until someone manually runs `cargo make framework-wasm`. **Action:** add a CI job — `cd wasi-tools && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`. Optionally chain a `framework-wasm` rebuild + the host `dist_digests_pinned` check to prove blobs match source.

### A2. Exit code 4 is unreachable and untested *(P1, M)*

`Exit::from` unit-maps all variants (`tests/cli/errors.rs`), and exit 3 has a real integration test (`tests/cli/base.rs`, `specify_version: 99.0.0`). But exit 4 (`ProjectNeedsMigration`) compares **majors** (`config.rs:352-356`) and the binary is `0.2.0` → major 0 → never fires; `ProjectConfig::load`'s migration-refusal path has zero tests of any kind. This is the wire-contract corner that will matter exactly once — at the 1.0 cut — when nobody is looking. **Action:** make the current version injectable for tests (test-only constructor or `#[cfg(test)]` hook), add a unit test for `load` raising `ProjectNeedsMigration`, and pre-stage the exit-4 integration mirror of the exit-3 test.

### A3. Decoupling (#62) deleted offline init coverage without replacement *(P1, S)*

`resolves_from_framework_root_env_fallback` (adapter resolve via env) and `init_shorthand_via_framework_root` (offline `specify init omnia@v1`) were removed with the `$SPECIFY_FRAMEWORK_ROOT` fallback. The only remaining shorthand-resolution tests are `#[ignore]`d network smokes. **Action:** add one offline integration test that `specify init` resolves an adapter from a local path/vendored fixture (e.g. copy the omnia fixture tree, init against `./adapters/targets/omnia`), so the project-local resolution path the decoupling left behind is actually proven.

### A4. Framework-schema (`Specify.toml`) binding: crate-tested, never end-to-end *(P1–P2, M)*

`FRAMEWORK_JSON_SCHEMA` has embed-parity tests and 3 hint-evaluator cases (`lint_hint/schema.rs:152-239`), but: (a) no test drives an invalid `Specify.toml` through the **binary** (`specify lint framework` → exit 2 + structured finding), and (b) only 2 of the 6 `oneOf` arms (`version`, bare `git`) are exercised — `path`, `rev`, `branch`, `tag`, and the root `additionalProperties: false` rejection are untested. **Action:** finish the matrix at crate level (cheap), add one binary-level case.

### A5. `specify agents` has no CLI integration binary *(P2, M)*

The `specify-agents` crate is well unit-tested (46 tests after last round), and `init_writes_agents_fences_and_lock` covers the init path — but the `agents assemble` / `agents generate` verbs themselves have only 2 handler unit tests and no `tests/agents.rs` hub (JSON envelope, error paths, fence-preservation on re-run). Every other verb family has one. **Action:** add the hub with ~4–6 tests.

### A6. Bootstrap verbs: happy-path only *(P2, S)*

`tests/bootstrap/migrate.rs` (2 tests) and `upgrade.rs` never exercise failure: unknown `--from`, missing project, consent refusal exit code, `CliTooOld` under the `load_for_migration` carve-out. Cheap to add; these are the verbs operators run under stress.

### A7. Small test-debt items *(P2–P3, S each)*

- **`tests/README.md` golden table is stale** — still lists `tests/fixtures/rules-export/` (deleted in #61).
- **Thin integration binaries:** `cache` (1 test), `slice/drop` (1), `source/resolve` (2), `plan/end_to_end` (2). One or two negative cases each.
- **wasi-tools per-tool gaps:** `skill-body`'s `check_step_body_duplicates` (CORE-046) has no test; `prose`'s embedded-schema description-cap drift branch untested; vectis `scaffold` subcommand has unit tests but no CLI tests.
- **Embedded-schema parity (wasi-tools):** `marketplace` and `prose` embed copies of `schemas/authoring/*.schema.json` that byte-match today but have no drift test (the host parity test covers host constants only). Add per-crate `embedded_matches_canonical` tests.
- Flakiness posture is good: no sleeps, env mutation confined to nextest-isolated processes, git driven through `GIT_ENV`. Nothing to fix.

### A8. Don't chase

- Golden-comparing LLM prose; collapsing test binaries (rejected, measured 7.3%); a shared cross-crate `copy_dir` test-support crate (3 copies of ~12 lines is blessed by docs — revisit only at a 4th copy).

---

## Part B — Code quality (specify-cli host workspace)

### B1. Doc-path sweep after the bin/decoupling commits *(P1, S — batch in one PR)*

Six accuracy drifts, all verified:

| Stale | Actual | Where |
| ----- | ------ | ----- |
| `src/runtime/output.rs` | `src/output.rs` | 8 refs: AGENTS.md:46, DECISIONS.md ×2, DIAGNOSTICS.md ×2, handler-shape.md ×3-ish, coding-standards.md ×2, `crates/error/src/error.rs:148` |
| `src/bin/specify.rs` | `src/main.rs` | `docs/standards/architecture.md:7` |
| `rust-version = "1.93"` | `1.95` | `architecture.md:30` vs `Cargo.toml:63` |
| Crate graph omits `specify-agents` | exists | `architecture.md:9-19` (AGENTS.md has it) |
| AGENTS.md schema-constant list | missing `DECISION_JSON_SCHEMA`, `SYNTHESIS_JSON_SCHEMA`, `FRAMEWORK_JSON_SCHEMA` | AGENTS.md:35 vs `crates/schema/src/constants.rs` |
| "framework checkout" comment | post-decoupling it's the Specify.toml repo root | `crates/standards/src/lint/model.rs:136` |

### B2. Triplicate `copy_dir_recursive` with divergent symlink semantics *(P2, M)*

Three independent implementations: `slice/actions/io.rs:55-70` (explicit symlink copy), `init/cache.rs:276-291` (dereferences via `is_dir()`), `registry/workspace/sync.rs:358-384` (no symlink handling, per-op `Error::Diag`). The divergence is the bug surface — a workspace-sync tree containing a symlink behaves differently from a slice-action copy. **Action:** one `workflow`-internal `copy_tree` with an explicit symlink policy knob + a small symlink test matrix.

### B3. `workspace/sync.rs` invents one-off `Error::Diag` codes for plain filesystem ops *(P2, M)*

~6 `map_err(|e| Error::Diag { code: "workspace-contracts-remove-failed", … })` sites for `remove_dir_all`/`copy`/`read_dir`, when `Error::Filesystem { op, path, source }` exists for exactly this (and `style.md` budgets variants by recovery). **Action:** map filesystem ops to `Error::Filesystem`; keep `Diag` for semantic failures. Update any tests pinning the old discriminants. This is also the model for the broader `Diag` burn-down (hot clusters: `registry/validate.rs` ~16 sites, `plugins.rs` ~9) — don't sweep it, just stop the bleeding when touching those files.

### B4. Wasm artifact version constants *(P2, S)*

`framework_tools.rs` hardcodes `*-0.1.0.wasm` in 7 `include_bytes!` paths; `Makefile.toml`'s `framework-wasm` task hardcodes `0.1.0` again; the wasi-tools crates are at `0.4.0`; contract's dist is `0.2.0`. Bumping anything requires N coordinated string edits with only the drift test as a net. **Action:** single `FRAMEWORK_WASM_VERSION` const consumed by both (or derive from `CARGO_PKG_VERSION` via build script), and document the intentional artifact-vs-crate version split if you keep it.

### B5. Small items *(P3)*

- `crates/standards/src/lint/index/agent_teams.rs` hand-rolls SHA-256 with `sha2` + manual hex while `specify-standards` already depends on `specify-digest::sha256_hex`. Swap and drop the direct `sha2` dep if unused elsewhere. (S)
- `proc-macro-error2 v2.0.1` future-incompat warning (via `wasm-pkg-client` → `oci-spec` → `getset` in `specify-tool`). Track upstream; check `cargo update` paths when convenient. (S, informational)
- Module splits — `tool/manifest.rs` (612 L, tests from ~346) and `diagnostics/diagnostic.rs` (611 L, tests from ~546) could move tests to sibling `tests.rs` per coding-standards layout; `plan/lifecycle.rs` (430 L) mixes validate/transition/next/archive handlers. Opportunistic only. (M each, informational)
- `framework/builder.rs` empty `CORE_ID_TABLE` is intentional scaffolding; leave until rust-quality predicates go declarative.

---

## Part C — wasi-tools workspace

### C1. CI job *(P1, M)* — see A1; the same job covers clippy enforcement (consistent pedantic config exists in `wasi-tools/Cargo.toml` but is never run anywhere).

### C2. Extract a `framework-wire` shared crate *(P2, M)*

Seven `main.rs` files re-implement the identical stdout envelope: `Report`/`Summary`/`Finding` DTOs, `PLACEHOLDER_FINGERPRINT`, `print_report`, `from_findings`, plus `parsed_config` (5×) and `requested_rule` (5×). ~650–750 of the 1,481 total `main.rs` lines are near-copies (e.g. `scenarios/src/main.rs:33-173` ≈ `agent-teams/src/main.rs:29-175`). A wasi-tools-internal shared crate (deps: `serde`/`serde_json` only) does **not** violate the carve-out — that rule constrains host-crate imports. **Action:** extract; each tool keeps only rule dispatch + `guidance()`.

### C3. Contract dist hygiene *(P2, S)*

`contract/dist/contract-0.2.0.wasm` (1.08 MB, mtime 2026-05-27) has **no** `.sha256` sidecar and **no** drift test — unlike all 7 framework tools — yet host tests load it directly (`tests/tool/contract.rs:11`). `cargo make contract-wasm` builds to `target/`, never refreshing `dist/`. **Action:** either add sidecar + drift test mirroring `dist_digests_pinned`, or stop checking the blob in and make `contract-wasm` a test precondition (it already is for vectis).

### C4. Vectis embedded schemas — document authority *(P3, S)*

`vectis/embedded/{tokens,assets,composition}.schema.json` have no canonical copies under `schemas/` (tool-owned per its DECISIONS appendix). Fine — but one line in `wasi-tools/vectis/README` or AGENTS.md saying "these are authoritative here, not copies" prevents a future false-drift hunt.

---

## Part D — `specify` repo (docs / adapters / acceptance)

D0 items (tracked binary, committed path pin) are in Part 0. The rest:

### D1. Reconcile the CI story *(P1, M)*

Reality (`.github/workflows/ci.yaml`): stable toolchain, sibling `specify-cli` checkout (branch-matching with `main` fallback), `cargo run --manifest-path specify-cli/Cargo.toml --bin specify -- lint framework --framework-root .`, plus a spec-runtime symlink check. Docs claim: `AGENTS.md:110` "this repo's CI runs only `make lint`"; `docs/contributing/checks.md:91-93` "CI runs the same resolver (`make lint` → nightly cargo-script) … caches the gitignored `.cli` install root". Neither is true. **Action:** rewrite both passages to describe local (`make lint`, nightly `-Zscript`, `Specify.local.toml`) vs CI (decoupled stable checkout) as two intentional paths.

### D2. Acceptance-doc accuracy *(P1–P2, S–M)*

- **Wrong CLI test name** in the automated-coverage matrix: `acceptance/scenarios/README.md:69` cites `synthesize_resolves_same_authority_conflict`; the real fn is `synthesize_same_authority_conflict` (`tests/slice/synthesize.rs:748`). (S)
- **Stale "lifecycle pack" prose**: `docs/contributing/acceptance.md:89-96` still describes the pre-#153 pack and lists now-automated entries (`combined-evidence`, `extract-failure`, …) as manual-sweep items. Rewrite against the current catalog groups. (M)
- **`make install-cli` docs overstate `.cli/bin/specify`**: with a `path` pin, `--install` prints `{checkout}/target/release/specify`, not `.cli/bin/specify` (`scripts/specify.rs:28-31`). Four files repeat the wrong claim (`acceptance/README.md:58`, `shared/setup.md:9`, `docs/contributing/acceptance.md:33-35`). Also a candidate for deduplicating the repeated install/PATH block into one shared snippet. (S)
- **Backtick-wrapped links don't render**: `acceptance/README.md` wraps markdown links in backticks throughout its tables (`` `[pure-intent](scenarios/pure-intent.md)` `` at lines 17, 32-45, 58, 65-68, 72) — these render as literal code, not links. Looks like an auto-formatter artifact; drop the backticks or move them inside the link text. (S)
- **Fixture stale id**: `acceptance/fixtures/sources/typescript/README.md:3` references "scenario `4`" — numbered ids were removed in #153; it's `code-multi-slice`. (S)
- **Assertion naming**: `code-multi-slice.md` still uses `sources-legacy-only` / `<legacy-key>` vocabulary post-rename; rename to source-key wording. (S)

### D3. RFC / reference hygiene *(P2–P3, S each)*

- The acceptance shape-traces RFC (`rfcs/future/`, line 127) claims a delivered `$SPECIFY_ROOT` adapter fallback that never shipped (and was explicitly removed by decoupling); correct the "delivered" bullet.
- The acceptance RFC (`rfcs/`, line 58) and `scripts/snapshot.sh:49` cite the deleted `docs/explanation/decision-log.md`; repoint or remove.
- `CORE-031` (recorded-trace rule) describes the removed `acceptance/recorded/` tree; add a "deferred — no tree today" banner or fold into the shape-traces RFC.
- `AGENTS.md:125` Bootstrap DECISIONS link is missing its `#bootstrap-upgrade-and-migration-lifecycle` fragment.
- `docs/contributing/skills-test-coverage.md:42` references a nonexistent `acceptance/**/*.json` fixture shape.
- `.cursor/rules/project.mdc` layout block omits `acceptance/`, `Specify.toml`, `Makefile`, `rfcs/` — the surfaces this round's changes all live in.
- Vectis `test-spec-mapping.md` vs Omnia `spec-to-test-mapping.md` filename asymmetry over the shared base — optional rename for grep-ability.

---

## Prioritized backlog

| P | Item | Repo | Section | Effort |
| - | ---- | ---- | ------- | ------ |
| P0 | Rename 43-char test fn; make `cargo make check` green | specify-cli | 0.1 | XS |
| P0 | Remove tracked `.bin/` binary + gitignore it | specify | 0.2 | S |
| P0–1 | Fetchable committed `Specify.toml` pin (path → local overlay) | specify | 0.3 | S |
| P1 | wasi-tools CI job (clippy + tests) | specify-cli | A1/C1 | M |
| P1 | Reconcile CI docs with `ci.yaml` | specify | D1 | M |
| P1 | Exit-4 testability + `ProjectConfig::load` migration test | specify-cli | A2 | M |
| P1 | Offline init/adapter-resolution integration test | specify-cli | A3 | S |
| P1 | Doc-path sweep (output.rs, main.rs, 1.95, agents crate, schema list) | specify-cli | B1 | S |
| P1–2 | Acceptance-doc accuracy batch (test name, lifecycle prose, install-cli path, backtick links) | specify | D2 | S–M |
| P2 | Framework-schema matrix + one binary e2e | specify-cli | A4 | M |
| P2 | `framework-wire` shared crate for 7 wasi-tools | specify-cli | C2 | M |
| P2 | Contract dist sidecar + drift test | specify-cli | C3 | S |
| P2 | `copy_dir_recursive` unification (symlink policy) | specify-cli | B2 | M |
| P2 | `workspace/sync.rs` → `Error::Filesystem` | specify-cli | B3 | M |
| P2 | Wasm version const; `specify agents` test hub; bootstrap negative paths | specify-cli | B4/A5/A6 | S–M |
| P3 | RFC/reference hygiene batch; sha2→digest dedup; schema-parity tests; small test-debt items | both | D3/B5/A7 | S |

---

## Post-mortem

Applied-finding calibration log — one line per executed finding: predicted vs actual ΔLOC, whether the done-when assertion flipped, regressions. Validate with `cargo make check` (specify-cli) / `make lint` (specify).

**Calibration priors (carried from the 2026-06-07 execution round):**

- Helper-extraction / dedupe findings systematically **over-predict** LOC reduction — the shared helper adds its body back.
- Deleting a production method often deletes its **sole test caller** too → over-delivers vs body-only prediction.
- Doc-comment fold-ups and orphaned blank separators expose **extra** deletions.
- Struct-update / skeleton refactors backfire when most fields vary — revert honestly if a finding fails its own LOC premise.
- "Dead rule" class: a lint rule can be silently dropped by profile filtering — when touching rule plumbing, empirically prove the rule fires (scratch violation → finding) before and after.

| Item | Predicted | Actual ΔLOC | Done-when flipped? | Regressions | Notes |
| ---- | --------- | ----------- | ------------------ | ----------- | ----- |
| 0.3 fetchable pin | S | +8/−4 (Specify.toml) | yes — committed `cli` is `{ git, branch = "main" }`; `path` lives in gitignored `Specify.local.toml` | none — `--resolved-ref` still resolves the overlay locally | docs already described the target posture; only the file flipped |
| A1/C1 wasi-tools CI | M | +30 ci.yaml | yes — clippy `-D warnings` + `cargo test --workspace` job added | fixed 6 latent clippy errors in `vectis/build.rs` + 1 in `tests/engine/assets.rs` the job would have caught | dist↔source parity left to host digest drift tests, not CI rebuild (avoids non-reproducible-wasm flakes) |
| A2 exit-4 | M | +70 | yes — `load_with_current` injectable; `load_refuses_migration_owed_pin` unit test; pre-staged exit-4 integration mirror whose expectation flips at the 1.0 cut | none | integration arm asserts exit-2 (gate dormant) pre-1.0, exit-4 + `project-needs-migration` envelope post-1.0, no test edit needed |
| C2 framework-wire | M (−650–750) | −830 in 7 mains, +~280 new crate ⇒ net ≈ −550 | yes — 7 `main.rs` keep only rule ids, dispatch, guidance | none — wasi-tools clippy+tests green, blobs rebuilt, host lint suite green | calibration prior held: shared crate adds its body back |
| C3 contract dist | S | +20 | yes — `.sha256` sidecar + `dist_digest_pinned`; `contract-wasm` refreshes dist | none — rebuilt blob passes behaviour tests | dist blob refreshed from current source in the same change |
| B1 doc sweep | S | ~12 lines across 5 files | yes — `src/main.rs`, `rust-version 1.95`, `specify-agents` in crate graph, schema-constant list (+4 ids), `emit` home, `model.rs` comment | none | `src/runtime/output.rs` row had self-resolved (module split since baseline); only the `emit` phrasing needed fixing |
| D1 CI story | M | ~10 lines (AGENTS.md + checks.md) | yes — local (`make lint`, nightly resolver) vs CI (stable sibling checkout) documented as two intentional paths | none — `make lint` green | |
| R6 doc-comments | S | 2 sites | yes — `plan/cli.rs` "plan lock *" dropped; `example.rs` "cache fingerprinting" → replay-verification anchor | none | stale `contract-wasm` task comment (target/ vs dist/) fixed in the same pass |
| R6 blob digests | S | 0 | already landed pre-round — sidecars + `dist_digests_pinned` exist; `sha256: None` documented as deliberate (host compiles staged bytes; sidecar+test is the trust anchor) | none | no-op verified, not re-done |

---

## Quick reference commands

```bash
# specify repo
make lint                       # framework rule enforcement (nightly cargo-script)
make install-cli                # build pinned CLI + symlink into ~/.local/bin

# specify-cli repo
cargo make ci                   # full gate (fmt + lint + test + docs + vet + outdated + deny)
cargo make check                # pre-commit subset
cargo test --test rust_quality  # naming / archaeology / bare-allow gates
cargo nextest run --test lint   # consumer + framework lint binaries
REGENERATE_GOLDENS=1 cargo nextest run --test <binary>   # review diff before commit

# wasi-tools (dedicated CI job since A1; same commands locally)
cd wasi-tools && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```
