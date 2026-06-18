# RFC-50: Adapter-Agnostic Core — Remove First-Party Adapter Coupling from `specify`

> Status: Draft - Depends: RFC-47 (adapter identity), RFC-48 (adapter packaging/registry), RFC-49 (adapter extraction to `specify-adapters`)

## Abstract

The `specify` platform repo should be agnostic of **any** specific source or target adapter. After the adapter-tree migration (see [Starting state](#starting-state)), the first-party adapter *content* (`intent`, `documentation`, `typescript`, `screenshots`, `captures` sources and the `omnia` target — joining the already-extracted `vectis` and `contracts`) lives in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters). What remains in `specify` is **coupling**: hard-coded adapter names in the engine, dedicated adapter documentation pages, adapter-specific plugin prose, and governance docs that use specific adapter names as load-bearing references rather than illustrative examples.

This RFC inventories that remaining coupling and proposes a phased plan to remove it, so that the only place a specific adapter name appears in `specify` is `evals/` and test fixtures (the deliberately-retained exception).

## Motivation

The operating model splits three concerns — **workflow** (phase orchestration), **engineering standards** (durable rule packs), and **artifacts** (slice/baseline intent) — and routes all adapter behavior through a uniform **source / target adapter** contract resolved by `SourceAdapter::resolve` / `TargetAdapter::resolve`. The contract is already axis-routed with no `if name == "intent"` branch in the resolver. But several engine paths, docs, and prose still special-case `omnia` / `vectis` / `contracts` by name. That coupling:

- **Breaks the abstraction.** A genuinely adapter-agnostic core lets a third-party adapter reach the same code paths as a first-party one. Name branches privilege first-party adapters.
- **Creates cross-repo drift.** Adapter knowledge duplicated in `specify` prose and in `specify-adapters` adapters must be kept in sync by hand.
- **Blocks clean packaging.** RFC-48/49 make adapters content-addressed artifacts; the host should consume them as opaque trees, not embed assumptions about their names or internals.

**Principle (from the project rules):** *If artifacts conflict with source, trust the artifacts.* The analogue here: if the engine needs adapter-specific behavior, that behavior belongs in the adapter manifest/extension, not in a host name-branch.

## Starting state

This RFC assumes the directory migration is already complete:

- `adapters/sources/*` and `adapters/targets/omnia` have moved to `specify-adapters`; their `references/spec-runtime` and `agent-teams.md` symlinks resolve against `specify-adapters/adapters/shared/`.
- Inbound `specify` links into the moved trees were converted to `https://github.com/augentic/specify-adapters/...` URLs (CORE-002 skips URL targets), and the moved adapters' outbound links into `specify` (`plugins/spec/...`, `evals/...`) likewise became `https://github.com/augentic/specify/...` URLs.
- The CI **Verify symlinks** step was guarded with `shopt -s nullglob` so it no-ops cleanly with no adapter trees present.
- `make lint` on `specify` is **green** (0 findings).

What remains in `specify/adapters/` is only `shared/` (the `core` + `universal` rule packs, `target-hooks/replay/`, and the `references/runtime` overlay). Removing that tree is **Phase 1** below.

## Scope

**In scope:** every reference to a *specific* source or target adapter name (`omnia`, `vectis`, `contracts`, `intent`, `documentation`, `typescript`, `screenshots`, `captures`) in `specify` outside the retained exception, across engine code, schemas, docs, plugins, and governance prose.

### Non-goals / retained exception

- **`evals/` and test fixtures may keep adapter names.** Scenario packs and fixtures legitimately exercise a concrete adapter end-to-end (e.g. `evals/fixtures/targets/omnia/`, `evals/fixtures/sources/screenshots/`, and the `omnia@1.0.0` routing in cross-repo scenarios). These are the proving ground and stay.
- **Generic vocabulary stays.** The *roles* `source adapter` / `target adapter`, the axis nouns, and the adapter contract are core vocabulary, not coupling.
- **Word collisions are not coupling.** `contracts` as an `ArtifactClass`, the `contract` WASI tool, and cross-project `contracts` wiring in `registry.yaml` are distinct from the `contracts` *adapter*. Audit must disambiguate (see [Acceptance criteria](#acceptance-criteria)).

## Inventory of remaining coupling

Severity legend: **B** = behavioral branch (engine changes its behavior based on an adapter name — the real abstraction break); **S** = structural/path assumption; **D** = documentation/prose; **C** = config/owner-map.

### A. Engine behavioral branches (highest priority)

| Sev | Location | Coupling |
| --- | --- | --- |
| B | `engine/crates/workflow/src/init/adapter_uri.rs:337` `first_party_repo()` | Routes `"contracts" \| "vectis" => "specify-adapters"`, everything else (incl. `omnia` + all sources) `=> "specify"`. **After the migration this is wrong** — `omnia` and the sources now live in `specify-adapters`, so `specify init omnia` fetches a dead path. See [Phase 0](#phase-0-unblock-init-urgent). |
| B | `engine/src/runtime/commands/slice/build.rs:66,147,179` | `const VECTIS_TARGET`/`VECTIS_TOOL`; `if manifest.name == VECTIS_TARGET { prepare_vectis_assets(...) }` runs Vectis asset auto-materialization in the build `prepare` phase. Resolution: fold `materialize → verify` into the target's own `build` brief (see [Mechanism](#mechanism-a-uniform-operation-envelope-runtime)). |
| B | `engine/src/runtime/commands/catalog/infer.rs:64,131` | `specify catalog infer` is a thin host wrapper around the `vectis` WASI tool (`run_captured(ctx, VECTIS_TOOL, ...)`). The whole `specify catalog` command exists to drive one target. |
| B | `engine/src/runtime/commands/slice.rs:28-40` | `artifact_classes()` hard-codes the "omnia default" `ArtifactClass` set (`specs` 3-way, `contracts` opaque) and `slice merge` string-filters `class_name == "specs"` / `"contracts"` on the wire (the merge engine itself is already name-agnostic — dispatch is on `MergeStrategy`). Resolution: keep the universal `spec.md`/decisions merge as core; move target-specific promotion behind the target's `merge` operation via a `MergeRequest`/`MergeReport` envelope (see [Mechanism](#mechanism-a-uniform-operation-envelope-runtime)). |
| B | `engine/crates/workflow/src/design_system.rs` | Component catalog factoring described/scoped as the Vectis target's build behavior. |
| B | `engine/crates/workflow/src/platform/{detect,bootstrap}.rs` + `crates/vectis-shell-detect/` | `vectis_missing_platforms`, `BootstrapContext`, and the in-process Crux shell-detection crate are Vectis-specific platform logic linked into the host (RFC-46). |
| B | `engine/crates/workflow/src/change/plan/core/propose/topology.rs` | Bootstrap DAG inserts `app-foundation` / `bootstrap-<platform>` slices driven by Vectis platform capability. |

### B. Engine structural / path assumptions

| Sev | Location | Coupling |
| --- | --- | --- |
| S | `engine/crates/standards/src/rules/resolve.rs:216-217` | `SHARED_REL = "adapters/shared/rules/universal"`, `CORE_REL = "adapters/shared/rules/core"` — the rule-pack resolution path. Also the consumer `rules export` / `rules sync` resolution order (project tree → monorepo `adapters/shared/rules/universal/` → codex cache) at lines 18-24, 149, 192-196, 318-323. |
| S | `engine/src/runtime/commands/lint/framework.rs:124` `canonical_framework_root()` | Hard-requires an `adapters/` directory to exist; aborts `framework-root` otherwise. `make lint --framework-root ..` depends on it. |
| S | `engine/crates/schema/src/cache.rs` | Adapter-store path helpers (`adapter_store_entry`, `verify_store_entry`) — generic, but keyed on adapter identity. |

### C. Engine config / owner maps (rule-pack-resident, not hard Rust)

| Sev | Location | Coupling |
| --- | --- | --- |
| C | `adapters/shared/rules/core/CORE-009-rule-namespace-owner.md` `config.owner-prefixes` | `omnia: [OMNIA, RUST, SEC]`, `contracts: [IFACE]`, `vectis: [VECTIS]`, `source-axis-prefixes: [SRC]`. Policy lives in rule config (correct), but it enumerates specific adapters. Travels with the rule pack in Phase 1. |
| C | `engine/crates/standards/src/lint/framework_tools/rules.rs`, `links_registry.rs` | Checker logic reads the owner map; no names baked in Rust, but verify. |
| C | `engine/crates/registry/src/permissions.rs`, `crates/extension-manifest/src/lib.rs` | Examples/permission sets referencing `vectis` / `contract` tools (largely illustrative; confirm none are load-bearing). |

### D. The remaining `adapters/shared/` tree

| Sev | Location | Coupling |
| --- | --- | --- |
| S/C | `adapters/shared/rules/{core,universal}/` | The framework + universal rule packs. Needed by `specify`'s own `make lint` and by the consumer `rules export`/`sync` contract. **Relocates** in Phase 1 (chosen home: top-level `rules/{core,universal}` — see [Decisions](#decisions-recorded)). |
| D | `adapters/shared/target-hooks/replay/` | Replay-hook adoption material consumed only by target adapters (`omnia`, `vectis`). Orphaned in `specify`; **moves to `specify-adapters`**. |
| S | `adapters/shared/references/runtime/` | Symlink overlay aliasing `plugins/spec/references/` + `docs/reference/`. Only existed to serve adapters' `spec-runtime` symlinks; with no adapters in `specify`, it serves nothing here. Resolve in Phase 1. |

### E. Documentation (`docs/`)

| Sev | Location | Coupling |
| --- | --- | --- |
| D | `docs/reference/targets/omnia.md`, `docs/reference/targets/vectis.md`, `docs/reference/cli/vectis.md` | Dedicated per-adapter pages. `omnia.md:4,55` still carry `https://github.com/augentic/specify/adapters/targets/omnia` (mirroring the `first_party_repo` bug) as code spans. |
| D | `docs/explanation/components.md` | Documents a Vectis-only feature (component catalog factoring). |
| D | `docs/reference/targets/index.md`, `docs/reference/sources/index.md`, `docs/explanation/adapter-anatomy.md`, `docs/explanation/layered-stack.md`, `docs/appendices/glossary.md` | Use `omnia`/`vectis`/named sources as the canonical examples of the adapter contract. |
| D | `docs/SUMMARY.md` | Navigation entries for the per-adapter pages. |
| D | `docs/reference/plugins/index.md`, `docs/reference/artifact-format.md`, `docs/reference/declared-tool-helper-inventory.md`, `docs/contributing/{index,checks,plugin-development,skills-test-coverage}.md`, `docs/orientation/prerequisites.md` | Scattered named-adapter examples (links already converted to `specify-adapters` URLs where they pointed into moved trees). |

### F. Plugins (`plugins/`)

| Sev | Location | Coupling |
| --- | --- | --- |
| D | `plugins/spec/rules/spec.mdc` | Lists `intent`/`documentation`/`typescript`/`screenshots` and target examples. |
| D | `plugins/spec/references/components.md` | Marked Vectis-only. |
| D | `plugins/spec/skills/*/SKILL.md` | Mostly templated `$TARGET` / `adapters/<axis>/<name>/briefs/...` (axis-generic — good), but some carry `omnia`/`vectis` examples. |
| D | `plugins/capture/**` (`README.md`, `rules/capture.mdc`, `skills/wiretapper/SKILL.md`) | The capture plugin is intrinsically about the `typescript` + `captures` sources and the `omnia` replay target. Decide: genericize, or accept as an adapter-coupled plugin and document the dependency. |

### G. Governance / prose

| Sev | Location | Coupling |
| --- | --- | --- |
| D | `AGENTS.md` (root) | Vocabulary and examples lean heavily on `omnia`/`vectis`/`contracts`/named sources. |
| D | `.cursor/rules/project.mdc` | Adapter examples + (now-converted) Omnia guardrails link. |
| D | `engine/AGENTS.md`, `engine/DECISIONS.md` | Adapter names as examples in module docs and decisions. |
| D | `README.md`, `REVIEW.md`, `branding/names*.md`, `update_names_again.py` / `fix_table_again.py` (root helper scripts) | Assorted named-adapter mentions. |
| — | `rfcs/**` | Historical RFCs (e.g. `rfc-24-omnia.md`) legitimately name adapters; leave as history. |

## Mechanism: a uniform operation-envelope runtime

The through-line for category **A** is one idea: the host's contract with a target is a **wire ABI dispatched generically**, and the host holds only *core* artifacts. The end state is **no adapter-specific code at all** in `specify` (the `evals/` + fixtures exception aside).

**Why not a host-side Rust trait.** The obvious Rust move — `trait TargetAdapter` with an `impl` per adapter compiled into the host — is the *strongest* coupling, not the weakest: it reintroduces the compile-time dependency RFC-48/49 removed (adapters are out-of-tree, content-addressed, independently versioned artifacts), it cannot cross the WASI process boundary the adapters live behind, and it cannot represent the half of each adapter that is **agent-executed markdown** (the briefs). The host already has the correctly-shaped abstraction: one trait (`ToolRunner`) with a single generic impl (`WasiToolRunner`) that dispatches on a runtime tool *name*, not a type. The core defines one seam with one impl; *which* adapter is data.

**The interface is the closed operation set + versioned envelopes.** The "methods" are the closed `TargetOperation` enum (`shape | build | merge`); each exchanges a fixed-shape, schema-validated JSON envelope, dispatched by a single host runtime that routes to the bound target's declared WASI tool (`execution: tool`) or its two-phase brief handoff (`execution: agent`). `build` is the reference implementation — the host assembles a `BuildRequest` from the manifest's declared `inputs[]`, hands off, and validates a `BuildReport`, never reading adapter internals. De-branching category A is, in one sentence: **make every operation look like `build`.** Each operation is two-phase with a dry-run / preview pass so the host keeps deterministic preview / drift checks by rendering *whatever the report says* — never an `if name ==` or a `class_name == "specs"` literal.

**Core vs adapter — the partition that makes "no adapter-specific code" true and checkable.** "No adapter-specific code" does *not* mean "no merge/build code in the host." It means **no host code that names, or varies by, any adapter or its private taxonomy.** The host legitimately owns the workflow artifacts Specify itself defines; everything that varies by target is reached only through the fixed envelope + brief handoff.

| Layer | Owns | Varies by adapter? |
| --- | --- | --- |
| **Core (host)** | Workflow artifacts Specify defines — `spec.md` / `model.yaml` / decisions / plan / slice lifecycle / archive — plus the generic operation-dispatch runtime | No — identical for every adapter |
| **Adapter (out-of-tree)** | Opaque `contracts/`, generated crates, shells, design-system, and *how* they fold into the baseline | Yes — reached only via the fixed envelope + brief handoff |

Litmus test for "done": a brand-new third-party target drops in with only `adapter.yaml` + briefs + an optional `adapter.wasm` and reaches every host code path identically, with no host change.

The per-item moves, each an instance of that model:

- **Build prepare hooks** (`prepare_vectis_assets`): fold the prepare work into the target's own `build` execution rather than adding a host `prepare` capability. The bound target's `build` brief opens with its own `materialize → verify` steps (for Vectis: materialize assets, then the asset / app-icon verification); the verify step fails fast on missing or contradictory exports — one step into the build, before any render/codegen — giving the same early failure the host gate provided, with no host precondition. The host keeps only its already-generic two-phase `prepare` (request assemble + schema-validate) / `finalize` (report validate + transition gate) seam and drops `VECTIS_TARGET`/`VECTIS_TOOL`, `prepare_vectis_assets`, and the entire `materialize_scope` module. A failed verify surfaces as a blocking finding the existing `finalize` already rejects (`target-build-success-with-blocking-finding` / `target-build-failed`). No `if name == "vectis"`, and no new host capability to declare or dispatch.
- **`specify catalog`** (`vectis infer`): retire the bespoke command in favor of the existing generic `specify extension run <target> -- infer`, or make `catalog` dispatch to the bound target's declared `infer` tool. The host stops naming `vectis`.
- **Artifact classes** (`omnia` default in `slice.rs`): the merge engine is *already* name-agnostic (dispatch is on the closed `MergeStrategy`, never on `ArtifactClass::name`); the coupling is that `slice merge` is **target-blind** — it applies an unconditional omnia-shaped class set (`specs` 3-way + `contracts` opaque) to every slice and even string-filters `class_name == "specs"` / `"contracts"` on the wire. Resolution is **not** "let the manifest declare its `ArtifactClass` set for the host to interpret" — that re-teaches the host every adapter's taxonomy (coupling-by-config, the same trap as a host `prepare` capability). Instead: the host keeps the **universal** `spec.md` 3-way merge + decisions promotion as *core* behavior (these are core artifacts, not adapter-specific) and drops the hard-coded `contracts` class and the `class_name` wire filters; target-specific promotion (opaque `contracts/`, generated trees) moves behind the bound target's `merge` operation via a fixed `MergeRequest` / `MergeReport` envelope (sibling to `build`'s), two-phase with a preview pass so the host keeps deterministic preview / conflict-check by rendering the report's operations generically.
- **Component factoring / `design_system.rs`**: move into the Vectis extension; the host exposes only the generic component-catalog plumbing (`components.yaml` validation already lives behind `specify slice validate` + the vectis extension).
- **Platform detection / bootstrap** (RFC-46): the hardest. `vectis-shell-detect` is linked in-process for plan-time speed. Options: (a) keep an in-process detector but drive it from a target-declared `platforms` capability + a generic "shell present?" probe interface; (b) move detection behind the target extension and accept a plan-time WASI call. Track as its own follow-up; it need not block Phases 0–1.

**The one thing no abstraction removes.** A target's briefs are prompts, not callable code, so the envelope captures only the deterministic seams (assemble request, validate report, gate lifecycle); the agent work crosses as a generic brief handoff. The contract is therefore hybrid by necessity — typed JSON envelopes for the deterministic boundary plus a brief handoff for the agent boundary — which is exactly why a single Rust trait is insufficient and the wire ABI is the real interface. (If the deterministic-tool seam ever wants a *typed* cross-process interface, the idiom is a WIT / Component-Model world each `execution: tool` adapter exports — a complement for tools, not a replacement for the handoff.)

## Phased plan

Each phase is independently mergeable and **must keep `make lint` and `cargo make ci` green**.

### Phase 0 — Unblock `init` (urgent)

The migration already moved `omnia` + sources to `specify-adapters`, so `first_party_repo` must route them there. Minimal change:

```rust
// engine/crates/workflow/src/init/adapter_uri.rs
fn first_party_repo(_name: &str) -> &'static str {
    // All first-party adapters now live in specify-adapters.
    "specify-adapters"
}
```

Update `adapter_uri/tests.rs` accordingly, and fix the `docs/reference/targets/omnia.md` `URL:`/`target:` code spans. **Without this, `specify init omnia` is broken today.** (This is the one engine change worth fast-tracking ahead of the rest.)

### Phase 1 — Empty `adapters/` from `specify`

1. **Relocate the rule packs** `adapters/shared/rules/{core,universal}` → top-level `rules/{core,universal}` (recorded decision). Update `SHARED_REL`/`CORE_REL` and the resolution-order prose/messages in `resolve.rs`; update the consumer `rules export`/`sync` resolution + any codex-cache layout, and `resolve/tests.rs`.
2. **Relax `canonical_framework_root`** so a framework root no longer requires `adapters/` (accept a root with `plugins/`+`docs/`+`rules/`). Update `framework.rs` and its fixtures.
3. **Update `make lint`** (and `.github/workflows/ci.yaml`): the framework profile must find the rule pack at its new path; drop the now-vacuous **Verify symlinks** step (or move it to `specify-adapters` CI, which should own adapter symlink integrity).
4. **Move `adapters/shared/target-hooks/replay/`** to `specify-adapters/adapters/shared/` (it serves only target adapters); update the `specify` copy's already-URL-ized references as needed.
5. **Resolve the `references/runtime` overlay**: with no adapters in `specify`, delete the overlay; the canonical bodies stay at `plugins/spec/references/` + `docs/reference/`.
6. **Delete `adapters/`** from `specify`. Re-run `make lint` (now reading `rules/`) to confirm green.

> Cross-repo note: `specify-adapters` carries its **own** forked `shared/` and lints via a branch-matched `specify` binary. Its rule pack still lives at `adapters/shared/rules/` (correct for its tree). Keep the two resolution roots distinct; do not assume one path serves both repos.

### Phase 2 — De-branch the engine (category A)

Apply the [Mechanism](#mechanism-a-uniform-operation-envelope-runtime): prepare hooks, `catalog`, artifact classes, `design_system`. The headline engine work is the `MergeRequest` / `MergeReport` envelope that makes `merge` look like `build`, shrinking the host's `slice merge` to the universal `spec.md` / decisions merge plus generic operation dispatch — dropping the hard-coded `artifact_classes()` and the `class_name == "specs"` / `"contracts"` wire filters. Each removal of a `== "vectis"` / `omnia` branch lands with the adapter-side change that replaces it (an operation brief/tool, or the `materialize → verify` steps folded into the target's own `build` brief) and an updated adapter in `specify-adapters`. Platform detection/bootstrap (RFC-46) is tracked separately.

### Phase 3 — Documentation (category E)

- Per-adapter pages (`targets/omnia.md`, `targets/vectis.md`, `cli/vectis.md`, `explanation/components.md`): relocate the substance to `specify-adapters` and leave thin pointer stubs, or genericize into the adapter-contract docs. Update `docs/SUMMARY.md`.
- Genericize the adapter-contract examples (`adapter-anatomy.md`, `sources/index.md`, `targets/index.md`, `glossary.md`) to use placeholder names (`<source>`, `<target>`) rather than `omnia`/`vectis`.

### Phase 4 — Plugins + prose (categories F, G)

- `plugins/spec/rules/spec.mdc`, `plugins/spec/references/components.md`: neutralize adapter-specific examples.
- Decide the `plugins/capture` question (genericize vs. document as adapter-coupled).
- `AGENTS.md` (root + `engine/`), `DECISIONS.md`, `.cursor/rules/project.mdc`, `README.md`, `REVIEW.md`, `branding/*`: keep the *role* vocabulary; replace load-bearing named-adapter references with neutral ones; retire the root `*.py` helper scripts if dead.

## Decisions recorded

- **Rule-pack home:** top-level `specify/rules/{core,universal}` (not `standards/`, not back into `specify-adapters`). `specify` keeps its own framework + universal packs for self-lint and for the consumer export contract.
- **Sequencing:** content move first (done) → Phase 0 routing fix → Phase 1 empty `adapters/` → Phase 2 de-branch → Phases 3–4 prose. Engine de-branching and prose were intentionally deferred out of the content move to keep each step green.
- **Prepare hook:** fold the Vectis `materialize → verify` steps into the target's own `build` execution (the build brief's opening steps) rather than adding a host `prepare` capability. The build's verify step preserves the early-failure guarantee with no host precondition gate, so the host drops the prepare hook and `materialize_scope` module outright instead of replacing them with new generic dispatch.
- **Adapter interface = wire ABI, not a Rust trait.** A host-side `trait TargetAdapter` with per-adapter `impl`s is rejected: it reintroduces compile-time coupling, cannot cross the WASI boundary, and cannot represent agent-executed briefs. The interface is the closed `TargetOperation` set exchanging versioned JSON envelopes, dispatched by one generic host runtime (the `ToolRunner` / `WasiToolRunner` "one seam, one impl, dispatch-by-name" shape) that routes to the declared WASI tool or the two-phase brief handoff.
- **Core vs adapter partition.** The host owns the workflow artifacts Specify defines (`spec.md` / `model.yaml` / decisions / plan / lifecycle / archive) plus the generic dispatch runtime; everything that varies by target is reached only through the fixed envelope + brief handoff. "No adapter-specific code" means no host code that names or varies by an adapter or its taxonomy — not "no merge/build code."
- **Artifact classes / merge.** Keep the universal `spec.md` 3-way merge + decisions promotion in the host as core; drop the hard-coded `contracts` opaque class and the `class_name` wire filters. Target-specific promotion moves behind the bound target's `merge` operation via a `MergeRequest` / `MergeReport` envelope (sibling to `build`'s), two-phase with a preview pass. Do not have the manifest declare an `ArtifactClass` set for the host to interpret.

## Risks and invariants

- **Green at every step.** `make lint` (framework) and `cargo make ci` must pass per phase. Phase 1 is the riskiest (touches `resolve.rs` + `framework.rs` + CI + the consumer contract).
- **Consumer `rules` contract.** Moving the `universal` pack changes the resolution path that downstream projects' `rules export`/`sync` rely on. Coordinate the path change and the codex-cache layout in one change; document the migration for existing consumer projects.
- **Cross-repo seam.** `specify-adapters` CI builds a branch-matched `specify` binary and runs `lint framework --framework-root .`. It currently has **pre-existing** CORE-002 findings (its `vectis`/`contracts`/forked-`shared` links into `specify` via relative paths) and a `CORE-055` schema-id skew against older binaries. These are independent of this RFC but worth a parallel cleanup (convert those cross-repo links to URLs; reconcile the `CORE-055`/`framework` schema version).
- **Don't over-genericize vocabulary.** The goal is removing *name coupling*, not erasing the `source`/`target` adapter model.

## Acceptance criteria

1. **Engine — no adapter names *or* taxonomy.** The host carries no adapter-name literal and no adapter-*taxonomy* literal (artifact-class names used as routing, per-class baseline/staged dir assumptions) outside `evals/` + fixtures:
   ```bash
   rg -n -i 'omnia|vectis' engine/src engine/crates --type rust \
     -g '!**/tests/**' -g '!**/fixtures/**'
   rg -n '"specs"|"contracts"' engine/src engine/crates --type rust \
     -g '!**/tests/**' -g '!**/fixtures/**'
   ```
   returns only generic/justified hits (and `contracts`/`contract` hits are confirmed to be the artifact-class / WASI-tool / cross-project usages, not the adapter). A standing guard test enforces this, so "no adapter-specific code" stays a regression-proof property rather than a one-time cleanup.
2. **Tree.** `specify/adapters/` no longer exists; the rule packs resolve from `rules/{core,universal}`; `make lint` is green.
3. **Docs/plugins.** No dedicated per-adapter page or adapter-specific plugin prose remains in `specify` (beyond pointer stubs); `docs/SUMMARY.md` carries no per-adapter nav.
4. **Prose.** `AGENTS.md`, `DECISIONS.md`, `.cursor/rules/project.mdc`, `README.md` use neutral role vocabulary; specific adapter names survive only as clearly-labelled examples or history.
5. **Exception honored.** `evals/` and test fixtures may still name adapters; nothing in this RFC removes them.
6. **Drop-in.** A synthetic third-party target with only `adapter.yaml` + briefs + an optional `adapter.wasm` reaches every host code path (`shape` / `build` / `merge`, prepare / preview / finalize) with no host change.
