# Linked Specify — Statically Linked Deployment, Evaluation, and Composition Boundaries

> Status: Draft
>
> Owns: the Wasm-free, statically linked deployment of the Specify engine and adapter libraries; the separation of linked execution, evaluation support, and concrete application composition.

## Abstract

Specify's workflow engine and adapter operation implementations are ordinary Rust libraries behind capability traits. The shipped application composes those libraries as Wasm guests on the Omnia runtime, while the current `harness` crate already composes the same engine and adapters directly for native tests, development commands, and live-model evaluation.

This RFC makes that second path explicit as a **linked deployment** rather than treating it as test scaffolding. It extracts the reusable deployment substrate into a crate named `linked`, narrows the shared live-model evaluation framework into a library crate named `eval`, and renames each repository's concrete composition binary to `workbench` — one package name, one shape, in both repositories.

Specify's workbench links only the local deterministic `fixture` adapters. It never depends on `augentic/specify-adapters`. The sibling adapter repository owns its own workbench that links the first-party adapters, depending one-way on Specify's `adapter`, `linked`, and `eval` crates.

The shipped Wasm deployment remains authoritative for component loading, WIT conformance, isolation, and adapter-store behavior. Linked execution is a first-class local deployment for development, integration tests, and prompt evaluation; it is not evidence that the Wasm boundary works.

## Motivation

The current package names describe their first consumers rather than their architectural roles:

- `crates/harness` contains the native provider, adapter catalog, engine-to-adapter conversion, Cursor model bridge, MCP reference host, typed invocation helper, command router, test environment support, live evaluation trials, scenarios, grading, telemetry, and sandbox management.
- `crates/fixture` contains deterministic source and target adapters, scripted model answers, a request-recording model decorator, and native project sessions.
- `crates/eval` is the concrete native composition binary over the fixture adapters.

This makes a coherent native application look like an accumulation of testing crates. It also obscures the correspondence between the two deployments — a correspondence that must be drawn at the right level. `src/lib.rs` and `src/provider.rs` are adapter-agnostic: adapters reach the Wasm deployment at composition time, through WIT imports resolved by a deployment manifest. The linked analog of that pair is the generic substrate (provider, catalog dispatch, router assembly), and the linked analog of the deployment manifest is the concrete binary's adapter declaration:

```text
Dynamic Wasm deployment                         Static linked deployment

src/lib.rs + src/provider.rs                    linked::Provider<M> + router assembly
deployment manifest composing components        workbench `adapters!` declaration
adapter components                              linked adapter libraries
```

The linked path should be understood and maintained as a deployment of the same engine, with tests and evaluation layered on top of it — and with concrete composition roots living beside the engine, exactly as deployment manifests do.

## Goals

1. Establish a first-class linked deployment of Specify with no Wasm guest or component runtime.
2. Keep the reusable substrate in the Specify workspace without making it part of the workflow core.
3. Preserve dependency inversion: workflow crates depend only on capability traits, never on the linked deployment.
4. Keep Specify's integration tests self-contained by composing only the local `fixture` adapters.
5. Let downstream repositories compose their own linked applications without the Specify repository depending back on them.
6. Separate linked execution from live-model evaluation and from concrete adapter selection — including model-backend selection, which belongs to the composition root end to end.
7. Preserve the current command, test, scenario, and evaluation behavior while changing ownership and names, with two declared exceptions: the live reference listener fails loudly instead of silently skipping, and the model-override variable loses its `EVAL` name.
8. Keep one implementation of provider dispatch, model bridging, adapter registration, and MCP reference serving.
9. Give both repositories the same composition shape: one `workbench` package declaring linked adapters, everything else shared.

## Non-goals

- Replacing the shipped Wasm deployment.
- Loading `.wasm` components from the linked application.
- Claiming WIT, component ABI, isolation, digest, or adapter-store coverage from linked-deployment tests.
- Moving first-party adapter implementations into the Specify repository.
- Making Specify depend on the sibling `specify-adapters` checkout.
- Changing workflow semantics, artifact schemas, lifecycle transitions, prompts, or adapter operation traits.
- Adding a compatibility alias for the old crate names; this is an internal pre-1.0 workspace refactor.
- Widening workflow APIs solely to support tests.
- Serving the HTTP transport from the linked deployment. The typed HTTP router exists and axum is already a substrate dependency, so this is a natural later extension; it is out of scope here.

## Terminology

- **Workflow core** — the deployment-neutral engine crates: `project`, `slice`, `change`, and `transport`, plus their dependency leaves.
- **Wasm deployment** — the shipped native Omnia host running the workflow and adapter Wasm guests. That host is itself a native process: what distinguishes the two deployments is not nativeness but *static Rust linking versus dynamic component composition*.
- **Linked deployment** — the workflow core and adapter libraries compiled into one native process and connected through Rust capability traits.
- **Linked substrate** — the reusable provider, adapter catalog, model bridge, MCP host, and invocation machinery in the `linked` crate.
- **Evaluation framework** — reusable trials, scenarios, grading, telemetry, and sandbox orchestration in the `eval` crate.
- **Workbench** — a concrete composition root: the package named `workbench` in each repository, declaring which adapters are linked and delegating its entrypoint to the shared framework. Specify's workbench binds the fixture adapters; the adapter repository's binds the first-party adapters.
- **Fixture adapters** — deterministic local implementations in `crates/fixture`; they are concrete SDK-native adapters, not mocks of the workflow seam.

## Decision

### Crate names and ownership

The current `harness` and `eval` responsibilities are split and renamed as follows:

| Target crate | Kind    | Responsibility                                           | Source of current code                                    |
| ------------ | ------- | -------------------------------------------------------- | --------------------------------------------------------- |
| `linked`     | library | Generic linked-deployment substrate                      | Native execution modules from current `harness`           |
| `eval`       | library | Shared live-model evaluation framework                   | Evaluation modules from current `harness`                 |
| `workbench`  | binary  | Concrete fixture-adapter linked application              | Current `eval` binary, renamed                            |
| `fixture`    | library | Deterministic adapters, answer corpus, scripted sessions | Existing `fixture`, retargeted from `harness` to `linked` |

The package name `eval` is freed by the workbench rename and reused for the code that actually evaluates prompts and scenarios. (A stale `cargo run -p eval` fails with a missing-binary error rather than doing something else.) After the migration there is no `harness` package in the Specify workspace, and no crate is named `native`: that word already means `cfg(not(target_arch = "wasm32"))`, "native tests", and "native-only" throughout both repositories, and the shipped deployment's host is a native process too. `linked` states the deployment's actual distinguishing property and stays greppable.

The sibling `augentic/specify-adapters` workspace mirrors the shape:

- its current `eval` binary becomes `workbench` (its `scenarios/` root moves with the package);
- that workbench links the first-party source and target adapter crates;
- it consumes Specify's `adapter`, `linked`, and `eval` crates as one-way dependencies.

### Dependency direction

The linked crates are leaves over the workflow core:

```text
augentic/specify

crates/workbench
  ├── crates/fixture
  │     └── crates/adapter
  ├── crates/linked
  │     ├── crates/adapter
  │     └── crates/transport        (behind the `cli` feature)
  │           ├── crates/change
  │           ├── crates/slice
  │           └── crates/project
  └── crates/eval
        ├── crates/linked
        ├── crates/change
        └── crates/project
```

No workflow-core crate has a normal dependency on `linked`, `eval`, `fixture`, or `workbench`. Integration-test targets may dev-depend on `linked` and `fixture`.

The cross-repository direction remains:

```text
augentic/specify-adapters
  └── workbench
        ├── first-party adapter crates
        ├── augentic/specify::linked
        └── augentic/specify::eval

augentic/specify
  ── no dependency on augentic/specify-adapters
```

This avoids the circular dependency that would result if Specify's own native tests selected first-party adapters from the sibling repository.

The lightweight `checks` package enforces these directions from Cargo manifests, parsed as TOML (the `toml` dev-dependency is already present) rather than substring-matched. One manifest walk rejects: `linked`, `eval`, `fixture`, `workbench`, or the removed `harness` in `[dependencies]` and `[build-dependencies]` of `error`, `diagnostics`, `artifacts`, `adapter`, `project`, `slice`, `change`, and `transport`; and `fixture` or any concrete adapter crate anywhere in `linked` and `eval`. Explicit `[dev-dependencies]` on `linked` and `fixture` remain legal where core integration suites require them. This check absorbs the current `harness/tests/boundary.rs`.

## Architecture

### Workflow core

The workflow core remains deployment-neutral:

- handlers implement `omnia_guest::api::operation::Operation<P>`;
- each operation states its minimum capability intersection on `P`;
- orchestrators receive `project::seam::Capabilities` where independent model, source, target, and resolver types are useful;
- `transport` assembles the typed command and HTTP routers;
- the adapter SDK owns `adapter::Source` and `adapter::Target`.

The workflow core does not know whether those capabilities are satisfied by WIT imports, linked Rust implementations, scripted doubles, or a live Cursor backend.

### The `linked` crate

`linked` is a generic deployment library. Its public surface is usable by a workbench, an integration test, or a downstream application without importing evaluation machinery.

It owns:

- `Catalog<M>` and its typed source/target registration builder;
- the `Binding` hook and the `adapters!` registration macro;
- `Provider<M>`, implementing `Anchor`, `Resolver`, `Hydrator`, `Model`, workflow `Source`, and workflow `Target`;
- adapter-SDK to workflow-seam DTO conversion;
- provider-neutral typed operation invocation;
- command-router assembly over a caller-supplied model backend;
- the guest-model to host-model bridge;
- the Cursor-backed `Model` implementation behind an optional `cursor` feature;
- ephemeral MCP serving for linked adapters' embedded reference shelves;
- process-scoped project-cache environment support used by sandboxes and tests.

It does not own:

- fixture adapters or scripted answers;
- live trial definitions;
- scenario configuration;
- grading;
- evaluation telemetry;
- a concrete adapter list;
- a model choice — composition roots supply the backend;
- a `main` function.

One acknowledged impurity: `env`'s scoped project-cache guard is sandbox and test support, not deployment configuration. It lives in `linked` because the dependency directions leave it no better home — `fixture` cannot host it without inverting the `fixture → linked` edge, and `eval` cannot host it without giving `fixture` an evaluation dependency.

The central application assembly:

```rust
let model = CursorModel::new(&root);
let provider = Provider::bound::<Adapters>(root, model).await?;
let invoker = Invoker::new("specify", provider);
let router = transport::command::router(invoker)?;
let response = router.execute(argv).await;
```

`Provider<M>` stays generic over `omnia_guest::Model`, and the command entry receives its model from the caller — `linked::command::run::<B, M>(argv, factory)` with a `fn(&Path) -> M` factory — so Cursor is selected by the composition root: not by provider dispatch, and not by the entry the composition root delegates to. Native integration tests substitute `fixture::RecordingModel<omnia_testkit::model::Scripted>` without introducing a second provider implementation; `eval` supplies `CursorModel::new`.

Feature layout: the default surface (catalog, conversion, invocation, provider) stays dependency-light for scripted workflow tests. The process-entry surface — command-router assembly and MCP reference serving — sits behind a `cli` feature carrying the transport and server dependencies. The Cursor backend and model bridge sit behind a `cursor` feature carrying the omnia host dependencies, and gate nothing else: a downstream application with its own `Model` implementation runs the full command surface with `cli` alone.

### Adapter catalog

The adapter SDK traits are associated-function traits generic over `P: Model` and are deliberately not object-safe. The catalog remains a typed, monomorphized vtable:

- a workbench registers each source and target adapter type;
- registration captures operation function pointers specialized for the application's model type;
- `Provider<M>` resolves `<axis>:<name>` identities against that catalog;
- workflow `Source` and `Target` calls narrow their DTOs, dispatch the registered operation with `&M`, and widen the result.

This is the linked equivalent of the workflow guest's source/target WIT imports. It is application composition, not a test mock layer.

### Model bridge

Workflow and adapter libraries consume `omnia_guest::Model`, while `omnia_cursor::Client` implements the host-side `omnia_wasi_model::WasiModelCtx`. The model bridge remains necessary to preserve deployed semantics:

- map guest `Request` to the host wire request;
- run the host request gate;
- expose the project root when `lend_workspace` is requested;
- invoke the Cursor backend;
- validate and project the answer back into a guest `Reply`;
- preserve typed model errors.

The current `DevModel` is renamed `CursorModel`. The current internal `Native<B>` bridge is renamed `ModelBridge<B>` so neither type collides with the `cfg`-axis vocabulary or hides its role. The Cursor backend's driver-side model-id override is renamed from `SPECIFY_EVAL_MODEL` to `SPECIFY_MODEL`: the override belongs to the deployment's Cursor backend — the command passthrough honors it as much as evaluation does — so its name must not claim it is evaluation-only.

### MCP references

Adapter judgment requests carry MCP grants for embedded adapter references. A linked deployment must serve the same reference shelves to preserve prompt behavior.

`Provider::bound` therefore remains the live constructor:

1. build the selected adapter catalog;
2. start the ephemeral MCP listener;
3. record its base URL;
4. rewrite each operation context to the selected adapter's `/mcp/<name>` shelf.

It now fails when the listener cannot start. The previous behavior — skip serving when no port can be bound — was test-scaffolding tolerance; in a deployment it silently strips reference shelves from every prompt and silently degrades trial quality. `Provider::new` remains the listener-free constructor for deterministic tests, which are unaffected.

### The `eval` crate

`eval` is a library over `linked` (enabling its `cli` and `cursor` features), not a composition root. It also depends directly on `change` and `project` because trials invoke typed plan operations and sandbox inspection loads workflow state. It owns:

- the multi-step live workflow trial;
- single-operation adapter scenarios;
- deterministic grading;
- model-request telemetry;
- sandbox seeding and cleanup;
- shared evaluation CLI parsing;
- the combined entry helper that selects command passthrough or the `eval` subcommand.

The entry takes one explicit configuration value instead of ambient anchors:

```rust
pub struct Config {
    /// Binding-owned prompt-scenario root, when the binding supports scenarios.
    pub scenarios: Option<PathBuf>,
    /// Trial and scenario scratch root; defaults to `sandbox/`.
    pub sandbox: PathBuf,
}

pub fn main<B: linked::Binding>(config: Config) -> ExitCode
```

This replaces the positional `Option<&Path>` scenario argument and the module-level sandbox constant, so the framework keeps no implicit current-directory anchors beyond the configured roots. The entry deliberately still couples command passthrough with the `eval` subcommand: that coupling is the workbench UX, and accepting it here is what keeps both workbenches pure declarations.

`eval` is generic over `linked::Binding`. It never selects concrete adapters and never depends on `fixture` or first-party adapter crates.

The evaluation provider remains a transparent composition:

```text
linked::Provider<Telemetry<CursorModel>>
```

### The Specify workbench

`crates/workbench` is the concrete linked application for the Specify repository. Its whole body is the fixture binding plus one entry call:

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

- ordinary arguments run a Specify command through the linked substrate;
- the `eval` subcommand runs the shared live trial through the evaluation framework.

This binary is the target of `cargo make dev` and `cargo make eval`. It provides a live Cursor-backed fixture application for local development, while integration tests assemble the same `linked::Provider` with scripted model answers directly.

The `adapters!` declaration is the linked analog of a deployment manifest, and like a manifest it lives beside the engine rather than inside the root package. The root package's targets stay what they are today: the Wasm guest lib, the shipped `specify` runtime, and the change example.

### The adapter repository workbench

`augentic/specify-adapters/crates/workbench` is the same shape over the first-party source and target adapters, passing its `scenarios/` root through `entry::Config`.

It is not a dependency of Specify, its fixture crate, or Specify's tests. Its purpose is:

- native development against first-party adapters;
- live first-party prompt trials;
- single-adapter prompt scenarios.

The sibling repository's Wasm composed tests and change example remain separate gates.

## Wasm and linked correspondence

The two deployments share engine and adapter operation code but differ at their deployment seams:

| Concern              | Wasm deployment                            | Linked deployment                                 |
| -------------------- | ------------------------------------------ | ------------------------------------------------- |
| Workflow composition | `src/lib.rs` and `src/provider.rs`         | `linked::Provider<M>` and router assembly         |
| Adapter selection    | component identity and deployment manifest | workbench `adapters!` declaration                 |
| Engine invocation    | `Invoker` and `transport` router           | same `Invoker` and `transport` router             |
| Model access         | `omnia:model/completion` host import       | composition-root `Model` (Cursor via the bridge)  |
| Adapter dispatch     | WIT source/target imports                  | typed linked `Catalog<M>`                         |
| References           | adapter HTTP guest routed by Omnia         | ephemeral linked MCP listener                     |
| Project tree         | shared Wasm preopen                        | native project path                               |
| Isolation            | component instance per call                | one native process                                |

The linked deployment must preserve observable workflow behavior:

- command input and output;
- exit codes;
- artifact writes;
- lifecycle transitions;
- adapter operation order;
- model request and answer schema;
- MCP reference contents;
- report validation.

It does not preserve or test:

- component ABI conformance;
- WIT mapping;
- Wasm isolation;
- instance-per-call behavior;
- dynamic component hydration;
- global adapter-store resolution;
- pinned component digest verification;
- deployment-manifest link configuration.

Those remain owned by adapter crate tests, composed-deployment tests, and the operator-invoked Wasm change example.

## Testing model

Native testing is a consumer of the linked deployment, not the reason it exists.

### Workflow integration tests

Workflow suites compose:

```text
linked::Provider<fixture::RecordingModel<omnia_testkit::model::Scripted>>
  ├── fixture adapter catalog
  ├── temporary project root
  └── scripted model answers
```

They invoke public operations through `linked::invoke::run` or the transport router. They do not define per-crate provider mocks when the shared fixture can reach the behavior.

### Substrate tests

Tests for catalog registration, provider dispatch, command routing, MCP serving, and model bridging live under `crates/linked/tests/`.

### Evaluation tests

Tests for scenario loading, grading, telemetry, and trial argument handling live under `crates/eval/tests/`.

### Fixture tests

Tests for deterministic fixture behavior, answer recording, and the exhaustive fixture adapter inventory remain owned by `crates/fixture/tests/`.

The request-recording model decorator in `crates/fixture/src/model.rs` is renamed from `Harness<B>` to `RecordingModel<B>`. Once the `harness` package is gone, `Harness` no longer describes the type's behavior and risks being mistaken for the removed crate. The same argument applies verbatim to the sibling repository's `testkit::Harness` copy — that workspace is also losing its `harness` dependency — so it is renamed in the same coordination stage. The later move upstream to `omnia-testkit` remains optional and does not block this RFC.

### Wasm boundary tests

No linked-deployment test claims Wasm coverage. The existing component gates remain:

- adapter composed-deployment tests in `augentic/specify-adapters/composed`;
- the Specify fixture change example;
- the first-party change example over the published core component.

## Module migration

The current `crates/harness/src` modules move as follows:

| Current module | Target                              | Notes                                                                 |
| -------------- | ----------------------------------- | --------------------------------------------------------------------- |
| `catalog.rs`   | `crates/linked/src/catalog.rs`      | Keeps `Binding`, catalog builder, entries, and the `adapters!` macro  |
| `convert.rs`   | `crates/linked/src/convert.rs`      | Keeps SDK/workflow DTO mapping                                        |
| `env.rs`       | `crates/linked/src/env.rs`          | Process-scoped cache-root support (sandbox/test support; see above)   |
| `invoke.rs`    | `crates/linked/src/invoke.rs`       | Provider-neutral typed operation invocation                           |
| `provider.rs`  | `crates/linked/src/provider.rs`     | Generic composite provider; `bound` becomes fallible                  |
| `command.rs`   | `crates/linked/src/command.rs`      | Model supplied by the caller; behind the `cli` feature                |
| `mcp.rs`       | `crates/linked/src/mcp.rs`          | Ephemeral adapter reference shelves; behind the `cli` feature         |
| `model.rs`     | `crates/linked/src/cursor_model.rs` | Rename `DevModel` to `CursorModel`; `SPECIFY_MODEL` override; `cursor` feature |
| `native.rs`    | `crates/linked/src/model_bridge.rs` | Rename `Native<B>` to `ModelBridge<B>`; `cursor` feature              |
| `entry.rs`     | `crates/eval/src/entry.rs`          | Combined command/eval entry over `linked`; takes `entry::Config`      |
| `fs.rs`        | `crates/eval/src/fs.rs`             | Evaluation tree-copy support                                          |
| `grade.rs`     | `crates/eval/src/grade.rs`          | Evaluation grading                                                    |
| `sandbox.rs`   | `crates/eval/src/sandbox.rs`        | Evaluation sandbox; root from `entry::Config`                         |
| `scenario.rs`  | `crates/eval/src/scenario.rs`       | Single-operation prompt scenarios                                     |
| `telemetry.rs` | `crates/eval/src/telemetry.rs`      | Model request counts                                                  |
| `trial.rs`     | `crates/eval/src/trial.rs`          | Live workflow trial; sandbox root from `entry::Config`                |

The current `crates/eval/src/main.rs` becomes `crates/workbench/src/main.rs` and changes imports:

- `harness::entry` → `eval::entry`, passing a default `entry::Config`;
- `harness::adapters!` → `linked::adapters!`.

The `fixture` host modules change imports:

- `harness::catalog` → `linked::catalog`;
- `harness::provider` → `linked::provider`;
- `harness::convert` references → `linked::convert`;
- `harness::env` → `linked::env`.

Workflow integration tests change:

- `harness::invoke::run` → `linked::invoke::run`;
- direct `harness::provider::Provider` → `linked::provider::Provider`.

User-facing strings move with the vocabulary in the same motion: the `catalog.rs` lookup and `unlinked` messages ("is not linked into the native harness" / "native shim"), the `provider.rs` Hydrator refusal ("the native harness links adapters directly"), and the `scenario.rs` binding validation all name the linked deployment instead.

## Test migration

Current `crates/harness/tests` ownership changes:

- move `catalog.rs`, `provider.rs`, `command.rs`, and `mcp.rs` to `crates/linked/tests`, with `required-features` narrowed to the new `cli` / `cursor` split;
- move `grade.rs` and `scenario.rs` to `crates/eval/tests`;
- `boundary.rs` is superseded by the `checks` manifest invariant and is deleted with the `harness` package;
- split suite-local support modules by their owning suite rather than introducing another shared test crate.

Existing `change`, `slice`, `project`, and `transport` integration tests replace their `harness` dev-dependency with `linked`. Tests using `fixture::Session` continue to do so.

## Cargo and feature layout

### Specify workspace

Add:

```toml
eval = { path = "crates/eval" }
linked = { path = "crates/linked" }
```

The `harness` workspace-dependency entry is removed. `workbench` needs no workspace-dependency entry — nothing depends on it.

`linked` keeps a dependency-light default for scripted workflow integration tests; the `cli` feature activates the transport router, MCP serving, and their dependencies; the `cursor` feature activates the Cursor backend and model bridge with the omnia host dependencies. Neither feature is default.

`eval` enables `linked/cli` and `linked/cursor` and carries only evaluation dependencies such as `clap`, `serde`, and trial-specific workflow types. Its Cargo dependencies name `change` and `project` directly rather than relying on transitive access through `linked`; the evaluation trial imports plan handlers and plan-entry types, while sandbox code reads `project::config::Layout`.

`workbench` mirrors the current concrete binary's manifest:

```toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
eval.workspace = true
fixture.workspace = true
linked.workspace = true
```

Workflow crates use `linked` only in `[dev-dependencies]`.

The shipped surface is untouched: the root package's targets do not change, release workflows continue to build and package `--bin specify`, and `cargo install --git ... --bin specify` remains the documented source installation path. The workbench is `publish = false`, is never attached to a release, and needs no feature gating to stay out of one.

### Adapter workspace

Replace the current shared dependency:

```toml
harness = { git = "https://github.com/augentic/specify.git" }
```

with:

```toml
adapter = { git = "https://github.com/augentic/specify.git" }
eval = { git = "https://github.com/augentic/specify.git" }
linked = { git = "https://github.com/augentic/specify.git" }
```

Update the committed sibling path patches accordingly. Rename the adapter workspace's current `crates/eval` package to `crates/workbench`, moving its `scenarios/` root with it and passing that root through `entry::Config`. No dependency aliases are required anywhere.

## Command surface

Operator-facing Specify commands do not change.

Development tasks retain their names:

- `cargo make dev -- ARGS` runs the workbench's command passthrough;
- `cargo make eval` runs the same binary's `eval` subcommand;
- `cargo make change-run` continues to run the Wasm composed example.

Package selection changes from `cargo run -p eval` to `cargo run -p workbench` — one word in each task body, identical in both repositories.

## Implementation plan

### Stage 1 — Extract the linked substrate

1. Add `crates/linked/Cargo.toml` and `crates/linked/src/lib.rs`.
2. Move `catalog`, `convert`, `env`, `invoke`, `provider`, `command`, `mcp`, `model`, and `native` from the current `harness`.
3. Rename `DevModel` to `CursorModel` and `Native<B>` to `ModelBridge<B>`; rename the model-override variable `SPECIFY_EVAL_MODEL` to `SPECIFY_MODEL`.
4. Split the features: dependency-light default; `cli` for the command entry and MCP serving; `cursor` for the backend and bridge. Parameterize `command::run` over the caller-supplied model factory.
5. Make `Provider::bound` fail when the reference listener cannot start.
6. Update the user-facing strings that name the "native harness" or "native shim".
7. Move the catalog, provider, command, and MCP tests into `crates/linked/tests`.
8. Add `linked` to workspace dependencies; retarget `fixture`, workflow tests, transport tests, and project test support to `linked`.
9. Rename `fixture::model::Harness<B>` to `fixture::RecordingModel<B>` and update session aliases, tests, rustdoc, and the fixture answer-recording surface.
10. Run the linked and affected workflow suites before changing the remaining package names.

This stage is behavior-preserving apart from the two declared exceptions (listener loudness, variable rename). The existing `eval` binary temporarily consumes `linked` for its adapter binding while the evaluation modules still live under the old `harness` package.

### Stage 2 — Rename the concrete binary

1. `git mv crates/eval crates/workbench`; update the package name and description.
2. Retain its `harness::entry` import for now (the entry moves in Stage 3); its adapter declaration already uses `linked::adapters!` after Stage 1.
3. Update `cargo make dev` and `cargo make eval` to select `-p workbench`.

This frees the `eval` package name and changes no behavior.

### Stage 3 — Make evaluation a library

1. Create the new `crates/eval` library from `entry`, `fs`, `grade`, `sandbox`, `scenario`, `telemetry`, and `trial`.
2. Replace internal provider, catalog, command, model, environment, and invocation imports with `linked`; enable `linked/cli` and `linked/cursor`.
3. Introduce `entry::Config` (scenario and sandbox roots) and delete the module-level sandbox constant.
4. Keep evaluation generic over `linked::Binding`.
5. Move grading and scenario tests under `crates/eval/tests`.
6. Retarget the workbench from `harness::entry` to `eval::entry::main` with a default `Config`.
7. Remove the emptied old `crates/harness` package.
8. Verify that `eval` has no dependency on `fixture` or concrete adapter crates.

### Stage 4 — Coordinate the adapter repository

1. Publish or pin a Specify revision exposing `adapter`, `linked`, and `eval`.
2. In `augentic/specify-adapters`, add `linked` and `eval` dependencies and sibling path patches; drop `harness`.
3. Rename its concrete `crates/eval` package to `crates/workbench`; move `scenarios/` with it and pass the root through `entry::Config`.
4. Move its first-party adapter binding to `linked::adapters!` and delegate its entry handling to `eval::entry`.
5. Rename the `testkit` crate's `Harness` copy to `RecordingModel`, for the same reason as the fixture rename: `harness` no longer names anything in that workspace either.
6. Update `cargo make dev` and `cargo make eval` to select `workbench`.
7. Confirm there is still no reverse dependency from Specify to `specify-adapters`.

### Stage 5 — Documentation and checks

Update:

- both repositories' `AGENTS.md` crate graphs and testing descriptions;
- `TESTING.md` in the adapter repository;
- Specify testing and architecture standards that name `harness` or the old `eval` binary;
- contributing quality-gate documentation;
- Makefile comments and package descriptions;
- any rustdoc links naming moved modules.

Add a short linked-deployment section to the standing architecture document. It must state that the Wasm deployment remains the release and component-boundary authority.

Extend `crates/checks/boundaries.rs` with the Cargo-manifest invariant defined under [Dependency direction](#dependency-direction) — parsed as TOML, absorbing the old `harness/tests/boundary.rs` — and update `crates/checks/Cargo.toml` comments that currently name `harness` as cross-crate test support.

### Stage 6 — Verification

Run in `augentic/specify`:

```bash
cargo make check
cargo make ci
cargo make dev -- --help
cargo make eval
```

Compile-check the Wasm guests:

```bash
cargo check --lib -p specify --example change --target wasm32-wasip2
```

Run in `augentic/specify-adapters`:

```bash
cargo make check
cargo make ci
cargo make dev -- --help
cargo make eval
```

Keep the live-model commands operator-invoked when credentials are unavailable, and report that limitation rather than weakening the gate.

## Acceptance criteria

1. `linked` is a reusable library containing no concrete adapter binding and no fixture dependency.
2. `linked` can run the Specify command router over any `Binding` and any `Model` supplied by the composition root; no entry inside `linked` constructs a Cursor backend.
3. `eval` is generic over `linked::Binding` and contains no concrete adapter or fixture dependency.
4. `workbench` is the only Specify package selecting fixture adapters for a linked application, and its whole body is the adapter declaration plus one entry call.
5. Specify's workflow integration tests use `linked` plus local `fixture` adapters and never reference `augentic/specify-adapters`.
6. The sibling workbench depends one-way on Specify's `adapter`, `linked`, and `eval` crates.
7. `cargo make dev` preserves native command passthrough behavior in both repositories.
8. `cargo make eval` preserves live trial and scenario behavior in both repositories.
9. The Wasm workflow guest, fixture example guest, component manifests, shipped runtime behavior, and release surface are unchanged; default and release builds produce only the shipped `specify` binary.
10. Linked-deployment tests explicitly avoid claiming component ABI, WIT, isolation, or adapter-store coverage.
11. Crate-level tests remain integration-first; no public workflow API is widened solely for the migration.
12. Full local CI passes in both repositories, or unavailable live gates are reported precisely.
13. `checks` parses Cargo manifests and rejects workflow-core production or build dependencies on `linked`, `eval`, `fixture`, `workbench`, or `harness`; rejects `fixture` and concrete adapter crates from `linked` and `eval`; and allows explicit dev-dependencies for integration tests.
14. The request-recording model doubles are exported as `RecordingModel` in both repositories; no type named `Harness` remains.
15. `Provider::bound` fails when the reference listener cannot start; no live path silently drops MCP reference shelves.
16. No user-facing string names the removed `harness` or calls the deployment a "shim".

## Risks and mitigations

### Linked behavior is mistaken for Wasm conformance

Mitigation: preserve distinct documentation and commands for linked and Wasm gates. Keep composed-deployment and change-example coverage explicit in testing standards.

### Extracting the old packages creates a difficult migration

Mitigation: extract `linked` first; the binary move is a package rename rather than a relocation, so no interim aliases or feature gates exist at any stage. Do not retain compatibility aliases.

### Cursor dependencies leak into ordinary tests

Mitigation: the `cursor` feature gates only the backend and bridge, and the `cli` feature gates the entry surface; `linked`'s default remains suitable for scripted integration tests.

### Evaluation regains concrete adapter dependencies

Mitigation: make every evaluation entry generic over `linked::Binding`; enforce the dependency boundary in the repository checks package.

### A second implementation of adapter dispatch appears

Mitigation: all linked providers use `linked::Catalog` and `linked::Provider`. Concrete composition roots declare only their adapter binding.

### Cross-repository development becomes lockstep

Mitigation: publish or revision-pin `linked` and `eval` like the existing `adapter` dependency. The sibling path patch remains a co-development convenience, not a runtime probe.

### The linked deployment is accidentally presented as the release architecture

Mitigation: retain the Wasm runtime as the shipped operator path. Document linked execution as a local, statically linked deployment with intentionally different loading and isolation properties.

### Two packages named `workbench` cause confusion

Mitigation: the two workbenches never share a dependency graph — each is repo-local, unpublished, and depended on by nothing. The shared name is deliberate: same role, same shape, either repository.

## Alternatives considered

### Keep the current crate names

Rejected. `harness` would continue to mean generic native runtime, test utilities, and evaluation framework, while `eval` would continue to be the actual application composition root. The names hide the architecture.

### Host Specify's composition root as a feature-gated root `[[bin]]` named `native`

Rejected; this was an earlier draft of this RFC. It holds the package count flat but pays with machinery: a binary target named `native` cannot link a library named `native` (rustc cannot distinguish the dependency from the current crate), forcing a permanent `native-runtime` dependency rename in exactly the file that invokes the registration macro; the target needs a `native-app` feature, optional dependencies, `required-features`, and a cfg-gated empty Wasm `main`; the invocation grows to `cargo run -p specify --bin native --features native-app`; the release workflow needs explicit caveats; and the terminology splits into "native binary" (Specify's) versus "workbench" (downstream's) for the same architectural role. The claimed benefit — displaying correspondence with `src/lib.rs` — draws the analogy at the wrong level: `src/lib.rs` is adapter-agnostic, and the `adapters!` declaration corresponds to the deployment manifest, which also lives beside the engine rather than inside the root package.

### Name the substrate crate `native`

Rejected. "Native" already means `cfg(not(target_arch = "wasm32"))`, "native tests", and "native-only" throughout both repositories, so the name is neither precise nor greppable — and the shipped Wasm deployment's host is itself a native process. The property that distinguishes this deployment is static Rust linking versus dynamic component composition; `linked` states it.

### Fold evaluation into `linked` behind a feature

Rejected. A feature cannot stop evaluation code from reaching into runtime internals, and it leaves the "one crate doing two jobs" reading intact. The crate boundary is what the checks package can enforce.

### Put all shared code in the workbench binaries

Rejected. Specify tests and downstream native applications would duplicate provider dispatch, model bridging, catalog registration, and MCP serving.

### Put the Specify workbench in `augentic/specify-adapters`

Rejected. Specify's integration tests must use local fixture adapters without a sibling checkout. A reverse dependency on the adapter repository would create a circular repository relationship.

### Put first-party adapters in Specify's workbench

Rejected for the same dependency-direction reason. Specify's workbench selects fixture adapters; the sibling workbench selects first-party adapters.

### Hard-code Cursor into `Provider` or the command entry

Rejected. `Provider<M>` is the capability-substitution seam shared by live execution, scripted tests, and telemetry, and the command entry is the surface composition roots actually delegate to. Hard-coding Cursor into either would force a second implementation for other backends and permit behavioral drift; the Cursor backend is a `cursor`-feature module selected by the composition root.

### Move `linked` to a separate repository

Rejected. It evolves atomically with workflow and adapter SDK seams, and Specify's own integration tests consume it. A workspace leaf crate provides architectural separation without cross-repository release and dependency cycles.

## Consequences

- Linked execution becomes an explicit deployment mode rather than implicit test scaffolding.
- Crate names describe architectural responsibility, and none collides with the `cfg`-axis vocabulary or with its own binary target.
- Both repositories share one composition shape: a `workbench` package whose whole content is an adapter declaration and an entry call.
- Specify's core tests remain self-contained over fixture adapters; first-party adapter evaluation remains downstream and one-way.
- The substrate becomes reusable by other linked applications, with the model backend chosen by each composition root.
- The workspace gains the `linked` and `eval` libraries and renames the concrete binary to `workbench` — one more package than today, and in exchange the root package carries no feature gates, no dependency aliases, and no second entrypoint.
- The Wasm deployment remains necessary and authoritative for the component boundary.
