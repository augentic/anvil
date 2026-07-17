# RFC-61: Specify-owned adapter SDK

> **Status: Accepted — implemented.** The SDK now lives at `specify/crates/adapter`; the adapters repository consumes it as a revision-pinned git dependency. This RFC proposed moving the shared `adapter` crate from `augentic/specify-adapters` into `augentic/specify`, preserving its current responsibilities and extending it into the canonical Rust SDK for the `specify:adapter` contract. Owns: WIT source, generated guest bindings, contract DTOs, the per-axis operations traits, adapter judgment scaffolding, and shared guest support

## Abstract

Specify owns the `specify:adapter` WIT package, but the Rust SDK used to implement that contract currently lives in `augentic/specify-adapters`. The engine separately generates workflow and fixture bindings, carries a parallel native seam vocabulary, and generates answer schemas that the adapters repository vendors back into its shared crate.

This RFC moves the existing `specify-adapters/crates/adapter` crate almost unchanged into `specify/crates/adapter` and makes it the canonical adapter SDK. The moved crate will own the WIT source and every Rust binding generated from it, retain the shared adapter seam, judgment, answer, phase, registry, and references support it already provides, and grow to cover the contract-level facilities currently duplicated across the engine, testkit, native eval harness, and adapters.

Adapter implementations, prompts, engineering standards, validators, and operation-specific orchestration remain in `augentic/specify-adapters`. Specify owns the SDK and contract; the adapters repository consumes a published, versioned SDK.

## Motivation

The current ownership is split across two repositories:

- `augentic/specify` owns and publishes `wit/specify.wit`.
- The Specify workflow guest generates the `workflow` world in `src/lib.rs`.
- Specify testkit generates the combined `adapter` world in `crates/testkit/src/wit.rs`.
- `augentic/specify-adapters/crates/adapter` generates the `source-adapter` and `target-adapter` worlds.
- The adapters repository vendors a copy of `specify.wit`.
- `project::seam` and `adapter::seam` independently model much of the same contract.
- Adapter answer schemas are generated from engine-owned Rust types and vendored into the adapters repository.
- The native adapter eval provider maps between the two seam vocabularies.
- The eval catalog dispatches to adapters through a duck-typed `$krate::operations::$op` macro convention, a third Rust restatement of the operation contract that no compiler checks as a contract.

The split produces multiple authored or generated representations of one contract. A WIT change requires coordinated edits and regeneration in both repositories, while Rust consumers cannot depend on one canonical SDK surface.

The existing `adapter` crate is already more than a binding wrapper. It contains the reusable behavior every adapter guest needs:

- bindgen-free source and target DTOs;
- generated source and target export bindings;
- model judgment and bounded repair helpers;
- answer schemas, parsers, and deterministic validation tails;
- target phase and report helpers;
- embedded prose registry support;
- the common MCP references server;
- the model capability vocabulary used by adapter cores and native tests.

These are contract-level adapter facilities. They are not specific to any first-party adapter and should evolve with the contract owned by Specify.

## Goals

1. Give `specify:adapter` one authored WIT source and one Rust SDK owner.
2. Move the existing adapter support crate without reducing it to a bindings-only package.
3. Generate all four WIT worlds in the SDK:
   - `workflow`;
   - `source-adapter`;
   - `target-adapter`;
   - `adapter`, for the combined fixture.
4. Preserve thin, wasm-only adapter guest shims and bindgen-free adapter cores.
5. Make the SDK the canonical home for native contract DTOs and shared operation behavior.
6. Remove vendored WIT and answer schemas from `augentic/specify-adapters`.
7. Reduce or remove the mapping layer between `adapter::seam` and `project::seam`.
8. Preserve independent repository builds and releases through a published SDK dependency.
9. Leave room for additional adapter-authoring facilities without moving adapter-specific behavior into Specify.

## Non-goals

- Moving first-party adapter implementations into Specify.
- Moving adapter prompts, references, engineering standards, or validators into Specify.
- Making the Specify runtime depend on concrete adapters.
- Replacing the `specify:adapter` component contract with a Rust-only interface.
- Adding a Wasmtime host binding layer before a native host consumer exists.
- Moving workflow lifecycle or artifact ownership into the adapter SDK.
- Combining the engine and adapters eval trials into one executable.

## Proposed ownership

The crate moves to:

```text
specify/
  crates/adapter/
    Cargo.toml
    wit/
      specify.wit
    schemas/
      answers/
        leads.schema.json
        evidence.schema.json
        report.schema.json
    src/
      lib.rs
      guest.rs
      source.rs
      target.rs
      fixture.rs
      workflow.rs
      seam.rs
      answers.rs
      call.rs
      phase.rs
      references.rs
      registry.rs
```

The exact module grouping may be flattened, but the ownership boundary is fixed:

- raw generated bindings and export macros belong to the SDK;
- contract DTOs and shared adapter behavior belong to the SDK;
- engine orchestration and adapter resolution do not;
- adapter-specific operations and prose do not.

The published Cargo package should use a Specify-qualified name such as `specify-adapter`. It may expose the Rust library name `adapter`, allowing existing adapter source to retain imports such as:

```rust
use adapter::source::{AdapterId, AdapterMetadata, Error, Evidence, Guest, Lead};

adapter::source::export!(Adapter with_types_in adapter::source);
```

Consumers can preserve the dependency key:

```toml
adapter = { package = "specify-adapter", version = "..." }
```

## WIT ownership and binding generation

The canonical `specify.wit` moves under `crates/adapter/wit/`. Keeping the WIT tree inside the crate makes `path: "wit"` stable in source checkouts and published Cargo packages, following Omnia's `wasi-*` crate convention.

One crate does not imply one `wit_bindgen::generate!` invocation. Each world has a distinct import or export shape and must retain its own generation:

- `workflow` imports `source` and `target`;
- `source-adapter` exports only `source`;
- `target-adapter` exports only `target`;
- `adapter` exports both axes for the fixture component.

The source and target worlds must remain separate. Generating the combined fixture world for production adapters would make a component export both axes and violate the one-axis adapter contract.

All generated modules remain `wasm32`-only. The SDK has no native `host.rs` initially because Specify does not directly host these interfaces through `wasmtime::component::bindgen!`; Omnia provides the runtime and dispatch boundary. A host module should be added only when a concrete host-side consumer requires it.

## SDK surface

### Generated bindings

The SDK exposes stable modules for each world:

- `adapter::workflow` for the Specify core guest;
- `adapter::source` for source adapter guests;
- `adapter::target` for target adapter guests;
- `adapter::fixture` for the combined test component.

The source, target, and fixture modules expose public export macros. Source and target retain the current generated-type conversions from the native seam.

### Native seam

The current `adapter::seam` moves with the crate. It remains bindgen-free so adapter cores compile and test natively without WIT-generated types.

The SDK becomes the preferred native representation of values that cross `specify:adapter`, including:

- adapter errors;
- source metadata, leads, evidence, claims, and authority;
- target metadata, inputs, platforms, working trees, merge phases, reports, findings, and outputs;
- call-scoped adapter context.

The engine should converge on these contract types instead of maintaining a parallel mirror. Engine-owned envelopes remain in their current crates. For example, stamped build request and report fields such as `version`, `slice`, and `target` are workflow artifacts rather than adapter contract values and remain under the workflow wire layer.

The `Source` and `Target` native capability traits are candidates to move from `project::seam` into the SDK because they directly mirror the WIT interfaces. Resolver capabilities and the workflow `Capabilities` bundle remain in `project`: resolution and orchestration are engine concerns rather than adapter contract concerns.

This convergence may be staged, but the target state is one native contract vocabulary shared by:

- adapter cores;
- the native adapters eval provider;
- the Specify workflow provider;
- fixture adapters and seam tests.

### Operations traits

The SDK owns one trait per axis stating what an adapter implements: `adapter::Source` and `adapter::Target` — wasm-free traits over the seam DTOs carrying the adapter name, resolve-time metadata, the embedded doc table, and the judgment operations as associated functions generic over `Model`. [RFC-62](rfc-62-adapter-component-declarations.md) owns their definition and the export macros that wire an implementor into a component.

The traits sit below the capability traits and are deliberately distinct from them; both pairs mirror the same WIT interfaces, so they share the bare names `Source` / `Target` and disambiguate by path:

- **Operations traits** (`adapter::Source` / `adapter::Target`) — what an *adapter implements*: static associated functions on a unit type, generic over the model backend, one implementor per adapter crate.
- **Capability traits** (`project::seam::Source` / `project::seam::Target`) — what the *engine calls*: instance-based, `<axis>:<name>`-routed, implemented by providers.

Providers bridge the two. The WASI provider implements the capability traits by dispatching components over WIT; the native eval provider implements them by dispatching operations-trait implementors linked into the shim. This replaces the eval catalog's duck-typed `$krate::operations::$op` macro convention with compile-checked trait bounds, and lets one native type implement both operations traits for the testkit fixture — the one-axis rule constrains component exports, not native impls, and stays enforced at the export macros.

### Judgment support

The existing model call and repair behavior moves with the crate:

- schema-gated model requests;
- adapter reference grants;
- workspace lending;
- bounded repair for non-mutating source legs;
- one-shot target legs that may mutate the workspace;
- shared target phase and report helpers.

The repair budget and behavior are part of the adapter SDK contract. They should no longer be maintained as an informal mirror of an engine constant. If engine and adapter judgment intentionally share one policy, that policy should be exposed from one dependency leaf or tested for parity explicitly.

### Answer schemas

The SDK owns the answer schemas for adapter operations:

- `leads`;
- `evidence`;
- `report`.

Their Rust types, schema generation, committed goldens, parsers, and deterministic tails should be generated and tested in the SDK. The adapters repository stops vendoring these files.

Engine-only judgment answers such as `proposal` and `synthesis` remain in their workflow crates. They are not part of the adapter SDK merely because they use the same model host.

### Embedded references

The existing registry and MCP reference server move with the crate. They are generic adapter guest infrastructure: every adapter embeds a document table and exposes the same `list_docs`, `read_doc`, and `doc://` surfaces.

The SDK does not own the documents themselves. Prompt bodies, references, and engineering standards remain compiled from each adapter's `prose/` tree in `augentic/specify-adapters`.

## Repository boundary after the move

`augentic/specify` owns:

- the `specify:adapter` WIT source and wasm-pkg publication;
- the Rust adapter SDK and its Cargo publication;
- generated bindings for every world;
- contract DTOs and native traits;
- adapter answer schemas and deterministic validation;
- generic judgment, phase, registry, and references support;
- workflow and fixture bridges to the SDK.

`augentic/specify-adapters` owns:

- first-party source and target adapter crates;
- operation implementations;
- adapter-specific prompts and references;
- engineering standards;
- target validators and composition policy;
- native prompt scenarios and adapter-specific grading;
- component builds, composed tests, and releases.

The runtime dependency direction remains one-way:

```text
specify-adapters -> specify-adapter SDK
```

Specify never depends on concrete adapter crates.

## Migration

### Stage 1: move without redesign

1. Create `specify/crates/adapter` from the current adapters crate.
2. Move the canonical WIT source into the crate.
3. Retain the current source and target bindings, seam, answers, judgment, phase, registry, and references modules.
4. Adapt the moved crate's tests to Specify's integration-first testkit and workspace conventions.
5. Publish the SDK before changing the adapters repository.

This stage should preserve behavior and public paths wherever practical.

### Stage 2: centralize every world

1. Add the `workflow` world currently generated in `specify/src/lib.rs`.
2. Add the combined `fixture` world currently generated in `crates/testkit/src/wit.rs`.
3. Change the core guest and testkit to consume the SDK modules.
4. Replace testkit `From` implementations that become invalid under Rust's orphan rules with explicit mapping functions or mappings owned by a crate that owns one side.
5. Remove direct `wit-bindgen` use outside the SDK.

### Stage 3: switch adapters

1. Publish or otherwise pin the SDK release.
2. Replace `specify-adapters/crates/adapter` with the external SDK dependency.
3. Remove the vendored `wit/specify.wit`.
4. Remove the vendored answer schemas.
5. Update component build, publication, and contributor documentation.
6. Preserve existing guest shim imports through the `adapter` dependency key and library name.

An interim compatibility crate in `specify-adapters/crates/adapter` is acceptable if adapter-specific helpers cannot move atomically. It must re-export the SDK and shrink to zero; it is not a permanent second SDK.

### Stage 4: converge native seams

1. Compare `project::seam` and `adapter::seam` field by field.
2. Move contract-level DTOs and the `Source` / `Target` traits to the SDK.
3. Retain workflow-only envelopes and capability bundles in `project`.
4. Update the workflow provider, testkit provider, and native adapters eval provider to use the shared types.
5. Delete mapping code made redundant by the shared vocabulary.

This stage must not force artifact-domain types or workflow lifecycle types into the SDK. Where a workflow artifact enriches a contract value with provenance or caller-owned fields, keep an explicit projection at that boundary.

### Stage 5: extend the authoring SDK

Once ownership is established, additions may include:

- the per-axis operations traits and the `source!` / `target!` macros ([RFC-62](rfc-62-adapter-component-declarations.md)) — the traits state the adapter-side contract once, and the macros carry only the wiring that must expand in the leaf component crate;
- metadata builders that enforce adapter identity and compatibility-floor invariants;
- shared contract conformance fixtures, written once as functions generic over the operations traits and instantiated per adapter;
- schema and WIT version constants;
- reusable operation context and reference-grant construction;
- typed helpers for common report and finding coherence;
- compile-time or test-time checks that one adapter exports exactly one axis.

Extensions must serve multiple adapters and remain contract-level. Adapter-specific prompt sequencing, file generation, and validation stay with the adapter implementation.

A harness generic over the operations traits also opens the path to a shared eval core: the engine and adapters eval crates currently keep a verbatim-mirror `native.rs` in sync by hand, and a trait-generic harness lets the engine instantiate it with the testkit fixture while the adapters repository instantiates it with the real implementors.

## Versioning and publication

The Rust SDK and WIT package are related but distinct artifacts:

- the WIT package versions the component wire contract;
- the Cargo package versions generated bindings and source-level SDK behavior.

Every SDK release pins exactly one `specify:adapter@<version>` WIT package and exposes that version for diagnostics and compatibility tests. An SDK-only helper change need not force a WIT package bump. A WIT change requires a new immutable WIT package version and a corresponding SDK release.

The adapters repository should consume a published Cargo package rather than a Git dependency in its root workspace. This preserves ordinary adapter builds without a sibling checkout or Specify repository authentication. Local cross-repository development may use an uncommitted Cargo path patch.

The adapter metadata `specify-floor` remains the compatibility statement for the running Specify CLI. It is not replaced by the SDK crate version.

## Testing

The moved SDK follows Specify's integration-first policy:

- native contract and helper behavior is tested through the SDK's public API under `crates/adapter/tests/`;
- generated world shape is compile-checked through the core guest, fixture guest, and adapter components;
- WIT/component conformance remains covered by the adapters repository's composed tests;
- adapter prompt behavior remains covered by adapter-native tests and live scenarios;
- the operator-invoked change example remains the end-to-end deployed seam.

The migration is complete when:

1. `specify.wit` has one authored copy under the SDK crate.
2. No crate outside the SDK invokes `wit_bindgen::generate!` for `specify:adapter`.
3. Every first-party adapter builds with the published SDK.
4. Existing adapter guest shims retain the same one-axis exports.
5. Adapter answer schemas have one generated owner.
6. The native eval harness no longer maintains avoidable contract DTO mappings.
7. Specify and specify-adapters full local CI gates pass.
8. The composed deployment tests and change example pass without changing WIT interface identities.

## Risks and invariants

- **The SDK must remain adapter-generic.** Moving it into Specify must not make Specify the owner of first-party adapter behavior.
- **One-axis components remain mandatory.** Shared generation cannot make a source component export the target world or vice versa.
- **The SDK is a dependency leaf with respect to workflow code.** It must not depend on `project`, `slice`, `change`, or `transport`.
- **Generated bindings remain an edge concern.** Native adapter cores continue to use bindgen-free SDK types.
- **Rust API stability becomes public contract.** Moving to a published SDK expands the compatibility surface beyond WIT and requires deliberate SemVer.
- **Release ordering matters.** A new WIT package and SDK release must land before adapters consume them.
- **Multi-world generation needs deployment tests.** Generated metadata and export macros must not cause unused worlds to appear in a component.
- **No permanent compatibility shell.** A temporary adapters-local re-export crate must be removed after migration.

## Open questions

1. Should the Cargo package be named `specify-adapter`, `specify-adapter-sdk`, or another registry-safe name while retaining the Rust crate name `adapter`?
2. Should native seam convergence happen in the initial move or in a follow-up after the bindings and schemas are centralized?
3. Which contract DTOs currently owned by `artifacts` should move into the SDK, and which should remain artifact-domain projections?
4. The SDK owns the per-axis *operations* traits ([RFC-62](rfc-62-adapter-component-declarations.md)); should the workflow *capability* traits (`project::seam::Source` / `Target`) also move into the SDK, or does `project` retain them as provider surfaces? If they move, they take a module path distinct from the crate root (for example `adapter::seam`), since the operations traits hold the bare `Source` / `Target` names there.
5. Should the Rust SDK publish to crates.io or the same organizational package infrastructure used for components?
6. Should adapter SDK releases be independently versioned from the WIT package or share major versions while allowing independent minor and patch releases?
