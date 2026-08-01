# Emery - Agent Instructions

This repository is **Rust plus embedded prose**: the workspace at the repository root produces the `emery` runtime binary, and the surviving markdown (ultrathin `/emery:*` skill wrappers, reference docs) ships alongside it. Source and target adapter prose lives in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters). Generated Rust crates and Swift shells appear in downstream projects, not in this repository itself.

## Vocabulary

Emery names two adapter roles and three workflow nouns. Use the terms verbatim.

### Adapter roles

- **source adapter** — input role with two operations: `survey` (plan time) and `extract` (slice time). Ships as a single WebAssembly component exporting the WIT `source` interface (one component, no manifest); the guest crate lives at `sources/<name>/` in `augentic/emery-adapters`. Examples: `intent`, `documentation`, `typescript`, `screenshots`, `captures`.
- **target adapter** — output role with three operations: `guidance` (read by core synthesis), `build`, and `merge`. Ships as a single WebAssembly component exporting the WIT `target` interface; the guest crate lives at `targets/<name>/` in `augentic/emery-adapters`. Examples: `omnia`, `vectis`, `contracts`. See [`docs/explanation/adapter-anatomy.md`](docs/explanation/adapter-anatomy.md) for the full source / target contract, including the [adapter-vs-Cursor-plugin manifest boundary](docs/explanation/adapter-anatomy.md#adapter-manifests-vs-cursor-plugin-manifests).
- **plugin** (adapter vocabulary) — operator-facing shorthand for the shared adapter shape, used where source + target authors share the same audience tag. Engine code resolves adapters through the provider-carried `adapter::Resolver` capability. The shipped WASI provider delegates to `adapter::resolver::Component`, which locates the identity's single `.wasm` component and reads its metadata from the component's own `metadata` export (no manifest file, no schema validation).
Do not confuse the adapter `plugin` noun with **Cursor plugins** under `plugins/` (e.g. `plugins/emery/`). Those are the IDE distribution surface for `/emery:*` skill wrappers and marketplace manifests; they are invisible to the `emery` CLI.

### Engine vs workflow

| Term         | Use for                                                                                                                                                                                                      |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **engine**   | This product / repository: the engine guest (`emery:engine`), engine crates (`project` / `slice` / `change` / `transport`), lifecycle ownership, and the seam opposite adapters                            |
| **workflow** | The operator loop `plan → execute → finalize` (and the per-slice `refine → build → merge` rhythm inside execute), plus the behavioral contract in [`docs/standards/workflow.md`](docs/standards/workflow.md) |

The WIT world remains named `workflow` (wire identity); prose calls the guest the **engine guest**.

### Synthesis terms

- **lead** — slice-sized unit emitted by `survey`; one raw, unmerged block per lead under `## Lead inventory` in `discovery.md`, each identified by its `(source, lead)` pair (`lead` is unique only within a `source`).
- **evidence** — per-source result of `extract`; structured document with `claims:` persisted to `.emery/slices/<slice>/evidence/<source>.yaml`.
- **provenance** — the sources behind one requirement (the `Sources:` list in `spec.md`).
- **conflict / divergence** — unresolvable vs authority-resolved disagreement; surfaced inline as `[conflict]` / `[divergence]` tags on requirement headers.
- **authority** — closed enum (`intent` > `documentation` > `behaviour`) controlling who wins a disagreement.
- **model.yaml** — the single structured slice artifact at `.emery/slices/<slice>/model.yaml`, carrying provenance **inline** on each requirement. The provenance audit view is **projected on demand** by `emery slice provenance` — there is no persisted `provenance.yaml`. Audit-only; `spec.md` is the authoritative artifact. See [`docs/reference/provenance.md`](docs/reference/provenance.md) for the projected shape and audit posture.
- **component catalog** — operator-curated file at `.emery/design-system/components.yaml` declaring shared UI components (`status: confirmed | rejected`). The Vectis target reads the catalog at build time and factors shared component code per shell tree. Follows the same pattern as `tokens.yaml` and `assets.yaml`. Opt-in; absent catalog means no component factoring. Validated by `emery slice validate` (`slice-catalog-drift`) and the vectis adapter's in-guest composition validation (catalog cross-reference check). See [docs/explanation/components.md](docs/explanation/components.md).

### Workflow nouns

- **slice** — the single unit that flows through the fixed `refine → build → merge` loop. Each slice has its own proposal, spec, design, tasks, and merge step. Lives at `.emery/slices/<name>/`. Driven by `/emery:refine`, `/emery:build`, `/emery:merge`, `/emery:drop` and the `emery slice *` CLI verbs.
- **change** — the operator-defined umbrella that coordinates one or more slices through `change.md` + `plan.yaml`. Driven by `/emery:plan`, `emery plan execute`, `/emery:finalize` and the `emery plan *` CLI verbs. `change` is on-disk vocabulary, not a slash-command namespace.

Use *slice loop* for the per-slice lifecycle; reserve *change* for the on-disk umbrella that owns `change.md` and `plan.yaml`.

### Workspace topology (disambiguation)

The word **workspace** overloads two related concepts. Use them verbatim:

| Term               | Meaning                                                                                                            |
| ------------------ | ------------------------------------------------------------------------------------------------------------------ |
| **Workspace**      | Registry-only platform repo: `workspace: true` in `project.yaml`, `registry.yaml`, plan artifacts at the repo root |
| **Workspace slot** | Materialised peer at top-level `workspace/<project>/`                                                              |

`/emery:init workspace` and `emery init --workspace` scaffold a workspace. Slot materialisation and publication are operator-owned outside Emery.

### Workflow, standards, and artifacts

Emery separates three concerns. Use the terms verbatim; see [docs/explanation/standards-layer.md](docs/explanation/standards-layer.md) for the full picture.

| Layer                     | Role                                          | Examples                                                                                            |
| ------------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| **Workflow**              | Phase orchestration and lifecycle transitions | `/emery:plan`, `emery plan execute`, `emery plan undo`                                     |
| **Artifacts**             | Slice-local and baseline product intent       | `spec.md`, `plan.yaml`, `.emery/specs/`                                                           |
| **Engineering standards** | Durable policy that outlives any slice        | Rules under `codex/rules/` and per-adapter `prose/rules/` overlays, embedded in each target adapter |

**Authoring standards** (`docs/standards/`) govern docs house style and the thin skill-wrapper shape; Developer Guide links are enforced in CI by `mdbook-linkcheck2` (`cargo make links` / `mdbook build docs`, config in `docs/book.toml`). The rest — including skill-wrapper shape — is applied in review. **Engineering standards** (rules in `augentic/emery-adapters` — `codex/rules/universal/` and per-adapter `prose/rules/` overlays, embedded in each target adapter's component and served by its references server) govern generated and hand-written code in consumer projects. Do not conflate them.

Engineering standards reach consumer projects through the target adapters' embedded prose, applied by their build review prompts — there is no engine-side lint or rules-export surface. Build-time `REVIEW.md` and plan Gate 1 `approved` are separate surfaces.

### Artifact authority and boundaries

When inputs disagree, use this precedence order:

1. Emery artifacts (`proposal.md`, `spec.md`, `design.md`, `tasks.md`; Vectis also owns `tokens.yaml` and `assets.yaml`, while `composition.yaml` is a target build output)
2. Guest orchestrations and embedded prompts (`crates/slice/prompts/`, `crates/change/prompts/`, and the CLI single-writer contract)
3. `docs/reference/`, `docs/standards/`, adapter prompts, and adapter-local references
4. `SKILL.md` wrappers, which may only elicit arguments, invoke one CLI verb, and relay its output
5. Source Evidence
6. Model inference

Artifacts override source behavior. When the authoritative inputs are incomplete, preserve the gap as `[unknown]` rather than guessing. Keep behavioral requirements platform-neutral in `spec.md`; target-specific implementation detail belongs in `design.md`, adapter prompts, and adapter references. See [artifact responsibilities](docs/explanation/artifacts.md) and the [specialist-versus-artifact boundary](docs/explanation/augentic-emery-usage.md).

### Authority and reconciliation mechanics

The headline rules:

- **Authority resolution order** — per-slice override → Evidence document-level `authority:` → conflict. (A per-Evidence per-kind override is deferred to a future RFC.) See [`crates/slice/prompts/synthesis/authority.md`](crates/slice/prompts/synthesis/authority.md) for the resolution order and override surface.
- **`captures` source adapter** — consumes runtime capture trees and emits `kind: example` Evidence claims with `replay-digest: sha256:…` anchors and default `authority: behaviour`.
- **Authority-override authoring** — `emery plan amend --authority-override <slice> <kind>=<key>`; orphan source keys are rejected by `emery slice validate` with `slice-authority-override-orphan-source`.
- **Reconciliation checks** — `emery slice validate` catches spec-vs-model staleness and orphan contributing claims; provenance is carried inline in `model.yaml` so there is no separate file to drift.
- **Extraction is agent-only and never cached** — `survey` / `extract` re-run the prompt every time; there is no extraction-result cache.

## Workflow overview

The default rhythm is `/emery:plan` → operator runs `emery plan execute` (its first run stamps `approved` — Gate 1) → `/emery:finalize`; `/emery:execute` carries the middle step behind an explicit Gate 1 confirmation. The operator surface, in the order it appears in a project's life:

- `/emery:init` — scaffold `.emery/`, run once per project.
- `/emery:plan` — wrap the guest-routed `emery plan author`: survey each bound source, reconcile leads into `slices[]`, author the Gate 1 prose, validate. Exits at `plan.lifecycle: pending` and prints the literal `emery plan execute` command. An existing `plan.yaml` requires operator confirmation and `--force`, which recreates the plan unconditionally.
- `emery plan execute` — the guest-routed drained loop and **Gate 1**: invoking it on a `pending` plan is the approval act — it stamps `pending → approved` (idempotent, journaling `plan.transition.approved` with the closed `--actor` enum, default `operator`), then runs refine → build → merge per entry until every per-entry `status` is `done` or a stop condition halts it. `/emery:plan` never runs it; `/emery:execute` wraps it behind an explicit operator confirmation.
- `/emery:refine` — breakout: wrap `emery slice refine` for one slice (extract per bound source, synthesis, validation, the `refined` transition).
- `/emery:build` — breakout: wrap `emery slice build` for one slice.
- `/emery:merge` — breakout: wrap `emery slice merge` for one slice; the only writer of per-entry `done`.
- `/emery:drop` — abandon a slice without merging (`emery slice drop`).
- `/emery:finalize` — run `emery plan archive` after operator-owned publication and merge.

N=1 is degenerate, not special: `intent.survey` produces one lead, the operator runs `emery plan execute` (stamping `approved`), and the loop drives the same single-slice rhythm as a 12-slice change.

## Skill / CLI responsibility split

Phase skills are ultrathin invoke-and-relay wrappers: each elicits any missing arguments, invokes the corresponding `emery` command, and relays its output verbatim. Everything else — manifest validation, `metadata.yaml` reads and writes, plan and slice lifecycle transitions, source and target resolution, artifact-completion checks, baseline conflict detection, delta merge, archive move, and the judgment legs (survey, extract, synthesis, target build) — runs through the `emery` CLI and its guest orchestrations. No skill body carries orchestration, synthesis, or validation prose.

The CLI surface skills depend on is documented in [`emery` `--help`](cli). The headline groups: `emery init` (with the re-entry flag `--upgrade`, which bumps the `emery` pin and re-scaffolds preservation-safe files only, and `--platforms <csv>`, which declares the project's target platform set — required when the target adapter declares `platforms.required`), `emery adapter {add, update}` (`add` — pre-init, axis-neutral seeding of a local `.wasm` component into the project component cache; `update` — the explicit update act refreshing a bare name to the newest published version), `emery source {resolve, survey, extract}` and `emery target {resolve}` (the adapter debug/breakout surface), `emery slice {list, refine, model show, build, validate, provenance, merge, drop}`, `emery plan {author, execute, add, amend, remove, undo, next, status, archive}` (`plan status` is the read-only next-action projection — `refine|build|merge <slice>` / `stop <reason>` / `drained` — over plan entries, slice metadata, and the journal tail), `emery archive {prune}` (retention-policy GC over the prunable slice/plan archive), and `emery journal show` (the read-only `--filter`/`--limit` projection over the journal; every journal write is an orchestration side effect — there is no emit verb). `emery source survey`/`extract` resolve `<source>` against `plan.yaml.sources.<key>` and run the bound source adapter's compiled-in prompt through one guest orchestration each (source extraction is agent-only). `emery slice build <slice>` is the guest-routed target-build verb: the orchestration assembles + schema-validates the build request, emits `target.execution.agent`, drives the target build prompts, validates the report, and owns the `built` transition, journaling `slice.build.started` / `.succeeded` / `.failed`. `emery slice merge` dispatches the bound target's phased merge gates (WIT `merge-phase`: `preflight` before the deterministic commit, `postflight` after it), schema-gates and persists each gate's report (including a failed postflight report beside the archive), and fires `slice.merge.started`, then `slice.merge.succeeded` / `slice.merge.failed` — or `slice.merge.postflight-failed` when the postflight gate fails after the commit (non-rollback, the merge stands; `emery plan execute` stops with `merge-postflight-failed` and re-running execute emits `plan.merge-postflight.acknowledged` to continue) — alongside the durable `slice.archive.created`.

Never hand-edit `metadata.yaml`, `project.yaml`, `plan.yaml`, `discovery.md`, `sources.yaml`, or `targets.yaml`; never `mkdir -p .emery/...`; never `mv` anything into `.emery/archive/`. Route through the CLI — it enforces the legal lifecycle set and validates inputs in one place for humans, agents, and CI.

## Contracts target adapter

The contracts target adapter owns API contract authoring, import, and validation. Its build operation runs the OpenAPI, AsyncAPI, and JSON Schema format sub-flows, each with author / import / verify references under `targets/contracts/prose/references/` in `augentic/emery-adapters`.

The matching validation surface is the contracts adapter's in-guest validator, run by the target build and merge orchestrations.

## Vectis asset materialization

Vectis-bound projects commit per-platform exports under `design-system/assets/exports/`; shell writers render by `assets.yaml` entry `kind` — never substitute platform glyphs for `vector` / `raster` ids at build time.

| Concern                          | Where                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Materialize + export conventions | In-guest vectis asset materialization — codified in the vectis core's decisions in [`emery-adapters`](https://github.com/augentic/emery-adapters)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Build-prelude auto-materialize   | The vectis guest's build prelude — runs automatically inside the guest-routed `emery slice build`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Render-by-`kind` review rule     | [`targets/vectis/prose/rules/VECTIS-006-asset-render-by-kind.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/rules/VECTIS-006-asset-render-by-kind.md)                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Writer / integration contracts   | [`targets/vectis/prose/references/ios/design-system-integration.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/references/ios/design-system-integration.md), [`android/design-system-integration.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/references/android/design-system-integration.md), [`prompts/build/ios/write.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/prompts/build/ios/write.md), [`prompts/build/android/write.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/prompts/build/android/write.md) |

## Plan-driven loop

`/emery:plan` authors the plan and exits at Gate 1; the operator approves by running `emery plan execute` (directly or through `/emery:execute`'s confirmation gate) — its first run stamps `approved`, then drives the loop; `/emery:finalize` closes it. Plan *entries* are only ever written via `emery plan add` / `emery plan amend`; plan *lifecycle* is only ever written by the first `emery plan execute`; per-entry `in-progress` is only ever written by `emery plan next`; per-entry `done` is only ever written by `emery slice merge`. Per-entry status walks backwards only via `emery plan undo <entry>`, which refuses to skip rungs (`done → in-progress`, then a second call for `in-progress → pending`) and fires one `plan.transition.undone` journal event per rung. The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Hand-driven fallback: `emery plan next` → `/emery:refine` → `/emery:build` → `/emery:merge`, repeat until drained.

## Testing Philosophy

Emery strictly enforces an **aggressive integration-first posture**. 

- **Design against the public surface:** before adding a unit test, ask whether integration can reach the behavior — reachable through a CLI input or `pub` fn, observable at a public boundary (stdout JSON, exit code, filesystem), and affordable to assert there without a subprocess explosion. If yes, write the integration test; the unit test is redundant.
- **Default to Deletion:** a `src` unit test survives only when it is reachable and observable but cheap *only* in-process against a **private** kernel (a proptest or dense matrix), or covers a genuinely CLI-unreachable branch. If the kernel is already `pub`, re-home the test to `crates/<name>/tests/` instead.
- **Crate-Level Integration:** Put tests in `crates/<name>/tests/` via the crate's public API. Wire-contract coverage lives in `crates/transport/tests/`.
- **Widening is a last resort:** do NOT alter public APIs simply to support integration tests — prefer collapse-and-keep. The target is *near-zero* unit tests (no redundant or integration-reachable ones), not literal zero. `cargo llvm-cov nextest` remains the brake that ensures coverage holds during migrations; adapter posture is enforced in `augentic/emery-adapters` by the WIT contract plus each adapter crate's `tests/` suite and that repo's operator-invoked wasm example.

The test surface is two rungs: native integration tests over `crates/mock`'s adapter catalog, the offline `crates/native` provider, and scripted model doubles (`cargo make test`, per push), and one native prompt-evaluation rung over a live model (`cargo make eval <case>`, explicit; the `crates/probe` library's typed case runner plus the shared cursor client, composed by the root `eval` example — case data under `examples/eval/cases/`, retained sandboxes under `sandbox/<case>/`). There is no automated WASM boundary rung: the real component seam is exercised by the operator-invoked wasm example (`cargo make wasm-run`, live model; see [`examples/wasm/README.md`](examples/wasm/README.md)) and by the matching wasm example in `augentic/emery-adapters`. `omnia-testkit` owns generic model/scripted/runtime test mechanics; Emery's `mock` crate owns the mock adapter, catalog registry, session helpers, and answer corpus; the suites own workflow scenario semantics and assertions. See [`docs/standards/testing.md`](docs/standards/testing.md) and [`docs/contributing/quality-gates.md`](docs/contributing/quality-gates.md).

## Commands

All commands are run from the repository root:

- `cargo make links` — Developer Guide link integrity via `mdbook build docs` (`mdbook-linkcheck2` in `docs/book.toml`).
- `make ci` — the full local gate: `cargo make ci` (the Rust workspace, `Makefile.toml` at the repo root), which includes the links gate.

Per-push CI is the shared org workflow (nextest, clippy, doc, doctest, vet, and deny over the whole workspace); no sibling checkout is required — the engine embeds no adapter-authored prose. There is no per-push WASM gate; the wasm32 guests compile-check locally with `cargo check --lib -p emery --examples --target wasm32-wasip2`, and boundary execution is the operator-invoked wasm example. See [docs/contributing/quality-gates.md](docs/contributing/quality-gates.md) for the gate model.

The eight `/emery:*` skills are ultrathin invoke-and-relay wrappers (see [Skill / CLI responsibility split](#skill--cli-responsibility-split)). Skill-wrapper body style is guidance in [docs/standards/cli-contract.md](docs/standards/cli-contract.md). Local Cursor preview: `cursor-agent --plugin-dir plugins/<name>` (see [docs/contributing/operator-plugins.md](docs/contributing/operator-plugins.md)).

## Gotchas

- In a fresh clone, run `/emery:init` before using other `/emery:*` commands. The workflow skills expect the `.emery/` project structure to exist.
- `cargo make links` (`mdbook build docs`) enforces Developer Guide link integrity — if you rename docs paths, update links in the same change.
- **Adapter names are unique across axes** — a name appears under `sources/<name>/` xor `targets/<name>/`, never both. The store carries no axis segment, so a colliding name would make a binding's axis ambiguous; the `<axis>:<name>` adapter-id routing at the metadata/dispatch seam is the enforcement point.
- **First-party adapters resolve local-first** — `emery init <adapter>` accepts a package reference (`emery:omnia@1.0.0`) or the first-party shorthand (`omnia`, `omnia@1.0.0`). A semver pin installs automatically on first use: the native launcher pulls the published Wasm OCI artifact from the compiled first-party mapping (`ghcr.io/augentic/emery-adapters/<name>:<version>`) into the global single-file store (`<store-root>/<name>@<version>.wasm`) — pull-on-miss applies to every command that resolves a pin, not just init, and no project configuration can redirect the mapping. A bare name persists bare (`project.yaml` / `plan.yaml` carry no auto-pinned version) and resolves local-first at every dispatch: the seeded project-cache entry when present (`<project-cache>/components/<name>.wasm`, populated by `emery adapter add <path.wasm>` — pre-init, axis-neutral — or a local `.wasm` component at init), else the newest installed store version (offline, no registry consultation), else — only when nothing local exists — the launcher lists the registry's exact-SemVer tags and installs the newest (pull-latest provisioning). `emery adapter update <name>` (and `emery init` / `emery init --upgrade`) is the explicit update act: it forces the registry check and installs a newer version when one is published. Cache hits always win, so the co-dev seed is never shadowed by a published component; the launcher logs every settled identity (host version + adapter version + origin) to stderr. There is no sibling-checkout or build-tree probe — an adapter built elsewhere reaches the project through `adapter add`, a local component at init, or a pinned install. GitHub URLs are refused (`adapter-github-uri-unsupported`).
- Target review prompts in `augentic/emery-adapters` symlink `agent-teams.md` from each adapter's `references/` directory to that repo's shared `codex/references/runtime/review-team-protocol.md` overlay, forked from the canonical `docs/reference/review-team-protocol.md` here. If the canonical document is removed, the adapter overlays break — keep the file when changing review-team prose.
- Crossing a major is a hard cut: no silent compatibility aliases for old manifests, verbs, prompt paths, or slash-namespaces, and no migration framework. Pre-1.0, a major bump means re-init — `emery init --upgrade` bumps the pin over an existing project; anything deeper is a fresh `emery init`.

## Related coding standards

- The external Rust baseline is the [Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/guidelines/index.html); [docs/standards/](docs/standards/) carries only the house deltas, project contracts, and explicit overrides layered on top (overrides win).
- CLI binary and crate conventions (errors, DTOs, hint colocation, brevity) live in [the Rust workspace section below](#the-rust-workspace-emery-cli) and [docs/standards/](docs/standards/). Skills that shell out to `emery` rely on the kebab-case `error` discriminants documented there.
- Markdown changes follow [documentation authoring standards](docs/standards/doc-authoring.md). Do not hard-wrap prose solely for column width; preserve semantically meaningful breaks in frontmatter, tables, lists, blockquotes, and fenced code.

## The Rust workspace (`emery` CLI) {#the-rust-workspace-emery-cli}

The repository root is a Rust workspace. It produces the `emery` runtime binary that the workflow skills shell out to. Generated Rust crates and Swift shells produced by the workflow live in downstream consumer repositories; this workspace owns the deterministic CLI primitives those workflows compose.

### Crate graph

The workspace is leaf → root. `error` is the dependency leaf and depends on no other workspace crate. Each package publishes to crates.io as `emery-<crate>` and is referenced that way in `[workspace.dependencies]`; short `use` paths come from each crate's `[lib] name` (the root binary and the `guest` / `launcher` / `mock` / `probe` crates stay `publish = false`).

```text
error                    # leaf — thiserror + serde-saphyr only
diagnostics              # dependency-light leaf (the neutral Diagnostic substrate: report, fingerprint, blocking — plus diagnostics::digest, SHA-256 hex via sha2 + base16ct, and diagnostics::cache)
artifacts                # depends on {error,diagnostics} (artifact types + parsers: spec, task, evidence, discovery; shared atomic writer; artifacts::validate artifact rule registry — NOT on the engine crates or anything named lint)
adapter                  # the adapter SDK (leaf over omnia-guest, no workspace-crate deps) — the per-axis operations traits (adapter::Source / adapter::Target), the WIT package and the source!/target! wasm export macros, seam DTOs, judgment/answer scaffolding, and the embedded prose registry; implemented by the mock crate here and by the first-party adapters in augentic/emery-adapters
project                  # foundation — depends on {error,diagnostics,artifacts,omnia-guest}: init (+ project::agents — init-time AGENTS.md scaffold), adapter resolution, config/Layout, journal, registry, the plan data model (Plan/Entry/Status/Lifecycle + transitions, doctor, the propose kernel), the slice data model (metadata/lifecycle/outcome), the seam capability traits + build wire DTOs (project::seam), the judgment kernel (repaired, MAX_REPAIRS), and the shared handler plumbing (project::handler: Anchor, Ctx, Render, ReportBody, the operation-layer Error); operation families in handlers submodules: journal::handlers, registry::handlers, adapter::handlers, init::handlers
slice                    # the slice loop — depends on project: refine/build/merge orchestration (slice::orchestrate incl. the extract half of the source axis), synthesis + the synthesize judgment leg, validation, provenance, the delta-merge engine, slice::handlers (the emery slice operations) + slice::source (source extract), and its own prompts/ corpus (synthesize.md + synthesis/*)
change                   # the change loop — depends on {project,slice}: plan author/execute orchestration (change::orchestrate incl. the survey half of the source axis and workspace routing), the propose judgment leg, change::plan::handlers (the emery plan operations) + change::source (source survey), and its own prompts/ corpus (propose.md)
transport                # wasm-clean transport assembly — explicit typed command/HTTP routers over Invoker, exhaustive Args-to-Input TryFrom conversions, projectors, exit contract, and the launcher-facing anchoring projection (transport::command::selectors — argv parsed through the shared grammar; only the `adapter add` seed request projects, everything else runs in-guest); depends on {project,slice,change}
launcher                 # native-only deployment-policy crate (publish = false) — the macro-facing mount and resolver expressions over one per-process anchoring (launcher::Policy: project-root walk, --project-dir override, Locations::from_env captured once, writable mount dirs created pre-run, the read-only self-named `adapter add` seed preopen, the per-invocation pre-bound HTTP listener (`launcher::http_listener` — split bind policy, any bind failure is a startup failure, its address injected as the guest-visible `HTTP_ADDR`) feeding the `/mcp/<axis>/<name>` `http_paths` hook (`launcher::mcp_route`) onto routed adapter ids; the global store is host-owned with no guest mount) plus the fail-closed adapters-only omnia::GuestResolver over the one captured ExecutionPaths: pinned routed ids resolve the store with pull-on-miss install (launcher::install — anonymous OCI pull from the compiled first-party GHCR constant ghcr.io/augentic/emery-adapters, layer/digest/wasm-magic validation, atomic store write, sidecar recording tree digest + OCI repository + manifest digest), bare routed ids resolve local-first (cache seed, else newest store version, else pull-latest provisioning via launcher::install::resolve_latest — the registry's newest exact-SemVer tag; the argv-derived refresh set from `adapter update` / `init` forces the registry check) with every settled identity logged to stderr (adapter-sidecar-missing / adapter-digest-mismatch / adapter-not-found / adapter-install-failed / adapter-install-invalid / adapter-latest-failed / adapter-latest-none) — the engine is embedded in the binary; the launcher is the only downloader in the deployment; depends on {project,transport}
prose                    # build-dependency crate — embed-time prompt-corpus walk + link check generating each crate's DOCS table
native                   # the native host — the validated adapter Catalog over the SDK operations traits, DynModel type erasure, the non-generic seam Provider (Anchor/Resolver/Model/Source/Target), native reference hosting, and cli-gated asynchronous command execution; depends on {adapter,project,transport,…} and never on a concrete adapter, mock, probe, or Cursor crate
guest                    # the engine guest as a library (wasm32-only) — the `workflow`-world WIT bindings, the WIT-backed seam Provider (Anchor/Resolver/Model/Source/Target over the world's imports), and the guest::export! macro wiring the shared typed transport routers onto wasi:cli/run + wasi:http/incoming-handler; depends on {project,slice,transport,…}; invoked by the root emery cdylib and by the wasm example guest in augentic/emery-adapters
mock                     # dev-only mock crate (publish = false) — the canonical SDK-native mock adapter core (mock::behaviour over the seam DTOs), the typed operations-trait implementors and exhaustive catalog registry (mock::registry + mock::catalog()), the scripted answer corpus, the host-only Session helpers over the native provider (binding `omnia_testkit::model::Harness`), and the mock::invoke test-suite entry; dev-dep'd (legally cyclically) by the engine/transport suites, the example adapter components, and the root eval composition example
probe                    # lab-only library (publish = false) — the typed eval case runner (probe::case: Workflow / Build cases over real emery verbs, stable retained sandboxes, gates), grading, telemetry, sandbox helpers (no runtime/catalog/Cursor); feature = "client": the shared cursor composition (probe::client — DevModel, the console + optional EVAL_LOG file tracing init, argv dispatch) consumed by each repo's eval composition example (examples/eval/ here and in augentic/emery-adapters); target of `cargo make lab` and `cargo make eval` via the root example
emery (root crate) # Omnia deployment unit under src/: guest cdylib (wasm32, one guest::export!() over the guest crate, versioned with the binary) + the shipped runtime (one omnia::runtime! invocation embedding the engine bytes via build.rs's EMERY_WASM, mounts/resolver as launcher expressions) + the examples/wasm adapter components (adapter::source!/target! over mock::Adapter)
```

The artifact validation rule registry lives in `artifacts::validate`: `artifacts` depends on none of the engine crates, so a rule cannot transition a slice or stamp a plan. `artifacts` is the lifecycle-free leaf holding the artifact types and parsers the engine layer reads. The neutral `Diagnostic` / `DiagnosticReport` substrate lives in the `diagnostics` crate alongside the shared `diagnostics::digest` SHA-256 helpers, so every check producer — validate and review alike — emits the same finding currency without importing the other surface's code. Engineering-standards rules ship inside the target adapters in `augentic/emery-adapters`; there is no engine-side rules crate.

Modules of note across the workspace (engine layer):

- `crates/project/src/platform.rs` — closed `Platform` enum (`Core | Ios | Android | Web | Desktop`, `#[serde(rename_all = "kebab-case")]`) representing the set of target platforms a project may declare in `project.yaml`. `Core` is mandatory in every set. `Ios` and `Android` have scaffold/build/verify support; `Web` and `Desktop` are type-system placeholders for future functionality. Includes `Display`, `FromStr`, and `parse_platforms_csv` for the `--platforms` CLI flag.
- `crates/project/src/adapter/` — the deployment-neutral `Resolver` capability (`resolve_*` plus the async `ensure_source` / `ensure_target` provisioning legs) over the closed `AdapterSelector` enum (`Bare { name }` | `Package { namespace, name, version }` | `Component { path }`), plus the shipped component-deployment implementation: `selector.rs` (the shared parse grammar — `omnia@1.0.0` is package-reference sugar for `emery:omnia@1.0.0`, bare `omnia` is the unpinned shorthand, a local `.wasm` path parses as a component selector, GitHub URLs are refused with `adapter-github-uri-unsupported`, a package reference missing an exact SemVer pin raises `adapter-package-ref-version-required`), `resolver::Component` (read-only resolution — package pins *and* bare cache-miss names dispatch the WIT `metadata` export by routed id *before* any guest-visible component file exists, so the host resolver faults the component in during that dispatch: pull-on-miss install for a pin, local-first resolution for a bare name), and `ensure.rs` (the provisioning kernels behind the WASI provider's ensure: local-component mirroring into `<project-cache>/components/<name>.wasm` with a re-ensure that resolves through the mirror after the original file is removed; package pins and bare names provision nothing in-guest — installation is host-owned in the launcher). A pinned identity is backed by the global single-file store entry (`<store-root>/<name>@<version>.wasm`; host-side verify-on-read against the recorded byte digest, `adapter-digest-mismatch` on drift, `adapter-sidecar-missing` when unverifiable); a bare name resolves the seeded project component cache when an entry exists (`<project-cache>/components/<name>.wasm`, populated by `emery adapter add` or a local component at init, provenance in a per-component `<name>.meta.yaml` sidecar) and otherwise dispatches the unversioned routed id, which the deployment resolves local-first (newest store version, else pull-latest); a persisted component selector resolves only the cache mirror; there is no sibling-checkout or build-tree probe. Both roots derive from one carried `project::handler::Locations` value — production layout is `$EMERY_HOME` (else `$HOME/.emery`, else `<temp>/emery`) with `store/` and `cache/` beneath it, captured once at each composition root; kernels never read `std::env`. A component-selector cache miss is `adapter-not-found` with an `adapter add` hint (the native host's ensure fails as `adapter-not-linked` instead, and never performs component I/O). An unpinned resolve carries no package identity: resolved versions are `Option<semver::Version>` (`None` off the wire) and the origin is labeled `cache` (seeded entry) or `store` with the routed id as reference (bare dispatch-first — the version settles host-side and is logged to stderr). Metadata (the `emery` host-CLI compatibility floor, a target's `inputs[]` and `PlatformsCapability`) comes from the component's deterministic `metadata` export through a runner passed explicitly to `resolver::Component`; answers are cached against the component file's SHA-256 in a `<component>.metadata.json` sidecar. There is no process-global resolver or metadata registration. A binding on the wrong axis fails at the dispatch seam (no deployed guest exports the requested `<axis>:<name>` id); an unparseable floor is `adapter-floor-malformed`; a floor newer than the running binary raises `Error::AdapterCliTooOld` (`adapter-cli-too-old`) on the exit-3 `EXIT_VERSION_TOO_OLD` path. Resolved values carry an opaque `Origin` (`label`, display `reference`), so engine code does not enumerate deployment mechanisms. The closed `SourceOperation` / `TargetOperation` enums in `adapter/operation.rs` are the typed per-axis operation sets derived from the WIT contract. Execution paths (`project::handler::ExecutionPaths` — canonical project root plus explicit or inherited cache parent) travel with every ensure/resolve call, so cache isolation is provider configuration, not process-global environment.
- `crates/native/` — the native host (see the crate graph above): compile-time `AdapterIdentity { name, version }` on each SDK implementor replaces component/store identity; native ensure is a static catalog match (bare names resolve to the entry's actual version; exact pins succeed only on a published exact compiled identity — `0.0.0` development placeholders stay bare-only; mismatches, unlinked names, and component selectors fail as `adapter-not-linked`).
- `crates/artifacts/src/spec/provenance.rs` — `spec.md` requirement-block parser (`ID:` / `Sources:` / `Status:` lines, closed `RequirementStatus` enum, inline `[…]` tag coherence).
- `crates/project/src/plan/propose.rs` — plan-time lead-reconciliation kernel. Envelope DTOs (closed `kind: request | response`), the pure `build_request` / `build_catalog` / `resolve_topology` assembly, and the `Plan::propose_from` projection kernel driven by the guest `plan author` orchestration.
- `crates/slice/src/build/` — target build envelope kernel. The closed-shape `BuildRequest` / `BuildReport` DTOs live in `project::seam::wire` (the typed serde parse is the envelope gate), alongside `BuildOutput` (`{ platform: Platform, path }` — the optional per-platform build outputs declared in `BuildReport.outputs[]`) and the `BuildReport::enforce_no_blocking` / `BuildReport::enforce_outputs_exist` gates; `assemble.rs` assembles a request from the bound target adapter's declared `inputs[]` against the slice tree (raising `target-build-input-missing`). The guest `orchestrate::build` orchestration (behind the guest-routed `emery slice build <slice>`) owns request assembly, report validation, the `target-build-*` aborts (including `target-build-output-missing` for absent/empty output paths), the `slice.build.*` events, and the `built` transition gate.
- `crates/project/src/journal.rs` — newline-delimited JSON journal event log at `<project_dir>/.emery/journal.jsonl`; closed `Event` / `EventKind` taxonomy with kebab-case wire ids and `snake_case` Rust variants joined by `#[serde(rename = "…")]`, including the single `PlanReconcileCompleted` variant covering a successful `plan author` write, `plan.entry.advanced`, and the closed `Actor` enum (`operator | agent`) on `plan.transition.approved`.
- `crates/project/src/answers.rs` + `crates/slice/src/answers.rs` — the generated judgment-answer schemas (`leads`, `evidence`, `report`, `proposal`; `synthesis` in `slice`), produced via `schemars` from the same Rust wire types the deterministic tails parse. The committed goldens under `crates/project/answers/` and `crates/slice/answers/` are parity-gated by each crate's `tests/answers.rs` (regenerate with `REGENERATE_GOLDENS=1`); adapters in `augentic/emery-adapters` vendor the `leads` / `evidence` / `report` documents. There is no other JSON Schema machinery — the typed serde parse is the load gate for every on-disk artifact.
- `crates/diagnostics/` — the neutral `Diagnostic` substrate: the `Diagnostic` / `DiagnosticReport` / `DiagnosticSummary` types with the orthogonal `source` (`deterministic | model-assisted | hybrid | human | tool`) and `kind` (`violation | review`) axes, the fingerprint algorithm, and the `blocking` predicate. Import it from the `diagnostics` crate root.
- **No lint engine, no `Check` substrate.** Repo consistency is the mdBook links gate (`cargo make links`). Contributor model: [docs/contributing/quality-gates.md](./docs/contributing/quality-gates.md).
- `crates/project/src/agents.rs` — crate-private init-time `AGENTS.md` scaffold (short fenced template + `.emery/context.lock`). `emery init` generates them when `AGENTS.md` is absent; skips inside materialised workspace slots.

The two **adapter validators** (`contract`, `vectis`) are in-guest adapter library code inside each adapter's published component in `augentic/emery-adapters` — the host dispatches no adapter WASI tool. Crux shell presence and launcher-icon heuristics live **only** in the vectis adapter's in-guest core: the host performs no plan-time shell detection.

### Exit codes

Part of the CLI wire contract. `Exit::from(&Error)` in [`crates/transport/src/command/output.rs`](./crates/transport/src/command/output.rs) is the single source of truth.

### Repository map

```text
src/main.rs              shipped binary — one omnia::runtime! invocation: embedded engine bytes, launcher mount/resolver expressions, cursor backends
src/lib.rs               wasm32 engine guest cdylib — one guest::export!() over crates/guest
crates/guest/            the engine guest library — workflow-world WIT bindings, the WIT-backed Provider, and the guest::export! macro
crates/launcher/         native-only deployment policy — anchoring, mount + resolver macro expressions, fail-closed adapters-only GuestResolver (verify-and-load)
crates/transport/         shared command/HTTP routing, clap grammar, conversions, projectors, exit contract, and the launcher seed-request projection
crates/adapter/          the adapter SDK — per-axis operations traits, WIT package + wasm export macros, seam DTOs, embedded prose registry
crates/project/          foundation — init, adapter resolution, config, journal, registry, plan + slice data models, seam traits, judgment kernel
crates/slice/            the slice loop — refine/build/merge orchestration, synthesis, validation, merge engine, prompts
crates/change/           the change loop — plan author/execute orchestration, plan operations, prompts
crates/mock/             dev-only mock crate — SDK-native mock adapter core, catalog registry, answer corpus, session helpers
crates/native/           the native host — validated adapter catalog, DynModel, seam provider, reference hosting, cli-gated command execution
crates/probe/            lab-only library — the typed eval case runner plus the shared cursor client (feature = "client")
examples/                wasm (Omnia-hosted component seam: mock adapter components + embedded engine, resolver-dynamic adapters — no omnia.toml) and eval (native mock-catalog composition behind cargo make lab / eval)
```

| Code | Name                     | When                                                                  |
| ---- | ------------------------ | --------------------------------------------------------------------- |
| 0    | `EXIT_SUCCESS`           | Command succeeded.                                                    |
| 1    | `EXIT_GENERIC_FAILURE`   | Any `Error` variant not listed below (I/O, YAML, schema, merge, …).   |
| 2    | `EXIT_VALIDATION_FAILED` | Validation findings, `Error::Validation`, `Error::Argument`.          |
| 3    | `EXIT_VERSION_TOO_OLD`   | `Error::CliTooOld` — `project.yaml.emery` is newer than the binary. |

### Documentation map

| Topic                                                                                                                                                                                 | Document                                                                                                                                    |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Cross-cutting code-quality rules (naming, error variants, traits-for-testability, archaeology)                                                                                        | [`docs/standards/style.md`](./docs/standards/style.md)                                                                                      |
| Lints, comments, brevity, DTOs, YAML/atomic writes, module layout (`<module>.rs` + `<module>/`, no `mod.rs` outside `tests/`)                                                         | [`docs/standards/coding-standards.md`](./docs/standards/coding-standards.md)                                                                |
| `Operation`, `Ctx`, `Render`, projectors, exit-code mapping, typed router contract                                                                                                    | [`docs/standards/handler-shape.md`](./docs/standards/handler-shape.md)                                                                      |
| Workspace layout, WASI carve-outs, `Layout<'a>`, time injection, atomic-write rationale, workflow domain modules, supply chain                                                        | [`docs/standards/architecture.md`](./docs/standards/architecture.md)                                                                        |
| `cargo nextest`, integration-first policy, golden files, `REGENERATE_GOLDENS`                                                                                                         | [`docs/standards/testing.md`](./docs/standards/testing.md)                                                                                  |
| Standing architectural decisions (error layering, exit codes, atomic writes, YAML library, wire compatibility, workflow type renames, plan lifecycle, adapter loader, journal events) | [`docs/standards/`](./docs/standards/) (per-area docs) and git history                                                                      |
| Engineering standards layer (adapter-embedded rules)                                                                                                                                  | [`docs/explanation/standards-layer.md`](./docs/explanation/standards-layer.md)                                                              |
| Vectis asset materialization                                                                                                                                                          | In-guest build prelude and asset-domain policy live in the vectis core (`augentic/emery-adapters`); the engine dispatches no prepare hook |

### Rust quality {#rust-quality}

**Aggressive Integration-First Posture:**
Emery mandates an aggressive integration-first test strategy. Agents must actively work to remove unit tests (`#[cfg(test)]`) in favor of crate-level (`crates/<name>/tests/`) and wire-contract integration (`crates/transport/tests/`).
- **Design against the public surface first:** before adding a unit test, ask whether integration can reach the behavior — is it reachable through a CLI input or a `pub` fn, is its effect observable at a public boundary (stdout JSON, exit code, filesystem), and is that affordable without a subprocess-pool explosion? If yes, write the integration test; the unit test is redundant.
- **Default to deletion:** a `src` unit test survives only when it is reachable and observable but cheap *only* in-process against a **private** kernel (a proptest or dense matrix), or it covers a genuinely CLI-unreachable branch. If the kernel is already `pub`, relocate the test to `crates/<name>/tests/` rather than leaving it in `src`.
- **Do NOT widen public APIs to test a private kernel.** Widening trades durable surface stability for coverage you already have; prefer collapse-and-keep. The target is *near-zero* `src` unit tests — no redundant or integration-reachable ones — not literal zero. Use `cargo llvm-cov nextest` to prove coverage holds when removing unit tests.
- **Push crate-specific tests down:** Crate-specific logic must be tested in `crates/<name>/tests/` via the crate's public API; wire-contract coverage lives in `crates/transport/tests/`.

Read [style.md](./docs/standards/style.md), [coding-standards.md](./docs/standards/coding-standards.md), and [testing.md § Test naming](./docs/standards/testing.md#test-naming) before adding types, suppressions, or tests. Run `cargo make ci` (not bare `cargo test` — CI uses `RUSTFLAGS=-Dwarnings`).

**Naming:** The module path is context — `registry::show`, not `show_registry`. Test function names are short identifiers; put the narrative in the test body ([testing.md](./docs/standards/testing.md#test-naming)).

**Lint suppressions:** Refactor first. Use `#[expect(lint, reason = "…")]` at the smallest scope. `#![allow]` only at module root when the lint applies to every item below and the reason is contract-locked. Prefer `#[expect]` with a reason over bare `#[allow]`.

| Do not                                                    | Do instead                                                       | See                                                                                                           |
| --------------------------------------------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `#[allow]` / `#[expect]` before trying a split or extract | Extract helper or submodule; suppress only if contract-locked    | [coding-standards § Lint suppression](./docs/standards/coding-standards.md#lint-suppression-posture)          |
| `trait Foo` + sole `RealFoo` for tests                    | `CmdRunner`, `AtomicYaml`, or filesystem/tempdir                 | [style.md § No traits for testability](./docs/standards/style.md#no-traits-for-testability-alone)             |
| `*RenderInput` wrapper for `Render`                       | `Render` on domain type or `ctx.emit_with` closure               | [style.md § One body per command](./docs/standards/style.md#one-body-per-command-no-wrapper-newtype)          |
| Transport formatting inside operations                    | Return a typed `Serialize + Render` body; project at the router  | [handler-shape.md](./docs/standards/handler-shape.md)                                                         |
| RFC/Phase/migration history in comments                   | ≤ 3 lines “what today”; history stays in git                     | [style.md § No archaeology](./docs/standards/style.md#no-archaeology-in-code)                                 |
| Sentence-length test fn names                             | Short name + `mod` grouping                                      | [testing.md § Test naming](./docs/standards/testing.md#test-naming)                                           |
| Add a `src` `#[cfg(test)]` for CLI-reachable behavior     | Exercise it through the public surface in `crates/<name>/tests/` | [testing.md § minimize the unit layer](./docs/standards/testing.md#the-three-layers--minimize-the-unit-layer) |
| Nested `struct Body` inside `fn`                          | Top-level `*Body` + `From` impl                                  | [coding-standards § DTOs](./docs/standards/coding-standards.md#dtos)                                          |
| New `Error::Diag` for one-off shapes                      | Typed variant after ≥3 identical call sites                      | [style.md § Error variants](./docs/standards/style.md#error-variants-budgeted-by-recovery-not-source)         |

External references:

- [Vocabulary](#vocabulary) at the top of this file — workflow vocabulary (slice / change), adapter `plugin` vs Cursor `plugins/`, plan-driven loop.
- [`docs/standards/workflow.md`](./docs/standards/workflow.md) — the in-force workflow contract this binary implements. Defines the `source` / `target` / `plugin` / `axis` vocabulary, the kebab-case wire format, the `Source` / `Lead` / `Evidence` / `Slice` implementation types, writer ownership, and the CLI surface. Stable `§`-anchors that source comments and adapter prompts cite by name.
- [`docs/release.md`](./docs/release.md) — tagging and the platform-binary release pipeline.
- [`crates/project/answers/`](./crates/project/answers/) and [`crates/slice/answers/`](./crates/slice/answers/) — the committed judgment-answer schema goldens, generated from the Rust wire types by `project::answers` / `slice::answers`; the workflow contract pins each wire shape through those types. There are no other JSON Schema files and no adapter-manifest schemas — adapter metadata is the WIT `metadata` record returned by `metadata`.

### Quick toolchain

All driven by `cargo make` (see [`Makefile.toml`](./Makefile.toml)). Run the full local CI suite before committing; do not rely on narrower substitutes such as `cargo test` or `cargo clippy`.

```bash
cargo make ci             # fmt + lint + test + test-docs + doc + links + vet + deny
cargo make check          # fmt + lint + test + test-docs + doc (the pre-commit subset; `cargo make fmt` fixes formatting)
cargo make links          # Developer Guide link integrity (`mdbook build docs`)
cargo make test           # cargo nextest run --locked --workspace --all-features --no-tests=pass, under -Dwarnings
cargo make wasm-run     # the end-to-end wasm example over the WASM seam; operator-invoked, needs CURSOR_API_KEY in examples/.env
cargo make eval <case>    # the live-model prompt-evaluation rung (one eval case; bare lists them); operator-invoked, needs cursor-agent credentials
cargo make lab -- ARGS # any emery verb through the native mock lab shim (the root eval example's command mode)
cargo make lint           # cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo make fmt            # nightly cargo fmt --all
cargo make audit          # cargo-audit; cargo make deny / outdated / deps / vet for the rest
```

Example-local tasks live in [`examples/Makefile.toml`](./examples/Makefile.toml).

### When working in the Rust workspace

1. Read the matching [`docs/standards/`](./docs/standards/) doc before changing error layering, exit codes, atomic writes, the YAML library, the JSON envelope shape, the workflow type names (`Target*` / `Plugin` / `SliceSourceBinding` / `Divergence`), the plan lifecycle (`pending | approved`), the journal event taxonomy, the per-axis cache layout, or adding a new workspace crate.
2. For any Rust change, consult [`docs/standards/`](./docs/standards/) — at minimum the doc that matches the area you are editing, plus [`style.md`](./docs/standards/style.md) for cross-cutting rules.
3. Run `cargo make ci` before committing. If it cannot run, say exactly why and which checks were run instead.
4. When you remove a symbol, `rg <SymbolName> -- AGENTS.md docs/` and update every hit in the same PR.
5. If you touch `Slice.target`, `SliceSourceBinding`, `Divergence`, `crates/artifacts/src/spec/provenance.rs`, `crates/project/src/adapter/`, `crates/project/src/plan/propose.rs`, `crates/project/src/journal.rs`, or `crates/diagnostics/src/`: `rg <symbol>` across the whole repo — Rust *and* prose (`plugins/`, `docs/`, `codex/`) — and the sibling [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters) checkout, and update every hit in the same PR (workflow §"Note to the implementing agent" applies — the workflow contract spans both repos).
6. A fresh contributor should be able to reach any rule from this spine in three hops or fewer. If you find yourself adding prose here that isn't navigational, it belongs in one of the standards docs.
7. For Rust changes, skim [Rust quality](#rust-quality) before adding types, suppressions, or tests; if you add `#[expect]`, state in the PR why a refactor was infeasible.
