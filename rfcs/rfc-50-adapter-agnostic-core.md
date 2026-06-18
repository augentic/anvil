# RFC-50: Adapter-Agnostic Core — Remove First-Party Adapter Coupling from `specify`

> Status: Draft - Depends: RFC-47 (adapter identity), RFC-48 (adapter packaging/registry), RFC-49 (adapter extraction to `specify-adapters`)

## Abstract

The `specify` platform repo should be agnostic of **any** specific source or target adapter. The adapter *content* — the `intent` / `documentation` / `typescript` / `screenshots` / `captures` sources and the `omnia` / `vectis` / `contracts` targets — lives in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters). What remains in `specify` is **coupling**: hard-coded adapter names in the engine, per-adapter docs, and prose that treats specific adapters as load-bearing rather than illustrative. This RFC inventories that coupling and phases its removal, so the only place a specific adapter name appears in `specify` is `evals/` and test fixtures.

## Motivation

The operating model routes all adapter behavior through a uniform **source / target adapter** contract (`SourceAdapter::resolve` / `TargetAdapter::resolve`), already axis-routed with no `if name ==` in the resolver. But several engine paths, docs, and prose still special-case `omnia` / `vectis` / `contracts` by name. That coupling:

- **Breaks the abstraction.** A third-party adapter should reach the same code paths as a first-party one; name branches privilege first-party adapters.
- **Creates cross-repo drift.** Adapter knowledge duplicated in `specify` prose and in `specify-adapters` must be hand-synced.
- **Blocks clean packaging.** RFC-48/49 make adapters content-addressed artifacts; the host should consume them as opaque trees, not embed their names or internals.

The analogue of the project's *trust the artifacts* principle: adapter-specific behavior belongs in the adapter manifest/extension, never in a host name-branch.

## Starting state

The directory migration is already complete:

- `adapters/sources/*` and `adapters/targets/omnia` have moved to `specify-adapters`; inbound/outbound cross-repo links are now `https://github.com/...` URLs, and `make lint` on `specify` is green.
- Platform detection / bootstrap (RFC-46) already moved into the vectis extension — `platform.rs` keeps only the generic `Platform` enum, and the host does no shell detection, inserts no bootstrap slices, and runs no launcher-icon gate.

What remains in `specify/adapters/` is only `shared/` (the `core` + `universal` rule packs, `target-hooks/replay/`, and the `references/runtime` overlay). Emptying that tree is **Phase 1**.

## Scope

**In scope:** every reference to a *specific* adapter name (`omnia`, `vectis`, `contracts`, `intent`, `documentation`, `typescript`, `screenshots`, `captures`) in `specify` outside the retained exception — across engine code, schemas, docs, plugins, and prose.

### Non-goals / retained exception

- **`evals/` and test fixtures keep adapter names.** They exercise a concrete adapter end-to-end; they are the proving ground and stay.
- **Generic vocabulary stays.** The *roles* `source adapter` / `target adapter`, the axis nouns, and the adapter contract are core vocabulary, not coupling.
- **Word collisions are not coupling.** `contracts` as an `ArtifactClass`, the `contract` WASI tool, and `contracts` wiring in `registry.yaml` are distinct from the `contracts` *adapter*; the audit must disambiguate.

## Inventory of remaining coupling

Severity: **B** = behavioral branch (engine behavior varies by adapter name — the real break); **S** = structural/path assumption; **D** = docs/prose; **C** = config/owner-map.

### A. Engine behavioral branches (highest priority)

| Sev | Location | Coupling |
| --- | --- | --- |
| B | `engine/crates/workflow/src/init/adapter_uri.rs:337` `first_party_repo()` | Routes `"contracts" \| "vectis" => "specify-adapters"`, everything else (incl. `omnia` + all sources) `=> "specify"`. **After the migration this is wrong** — `omnia` and the sources now live in `specify-adapters`, so `specify init omnia` fetches a dead path. See [Phase 0](#phase-0-unblock-init-urgent). |
| B | `engine/src/runtime/commands/slice/build.rs:66,147,179` | `const VECTIS_TARGET`/`VECTIS_TOOL`; `if manifest.name == VECTIS_TARGET { prepare_vectis_assets(...) }` runs Vectis asset auto-materialization in the build `prepare` phase. |
| B | `engine/src/runtime/commands/catalog/infer.rs:64,131` | `specify catalog infer` is a thin host wrapper around the `vectis` WASI tool (`run_captured(ctx, VECTIS_TOOL, ...)`). The whole `specify catalog` command exists to drive one target. |
| B | `engine/src/runtime/commands/slice.rs:28-40` | `artifact_classes()` hard-codes the "omnia default" `ArtifactClass` set (`specs` 3-way, `contracts` opaque) and `slice merge` string-filters `class_name == "specs"` / `"contracts"` on the wire (the merge engine itself is name-agnostic — dispatch is on `MergeStrategy`). |
| B | `engine/crates/workflow/src/design_system.rs` | Component-catalog factoring scoped as the Vectis target's build behavior. |

Category A resolutions follow one model — see [Mechanism](#mechanism-a-uniform-operation-envelope-runtime). (The platform-detection / bootstrap branch that previously sat here is already resolved; see [Starting state](#starting-state).)

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
| C | `engine/crates/registry/src/permissions.rs`, `crates/extension/src/lib.rs` | Examples/permission sets referencing `vectis` / `contract` tools (largely illustrative; confirm none are load-bearing). |

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
| D | `README.md`, `REVIEW.md`, `branding/names*.md`, root helper scripts | Assorted named-adapter mentions. |
| — | `rfcs/**` | Historical RFCs (e.g. `rfc-24-omnia.md`) legitimately name adapters; leave as history. |

## Mechanism: a uniform operation-envelope runtime

Category **A** reduces to one idea: the host's contract with a target is a **wire ABI dispatched generically**, and the host holds only *core* artifacts. The end state is **no adapter-specific code** in `specify` (the `evals/` exception aside).

**Not a host-side Rust trait.** The obvious move — `trait TargetAdapter` with a per-adapter `impl` compiled into the host — is the *strongest* coupling: it reintroduces the compile-time dependency RFC-48/49 removed, it can't cross the WASI boundary the adapters live behind, and it can't represent the half of each adapter that is agent-executed markdown (the briefs). The host already has the right shape: one trait (`ToolRunner`) with a single generic impl (`WasiToolRunner`) that dispatches on a runtime *name*, not a type. *Which* adapter is data.

**The interface is the closed operation set + versioned envelopes.** The "methods" are `TargetOperation` (`shape | build | merge`); each exchanges a fixed-shape, schema-validated JSON envelope, dispatched by one host runtime that routes to the target's declared WASI tool (`execution: tool`) or its two-phase brief handoff (`execution: agent`). `build` is the reference: the host assembles a `BuildRequest` from the manifest's declared `inputs[]`, hands off, and validates a `BuildReport`, never reading adapter internals. De-branching category A is one sentence — **make every operation look like `build`**: two-phase with a preview pass, the host rendering *whatever the report says*, never an `if name ==` or a `class_name == "specs"` literal.

**Core vs adapter.** "No adapter-specific code" means no host code that names or varies by an adapter or its private taxonomy — not "no merge/build code." The host owns the artifacts Specify itself defines; everything target-specific is reached only through the envelope + handoff.

| Layer | Owns | Varies by adapter? |
| --- | --- | --- |
| **Core (host)** | `spec.md` / `model.yaml` / decisions / plan / slice lifecycle / archive, plus the generic operation-dispatch runtime | No |
| **Adapter (out-of-tree)** | Opaque `contracts/`, generated crates, shells, design-system, and *how* they fold into the baseline | Yes — via envelope + handoff only |

Litmus for "done": a brand-new third-party target drops in with only `adapter.yaml` + briefs + an optional `adapter.wasm` and reaches every host path identically, with no host change.

The category-A moves, each an instance of this model:

- **Build prepare hooks** (`prepare_vectis_assets`): fold `materialize → verify` into the target's own `build` brief rather than adding a host `prepare` capability. The verify step fails fast on missing/contradictory exports — before any render — giving the same early failure with no host precondition. The host keeps its already-generic two-phase `prepare`/`finalize` seam and drops `VECTIS_TARGET`/`VECTIS_TOOL`, `prepare_vectis_assets`, and the `materialize_scope` module; a failed verify is a blocking finding `finalize` already rejects.
- **`specify catalog`** (`vectis infer`): retire the bespoke command in favor of generic `specify extension run <target> -- infer`. The host stops naming `vectis`.
- **Artifact classes** (`omnia` default in `slice.rs`): keep the **universal** `spec.md` 3-way merge + decisions promotion as core; drop the hard-coded `contracts` opaque class and the `class_name` wire filters. Target-specific promotion (opaque `contracts/`, generated trees) moves behind the target's `merge` operation via a `MergeRequest` / `MergeReport` envelope (sibling to `build`'s), two-phase with a preview pass. *Not* "let the manifest declare its `ArtifactClass` set for the host to interpret" — that re-teaches the host every adapter's taxonomy.
- **Component factoring** (`design_system.rs`): move into the Vectis extension; the host keeps only the generic component-catalog plumbing (`components.yaml` validation behind `specify slice validate`).

**The one thing no abstraction removes.** Briefs are prompts, not callable code, so the envelope captures only the deterministic seams (assemble request, validate report, gate lifecycle); the agent work crosses as a generic handoff. The contract is hybrid by necessity — typed envelopes for the deterministic boundary, a brief handoff for the agent boundary — which is why a single Rust trait is insufficient. (If the deterministic-tool seam ever wants a *typed* cross-process interface, the idiom is a WIT / Component-Model world each `execution: tool` adapter exports — a complement for tools, not a replacement for the handoff.)

## Phased plan

Each phase is independently mergeable and **must keep `make lint` and `cargo make ci` green**.

### Phase 0 — Unblock `init` (urgent)

The migration already moved `omnia` + sources to `specify-adapters`, so `first_party_repo` must route them there:

```rust
// engine/crates/workflow/src/init/adapter_uri.rs
fn first_party_repo(_name: &str) -> &'static str {
    // All first-party adapters now live in specify-adapters.
    "specify-adapters"
}
```

Update `adapter_uri/tests.rs` and fix the `docs/reference/targets/omnia.md` `URL:`/`target:` code spans. Without this, `specify init omnia` is broken today — the one engine change worth fast-tracking.

### Phase 1 — Empty `adapters/` from `specify`

1. **Relocate the rule packs** `adapters/shared/rules/{core,universal}` → top-level `rules/{core,universal}`. Update `SHARED_REL`/`CORE_REL` and the resolution-order prose/messages in `resolve.rs`; update the consumer `rules export`/`sync` resolution + codex-cache layout, and `resolve/tests.rs`.
2. **Relax `canonical_framework_root`** so a framework root no longer requires `adapters/` (accept a root with `plugins/`+`docs/`+`rules/`). Update `framework.rs` and its fixtures.
3. **Update `make lint`** (and `.github/workflows/ci.yaml`): the framework profile must find the rule pack at its new path; drop the now-vacuous **Verify symlinks** step (or move it to `specify-adapters` CI).
4. **Move `adapters/shared/target-hooks/replay/`** to `specify-adapters/adapters/shared/`; update the `specify` copy's references as needed.
5. **Resolve the `references/runtime` overlay**: with no adapters in `specify`, delete it; the canonical bodies stay at `plugins/spec/references/` + `docs/reference/`.
6. **Delete `adapters/`** from `specify`. Re-run `make lint` (now reading `rules/`) to confirm green.

> Cross-repo note: `specify-adapters` carries its **own** forked `shared/` and lints via a branch-matched `specify` binary; its rule pack still lives at `adapters/shared/rules/`. Keep the two resolution roots distinct.

### Phase 2 — De-branch the engine (category A)

Apply the [Mechanism](#mechanism-a-uniform-operation-envelope-runtime): prepare hooks, `catalog`, artifact classes, `design_system`. The headline engine work is the `MergeRequest` / `MergeReport` envelope that makes `merge` look like `build`, shrinking `slice merge` to the universal `spec.md`/decisions merge plus generic dispatch (dropping `artifact_classes()` and the `class_name` wire filters). Each branch removal lands with the adapter-side change that replaces it and an updated adapter in `specify-adapters`.

### Phase 3 — Documentation (category E)

- Per-adapter pages (`targets/omnia.md`, `targets/vectis.md`, `cli/vectis.md`, `explanation/components.md`): relocate the substance to `specify-adapters` and leave thin pointer stubs, or genericize into the adapter-contract docs. Update `docs/SUMMARY.md`.
- Genericize the adapter-contract examples (`adapter-anatomy.md`, `sources/index.md`, `targets/index.md`, `glossary.md`) to placeholder names (`<source>`, `<target>`).

### Phase 4 — Plugins + prose (categories F, G)

- `plugins/spec/rules/spec.mdc`, `plugins/spec/references/components.md`: neutralize adapter-specific examples.
- Decide the `plugins/capture` question (genericize vs. document as adapter-coupled).
- `AGENTS.md` (root + `engine/`), `DECISIONS.md`, `.cursor/rules/project.mdc`, `README.md`, `REVIEW.md`, `branding/*`: keep the *role* vocabulary; replace load-bearing named-adapter references with neutral ones; retire dead root helper scripts.

## Decisions recorded

- **Rule-pack home:** top-level `specify/rules/{core,universal}` (not `standards/`, not back into `specify-adapters`).
- **Sequencing:** Phase 0 routing fix → Phase 1 empty `adapters/` → Phase 2 de-branch → Phases 3–4 prose. Engine/prose deferred out of the content move to keep each step green.
- **Adapter interface = wire ABI, not a Rust trait** — the closed `TargetOperation` set exchanging versioned JSON envelopes, dispatched by one generic runtime (the `ToolRunner`/`WasiToolRunner` "one seam, one impl, dispatch-by-name" shape).
- **Core vs adapter partition** — host owns Specify's workflow artifacts + generic dispatch; everything target-specific is reached only via envelope + handoff. "No adapter-specific code" = no host code that names or varies by an adapter or its taxonomy.
- **Prepare hook** — fold Vectis `materialize → verify` into the target's `build` brief; no host `prepare` capability; drop the hook and `materialize_scope` outright (the build's verify preserves the early-failure guarantee).
- **Artifact classes / merge** — universal `spec.md`/decisions merge stays core; target-specific promotion moves behind the target's `merge` operation via a `MergeRequest`/`MergeReport` envelope. The manifest does *not* declare an `ArtifactClass` set for the host to interpret.

## Companion: test-fixture decoupling (landed ahead of the phases)

Ahead of the engine de-branching, a coverage-neutral sweep converted the **incidental** adapter names in `specify` *unit-test* fixtures to contrived placeholders, so a fixture no longer implies a first-party adapter is load-bearing. Convention: sources `demo-source` / `demo-docs`, targets `demo-target` / `other-target`, WASI tool `demo-tool`; target-owned rule IDs use the schema-valid generic prefix `ORG-` (e.g. `ORG-001`). Touched: `workflow` (`adapter/core`, `change/plan/core/{model,validate,status,propose}`, `agents/render`, `slice/build/wire`, `init/adapter_uri`), `standards` (`rules/resolve{,/filter,/sort}`, `rules/parse`, `lint/eval/{finding,cli_contract}`, `lint/framework_tools/{links_registry,extension}`), `registry` (`oci`, `store`, `resolver`, `cache`, `load`, `lib`, `package`), `schema` (`cache`), `extension` (`lib`, `validate`), and the binary (`runtime/commands/source/prep`). The `evals/` + on-disk fixture-tree exception ([Scope](#scope)) is honored — only inline / `*tests.rs` fixtures changed; `specify-model` needed none (its `documentation` / `intent` tokens are the `authority` enum, not adapters).

**Deliberately left coupled** — these tests *pin the src / policy coupling this RFC removes*, so they must change **with** the owning phase, not in the fixture sweep:

| Test | Why real names stay | Revised by |
| --- | --- | --- |
| `init/adapter_uri/tests.rs::first_party_repo_routes_extracted` | Asserts the `name → repo` routing map directly (the Phase 0 bug, [Category A](#a-engine-behavioral-branches-highest-priority)). | Phase 0 |
| `init/adapter_uri/tests.rs::shorthand_resolves_via_github` (networked) | Resolves a real first-party shorthand against GitHub end-to-end — the `evals` / smoke exception. | — (exception) |
| `standards/.../framework_tools/rules.rs` CORE-009 owner-map cases | Mirror the `owner-prefixes` policy (`omnia` / `vectis` / `contracts`) in [CORE-009](#c-engine-config--owner-maps-rule-pack-resident-not-hard-rust)'s rule config. | Phase 1 (travels with the rule pack) |
| Rule-id **schema prefix enum** (`OMNIA` / `VECTIS` in the rule schema) | The schema still enumerates adapter-named prefixes; `ORG-` is the neutral stand-in until the enum is opened. | Phase 1–2 (schema de-branch) |
| `standards` on-disk fixture trees under `tests/fixtures/lint/**` | Verbatim fixture files, outside the inline-fixture scope. | Phase 1–2 alongside `resolve.rs` / `framework.rs` |
| Any merge / `class_name` (`"specs"` / `"contracts"`) or `design_system` test | Pins the `artifact_classes()` / Vectis-factoring branch itself. | Phase 2 |

This makes the surviving real-name unit fixtures an *intentional, enumerated* set tied to category-A/B/C coupling — not drift. A future phase clears each row as it removes the matching src branch.

## Risks and invariants

- **Green at every step.** `make lint` (framework) and `cargo make ci` must pass per phase. Phase 1 is riskiest (touches `resolve.rs` + `framework.rs` + CI + the consumer contract).
- **Consumer `rules` contract.** Moving the `universal` pack changes the resolution path downstream `rules export`/`sync` rely on. Coordinate the path change + codex-cache layout in one change; document the migration.
- **Cross-repo seam.** `specify-adapters` CI builds a branch-matched `specify` binary and lints `--framework-root .`. It has pre-existing CORE-002 (cross-repo relative links) and CORE-055 (schema-id skew) findings — independent of this RFC, worth a parallel cleanup.
- **Don't over-genericize vocabulary.** Remove *name coupling*, not the `source`/`target` adapter model.

## Acceptance criteria

1. **Engine — no adapter names or taxonomy.** Outside `evals/` + fixtures, the host carries no adapter-name literal and no adapter-*taxonomy* literal (artifact-class names used as routing, per-class dir assumptions):
   ```bash
   rg -n -i 'omnia|vectis' engine/src engine/crates --type rust \
     -g '!**/tests/**' -g '!**/fixtures/**'
   rg -n '"specs"|"contracts"' engine/src engine/crates --type rust \
     -g '!**/tests/**' -g '!**/fixtures/**'
   ```
   returns only generic/justified hits (`contracts`/`contract` confirmed as the artifact-class / WASI-tool / cross-project usages, not the adapter). A standing guard test keeps this regression-proof.
2. **Tree.** `specify/adapters/` no longer exists; the rule packs resolve from `rules/{core,universal}`; `make lint` is green.
3. **Docs/plugins.** No dedicated per-adapter page or adapter-specific plugin prose remains (beyond pointer stubs); `docs/SUMMARY.md` carries no per-adapter nav.
4. **Prose.** `AGENTS.md`, `DECISIONS.md`, `.cursor/rules/project.mdc`, `README.md` use neutral role vocabulary; specific adapter names survive only as clearly-labelled examples or history.
5. **Exception honored.** `evals/` and test fixtures may still name adapters.
6. **Drop-in.** A synthetic third-party target with only `adapter.yaml` + briefs + an optional `adapter.wasm` reaches every host code path (`shape` / `build` / `merge`, prepare / preview / finalize) with no host change.
