# RFC-62: Adapter operations traits and component exports

> **Status: Accepted — implemented.** Depends: [RFC-61](rfc-61-adapter-sdk.md) · Owns: the per-axis adapter operations traits and the source and target export macros. The traits, export macros, and the shared native harness over them (`specify/crates/harness`, consumed by both the engine's `eval` wrapper and the adapters repository's `engine`) are in place.

## Abstract

Every source adapter repeats the same WIT conversion, `WasiModel` routing, context construction, and references-server export. Every target adapter repeats the corresponding target glue. Beyond the wasm shims, the same operation contract is restated as duck-typed macro conventions in the native eval catalog. Only the adapter name, operation bodies, and embedded document registry actually vary.

The adapter SDK will state the contract once, as one Rust trait per axis:

- `adapter::Source`;
- `adapter::Target`.

An adapter implements its axis trait on a unit type. A minimal wasm-only export macro per axis (`adapter::source!`, `adapter::target!`) performs only the wiring that must expand in the leaf component crate: the generated WIT `export!`, the `wasi:http` references export, and the crate-version stamp. Every other consumer of the contract — the wasm shim body, the native eval catalog, the testkit fixture, conformance fixtures — becomes a derivation of the trait.

## Motivation

The `specify:adapter` contract has one WIT statement but several Rust restatements, none of which the compiler checks *as a contract*:

1. Each adapter's hand-written `guest.rs` re-encodes the operation set, conversions, and context construction — roughly fifty lines of wasm-only glue per adapter that can drift whenever context construction, conversion, or reference serving changes.
2. The native eval catalog (`eval/src/catalog.rs` in `augentic/specify-adapters`) re-encodes it as macro conventions: `linked!` and five helper macros dispatch to `$krate::operations::$op` paths and hand-roll a vtable (`metadata: fn() -> Metadata`, `docs: fn() -> &'static [Doc]`) — an interface enforced by nothing but macro-expansion errors.
3. Adapter crates follow the `operations::*` / `registry::docs` module-shape convention those encoders assume, documented only by imitation.

An earlier draft of this RFC proposed per-axis declaration macros (`adapter::source! { name, metadata, survey, extract, docs }`). That removes the first restatement but leaves the second and third conventional, and its signature contract lives only in macro documentation, with mismatches erroring inside the expansion. A nominal trait removes all three restatements: the trait is the single Rust statement of what an adapter implements, the compiler enforces it at every `impl` site, and both shims shrink to wiring.

## Proposal



### The operations traits

The SDK defines one wasm-free trait per axis over the bindgen-free seam vocabulary. The entries and signatures below are normative; exact future bounds follow `omnia_guest::Model`'s existing posture (open question 1).

```rust
pub trait Source {
    /// The axis-local adapter name, e.g. `"captures"`.
    const NAME: &'static str;

    /// Resolve-time metadata.
    fn metadata() -> seam::SourceMetadata;

    /// The embedded prose registry.
    fn docs() -> &'static [registry::Doc];

    /// Lightly survey the bound source into a lead set.
    fn survey<P: Model>(
        model: &P,
        ctx: &Context<'_>,
    ) -> impl Future<Output = Result<Vec<seam::Lead>, seam::Error>> + Send;

    /// Thoroughly extract one lead's Evidence.
    fn extract<P: Model>(
        model: &P,
        ctx: &Context<'_>,
        lead: &seam::Lead,
    ) -> impl Future<Output = Result<seam::Evidence, seam::Error>> + Send;
}
```

```rust
pub trait Target {
    const NAME: &'static str;

    fn metadata() -> seam::TargetMetadata;

    fn docs() -> &'static [registry::Doc];

    /// The synthesis-guidance prompt. Async and fallible per the WIT
    /// contract; deterministic implementors ignore `model`.
    fn guidance<P: Model>(
        model: &P,
        ctx: &Context<'_>,
    ) -> impl Future<Output = Result<String, seam::Error>> + Send;

    /// Build `slice` against the lent working tree.
    fn build<P: Model>(
        model: &P,
        ctx: &Context<'_>,
        slice: &str,
        inputs: &[seam::Input],
        tree: &seam::WorkingTree,
    ) -> impl Future<Output = Result<seam::Report, seam::Error>> + Send;

    /// Run one phased merge gate.
    fn merge<P: Model>(
        model: &P,
        ctx: &Context<'_>,
        slice: &str,
        phase: seam::MergePhase,
        tree: &seam::WorkingTree,
    ) -> impl Future<Output = Result<seam::Report, seam::Error>> + Send;
}
```

The judgment operations stay generic over `Model`, so native tests keep binding scripted doubles and the wasm shim binds `WasiModel`. The methods are associated functions, not `&self` methods: each component contains exactly one adapter implementation and carries no instance state. The traits are deliberately not object-safe; no consumer wants `dyn` dispatch, and the native catalog dispatches statically.

These traits state what an *adapter implements*. They are distinct from the workflow capability traits (`project::seam::Source` / `project::seam::Target`), which state what the *engine calls*: instance-based, `<axis>:<name>`-routed, implemented by providers. Both pairs mirror the same WIT interfaces, so they share the bare names and disambiguate by module path. Providers bridge the two — the WASI provider dispatches components over WIT, and the native eval provider dispatches trait implementors (see [Native consumers](#native-consumers)). RFC-61's operations-traits section owns this layering.

### The export macros

Two wasm-only macros wire a trait implementor into the component exports:

```rust
adapter::source!(Captures);
```

```rust
adapter::target!(Vectis);
```

Each macro expands, in the declaring adapter crate, to:

1. the generated WIT `Guest` implementation, delegating every operation to the trait implementor through the WIT-to-seam conversions the SDK already owns;
2. guest context construction, resolving the adapter's MCP references URL from `A::NAME`;
3. the `export!` invocation for exactly the declared axis world;
4. the `wasi:http` references-server export, serving `A::docs()` with the server identity derived from `A::NAME` (`<name>-references`) and the component version from `env!("CARGO_PKG_VERSION")`, expanded in the declaring crate.

The macro body is thin by construction: everything with behavior lives in named, typed SDK functions and the trait; the macro contributes only what must expand at the leaf (the `export!` macros and the version stamp). The references-server identity moves from a compile-time `concat!` over a name literal to a projection of `A::NAME`, removing the last place an adapter restated its own name.

### Why a trait plus a minimal macro

A macro is unavoidable: the WIT export macros and `env!("CARGO_PKG_VERSION")` must expand in the final component crate, so no trait-only design can eliminate the shim. The real choice is between a declaration macro that carries the whole contract as key-value conventions and a trait that carries the contract nominally with the macro reduced to wiring. The trait wins on every axis that matters here:

- **Diagnostics.** A drifted operation signature fails on the adapter's `impl` block with rustc's trait-mismatch message, not deep inside a macro expansion.
- **Discoverability.** The contract is rustdoc'd and IDE-navigable; a key-value macro documents its expected signatures only in prose.
- **One contract, many consumers.** The wasm export macro, the native eval catalog, the testkit fixture, and conformance fixtures all consume the same trait; a declaration macro fixes only the wasm copy and leaves the native encoders conventional.
- **Identical drift-proofing.** Adding a WIT operation adds a trait method, which fails compilation in every adapter — the same compile-failure guarantee the declaration macro offered.

The path-based declaration macro had one genuine technical advantage — macro paths sidestep the higher-ranked `AsyncFn` bounds a value-level generic API would need to accept `operations::survey` as a parameter. The trait keeps that advantage: `A::survey` is a path resolved at expansion, not a function value.

The accepted costs: `metadata` ceases to be `const fn`; operations move from free functions to associated functions (`Captures::survey(&model, &ctx)` — still directly callable from native tests); and the traits cannot be trait objects, which nothing requires.

## Worked source example

The captures crate keeps its wasm-free operation bodies and moves them into the trait impl:

```rust
pub struct Captures;

impl adapter::Source for Captures {
    const NAME: &'static str = "captures";

    fn metadata() -> SourceMetadata {
        SourceMetadata { specify_floor: None }
    }

    fn docs() -> &'static [Doc] {
        registry::docs()
    }

    async fn survey<P: Model>(model: &P, ctx: &Context<'_>) -> Result<Vec<Lead>, Error> {
        // Adapter-specific judgment, unchanged.
    }

    async fn extract<P: Model>(
        model: &P, ctx: &Context<'_>, lead: &Lead,
    ) -> Result<Evidence, Error> {
        // Adapter-specific judgment, unchanged.
    }
}
```

Its embedded prose registry remains unchanged (`adapter::registry!()`), and the complete wasm shim becomes:

```rust
adapter::source!(crate::Captures);
```

Native tests call `Captures::survey(&scripted, &ctx)` exactly as they called `operations::survey` before, with any non-WASI `Model`.

## Worked target example

Vectis implements the target trait over its existing operation bodies:

```rust
pub struct Vectis;

impl adapter::Target for Vectis {
    const NAME: &'static str = "vectis";

    // metadata, docs, guidance as today.

    async fn build<P: Model>(
        model: &P, ctx: &Context<'_>, slice: &str, inputs: &[Input], tree: &WorkingTree,
    ) -> Result<Report, Error> {
        // Adapter-specific build orchestration, unchanged.
    }

    async fn merge<P: Model>(
        model: &P, ctx: &Context<'_>, slice: &str, phase: MergePhase, tree: &WorkingTree,
    ) -> Result<Report, Error> {
        // Adapter-specific merge gates, unchanged.
    }
}
```

Its wasm shim becomes:

```rust
adapter::target!(crate::Vectis);
```

The expansion converts WIT `Input`, `WorkingTree`, and `MergePhase` values into seam values, invokes the trait operations with `WasiModel`, and converts the report or error back to generated WIT values — the glue every hand-written target shim carries today.

## Native consumers {#native-consumers}

The traits pay for themselves off `wasm32`:

- **The eval catalog.** `linked!`'s helper macros (`source_leg!`, `guidance_leg!`, `build_leg!`, `merge_leg!`, `metadata_of!`) exist to compensate for the missing trait. With the trait, the catalog keeps its one-line-per-adapter table, but each entry becomes a typed constructor (`Entry::of::<Captures>()`) and each dispatch leg a compile-checked `A::survey(model, ctx)` call. The duck-typed `$krate::operations::$op` path convention disappears.
- **The testkit fixture.** One native type may implement both operations traits — the one-axis rule constrains component *exports* and remains enforced where it lives, at the export macros. The fixture stops being the last hand-written copy of the shim glue.
- **Conformance fixtures.** RFC-61's shared contract conformance fixtures become functions generic over the traits (`fn conformance<A: adapter::Source>()`), written once and instantiated per adapter.
- **A shared eval core.** The engine and adapters eval harnesses used to keep a verbatim-mirror module in sync by hand. A harness generic over the operations traits serves both — realized as `specify/crates/harness`: the engine's `eval` wrapper instantiates it with the testkit fixture, the adapters repository's `engine` with the real implementors.



## Scope

- Define `adapter::Source` and `adapter::Target` in the adapter SDK ([RFC-61](rfc-61-adapter-sdk.md) Stage 5).
- Add `source!` / `target!`, centralizing the generated-WIT conversion, guest-context construction, and references-server glue they expand to.
- Convert every first-party adapter to a trait impl plus a one-line shim.
- Rewrite the native eval catalog dispatch over the traits.
- Compile-check every source and target component after migration (`cargo make release` in `augentic/specify-adapters`).



## Non-goals

- Moving adapter operations, prompts, references, or validators into the SDK.
- Replacing the WIT component contract with Rust traits; WIT remains the wire truth and the traits mirror it.
- Object safety or dynamic adapter selection inside a component or the native shim.
- Moving the workflow capability traits (`project::seam::Source` / `Target`) into the SDK; that remains RFC-61's open question.
- Combining the source and target worlds in any component export.
- Changing operation behavior, prompts, or the native test seams.



## Acceptance criteria

1. Every first-party source adapter implements `adapter::Source`, and its wasm shim is a single `source!` invocation.
2. Every first-party target adapter implements `adapter::Target`, and its wasm shim is a single `target!` invocation.
3. Each built component exports exactly its declared axis plus the HTTP references handler, verified by the composed deployment tests.
4. Existing native operation tests pass against the traits' associated functions with non-WASI `Model` implementations.
5. MCP URL lookup, server identity (`<name>-references`), version reporting, and document serving are behavior-identical.
6. The eval catalog dispatches through the traits, with no `$krate::operations::*` path convention remaining.
7. A wrong operation signature fails compilation on the adapter's `impl` block, not inside a macro expansion.



## Risks and invariants

- **Exports remain leaf-owned.** The export macros must expand in the adapter crate; the SDK cannot discover downstream implementations.
- **One axis per component.** `source!` and `target!` each export exactly one axis world; nothing generates the combined fixture world for a production component.
- **No hidden adapter behavior.** The trait and macros own conversion and routing only; operation sequencing remains in adapter impls.
- **The traits stay wasm-free.** They are defined over the seam DTOs and `omnia_guest::Model` only, so adapter cores and native harnesses consume them without generated bindings.
- **Call-site package identity.** The references server reports the declaring adapter's `CARGO_PKG_VERSION`, stamped by the macro in the leaf crate.
- **All-or-nothing shim.** An adapter needing boundary behavior the macro does not model (extra HTTP routes, bespoke conversions) falls back to a hand-written shim over the same trait; the macros take no partial overrides.



## Open questions

1. Should the trait futures carry explicit `Send` bounds to match the workflow capability traits, or follow `omnia_guest::Model`'s posture and let the native side prove `Send` at instantiation?
2. Should source `metadata` carry a default body (`SourceMetadata { specify_floor: None }`, today's value in every first-party source), or stay mandatory for explicitness?
3. Should the references-server identity stay a runtime projection of `A::NAME` (relaxing `References.server_name` from `&'static str`), or should the macro take a name literal and const-assert it against `A::NAME` to preserve the fully static identity?

