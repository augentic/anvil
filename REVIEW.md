# Specify + Specify-CLI — improve & optimize review

**Mode:** subtraction, determinism, test depth, and operator proof — not new features (native-hint work, RM-12, synthesis transcript replay, etc. stay deferred).

**Baseline (re-verified 2026-06-07):** `specify` @ `f6de3016`, `specify-cli` @ `c4dc3b40`.

---

## Where to focus (ranked)


| Rank  | Focus                                             | Why (optimize lens)                                                                                                                                                                                                                                                 |
| ----- | ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **1** | **Consumer `specify lint` integration depth**     | Framework lint is CI-native and fast (~1.4s release on full `specify` tree locally). Consumer lint (`specify lint project`) is thinly covered (`tests/lint_project.rs` + a few `crates/standards/tests/lint_hint/`); engineering-standards regressions are easier to miss. |
| **2** | `**docs/quality-debt.md` burn-down**              | `rust.archaeology-in-doc-comment` (202 residual, burn-down-only). Low urgency while CI is green.                                                                                                                                                                    |


**Explicitly deprioritize in this mode:** new roadmap items (RM-11+, native-hint work), client/AsyncAPI/captures *feature* scenarios, and re-litigating retired legacy verbs.

---

## Test improvement guide

### Two acceptance surfaces (do not conflate)

1. **Automated (CLI):** `cargo make test` in `specify-cli`, especially `[tests/fan_in_fan_out.rs](../specify-cli/tests/fan_in_fan_out.rs)`. Proves envelopes, ordering, determinism — **not** real target codegen quality.
2. **Manual (operator + agent):** 24 lifecycle scenarios under `[acceptance/scenarios/](acceptance/scenarios/)` — all `**pending`**. Proves `/spec:*` rhythm and (when run fully) generated-output correctness.

See `[docs/contributing/acceptance.md](docs/contributing/acceptance.md)` and `[docs/contributing/skills-test-coverage.md](docs/contributing/skills-test-coverage.md)`.

### High-ROI automated additions

#### A. The `make acceptance` target

The `[Makefile](Makefile)` `acceptance` target **exists** — it builds the release `specify`, runs `make lint`, runs a set of `specify-cli` integration tests (`fan_in_fan_out`, `source_extract`, `slice`, `plan_orchestrate`, `workspace`), and symlinks the build onto `PATH`.

Keep it **out of** `specify` CI unless you accept the cross-repo clone/build cost (today `specify` CI = lint + symlink check).

#### B. Expand consumer `specify lint` tests


| Gap                         | Today                                              | Suggested                                                                                                                           |
| --------------------------- | -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Hint kinds                  | `lint_project.rs` exercises regex (`UNI-100` TODO) | Add one integration case each for `path-pattern`, `schema`, `tool` (mirror `crates/standards/tests/lint_hint/*.rs` at binary level) |
| `--dump-model`              | Covered in `lint_project.rs`                       | Keep; extend with monorepo + `rules-root` edge cases from `[tests/rules_export.rs](../specify-cli/tests/rules_export.rs)`           |
| Silence / ignore directives | `lint_hint/ignore_directive_pass` (crate)          | One `tests/lint_project.rs` case mirroring UNI-100 demotion pattern                                                                 |
| Blocking vs review          | Partial                                            | Assert exit code `2` vs `0` with `deny_blocking_findings` on mixed severity fixtures                                                |


#### C. `rules_export` sibling checkout

`[tests/rules_export.rs](../specify-cli/tests/rules_export.rs)` **skips** when `../specify` is absent. CI for `specify` checks out both repos; local solo clones silently skip goldens.

**Optimize:** document in AGENTS/contributing that monorepo/sibling layout is required for full test parity; optionally `#[ignore]` with explicit message vs silent `SKIP` eprintln (harder to notice in nextest summary).

#### D. Lifecycle gaps with **CLI** tests but **no** scenario


| Skill / path                | CLI test                                                    | Lifecycle scenario                                                          |
| --------------------------- | ----------------------------------------------------------- | --------------------------------------------------------------------------- |
| `/spec:drop`                | `[tests/slice/drop.rs](../specify-cli/tests/slice/drop.rs)` | **gap** ([skills-test-coverage](docs/contributing/skills-test-coverage.md)) |
| `captures` adapter          | journal / plan fixtures only                                | **gap**                                                                     |
| Omnia / Vectis build output | reached in cross-repo scenario, **not asserted**            | **partial**                                                                 |


Automating drop via CLI is cheap; proving drop in a lifecycle scenario is manual-only today.

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

`[docs/contributing/checks.md](docs/contributing/checks.md)` and `[specify-cli/DIAGNOSTICS.md](../specify-cli/DIAGNOSTICS.md)` still cite **~247s** from the pre-migration lint era (2026-06-04). That number is misleading for current release builds and can send optimizers down the wrong path.

**Action.** Replace with measured release guidance + note that debug/unoptimized binaries are not representative; drop the obsolete 247s figure or label it historical.

**Done when.** `rg '247' docs/contributing/checks.md specify-cli/DIAGNOSTICS.md` → only historical context or empty.

---

### S2 — `split_frontmatter` duplication

**Evidence.** Two implementations: `crates/standards/src/rules/parse.rs` and `crates/model/src/decision.rs` (prior review deferred due to crate graph).

**Action.** Only if you are already moving markdown helpers: expose one `specify-model` helper and delete the duplicate (~30–40 LOC).

**Risk.** Dependency-direction invariant — do not pull `specify-standards` into `specify-model`.

---

### S3 — Archaeology predicate (202 findings, burn-down-only)

**Evidence.** `[quality-debt.md](../specify-cli/docs/quality-debt.md)`: `rust.archaeology-in-doc-comment` over-fires on legitimate `RFC-NN` / `Phase N` anchor vocabulary.

**Action options:** (a) narrow predicate markers to history phrases only, then gate; (b) leave as burn-down; (c) strip anchors from code comments into `DECISIONS.md` links (high churn, low value).

**Recommendation.** (b) unless you need a hard gate — promoting now would be perpetually red.

---

## CI & repo boundaries


| Repo            | CI does                                                                                 | Does not                                         |
| --------------- | --------------------------------------------------------------------------------------- | ------------------------------------------------ |
| **specify**     | `lint framework` + spec-runtime symlink check; checks out matching `specify-cli` branch | `cargo make test` (by design)                    |
| **specify-cli** | Full `cargo make ci` via reusable workflow                                              | Framework lint of `specify` unless you add a job |


**Optimize CI time:** `specify-cli` `cargo make test` builds `vectis-wasm` first — cache that artifact in CI; use `cargo nextest run --test <area>` locally when iterating.

**Cross-repo coupling:** Framework rules live in `specify/adapters/shared/rules/core/`; their generic dispatcher (declarative hints Road A / WASI tools Road B) runs in `specify-cli` — the imperative `authoring-predicate` bridge has been removed. Any rule change should run **both** `make lint` and `cargo nextest run -p specify-standards`.

---

## Skill / scenario coverage (manual debt map)

From `[docs/contributing/skills-test-coverage.md](docs/contributing/skills-test-coverage.md)`:


| Status  | Count | Examples                                                                      |
| ------- | ----- | ----------------------------------------------------------------------------- |
| ✓       | 7     | plan, refine, init, finalize, execute, contract OpenAPI/JSON Schema           |
| partial | 5     | build, merge, omnia build brief, vectis build + merge briefs                  |
| gap     | 5     | client SoW, contract AsyncAPI, captures adapter, wiretapper, drop (lifecycle) |


**Optimize-mode recommendation:** Do not author new skills — close gaps by (1) running existing lifecycle scenarios, (2) adding **one** focused contracts-style target test pack for omnia/vectis *only if* generated-output asserts are required before RM-05 sign-off.

---

## Suggested 4-week attention plan (improve-only)

1. **Week 1 — Gate proof:** Build the release `specify` and put it on PATH, run `01-pure-intent`, file run-summary, halt/fix until green.
2. **Week 2 — Lint depth:** Consumer `lint_project` hint-kind matrix (§B). Refresh perf docs (S1).

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

