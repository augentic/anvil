# Linked Specify — Native Deployment, Eval, and Lab Composition

> Status: Draft
>
> Owns: the Wasm-free, statically composed deployment of the Specify engine; the deployment-neutral adapter identity and provisioning seams it requires; and its separation from live-model eval and in-repo lab composition.

## Abstract

Specify is one operator product with two deployment flavors over the same workflow core and command contract:

- the existing **Wasm deployment** composes the workflow and adapters as components through Omnia;
- the **linked deployment** compiles the workflow core and selected Rust adapter libraries into one process.

The current `crates/harness` already approximates the linked deployment for native workflow tests, development commands, and live-model eval, but its architecture is shaped by those first consumers. This RFC replaces it with:

- `linked` — the reusable linked-deployment host library;
- `eval` — the lab-only library for trials, scenarios, grading, and telemetry;
- `lab` — each repository's unpublished composition binary.

Composition is value-level. A product or lab binary supplies a validated `Catalog` and a model backend; `linked` carries no repository binding trait, registration macro, or model type parameter. The first-party linked composition lives in `augentic/specify-adapters`, where the source adapters and target adapters already live. Its library target owns the first-party catalog, its executable drives the linked command entry, and the adapter repository's lab reuses that same catalog.

```text
ONE SPECIFY PRODUCT
┌──────────────────────────────┬──────────────────────────────┐
│ Wasm deployment              │ Linked deployment            │
│                              │                              │
│ Omnia runtime                │ composition executable       │
│ workflow + adapter guests    │ linked host + adapter rlibs  │
│ WIT / store / digest         │ Catalog / DynModel / MCP     │
└──────────────────────────────┴──────────────────────────────┘

LAB ONLY
┌──────────────────────────────┬──────────────────────────────┐
│ lab                         │ eval                         │
│ repository composition      │ trials / scenarios / grading│
│ command + eval dispatch     │ model telemetry             │
└──────────────────────────────┴──────────────────────────────┘
```

The Wasm deployment remains authoritative for component loading, WIT conformance, isolation, digests, and adapter-store behavior. Linked tests never claim those properties.

This RFC defines the linked composition boundary and the shape of a first-party executable, but deliberately does not choose its distribution package name, archive identity, installation precedence, bundle version, or release cadence. Those product-distribution decisions remain a follow-up concern.

## Motivation

The current package names describe their first consumers rather than their architectural roles:

- `crates/harness` contains the native provider, adapter catalog, engine-to-adapter conversion, Cursor model bridge, MCP reference host, typed invocation helpers, command router, process-environment test support, live eval trials, scenarios, grading, telemetry, and sandbox management.
- `crates/fixture` contains deterministic source and target adapters, scripted model answers, a request-recording model decorator, and native project sessions.
- `crates/eval` is the concrete native composition binary over the fixture adapters.

This makes a coherent linked deployment look like accumulated testing machinery. It also hides the correspondence between the two deployment flavors:

```text
Wasm deployment                              Linked deployment

src/lib.rs + src/provider.rs                 linked::Provider + command assembly
component deployment configuration           composition library's Catalog value
adapter components                           linked adapter libraries
host model import                            composition-root model value
adapter HTTP references                      linked ReferenceHost
```

A second muddle is mechanical. The harness catalog already erases source-adapter and target-adapter operations into `BoxFuture` function pointers, but leaves the model type generic. That one generic forces `Catalog<M>`, `Provider<M>`, a for-all-model `Binding` trait, the `adapters!` registration macro, and generic command/eval entrypoints. The tower exists to avoid one reference-counted vtable hop on network-bound model completions.

A third muddle is architectural. Adapter references are currently reduced too early to `{ name, version }`, while initialization separately knows whether the operator supplied a bare name, package reference, or local component path. The linked provider therefore cannot distinguish a local `.wasm` selector from a bare name after parsing, and the low-level `Hydrator::fetch` capability forces linked initialization through component-oriented provisioning before linked resolution can refuse or satisfy the selector.

The linked deployment makes those concerns visible:

- deployment-neutral workflow behavior belongs in the workflow core;
- adapter selector parsing and resolved identity belong in shared contracts;
- component provisioning and linked catalog matching are deployment policies;
- networking and cache isolation need explicit lifecycle ownership;
- eval is a client of the linked host, not part of the product host.

## Goals

1. Establish the linked flavor as a first-class Specify deployment over the same workflow core and command contract as the Wasm flavor.
2. Keep the linked host out of the workflow core; workflow crates depend only on capability contracts and deployment-neutral adapter types.
3. Preserve dependency inversion: `linked` never depends on `eval`, `lab`, `fixture`, or `augentic/specify-adapters`.
4. Compose with values: catalogs and model backends are supplied by product and lab composition roots; the host carries no binding trait, registration macro, or model type parameter.
5. Make the linked catalog a complete static deployment declaration, including each adapter's package identity, operation axis, metadata, and embedded references.
6. Let the linked deployment satisfy an exact adapter pin when the compiled catalog contains that exact package identity; reject mismatches and local component selectors without silently substituting another implementation.
7. Move adapter provisioning policy behind a deployment capability so initialization and plan authoring do not run component hydration before linked catalog resolution.
8. Preserve workflow command I/O, exit codes, artifacts, lifecycle transitions, adapter operation order, and model request/answer shapes across deployment flavors; flavor reporting, selector diagnostics, and linked provisioning remain the explicit divergences.
9. Keep Specify's workflow integration tests self-contained over local fixture adapters through `linked`'s library API.
10. Separate command execution from live-model eval: command mode lives in `linked`; `eval` is optional and lab-only.
11. Make runtime values and resources explicit: canonical project root, cache root, model configuration, MCP listener ownership, and deployment flavor.
12. Keep one linked implementation of provider dispatch, model bridging, catalog construction, and MCP reference serving, with adapter operations monomorphized at exactly one model type.

## Non-goals

- Replacing the Wasm deployment.
- Loading `.wasm` components from the linked deployment.
- Claiming WIT, component ABI, isolation, digest, or adapter-store coverage from linked tests.
- Moving first-party adapter implementations into the Specify repository.
- Making Specify depend on the sibling `specify-adapters` checkout.
- Changing workflow semantics, artifact schemas, lifecycle transitions, or prompts.
- Making `adapter::Source` / `adapter::Target` object-safe; their operation methods remain generic associated functions. This RFC only adds static identity to their contract and erases the host's model handle.
- Guaranteeing a fully static executable. The linked deployment statically composes the Rust workflow and adapters but may still use platform libraries and the external Cursor backend.
- Supporting two published adapters with the same name under different package namespaces. Adapter names remain globally unique across namespaces and axes; namespace selects registry provenance rather than creating a second workflow identity.
- Defining the linked distribution's final package name, archive naming, installation precedence, bundle version, release cadence, or release-pipeline matrix. The internal source package is named `composition`; that name is not a distribution decision. The architecture must permit later choices without making the lab an install path.
- Serving the HTTP transport from the linked deployment. The resource and provider design must not prevent a later HTTP host.
- Adding cooperative cancellation or concurrent multi-command service semantics. The initial linked command path is one command to completion per process.
- Upstreaming the guest-model/host-model bridge into Omnia in this change. The linked copy remains private and parity-tested until an upstream bridge exists.
- Adding compatibility aliases for removed internal crate names.
- Widening public workflow APIs solely to support tests.

## Terminology

- **Workflow core** — the deployment-neutral engine crates: `project`, `slice`, `change`, and `transport`, plus their dependency leaves.
- **Wasm deployment** — the existing Omnia-hosted flavor: native host process plus workflow and adapter Wasm guests. Authoritative for component loading, WIT, isolation, digests, and the adapter store.
- **Linked deployment** — the Rust-native flavor: workflow core and selected adapter libraries compiled into one process and connected through Rust capability traits.
- **Linked host (`linked`)** — the reusable library surface of the linked package: catalog, provider, erased model handle, model bridge, reference host, and asynchronous command execution.
- **Adapter identity** — the immutable package identity a catalog entry provides: optional package namespace, adapter name, and exact version. Identity is distinct from resolve-time adapter metadata.
- **Adapter selector** — the operator-supplied shape retained through provisioning and resolution: bare name, exact package reference, or local component path.
- **Erased model (`DynModel`)** — the linked host's one concrete model type: a reference-counted object-safe wrapper that any `Model` backend converts into.
- **Eval library (`eval`)** — lab-only trial, scenario, grading, telemetry, sandbox, and eval CLI logic.
- **Lab (`lab`)** — an unpublished repository composition binary that dispatches between linked command passthrough and eval. Specify's lab binds fixture adapters; the adapter repository's lab reuses the first-party composition catalog.
- **First-party linked composition (`composition`)** — the unpublished internal source package at `augentic/specify-adapters/crates/composition`: a library plus executable that owns the first-party catalog and invokes the linked command path. Its final distribution identity is follow-up work.
- **Fixture adapters** — deterministic local SDK-native adapters in `crates/fixture`, not mocks of the workflow seam.

## Decision

### Mental model

```text
                         workflow core + transport
                                    ▲
                                    │ capability contracts
                   ┌────────────────┴────────────────┐
                   │                                 │
          Wasm deployment                    linked deployment
          WIT-backed Provider                linked::Provider
          Component resolver                 Catalog resolver
          model host import                  DynModel value
          adapter HTTP refs                  ReferenceHost
                   │                                 │
                   │                                 ├── workflow tests
                   │                                 ├── repository labs
                   │                                 └── first-party composition
                   │
          component boundary gates

eval ──► linked
lab ──► eval + linked + one repository-provided catalog
```

Dependency direction in the Specify workspace:

```text
lab ──► fixture
lab ──► linked
lab ──► eval ──► linked
tests ──► linked (+ fixture as needed)

linked ──✗──► eval
linked ──✗──► lab
linked ──✗──► fixture
linked ──✗──► specify-adapters

workflow-core production dependencies ──✗──► linked / eval / fixture / lab
workflow-core dev-dependencies ────────────► linked + fixture where integration suites need them
```

### Crate names and ownership

| Target crate | Kind | Responsibility | Source of current code |
| --- | --- | --- | --- |
| `linked` | library | Linked deployment host: catalog, provider, erased model, references, command execution, optional Cursor backend | Native execution modules from `harness` |
| `eval` | library (`publish = false`) | Lab-only workflow trial, adapter scenarios, grading, telemetry, sandbox, eval CLI | Eval modules from `harness` |
| `lab` | binary (`publish = false`) | In-repo composition and argv dispatch | Current `eval` binary, renamed |
| `fixture` | library (`publish = false`) | Deterministic adapters, answer corpus, recording model, sessions | Existing `fixture`, retargeted to `linked` |

After migration there is no `harness` package. The current `eval` binary package moves to `lab`, and a new library package takes the freed `eval` name. This reuse is deliberate: `eval` is the established project term and `cargo make eval` remains the task surface.

No crate is named `native`: that word already means `cfg(not(target_arch = "wasm32"))` and native tests, while the Wasm deployment's host is itself a native process. `linked` names the deployment's distinguishing composition property.

### Adapter composition

Wasm `specify` ships the engine and composes adapter components dynamically. The operator binary has no compile-time dependency on `specify-adapters`.

Linked composition is static:

1. Specify owns `crates/linked` and the linked command API.
2. Specify ships no first-party-bound linked executable because that would introduce a reverse dependency on `specify-adapters`.
3. `augentic/specify-adapters` owns the unpublished internal `crates/composition` package with:
   - a library target exporting `catalog()` and linked build inputs;
   - an executable target that supplies that catalog and a Cursor model to `linked::command::run`;
   - no dependency on `eval` or `lab`.
4. The adapter repository's `lab` depends on `composition`'s library target and `eval`, so the executable and lab cannot drift onto different first-party catalogs.
5. Specify's own `lab` uses `fixture::catalog()`.

```text
augentic/specify                         augentic/specify-adapters
────────────────                         ─────────────────────────
linked host library ───────────────────► composition package
eval library ──────────────────────────►   lib: catalog()
                                         executable: linked command
fixture catalog ─► Specify lab            ▲
                                           │ shared catalog
                                         adapters lab + eval
```

The final distribution package name, executable installation identity, release artifact naming, and relationship to the existing `specify` release channel are not decided here. The internal `composition` package is `publish = false`; this RFC requires only that its executable be product-shaped, that it never depend on eval code, and that no documentation present the lab as its substitute.

### Dependency enforcement

The Specify `checks` package enforces dependency direction by parsing Cargo manifests:

- `error`, `diagnostics`, `artifacts`, `adapter`, `project`, `slice`, `change`, and `transport` reject `linked`, `eval`, `fixture`, `lab`, and the removed `harness` in production and build dependencies;
- explicit dev-dependencies on `linked` and `fixture` remain legal for workflow integration suites;
- `linked` rejects `fixture`, `eval`, `lab`, and concrete adapter crates in production dependencies;
- `eval` depends on `linked` plus deployment-neutral artifact/project types needed for grading, and rejects `fixture`, `lab`, `change`, Cursor integration, and concrete adapter crates.

The adapter repository adds its own checks:

- `composition` has no dependency on `eval` or `lab`;
- the lab consumes `composition::catalog()` rather than declaring another first-party catalog;
- the default catalog contains every first-party adapter exactly once on its declared axis; any subset feature declares and tests its reduced expected inventory.

Extend the existing `checks` manifest coverage with these rules, then retire `crates/harness/tests/boundary.rs`; the invariant moves rather than disappearing.

## Deployment-neutral seams

### Workflow core

Workflow semantics remain unchanged:

- handlers implement `omnia_guest::api::operation::Operation<P>`;
- each operation states its minimum capability intersection on `P`;
- orchestrators receive provider-carried capabilities;
- `transport` assembles typed command and HTTP routers;
- the adapter SDK owns `adapter::Source` and `adapter::Target`.

The workflow core does not know whether model, source-adapter, target-adapter, provisioning, or resolution capabilities are satisfied by WIT imports, linked Rust implementations, scripted doubles, or a live Cursor backend.

Three deployment assumptions move into explicit contracts: adapter identity/selection, provisioning, and execution paths.

### Adapter identity

The component deployment obtains identity from the component package and its resolved artifact location. The linked deployment has no component artifact, so each SDK implementor must expose the equivalent compile-time identity.

Add an SDK identity value:

```rust
pub struct AdapterIdentity {
    pub namespace: Option<&'static str>,
    pub name: &'static str,
    pub version: &'static str,
}
```

`adapter::Source` and `adapter::Target` expose `const IDENTITY: AdapterIdentity` in place of a name-only constant. Their generic associated operation methods remain unchanged and non-object-safe.

- Published adapters set `namespace` and exact package version, normally from `env!("CARGO_PKG_VERSION")`.
- Unpublished fixture/probe adapters may omit `namespace`; they remain bare-only identities.
- Resolve-time `SourceMetadata` / `TargetMetadata` remain non-identity metadata. Version does not move into the WIT `metadata` answer.
- Adapter crate tests assert that published identity name/version agree with the crate package and component publication configuration.

The linked catalog uses identity for exact-pin matching, resolve output, reference-server version reporting, inventory diagnostics, and duplicate validation.

Specify's workflow identity remains globally unique `(name, version)`, matching existing `SourceBinding`, `TargetRef`, and adapter-id wires. Package namespace is retained on the selector, linked catalog entry, project target binding, and component store metadata so provisioning can verify provenance. Component provisioning refuses to reuse an existing `(name, version)` store entry recorded from another namespace. A legacy store sidecar without namespace fails as provenance-unknown and must be reinstalled; the resolver does not guess. Supporting same-named packages from multiple namespaces would require a separate artifact/store wire change and is outside this RFC.

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

Parsing moves from `project::init::adapter_uri` into `project::adapter`, because source resolution, target resolution, initialization, and both deployment flavors share the grammar.

The selector preserves:

- a bare development request;
- the namespace and exact version of a package request;
- the fact that a local path is a component artifact, even when its filename matches a linked adapter.

Parsing is syntactic. Local-file existence, canonicalization, and component validation remain provisioning concerns, so a persisted local selector can still resolve through its project cache after the operator's original input file is removed. GitHub source URLs remain unsupported and fail during selector parsing rather than falling through as component paths.

`AdapterRef` is deleted rather than retained as a second lossy view. Persisted target values parse directly to `AdapterSelector`; successful provisioning/resolution returns `ResolvedSource` or `ResolvedTarget` with the globally unique `(name, version)` workflow identity plus opaque origin/provenance.

### Provisioning

The low-level `Hydrator::fetch(url)` capability is replaced by a deployment-level `Provisioner`. Initialization and plan authoring pass typed selectors to the active deployment before writing adapter-specific cache state or canonical bindings.

Conceptually:

```rust
pub struct ProvisionContext<'a> {
    pub paths: &'a ExecutionPaths,
    pub now: jiff::Timestamp,
    pub mode: ProvisionMode,
}

pub enum ProvisionMode {
    Init,
    Upgrade,
    Plan,
}

pub trait Provisioner: Send + Sync {
    fn provision_source(
        &self,
        selector: &AdapterSelector,
        context: &ProvisionContext<'_>,
    ) -> impl Future<Output = Result<ProvisionedSource, Error>> + Send;

    fn provision_target(
        &self,
        selector: &AdapterSelector,
        context: &ProvisionContext<'_>,
    ) -> impl Future<Output = Result<ProvisionedTarget, Error>> + Send;
}
```

`ProvisionedSource` / `ProvisionedTarget` carry:

- the canonical binding fields/value to persist in `plan.yaml` or `project.yaml`;
- the resolved identity, metadata, and opaque origin;
- any hydration/install fact needed by the init report.

The component provisioner owns the existing package fetch, store write, digest sidecar, verify-after-write, local component mirror, and development-probe policy. A local selector is validated and mirrored before the canonical binding is returned; later component resolution deliberately derives the cached adapter name from that still-typed selector. The existing deterministic fetch/store kernels remain in `project`; only the unconditional byte-oriented `Hydrator::fetch` entry path is removed. `ProvisionContext` preserves injected time and fresh-init/upgrade behavior.

The linked provisioner performs no component I/O:

- bare selector: match by name; persist the exact package reference when the entry has a published namespace, otherwise preserve the bare name;
- exact package selector: succeed only when namespace, name, and version equal the compiled catalog entry;
- version mismatch or absent entry: `adapter-not-linked`, including requested and available identities;
- component selector: `adapter-not-linked`, explicitly stating that linked execution does not load the supplied component.

Resolution methods also receive `AdapterSelector`, so a persisted local component URI can never be silently narrowed to a bare linked name.

Linked init/plan provisioning canonicalizes a bare published adapter to its exact package identity using the artifact's existing fields. Upgrade preserves the project's recorded target binding and only re-resolves it; it does not silently rewrite legacy bare values while updating the Specify pin.

### Adapter reference semantics

Linked resolution is a static package match, not a component-store lookup:

- **Bare references** resolve against the active catalog and report the entry's actual version, not `0.0.0`.
- **Exact package references** resolve when the catalog contains that exact identity.
- **Mismatched pins** fail as `adapter-not-linked`, naming the linked version and pointing to a compatible linked build or the Wasm deployment.
- **Local components** fail as unsupported by the linked deployment before component cache writes occur.
- **Hydration and digests** do not apply to linked entries.
- **Version floors** remain enforced at runtime. Rust compilation proves trait compatibility, not semantic compatibility with the adapter's declared `specify_floor`.

A project carrying an exact first-party adapter pin can therefore move between deployment flavors once the active deployment has provisioned that identity (`specify init --upgrade` installs a missing component; linked provisioning matches the catalog). The component deployment verifies component bytes; the linked deployment asserts the package identity compiled into its catalog. Release attestation for the linked binary is part of the future product-identity work.

### Source binding resolution

Today `SourceBinding.version` is persisted but survey/extract dispatch ignores it and never calls `Resolver`. This RFC closes that gap without changing the `plan.yaml` source-binding schema:

- plan-author source parsing accepts the first-party shorthand `<name>@<semver>` (implicit `specify` namespace) and materializes the existing `adapter` plus `version` fields;
- plan author provisions and resolves every source binding before survey; an exact component source can therefore be installed, a bare linked published source is rewritten with the catalog's actual version, and a component development source remains unpinned;
- `source survey` and `source extract` resolve the binding again before dispatch, enforcing exact version and `specify_floor`;
- dispatch uses the resolved source name only after resolution succeeds;
- the survey/extract capability bounds therefore include `Resolver`.

Package namespace is not added to `SourceBinding`: adapter names are globally unique, and the existing source wire treats an exact `(name, version)` as the workflow identity. Registry namespace remains provisioning provenance rather than another source key. A future multi-namespace identity model would require an artifact-schema revision.

### Execution paths and cache isolation

The current eval helper mutates `SPECIFY_PROJECT_CACHE` under a scoped unsafe guard. That is not a sound configuration mechanism for a reusable multithreaded host.

The provider-carried path value gains an optional explicit cache parent alongside the project root:

```rust
pub struct ExecutionPaths {
    project_root: PathBuf,
    cache_parent: Option<PathBuf>,
}
```

The type lives in `project::handler` beside `Anchor`, exposes read-only accessors, and is re-exported by `linked`. Cache resolution receives it rather than mutating process environment:

- linked product execution passes a canonical project root and inherits the operator's configured/default cache parent;
- eval and fixture sessions pass a sandbox-local cache parent;
- the Wasm provider continues to resolve the guest cache mount;
- `SPECIFY_PROJECT_CACHE` remains a process-start configuration input, not a variable changed while tasks are running.

`ExecutionPaths::operator(root)` captures the process-start cache configuration; `ExecutionPaths::isolated(root, cache_parent)` supplies an explicit parent. The provider's anchoring capability exposes the value, and internal `Layout`/cache helpers carry the override from operation context. This is deployment configuration, not a test-only public API widening.

## The `linked` package

`linked` is the reusable linked-deployment host used by workflow tests, repository labs, downstream compositions, and the first-party composition executable.

It owns:

- validated `Catalog` and typed source/target registration;
- `DynModel`;
- `Provider`, implementing project anchoring, provisioning, resolution, model, workflow source, and workflow target capabilities;
- adapter-SDK to workflow-seam conversion;
- asynchronous command-router execution over caller-supplied values;
- the guest-model to host-model bridge;
- the optional Cursor-backed model implementation;
- linked reference routing and listener lifecycle.

It does not own:

- fixture adapters or scripted answers;
- trials, scenarios, grading, telemetry, or sandbox orchestration;
- lab argv parsing or `--project-dir`;
- Tokio runtime creation;
- process-global cache mutation;
- a concrete adapter catalog;
- a hard-coded model choice.

### Erased model (`DynModel`)

`omnia_guest::Model` is not object-safe because `create` returns `impl Future`. `DynModel` uses one private object-safe trait returning a boxed future behind `Arc`.

Every adapter operation in a catalog is monomorphized once at `DynModel`. The SDK's generic source-adapter and target-adapter operation traits remain unchanged apart from their identity constant.

The cost is one vtable hop per model completion. The benefits are:

- `Catalog` and `Provider` carry no model type parameter;
- `Binding` and `adapters!` disappear;
- command and eval entrypoints consume values;
- conditional catalog activation is ordinary Rust;
- model middleware remains composable before erasure.

`DynModel::new` accepts `impl Model + Send + Sync + 'static`. Composition roots erase exactly once, then `Provider::offline(paths, model, catalog)`, `Provider::online(paths, model, catalog)`, and command APIs accept `DynModel` directly. The two provider constructors differ only in reference-host policy.

Backends with post-run state expose it through caller-held clones:

```rust
let recording = RecordingModel::answering(answers);
let paths = ExecutionPaths::isolated(root, cache_parent);
let model = DynModel::new(recording.clone());
let provider = Provider::offline(paths, model, fixture::catalog()?);

// Run operations...
recording.assert_exhausted();
```

`Provider` exposes no model accessor.

If `Model` later gains a signature that cannot be boxed, only the private erasure module and catalog function-pointer aliases need reconsideration; adapter implementations remain generic.

### Catalog

The catalog is the linked deployment declaration. Registration captures operation function pointers at `DynModel`:

```rust
fn catalog() -> Result<linked::Catalog, linked::Error> {
    linked::Catalog::builder()
        .source::<intent::Adapter>()
        .target::<vectis::Adapter>()
        .build()
}
```

`build()` validates:

- duplicate entries on an axis;
- cross-axis name collisions;
- malformed names, namespaces, and versions;
- published identities without exact SemVer;
- conflicting MCP shelf identities.

The current fixture's default type is registered as `fixture` on both axes. Migration gives its source implementation the identity `fixture-source` while the target remains `fixture`, and updates fixture plans/guests accordingly; production validation does not gain a test-only collision exception.

The active catalog may be assembled conditionally. Runtime conditions activate a subset of adapters already linked into the binary; producing a smaller binary still requires Cargo feature/dependency gating at the composition package.

The catalog exposes a read-only inventory for diagnostics and build information but does not expose operation function pointers publicly.

### Command execution

Libraries do not construct or block Tokio runtimes. `linked::command` exposes asynchronous APIs:

```rust
pub async fn execute(
    paths: ExecutionPaths,
    model: DynModel,
    catalog: Catalog,
    info: CommandInfo,
    argv: Vec<String>,
) -> Result<CommandResponse, Error>;

pub async fn run(/* same values */) -> std::process::ExitCode;
```

`execute` builds an online provider, runs the shared router, and awaits reference-host shutdown before returning the typed transport response. `run` writes that response, renders linked setup/router/shutdown failures, and returns the resulting exit code. Tests that need an offline provider assemble it directly.

The composition root owns:

- runtime construction;
- `std::env::args`;
- construction of `ExecutionPaths` from a canonical project root;
- Cursor configuration;
- catalog construction;
- rendering errors that occur before `linked::command::run` is entered.

Product-shaped entry:

```rust
#[tokio::main]
async fn main() -> std::process::ExitCode {
    render(entry().await)
}

async fn entry() -> Result<std::process::ExitCode, Box<dyn std::error::Error>> {
    let root = std::env::current_dir()?.canonicalize()?;
    let paths = linked::ExecutionPaths::operator(root);
    let catalog = composition::catalog()?;
    let model = linked::DynModel::new(linked::CursorModel::new(
        paths.project_root(),
        linked::CursorOptions::from_env(),
    ));
    Ok(linked::command::run(
        paths,
        model,
        catalog,
        linked::CommandInfo::linked(),
        std::env::args().collect(),
    )
    .await)
}
```

`--project-dir` remains a lab convenience implemented by the lab binary before canonicalization. It is not exported by `linked` or documented as product command surface.

### Command identity and shared help

`transport::command::router` receives `CommandInfo` rather than hard-coding version text:

```rust
pub struct CommandInfo {
    pub flavor: DeploymentFlavor,
}

impl CommandInfo {
    pub const fn wasm() -> Self;
    pub const fn linked() -> Self;
}
```

The Wasm and linked composition roots supply `Wasm` and `Linked` respectively. `transport` derives the engine version from the same workspace package version used by the runtime floor check, so callers cannot inject a contradictory version. The application/invoker name remains the shared constant `specify`; `CommandInfo` selects flavor-specific support text, not a second command identity. `linked` re-exports the value for downstream composition roots.

The initial support format is `specify <engine-version> (<flavor>)`, for example `specify 0.27.2 (linked)`. Bundle version, source revision, package name, and release artifact identity remain extensible future build information.

Shared command help is deployment-neutral:

- no command is described as “guest-only” when both providers route it;
- source/target resolution refers to the active deployment rather than a `.wasm` path;
- initialization states that supported selector forms depend on the active deployment;
- `guest.lock` wording describes the workflow command lock rather than a Wasm-only property.

Model and reference resources remain lazy enough that `--help`, `--version`, and completions do not connect Cursor or bind a listener.

### Features

- **always-on core** — catalog, identity validation, `DynModel`, conversion, offline `Provider`.
- **cli** — command execution and linked reference hosting (`transport`, axum, Tokio networking).
- **cursor** — `CursorModel`, `CursorOptions`, and the private model bridge.

There is no `live` union feature. Composition packages enable the explicit features they use.

### Model bridge and Cursor configuration

Workflow and adapter libraries consume `omnia_guest::Model`, while `omnia_cursor::Client` implements host-side `omnia_wasi_model::WasiModelCtx`. The bridge:

- maps guest requests to the host wire request;
- runs the host request gate;
- exposes the canonical project root when workspace lending is requested;
- invokes the backend;
- validates and projects replies;
- preserves typed model errors.

Rename `DevModel` to `CursorModel` and `Native<B>` to private `ModelBridge<B>`.

`CursorModel` accepts a `CursorOptions` value. `SPECIFY_MODEL` is read only by explicit composition-root configuration (`CursorOptions::from_env`); the model implementation and scenario reporter do not read it independently. `CursorOptions` exposes its configured model id so the lab's model factory can populate `ModelInstance.default_model`. A request-supplied model id continues to win over the driver default.

The bridge remains private to `linked` and gains parity tests for every request/reply field it mirrors from Omnia. Moving it upstream remains desirable follow-up work.

### Reference hosting

Adapter judgment requests carry MCP grants for embedded adapter references. Linked execution serves those documents on loopback.

`Provider` carries a shared `ReferenceHost`:

- offline mode never starts a listener and is explicit in deterministic tests;
- online mode starts lazily on the first adapter operation that has non-empty reference documents;
- a catalog with no reference documents remains a no-op;
- bind failure fails the operation rather than stripping grants;
- all provider clones share one listener;
- command execution requests graceful shutdown and awaits the server task on every exit path;
- embedders can call the same asynchronous shutdown method, while `Drop` retains an abort fallback;
- the listener binds only to `127.0.0.1`.

Reference shelf identity uses the adapter catalog entry's name and version, not the `linked` crate version.

The source/target seam mirrors the closed WIT error shape, so a lazy listener failure crosses it as `project::seam::Error::Internal` with the stable detail prefix `reference-listener-unavailable`; transport keeps its existing `seam-dispatch-failed` outer code. Direct `ReferenceHost` startup retains a typed linked error. This RFC does not widen the WIT error variant solely for host diagnostics.

The reference module exposes router construction separately from ephemeral listener ownership so a later linked HTTP host can mount the same routes without a second implementation.

### Concurrency posture

The initial command path is single-flight:

- one command runs to completion per provider graph, and the supplied composition binaries run one command per process;
- `Provider: Clone` supports router invocation and shared capabilities, not concurrent independent commands;
- embedders needing concurrency create independent providers and cache/reference contexts;
- cancellation and long-running serve mode are future work.

This posture is explicit rather than inferred from process-global environment or detached tasks.

## The `eval` package

`eval` is a lab-only library over `linked` with its `cli` feature. It owns:

- the multi-step live workflow eval;
- single-operation adapter scenarios;
- deterministic grading;
- model-request telemetry;
- sandbox seeding and cleanup;
- eval CLI parsing.

It does not:

- construct a concrete adapter catalog;
- depend on fixture or first-party adapter crates;
- construct a Cursor backend;
- create or block a Tokio runtime;
- mutate process-global cache environment;
- depend directly on `change` merely to bypass the command surface.

Its process-facing client supplies a catalog and a model factory:

```rust
pub struct ModelInstance {
    pub model: linked::DynModel,
    pub default_model: Option<String>,
}

pub type ModelFactory = Arc<
    dyn Fn(&Path) -> Result<ModelInstance, Error> + Send + Sync,
>;

pub async fn run(
    workspace_root: PathBuf,
    catalog: linked::Catalog,
    model: ModelFactory,
    args: &[String],
    scenarios: Option<&Path>,
) -> Result<std::process::ExitCode, Error>;
```

`workspace_root` anchors the persistent `sandbox/` tree and relative scenario roots; eval does not consult process current-directory state after entry. The library wraps `ModelInstance.model` in `Telemetry`, gives telemetry the configured default for effective-model reporting, keeps its own clone for read-back, and drives workflow phases through `linked::command::execute`. It uses public project/artifact types only where deterministic grading needs structured state.

Scenario reports derive the effective model from each observed request (`request.model` when present, otherwise `ModelInstance.default_model`), not by rereading `SPECIFY_MODEL`.

Eval scratch state receives an explicit sandbox-local cache parent. The persistent full-trial sandbox is single-writer and guarded against a second concurrent eval in the same checkout; per-scenario run directories remain unique.

## Lab binaries

Each repository owns an unpublished `lab` binary. It creates one Tokio runtime through its async `main`, parses its own lab-only arguments, and dispatches visibly:

```rust
#[tokio::main]
async fn main() -> std::process::ExitCode {
    render(entry().await)
}

async fn entry() -> Result<std::process::ExitCode, Box<dyn std::error::Error>> {
    let mut argv: Vec<String> = std::env::args().collect();
    let root = lab_project_root(&mut argv)?;
    let catalog = fixture::catalog()?;
    let options = linked::CursorOptions::from_env();

    if argv.get(1).is_some_and(|arg| arg == "eval") {
        let factory = cursor_factory(options);
        return Ok(eval::run(root, catalog, factory, &argv[1..], None).await?);
    }

    let paths = linked::ExecutionPaths::operator(root);
    let model =
        linked::DynModel::new(linked::CursorModel::new(paths.project_root(), options));
    Ok(linked::command::run(
        paths,
        model,
        catalog,
        linked::CommandInfo::linked(),
        argv,
    )
    .await)
}
```

The example is schematic about the common `render` helper; the real binary preserves one stable startup-error rendering path.

- ordinary arguments run a Specify command through `linked`;
- `eval` runs the shared eval client;
- `--project-dir` is parsed only here;
- when placed before `eval`, `--project-dir` intentionally anchors that eval's `sandbox/` instead of relying on process current directory;
- the binary is the target of `cargo make dev` and `cargo make eval`;
- it is never an install or release artifact.

The two repositories intentionally duplicate the small dispatch. They do not duplicate the first-party catalog: the adapter lab calls `composition::catalog()`.

Integration tests assemble an offline `linked::Provider` directly and never require a lab binary.

## Adapter-repository composition

`augentic/specify-adapters` consumes Specify's `adapter`, `linked`, and lab-only `eval` crates one-way.

| Target | Role | Entry | Catalog |
| --- | --- | --- | --- |
| `composition` library | Static first-party deployment declaration | `catalog()` | owns the first-party catalog |
| `composition` executable | Product-shaped linked command | async `linked::command::run` | calls its library |
| `lab` | Dev command + eval | inline linked/eval dispatch | reuses `composition` library |

The unpublished `composition` package validates the complete source-adapter and target-adapter inventory by default. A subset Cargo feature must declare its expected reduced inventory; runtime activation may select within the adapters that feature compiled.

Wasm composed tests and the change example remain separate gates.

## Wasm and linked correspondence

| Concern | Wasm deployment | Linked deployment |
| --- | --- | --- |
| Specify product | Existing released `specify` distribution | First-party/downstream linked composition; distribution identity follow-up |
| Workflow composition | `src/lib.rs` + WIT-backed provider | `linked::Provider` + async command assembly |
| Adapter declaration | component deployment configuration | validated `Catalog` value |
| Adapter identity | package/store identity | compile-time SDK identity |
| Adapter selection | component resolver | catalog resolver |
| Provisioning | registry/store/cache component policy | exact catalog match; no component I/O |
| Engine invocation | `Invoker` + shared transport router | same |
| Model access | `omnia:model/completion` host import | composition-root model value |
| Adapter dispatch | WIT source/target imports | function-pointer table at `DynModel` |
| References | adapter HTTP guest routed by Omnia | owned loopback `ReferenceHost` |
| Execution paths | shared Wasm project/cache preopens | canonical project root + explicit/inherited cache parent |
| Isolation | component instance per call | trusted code in one native process |
| Exact adapter pin | store entry + digest verification | succeeds only on exact compiled identity |
| Local `.wasm` selector | supported by component provisioning | rejected before cache mutation |
| Lab composition | none | unpublished `lab` + `eval` |

Observable behavior shared across flavors:

- command grammar, output shapes, and exit codes;
- workflow artifacts and lifecycle transitions;
- adapter operation order;
- model request/answer schemas;
- MCP reference contents;
- report and validation gates.

Behavior intentionally specific to a flavor:

- component ABI and WIT mapping;
- component instance isolation;
- dynamic component provisioning and hydration;
- global adapter-store lookup and digest verification;
- static catalog inventory and linked trust boundary;
- exact linked package set.

## Testing model

### Workflow integration tests

```text
offline linked::Provider
  ├── fixture catalog
  ├── temporary project root + explicit cache parent
  └── RecordingModel<omnia_testkit::model::Scripted>
      └── caller-held clone for requests / exhaustion
```

CLI-reachable workflow behavior goes through the transport router. Linked host dispatch tests and single-adapter eval scenarios call the public provider capability traits directly. No second public `linked::invoke` wrapper is introduced.

### Linked host tests

`crates/linked/tests/` uses crate-local probe implementors and covers:

- `DynModel` forwarding and shared state;
- catalog identity validation and duplicate rejection;
- source/target function-pointer dispatch;
- bare and exact-pin resolution;
- mismatched-pin and component-selector refusal;
- runtime `specify_floor` enforcement;
- command flavor/version projection and deployment-neutral help;
- reference no-op, bind failure, grant routing, and shutdown.

No linked production dependency on fixture is introduced.

The private generic `ModelBridge<B>` keeps dense module-level parity tests for every request/reply field. This is the narrow private-kernel exception to the integration-first posture; its type is not made public solely to relocate those tests.

### Eval tests

`crates/eval/tests/` uses injected scripted model factories and covers:

- argument handling;
- scenario loading;
- deterministic grading;
- telemetry;
- sandbox locking and unique run directories;
- command-response handling.

### Fixture tests

Fixture behavior, answer recording, and catalog inventory remain under `crates/fixture/tests/`. `Session` stores its `RecordingModel` beside the provider and uses an explicit temporary cache parent.

Rename `fixture::model::Harness<B>` to `fixture::RecordingModel<B>` and the sibling repository's copy likewise.

### Wasm boundary tests

No linked test claims Wasm coverage. Existing component gates remain:

- adapter crate tests;
- adapters `composed` tests;
- the Specify fixture change example;
- the first-party change example over the published core component.

## Module migration

| Current module | Target | Notes |
| --- | --- | --- |
| `catalog.rs` | `crates/linked/src/catalog.rs` | identity-aware builder at `DynModel`; `Binding` and `adapters!` deleted |
| — | `crates/linked/src/model.rs` | `DynModel` and private erased-model trait |
| `convert.rs` | `crates/linked/src/convert.rs` | private SDK/workflow DTO mapping |
| `provider.rs` | `crates/linked/src/provider.rs` | non-generic provider; offline/online reference modes; no model accessor |
| `command.rs` | `crates/linked/src/command.rs` | async `execute` / `run`; no runtime or lab argv parsing |
| `mcp.rs` | `crates/linked/src/references.rs` | router plus owned lazy `ReferenceHost` |
| `model.rs` | `crates/linked/src/cursor.rs` | `CursorModel`, `CursorOptions`, `SPECIFY_MODEL` |
| `native.rs` | `crates/linked/src/model_bridge.rs` | private `ModelBridge<B>` with parity tests |
| `invoke.rs` | — | public duplication removed; call Omnia `Invoker` or command API directly |
| `env.rs` | — | process mutation removed; cache parent enters execution context explicitly |
| `entry.rs` | — | runtime and dispatch move to composition binaries |
| `fs.rs` | `crates/eval/src/fs.rs` | eval tree copy |
| `grade.rs` | `crates/eval/src/grade.rs` | deterministic grading |
| `sandbox.rs` | `crates/eval/src/sandbox.rs` | sandbox and single-writer guard |
| `scenario.rs` | `crates/eval/src/scenario.rs` | prompt scenarios over supplied catalog/model factory |
| `telemetry.rs` | `crates/eval/src/telemetry.rs` | caller-held model request counts |
| `trial.rs` | `crates/eval/src/run.rs` | full eval workflow |

Move the current `crates/eval` binary to `crates/lab`, then create the new `crates/eval` library. User-facing strings naming the native harness or shim become linked-deployment language.

## Cargo and feature layout

### Specify workspace

```toml
linked = { path = "crates/linked" }
eval = { path = "crates/eval" }
```

Remove the `harness` workspace dependency. `lab` needs no workspace-dependency entry.

`linked`:

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
cursor = [
    "dep:omnia",
    "dep:omnia-cursor",
    "dep:omnia-wasi-model",
    "dep:tokio",
    "tokio/sync",
]
```

The actual default dependency list still includes the workspace contracts required by catalog/provider code; `default = []` means no optional command/Cursor stack.

`eval` enables `linked/cli` and accepts model factories. It does not force `linked/cursor`.

`lab` enables both:

```toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
eval.workspace = true
fixture.workspace = true
linked = { workspace = true, features = ["cli", "cursor"] }
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
```

Both `eval` and `lab` are `publish = false` and never attached to a release.

### Adapter workspace

Replace the harness dependency with:

```toml
adapter = { git = "https://github.com/augentic/specify.git" }
linked = { git = "https://github.com/augentic/specify.git" }
eval = { git = "https://github.com/augentic/specify.git" }
```

These are root `[workspace.dependencies]` declarations, not dependencies of every member:

| Adapter-repository member | Specify dependencies |
| --- | --- |
| source/target adapter crates | `adapter` |
| `composition` library/executable | `linked` with `cli,cursor` plus concrete first-party adapters |
| `lab` | `composition`, `eval`, and `linked/cursor` |

- Rename current `crates/eval` to `crates/lab`, moving scenarios with it.
- Add unpublished `crates/composition` with a catalog library and linked executable.
- Make the lab depend on `composition` rather than concrete adapters directly for catalog declaration.
- Update sibling path patches.

The internal source package is `composition`; its final distribution package name, executable installation name, archive naming, and release attachment are deferred.

## Command surface

Operator-facing workflow verbs and wire outputs do not change.

Shared help becomes deployment-neutral, and `--version` adds the deployment flavor.

Development tasks retain their names but target the lab:

- `cargo make dev -- ARGS` — lab command passthrough;
- `cargo make eval` — lab eval command;
- `cargo make eval scenario <id>` — one prompt scenario;
- `cargo make change-run` — Wasm composed example.

Package selection for lab tasks is `cargo run -p lab`. No installation or release documentation points operators at that package.

## Implementation plan

Stages are dependency order, not permission to merge a broken workspace. While Stage 2 leaves eval modules in `harness`, that package temporarily depends on `linked` and retains the existing `--project-dir` forwarding shim. Stage 3 moves the current `eval` binary to `lab`, creates the new `eval` library, switches task targets, and only then removes the shim and `harness`; Stages 2 and 3 may land atomically when that is simpler.

### Stage 1 — Deployment-neutral identity and provisioning

1. Add SDK `AdapterIdentity`; update source-adapter and target-adapter implementors in both repositories.
2. Delete the lossy `AdapterRef`; parse persisted/raw values directly into typed `AdapterSelector` in `project::adapter`.
3. Replace low-level `Hydrator` with deployment-level `Provisioner` plus `ProvisionContext`; keep current deterministic component fetch/store/cache kernels behind the component implementation.
4. Make resolver methods preserve selector kind and return actual resolved identity.
5. Route plan-author source bindings through `Provisioner` + `Resolver`, and survey/extract through `Resolver`, preserving existing source wire fields while enforcing pins and floors.
6. Record package namespace in component store metadata and refuse namespace provenance collisions for a globally unique adapter name.
7. Add explicit cache-parent plumbing from provider execution context to cache resolution; remove scoped runtime environment mutation.
8. Add `CommandInfo` / `DeploymentFlavor`; make shared command help deployment-neutral.
9. Preserve existing Wasm initialization, hydration, store, digest, and command behavior through component-provider integration tests.

### Stage 2 — Extract the linked host

1. Add `crates/linked` and move catalog, conversion, provider, command, reference, Cursor model, and model bridge code from `harness`.
2. Introduce `DynModel`; remove the model type parameter from `Catalog` and `Provider`.
3. Delete `Binding` and `adapters!`; make fixture catalogs plain validated values.
4. Implement linked provisioning and resolution over exact catalog identities.
5. Make command execution asynchronous and value-consuming; composition roots own Tokio.
6. Add owned lazy reference hosting with explicit failure projection and awaited shutdown.
7. Rename `DevModel` to `CursorModel`, `Native<B>` to `ModelBridge<B>`, and `SPECIFY_EVAL_MODEL` to `SPECIFY_MODEL`.
8. Retarget `fixture` and workflow suites from `harness` to `linked`.
9. Rename `fixture::model::Harness` to `fixture::RecordingModel` and give the default fixture source/target distinct identities.
10. Move linked host tests and replace `Provider::model()` call sites with caller-held recording/telemetry handles.

### Stage 3 — Eval and lab

1. Move the current `crates/eval` binary to `crates/lab`, then create a new `crates/eval` library from trial, scenario, grading, telemetry, sandbox, and tree-copy modules.
2. Inject catalog and model factory values; remove concrete Cursor and adapter construction.
3. Route workflow phases through the linked command API and keep only grading-required project/artifact dependencies.
4. Inline async dispatch and lab-only project-root parsing in `crates/lab`.
5. Remove the emptied `crates/harness`.
6. Point `cargo make dev` and `cargo make eval` at `-p lab`.

### Stage 4 — Adapter repository

1. Pin a Specify revision exposing `adapter`, `linked`, and `eval`.
2. Update every first-party adapter identity and native test support.
3. Add unpublished `crates/composition` with a library and executable over `linked`.
4. Rename adapter `eval` to `lab`, flip it to `publish = false`, reuse `composition::catalog()`, and inject the Cursor model factory.
5. Rename `testkit::Harness` to `RecordingModel`.
6. Add adapter-repository dependency and catalog completeness checks.
7. Confirm no reverse dependency from Specify to `specify-adapters`.

### Stage 5 — Documentation and checks

Update both repositories' `AGENTS.md`, adapter `TESTING.md`, Specify testing/architecture/workflow standards, quality-gate docs, Makefiles, CLI help, and rustdoc.

Document:

- one Specify product with Wasm and linked deployment flavors;
- linked exact-pin matching and component-selector refusal;
- linked adapters as trusted in-process code;
- lab/eval as unpublished tooling;
- linked tests as non-WIT/non-store coverage;
- the linked distribution's package/install/release identity as unresolved follow-up.

Extend Specify and adapters checks with the dependency rules above.

### Stage 6 — Verification

In `augentic/specify`:

```bash
cargo make check
cargo make ci
cargo make dev -- --help
cargo make dev -- --version
cargo make eval
cargo check --lib -p specify --example change --target wasm32-wasip2
cargo make change-run
```

In `augentic/specify-adapters`:

```bash
cargo make check
cargo make ci
cargo make dev -- --help
cargo make dev -- --version
cargo make eval
cargo make change-run
```

Live-model and composed change-run commands remain operator-invoked when credentials are unavailable.

## Acceptance criteria

1. Documentation presents Specify as one product with Wasm and linked deployment flavors, without claiming linked component/WIT/store properties.
2. `adapter::Source` and `adapter::Target` expose validated static identity while retaining generic associated operation methods.
3. `AdapterSelector` preserves bare, package, and component input kinds through provisioning and resolution; the lossy `AdapterRef` no longer exists.
4. Package namespace provenance is recorded in component store metadata, and a same-name/version entry from another namespace is refused under the global adapter-name invariant.
5. Plan author provisions and resolves source bindings, while survey/extract resolve them again; source pins and `specify_floor` can no longer bypass deployment policy.
6. The Wasm deployment preserves package hydration, local component caching, store lookup, digest verification, and existing init artifacts.
7. The linked deployment resolves bare names to actual catalog versions and satisfies exact package pins present in the catalog.
8. Mismatched pins and local component selectors fail as `adapter-not-linked`; a local path can never select a same-named compiled adapter.
9. Fresh linked bare initialization persists the exact package identity when the catalog entry declares one, while upgrade preserves an existing binding.
10. The runtime `specify_floor` gate remains active for linked entries.
11. `Catalog` and `Provider` carry no model type parameter; no `Binding` trait or `adapters!` macro remains.
12. Catalog construction validates identities, duplicate axes, cross-axis collisions, and reference shelf coherence; fixture source/target identities no longer collide.
13. `linked` contains no concrete adapter, fixture, eval, or lab dependency in production dependencies.
14. Command execution accepts execution paths, model, catalog, command info, and argv values; libraries do not construct Tokio runtimes.
15. `CommandInfo` makes `--version` flavor-aware without creating a caller-supplied engine version, and shared CLI help contains no false guest/component/native-only claims.
16. Product command execution anchors at a canonical current directory; lab-only `--project-dir` is canonicalized before provider construction.
17. Cache isolation is explicit in execution context; no linked/eval/fixture path mutates `SPECIFY_PROJECT_CACHE` after runtime startup.
18. Online providers fail loudly when reference documents cannot be served, no-op for no-doc catalogs, share one listener across clones, and expose an awaited shutdown path used by command execution.
19. `eval` receives workspace root, catalog, and model factory values, constructs neither concrete adapters nor Cursor backends, and creates no runtime.
20. Scenario reports derive the effective model from observed requests plus the factory-supplied default; `SPECIFY_MODEL` has one explicit composition-root read path.
21. Specify workflow integration tests use offline `linked` plus fixture adapters and caller-held `RecordingModel` handles.
22. The adapter repository's lab and `composition` executable share one first-party catalog declaration.
23. The `composition` executable has no dependency on `eval` or `lab`; `composition` and both labs remain unpublished.
24. `cargo make dev`, `cargo make eval`, and prompt scenarios preserve their lab behavior.
25. The Wasm workflow guest, component manifests, shipped Wasm runtime behavior, and current Wasm release surface remain intact.
26. Linked tests explicitly avoid claiming component ABI, WIT, isolation, digest, or adapter-store coverage.
27. The linked command path is documented as single-flight; eval guards its persistent sandbox against concurrent writers.
28. Specify and adapter-repository checks enforce their respective dependency and catalog boundaries.
29. Full local CI passes in both repositories, or unavailable live gates are reported precisely.
30. Documentation never presents `lab` as the linked install path and explicitly leaves linked distribution package/archive/install/release identity to follow-up work.

## Risks and mitigations

### Linked behavior is mistaken for Wasm conformance

Mitigation: keep component boundary gates explicit and document linked tests as workflow/host coverage only.

### Linked adapters have full process authority

Linked adapter code can access process environment and filesystem beyond the provided context, panic the command, and share dependency/global state with other adapters.

Mitigation: treat catalog entries as trusted code, document the trust boundary, keep untrusted/dynamic adapters on the Wasm deployment, and preserve separate component gates.

### Static identity is asserted rather than digest-verified

The linked catalog claims package identity from compiled Rust code; it does not prove equivalence to published component bytes.

Mitigation: validate identity against crate/package configuration now. Build provenance, bundle attestation, and release identity belong to the future distribution work.

### Product identity remains undefined

This RFC creates a product-shaped executable in the internal `composition` source package but does not decide the distribution package name, installed binary name, coexistence with the Wasm distribution, bundle version, archive naming, or release cadence.

Mitigation: keep placeholders explicit, never route operators to the lab, and require a follow-up distribution decision before advertising or attaching linked release artifacts.

### Component provisioning regresses

Moving byte hydration behind `Provisioner` could accidentally alter existing store/cache/init behavior.

Mitigation: land the deployment-neutral selector/provisioner stage first and pin current component behavior through CLI integration tests before extracting linked.

### Cursor dependencies leak into ordinary tests

Mitigation: keep `cli` and `cursor` explicit; default linked tests use offline providers and scripted models.

### Eval regains concrete adapter or model dependencies

Mitigation: eval consumes catalog and model factory values; manifest checks reject concrete adapters, fixture, and Cursor dependencies.

### Reference listeners leak or silently disappear

Mitigation: one owned lazy `ReferenceHost` per online provider graph, explicit bind failures with a stable detail prefix, shared ownership, and shutdown tests.

### Explicit cache plumbing expands the core change

Removing process-global mutation touches operation context and cache call sites.

Mitigation: keep the value small and path-only, preserve environment lookup at process startup, and land it before the crate migration so failures stay local.

### `omnia_guest::Model` grows an erasure-resistant surface

A borrowed streaming or generic-return API may resist boxing.

Mitigation: the erasure remains one private linked module; adapters stay generic and require no change if the host returns to a generic model later.

### Cross-repository development becomes lockstep

Mitigation: revision-pin `adapter`, `linked`, and `eval`; keep sibling path patches for co-development only; validate the first-party catalog in adapter CI.

### Single-flight assumptions are violated

Mitigation: document one command/provider graph per process, use independent providers for embedding, and guard eval's persistent sandbox.

## Alternatives considered

### Keep the current crate names

Rejected. `harness` conflates deployment host and eval, while the current `eval` package names a composition binary. Moving that binary to `lab` frees the established `eval` term for the reusable library.

### Keep the model-generic catalog behind `Binding` and `adapters!`

Rejected. The catalog already erases adapter operations; retaining the model generic preserves a factory trait, macro, generic provider, and generic entrypoints to avoid one vtable hop on network-bound calls.

### Reject every pinned reference in linked execution

Rejected. Bare names are development shorthand while exact pins are production identities. A linked deployment declaration has enough information to satisfy its own exact compiled identity. Refusing all pins makes ordinary projects deployment-bound and reports linked adapters as `0.0.0`.

### Put linked version identity in adapter metadata

Rejected. Identity and non-identity metadata remain separate. Component identity comes from its package/artifact; linked identity comes from the SDK implementor's static descriptor. The WIT metadata answer continues to carry only compatibility, inputs, and platforms.

### Keep `{ name, version }` references and low-level `Hydrator::fetch`

Rejected. The reduced reference loses local-component provenance, and initialization invokes component policy before linked resolution. A typed selector plus deployment-level provisioning makes each flavor own its legal inputs.

### Keep scoped process-environment cache mutation

Rejected. It is unsafe in a multithreaded reusable host and cannot support independent providers in one process. Cache placement is an execution-context value.

### Let libraries create Tokio runtimes

Rejected. A sync `main` helper is convenient for one binary but fails under an existing runtime and splits runtime policy between command and eval libraries. Composition roots already exist and should own runtime creation.

### Name the eval library `trial`

Rejected. The package owns trials, scenarios, grading, telemetry, and sandboxing. `eval` is the established umbrella and already matches the task name.

### Fold eval into `linked` behind a feature

Rejected. It blurs the product host with lab machinery and weakens dependency enforcement. A command-only composition must not pull eval code.

### Duplicate the first-party catalog in product and lab binaries

Rejected. The lab would stop evaluating the intended static set as soon as one declaration drifted. `composition`'s library target owns the catalog once.

### Put the first-party linked executable in Specify

Rejected. It would create a Specify-to-`specify-adapters` dependency or move adapters in-tree.

### Treat the lab as the linked product

Rejected. The lab includes eval UX and scratch behavior. It is unpublished tooling, not the operator distribution.

### Define final linked product identity in this RFC

Deferred. Package naming, installed command coexistence, bundle versioning, release cadence, and artifact attachment require a distribution decision informed by the final linked composition. The architecture exposes flavor and catalog identity without preselecting those answers.

### Move the model bridge to Omnia now

Deferred. The bridge is an Omnia concern, but upstreaming it is not required to separate Specify's deployment and eval boundaries. Keep one private parity-tested implementation until the upstream API exists.

### Move `linked` to a separate repository

Rejected. It evolves atomically with workflow, transport, and adapter SDK contracts; Specify's tests consume it directly.

## Consequences

- Specify has one workflow product with two explicit deployment flavors.
- `linked` is a reusable deployment host; `eval` and `lab` are lab-only.
- Composition is value-level end to end: catalog, model, execution paths, command identity, and argv.
- Adapter selectors retain their input kind, and provisioning policy belongs to the active deployment.
- A linked catalog carries real package identities, satisfies exact matching pins, and rejects local components without substitution.
- `DynModel` removes the generic binding tower while leaving adapter operation methods generic.
- Command and eval libraries are asynchronous; composition roots own Tokio.
- MCP listener and cache lifecycles become explicit rather than detached or process-mutating.
- The adapter repository's internal `composition` package owns one first-party catalog shared by its executable and lab.
- Linked source adapters and target adapters are trusted in-process code, not isolated components.
- The Wasm deployment remains necessary and authoritative for WIT, isolation, dynamic provisioning, store, and digest behavior.
- The linked distribution's final package, install, bundle, and release identity remains a deliberate future concern.
