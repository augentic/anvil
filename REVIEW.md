# Specify + Specify-CLI — improve & optimize review

**Mode:** subtraction, determinism, test depth, and operator proof — not new features (native-hint work, RM-12, synthesis transcript replay, etc. stay deferred).

**Baseline (re-verified 2026-06-04):** `specify` @ `e502e450`, `specify-cli` @ `07e9d6e7`.

**Prior pass:** The subtraction items in the previous `REVIEW.md` post-mortem (CORE-051 removal, `merge-runbook.md` deletion, `from_evidence_yaml` visibility, drop skill `argument-hint`) are **landed** — do not re-open them.

---

## Where to focus (ranked)

| Rank | Focus | Why (optimize lens) |
| --- | --- | --- |
| **1** | **RM-05 manual acceptance** — Wave 0 `pure-intent` first | Release gate is **0/24** lifecycle scenarios `passed`; deterministic CLI proof is green. No amount of repo lint fixes substitutes for proving N=1 on a live `specify` binary. Catalog: [`acceptance/lifecycle/README.md`](acceptance/lifecycle/README.md). |
| **2** | **Executable harness for `acceptance/examples/`** | ~132 fixture files document inputs/expected shapes but **zero** Rust tests reference them (`rg acceptance/examples` in `specify-cli` → empty). Highest ROI for automated test depth without LLM bytes. |
| **3** | **Consumer `specify lint` integration depth** | Framework lint is CI-native and fast (~1.4s release on full `specify` tree locally). Consumer lint (`specify lint run`) is thinly covered (`tests/lint_run.rs` + a few `crates/standards/tests/lint_hint_*`); engineering-standards regressions are easier to miss. |
| **4** | **CORE parity coverage gaps** | `crates/standards/tests/core_parity.rs` pins **16** ids; **36** `CORE-*` rule files still run via `authoring-predicate` bridge without parity tests. Incremental parity reduces bridge retirement risk. |
| **5** | **Doc / Makefile hygiene** | `make acceptance` is documented but **not implemented** in [`Makefile`](Makefile). Performance notes cite **~247s** `make lint` while release runs measure **~1.2–1.4s** — stale guidance mis-prioritizes perf work. |
| **6** | **`docs/quality-debt.md` burn-down** | `framework.rs` module `#![allow(pedantic, …)]` (T2) and `rust.archaeology-in-doc-comment` (202 residual, burn-down-only). Low urgency while CI is green. |
| **7** | **Native-hint migration (optional)** | Steady state is correct; migrating `CORE-010..052` off `authoring-predicate` is maintainability, not correctness. Do one id at a time per [checks parity contract](docs/contributing/checks.md#parity-contract-for-predicate-retirement). |

**Explicitly deprioritize in this mode:** new roadmap items (RM-11+, native-hint work), client/AsyncAPI/captures *feature* scenarios, and re-litigating retired legacy verbs.

---

## Health snapshot (what is already strong)

| Surface | Status | Notes |
| --- | --- | --- |
| `make lint` (`specify lint framework`) | **0 findings** | Release binary; ~1.4s wall on full `specify` tree (2026-06-04 local). |
| `cargo clippy -D warnings` | Pass | Per prior pass; re-run before large refactors. |
| Production `unwrap`/`expect` | **0** in `crates/` + `src/` | All confined to `#[cfg(test)]`. |
| `tests/fan_in_fan_out.rs` | Shipped | RM-05 deterministic proof: survey → propose → extract → synthesize → build → merge + `depends-on` + re-projection determinism. |
| Test count | **~1773** nextest cases, **~1699** `#[test]` fns | `specify-cli`; integration-first layout per [`docs/standards/testing.md`](../specify-cli/docs/standards/testing.md). |
| Spec-runtime symlinks | Enforced in CI | [`specify/.github/workflows/ci.yaml`](.github/workflows/ci.yaml) rejects materialised copies. |
| Declarative lint architecture | Steady state | Declarative pass + CORE-009 bridge only; full imperative `Check` batch retired from hot path. |

---

## Test improvement guide

### Two acceptance surfaces (do not conflate)

1. **Automated (CLI):** `cargo make test` in `specify-cli`, especially [`tests/fan_in_fan_out.rs`](../specify-cli/tests/fan_in_fan_out.rs). Proves envelopes, ordering, determinism — **not** real target codegen quality.
2. **Manual (operator + agent):** 24 lifecycle scenarios under [`acceptance/lifecycle/`](acceptance/lifecycle/) — all **`pending`**. Proves `/spec:*` rhythm and (when run fully) generated-output correctness.

See [`docs/contributing/acceptance.md`](docs/contributing/acceptance.md) and [`docs/contributing/skills-test-coverage.md`](docs/contributing/skills-test-coverage.md).

### High-ROI automated additions

#### A. Wire `acceptance/examples/` into `specify-cli` tests

The tree is intentionally aligned for chaining (e.g. screenshots source fixture ↔ vectis `task-list` target fixture). Today it is **documentation-only**.

| Example area | Suggested test shape | CLI verbs exercised |
| --- | --- | --- |
| [`acceptance/examples/sources/intent/`](acceptance/examples/sources/intent/) | Copy fixture → `specify source survey` / `extract` → compare `discovery.md` / `evidence/*.yaml` to `expected-*` (normalise paths) | `source survey`, `source extract` |
| [`acceptance/examples/sources/documentation/`](acceptance/examples/sources/documentation/) | Same pattern | survey + extract |
| [`acceptance/examples/sources/code-typescript/`](acceptance/examples/sources/code-typescript/) | Same; bound to tempdir TypeScript tree | survey + extract |
| [`acceptance/examples/sources/captures/`](acceptance/examples/sources/captures/) | Assert `kind: example` + `replay-digest` in evidence | extract |
| [`acceptance/examples/sources/screenshots/`](acceptance/examples/sources/screenshots/) | Evidence YAML shape + discovery lead blocks | survey + extract |
| [`acceptance/examples/skills/refine/*/`](acceptance/examples/skills/refine/) | Pre-seed slice tree + evidence → `specify slice synthesize` (if inputs are kernel-complete) **or** structural diff on `expected/` artifacts after hand-staged synthesis | `slice synthesize`, `slice validate` |
| [`acceptance/examples/targets/omnia/expected/crate/`](acceptance/examples/targets/omnia/expected/crate/) | Static file presence + `cargo check` in fixture crate (no LLM) | optional `slice build` prepare-only |
| [`acceptance/examples/targets/vectis/task-list/`](acceptance/examples/targets/vectis/task-list/) | `composition.yaml` schema + key paths vs `expected/composition.yaml` | `specify tool run vectis -- validate composition` |

**Pattern to copy:** [`tests/fan_in_fan_out.rs`](../specify-cli/tests/fan_in_fan_out.rs) (tempdir + `specify_cmd` + structural JSON asserts) and golden discipline in [`tests/README.md`](../specify-cli/tests/README.md).

**Defer (per acceptance doc):** byte-for-byte `/spec:refine` or `/spec:build` skill body replay — needs transcript or structured-trace RFC ([`acceptance.md` § Synthesis byte-replay](docs/contributing/acceptance.md#synthesis-byte-replay-deferred)).

#### B. Add `make acceptance` (doc–code alignment)

[`docs/contributing/acceptance.md`](docs/contributing/acceptance.md) documents:

```bash
make acceptance   # make lint + fan_in_fan_out
```

[`Makefile`](Makefile) has **no** `acceptance` target — only `lint`, `use-local-plugins`, `use-team-plugins`. Implementing this removes recurring operator/agent confusion and gives a one-command pre-release smoke.

Suggested recipe:

```makefile
acceptance: lint
	cargo test --release --manifest-path $(SPECIFY_MANIFEST) --test fan_in_fan_out
	@echo "Manual sweep: docs/contributing/acceptance.md"
```

Keep it **out of** `specify` CI if the policy is unchanged; wire only in docs/Makefile until you want cross-repo CI cost.

#### C. Expand consumer `specify lint` tests

| Gap | Today | Suggested |
| --- | --- | --- |
| Hint kinds | `lint_run.rs` exercises regex (`UNI-100` TODO) | Add one integration case each for `path-pattern`, `schema`, `tool` (mirror `crates/standards/tests/lint_hint_*.rs` at binary level) |
| `--dump-model` | Covered in `lint_run.rs` | Keep; extend with monorepo + `rules-root` edge cases from [`tests/rules_export.rs`](../specify-cli/tests/rules_export.rs) |
| Silence / ignore directives | `lint_ignore_directive_pass` (crate) | One `tests/lint_run.rs` case mirroring UNI-100 demotion pattern |
| Blocking vs review | Partial | Assert exit code `2` vs `0` with `deny_blocking_findings` on mixed severity fixtures |

#### D. CORE parity harness

[`crates/standards/tests/core_parity.rs`](../specify-cli/crates/standards/tests/core_parity.rs) modules today: `001–009`, `014`, `016`, `023`, `025`, `037`, `038`, `050`.

**Missing parity modules** (bridge-only, regression risk): e.g. `010`, `011–013`, `015`, `017–022`, `024`, `026–036`, `039–049`, `052` — prioritise rules that encode non-trivial imperative semantics (skill frontmatter, links, scenarios, brief size).

Workflow: one `core_parity/core_NNN.rs` per rule → synthetic fixture → imperative reference set == declarative finding set (existing contract in file header).

#### E. `rules_export` sibling checkout

[`tests/rules_export.rs`](../specify-cli/tests/rules_export.rs) **skips** when `../specify` is absent. CI for `specify` checks out both repos; local solo clones silently skip goldens.

**Optimize:** document in AGENTS/contributing that monorepo/sibling layout is required for full test parity; optionally `#[ignore]` with explicit message vs silent `SKIP` eprintln (harder to notice in nextest summary).

#### F. Lifecycle gaps with **CLI** tests but **no** scenario

| Skill / path | CLI test | Lifecycle scenario |
| --- | --- | --- |
| `/spec:drop` | [`tests/slice/drop.rs`](../specify-cli/tests/slice/drop.rs) | **gap** ([skills-test-coverage](docs/contributing/skills-test-coverage.md)) |
| `captures` adapter | journal / plan fixtures only | **gap** |
| Omnia / Vectis build output | reached in cross-repo scenario, **not asserted** | **partial** |

Automating drop via CLI is cheap; proving drop in a lifecycle scenario is manual-only today.

### Unit vs integration balance (CLI repo)

| Layer | Strength | Stretch |
| --- | --- | --- |
| **Integration (`tests/*.rs`)** | Broad command coverage: plan, slice, source, workspace, tool, migrate, journal | Parametrise over `tests/fixtures/` trees; reduce per-binary `copy_dir` duplication via `tests/common` only |
| **Crate unit tests** | Workflow goldens, merge, propose kernel, model parsers | `split_frontmatter` still duplicated (`rules/parse.rs` vs `model/decision.rs`) — unify only if you accept a small shared crate or move parser to `specify-model` |
| **WASI tools** | `contract` + `vectis` CLI tests; vectis engine tests | Carved out; `cargo make test` depends on **`vectis-wasm`** build — budget ~cold CI time when optimizing pipelines |
| **Framework checks** | Many `crates/standards/tests/check_*.rs` + lint indexers | Run `cargo nextest run -p specify-standards` when touching rules/hints |

### What not to chase (test anti-patterns for this mode)

- Golden-comparing LLM prose from `/spec:refine` / `/spec:build` (explicitly out of scope).
- Collapsing `tests/*.rs` into one binary (measured **7.3%** win, rejected in DECISIONS).
- Adding `make acceptance` to `specify` CI without agreeing cross-repo clone cost (today `specify` CI = lint + symlink check only).

---

## Subtraction & maintainability (code/docs)

### S1 — Refresh stale `make lint` performance docs

**Evidence.** Local release runs:

```text
/usr/bin/time make lint          → real ~1.2s  (specify repo)
cargo run --release … framework  → real ~1.4s  (--framework-root ../specify)
```

[`docs/contributing/checks.md`](docs/contributing/checks.md) and [`specify-cli/DIAGNOSTICS.md`](../specify-cli/DIAGNOSTICS.md) still cite **~247s** from the pre-migration lint era (2026-06-04). That number is misleading for current release builds and can send optimizers down the wrong path.

**Action.** Replace with measured release guidance + note that debug/unoptimized binaries are not representative; drop the obsolete 247s figure or label it historical.

**Done when.** `rg '247' docs/contributing/checks.md specify-cli/DIAGNOSTICS.md` → only historical context or empty.

---

### S2 — Implement `make acceptance` or fix docs

**Evidence.** `docs/contributing/acceptance.md` references `make acceptance`; root `Makefile` has no target.

**Action.** Add Makefile target (see §Test B) **or** change docs to the explicit two-command sequence only.

---

### S3 — `framework.rs` module allows (T2)

**Evidence.** [`specify-cli/docs/quality-debt.md`](../specify-cli/docs/quality-debt.md): `crates/standards/src/framework.rs` carries a broad `#![allow(pedantic, missing_docs, …)]` tied to dissolved `specify-authoring` posture.

**Action.** Burn down incrementally as predicates move to declarative-only paths (already complete for hot path); shrink allow list when touching `framework/check/*`.

**Net.** Maintainability; not blocking CI.

---

### S4 — `split_frontmatter` duplication

**Evidence.** Two implementations: `crates/standards/src/rules/parse.rs` and `crates/model/src/decision.rs` (prior review deferred due to crate graph).

**Action.** Only if you are already moving markdown helpers: expose one `specify-model` helper and delete the duplicate (~30–40 LOC).

**Risk.** Dependency-direction invariant — do not pull `specify-standards` into `specify-model`.

---

### S5 — Archaeology predicate (202 findings, burn-down-only)

**Evidence.** [`quality-debt.md`](../specify-cli/docs/quality-debt.md): `rust.archaeology-in-doc-comment` over-fires on legitimate `RFC-NN` / `Phase N` anchor vocabulary.

**Action options:** (a) narrow predicate markers to history phrases only, then gate; (b) leave as burn-down; (c) strip anchors from code comments into `DECISIONS.md` links (high churn, low value).

**Recommendation.** (b) unless you need a hard gate — promoting now would be perpetually red.

---

## CI & repo boundaries

| Repo | CI does | Does not |
| --- | --- | --- |
| **specify** | `lint framework` + spec-runtime symlink check; checks out matching `specify-cli` branch | `cargo make test` (by design) |
| **specify-cli** | Full `cargo make ci` via reusable workflow | Framework lint of `specify` unless you add a job |

**Optimize CI time:** `specify-cli` `cargo make test` builds `vectis-wasm` first — cache that artifact in CI; use `cargo nextest run --test <area>` locally when iterating.

**Cross-repo coupling:** Framework rules live in `specify/adapters/shared/rules/core/`; predicates run in `specify-cli`. Any rule change should run **both** `make lint` and `cargo nextest run -p specify-standards`.

---

## Skill / scenario coverage (manual debt map)

From [`docs/contributing/skills-test-coverage.md`](docs/contributing/skills-test-coverage.md) (unchanged audit):

| Status | Count | Examples |
| --- | --- | --- |
| ✓ | 7 | plan, refine, init, finalize, execute, contract OpenAPI/JSON Schema |
| partial | 5 | build, merge, omnia build brief, vectis build + merge briefs |
| gap | 5 | client SoW, contract AsyncAPI, captures adapter, wiretapper, drop (lifecycle) |

**Optimize-mode recommendation:** Do not author new skills — close gaps by (1) running existing lifecycle scenarios, (2) promoting `acceptance/examples/` into CLI tests (table above), (3) adding **one** focused contracts-style target test pack for omnia/vectis *only if* generated-output asserts are required before RM-05 sign-off.

---

## Suggested 4-week attention plan (improve-only)

1. **Week 1 — Gate proof:** Build `SPECIFY_BIN`, run `01-pure-intent`, file run-summary, halt/fix until green. Parallel: add `make acceptance` to `Makefile`.
2. **Week 2 — Fixture harness (sources):** One `tests/acceptance_sources.rs` (or per-adapter binaries) driving `acceptance/examples/sources/*` through `specify source survey/extract`.
3. **Week 3 — Fixture harness (refine/synthesize):** Structural asserts on `acceptance/examples/skills/refine/*/expected` where inputs are complete; `slice validate` on outputs.
4. **Week 4 — Lint depth:** Consumer `lint_run` hint-kind matrix + 3–5 CORE parity modules for highest-churn rules (`044`, `042`, `019`). Refresh perf docs (S1).

---

## Dropped / do not re-litigate

- CORE-051 `adapter.execution-agent` — deleted.
- `merge-runbook.md` orphan — deleted.
- `from_evidence_yaml` `pub(crate)` — done.
- Drop skill `argument-hint` — done.
- `ToolSource` serde impls, `validate_*_json` wrappers, `migrate`/`upgrade` emit helpers — still justified (prior review).
- Materialising `spec-runtime` copies — CI forbids.
- Feature roadmap (native-hint work, RM-11, RM-12, synthesis transcript golden) — out of scope for this review mode.

---

## Quick reference commands

```bash
# specify repo
make lint
# after adding target:
make acceptance

# specify-cli repo (full CI)
cargo make ci

# targeted
cargo nextest run --test fan_in_fan_out
cargo nextest run -p specify-standards
cargo test --test rust_quality
REGENERATE_GOLDENS=1 cargo nextest run --test e2e   # review diff before commit
```

---

*Generated for improve/optimize mode. Re-run `make lint`, `cargo make check`, and skim `acceptance/lifecycle/README.md` status before acting on stale scenario counts.*
