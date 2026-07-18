# Linked Specify — Native Operator Product, Evaluation, and Lab Composition

> Status: Draft
>
> Owns: the Wasm-free, statically linked operator deployment of the Specify engine; its separation from live-model evaluation and from in-repo lab composition.

## Abstract

Specify's workflow engine and adapter operation implementations are ordinary Rust libraries behind capability traits. Today the shipped operator product composes those libraries as Wasm guests on the Omnia runtime, while `crates/harness` already composes the same engine and adapters directly for native tests, development commands, and live-model evaluation.

This RFC makes that second path a **peer operator product**: the native, statically linked deployment. It extracts the host into a crate named `linked` (a library exposing the product command entry over plain catalog and model values), narrows live-model evaluation into a lab-only library named `trial`, and keeps each repository's fixture/first-party **lab** composition in an unpublished binary named `lab`. The first-party shippable fat binary is composed in `specify-adapters` over that host and installs under the operator name `specify` — the lab is never the install path.

```text
OPERATOR PRODUCTS                              LAB ONLY
┌──────────────┐  ┌───────────────────┐       ┌────────────┐  ┌──────────┐
│ specify      │  │ linked            │       │ lab        │  │ trial    │
│ Wasm product │  │ Native product    │       │ in-repo    │  │ trials / │
│              │  │ host (this repo)  │       │ composition│  │ scenarios│
│ ship/install │  │ + fat bin*        │       │ unpublished│  │ lab lib  │
└──────────────┘  └───────────────────┘       └────────────┘  └──────────┘

* First-party fat binary composed/released from specify-adapters over the
  host; installs as `specify`.
```

`linked` does not depend on `lab`, `trial`, or `fixture`. Lab tools and tests depend on `linked`. Composition is value-level: a binary hands the host a `Catalog` value and a model value — there is no binding trait, no registration macro, and no model type parameter threaded through the host. The Wasm deployment remains authoritative for component loading, WIT conformance, isolation, and adapter-store behavior; linked tests never claim those properties.

## Motivation

The current package names describe their first consumers rather than their architectural roles:

- `crates/harness` contains the native provider, adapter catalog, engine-to-adapter conversion, Cursor model bridge, MCP reference host, typed invocation helper, command router, test environment support, live evaluation trials, scenarios, grading, telemetry, and sandbox management.
- `crates/fixture` contains deterministic source and target adapters, scripted model answers, a request-recording model decorator, and native project sessions.
- `crates/eval` is the concrete native composition binary over the fixture adapters.

This makes a coherent native **operator product** look like an accumulation of testing crates. It also obscures the correspondence between the two deployments. `src/lib.rs` and `src/provider.rs` are adapter-agnostic: adapters reach the Wasm deployment at composition time through WIT imports. The linked analog of that pair is the host (`Provider`, catalog dispatch, router assembly); the linked analog of the deployment manifest is the product binary's catalog declaration:

```text
Dynamic Wasm deployment                         Static linked deployment

src/lib.rs + src/provider.rs                    linked::Provider + router assembly
deployment manifest composing components        product binary catalog declaration
adapter components                              linked adapter libraries
shipped operator binary: specify                shipped operator binary: specify (linked build)
```

A second muddle is mechanical rather than nominal. The harness catalog already erases adapter operations into `BoxFuture` fn pointers, but leaves the model type generic — and that one surviving generic drags a for-all-model factory trait (`Binding`), a registration macro (`adapters!`), a generic `Provider<M>`, and generic entrypoints behind it. The tower exists to avoid a single reference-counted vtable hop on calls that are network-bound model completions. A product host should compose with values.

Evaluation and in-repo fixture/first-party scratch binaries are consumers of that product host — not the product itself.

## Goals

1. Establish the linked deployment as a first-class **shippable native operator product**, peer to Wasm `specify`: Specify owns the `linked` host library and product command entry; the first-party fat binary is composed where the adapters live and installs as `specify`.
2. Keep the linked host out of the workflow core; workflow crates depend only on capability traits.
3. Preserve dependency inversion: `linked` never depends on `trial`, `lab`, `fixture`, or `augentic/specify-adapters`.
4. Keep Specify's integration tests self-contained over local `fixture` adapters via `linked`'s library API.
5. Let downstream repositories compose their own linked binaries (product or lab) without Specify depending back on them.
6. Separate the operator command surface from live-model evaluation: command mode lives in `linked`; `trial` is optional and lab-only.
7. Compose with values: catalogs and model backends are runtime values supplied by the composition root (product `main` or lab `main`); the host carries no binding trait, no registration macro, and no model type parameter.
8. Preserve current command, test, scenario, and evaluation behavior while changing ownership and names, with three declared exceptions: the live reference listener fails loudly instead of silently skipping, the model-override variable loses its `EVAL` name, and pinned adapter references fail with a linked-specific diagnostic instead of a generic `adapter-not-found`.
9. Keep one implementation of provider dispatch, model bridging, adapter registration, and MCP reference serving — monomorphized at exactly one model type.
10. Demote the lab binary to unpublished in-repo composition (fixture or first-party adapters plus trial UX) — not the answer to "how do I ship native Specify?"

## Non-goals

- Replacing the shipped Wasm deployment.
- Loading `.wasm` components from the linked application.
- Claiming WIT, component ABI, isolation, digest, or adapter-store coverage from linked-deployment tests.
- Moving first-party adapter implementations into the Specify repository.
- Making Specify depend on the sibling `specify-adapters` checkout.
- Changing workflow semantics, artifact schemas, lifecycle transitions, prompts, or the adapter SDK traits — `adapter::Source` / `adapter::Target` stay generic, non-object-safe associated-function traits; only the host's model handle is erased.
- Satisfying pinned adapter references against compiled-in adapter versions (requires the adapter version in SDK metadata; named as the follow-on in [Adapter reference semantics](#adapter-reference-semantics)).
- Adding a compatibility alias for the old crate names; this is an internal pre-1.0 workspace refactor.
- Widening workflow APIs solely to support tests.
- Serving the HTTP transport from the linked deployment (natural later extension; out of scope).
- Defining the full release-pipeline matrix for the linked binary in this RFC (artifact naming and CI attachment are follow-on; the architecture must make that attachment possible).

## Terminology

- **Workflow core** — the deployment-neutral engine crates: `project`, `slice`, `change`, and `transport`, plus their dependency leaves.
- **Wasm deployment / `specify`** — the shipped Omnia-hosted operator product: native host process + workflow and adapter Wasm guests. Authoritative for component loading, WIT, isolation, digests, and the adapter store.
- **Linked deployment** — the shipped native operator product: workflow core and adapter libraries compiled into one process and connected through Rust capability traits. Peer to Wasm `specify`, not a harness; installs under the same operator name.
- **Linked host (`linked`)** — the reusable library surface of the `linked` package: provider, catalog, erased model handle, model bridge, MCP, command entry, process cache isolation.
- **Erased model (`DynModel`)** — the host's one concrete model type: a reference-counted, object-safe wrapper any `Model` backend converts into. The catalog and provider are monomorphized at exactly this type.
- **Evaluation library (`trial`)** — lab-only trials, scenarios, grading, telemetry, and sandbox orchestration.
- **Lab (`lab`)** — the unpublished in-repo composition binary: declares a catalog and dispatches between command passthrough and the trial. Not an operator product. Specify's lab binds `fixture`; the adapter repository's binds first-party adapters.
- **Fixture adapters** — deterministic local implementations in `crates/fixture`; concrete SDK-native adapters, not mocks of the workflow seam.

## Decision

### Mental model

```text
OPERATOR PRODUCTS (two deployments of the same engine)
┌──────────────────────────┐     ┌──────────────────────────┐
│ specify (Wasm)           │     │ specify (linked)         │
│                          │     │                          │
│ omnia runtime + guests   │     │ linked host library +    │
│ WIT provider             │     │ product command entry    │
│ component adapters       │     │ Provider + Catalog over  │
│ adapter-store / digests  │     │ the erased model / MCP   │
│ ship / install / release │     │ ship / install / release*│
└──────────────────────────┘     └────────────┬─────────────┘
                                              │
                         library API of the same package
                              ┌───────────────┼───────────────┐
                              │               │               │
                              ▼               ▼               ▼
                  ┌────────────────┐ ┌────────────────┐ ┌────────────────┐
                  │ Integration    │ │ lab            │ │ First-party    │
                  │ tests          │ │ (lab only)     │ │ product bin*   │
                  │ RecordingModel │ │ fixture catalog│ │ (adapters repo)│
                  │ fixture catalog│ │ + trial UX     │ │ first-party    │
                  │ no trial       │ └───────┬────────┘ │ catalog        │
                  └────────────────┘         │          │ no trial       │
                                             ▼          └────────────────┘
                                     ┌────────────────┐
                                     │ trial (lab lib)│
                                     │ trials /       │
                                     │ scenarios      │
                                     └────────────────┘

* First-party fat binary ships from the adapters repo over this host
  library and installs as `specify`; see
  [Adapter composition and shipping](#adapter-composition-and-shipping).
```

Dependency direction (Cargo):

```text
lab   ──► linked
lab   ──► trial ──► linked
lab   ──► fixture
tests ──► linked   (+ fixture as needed)

linked  ──✗──► lab
linked  ──✗──► trial
linked  ──✗──► fixture
linked  ──✗──► specify-adapters
specify ──✗──► specify-adapters
```

### Crate names and ownership

| Target crate | Kind | Responsibility | Source of current code |
| --- | --- | --- | --- |
| `linked` | library | Native operator host: catalog, provider, erased model handle, MCP, command entry, model bridge, Cursor backend | Native execution modules from `harness` |
| `trial` | library | Lab-only live-model evaluation framework | Evaluation modules from `harness` |
| `lab` | binary (`publish = false`) | In-repo lab composition (fixture catalog + trial UX) | Current `eval` binary, renamed and demoted |
| `fixture` | library | Deterministic adapters, answers, sessions | Existing `fixture`, retargeted to `linked` |

The shippable first-party **linked fat binary** is not a Specify workspace package — it is composed in `specify-adapters` over this `linked` library (see below).

After migration there is no `harness` package and no package named `eval` — the name is retired, not recycled. A stale `cargo run -p eval` fails with a missing package rather than resolving to a crate whose nature changed underneath the caller, and "the eval crate" keeps one meaning per era across git history, documentation, and the sibling repository's dependency pins (`cargo make eval` keeps its task name regardless). No crate is named `native`: that word already means `cfg(not(target_arch = "wasm32"))` and "native tests," and the Wasm product's host is itself a native process. `linked` names the deployment's distinguishing property (static linking vs dynamic component composition).

### Adapter composition and shipping

Wasm `specify` ships the engine from this repository and loads adapter components dynamically from the store — so the operator binary has no compile-time dependency on `specify-adapters`.

Linked composition is static. Therefore:

1. **Specify owns the linked host** — `crates/linked` as a library (and the product-shaped entry APIs: `command::main` / `command::run`, `Provider`, `Catalog`).
2. **Specify's workspace ships no first-party-bound linked binary** — that would require a dependency on `specify-adapters`. `linked`'s production dependencies stay free of `fixture`, `trial`, and `lab` as well.
3. **The first-party native operator product** — the fat binary operators install when they want omnia/vectis/contracts/… linked in-process — is **composed and released from `augentic/specify-adapters`**: depends one-way on Specify's `linked` library, declares the first-party catalog, and calls `linked::command::main` only (no `trial`).
4. **The lab is never that product** — in both repositories it remains the unpublished lab binary (dev command passthrough + `eval` subcommand).

```text
augentic/specify                         augentic/specify-adapters
────────────────                         ─────────────────────────
specify     ← shipped Wasm product
linked      ← host lib (+ product APIs) ──► product binary (installs as specify)
              (no first-party adapters)     (first-party catalog;
                                             shippable native product)
trial       ← lab framework
lab         ← lab: fixture + trial       lab ← lab: first-party + trial
```

The first-party fat binary installs under the operator name `specify`. The two products expose one CLI contract — the same verbs, exit codes, and artifacts — so the deployment flavor is a distribution-channel concern, not a second command vocabulary; `specify --version` reports the flavor (`wasm` / `linked`) so support and bug reports can tell them apart. The Cargo package name behind the binary is a packaging detail owned by the adapters repository.

### Dependency graph (Specify workspace)

```text
augentic/specify

crates/linked                          # library — native product host
  ├── crates/adapter
  └── crates/transport                 (behind the cli feature)
        ├── crates/change
        ├── crates/slice
        └── crates/project

crates/trial                           # lab library
  ├── crates/linked                    (cli + cursor features)
  ├── crates/change
  └── crates/project

crates/lab                             # lab binary only
  ├── crates/fixture
  │     └── crates/adapter
  ├── crates/linked
  └── crates/trial

# Integration-test targets may dev-depend on linked + fixture.
# No workflow-core crate has a normal dependency on linked, trial,
# fixture, or lab.
```

Cross-repository:

```text
augentic/specify-adapters
  ├── <product package> (bin: specify)  # linked::command + first-party catalog
  │     ├── first-party adapter crates
  │     └── augentic/specify::linked    # library only
  └── lab                               # lab only
        ├── first-party adapter crates
        ├── augentic/specify::linked
        └── augentic/specify::trial
```

The lightweight `checks` package enforces these directions from Cargo manifests, parsed as TOML. One manifest walk rejects: `linked`, `trial`, `fixture`, `lab`, or the removed `harness` in `[dependencies]` and `[build-dependencies]` of `error`, `diagnostics`, `artifacts`, `adapter`, `project`, `slice`, `change`, and `transport`; rejects `fixture`, `trial`, `lab`, or any concrete adapter crate in `linked`'s production dependencies; and rejects `fixture` or concrete adapter crates in `trial`. Explicit `[dev-dependencies]` on `linked` and `fixture` remain legal where core integration suites require them. This check absorbs the current `harness/tests/boundary.rs`.

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

`linked` is the native deployment host: a **library** used by tests, the lab, and the shippable first-party product binary. Product `main` functions live in composition roots that depend on this library; they build a `Catalog` value and a model value and call `linked::command`.

It owns:

- `Catalog` and its typed source/target registration builder;
- `DynModel`, the erased model handle every composition root wraps its backend into;
- `Provider`, implementing `Anchor`, `Resolver`, `Hydrator`, `Model`, workflow `Source`, and workflow `Target`;
- adapter-SDK to workflow-seam DTO conversion;
- provider-neutral typed operation invocation;
- **command-router assembly and the process command entry** over caller-supplied catalog and model values;
- the guest-model to host-model bridge;
- the Cursor-backed `Model` implementation behind the `cursor` feature;
- ephemeral MCP serving for linked adapters' embedded reference shelves;
- process-scoped project-cache isolation (`env`) used when a linked process must not inherit the operator's global cache location (sandboxes, tests, and isolated product runs).

It does not own:

- fixture adapters or scripted answers;
- live trial definitions, scenarios, grading, or evaluation telemetry;
- the lab argv dispatch — each lab `main` owns its own ~15 lines (there is no shared "lab entry");
- a hard-coded model choice inside provider dispatch — composition roots supply the backend as a value.

#### The erased model (`DynModel`)

The adapter SDK operation traits stay generic over `P: Model` and stay non-object-safe — that is a guest-facing contract this RFC does not touch. The host, however, is monomorphized at exactly one model type. `DynModel` is a reference-counted, object-safe wrapper (an `Arc`'d vtable behind a `Model` impl) that any backend converts into: `CursorModel`, `Telemetry<CursorModel>`, and `RecordingModel<Scripted>` all erase into the same handle.

Erasing the model deletes the type-level tower described in [Motivation](#motivation) — `Binding`, `adapters!`, `Provider<M>`, and the generic entrypoints — in one move. The cost is one vtable hop per completion call, which is network-bound by definition. Each adapter's generic operations monomorphize exactly once per binary, at `DynModel`.

Because the erasure is host-local (one module), it is reversible without touching any adapter: if `omnia_guest::Model` ever grows a signature that resists boxing (borrowed streaming, say), the seam to revisit is this module alone.

Construction sugar keeps call sites clean: `Provider::new` and `Provider::bound` accept `impl Model + Send + Sync + 'static` and erase internally, so tests write `Provider::new(root, recording.clone(), fixture::catalog())` unchanged. Backends with post-run read-back (telemetry counts, recorded requests) already share their state through their own `Arc`s; callers keep a clone and read it after the run. `Provider` therefore exposes no model accessor.

#### Command entry (product path)

The operator command surface lives in `linked`, not in `trial`:

```rust
// Product main: anchor at the current directory, supply values.
let root = std::env::current_dir()?;
let model = DynModel::new(CursorModel::new(&root));
linked::command::main(root, model, catalog(), argv) // sync: builds the runtime, runs the router
```

Central assembly (`command::run`, the async body behind `main`):

```rust
let provider = Provider::bound(root, model, catalog).await?;
let invoker = Invoker::new("specify", provider);
let router = transport::command::router(invoker)?;
let response = router.execute(argv).await;
```

The product anchors at the current directory, exactly as the Wasm guest anchors at its `"."` preopen — the two products keep one operator surface. The `--project-dir` pre-verb option is lab convenience (running `cargo make dev` without `cd`), provided as the `command::project_dir` argv helper the lab `main`s call before `command::main`; it is not product surface.

A command-only product binary never depends on `trial`.

#### Features

- **default** — catalog, `DynModel`, convert, invoke, `Provider::new`, `env` (dependency-light for scripted workflow tests).
- **cli** — transport router, command entry, MCP serving (axum, tokio, `transport`).
- **cursor** — `CursorModel` and the model bridge (omnia-cursor, omnia-wasi-model).
- **live** — convenience union: `["cli", "cursor"]`.

The split follows the seams the value-level entry already drew: a composition root supplying a non-Cursor backend compiles the router without the Cursor dependency tree. Bundling them would re-couple, by feature accident, exactly what goal 7 decouples.

#### Adapter catalog

The catalog is plain data. A composition root registers each source and target adapter type through the typed builder; registration captures the operation legs as fn pointers monomorphized at `DynModel`; `Provider` resolves `<axis>:<name>` identities against the resulting value; workflow `Source` and `Target` calls narrow their DTOs, dispatch the registered operation, and widen the result.

```rust
fn catalog() -> linked::Catalog {
    linked::Catalog::builder()
        .source::<intent::Adapter>()
        .target::<vectis::Adapter>()
        .build()
}
```

This builder expression is the linked equivalent of the Wasm deployment manifest — and because the catalog is a runtime value rather than a type-level binding, a composition root may assemble it conditionally (a config-gated adapter set, a subset build) without new machinery. There is no `Binding` trait and no `adapters!` macro.

#### Adapter reference semantics

Static linking changes what an adapter *reference* can mean, and the product must say so explicitly rather than inherit the dev-shim rule:

- **Bare references** (`omnia`) resolve against the compiled-in catalog; a miss names the linked set.
- **Pinned references** (`omnia@1.2.0`, `specify:omnia@1.2.0`) are refused with a distinct `adapter-not-linked` diagnostic that names the compiled-in adapter set and points at the Wasm deployment. The linked product has no store, no digests, and no way to honor an arbitrary pin; reporting that as `adapter-not-found` (the current shim behavior) would misreport a deployment property as a missing adapter.
- **`specify init`** with a package reference fails the same way; `specify init <bare-name>` binds the linked adapter. Projects whose `project.yaml` carries pins are therefore Wasm-bound until re-initialised — an explicit, documented portability caveat. The follow-on (out of scope here) is satisfying a pin against the compiled-in adapter's own version, which requires the adapter version in SDK metadata.
- **Hydration** — the linked provider fetches nothing at resolve time; its refusal diagnostic names the linked deployment, not a "native harness".
- **Version floors** — a fat binary compiles adapters and host from one revision, so the metadata `specify_floor` gate is satisfied by construction: a property of static linking worth stating, not a skipped check.

#### Model bridge

Workflow and adapter libraries consume `omnia_guest::Model`, while `omnia_cursor::Client` implements the host-side `omnia_wasi_model::WasiModelCtx`. The bridge remains necessary:

- map guest `Request` to the host wire request;
- run the host request gate;
- expose the project root when `lend_workspace` is requested;
- invoke the Cursor backend;
- validate and project the answer back into a guest `Reply`;
- preserve typed model errors.

Rename `DevModel` → `CursorModel` and internal `Native<B>` → `ModelBridge<B>`. Rename the driver-side model-id override from `SPECIFY_EVAL_MODEL` to `SPECIFY_MODEL`: the override belongs to the deployment's Cursor backend (product command passthrough and evaluation alike). The rename covers every read, including the scenario report envelope, which currently reads the variable directly rather than through the model.

#### MCP references

Adapter judgment requests carry MCP grants for embedded adapter references. A linked deployment must serve the same reference shelves to preserve prompt behavior.

Constructors:

- `Provider::new(root, model, catalog)` — no listener (deterministic tests).
- `Provider::serve_references(self).await?` — start the ephemeral MCP listener; **fails** when the catalog carries reference documents and no port can bind (no silent shelf stripping). A catalog serving no documents is a no-op `Ok` — nothing to serve, no port demanded.
- `Provider::bound(root, model, catalog).await?` — sugar: `new`, then `serve_references`.

The previous skip-when-unbound behavior was test-scaffolding tolerance; in an operator product it silently degrades every prompt that needs references.

#### Process cache isolation (`env`)

`env`'s scoped project-cache guard isolates the process-global cache location for sandboxes, tests, and other isolated runs. That is legitimate linked-host process configuration, not evaluation residue. It stays in `linked`.

### The `trial` crate (lab only)

`trial` is a library over `linked` (enabling its `live` features). It also depends directly on `change` and `project` because trials invoke typed plan operations and sandbox inspection loads workflow state. It owns:

- the multi-step live workflow trial;
- single-operation adapter scenarios;
- deterministic grading;
- model-request telemetry;
- sandbox seeding and cleanup (the scratch root stays `sandbox/`);
- shared evaluation CLI parsing.

```rust
/// Lab trial entry: parses the `eval` argv, builds the runtime, runs the
/// requested phase (or the complete trial) over the given catalog.
/// `scenarios` is the composition root's prompt-scenario tree, when it has one.
pub fn main(catalog: linked::Catalog, args: &[String], scenarios: Option<&Path>) -> ExitCode
```

`trial` receives catalogs as values and constructs none; it never depends on `fixture` or first-party adapter crates. It composes its own live backend — that is what makes it the lab:

```rust
let telemetry = Telemetry::new(CursorModel::new(&root));
let provider = Provider::bound(&root, telemetry.clone(), catalog.clone()).await?;
// … phases stream through linked::command::invoke and linked::invoke::run …
telemetry.counts() // read back from the caller-held clone
```

There is no shared "lab entry" multiplexer: command passthrough belongs to `linked::command`, the trial belongs here, and each lab `main` dispatches between them itself.

### The lab binary (lab only)

`crates/lab` is the unpublished in-repo lab binary. Its `main` is the whole composition — the catalog declaration plus the argv dispatch:

```rust
fn main() -> std::process::ExitCode {
    let mut argv: Vec<String> = std::env::args().collect();
    if argv.get(1).is_some_and(|arg| arg == "eval") {
        return trial::main(catalog(), &argv[1..], None);
    }
    let root = match linked::command::project_dir(&mut argv) {
        Ok(root) => root,
        Err(error) => {
            eprintln!("error: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let model = linked::DynModel::new(linked::CursorModel::new(&root));
    linked::command::main(root, model, catalog(), argv)
}

fn catalog() -> linked::Catalog {
    linked::Catalog::builder()
        .source::<fixture::Docs>()
        .source::<fixture::Code>()
        .target::<fixture::Adapter>()
        .build()
}
```

- ordinary arguments run a Specify command through `linked::command`;
- the `eval` subcommand runs the shared live trial.

The dispatch lines are deliberately duplicated between the two repositories' labs: a composition root that composes in the open beats a shared entry that hides which pieces a lab wires together.

This binary is the target of `cargo make dev` and `cargo make eval`. It is **not** the shipped native operator product; do not document "build the lab" as how to ship or install linked Specify.

Integration tests assemble `linked::Provider` with scripted model answers directly — they do not require the lab.

### Adapter-repository composition

`augentic/specify-adapters` consumes Specify's `adapter`, `linked`, and (for lab only) `trial` crates one-way.

| Binary | Role | Entry | Adapters |
| --- | --- | --- | --- |
| Product binary (installs as `specify`) | Shippable native operator product | `linked::command::main` + Cursor model value | first-party catalog |
| `lab` | Lab only (`publish = false`) | inline dispatch: `linked::command` / `trial::main` + scenarios root | first-party catalog |

Wasm composed tests and the change example remain separate gates.

## Wasm and linked correspondence

| Concern | Wasm (`specify`) | Linked |
| --- | --- | --- |
| Operator product | shipped `specify` binary | linked `specify` fat binary (composed in the adapters repo) |
| Workflow composition | `src/lib.rs` + `src/provider.rs` | `linked::Provider` + router assembly |
| Adapter selection | component identity + deployment manifest | product binary catalog declaration |
| Engine invocation | `Invoker` + `transport` router | same |
| Model access | `omnia:model/completion` host import | composition-root model value (Cursor behind the bridge) |
| Adapter dispatch | WIT source/target imports | `Catalog` fn-pointer table over the erased model |
| References | adapter HTTP guest routed by Omnia | ephemeral linked MCP listener |
| Project tree | shared Wasm preopen | current directory |
| Isolation | component instance per call | one native process |
| Adapter references | store install + digest verify | compiled-in set; pins refused (`adapter-not-linked`) |
| Lab composition | n/a | unpublished `lab` + `trial` |

Observable behavior the linked product must preserve: command I/O, exit codes, artifact writes, lifecycle transitions, adapter operation order, model request/answer schema, MCP reference contents, report validation. Adapter reference *resolution* intentionally diverges as specified in [Adapter reference semantics](#adapter-reference-semantics) — that divergence is part of the product contract, not an accident to preserve away.

It does not preserve or test: component ABI, WIT mapping, Wasm isolation, instance-per-call behavior, dynamic component hydration, global adapter-store resolution, pinned digest verification, deployment-manifest link configuration. Those remain owned by adapter crate tests, composed-deployment tests, and the operator-invoked Wasm change example.

## Testing model

Native testing is a consumer of the linked **library**, not a reason the product exists — and not routed through the lab.

### Workflow integration tests

```text
linked::Provider
  ├── fixture adapter catalog (fixture::catalog(), a value)
  ├── temporary project root
  └── RecordingModel<omnia_testkit::model::Scripted> — erased into the
      provider; the suite keeps its own handle for request and
      exhaustion asserts
```

Invoke public operations through `linked::invoke::run` or the transport router.

### Substrate / product-host tests

Catalog registration, provider dispatch, command routing, MCP serving, and model bridging live under `crates/linked/tests/`, over crate-local probe implementors (no fixture dependency — the boundary stays enforced by `checks`).

### Evaluation tests

Scenario loading, grading, telemetry, and trial argument handling live under `crates/trial/tests/`.

### Fixture tests

Deterministic fixture behavior, answer recording, and the exhaustive fixture inventory remain in `crates/fixture/tests/`. `fixture` deliberately keeps its four roles — adapter implementations, the scripted answer corpus, the `RecordingModel` double, and the `Session` helpers — because `Session` is inherently fixture-bound; only `RecordingModel` is adapter-agnostic, and its later move upstream to `omnia-testkit` remains optional.

Rename `fixture::model::Harness<B>` → `fixture::RecordingModel<B>`. Rename the sibling repository's `testkit::Harness` copy the same way in the coordination stage.

### Wasm boundary tests

No linked test claims Wasm coverage. Existing component gates remain: adapters `composed` tests, the Specify fixture change example, and the first-party change example over the published core component.

## Module migration

| Current module | Target | Notes |
| --- | --- | --- |
| `catalog.rs` | `crates/linked/src/catalog.rs` | typed builder over the SDK traits, monomorphized at `DynModel`; `Binding` and `adapters!` deleted |
| — | `crates/linked/src/model.rs` | new: `DynModel`, the erased model handle (default feature) |
| `convert.rs` | `crates/linked/src/convert.rs` | SDK/workflow DTO mapping |
| `env.rs` | `crates/linked/src/env.rs` | Process cache isolation |
| `invoke.rs` | `crates/linked/src/invoke.rs` | Typed operation invocation |
| `provider.rs` | `crates/linked/src/provider.rs` | non-generic over `DynModel`; loud `serve_references` / `bound`; `adapter-not-linked` for pinned refs; no model accessor |
| `command.rs` | `crates/linked/src/command.rs` | value-level `run` + sync `main`; `project_dir` argv helper (lab-facing); `cli` feature |
| `mcp.rs` | `crates/linked/src/mcp.rs` | Ephemeral reference shelves; no-docs no-op; `cli` feature |
| `model.rs` | `crates/linked/src/cursor_model.rs` | `CursorModel`; `SPECIFY_MODEL`; `cursor` feature |
| `native.rs` | `crates/linked/src/model_bridge.rs` | `ModelBridge<B>`; `cursor` feature |
| `entry.rs` | — | dissolved: dispatch into the lab `main`s, runtime construction into `trial::main` |
| `fs.rs` | `crates/trial/src/fs.rs` | Evaluation tree-copy |
| `grade.rs` | `crates/trial/src/grade.rs` | Grading |
| `sandbox.rs` | `crates/trial/src/sandbox.rs` | Sandbox; scratch root stays `sandbox/` |
| `scenario.rs` | `crates/trial/src/scenario.rs` | Prompt scenarios; the report envelope's direct `SPECIFY_EVAL_MODEL` read follows the `SPECIFY_MODEL` rename |
| `telemetry.rs` | `crates/trial/src/telemetry.rs` | Model request counts; read back through a caller-held clone |
| `trial.rs` | `crates/trial/src/run.rs` | Live workflow trial behind `trial::main` |

Current `crates/eval` becomes `crates/lab`. User-facing strings that name the "native harness" or "native shim" name the linked deployment / linked host instead.

## Test migration

- Move `catalog.rs`, `provider.rs`, `command.rs`, and `mcp.rs` to `crates/linked/tests` (probe implementors stay crate-local).
- Move `grade.rs` and `scenario.rs` to `crates/trial/tests`.
- Delete `boundary.rs` in favor of the `checks` manifest invariant.
- Workflow suites: `harness` → `linked`; `fixture::Session` unchanged in role (it now holds the recording handle beside the provider instead of reading it back through the provider).
- Rename `RecordingModel` across both repositories.

## Cargo and feature layout

### Specify workspace

```toml
linked = { path = "crates/linked" }
trial = { path = "crates/trial" }
```

Remove the `harness` workspace-dependency entry. `lab` needs no workspace-dependency entry.

`linked` default stays dependency-light; `cli` pulls transport and the MCP stack, `cursor` pulls the Cursor/omnia host deps, `live` is the union.

`trial` enables `linked`'s `live` features and names `change` / `project` directly.

`lab`:

```toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
fixture.workspace = true
linked.workspace = true
trial.workspace = true
```

`lab` is `publish = false` and never attached to a release.

The Wasm release surface (`--bin specify`, core guest, adapter contract) remains. Attaching a linked fat binary to releases is follow-on work in the adapters (and possibly Specify) release pipelines; this RFC requires that architecture not block it.

### Adapter workspace

Replace:

```toml
harness = { git = "https://github.com/augentic/specify.git" }
```

with:

```toml
adapter = { git = "https://github.com/augentic/specify.git" }
linked = { git = "https://github.com/augentic/specify.git" }
trial = { git = "https://github.com/augentic/specify.git" }
```

- Rename current `crates/eval` → `crates/lab` (move `scenarios/` with it; inline the argv dispatch in its `main`).
- Add the **product binary** package that depends on `linked` (library) + first-party adapters, calls `linked::command::main` only, and installs its binary as `specify`.
- Update sibling path patches. No dependency aliases required.

## Command surface

Operator-facing Specify verbs do not change.

Development tasks retain their names but target the lab binary:

- `cargo make dev -- ARGS` → lab command passthrough (via `linked::command`);
- `cargo make eval` → lab `eval` subcommand (via `trial::main`);
- `cargo make change-run` → Wasm composed example.

Package selection for lab tasks: `cargo run -p lab`. Installing or releasing the native operator product is **not** `cargo install` of the lab.

## Implementation plan

### Stage 1 — Extract the linked host

1. Add `crates/linked` and move `catalog`, `convert`, `env`, `invoke`, `provider`, `command`, `mcp`, `model`, and `native` from `harness`.
2. Introduce `DynModel`; drop the model type parameter from `Catalog` and `Provider`; delete `Binding` and `adapters!` (`fixture::catalog()` returns a plain `Catalog`).
3. Rename `DevModel` → `CursorModel`, `Native<B>` → `ModelBridge<B>`, `SPECIFY_EVAL_MODEL` → `SPECIFY_MODEL` (including the scenario report envelope's direct read).
4. Make `command::run` / `command::main` take root, model, and catalog values; add the `project_dir` argv helper; split the `cli` / `cursor` features.
5. Add `serve_references` (fallible; no-op when the catalog serves no documents); make `bound` use it; fail loud on bind failure.
6. Introduce the `adapter-not-linked` diagnostic for pinned references; update user-facing "native harness" / "shim" strings.
7. Move host tests into `crates/linked/tests`; retarget `fixture` and workflow tests to `linked`; replace `Provider::model()` call sites with caller-held handles.
8. Rename `fixture::model::Harness` → `RecordingModel`.
9. Run linked and affected workflow suites. (`harness` temporarily depends on `linked` for its remaining evaluation modules — expected until Stage 2.)

### Stage 2 — Evaluation library

1. Create `crates/trial` from `fs`, `grade`, `sandbox`, `scenario`, `telemetry`, and `trial` (landing as `run.rs` behind `trial::main`, which absorbs `entry.rs`'s runtime construction).
2. Retarget the in-repo `eval` binary's `main` to the inline dispatch (`linked::command` / `trial::main`); delete `entry.rs`.
3. Keep `trial` value-consuming: catalogs arrive as arguments; no fixture or adapter-crate dependencies.
4. Remove the emptied `crates/harness`.

### Stage 3 — Lab rename

1. `git mv crates/eval crates/lab`; update package name and description (`publish = false`, lab-only docs).
2. Point `cargo make dev` / `cargo make eval` at `-p lab`.

(No stage frees a package name for reuse — `eval` is retired, not recycled — so the stages carry no rename-ordering hazard.)

### Stage 4 — Adapter repository

1. Pin Specify revision exposing `adapter`, `linked`, and `trial`.
2. Rename lab `eval` → `lab`; inline the dispatch `main` over the first-party catalog + scenarios root.
3. Introduce the first-party **product** binary over `linked::command::main` (no `trial`), installing as `specify`.
4. Rename `testkit::Harness` → `RecordingModel`.
5. Confirm no reverse dependency from Specify to `specify-adapters`.

### Stage 5 — Documentation and checks

Update both repositories' `AGENTS.md`, adapter `TESTING.md`, Specify testing/architecture standards, quality-gate docs, Makefiles, and rustdoc.

Add a linked-deployment section to the architecture document that states:

- `specify` ships as two peer operator products — one operator name, two deployment flavors (Wasm vs static), distinguished by `--version`;
- `lab` / `trial` are lab-only;
- linked tests never satisfy component/WIT/digest/store gates;
- pinned adapter references refuse under the linked flavor (`adapter-not-linked`) with the documented portability caveat.

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

1. `linked` is a reusable host library containing no concrete adapter binding and no `fixture`, `trial`, or `lab` dependency in its production dependencies.
2. Catalogs and models are composition-root values: `linked` runs the Specify command router over any `Catalog` value and any `Model` value; `Catalog` and `Provider` carry no model type parameter; no `Binding` trait or `adapters!` macro exists; no path inside `linked` constructs a Cursor backend except behind the `cursor` feature.
3. A command-only product binary can depend on `linked` alone (plus its adapter crates) — it must not require `trial` or `lab`.
4. `trial` receives catalogs as values and constructs none; it contains no concrete adapter or fixture dependency; it is documented and packaged as lab-only.
5. `lab` is unpublished lab composition (Specify: fixture catalog + trial UX) whose `main` owns the argv dispatch; it is never described as the shipped native operator product.
6. Specify's workflow integration tests use `linked` plus local `fixture` adapters and never reference `augentic/specify-adapters`.
7. The sibling repository's product binary depends one-way on Specify's `linked` library, calls `linked::command::main`, and installs as `specify`; its lab remains lab-only over `trial`.
8. `cargo make dev` / `cargo make eval` preserve lab behavior via the lab binary in both repositories.
9. The Wasm workflow guest, fixture example guest, component manifests, shipped `specify` runtime behavior, and existing Wasm release surface remain intact.
10. Linked-deployment tests explicitly avoid claiming component ABI, WIT, isolation, or adapter-store coverage.
11. Crate-level tests remain integration-first; no public workflow API is widened solely for the migration.
12. Full local CI passes in both repositories, or unavailable live gates are reported precisely.
13. `checks` enforces the dependency rules in [Dependency graph](#dependency-graph-specify-workspace).
14. Request-recording doubles are exported as `RecordingModel` in both repositories; no type named `Harness` remains.
15. `Provider::serve_references` / `bound` fail when reference documents exist and the listener cannot start, and no-op when the catalog serves none.
16. Bare adapter references resolve against the compiled-in catalog; pinned references and package-reference `init` fail with `adapter-not-linked`, naming the linked set and pointing at the Wasm deployment.
17. `SPECIFY_MODEL` covers every model-override read, including the scenario report envelope; no user-facing string names the removed `harness`, a "shim", or `SPECIFY_EVAL_MODEL`.
18. Documentation answers "how do I ship/install native Specify?" with the linked operator product — never the lab.

## Risks and mitigations

### Linked behavior is mistaken for Wasm conformance

Mitigation: distinct documentation and commands for linked vs Wasm gates; keep composed-deployment and change-example coverage explicit.

### Operators confuse the lab with the product

Mitigation: the lab stays `publish = false`; docs and acceptance criterion 18 forbid describing it as the install path; the product entry is `linked::command`.

### Two `specify` binaries on one machine confuse diagnosis

One operator name across two deployment flavors means a bug report's "specify" is ambiguous. Mitigation: `specify --version` reports the flavor; the linked flavor's `adapter-not-linked` diagnostic names itself; distribution channels stay documented and distinct.

### Specifying "linked is the product" while first-party adapters live elsewhere

Mitigation: document the static-composition constraint explicitly — Specify owns the host; the first-party fat binary is composed in `specify-adapters`. Same dependency inversion as Wasm components vs the `specify` binary, different linking time.

### Cursor dependencies leak into ordinary tests

Mitigation: `cli` / `cursor` are opt-in; default `linked` stays scripted-test suitable.

### Evaluation regains concrete adapter dependencies

Mitigation: `trial` consumes catalogs as values and declares no adapter crates; enforce in `checks`.

### A second implementation of adapter dispatch appears

Mitigation: all linked providers use `linked::Catalog` and `linked::Provider`, monomorphized at the one erased model type.

### `omnia_guest::Model` grows an erasure-resistant surface

A future model capability (borrowed streaming, generic returns) could resist boxing behind `DynModel`. Mitigation: the erasure is one host-local module and the SDK traits stay generic, so reverting to a generic host is mechanical and touches no adapter; revisit when such a capability lands, not before.

### Cross-repository development becomes lockstep

Mitigation: revision-pin `linked` and `trial` like `adapter`; sibling path patch remains co-development only.

## Alternatives considered

### Keep the current crate names

Rejected. `harness` conflates host, evaluation, and composition; `eval` names the lab binary rather than the evaluation library.

### Keep the model-generic catalog behind a `Binding` trait and an `adapters!` macro

Rejected — this is the harness design, preserved by inertia rather than chosen. The fn-pointer catalog already erases adapter operations into `BoxFuture`s; the model parameter was the last generic standing, and it forced a for-all-model factory trait, a macro to implement it, a generic provider, generic entrypoints, and a binding-generic evaluation library — all to avoid one reference-counted vtable hop on network-bound completion calls. Value-level catalogs also unlock conditional assembly (config-gated adapter sets) that type-level bindings cannot express without more machinery. Revisit only if the `Model` trait grows a signature that resists boxing.

### Reuse the freed `eval` package name for the evaluation library

Rejected. Recycling a package name across one migration makes git history, documentation, and the sibling repository's dependency pins ambiguous — "the eval crate" would mean a binary before the migration and an unrelated library after it. Retiring the name costs nothing (`cargo make eval` keeps its task name) and keeps every reference unambiguous.

### Treat the lab as the linked deployment / composition root for the product

Rejected. "Build the lab" is the wrong operator intuition for a shippable native product. The lab is lab-only; the product entry is `linked::command` (and the first-party fat binary in the adapters repo).

### Host the composition root as a feature-gated root `[[bin]]` named `native`

Rejected. Package/binary name collisions, feature gates, release caveats, and split terminology ("native" vs "lab") for the same role. Correspondence with `src/lib.rs` is drawn at the host layer (`Provider` + router), not by stuffing the adapter manifest into the root package.

### Name the host crate `native`

Rejected. Collides with `cfg(not(target_arch = "wasm32"))` vocabulary; the Wasm host is also a native process. `linked` states the distinguishing property.

### Fold evaluation into `linked` behind a feature

Rejected. Blurs the operator product with lab machinery; `checks` cannot enforce a clean boundary as well as a crate edge. Command-only product binaries must not pull evaluation code.

### Put all shared host code only in lab binaries

Rejected. Duplicates provider dispatch, bridging, catalog, and MCP across tests and product binaries.

### Put first-party adapters in Specify's linked binary

Rejected. Would create a Specify → `specify-adapters` dependency (or move adapters in-tree). Same inversion rule as today: Specify does not depend on the sibling adapter repository.

### Put Specify's lab in `specify-adapters`

Rejected. Specify's integration tests and fixture lab must work without a sibling checkout.

### Hard-code Cursor into `Provider` or `command::run`

Rejected. `Provider` is the capability-substitution seam for live execution, scripted tests, and telemetry. The composition root supplies the model value.

### Route operator command mode through a shared lab entry

Rejected — for the product path and for the lab path alike. An `entry::main` multiplexer made the evaluation library double as a UX shell and hid which pieces a lab wires together; the dispatch is ~15 self-describing lines that belong in each lab `main`. Product binaries call `linked::command::main` directly.

### Move `linked` to a separate repository

Rejected. It evolves atomically with workflow and adapter SDK seams; Specify's tests consume it. A workspace package provides separation without extra release cycles for the host library.

## Consequences

- Linked execution is an explicit **operator product**, peer to Wasm `specify`, shipping under the same operator name.
- `linked` is the product host library (`command` and friends); `trial` and `lab` are lab-only and unpublished as products.
- Composition is value-level end to end: a binary is a catalog value, a model value, and one entry call — no binding trait, no macro, no generics tower.
- Command mode lives in `linked`; evaluation is an optional lab client of that host.
- Adapter reference semantics are an explicit product contract: bare names resolve against the compiled-in set, pins refuse loudly, and the portability caveat is documented.
- Specify's core tests remain self-contained over fixture adapters; the first-party fat binary and first-party lab remain downstream and one-way.
- The Wasm deployment remains necessary and authoritative for the component boundary.
- First-party native shipping is composed where the adapters live, over Specify's `linked` library — the static-linking dual of publishing `.wasm` components for the Wasm product.
