# Linked Specify — Native Operator Product, Evaluation, and Lab Composition

> Status: Draft
>
> Owns: the Wasm-free, statically linked operator deployment of the Specify engine; its separation from live-model evaluation and from in-repo lab composition.

## Abstract

Specify's workflow engine and adapter operation implementations are ordinary Rust libraries behind capability traits. Today the shipped operator product composes those libraries as Wasm guests on the Omnia runtime, while `crates/harness` already composes the same engine and adapters directly for native tests, development commands, and live-model evaluation.

This RFC makes that second path a **peer operator product**: the native, statically linked deployment. It extracts the host into a crate named `linked` (library exposing the product command entry), narrows live-model evaluation into a lab-only library named `eval`, and keeps each repository's fixture/first-party **lab** composition in a package named `workbench`. The first-party shippable fat binary is composed in `specify-adapters` over that host — workbench is never the install path.

```text
OPERATOR PRODUCTS                              LAB ONLY
┌──────────────┐  ┌───────────────────┐       ┌────────────┐  ┌──────────┐
│ specify      │  │ linked            │       │ workbench  │  │ eval     │
│ Wasm product │  │ Native product    │       │ in-repo    │  │ trials / │
│              │  │ host (this repo)  │       │ composition│  │ scenarios│
│ ship/install │  │ + fat bin*        │       │ unpublished│  │ lab lib  │
└──────────────┘  └───────────────────┘       └────────────┘  └──────────┘

* First-party fat binary composed/released from specify-adapters over the host.
```

`linked` does not depend on `workbench`, `eval`, or `fixture`. Lab tools and tests depend on `linked`. The Wasm deployment remains authoritative for component loading, WIT conformance, isolation, and adapter-store behavior; linked tests never claim those properties.

## Motivation

The current package names describe their first consumers rather than their architectural roles:

- `crates/harness` contains the native provider, adapter catalog, engine-to-adapter conversion, Cursor model bridge, MCP reference host, typed invocation helper, command router, test environment support, live evaluation trials, scenarios, grading, telemetry, and sandbox management.
- `crates/fixture` contains deterministic source and target adapters, scripted model answers, a request-recording model decorator, and native project sessions.
- `crates/eval` is the concrete native composition binary over the fixture adapters.

This makes a coherent native **operator product** look like an accumulation of testing crates. It also obscures the correspondence between the two deployments. `src/lib.rs` and `src/provider.rs` are adapter-agnostic: adapters reach the Wasm deployment at composition time through WIT imports. The linked analog of that pair is the host (`Provider`, catalog dispatch, router assembly); the linked analog of the deployment manifest is the binary's `adapters!` declaration:

```text
Dynamic Wasm deployment                         Static linked deployment

src/lib.rs + src/provider.rs                    linked::Provider<M> + router assembly
deployment manifest composing components        product binary `adapters!` declaration
adapter components                              linked adapter libraries
shipped operator binary: specify                shipped operator binary: linked
```

Evaluation and in-repo fixture/first-party scratch binaries are consumers of that product host — not the product itself.

## Goals

1. Establish the linked deployment as a first-class **shippable native operator product**, peer to Wasm `specify`: Specify owns the `linked` host library and product command entry; the first-party fat binary is composed where the adapters live.
2. Keep the linked host out of the workflow core; workflow crates depend only on capability traits.
3. Preserve dependency inversion: `linked` never depends on `eval`, `workbench`, `fixture`, or `augentic/specify-adapters`.
4. Keep Specify's integration tests self-contained over local `fixture` adapters via `linked`'s library API.
5. Let downstream repositories compose their own linked binaries (product or lab) without Specify depending back on them.
6. Separate the operator command surface from live-model evaluation: command mode lives in `linked`; `eval` is optional and lab-only.
7. Model-backend selection belongs to the composition root (product `main` or lab entry), not to provider dispatch.
8. Preserve current command, test, scenario, and evaluation behavior while changing ownership and names, with two declared exceptions: the live reference listener fails loudly instead of silently skipping, and the model-override variable loses its `EVAL` name.
9. Keep one implementation of provider dispatch, model bridging, adapter registration, and MCP reference serving.
10. Demote `workbench` to unpublished in-repo lab composition (fixture or first-party adapters plus eval UX) — not the answer to "how do I ship native Specify?"

## Non-goals

- Replacing the shipped Wasm deployment.
- Loading `.wasm` components from the linked application.
- Claiming WIT, component ABI, isolation, digest, or adapter-store coverage from linked-deployment tests.
- Moving first-party adapter implementations into the Specify repository.
- Making Specify depend on the sibling `specify-adapters` checkout.
- Changing workflow semantics, artifact schemas, lifecycle transitions, prompts, or adapter operation traits.
- Adding a compatibility alias for the old crate names; this is an internal pre-1.0 workspace refactor.
- Widening workflow APIs solely to support tests.
- Serving the HTTP transport from the linked deployment (natural later extension; out of scope).
- Defining the full release-pipeline matrix for the linked binary in this RFC (artifact naming and CI attachment are follow-on; the architecture must make that attachment possible).

## Terminology

- **Workflow core** — the deployment-neutral engine crates: `project`, `slice`, `change`, and `transport`, plus their dependency leaves.
- **Wasm deployment / `specify`** — the shipped Omnia-hosted operator product: native host process + workflow and adapter Wasm guests. Authoritative for component loading, WIT, isolation, digests, and the adapter store.
- **Linked deployment / `linked`** — the shipped native operator product: workflow core and adapter libraries compiled into one process and connected through Rust capability traits. Peer to `specify`, not a harness.
- **Linked host** — the reusable library surface of the `linked` package: provider, catalog, model bridge, MCP, command entry, process cache isolation.
- **Evaluation framework (`eval`)** — lab-only trials, scenarios, grading, telemetry, and sandbox orchestration.
- **Workbench** — unpublished in-repo lab composition: declares adapters and runs the shared eval/dev UX. Not an operator product. Specify's workbench binds `fixture`; the adapter repository's binds first-party adapters.
- **Fixture adapters** — deterministic local implementations in `crates/fixture`; concrete SDK-native adapters, not mocks of the workflow seam.

## Decision

### Mental model

```text
OPERATOR PRODUCTS (two deployments of the same engine)
┌──────────────────────────┐     ┌──────────────────────────┐
│ specify                  │     │ linked                   │
│ Wasm deployment          │     │ Native deployment        │
│                          │     │                          │
│ omnia runtime + guests   │     │ host library +           │
│ WIT provider             │     │ product command entry    │
│ component adapters       │     │ Provider + Catalog       │
│ adapter-store / digests  │     │ convert / MCP / command  │
│ ship / install / release │     │ ship / install / release*│
└──────────────────────────┘     └────────────┬─────────────┘
                                              │
                         library API of the same package
                              ┌───────────────┼───────────────┐
                              │               │               │
                              ▼               ▼               ▼
                  ┌────────────────┐ ┌────────────────┐ ┌────────────────┐
                  │ Integration    │ │ workbench      │ │ First-party    │
                  │ tests          │ │ (lab only)     │ │ product bin*   │
                  │ scripted Model │ │ adapters!      │ │ (adapters repo)│
                  │ fixture bind   │ │ + eval::entry  │ │ adapters!      │
                  │ no eval        │ └───────┬────────┘ │ linked::command│
                  └────────────────┘         │          │ no eval        │
                                             ▼          └────────────────┘
                                     ┌────────────────┐
                                     │ eval (lab lib) │
                                     │ trials /       │
                                     │ scenarios      │
                                     └────────────────┘

* First-party fat binary ships from the adapters repo over this host library;
  see [Adapter composition and shipping](#adapter-composition-and-shipping).
```

Dependency direction (Cargo):

```text
workbench ──► linked
workbench ──► eval ──► linked
tests     ──► linked   (+ fixture as needed)

linked  ──✗──► workbench
linked  ──✗──► eval
linked  ──✗──► fixture
linked  ──✗──► specify-adapters
specify ──✗──► specify-adapters
```

### Crate names and ownership

| Target crate | Kind | Responsibility | Source of current code |
| --- | --- | --- | --- |
| `linked` | library | Native operator host: catalog, provider, MCP, `command::run`, model bridge | Native execution modules from `harness` |
| `eval` | library | Lab-only live-model evaluation framework | Evaluation modules from `harness` |
| `workbench` | binary (`publish = false`) | In-repo lab composition (fixture + eval UX) | Current `eval` binary, renamed and demoted |
| `fixture` | library | Deterministic adapters, answers, sessions | Existing `fixture`, retargeted to `linked` |

The shippable first-party **linked fat binary** is not a Specify workspace package — it is composed in `specify-adapters` over this `linked` library (see below).

After migration there is no `harness` package. No crate is named `native`: that word already means `cfg(not(target_arch = "wasm32"))` and "native tests," and the Wasm product's host is itself a native process. `linked` names the deployment's distinguishing property (static linking vs dynamic component composition).

The package name `eval` is freed by the workbench rename and reused for code that evaluates prompts and scenarios. A stale `cargo run -p eval` fails with a missing-binary error rather than doing something else.

### Adapter composition and shipping

Wasm `specify` ships the engine from this repository and loads adapter components dynamically from the store — so the operator binary has no compile-time dependency on `specify-adapters`.

Linked composition is static. Therefore:

1. **Specify owns the linked host** — `crates/linked` as a library (and the product-shaped binary entry APIs: `command::run`, `Provider`, `adapters!`).
2. **Specify's `linked` binary does not bind first-party adapters** — that would require a dependency on `specify-adapters`. Its production dependencies stay free of `fixture`, `eval`, and `workbench` as well.
3. **The first-party native operator product** — the fat binary operators install when they want omnia/vectis/contracts/… linked in-process — is **composed and released from `augentic/specify-adapters`**: depends one-way on Specify's `linked` library, declares first-party `adapters!`, and calls `linked::command` only (no `eval`).
4. **Workbench is never that product** — in both repositories it remains the unpublished lab binary (dev command passthrough + `eval` subcommand).

```text
augentic/specify                         augentic/specify-adapters
────────────────                         ─────────────────────────
specify     ← shipped Wasm product
linked      ← host lib (+ product APIs) ──► linked fat binary
              (no first-party adapters)     (first-party adapters;
                                             shippable native product)
eval        ← lab framework
workbench   ← lab: fixture + eval        workbench ← lab: first-party + eval
```

The released argv0 for the first-party fat binary is a follow-on packaging choice (`linked` vs another operator-facing name). Architecturally it is the linked deployment product; it is not `workbench`.

### Dependency graph (Specify workspace)

```text
augentic/specify

crates/linked                          # library — native product host
  ├── crates/adapter
  └── crates/transport                 (behind the live feature)
        ├── crates/change
        ├── crates/slice
        └── crates/project

crates/eval                            # lab library
  ├── crates/linked
  ├── crates/change
  └── crates/project

crates/workbench                       # lab binary only
  ├── crates/fixture
  │     └── crates/adapter
  ├── crates/linked
  └── crates/eval

# Integration-test targets may dev-depend on linked + fixture.
# No workflow-core crate has a normal dependency on linked, eval,
# fixture, or workbench.
```

Cross-repository:

```text
augentic/specify-adapters
  ├── <first-party product binary>     # linked::command + first-party adapters!
  │     ├── first-party adapter crates
  │     └── augentic/specify::linked   # library only
  └── workbench                        # lab only
        ├── first-party adapter crates
        ├── augentic/specify::linked
        └── augentic/specify::eval
```

The lightweight `checks` package enforces these directions from Cargo manifests, parsed as TOML. One manifest walk rejects: `linked`, `eval`, `fixture`, `workbench`, or the removed `harness` in `[dependencies]` and `[build-dependencies]` of `error`, `diagnostics`, `artifacts`, `adapter`, `project`, `slice`, `change`, and `transport`; rejects `fixture`, `eval`, `workbench`, or any concrete adapter crate in `linked`'s production dependencies; and rejects `fixture` or concrete adapter crates in `eval`. Explicit `[dev-dependencies]` on `linked` and `fixture` remain legal where core integration suites require them. This check absorbs the current `harness/tests/boundary.rs`.

## Architecture

### Workflow core

Unchanged and deployment-neutral:

- handlers implement `omnia_guest::api::operation::Operation<P>`;
- each operation states its minimum capability intersection on `P`;
- orchestrators receive `project::seam::Capabilities` where useful;
- `transport` assembles the typed command and HTTP routers;
- the adapter SDK owns `adapter::Source` and `adapter::Target`.

The workflow core does not know whether capabilities are satisfied by WIT imports, linked Rust implementations, scripted doubles, or a live Cursor backend.

### The `linked` package (operator product host)

`linked` is the native deployment host: a **library** used by tests, workbench, and the shippable first-party product binary. Product `main` functions live in composition roots that depend on this library; they call `linked::command` and declare `adapters!`.

It owns:

- `Catalog<M>` and its typed source/target registration builder;
- the `Binding` hook and the `adapters!` registration macro;
- `Provider<M>`, implementing `Anchor`, `Resolver`, `Hydrator`, `Model`, workflow `Source`, and workflow `Target`;
- adapter-SDK to workflow-seam DTO conversion;
- provider-neutral typed operation invocation;
- **command-router assembly and process command entry** over a caller-supplied model backend;
- the guest-model to host-model bridge;
- the Cursor-backed `Model` implementation behind an optional live/cursor feature;
- ephemeral MCP serving for linked adapters' embedded reference shelves;
- process-scoped project-cache isolation (`env`) used when a linked process must not inherit the operator's global cache location (sandboxes, tests, and isolated product runs).

It does not own:

- fixture adapters or scripted answers;
- live trial definitions, scenarios, grading, or evaluation telemetry;
- the lab `eval` subcommand multiplexer (that stays in `eval` for workbench UX);
- a hard-coded model choice inside provider dispatch — composition roots supply the backend.

#### Command entry (product path)

The operator command surface lives in `linked`, not in `eval`:

```rust
// Product or lab composition root supplies Binding + model factory.
linked::command::run::<B, M>(argv, model_factory).await
```

Central assembly:

```rust
let model = model_factory(&root);
let provider = Provider::bound::<B>(root, model).await?;
let invoker = Invoker::new("specify", provider);
let router = transport::command::router(invoker)?;
let response = router.execute(argv).await;
```

`Provider<M>` stays generic over `omnia_guest::Model`. Native integration tests substitute `fixture::RecordingModel<omnia_testkit::model::Scripted>` without a second provider implementation. Product binaries and `eval` supply `CursorModel::new` (or another backend) at their composition root.

A command-only product binary never depends on `eval`.

#### Features

- **default** — catalog, convert, invoke, `Provider::new` (dependency-light for scripted workflow tests).
- **live** (or split `cli` + `cursor` if a non-Cursor `Model` consumer appears) — transport router, MCP serving, Cursor backend and model bridge. Not default.

Headline extension seam: caller-supplied model factory on `command::run`. The feature split is implementation detail supporting that seam.

#### Adapter catalog

The adapter SDK traits are associated-function traits generic over `P: Model` and are deliberately not object-safe. The catalog remains a typed, monomorphized vtable:

- a composition root registers each source and target adapter type;
- registration captures operation function pointers specialized for the application's model type;
- `Provider<M>` resolves `<axis>:<name>` identities against that catalog;
- workflow `Source` and `Target` calls narrow their DTOs, dispatch the registered operation with `&M`, and widen the result.

This is the linked equivalent of the workflow guest's source/target WIT imports — application composition, not a test mock layer.

#### Model bridge

Workflow and adapter libraries consume `omnia_guest::Model`, while `omnia_cursor::Client` implements the host-side `omnia_wasi_model::WasiModelCtx`. The bridge remains necessary:

- map guest `Request` to the host wire request;
- run the host request gate;
- expose the project root when `lend_workspace` is requested;
- invoke the Cursor backend;
- validate and project the answer back into a guest `Reply`;
- preserve typed model errors.

Rename `DevModel` → `CursorModel` and internal `Native<B>` → `ModelBridge<B>`. Rename the driver-side model-id override from `SPECIFY_EVAL_MODEL` to `SPECIFY_MODEL`: the override belongs to the deployment's Cursor backend (product command passthrough and evaluation alike).

#### MCP references

Adapter judgment requests carry MCP grants for embedded adapter references. A linked deployment must serve the same reference shelves to preserve prompt behavior.

Constructors:

- `Provider::new(root, model, catalog)` — no listener (deterministic tests).
- `Provider::serve_references(self).await?` — start the ephemeral MCP listener; **fails** when no port can bind (no silent shelf stripping).
- `Provider::bound::<B>(root, model).await?` — sugar: build `B`'s catalog, `new`, then `serve_references`.

The previous skip-when-unbound behavior was test-scaffolding tolerance; in an operator product it silently degrades every prompt that needs references.

#### Process cache isolation (`env`)

`env`'s scoped project-cache guard isolates the process-global cache location for sandboxes, tests, and other isolated runs. That is legitimate linked-host process configuration, not evaluation residue. It stays in `linked`.

### The `eval` crate (lab only)

`eval` is a library over `linked` (enabling its live features). It also depends directly on `change` and `project` because trials invoke typed plan operations and sandbox inspection loads workflow state. It owns:

- the multi-step live workflow trial;
- single-operation adapter scenarios;
- deterministic grading;
- model-request telemetry;
- sandbox seeding and cleanup;
- shared evaluation CLI parsing;
- the combined **lab** entry helper that selects command passthrough or the `eval` subcommand for workbench UX.

```rust
pub struct Config {
    /// Binding-owned prompt-scenario root, when the binding supports scenarios.
    pub scenarios: Option<PathBuf>,
    /// Trial and scenario scratch root; defaults to `sandbox/`.
    pub sandbox: PathBuf,
}

/// Lab entry used by workbench only. Not the operator product entry.
pub fn main<B: linked::Binding>(config: Config) -> ExitCode
```

Command passthrough inside this lab entry still delegates to `linked::command` with a model factory — it does not reimplement the router. Product binaries call `linked::command` directly and never go through `eval::main`.

`eval` is generic over `linked::Binding`. It never selects concrete adapters and never depends on `fixture` or first-party adapter crates.

Evaluation provider composition remains:

```text
linked::Provider<Telemetry<CursorModel>>
```

### Workbench (lab only)

`crates/workbench` is the unpublished in-repo lab binary. Its body is the fixture binding plus one lab entry call:

```rust
fn main() -> std::process::ExitCode {
    eval::entry::main::<Adapters>(eval::entry::Config::default())
}

linked::adapters! {
    Adapters {
        source fixture::Docs,
        source fixture::Code,
        target fixture::Adapter,
    }
}
```

- ordinary arguments run a Specify command through `linked` (via the lab entry);
- the `eval` subcommand runs the shared live trial.

This binary is the target of `cargo make dev` and `cargo make eval`. It is **not** the shipped native operator product; do not document "build workbench" as how to ship or install linked Specify.

Integration tests assemble `linked::Provider` with scripted model answers directly — they do not require workbench.

### Adapter-repository composition

`augentic/specify-adapters` consumes Specify's `adapter`, `linked`, and (for lab only) `eval` crates one-way.

| Binary | Role | Entry | Adapters |
| --- | --- | --- | --- |
| First-party **product** binary | Shippable native operator product | `linked::command` + model factory | first-party `adapters!` |
| `workbench` | Lab only (`publish = false`) | `eval::entry::main` | first-party `adapters!` + scenarios root |

Wasm composed tests and the change example remain separate gates.

## Wasm and linked correspondence

| Concern | Wasm (`specify`) | Linked (`linked`) |
| --- | --- | --- |
| Operator product | shipped `specify` binary | shipped linked fat binary (first-party: adapters repo) |
| Workflow composition | `src/lib.rs` + `src/provider.rs` | `linked::Provider<M>` + router assembly |
| Adapter selection | component identity + deployment manifest | product binary `adapters!` |
| Engine invocation | `Invoker` + `transport` router | same |
| Model access | `omnia:model/completion` host import | composition-root `Model` (Cursor via bridge) |
| Adapter dispatch | WIT source/target imports | typed linked `Catalog<M>` |
| References | adapter HTTP guest routed by Omnia | ephemeral linked MCP listener |
| Project tree | shared Wasm preopen | native project path |
| Isolation | component instance per call | one native process |
| Lab composition | n/a | unpublished `workbench` + `eval` |

Observable behavior the linked product must preserve: command I/O, exit codes, artifact writes, lifecycle transitions, adapter operation order, model request/answer schema, MCP reference contents, report validation.

It does not preserve or test: component ABI, WIT mapping, Wasm isolation, instance-per-call behavior, dynamic component hydration, global adapter-store resolution, pinned digest verification, deployment-manifest link configuration. Those remain owned by adapter crate tests, composed-deployment tests, and the operator-invoked Wasm change example.

## Testing model

Native testing is a consumer of the linked **library**, not a reason the product exists — and not routed through workbench.

### Workflow integration tests

```text
linked::Provider<fixture::RecordingModel<omnia_testkit::model::Scripted>>
  ├── fixture adapter catalog
  ├── temporary project root
  └── scripted model answers
```

Invoke public operations through `linked::invoke::run` or the transport router.

### Substrate / product-host tests

Catalog registration, provider dispatch, command routing, MCP serving, and model bridging live under `crates/linked/tests/`.

### Evaluation tests

Scenario loading, grading, telemetry, and trial argument handling live under `crates/eval/tests/`.

### Fixture tests

Deterministic fixture behavior, answer recording, and the exhaustive fixture inventory remain in `crates/fixture/tests/`.

Rename `fixture::model::Harness<B>` → `fixture::RecordingModel<B>`. Rename the sibling repository's `testkit::Harness` copy the same way in the coordination stage. A later move upstream to `omnia-testkit` remains optional.

### Wasm boundary tests

No linked test claims Wasm coverage. Existing component gates remain: adapters `composed` tests, the Specify fixture change example, and the first-party change example over the published core component.

## Module migration

| Current module | Target | Notes |
| --- | --- | --- |
| `catalog.rs` | `crates/linked/src/catalog.rs` | `Binding`, catalog builder, `adapters!` |
| `convert.rs` | `crates/linked/src/convert.rs` | SDK/workflow DTO mapping |
| `env.rs` | `crates/linked/src/env.rs` | Process cache isolation |
| `invoke.rs` | `crates/linked/src/invoke.rs` | Typed operation invocation |
| `provider.rs` | `crates/linked/src/provider.rs` | Generic provider; loud `serve_references` / `bound` |
| `command.rs` | `crates/linked/src/command.rs` | Model factory from caller; live feature |
| `mcp.rs` | `crates/linked/src/mcp.rs` | Ephemeral reference shelves; live feature |
| `model.rs` | `crates/linked/src/cursor_model.rs` | `CursorModel`; `SPECIFY_MODEL`; live/cursor feature |
| `native.rs` | `crates/linked/src/model_bridge.rs` | `ModelBridge<B>`; live/cursor feature |
| `entry.rs` | `crates/eval/src/entry.rs` | Lab command/eval multiplexer; `entry::Config` |
| `fs.rs` | `crates/eval/src/fs.rs` | Evaluation tree-copy |
| `grade.rs` | `crates/eval/src/grade.rs` | Grading |
| `sandbox.rs` | `crates/eval/src/sandbox.rs` | Sandbox; root from `Config` |
| `scenario.rs` | `crates/eval/src/scenario.rs` | Prompt scenarios |
| `telemetry.rs` | `crates/eval/src/telemetry.rs` | Model request counts |
| `trial.rs` | `crates/eval/src/trial.rs` | Live workflow trial |

Current `crates/eval` becomes `crates/workbench` (lab). User-facing strings that name the "native harness" or "native shim" name the linked deployment / linked host instead.

## Test migration

- Move `catalog.rs`, `provider.rs`, `command.rs`, and `mcp.rs` to `crates/linked/tests`.
- Move `grade.rs` and `scenario.rs` to `crates/eval/tests`.
- Delete `boundary.rs` in favor of the `checks` manifest invariant.
- Workflow suites: `harness` → `linked`; `fixture::Session` unchanged in role.
- Rename `RecordingModel` across both repositories.

## Cargo and feature layout

### Specify workspace

```toml
eval = { path = "crates/eval" }
linked = { path = "crates/linked" }
```

Remove the `harness` workspace-dependency entry. `workbench` needs no workspace-dependency entry.

`linked` default stays dependency-light; live features pull transport, MCP, and Cursor/omnia host deps.

`eval` enables `linked`'s live features and names `change` / `project` directly.

`workbench` (lab):

```toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
eval.workspace = true
fixture.workspace = true
linked.workspace = true
```

`workbench` is `publish = false` and never attached to a release.

The Wasm release surface (`--bin specify`, core guest, adapter contract) remains. Attaching a linked fat binary to releases is follow-on work in the adapters (and possibly Specify) release pipelines; this RFC requires that architecture not block it.

### Adapter workspace

Replace:

```toml
harness = { git = "https://github.com/augentic/specify.git" }
```

with:

```toml
adapter = { git = "https://github.com/augentic/specify.git" }
eval = { git = "https://github.com/augentic/specify.git" }
linked = { git = "https://github.com/augentic/specify.git" }
```

- Rename current `crates/eval` → `crates/workbench` (lab; move `scenarios/` with it).
- Add (or rename toward) the **first-party product binary** that depends on `linked` (library) + first-party adapters and calls `linked::command` only.
- Update sibling path patches. No dependency aliases required.

## Command surface

Operator-facing Specify verbs do not change.

Development tasks retain their names but target the lab binary:

- `cargo make dev -- ARGS` → workbench command passthrough (via `eval::entry` → `linked::command`);
- `cargo make eval` → workbench `eval` subcommand;
- `cargo make change-run` → Wasm composed example.

Package selection for lab tasks: `cargo run -p workbench`. Installing or releasing the native operator product is **not** `cargo install` of workbench.

## Implementation plan

### Stage 1 — Extract the linked host

1. Add `crates/linked` and move `catalog`, `convert`, `env`, `invoke`, `provider`, `command`, `mcp`, `model`, and `native` from `harness`.
2. Rename `DevModel` → `CursorModel`, `Native<B>` → `ModelBridge<B>`, `SPECIFY_EVAL_MODEL` → `SPECIFY_MODEL`.
3. Parameterize `command::run` over a caller-supplied model factory; keep Cursor out of provider dispatch.
4. Add `serve_references` (fallible); make `bound` use it; fail loud on bind failure.
5. Update user-facing "native harness" / "shim" strings.
6. Move host tests into `crates/linked/tests`; retarget `fixture` and workflow tests to `linked`.
7. Rename `fixture::model::Harness` → `RecordingModel`.
8. Run linked and affected workflow suites.

### Stage 2 — Lab binary rename

1. `git mv crates/eval crates/workbench`; update package name and description (`publish = false`, lab-only docs).
2. Point `cargo make dev` / `cargo make eval` at `-p workbench`.

### Stage 3 — Evaluation library

1. Create `crates/eval` from `entry`, `fs`, `grade`, `sandbox`, `scenario`, `telemetry`, and `trial`.
2. Lab entry delegates command mode to `linked::command`; introduce `entry::Config`.
3. Keep evaluation generic over `linked::Binding`; no fixture/adapter deps.
4. Retarget workbench to `eval::entry::main`.
5. Remove emptied `crates/harness`.

### Stage 4 — Adapter repository

1. Pin Specify revision exposing `adapter`, `linked`, and `eval`.
2. Rename lab `eval` → `workbench`; wire `eval::entry` + first-party `adapters!`.
3. Introduce the first-party **product** binary over `linked::command` (no eval).
4. Rename `testkit::Harness` → `RecordingModel`.
5. Confirm no reverse dependency from Specify to `specify-adapters`.

### Stage 5 — Documentation and checks

Update both repositories' `AGENTS.md`, adapter `TESTING.md`, Specify testing/architecture standards, quality-gate docs, Makefiles, and rustdoc.

Add a linked-deployment section to the architecture document that states:

- `specify` and `linked` are peer operator products (Wasm vs static);
- `workbench` / `eval` are lab-only;
- linked tests never satisfy component/WIT/digest/store gates.

Extend `crates/checks/boundaries.rs` with the manifest invariant under [Dependency direction](#dependency-graph-specify-workspace).

### Stage 6 — Verification

In `augentic/specify`:

```bash
cargo make check
cargo make ci
cargo make dev -- --help
cargo make eval
cargo check --lib -p specify --example change --target wasm32-wasip2
```

In `augentic/specify-adapters`:

```bash
cargo make check
cargo make ci
cargo make dev -- --help
cargo make eval
```

Keep live-model commands operator-invoked when credentials are unavailable.

## Acceptance criteria

1. `linked` is a reusable host library containing no concrete adapter binding, no fixture dependency, no eval dependency, and no workbench dependency.
2. `linked` can run the Specify command router over any `Binding` and any `Model` supplied by the composition root; no path inside `linked` constructs a Cursor backend except behind an optional feature module selected by that root.
3. A command-only product binary can depend on `linked` alone (plus its adapter crates) — it must not require `eval` or `workbench`.
4. `eval` is generic over `linked::Binding` and contains no concrete adapter or fixture dependency; it is documented and packaged as lab-only.
5. `workbench` is unpublished lab composition (Specify: fixture adapters + eval UX) and is never described as the shipped native operator product.
6. Specify's workflow integration tests use `linked` plus local `fixture` adapters and never reference `augentic/specify-adapters`.
7. The sibling repository's first-party **product** binary depends one-way on Specify's `linked` library and calls `linked::command`; its workbench remains lab-only over `eval`.
8. `cargo make dev` / `cargo make eval` preserve lab behavior via workbench in both repositories.
9. The Wasm workflow guest, fixture example guest, component manifests, shipped `specify` runtime behavior, and existing Wasm release surface remain intact.
10. Linked-deployment tests explicitly avoid claiming component ABI, WIT, isolation, or adapter-store coverage.
11. Crate-level tests remain integration-first; no public workflow API is widened solely for the migration.
12. Full local CI passes in both repositories, or unavailable live gates are reported precisely.
13. `checks` enforces the dependency rules in [Dependency graph](#dependency-graph-specify-workspace).
14. Request-recording doubles are exported as `RecordingModel` in both repositories; no type named `Harness` remains.
15. `Provider::serve_references` / `bound` fail when the reference listener cannot start.
16. No user-facing string names the removed `harness` or calls the linked deployment a "shim".
17. Documentation answers "how do I ship/install native Specify?" with the linked operator product — not workbench.

## Risks and mitigations

### Linked behavior is mistaken for Wasm conformance

Mitigation: distinct documentation and commands for linked vs Wasm gates; keep composed-deployment and change-example coverage explicit.

### Operators confuse workbench with the product

Mitigation: workbench stays `publish = false`; docs and acceptance criterion 17 forbid describing it as the install path; product entry is `linked::command`.

### Specifying "linked is the product" while first-party adapters live elsewhere

Mitigation: document the static-composition constraint explicitly — Specify owns the host; the first-party fat binary is composed in `specify-adapters`. Same dependency inversion as Wasm components vs the `specify` binary, different linking time.

### Cursor dependencies leak into ordinary tests

Mitigation: live features are opt-in; default `linked` stays scripted-test suitable.

### Evaluation regains concrete adapter dependencies

Mitigation: every evaluation entry generic over `Binding`; enforce in `checks`.

### A second implementation of adapter dispatch appears

Mitigation: all linked providers use `linked::Catalog` and `linked::Provider`.

### Cross-repository development becomes lockstep

Mitigation: revision-pin `linked` and `eval` like `adapter`; sibling path patch remains co-development only.

### Released argv0 / packaging bikeshed blocks the refactor

Mitigation: this RFC locks architecture and dependency arrows; exact release artifact names and CI attachment are follow-on.

## Alternatives considered

### Keep the current crate names

Rejected. `harness` conflates host, evaluation, and composition; `eval` names the lab binary rather than the evaluation library.

### Treat workbench as the linked deployment / composition root for the product

Rejected. "Build workbench" is the wrong operator intuition for a shippable native product. Workbench is lab-only; the product entry is `linked::command` (and the first-party fat binary in the adapters repo).

### Host the composition root as a feature-gated root `[[bin]]` named `native`

Rejected. Package/binary name collisions, feature gates, release caveats, and split terminology ("native" vs "workbench") for the same role. Correspondence with `src/lib.rs` is drawn at the host layer (`Provider` + router), not by stuffing the adapter manifest into the root package.

### Name the host crate `native`

Rejected. Collides with `cfg(not(target_arch = "wasm32"))` vocabulary; the Wasm host is also a native process. `linked` states the distinguishing property.

### Fold evaluation into `linked` behind a feature

Rejected. Blurs the operator product with lab machinery; `checks` cannot enforce a clean boundary as well as a crate edge. Command-only product binaries must not pull eval.

### Put all shared host code only in workbench binaries

Rejected. Duplicates provider dispatch, bridging, catalog, and MCP across tests and product binaries.

### Put first-party adapters in Specify's linked binary

Rejected. Would create a Specify → `specify-adapters` dependency (or move adapters in-tree). Same inversion rule as today: Specify does not depend on the sibling adapter repository.

### Put Specify's lab workbench in `specify-adapters`

Rejected. Specify's integration tests and fixture lab must work without a sibling checkout.

### Hard-code Cursor into `Provider` or `command::run`

Rejected. `Provider<M>` is the capability-substitution seam for live execution, scripted tests, and telemetry. The composition root supplies the model factory.

### Route operator command mode through `eval::entry`

Rejected for the product path. Lab workbench may use `eval::entry` as UX sugar (delegating to `linked::command`); the shippable product must not depend on `eval`.

### Move `linked` to a separate repository

Rejected. It evolves atomically with workflow and adapter SDK seams; Specify's tests consume it. A workspace package provides separation without extra release cycles for the host library.

## Consequences

- Linked execution is an explicit **operator product**, peer to Wasm `specify`.
- `linked` is the product host library (`command::run` and friends); `eval` and `workbench` are lab-only and unpublished as products.
- Command mode lives in `linked`; evaluation is an optional lab client of that host.
- Specify's core tests remain self-contained over fixture adapters; first-party fat binaries and first-party lab workbench remain downstream and one-way.
- The Wasm deployment remains necessary and authoritative for the component boundary.
- First-party native shipping is composed where the adapters live, over Specify's `linked` library — the static-linking dual of publishing `.wasm` components for the Wasm product.
