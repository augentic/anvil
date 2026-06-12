# Specify + Specify-CLI — improve & optimize review

**Mode:** improve / optimize / subtract — *not* new features.

**Baseline:** `specify` @ `09980416`, `specify-cli` @ `9fda29bc` (2026-06-12). Supersedes the 2026-06-10 review. Prior-round P0s re-verified as **resolved**: `Specify.toml` now carries a fetchable git pin with a gitignored `Specify.local.toml` overlay; no binary is tracked under `.bin/`; `wasi-tools` has a dedicated CI job; the seven framework checkers share `framework-wire`.

**Method:** four parallel deep-dives (CLI architecture, CLI test suite, wasi-tools workspace, specify docs repo). Headline findings re-verified by hand against the live tree (notably `crates/validate/src/run.rs` vs `registry.rs`).

---

## TL;DR — where to focus, ranked

| # | Finding | Repo | Effort | Payoff |
|---|---------|------|--------|--------|
| **1** | **Dead validation namespace: `composition` rules registered but never run** — `validate_slice` silently skips Vectis composition checks | specify-cli | S | Correctness bug + ~240 lines dead-or-alive decision |
| **2** | **Framework checkers as WASM is the biggest YAGNI** — 7 crates, 7 dist blobs, sidecars, drift tests, a parallel wire format, ~16 wasmtime runs per `make lint`, all labelled "interim B-2 posture" in DECISIONS.md | specify-cli | L | Deletes an entire build/release pipeline; faster lint |
| **3** | **Registry/workspace double test stack** — 97 crate-level tests re-testing what 32 CLI tests already cover | specify-cli | M | Largest test-budget win |
| **4** | **`specify-workflow` links Wasmtime via `specify-tool`** for manifest DTOs only — every workflow compile pays the wasmtime tree | specify-cli | M | Compile-time / CI iteration speed |
| **5** | **Triple agent-teams enforcement** (CORE-008 digest + CORE-011 presence + CORE-012 WASI tool) over a symlink chain that "can never drift" | specify | M | Simpler mental model; one less Road B tool |
| **6** | **Orphan plugin trees** — `plugins/omnia/`, `plugins/plan/`, `plugins/rt/` not in the marketplace, zero inbound links | specify | S | Removes a confusing second Omnia surface |
| **7** | **Doc canonicalization (RFC-44 R5)** — authority hierarchy restated in ~17 files, slice loop in ~65; CORE-058 digest-pins prose that could be a link | specify | M | Cuts agent context + drift surface |

---

## Part 1 — specify-cli architecture

### 1.1 Dead `composition` rules in `specify-validate` (verified) — **fix first**

`crates/validate/src/registry.rs` registers `"composition" => composition::COMPOSITION_RULES` (~240 lines in `registry/composition.rs`: valid-yaml, has-version, screens-or-delta, kebab slugs), and `artifact_for` even maps `"composition"` → `Artifact::Composition`. But the runner's canonical set never includes it:

```rust
// crates/validate/src/run.rs
const CANONICAL_ARTIFACTS: &[(&str, &str)] = &[
    ("proposal", "proposal.md"),
    ("specs", "specs/**/*.md"),
    ("design", "design.md"),
    ("tasks", "tasks.md"),
    ("contracts", "contracts/**/*.yaml"),
];
```

Only `cross.composition-maps-to-consistent` (in `registry/cross.rs`) actually executes. The unit tests assert `rules_for("composition")` is non-empty — false confidence. **Either** add `("composition", "composition.yaml")` to `CANONICAL_ARTIFACTS` (literal-but-optional semantics, like `contracts/`), **or** delete `composition.rs` and rely on the vectis tool's own `validate composition`. Given `specify tool run vectis -- validate composition` already owns deep composition validation, deletion may be the honest answer.

### 1.2 Framework checkers over WASM — biggest YAGNI (Effort L, Impact H)

`src/runtime/commands/lint/framework_tools.rs` documents itself as an "interim posture" with an explicit exit condition. The seven checkers (`scenarios`, `skill-body`, `agent-teams`, `links-registry`, `marketplace`, `prose`, `rules`):

- are first-party, embedded in the same binary they sandbox against — **no security win**;
- have no third-party extensibility path (`is_declared` never reads `tools.yaml`; RM-21 is "no pre-1.0 commitment");
- cost: 7 crates + 7 `dist/*.wasm` blobs + `.sha256` sidecars in git + `dist_digests_pinned` drift test + `cargo make framework-wasm` pipeline + duplicate `DiagnosticReport` wire (`framework-wire` hand-rolls the envelope with `PLACEHOLDER_FINGERPRINT`, hardcoded severity) + schema copies **without** byte-parity tests (`prose`'s embedded `skill.schema.json`, `marketplace`'s schema — drift is silent).

The dependency-direction win (`specify-standards` avoids wasmtime) is achievable with an in-process crate behind the same `ToolRunner` trait. Roadmap, effort-ordered:

1. **S** — add byte-parity tests for the embedded schema copies (`prose`, `marketplace`) mirroring `embedded_schemas_match_on_disk_sources`.
2. **S** — dedupe the copy-pasted `walk_files` / `relative_display` / `walk_markdown` helpers (~150–200 lines across 5 crates) into `framework-wire`.
3. **M** — stop redundant tree scans: `scenarios` runs **all** checks then filters per scoped rule, so the scenarios family alone does 5 full passes per lint; dedupe wasmtime invocations to one per tool, or dispatch per rule inside the tool.
4. **M** — merge 7 crates → 1 multi-command crate (one wasm, one sidecar) as an interim step.
5. **L** — execute the documented B-2 exit: in-process Rust impl of `ToolRunner`; delete the dist blobs, sidecars, and staging. Keep WASM only for `contract`/`vectis`, where the sandbox and independent release cadence are real.

### 1.3 Wasmtime in the workflow compile graph (Effort M, Impact H)

`crates/workflow/Cargo.toml` depends on `specify-tool` solely for manifest DTOs (`ProjectConfig.tools: Vec<Tool>`, init scaffolding). No workflow code runs wasmtime, but every `cargo check -p specify-workflow` — and its many test binaries — links the wasmtime tree. Extract a `specify-tool-manifest` leaf (serde DTOs + schema validation) consumed by both. While there, drop `clap` from `specify-workflow` (used only for `ValueEnum` in ~6 files; `strum` is already a dependency).

### 1.4 Boilerplate and vestigial substrate (Effort S–M each)

- **`crates/workflow/src/schema.rs` (~587 lines)** — ~12 near-identical `validate_*` wrappers differing only in schema constant and error code. One generic `validate_artifact<T: Serialize>(value, schema, code)` collapses them.
- **Vestigial `framework::Check` substrate** — already flagged T2 in `docs/quality-debt.md`; the `Check` trait, `Context`, `builder.rs` (with an empty `CORE_ID_TABLE`) survive only for the two repo-local rust-quality predicates. Move those to a dev-only home and delete the rest.
- **Dormant migration framework (~500 lines)** — `MigrationKind` is an empty enum; `resolve` always returns `[]`. Keep the trait + `apply_staged` as cheap insurance, but don't grow the DTO/test scaffolding until the first migrator is scheduled. Related: exit code 4 remains structurally unexercisable until then.
- **`Platform::Web`/`Desktop`** — documented placeholders, ~17 references, all tests/validation. Harmless but ensure platform reconciliation never inserts bootstrap slices for them.
- **Minor hygiene** — `lint/index/agent_teams.rs` imports `sha2` directly instead of `specify-digest`; `change/plan/core/status.rs` (~503 + 393 test lines) would benefit from extracting a pure projection kernel and property-testing it instead of scenario matrices.

### 1.5 Explicitly *not* worth touching

The 10-crate split is justified — every crate carries runtime code and the dependency-direction invariants are load-bearing (`specify-digest`'s 64 lines exist to keep wasmtime out of `specify-standards`). The 15-hint-kind lint engine is **not** YAGNI: 58 CORE rules use it, every kind has production users (thinnest is `set-eq` with one rule — not worth folding). The four diagnostic renderers, the envelope DTO discipline, `petgraph` for plan cycles, and the registry/workspace subsystem all earn their weight. Don't macro-ize the handler `*Body` pattern.

---

## Part 2 — Testing

Inventory: **~2,104 `#[test]` fns** total. ~427 binary integration (`tests/`), ~738 in `specify-workflow` (582 unit + 156 crate-level integration), 284 in `specify-standards`, 220 in wasi-tools (vectis alone: 142), remainder spread across schema/validate/tool/model/diagnostics. The repo is integration-first at the binary surface but **unit-heavy for workflow internals** — inverted from its own `docs/standards/testing.md` posture in places.

### Unit tests already covered by integration tests (fold/retire candidates)

| Area | Duplication | Action |
|------|-------------|--------|
| **Registry** | `crates/workflow/tests/registry.rs` (49 tests, direct `Registry::load`) vs `tests/registry.rs` (19, via CLI) — same URL classification, contract roles, shape invariants | Shrink crate tests to ~10 parse-edge cases unreachable via CLI |
| **Workspace** | `crates/workflow/tests/workspace.rs` (48, in-process sync/mirror/topology) vs `tests/workspace.rs` (13, fake forge through binary) | Same treatment; biggest runtime saving |
| **Plan propose** | `propose/tests.rs` (32 unit) + `tests/workflow/propose.rs` (21 integration) assert the same N=1 / fan-out envelopes | Keep kernel edge cases unit-side; drop integration tests that only re-assert envelope shape |
| **Plan next / validate / doctor** | `next/tests.rs` (9) vs `tests/workflow/next.rs` (7); `validate/tests.rs` (27) + `doctor/tests.rs` (17) vs `tests/workflow/validate.rs` | One CLI test per doctor rule, rest unit-only |
| **Schema validation** | Triple coverage: 33× `*_schema_compiles` + byte parity (`crates/schema/tests/schemas.rs`), valid/invalid golden matrix (`workflow/tests/goldens/schemas.rs`), `tests/plan/schema.rs` | Table-drive the 33 compile smokes into 1–2 tests; relocate `tests/plan/schema.rs` (its own comment admits it's pure-library code in the binary harness — a policy violation); never add a 4th path |

### Questionable value / CPU sinks

- **`rust_quality.rs`**: 3 tests, each a full repo scan → merge to one pass filtered by rule id (3× → 1× tree walk per CI run).
- **`tests/cli_contract.rs`**: spawns the binary 3× for `dump_json()` → share via `OnceLock`.
- **`tests/tool/run.rs`** (13 wasmtime tests): `ToolFixtures::new()` copies 3 fixture trees per test → shared lazy root.
- **Text/JSON duplicate pairs** (`plan_validate_clean_text`/`_json`, etc., ~6–10 tests): keep JSON only, per the policy preferring structural assertions.
- **Serde/Display round-trips**: `platform.rs` has 9 tests asserting what derives guarantee → keep one CSV-parse edge case.
- **Exact help-text assertions** (`help_exits_zero_and_prints_usage`, `help_lists_active_verbs`): brittle against clap wording; assert exit 0 / verb inventory via `contract dump` instead.
- **Plan-lock refusal tests**: 4 near-identical copy-pastes → parameterize.
- **Lint fixture duplication**: `tests/lint/framework.rs` and `framework_json.rs` carry an *identical* `scaffold_framework()` (acknowledged in comments) → extract `tests/lint/support.rs`.

### Keep as-is

Journal unit↔integration split (complementary, not duplicate); plan-lock OS-probe unit tests; vectis engine unit tests (142 — the WASI sandbox makes in-tool tests the right layer; resist adding more host-side `tool run` vectis coverage); `embedded_schemas_match_on_disk_sources`; scaffold golden hashes (catches template drift). Host-vs-WASI double coverage for `contract` is intentional and already thin (3 host tests).

**Structural recommendation:** document the three-layer pyramid (kernel unit / crate integration / binary integration) in `testing.md` with explicit criteria for when each layer is *required* — the registry/workspace duplication happened because that boundary is undocumented.

---

## Part 3 — wasi-tools (beyond §1.2)

- **`vectis/src/validate/engine/composition.rs` (733 lines, 26× `json!`)** — the domain is legitimately branchy, but it's one monolith mixing schema validation, structural identity, sibling auto-invoke, token/asset refs, and catalog cross-ref, built on manual `json!` instead of typed `Serialize` DTOs. It's also the highest-churn file right now. Split into `refs.rs` / `catalog.rs` / `structural_identity.rs` with typed findings. (M)
- **`requested_rule` substring dispatch** in `framework-wire` (`arg.contains(rule)`) — fragile; parse `CORE-\d+` from the basename or pass the rule id explicitly. (S)
- **`prose` CORE-024 check is `SKILL_SCHEMA_SOURCE.contains("512")`** — matches anywhere in the schema text; parse the JSON and read `properties.description.maxLength`. (S)
- **Missing-`PROJECT_DIR` → silent clean report** in all framework tools masks host misconfiguration. (S)
- **Scaffold template registry codegen** (231-line `build.rs` generating a checked-in 258-line `registry.rs`) — ceremony, but the orphan/manifest validation is real value; keep, though the checked-in generated file creates noisy diffs.

---

## Part 4 — specify (docs repo)

1. **Orphan plugin trees** — `plugins/omnia/` (15 files duplicating `adapters/targets/omnia/references/` concerns), `plugins/plan/`, `plugins/rt/`: not in `.cursor-plugin/marketplace.json`, zero inbound links. Delete. (S/H)
2. **Agent-teams over-enforcement** — CORE-008 (`content-digest-eq`) + CORE-011 (presence) + CORE-012 (Road B WASI tool, "deliberately stricter than CORE-008") all police a symlink chain whose own README says symlinks "can never drift", and CI already verifies the targets. Keep CORE-011 + the CI symlink check; retire CORE-008/012 and the `agent-teams` tool, forbidding regular-file overlays. (M/M)
3. **CORE-058 digest-pinned README cheat sheet** — an entire rule + manual dual-edit to keep two paragraphs byte-identical with AGENTS.md. Replace with links; delete the rule. (S/M)
4. **RFC-44 R5 canonicalization** — authority hierarchy in ~17 files, slice loop in ~65; `.cursor/rules/project.mdc` still restates Authority Hierarchy and Artifact Boundaries despite already deferring Vocabulary to AGENTS.md. Make every secondary site link-only. (M/H)
5. **CORE-025 hint fan-out** — 9 `path-pattern` + 8 `regex` hints to ban retired vocabulary; collapse into one prose-tool check or a single multi-pattern hint. (S)
6. **Dead/stub docs** — `docs/explanation/workspace-tiers.md`, `platform-repo.md` (retired, 0 inbound links outside future RFCs): delete. The mdBook redirect stubs (`reconciliation`, `components`, `augentic-specify-usage`) and 6 `slice-skills/` stubs could be generated or collapsed to one index. (S)
7. **RFC hygiene** — RFC-45 is accepted and implemented: record outcomes in `specify-cli/DECISIONS.md` and archive it. Consolidate the 7 `rfcs/future/*` into a single ideas section of `roadmap.md`. (S)
8. **Evals** — 13 scenarios, 4 long-pending with no run records; `client-sow-writer` and `capture-wiretapper` flagged as coverage gaps. Park the pending `full`-tier scenarios until they have an owner; don't grow the catalog. Mark `plugins/client` (1 skill, no scenario) experimental or demote from the marketplace. (M)
9. **Reference corpus weight** — omnia (~15k lines) + vectis (~8k) references ≈ 45% of the repo. Only act if agent context cost is hurting: tier into "active patterns" vs "archived examples". The vectis `team-protocol-{android,ios}` pair is ~90% identical — one template + platform appendix. (L, defer unless token budget bites)
10. **Lint bootstrap duality** — local `make lint` (nightly `-Zscript`, builds the CLI per run) vs CI (stable, sibling checkout, direct `cargo run`) remain two intentional-but-divergent paths. The fetchable-pin fix landed; the remaining cost is the per-run Cargo build and the nightly pin. Consider caching the built binary under a gitignored path and having `make lint` reuse it. (M)

### What *not* to cut

The `spec-runtime` symlink bundle, the `slice-skills` stub pattern, per-target brief differentiation, and the deterministic eval boundary (evals deliberately not CI-automated) are all correctly designed — spend no budget there.

---

## Suggested execution order

1. **Day 1:** fix or delete the `composition` validate namespace (§1.1); delete orphan plugin trees and dead doc stubs; archive RFC-45.
2. **Week 1:** test-suite diet — registry/workspace crate-test shrink, schema-smoke table-drive, text/JSON pair folds, `rust_quality` merge, shared fixtures (§2).
3. **Week 2:** `specify-tool-manifest` extraction + drop `clap` from workflow (§1.3); `workflow/schema.rs` dedupe; vestigial `Check` burn-down (§1.4).
4. **Week 2–3:** lint simplification in specify — agent-teams rule family, CORE-058, CORE-025 (§4.2–4.5); doc canonicalization R5.
5. **When ready:** execute the B-2 exit for framework checkers (§1.2 step 5) — the largest single subtraction in either repo.

---

**Caveats:** the §1.1 composition finding was verified directly against the source; most counts come from the four sub-audits and are spot-checked but not exhaustively re-derived. The working tree has uncommitted changes in the vectis/verify area — the §3 `composition.rs` refactor should wait until that lands.
