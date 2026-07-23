# Native Specify — Native Host, Eval, and Lab Composition

> Status: Implemented (archived). Stages 1–5 complete; optional Stage 5 polish (`--version` flavor labels / help neutralization) deferred. Native operator distribution and shared catalog library remain follow-up (out of scope).
>
> Owns: the Wasm-free, statically composed host for the Specify engine; the small shared seams it needs; and its separation from live-model eval and in-repo lab composition.

## Abstract

Specify is one product with two ways to satisfy the same capability contracts:

- the existing **Wasm deployment** composes the engine and adapters as components through Omnia;
- the **native host** compiles the engine core and selected Rust adapter libraries into one process.

The current `crates/harness` already approximates the native host for native engine tests, development commands, and live-model eval, but its architecture is shaped by those first consumers. This RFC replaces it with:

- `native` — the reusable in-process host library;
- `eval` — the lab-only library for trials, scenarios, grading, and telemetry;
- `lab` — each repository's unpublished composition binary.

Composition is value-level. A lab or downstream binary supplies a validated `Catalog` and a model backend; `native` carries no repository binding trait, registration macro, Cursor dependency, or model type parameter. The first-party catalog is declared by the adapters repository's `lab` until a native operator distribution needs a shared library. Product distribution is follow-up work; this RFC does not invent a product binary or a `composition` package.

```text
ENGINE CORE
  capability contracts only
        │
 ┌──────┴──────┐
 │             │
 Wasm       native host
 components Catalog + DynModel + paths
               │
        ┌──────┼──────┐
        │      │      │
   engine     lab   future product
   tests            (distribution RFC)
   + mock
```

The Wasm deployment remains authoritative for component loading, WIT conformance, isolation, digests, and adapter-store behavior. Native tests never claim those properties.

## Motivation

The current package names describe their first consumers rather than their architectural roles:

- `crates/harness` contains the native provider, adapter catalog, engine-to-adapter conversion, Cursor model bridge, MCP reference host, typed invocation helpers, command router, process-environment test support, live eval trials, scenarios, grading, telemetry, and sandbox management.
- `crates/mock` contains deterministic source and target adapters, scripted model answers, a request-recording model decorator, and native project sessions.
- `crates/eval` is the concrete native composition binary over the mock adapters.

This makes a coherent native host look like accumulated testing machinery. It also hides the correspondence between the two providers:

```text
Wasm provider                                Native provider

src/lib.rs + crates/guest                    native::Provider + command assembly
component deployment configuration           lab-owned Catalog value
adapter components                           native adapter libraries
host model import                            composition-root DynModel value
adapter HTTP references                      native ReferenceHost
```

A second muddle is mechanical. The harness catalog already erases source-adapter and target-adapter operations into `BoxFuture` function pointers, but leaves the model type generic. That one generic forces `Catalog<M>`, `Provider<M>`, a for-all-model `Binding` trait, the `adapters!` registration macro, and generic command/eval entrypoints. The tower exists to avoid one reference-counted vtable hop on network-bound model completions.

A third muddle is architectural. Adapter references are currently reduced too early to `{ name, version }`, while initialization separately knows whether the operator supplied a bare name, package reference, or local component path. The native provider therefore cannot distinguish a local `.wasm` selector from a bare name after parsing, and the low-level `Hydrator::fetch` capability forces native initialization through component-oriented provisioning before native resolution can refuse or satisfy the selector.

The native host makes those concerns visible:

- deployment-neutral engine behavior belongs in the engine core;
- adapter selector parsing belongs in shared contracts;
- how a selector becomes a usable adapter is deployment policy (`ensure` on the existing resolver capability);
- networking and cache isolation need explicit lifecycle ownership;
- eval is a client of the native host, not part of the product host;
- Cursor and product packaging are composition-root concerns, not host concerns.

## Goals

1. Establish the native host as a first-class way to run the same engine core and command contract as the Wasm flavor.
2. Keep the native host out of the engine core; engine crates depend only on capability contracts and deployment-neutral adapter types.
3. Preserve dependency inversion: `native` never depends on `eval`, `lab`, `mock`, Cursor crates, or `augentic/specify-adapters`.
4. Compose with values: catalogs and model backends are supplied by composition roots; the host carries no binding trait, registration macro, or model type parameter.
5. Make the native catalog a complete static declaration: each adapter's identity, operation axis, metadata, and embedded references.
6. Let the native host satisfy an exact adapter pin when the compiled catalog contains that exact `(name, version)`; reject mismatches and local component selectors without silently substituting another implementation.
7. Replace byte-oriented `Hydrator::fetch` with `Resolver::ensure` so initialization does not run component hydration before native catalog matching.
8. Preserve workflow command I/O, exit codes, artifacts, lifecycle transitions, adapter operation order, and model request/answer shapes across providers; selector diagnostics and native provisioning remain the explicit divergences.
9. Keep Specify's engine integration tests self-contained over local mock adapters through `native`'s library API.
10. Separate command execution from live-model eval: command mode lives in `native`; `eval` is optional and lab-only.
11. Make runtime values and resources explicit: project root, cache parent, model value, MCP listener ownership.
12. Keep one native implementation of provider dispatch, catalog construction, and MCP reference serving, with adapter operations monomorphized at exactly one model type (`DynModel`).

## Non-goals

- Replacing the Wasm deployment.
- Loading `.wasm` components from the native host.
- Claiming WIT, component ABI, isolation, digest, or adapter-store coverage from native tests.
- Moving first-party adapter implementations into the Specify repository.
- Making Specify depend on the sibling `specify-adapters` checkout.
- Changing workflow semantics, artifact schemas, lifecycle transitions, or prompts.
- Making `adapter::Source` / `adapter::Target` object-safe; their operation methods remain generic associated functions. This RFC only adds static identity to their contract and erases the host's model handle.
- Guaranteeing a fully static executable. The native host statically composes the Rust engine and adapters but may still use platform libraries and an external model backend.
- Supporting two published adapters with the same name under different package namespaces. Adapter names remain globally unique across namespaces and axes for published adapters; namespace on a package selector is parse/display provenance, not a second engine identity.
- Defining a native operator distribution (package name, install identity, coexistence with Wasm `specify`, bundle version, release cadence). The adapters `lab` owns the first-party catalog until a distribution decision needs a shared library.
- Adding a `composition` package in this RFC. Extract `catalog()` from the adapters lab when a second consumer (a product binary) appears.
- Serving the HTTP transport from the native host. The reference router must remain mountable by a later HTTP host without a second implementation.
- Adding cooperative cancellation or concurrent multi-command service semantics. The initial native command path is one command to completion per process.
- Upstreaming the guest-model/host-model bridge into Omnia in this change. Composition roots that need Cursor keep a private bridge until an upstream API exists.
- Recording package namespace in component store sidecars, or migrating legacy sidecars. That is Wasm-store follow-up; this RFC does not require it for the native host.
- Closing the existing gap where `SourceBinding.version` is persisted but survey/extract ignore it. That correctness fix is Stage 1b and is not required to extract the native host.
- Rewriting bare native init selectors into exact package pins. Persist what the operator typed; resolve reports the catalog version.
- Changing `--version` flavor labels or neutralizing shared help text in Stage 1. Those are Stage 5 documentation polish.
- Renaming `EVAL_MODEL`, `mock::model::Harness`, or `DevModel` as part of the architectural cut. Optional later polish.
- Renaming the default mock source or target identity. `mock` remains valid on both axes; catalog uniqueness is per-axis.
- Adding compatibility aliases for removed internal crate names.
- Widening public engine APIs solely to support tests.

## Terminology

- **Workflow core** — the deployment-neutral engine crates: `project`, `slice`, `change`, and `transport`, plus their dependency leaves.
- **Wasm deployment** — the existing Omnia-hosted flavor: native host process plus engine and adapter Wasm guests. Authoritative for component loading, WIT, isolation, digests, and the adapter store.
- **Native host (`native`)** — the reusable library: catalog, provider, erased model handle, reference host, and asynchronous command execution. Not a Cursor client and not a product installer.
- **Adapter identity** — the immutable `(name, version)` a catalog entry provides. Distinct from resolve-time adapter metadata.
- **Adapter selector** — the operator-supplied shape retained through `ensure`: bare name, exact package reference, or local component path.
- **Erased model (`DynModel`)** — the native host's one concrete model type: a reference-counted object-safe wrapper that any `Model` backend converts into.
- **Eval library (`eval`)** — lab-only trial, scenario, grading, telemetry, sandbox, and eval CLI logic.
- **Lab (`lab`)** — an unpublished repository composition binary that dispatches between native command passthrough and eval. Specify's lab binds mock adapters; the adapter repository's lab declares the first-party catalog and depends on the concrete adapter crates.
- **Fixture adapters** — deterministic local SDK-native adapters in `crates/mock`, not mocks of the engine seam.

## Decision

### Mental model

```text
                    engine core + transport
                               ▲
                               │ capability contracts
              ┌────────────────┴────────────────┐
              │                                 │
     Wasm Provider                       native::Provider
     component ensure                    catalog ensure
     model host import                   DynModel value
     adapter HTTP refs                   ReferenceHost
              │                                 │
              │                                 ├── engine tests (+ mock)
              │                                 ├── repository labs
              │                                 └── (future product binary)
              │
     component boundary gates

eval ──► native
lab  ──► eval + native + one repository-owned catalog
```

One breath: engine code talks to capabilities; Wasm satisfies them with components; native satisfies them with a validated catalog and an erased model; labs and tests are composition roots that pass those values in; eval is just another client of the native command API.

Dependency direction in the Specify workspace:

```text
lab ──► mock
lab ──► native
lab ──► eval ──► native
tests ──► native (+ mock as needed)

native ──✗──► eval
native ──✗──► lab
native ──✗──► mock
native ──✗──► specify-adapters
native ──✗──► Cursor / omnia-cursor

engine-core production dependencies ──✗──► native / eval / mock / lab
engine-core dev-dependencies ────────────► native + mock where integration suites need them
```

### Crate names and ownership

| Target crate | Kind                        | Responsibility                                                                    | Source of current code                                         |
| ------------ | --------------------------- | --------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `native`     | library                     | Native host: catalog, provider, erased model, references, command execution       | Native execution modules from `harness` (not Cursor, not eval) |
| `eval`       | library (`publish = false`) | Lab-only workflow trial, adapter scenarios, grading, telemetry, sandbox, eval CLI | Eval modules from `harness`                                    |
| `lab`        | binary (`publish = false`)  | In-repo composition and argv dispatch                                             | Current `eval` binary, renamed                                 |
| `mock`       | library (`publish = false`) | Deterministic adapters, answer corpus, recording model, sessions                  | Existing `mock`, retargeted to `native`                        |

After migration there is no `harness` package. The current `eval` binary package moves to `lab`, and a new library package takes the freed `eval` name. This reuse is deliberate: `eval` is the established project term and `cargo make eval` remains the task surface.

The host crate is named `native`. This deliberately accepts overlap with `cfg(not(target_arch = "wasm32"))` and native tests; prose uses **native host** for this deployment and **native-only** for target constraints when ambiguity matters.

Cursor integration lives at composition roots (labs, and any future product binary), not inside `native`. Those roots may keep a private guest/host model bridge and parity tests until Omnia exposes one.

### Adapter composition

Wasm `specify` ships the engine and composes adapter components dynamically. The operator binary has no compile-time dependency on `specify-adapters`.

Native composition is static:

1. Specify owns `crates/native` and the native command API.
2. Specify ships no first-party-bound native executable; that would introduce a reverse dependency on `specify-adapters`.
3. Specify's `lab` uses `mock::catalog()`.
4. The adapter repository's `lab` depends on `eval`, `native`, and the concrete first-party adapters, and owns `catalog()` (plus a CI inventory check that every first-party adapter appears exactly once on its axis).
5. When a native operator distribution binary appears later, extract the shared catalog into a library both the lab and that binary consume. This RFC does not add that library.

```text
augentic/specify                         augentic/specify-adapters
────────────────                         ─────────────────────────
native host library ───────────────────► lab binary
eval library ──────────────────────────►   catalog() + command/eval dispatch
mock catalog ─► Specify lab             concrete first-party adapters
```

### Dependency enforcement

The Specify `checks` package enforces dependency direction by parsing Cargo manifests:

- `error`, `diagnostics`, `artifacts`, `adapter`, `project`, `slice`, `change`, and `transport` reject `native`, `eval`, `mock`, `lab`, and the removed `harness` in production and build dependencies;
- explicit dev-dependencies on `native` and `mock` remain legal for engine integration suites;
- `native` rejects `mock`, `eval`, `lab`, Cursor crates, and concrete adapter crates in production dependencies;
- `eval` depends on `native` plus deployment-neutral artifact/project types needed for grading, and rejects `mock`, `lab`, `change`, Cursor integration, and concrete adapter crates.

The adapter repository adds its own checks:

- the lab's default catalog contains every first-party adapter exactly once on its declared axis; any subset feature declares and tests its reduced expected inventory;
- published first-party adapter names remain globally unique across axes (store has no axis segment). Fixture's dual-axis `mock` name is Specify-local and unpublished.

Extend the existing `checks` manifest coverage with these rules, then retire `crates/harness/tests/boundary.rs`; the invariant moves rather than disappearing.

## Deployment-neutral seams

### Workflow core

Workflow semantics remain unchanged:

- handlers implement `omnia_guest::api::operation::Operation<P>`;
- each operation states its minimum capability intersection on `P`;
- orchestrators receive provider-carried capabilities;
- `transport` assembles typed command and HTTP routers;
- the adapter SDK owns `adapter::Source` and `adapter::Target`.

The engine core does not know whether model, source-adapter, target-adapter, or ensure/resolution capabilities are satisfied by WIT imports, native Rust implementations, scripted doubles, or a live backend.

### Adapter identity

The component deployment obtains identity from the component package and its resolved artifact location. The native host has no component artifact, so each SDK implementor must expose the equivalent compile-time identity.

```rust
pub struct AdapterIdentity {
    pub name: &'static str,
    pub version: &'static str,
}
```

`adapter::Source` and `adapter::Target` expose `const IDENTITY: AdapterIdentity` in place of a name-only constant. Their generic associated operation methods remain unchanged and non-object-safe.

- Published adapters set exact package version, normally from `env!("CARGO_PKG_VERSION")`.
- Unpublished mock/probe adapters may use a development placeholder version; they remain bare-only identities for pin matching.
- Resolve-time `SourceMetadata` / `TargetMetadata` remain non-identity metadata. Version does not move into the WIT `metadata` answer.
- Adapter crate tests assert that published identity name/version agree with the crate package and component publication configuration.

Workflow identity remains globally unique `(name, version)`, matching existing `SourceBinding`, `TargetRef`, and adapter-id wires. Package namespace appears only on `AdapterSelector::Package` for parse fidelity and optional provenance checks; it is not a second identity axis and is not stored on `AdapterIdentity`.

### Adapter selectors

Replace the lossy `{ name, version }` adapter reference with a selector that preserves the operator's input kind:

```rust
pub enum AdapterSelector {
    Bare { name: String },
    Package {
        namespace: String,
        name: String,
        version: semver::Version,
    },
    Component { path: PathBuf },
}
```

Parsing moves from `project::init::adapter_uri` into `project::adapter`, because source resolution, target resolution, initialization, and both providers share the grammar.

The selector preserves:

- a bare development request;
- the namespace and exact version of a package request;
- the fact that a local path is a component artifact, even when its filename matches a native adapter.

Parsing is syntactic. Local-file existence, canonicalization, and component validation remain ensure concerns, so a persisted local selector can still resolve through its project cache after the operator's original input file is removed. GitHub source URLs remain unsupported and fail during selector parsing rather than falling through as component paths.

`AdapterRef` is deleted rather than retained as a second lossy view. Persisted target values parse directly to `AdapterSelector`; successful ensure returns the existing `ResolvedSource` / `ResolvedTarget` shapes with the globally unique `(name, version)` engine identity plus opaque origin/provenance.

### Ensure (on `Resolver`, replaces `Hydrator`)

Delete `Hydrator`. Extend the existing resolver capability with axis-specific ensure methods:

```rust
pub trait Resolver: Send + Sync {
    fn ensure_source(
        &self,
        selector: &AdapterSelector,
        paths: &ExecutionPaths,
    ) -> impl Future<Output = Result<ResolvedSource, Error>> + Send;

    fn ensure_target(
        &self,
        selector: &AdapterSelector,
        paths: &ExecutionPaths,
    ) -> impl Future<Output = Result<ResolvedTarget, Error>> + Send;

    // Existing resolve_* methods remain for read-only re-resolution when useful,
    // or become thin wrappers over ensure without side effects.
}
```

Init, upgrade, and plan differences stay in handlers (`if upgrade { … }`); they are not a mode enum on the capability. No new `AdapterDeployment` trait and no new `ResolvedAdapter` axis enum.

- The **Wasm** `ensure_*` owns today's package fetch, store write, digest sidecar, verify-after-write, local component mirror, and development-probe policy. Deterministic fetch/store kernels remain in `project`; only the unconditional byte-oriented `Hydrator::fetch` entry path is removed.
- The **native** `ensure_*` performs no component I/O:
  - bare selector: match by name on the requested axis; persist the bare name as typed;
  - exact package selector: succeed only when `(name, version)` equals the compiled catalog entry (namespace may be checked against the expected first-party namespace, but is not part of engine identity);
  - version mismatch or absent entry: `adapter-not-linked`, including requested and available identities;
  - component selector: `adapter-not-linked`, stating that native execution does not load the supplied component.

Ensure and resolve receive `AdapterSelector`, so a persisted local component URI can never be silently narrowed to a bare native name.

Native initialization and upgrade persist the operator's selector as given. Resolve/ensure report the catalog entry's actual version in the resolved identity; they do not rewrite a bare binding into an exact package pin. Upgrade re-ensures the recorded binding and updates the Specify pin without silently changing the adapter selector.

### Adapter reference semantics

Native ensure is a static package match, not a component-store lookup:

- **Bare references** resolve against the active catalog and report the entry's actual version, not `0.0.0`.
- **Exact package references** resolve when the catalog contains that exact `(name, version)`.
- **Mismatched pins** fail as `adapter-not-linked`, naming the native version and pointing to a compatible native build or the Wasm deployment.
- **Local components** fail as unsupported by the native host before component cache writes occur.
- **Hydration and digests** do not apply to native entries.
- **Version floors** remain enforced at runtime. Rust compilation proves trait compatibility, not semantic compatibility with the adapter's declared `specify_floor`.

A project carrying an exact first-party adapter pin can move between providers once the active deployment has ensured that identity (`specify init --upgrade` installs a missing component; native ensure matches the catalog). The component deployment verifies component bytes; the native host asserts the package identity compiled into its catalog.

### Source binding resolution (Stage 1b)

Today `SourceBinding.version` is persisted but survey/extract dispatch ignores it and never calls ensure/resolve. That is a real correctness gap, but it is not on the critical path to extracting the native host: init/target ensure already needs selectors, and native catalog matching does not depend on plan-source pin enforcement.

Stage 1b closes the gap after Stage 1's shared seams land:

- plan-author source parsing accepts the first-party shorthand `<name>@<semver>` (implicit `specify` namespace) and materializes the existing `adapter` plus `version` fields;
- plan author ensures every source binding before survey;
- `source survey` and `source extract` ensure/resolve the binding again before dispatch, enforcing exact version and `specify_floor`;
- dispatch uses the resolved source name only after that succeeds.

Package namespace is not added to `SourceBinding`. Stage 1b may land before or after Stage 2; it must not block the `native` extract.

### Execution paths and cache isolation

The current eval helper mutates `SPECIFY_PROJECT_CACHE` under a scoped unsafe guard. That is not a sound configuration mechanism for a reusable multithreaded host.

The provider-carried path value gains an optional explicit cache parent alongside the project root:

```rust
pub struct ExecutionPaths {
    project_root: PathBuf,
    cache_parent: Option<PathBuf>,
}
```

The type lives in `project::handler` beside `Anchor`, exposes read-only accessors, and is re-exported by `native`. Cache resolution receives it rather than mutating process environment:

- native lab execution passes a canonical project root and inherits the operator's configured/default cache parent;
- eval and mock sessions pass a sandbox-local cache parent;
- the Wasm provider continues to resolve the guest cache mount;
- `SPECIFY_PROJECT_CACHE` remains a process-start configuration input, not a variable changed while tasks are running.

`ExecutionPaths::operator(root)` captures the process-start cache configuration; `ExecutionPaths::isolated(root, cache_parent)` supplies an explicit parent. The provider's anchoring capability exposes the value, and internal `Layout`/cache helpers carry the override from operation context. This is deployment configuration, not a test-only public API widening.

## The `native` package

`native` is the reusable native host used by engine tests, repository labs, and (later) any product binary that supplies its own catalog.

It owns:

- validated `Catalog` and typed source/target registration;
- `DynModel`;
- `Provider`, implementing project anchoring, ensure/resolve, model, engine source, and engine target capabilities;
- adapter-SDK to engine-seam conversion;
- asynchronous command-router execution over caller-supplied values;
- native reference routing and listener lifecycle.

It does not own:

- mock adapters or scripted answers;
- trials, scenarios, grading, telemetry, or sandbox orchestration;
- lab argv parsing or `--project-dir`;
- Tokio runtime creation;
- process-global cache mutation;
- a concrete adapter catalog;
- Cursor, Cursor options, or any Omnia host-model bridge;
- a hard-coded model choice.

### Erased model (`DynModel`)

`omnia_guest::Model` is not object-safe because `create` returns `impl Future`. `DynModel` uses one private object-safe trait returning a boxed future behind `Arc`.

Every adapter operation in a catalog is monomorphized once at `DynModel`. The SDK's generic source-adapter and target-adapter operation traits remain unchanged apart from their identity constant.

The cost is one vtable hop per model completion. The benefits are:

- `Catalog` and `Provider` carry no model type parameter;
- `Binding` and `adapters!` disappear;
- command and eval entrypoints consume values;
- conditional catalog activation is ordinary Rust;
- model middleware remains composable before erasure (`DynModel::new(Telemetry::new(backend))`).

`DynModel::new` accepts `impl Model + Send + Sync + 'static`. Composition roots erase exactly once, then `Provider::new` and command APIs accept `DynModel` directly.

Backends with post-run state expose it through caller-held clones:

```rust
let recording = mock::model::Harness::answering(answers); // rename optional later
let paths = ExecutionPaths::isolated(root, cache_parent);
let model = DynModel::new(recording.clone());
let provider = Provider::new(
    paths,
    model,
    mock::catalog()?,
    ReferenceMode::Offline,
);

// Run operations...
recording.assert_exhausted();
```

`Provider` exposes no model accessor.

If `Model` later gains a signature that cannot be boxed, only the private erasure module and catalog function-pointer aliases need reconsideration; adapter implementations remain generic.

### Catalog

The catalog is the native host's adapter declaration. Registration captures operation function pointers at `DynModel`:

```rust
fn catalog() -> Result<native::Catalog, native::Error> {
    native::Catalog::builder()
        .source::<intent::Adapter>()
        .target::<vectis::Adapter>()
        .build()
}
```

`build()` validates:

- duplicate entries on the same axis;
- malformed names and versions;
- published identities without exact SemVer;
- conflicting MCP shelf identities.

Cross-axis name collisions are **not** rejected by the native catalog. Dispatch is always axis-qualified (`source:mock` vs `target:mock`). Published first-party adapters must still keep globally unique names because the Wasm store has no axis segment; that invariant is enforced by adapter-repository checks, not by renaming mock. The default mock source and target both keep the identity name `mock`.

Runtime subsets are ordinary post-build filters (or builder-time `if` around `.source::<T>()`). Producing a smaller binary still requires Cargo feature/dependency gating at the lab (or later shared catalog library).

The catalog exposes a read-only inventory for diagnostics and build information but does not expose operation function pointers publicly.

### Command execution

Libraries do not construct or block Tokio runtimes. `native::command` exposes asynchronous APIs:

```rust
pub async fn execute(
    paths: ExecutionPaths,
    model: DynModel,
    catalog: Catalog,
    argv: Vec<String>,
) -> Result<CommandResponse, Error>;

pub async fn run(/* same values */) -> std::process::ExitCode;
```

`execute` builds a provider with online references, runs the shared router, and awaits reference-host shutdown before returning the typed transport response. `run` writes that response, renders native setup/router/shutdown failures, and returns the resulting exit code. Tests that need an offline provider assemble it directly.

The composition root owns:

- runtime construction;
- `std::env::args`;
- construction of `ExecutionPaths` from a canonical project root;
- model backend construction (including any Cursor client);
- catalog construction;
- rendering errors that occur before `native::command::run` is entered.

Lab-shaped entry:

```rust
#[tokio::main]
async fn main() -> std::process::ExitCode {
    render(entry().await)
}

async fn entry() -> Result<std::process::ExitCode, Box<dyn std::error::Error>> {
    let mut argv: Vec<String> = std::env::args().collect();
    let root = lab_project_root(&mut argv)?;
    let catalog = catalog()?; // mock::catalog() or first-party lab catalog()
    let options = CursorOptions::from_env(); // lab-local, not native

    if argv.get(1).is_some_and(|arg| arg == "eval") {
        let factory = cursor_factory(options);
        return Ok(eval::run(root, catalog, factory, &argv[1..], None).await?);
    }

    let paths = native::ExecutionPaths::operator(root);
    let model = native::DynModel::new(CursorModel::new(paths.project_root(), options));
    Ok(native::command::run(paths, model, catalog, argv).await)
}
```

`--project-dir` remains a lab convenience implemented by the lab binary before canonicalization. It is not exported by `native` or documented as product command surface.

### Features

- **always-on core** — catalog, identity validation, `DynModel`, conversion, `Provider`.
- **cli** — command execution and native reference hosting (`transport`, axum, Tokio networking).

There is no `cursor` feature on `native` and no `live` union feature. Labs that need Cursor depend on Cursor crates directly and erase into `DynModel`.

### Reference hosting

Adapter judgment requests carry MCP grants for embedded adapter references. Native execution serves those documents on loopback.

`Provider` is constructed once:

```rust
pub enum ReferenceMode {
    Offline,
    Online,
}

impl Provider {
    pub fn new(
        paths: ExecutionPaths,
        model: DynModel,
        catalog: Catalog,
        references: ReferenceMode,
    ) -> Self;
}
```

- offline mode never starts a listener and is explicit in deterministic tests;
- online mode starts lazily on the first adapter operation that has non-empty reference documents;
- a catalog with no reference documents remains a no-op;
- bind failure fails the operation rather than stripping grants;
- all provider clones share one listener;
- command execution requests graceful shutdown and awaits the server task on every exit path;
- embedders can call the same asynchronous shutdown method, while `Drop` retains an abort fallback;
- the listener binds only to `127.0.0.1`.

Reference shelf identity uses the adapter catalog entry's name and version, not the `native` crate version.

The source/target seam mirrors the closed WIT error shape, so a lazy listener failure crosses it as `project::seam::Error::Internal` with the stable detail prefix `reference-listener-unavailable`; transport keeps its existing `seam-dispatch-failed` outer code. Direct `ReferenceHost` startup retains a typed native error. This RFC does not widen the WIT error variant solely for host diagnostics.

The reference module exposes router construction separately from ephemeral listener ownership so a later native HTTP host can mount the same routes without a second implementation.

### Concurrency posture

The initial command path is single-flight:

- one command runs to completion per provider graph, and the supplied lab binaries run one command per process;
- `Provider: Clone` supports router invocation and shared capabilities, not concurrent independent commands;
- embedders needing concurrency create independent providers and cache/reference contexts;
- cancellation and long-running serve mode are future work.

This posture is documented rather than encoded as process-global singletons.

## The `eval` package

`eval` is a lab-only library over `native` with its `cli` feature. It owns:

- the multi-step live workflow eval;
- single-operation adapter scenarios;
- deterministic grading;
- model-request telemetry;
- sandbox seeding and cleanup;
- eval CLI parsing.

It does not:

- construct a concrete adapter catalog;
- depend on mock or first-party adapter crates;
- construct a Cursor backend;
- create or block a Tokio runtime;
- mutate process-global cache environment;
- depend directly on `change` merely to bypass the command surface.

Its process-facing client supplies a catalog and a model factory:

```rust
pub struct ModelInstance {
    pub model: native::DynModel,
    pub default_model: Option<String>,
}

pub type ModelFactory = Arc<
    dyn Fn(&Path) -> Result<ModelInstance, Error> + Send + Sync,
>;

pub async fn run(
    workspace_root: PathBuf,
    catalog: native::Catalog,
    model: ModelFactory,
    args: &[String],
    scenarios: Option<&Path>,
) -> Result<std::process::ExitCode, Error>;
```

`workspace_root` anchors the persistent `sandbox/` tree and relative scenario roots; eval does not consult process current-directory state after entry. The library wraps `ModelInstance.model` in `Telemetry`, gives telemetry the configured default for effective-model reporting, keeps its own clone for read-back, and drives workflow phases through `native::command::execute`. It uses public project/artifact types only where deterministic grading needs structured state.

Scenario reports derive the effective model from each observed request (`request.model` when present, otherwise `ModelInstance.default_model`), not by rereading environment variables. `EVAL_MODEL` remains the composition-root env var until an optional later rename.

Eval scratch state receives an explicit sandbox-local cache parent. The persistent full-trial sandbox is single-writer and guarded against a second concurrent eval in the same checkout; per-scenario run directories remain unique.

## Lab binaries

Each repository owns an unpublished `lab` binary. It creates one Tokio runtime through its async `main`, parses its own lab-only arguments, constructs any Cursor backend, owns its catalog declaration, and dispatches visibly between native command mode and `eval::run`.

- ordinary arguments run a Specify command through `native`;
- `eval` runs the shared eval client;
- `--project-dir` is parsed only here;
- when placed before `eval`, `--project-dir` intentionally anchors that eval's `sandbox/` instead of relying on process current directory;
- the binary is the target of `cargo make specify` and `cargo make eval`;
- it is never an install or release artifact.

The two repositories intentionally duplicate the small dispatch. The adapters lab also owns the first-party `catalog()` declaration and depends on the concrete adapter crates directly. A shared catalog library appears only when a second consumer needs it.

Integration tests assemble an offline `native::Provider` directly and never require a lab binary.

## Adapter-repository composition

`augentic/specify-adapters` consumes Specify's `adapter`, `native`, and lab-only `eval` crates one-way.

| Target | Role                                     | Entry                         | Catalog                      |
| ------ | ---------------------------------------- | ----------------------------- | ---------------------------- |
| `lab`  | Dev command + eval + first-party catalog | `catalog()` + inline dispatch | owns the first-party catalog |

The wasm example remains a separate Wasm gate. A future native product binary would extract the catalog into a shared library; that extraction is out of scope here.

## Wasm and native correspondence

| Concern                | Wasm deployment                          | Native host                                              |
| ---------------------- | ---------------------------------------- | -------------------------------------------------------- |
| Specify product        | Existing released `specify` distribution | Follow-up distribution; lab owns catalog until then      |
| Workflow composition   | `src/lib.rs` + WIT-backed provider       | `native::Provider` + async command assembly              |
| Adapter declaration    | component deployment configuration       | validated `Catalog` value                                |
| Adapter identity       | package/store identity                   | compile-time SDK `(name, version)`                       |
| Adapter selection      | `Resolver::ensure_*` (component policy)  | `Resolver::ensure_*` (catalog match)                     |
| Engine invocation      | `Invoker` + shared transport router      | same                                                     |
| Model access           | `omnia:model/completion` host import     | composition-root `DynModel`                              |
| Adapter dispatch       | WIT source/target imports                | function-pointer table at `DynModel`                     |
| References             | adapter HTTP guest routed by Omnia       | owned loopback `ReferenceHost`                           |
| Execution paths        | shared Wasm project/cache preopens       | canonical project root + explicit/inherited cache parent |
| Isolation              | component instance per call              | trusted code in one native process                       |
| Exact adapter pin      | store entry + digest verification        | succeeds only on exact compiled identity                 |
| Local `.wasm` selector | supported by component ensure            | rejected before cache mutation                           |
| Lab composition        | none                                     | unpublished `lab` + `eval`                               |

Observable behavior shared across providers:

- command grammar, output shapes, and exit codes;
- workflow artifacts and lifecycle transitions;
- adapter operation order;
- model request/answer schemas;
- MCP reference contents;
- report and validation gates.

Behavior intentionally specific to a provider:

- component ABI and WIT mapping;
- component instance isolation;
- dynamic component provisioning and hydration;
- global adapter-store lookup and digest verification;
- static catalog inventory and native trust boundary;
- exact native package set.

## Testing model

### Workflow integration tests

```text
offline native::Provider
  ├── mock catalog
  ├── temporary project root + explicit cache parent
  └── mock recording model over omnia_testkit::model::Scripted
      └── caller-held clone for requests / exhaustion
```

CLI-reachable workflow behavior goes through the transport router. Native host dispatch tests and single-adapter eval scenarios call the public provider capability traits directly. No second public `native::invoke` wrapper is introduced.

### Native host tests

`crates/native/tests/` uses crate-local probe implementors and covers:

- `DynModel` forwarding and shared state;
- catalog identity validation and per-axis duplicate rejection;
- source/target function-pointer dispatch;
- bare and exact-pin ensure;
- mismatched-pin and component-selector refusal;
- runtime `specify_floor` enforcement;
- reference no-op, bind failure, grant routing, and shutdown.

No native production dependency on mock is introduced. Same-name source and target probe entries are allowed (mirroring mock).

### Eval tests

`crates/eval/tests/` uses injected scripted model factories and covers:

- argument handling;
- scenario loading;
- deterministic grading;
- telemetry;
- sandbox locking and unique run directories;
- command-response handling.

### Fixture tests

Fixture behavior, answer recording, and catalog inventory remain under `crates/mock/tests/`. `Session` stores its recording model beside the provider and uses an explicit temporary cache parent. Renaming `mock::model::Harness` is optional polish, not required by this RFC. Do **not** rename the default mock source/target identity `mock`.

### Wasm boundary tests

No native test claims Wasm coverage. Existing component gates remain:

- adapter crate tests;
- the Specify mock wasm example;
- the first-party wasm example over the published engine component.

## Module migration

| Current module           | Target                            | Notes                                                                      |
| ------------------------ | --------------------------------- | -------------------------------------------------------------------------- |
| `catalog.rs`             | `crates/native/src/catalog.rs`    | identity-aware builder at `DynModel`; `Binding` and `adapters!` deleted    |
| —                        | `crates/native/src/model.rs`      | `DynModel` and private erased-model trait                                  |
| `convert.rs`             | `crates/native/src/convert.rs`    | private SDK/engine DTO mapping                                             |
| `provider.rs`            | `crates/native/src/provider.rs`   | non-generic provider; `ReferenceMode`; no model accessor                   |
| `command.rs`             | `crates/native/src/command.rs`    | async `execute` / `run`; no runtime or lab argv parsing                    |
| `mcp.rs`                 | `crates/native/src/references.rs` | router plus owned lazy `ReferenceHost`                                     |
| `model.rs` / `native.rs` | private lab modules               | Cursor bridge leaves `native`                                              |
| `invoke.rs`              | —                                 | public duplication removed; call Omnia `Invoker` or command API directly   |
| `env.rs`                 | —                                 | process mutation removed; cache parent enters execution context explicitly |
| `entry.rs`               | —                                 | runtime and dispatch move to lab binaries                                  |
| `fs.rs`                  | `crates/eval/src/fs.rs`           | eval tree copy                                                             |
| `grade.rs`               | `crates/eval/src/grade.rs`        | deterministic grading                                                      |
| `sandbox.rs`             | `crates/eval/src/sandbox.rs`      | sandbox and single-writer guard                                            |
| `scenario.rs`            | `crates/eval/src/scenario.rs`     | prompt scenarios over supplied catalog/model factory                       |
| `telemetry.rs`           | `crates/eval/src/telemetry.rs`    | caller-held model request counts                                           |
| `trial.rs`               | `crates/eval/src/run.rs`          | full eval workflow                                                         |

Move the current `crates/eval` binary to `crates/lab`, then create the new `crates/eval` library. User-facing strings naming the native harness or shim become native-host language.

## Cargo and feature layout

### Specify workspace

```toml
native = { path = "crates/native" }
eval = { path = "crates/eval" }
```

Remove the `harness` workspace dependency. `lab` needs no workspace-dependency entry.

`native`:

```toml
[features]
default = []
cli = [
    "dep:axum",
    "dep:tokio",
    "dep:transport",
    "tokio/net",
    "tokio/rt",
    "tokio/sync",
]
```

The actual default dependency list still includes the workspace contracts required by catalog/provider code; `default = []` means no optional command stack. There is no `cursor` feature.

`eval` enables `native/cli` and accepts model factories.

`lab` enables `cli` and depends on Cursor crates directly:

```toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
eval.workspace = true
mock.workspace = true
native = { workspace = true, features = ["cli"] }
omnia-cursor.workspace = true
# plus whatever the private Cursor bridge needs
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
```

Both `eval` and `lab` are `publish = false` and never attached to a release.

### Adapter workspace

Replace the harness dependency with:

```toml
adapter = { git = "https://github.com/augentic/specify.git" }
native = { git = "https://github.com/augentic/specify.git" }
eval = { git = "https://github.com/augentic/specify.git" }
```

These are root `[workspace.dependencies]` declarations, not dependencies of every member:

| Adapter-repository member    | Specify dependencies                                                   |
| ---------------------------- | ---------------------------------------------------------------------- |
| source/target adapter crates | `adapter`                                                              |
| `lab`                        | `eval`, `native/cli`, Cursor crates, and concrete first-party adapters |

- Rename current `crates/eval` to `crates/lab`, moving scenarios with it.
- Put first-party `catalog()` in the lab and add a CI inventory check.
- Update sibling path patches.

## Command surface

Operator-facing workflow verbs and wire outputs do not change in this RFC. Stage 5 may neutralize help wording that falsely claims guest/component-only behavior; `--version` flavor labels remain optional polish.

Development tasks retain their names but target the lab:

- `cargo make specify -- ARGS` — lab command passthrough;
- `cargo make eval` — lab eval command;
- `cargo make eval scenario <id>` — one prompt scenario;
- `cargo make wasm-run` — Wasm composed example.

Package selection for lab tasks is `cargo run -p lab`. No installation or release documentation points operators at that package.

## Implementation plan

Stages are dependency order. Prefer small mergeable cuts over one flag day. While Stage 2 leaves eval modules in `harness`, that package temporarily depends on `native` and retains the existing `--project-dir` forwarding shim. Stage 3 moves the current `eval` binary to `lab`, creates the new `eval` library, switches task targets, and only then removes the shim and `harness`; Stages 2 and 3 may land atomically when that is simpler.

### Stage 1 — Shared seams the native host needs

1. Add SDK `AdapterIdentity { name, version }`; update source-adapter and target-adapter implementors in both repositories. Keep the default mock name `mock` on both axes.
2. Delete the lossy `AdapterRef`; parse persisted/raw values directly into typed `AdapterSelector` in `project::adapter`.
3. Delete `Hydrator`; add `Resolver::ensure_source` / `ensure_target` over `AdapterSelector` and `ExecutionPaths`. Keep current deterministic component fetch/store/cache kernels behind the Wasm implementation. Init/upgrade/plan policy stays in handlers.
4. Make ensure/resolve preserve selector kind and return actual resolved identity for the selectors init and target resolution already consume. Persist selectors as typed; do not rewrite bare to exact package pins.
5. Add explicit cache-parent plumbing from provider execution context to cache resolution; remove scoped runtime environment mutation.
6. Preserve existing Wasm initialization, hydration, store, digest, and command behavior through component-provider integration tests.

Deferred from this stage (follow-ups, not blockers):

- source-binding pin enforcement at plan author / survey / extract (Stage 1b);
- shared help neutralization and `--version` flavor labels (Stage 5);
- package-namespace recording in component store sidecars;
- multi-namespace store collision policy;
- any native operator product binary or shared catalog library;
- cosmetic renames (`EVAL_MODEL`, `Harness`, `DevModel`).

### Stage 1b — Source binding pin enforcement

Depends on Stage 1's `AdapterSelector` and `ensure_*`. Does not block Stage 2.

1. Accept `<name>@<semver>` at plan-author source parsing into the existing `adapter` / `version` wire fields.
2. Ensure every plan source binding before survey.
3. Ensure/resolve again in `source survey` and `source extract` before dispatch; enforce exact version and `specify_floor`.
4. Keep the `SourceBinding` schema unchanged; add no package namespace field.

### Stage 2 — Extract the native host

1. Add `crates/native` and move catalog, conversion, provider, command, and reference code from `harness`.
2. Introduce `DynModel`; remove the model type parameter from `Catalog` and `Provider`.
3. Delete `Binding` and `adapters!`; make mock catalogs plain validated values.
4. Implement native ensure over exact catalog identities; catalog validation is per-axis only; bare selectors persist as bare.
5. Make command execution asynchronous and value-consuming; composition roots own Tokio.
6. Add owned lazy reference hosting with `ReferenceMode`, explicit failure projection, and awaited shutdown.
7. Retarget `mock` and engine suites from `harness` to `native`.
8. Move native host tests and replace `Provider::model()` call sites with caller-held recording/telemetry handles.
9. Leave Cursor bridge code in the temporary harness/lab path until Stage 3; do not import it into `native`.

### Stage 3 — Eval and lab

1. Move the current `crates/eval` binary to `crates/lab`, then create a new `crates/eval` library from trial, scenario, grading, telemetry, sandbox, and tree-copy modules.
2. Inject catalog and model factory values; remove concrete Cursor and adapter construction from `eval`.
3. Route workflow phases through the native command API and keep only grading-required project/artifact dependencies.
4. Inline async dispatch, lab-only project-root parsing, Cursor model construction, and catalog declaration in `crates/lab`.
5. Remove the emptied `crates/harness`.
6. Point `cargo make specify` and `cargo make eval` at `-p lab`.

### Stage 4 — Adapter repository

1. Pin a Specify revision exposing `adapter`, `native`, and `eval`.
2. Update every first-party adapter identity and native test support.
3. Rename adapter `eval` to `lab`, flip it to `publish = false`, declare first-party `catalog()` in the lab, depend on concrete adapters, and construct the Cursor model in the lab.
4. Add a catalog inventory check (every first-party adapter exactly once on its axis; global published-name uniqueness across axes).
5. Confirm no reverse dependency from Specify to `specify-adapters`.

### Stage 5 — Documentation and checks

Update both repositories' `AGENTS.md`, adapter `TESTING.md`, Specify testing/architecture/workflow standards, quality-gate docs, Makefiles, CLI help, and rustdoc.

Document:

- engine core with Wasm provider and native host;
- native exact-pin matching and component-selector refusal;
- native adapters as trusted in-process code;
- lab/eval as unpublished tooling;
- native tests as non-WIT/non-store coverage;
- adapters lab as the first-party catalog owner until a product binary needs a shared library;
- native operator distribution as unresolved follow-up;
- mock's dual-axis `mock` name as intentional and unpublished.

Optionally neutralize shared help wording and add a composition-supplied `--version` label. Cosmetic renames remain optional.

Extend Specify and adapters checks with the dependency rules above.

### Stage 6 — Verification

In `augentic/specify`:

```bash
cargo make check
cargo make ci
cargo make specify -- --help
cargo make specify -- --version
cargo make eval
cargo check --lib -p specify --examples --target wasm32-wasip2
cargo make wasm-run
```

In `augentic/specify-adapters`:

```bash
cargo make check
cargo make ci
cargo make specify -- --help
cargo make specify -- --version
cargo make eval
cargo make wasm-run
```

Live-model and composed wasm-run commands remain operator-invoked when credentials are unavailable.

## Acceptance criteria

1. Documentation presents Specify's engine core with a Wasm provider and a native host, without claiming native component/WIT/store properties.
2. `adapter::Source` and `adapter::Target` expose validated static `AdapterIdentity { name, version }` while retaining generic associated operation methods.
3. `AdapterSelector` preserves bare, package, and component input kinds through ensure; the lossy `AdapterRef` no longer exists.
4. `Hydrator` is gone; `Resolver::ensure_source` / `ensure_target` own deployment policy. There is no `AdapterDeployment` trait.
5. The Wasm deployment preserves package hydration, local component caching, store lookup, digest verification, and existing init artifacts.
6. The native host resolves bare names to actual catalog versions and satisfies exact package pins present in the catalog.
7. Mismatched pins and local component selectors fail as `adapter-not-linked`; a local path can never select a same-named compiled adapter.
8. Native init and upgrade persist the operator's selector as typed; bare bindings are not rewritten into exact package pins.
9. The runtime `specify_floor` gate remains active for native catalog entries at ensure time.
10. `Catalog` and `Provider` carry no model type parameter; no `Binding` trait or `adapters!` macro remains.
11. Catalog construction validates identities, per-axis duplicates, and reference shelf coherence; same-name source and target entries remain legal (mock keeps `mock` on both axes).
12. `native` contains no concrete adapter, mock, eval, lab, or Cursor dependency in production dependencies.
13. Command execution accepts execution paths, model, catalog, and argv values; libraries do not construct Tokio runtimes.
14. Lab command execution anchors at a canonical project root; lab-only `--project-dir` is canonicalized before provider construction.
15. Cache isolation is explicit in execution context; no native/eval/mock path mutates `SPECIFY_PROJECT_CACHE` after runtime startup.
16. Online providers fail loudly when reference documents cannot be served, no-op for no-doc catalogs, share one listener across clones, and expose an awaited shutdown path used by command execution.
17. `eval` receives workspace root, catalog, and model factory values, constructs neither concrete adapters nor Cursor backends, and creates no runtime.
18. Scenario reports derive the effective model from observed requests plus the factory-supplied default.
19. Specify engine integration tests use offline `native` plus mock adapters and caller-held recording-model handles.
20. The adapter repository's lab owns first-party `catalog()`; this RFC adds no `composition` package.
21. Both labs remain unpublished and are never documented as install paths.
22. `cargo make specify`, `cargo make eval`, and prompt scenarios preserve their lab behavior.
23. The Wasm engine guest, component manifests, shipped Wasm runtime behavior, and current Wasm release surface remain intact.
24. Native tests explicitly avoid claiming component ABI, WIT, isolation, digest, or adapter-store coverage.
25. The native command path is documented as single-flight; eval guards its persistent sandbox against concurrent writers.
26. Specify and adapter-repository checks enforce their respective dependency and catalog boundaries.
27. Full local CI passes in both repositories, or unavailable live gates are reported precisely.
28. Documentation leaves native operator distribution and any shared catalog-library extraction to follow-up work.

Stage 1b acceptance (does not block Stages 2–6):

29. Plan author ensures source bindings, while survey/extract ensure/resolve them again; source pins and `specify_floor` can no longer bypass deployment policy on the source axis.

## Risks and mitigations

### Native behavior is mistaken for Wasm conformance

Mitigation: keep component boundary gates explicit and document native tests as workflow/host coverage only.

### Native adapters have full process authority

Native adapter code can access process environment and filesystem beyond the provided context, panic the command, and share dependency/global state with other adapters.

Mitigation: treat catalog entries as trusted code, document the trust boundary, keep untrusted/dynamic adapters on the Wasm deployment, and preserve separate component gates.

### Static identity is asserted rather than digest-verified

The native catalog claims package identity from compiled Rust code; it does not prove equivalence to published component bytes.

Mitigation: validate identity against crate/package configuration now. Build provenance and release identity belong to future distribution work.

### Product identity remains undefined

This RFC extracts the host and leaves the first-party catalog on the adapters lab. It does not decide a native operator package name, installed binary name, coexistence with the Wasm distribution, or release cadence.

Mitigation: never route operators to the lab, and extract a shared catalog library only when a second consumer (a product binary) appears.

### Lab-owned catalog drifts when a product binary appears

Mitigation: the extraction is mechanical—one `catalog()` function and its inventory test move into a shared library both binaries depend on. Do that in the distribution RFC, not preemptively.

### Component ensure regresses

Moving byte hydration behind `ensure_*` could accidentally alter existing store/cache/init behavior.

Mitigation: land the shared selector/ensure stage first and pin current component behavior through CLI integration tests before extracting native.

### Cursor dependencies leak into ordinary tests

Mitigation: Cursor stays out of `native` and `eval`; labs construct backends and erase into `DynModel`.

### Eval regains concrete adapter or model dependencies

Mitigation: eval consumes catalog and model factory values; manifest checks reject concrete adapters, mock, and Cursor dependencies.

### Reference listeners leak or silently disappear

Mitigation: one owned lazy `ReferenceHost` per online provider graph, explicit bind failures with a stable detail prefix, shared ownership, and shutdown tests.

### Explicit cache plumbing expands the core change

Removing process-global mutation touches operation context and cache call sites.

Mitigation: keep the value small and path-only, preserve environment lookup at process startup, and land it before the crate migration so failures stay local.

### `omnia_guest::Model` grows an erasure-resistant surface

A borrowed streaming or generic-return API may resist boxing.

Mitigation: the erasure remains one private native module; adapters stay generic and require no change if a composition root later uses a different host shape.

### Cross-repository development becomes lockstep

Mitigation: revision-pin `adapter`, `native`, and `eval`; keep sibling path patches for co-development only; validate the first-party catalog in adapter CI.

### Single-flight assumptions are violated

Mitigation: document one command/provider graph per process, use independent providers for embedding, and guard eval's persistent sandbox.

## Alternatives considered

### Keep the current crate names

Rejected. `harness` conflates host and eval, while the current `eval` package names a composition binary. Moving that binary to `lab` frees the established `eval` term for the reusable library.

### Keep the model-generic catalog behind `Binding` and `adapters!`

Rejected. The catalog already erases adapter operations; retaining the model generic preserves a factory trait, macro, generic provider, and generic entrypoints to avoid one vtable hop on network-bound calls.

### Reject every pinned reference in native execution

Rejected. Bare names are development shorthand while exact pins are production identities. A native catalog has enough information to satisfy its own exact compiled identity. Refusing all pins makes ordinary projects deployment-bound and reports native adapters as `0.0.0`.

### Put native version identity in adapter metadata

Rejected. Identity and non-identity metadata remain separate. Component identity comes from its package/artifact; native identity comes from the SDK implementor's static descriptor. The WIT metadata answer continues to carry only compatibility, inputs, and platforms.

### Keep `{ name, version }` references and low-level `Hydrator::fetch`

Rejected. The reduced reference loses local-component provenance, and initialization invokes component policy before native matching. A typed selector plus `Resolver::ensure_*` makes each provider own its legal inputs.

### Add an `AdapterDeployment` / `Provisioner` trait

Rejected. Extending `Resolver` with `ensure_source` / `ensure_target` keeps one familiar capability. Init/upgrade/plan differences belong in handlers.

### Bundle source-binding pin enforcement into Stage 1

Rejected. Closing the survey/extract pin gap is valuable, but init/target `ensure` and `ExecutionPaths` are enough to extract `native`. Stage 1b depends on those seams and must not block Stage 2.

### Rewrite bare native init selectors to exact package pins

Rejected. Persist what the operator typed; resolve reports the catalog version. Silent rewrite fights upgrade's preserve-binding rule and adds policy without unlocking the host extract.

### Put namespace on `AdapterIdentity` and store sidecars now

Deferred. Workflow identity is `(name, version)`. Namespace on `AdapterSelector::Package` preserves parse fidelity. Store-sidecar provenance is Wasm follow-up and is not required to extract the native host.

### Rename mock source to `mock-source`

Rejected. The default mock intentionally shares the name `mock` on both axes, matching the Wasm guest. Native catalog uniqueness is per-axis; published global uniqueness stays an adapters-repo check.

### Keep scoped process-environment cache mutation

Rejected. It is unsafe in a multithreaded reusable host and cannot support independent providers in one process. Cache placement is an execution-context value.

### Let libraries create Tokio runtimes

Rejected. A sync `main` helper is convenient for one binary but fails under an existing runtime and splits runtime policy between command and eval libraries. Composition roots already exist and should own runtime creation.

### Name the eval library `trial`

Rejected. The package owns trials, scenarios, grading, telemetry, and sandboxing. `eval` is the established umbrella and already matches the task name.

### Fold eval into `native` behind a feature

Rejected. It blurs the host with lab machinery and weakens dependency enforcement. A command-only composition must not pull eval code.

### Put Cursor behind a `native` feature

Rejected. The host only needs `DynModel`. Cursor crates at the composition root keep tests free of vendor features and leave bridge upstreaming outside `native`'s public surface.

### Add a `composition` package before a product binary exists

Rejected for this cut. With one consumer, the adapters lab can own `catalog()` and the inventory check. Extract a shared library when a second consumer appears; that move is mechanical.

### Put a product-shaped composition executable in this RFC

Deferred. Distribution identity is undecided. Lab-owned catalog is enough until then.

### Put the first-party native executable in Specify

Rejected. It would create a Specify-to-`specify-adapters` dependency or move adapters in-tree.

### Treat the lab as the native product

Rejected. The lab includes eval UX and scratch behavior. It is unpublished tooling, not the operator distribution.

### Put `--version` flavor labels and help neutralization in Stage 1

Rejected. They are documentation polish. Stage 1 is identity, selector, `ensure`, and `ExecutionPaths`.

### Add a `DeploymentFlavor` enum to transport

Rejected. If a version label is added later, an opaque string from the composition root is enough and keeps transport deployment-ignorant.

### Move the model bridge to Omnia now

Deferred. Labs may keep a private bridge. Upstreaming it is not required to separate Specify's host and eval boundaries.

### Move `native` to a separate repository

Rejected. It evolves atomically with workflow, transport, and adapter SDK contracts; Specify's tests consume it directly.

## Consequences

- Specify has one engine core with a Wasm provider and a native host.
- `native` is a reusable host library; `eval` and `lab` are lab-only; Cursor stays at composition roots.
- Composition is value-level end to end: catalog, model, execution paths, and argv.
- Adapter selectors retain their input kind; `Resolver::ensure_*` is the deployment policy entry.
- Native init persists selectors as typed; resolve reports catalog versions without rewriting bare pins.
- A native catalog carries real package identities, satisfies exact matching pins, and rejects local components without substitution.
- Catalog uniqueness is per-axis; mock keeps dual-axis `mock`; published global uniqueness remains an adapters check.
- `DynModel` removes the generic binding tower while leaving adapter operation methods generic.
- Command and eval libraries are asynchronous; composition roots own Tokio.
- MCP listener and cache lifecycles become explicit rather than detached or process-mutating.
- The adapters lab owns the first-party catalog until a product binary needs a shared library.
- Native source adapters and target adapters are trusted in-process code, not isolated components.
- The Wasm deployment remains necessary and authoritative for WIT, isolation, dynamic provisioning, store, and digest behavior.
- Native operator distribution remains a deliberate future concern.
