# RFC-51: Adapter WIT

> Status: Draft - Depends: RFC-47 (adapter identity), RFC-48 (adapter packaging/registry), RFC-49 (adapter extraction to `specify-adapters`), RFC-50 (adapter-agnostic core) - Framed by: [the effect-oriented architecture](architecture.md) as Stages 0–1 - Hands off: capabilities → [RFC-52](rfc-52-effects.md), brief-typing → [RFC-53](rfc-53-orchestration.md)

## Position in the architecture

This RFC is the **foundation** (Stages 0–1) of the [effect-oriented architecture](architecture.md): the typed WIT contract (the stratum-1 transfer records) and typed tool dispatch that every later stage rides on. It is deliberately scoped to that foundation — two concerns earlier drafts folded in here now live in the stage that owns them:

- **Capabilities / resources** (the narrow host-data accessors) move to [RFC-52](rfc-52-effects.md), which generalizes them from a capability grant into the runtime's first named **data effect**.
- **The typed brief contract and lazy discovery** move to [RFC-53](rfc-53-orchestration.md): once a brief becomes the *body of the `judge` effect*, "the brief binds a WIT signature" becomes "the `judge` call-site declares the signature," so the heavier `implements` / `consumes` / `produces` / `capabilities` machinery is re-evaluated there rather than built out here. Lazy discovery is a standing invariant in the architecture north star.

What remains in RFC-51 is durable and unaffected: the transfer records (stratum 1), typed tool dispatch, and the typed agent envelopes that reuse the same records.

## Abstract

Adapters are invoked today through the generic `wasi:cli/run` world: the host packs **argv**, reads an **exit code**, and exchanges data as **stdout/stderr JSON** plus **preopened directories**. Operation semantics live in argv conventions and in JSON envelopes validated *at runtime* against embedded `*.schema.json` constants.

This RFC proposes replacing that loose contract with a typed **WebAssembly Component Model** contract: one versioned **WIT package** defining every operation's request/report records, **per-axis worlds** that export the deterministic operations, and an **agent brief handoff that reuses the same WIT types** as its serialized envelope. The host calls deterministic adapters through generated, typed bindings; the schema-constant + parity-test machinery collapses into the WIT package as a single source of truth.

This is the *typed realization* of RFC-50's "uniform operation-envelope runtime": RFC-50 says the host's contract is a fixed envelope dispatched generically; this RFC says those envelopes are WIT records and the deterministic operations are WIT exports.

The contract reaches the agent path too, but here only as **shared envelope currency**: an `agent`-executed operation is fulfilled by an LLM running a markdown brief, and the host serializes the *same* WIT request record into that brief's handoff and validates the *same* report record on return. Binding a brief more deeply to its signature — the `implements` / `consumes` / `produces` contract — is deferred to [RFC-53](rfc-53-orchestration.md); here the agent path simply reuses the records, and the brief body stays prose executed by an LLM.

**Every adapter is a component.** Both axes are now *required* to ship a WASM component that implements their axis world — the prose-only / types-only adapter is gone. That component and the adapter's prose (briefs, phase sub-briefs, references) are co-packaged and published to the registry as a single composite extension (RFC-48 packaging/transport), downloaded and resolved as one unit. The `tool` / `agent` split therefore describes *how an operation is executed inside a component-backed adapter*, not *whether a component exists*: a `tool` operation is a callable world export the host invokes through wasmtime; an `agent` operation is fulfilled by the LLM running the co-packaged brief, exchanging the same WIT records through the handoff. The component is the mandatory wasm half of every adapter; the briefs are its prose half.

**Explicit non-goal up front:** adapter **briefs do not become callable WIT functions.** They are markdown executed by an LLM; there is no component to instantiate and no export to invoke. WIT types the data crossing the boundary and makes the *deterministic* operations callable — but the agent execution stays a two-phase handoff and the prose body is never machine-executed.

## Motivation

RFC-50 routes every adapter behavior through one generic operation-envelope runtime and forbids adapter-specific code in the host. The contract those envelopes ride on is still untyped, and that creates four standing costs:

- **Schema/code drift.** The embedded `*_JSON_SCHEMA` constants in `engine/crates/schema/src/constants.rs` need a dedicated byte-parity test (`engine/crates/schema/tests/schemas.rs`) and wire fixtures to police drift between hand-maintained schemas and the DTOs that round-trip them. A typed boundary removes the drift surface instead of policing it.
- **Untyped invocation.** The `wasi:cli/run` path means the operation contract is "argv conventions + parse stdout as JSON." Mistakes surface at runtime as validation errors, not at the binding boundary.
- **Broad capability grants.** Adapters receive filesystem access via preopened directories and the `$CAPABILITY_DIR` env var rather than a *named* set of host capabilities. The world cannot tell, by inspection, what an adapter is allowed to touch.
- **Convention-based errors.** Failures cross as exit codes + stderr text rather than a typed `result<_, error>`.

The Component Model is the native idiom here, not a new dependency: **WASI Preview 2 is itself defined in WIT**, and the host already compiles with the `component-model` feature (see [Starting state](#starting-state)). The interface-definition layer is already present; this RFC uses it for the adapter contract rather than only for WASI.

## Starting state

- **Runtime.** `wasmtime` and `wasmtime-wasi` are pinned at `45.0.0` with the `component-model`, `cranelift`, `runtime`, `cache`, and (for `wasmtime-wasi`) `p2` features enabled (`engine/Cargo.toml`). The Component Model machinery (records, variants, `result`, resources) is therefore already linked into the host binary.
- **Invocation ABI.** `engine/crates/registry/src/host.rs` instantiates each adapter `.wasm` as the prebuilt `wasi:cli/command` world (`Command::instantiate(...)`) and calls `command.wasi_cli_run().call_run(...)`. Arguments are passed via `WasiCtxBuilder::args` (argv[0] = tool name); data crosses via captured stdout/stderr and `preopened_dir` grants plus `$CAPABILITY_DIR`.
- **Operation set.** `SourceOperation` ∈ `{ survey, extract }` and `TargetOperation` ∈ `{ shape, build, merge }` (`engine/crates/workflow/src/adapter/operation.rs`). Each operation is `execution: tool` (single-phase WASI dispatch) or `execution: agent` (two-phase brief handoff, the default). No first-party `build`/`merge` *tool* is wired today — every first-party target is `execution: agent`; source `survey`/`extract` are agent-only.
- **Envelopes.** Request/report shapes are embedded JSON-Schema constants in `crates/schema/src/constants.rs` (`BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`, `EVIDENCE_JSON_SCHEMA`, `LEAD_JSON_SCHEMA`, …), byte-parity-tested against the on-disk `schemas/` tree.

## Scope

**In scope:** the host↔adapter wire ABI for deterministic operations; and the typed envelope shapes for *every* operation on *both* axes, reused by both the tool path and the agent handoff.

### Non-goals

- **Brief bodies stay markdown; brief execution stays an LLM handoff.** The agent path is not a WIT export and is not made callable. This RFC types the *envelopes* the briefs exchange; it does not type, generate, or machine-execute the prose body, and the deeper brief-to-signature contract is [RFC-53](rfc-53-orchestration.md). Correctness of the instructions themselves remains the eval / review layer's job (see [Risks and invariants](#risks-and-invariants)).
- **No workflow or operation-set change.** The lifecycle, the `survey/extract/shape/build/merge` operation set, and adapter identity (RFC-47) are unchanged.
- **Source axis exports nothing callable today, but still ships a component.** Both source operations are agent-only, so the source world currently exports nothing callable; the source adapter is nonetheless required to ship a component (the composite's wasm half — the typed `source` world) and gains callable exports only when a deterministic source tool is written. Mandating the component does *not* turn `survey` / `extract` into deterministic functions; their execution stays a brief handoff.
- **No compatibility shim.** Consistent with the project's pre-1.0 "hard cut" posture, the `wasi:cli/run` → world migration is a clean ABI cut at `specify extension run`, not a dual-path bridge.

## The model

### A. One shared WIT package — the envelope currency

A single versioned package, `specify:adapter@<semver>`, defines an `interface types` carrying every operation's request/report records for both axes (build, merge, shape, survey, extract, evidence), plus the shared `finding` / `adapter-error` shapes. This package becomes the single source of truth for those shapes; the `*_JSON_SCHEMA` constants are generated from it (or retired in favour of it), eliminating the drift the parity test currently guards.

### B. The interface declares every operation; worlds export only the deterministic subset

```wit
package specify:adapter@1.0.0;

// Shared envelope currency — used by BOTH the tool path and the agent handoff.
interface types {
  enum severity { critical, important, suggestion, optional }
  record finding { rule-id: string, severity: severity, detail: string }

  record build-request { slice-name: string, inputs: list<tuple<string, string>> }
  enum build-status { success, failure }
  record build-output { platform: string, path: string }
  record build-report {
    slice-name: string,
    status: build-status,
    findings: list<finding>,
    outputs: list<build-output>,
  }
  // merge-request / merge-report, shape-*, and the source-side
  // survey / extract / evidence records live here too.

  variant adapter-error { invalid-request(string), io(string), internal(string) }
}

// The deterministic operation signatures a `tool` component exports.
// An `agent` operation reuses these same records via the handoff (§C).
interface target {
  use types.{ build-request, build-report, merge-request, merge-report, adapter-error };
  build: func(req: build-request) -> result<build-report, adapter-error>;
  merge: func(req: merge-request) -> result<merge-report, adapter-error>;
}

world target-adapter {
  export target;   // only the operations this component implements
  // Host-capability imports are deferred to RFC-52 (named host-data
  // effects); until then `tool` operations use the existing
  // preopen / $CAPABILITY_DIR grant.
}
```

A deterministic target `world` exports the subset of `target` operations it implements; the host calls `instance.call_build(&mut store, &req)` and gets a typed `result<build-report, adapter-error>`. An operation a target fulfils with a brief is *not* exported by any world — its request and report still cross as these records, but via the agent handoff (§C) rather than a call. The `source` world is analogous; both source operations are agent-only today, so the source world exports nothing callable and `survey` / `extract` run as brief handoffs.

Mandating a component on both axes does **not** force every operation to be a callable export: a component exports only its `tool` operations (so an all-agent adapter's component currently exports nothing callable), while its `agent` operations stay LLM-executed through the handoff. What is now required is that the component *ships* on both axes — ending the prose-only adapter — and that it, plus the adapter's prose, travels as one composite package (§A, Abstract). The wasm artifact is simply always present.

### C. The agent handoff reuses the WIT types

For `execution: agent`, there is no export to call. The host serializes the WIT `build-request` record into the two-phase brief handoff, the agent runs the brief, and the host parses a `build-report` back and validates it at `finalize` against the operation's WIT-derived report type. Host-side Rust types come from `wasmtime::component::bindgen!`; the JSON the brief reads and writes is a projection of the *same* records, so there is one definition for both the typed (tool) path and the serialized (agent) path. Structurally the handoff becomes a typed call — "here is your `build-request` value, here is the signature you fulfil, return a `build-report`" — with an LLM rather than wasmtime as the interpreter and the `finalize` validation as the return-type check.

The brief the agent runs is the prose half of the same composite adapter the component anchors: tool path and agent path share one package and one contract. The component always ships (§B); the host runs the co-packaged brief for `agent` operations and invokes the component export for `tool` operations, but either way the request and report cross as the same WIT records.

### D. Versioning

The `specify` repo **owns and publishes** the `specify:adapter@<semver>` WIT package: it is authored under `specify/wit/`, published to the registry from specify CI (`wkg publish`), and consumed by `specify-adapters` (and any third-party adapter) as a pinned dependency resolved through `wkg`. The dependency direction is strictly one-way — specify produces the contract, adapters consume it — so the package is the single upstream source for both repos' generated bindings.

The package is semver-versioned and ties into RFC-47 adapter identity and the `requires_specify` floor: the host advertises the world version(s) it supports, an adapter targets a world version, and a mismatch is a typed resolve error rather than a runtime surprise.

### E. Brief-typing and lazy discovery → RFC-53

Earlier drafts continued here with a full **typed brief contract** — binding each agent brief to its operation signature via `implements` / `consumes` / `produces` / `capabilities` frontmatter and four authoring-time checks — plus a **lazy reference-discovery** model. That material is **relocated to [RFC-53](rfc-53-orchestration.md)**: once a brief becomes the body of the `judge` effect, the signature is declared at the call-site rather than re-stated in frontmatter, so RFC-53 is the right place to decide how much of that contract is worth building as authoring-time lint. RFC-51 keeps only the shared records those briefs exchange (§A) and the agent handoff that carries them (§C); lazy discovery is a standing invariant of the [architecture north star](architecture.md).

## The hard boundary (non-goal)

A brief is a prompt executed by the LLM; **no WASM component runs the prose**, so there is no export to instantiate or invoke. The contract is therefore hybrid by design, not by accident:

| Operation execution                    | WIT role                                                                                                                                                                                                                              |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `execution: tool` (deterministic WASM) | Callable world export (`build`/`merge`/validators) + typed envelope                                                                                                                                                                   |
| `execution: agent` (markdown brief)    | Typed envelope only — request in, report out, validated at `finalize`; execution stays a two-phase host handoff and the prose body is never machine-executed. Deeper brief-to-signature binding is [RFC-53](rfc-53-orchestration.md). |

The dominant mode today is `agent`, so most operations gain *typed envelopes* but not *callable exports* — yet the adapter still ships a component on both axes (the composite's wasm half), so "no callable export" no longer means "no component." The honest ceiling is that the contract types the data crossing the seam, not the *semantics* of the instructions: an LLM can follow a perfectly-typed brief and still emit a structurally-valid-but-wrong report. Typing raises the floor — no orphan operations, structurally-valid I/O — while correctness stays with the eval / review layer. That ceiling is a feature: it is exactly what lets the host stay agent-runtime-agnostic.

## Phased plan

Each phase is independently mergeable and **must keep `make lint` and `cargo make ci` green**. The ABI break is contained at the single `specify extension run` chokepoint.

### Phase 0 — Author the WIT package + host bindings (no behavior change)

Define `specify:adapter` with the `types` interface and the per-axis worlds; wire `wasmtime::component::bindgen!` host-side. Assert the generated types match the current envelope records (transitional parity), so nothing changes at runtime yet. Publish the package from specify CI (`wkg publish`) so `specify-adapters` can resolve a pinned version (§D) — this establishes specify as the owner/publisher before any adapter consumes the contract.

### Phase 1 — Typed exports for the existing tool components

Re-export a world from the deterministic components (`contract`, `vectis`) instead of `wasi:cli/run`, and route the `execution: tool` dispatch through the generated bindings. Lowest-risk first step: these are real callable exports that exist today behind the argv contract. Shipping a component becomes mandatory on both axes from here on (the composite's wasm half, even when an adapter exports nothing callable yet); the deterministic components are simply the first to carry real exports.

### Phase 2 — Type the agent envelopes

Project the WIT `types` records into the survey/extract/shape/build/merge handoffs the host already drives; retire or regenerate the `*_JSON_SCHEMA` constants + parity test from the WIT package so there is a single source of truth.

### Beyond Phase 2

Two follow-on tracks build on the records and bindings this RFC lands, each in the stage that owns it:

- **Capability / resource model** — replacing `$CAPABILITY_DIR` + broad preopens with named host-data effects — continues in [RFC-52](rfc-52-effects.md).
- **Typed brief contract and lazy discovery** — binding briefs to their signatures and the reference-shelf model — continues in [RFC-53](rfc-53-orchestration.md).

## Decisions to record (open until reviewed)

- **WIT package as schema source of truth** — the fate of the `*_JSON_SCHEMA` constants, the byte-parity test, and the wire fixtures once shapes are generated from WIT.
- `**wasi:cli/run` → custom world migration** — a breaking extension ABI cut; confirm it stays contained to `specify extension run` and the `specify-adapters` `.wasm` build.
- **Agent-handoff serialization** — the JSON projection of the WIT records used by the brief handoff.
- **Versioning, ownership & publishing** — that the `specify` repo owns and publishes `specify:adapter@<semver>` (`wkg publish`) while `specify-adapters` consumes a pinned version (one-way dependency); and how the world version relates to RFC-47 identity and `requires_specify`.
- `**shape` semantics** — whether `shape` is a world export, a host-read manifest-declared file, or an envelope.
- **Operation set vs declared tools** — whether the manifest's declared-tool set (`contract`, `vectis`) and the operation set unify under one world.
- **Capability model** — deferred to [RFC-52](rfc-52-effects.md): named host-data effects vs. the current preopen grant, and which host functions the world exposes.
- **Brief-typing surface** — deferred to [RFC-53](rfc-53-orchestration.md): how much of the `implements` / `consumes` / `produces` / `capabilities` brief contract, its coverage check, and the reference catalog survives once a brief is the body of the `judge` effect.

## Risks and invariants

- **Agent path unchanged.** Most operations remain a handoff; this is "type the envelopes and make the deterministic tools callable," not "turn briefs into functions." The prose-only adapter is gone, but prose *execution* is unchanged. Typing raises the structural floor, not the correctness ceiling (see [The hard boundary](#the-hard-boundary-non-goal)).
- **Toolchain — now mandatory on both axes.** Components + `wit-bindgen` add build steps for adapter authors, and shipping a component is now required for *every* adapter — including the agent-only source adapters (`intent`, `documentation`, `typescript`, `screenshots`, `captures`) and the currently all-agent first-party targets, not just the validators that already pay this cost. Language-agnostic implementation is a benefit, but this is the principal adoption cost; an all-agent adapter still has to produce and publish a component (the composite's wasm half) even when it exports nothing callable yet.
- **wasmtime feature maturity.** v45 ships stable Component Model support for the records, variants, and `result` this RFC uses; the `resource` ergonomics that the named host-data effects need are confirmed in [RFC-52](rfc-52-effects.md) before that stage relies on them.
- **Cross-repo seam (one-directional).** The `specify` repo owns and publishes the `specify:adapter` WIT package; the adapter `.wasm` builds live in `specify-adapters` and consume a pinned published version. The ABI cut is therefore a publish-then-pin sequence — specify ships the world version, `specify-adapters` bumps its pin and re-exports — rather than a symmetric lockstep edit. The workflow contract still spans both repos, so the version bump and the consuming build must be sequenced deliberately.
- **RFC-50 invariant preserved.** The WIT package is generic — it carries no adapter *name* and no adapter *taxonomy*. The host still holds zero adapter-specific code; this RFC types the contract, it does not re-open the host to any adapter.

## Acceptance criteria

1. **Single typed contract.** One `specify:adapter` WIT package defines every operation's request/report on both axes; no hand-rolled DTO or embedded JSON-Schema constant duplicates those shapes.
2. **Typed tool dispatch.** A deterministic adapter (`contract` / `vectis`) is invoked through generated bindings — no argv packing or stdout-JSON parsing on that path.
3. **No drift surface.** The `*_JSON_SCHEMA` constants + parity test are retired or regenerated from WIT.
4. **Brief bodies unchanged.** Brief bodies remain markdown and execution stays a two-phase handoff; an agent operation's request and report cross as the same WIT records, validated at `finalize`.
5. **RFC-50 invariant intact.** The host still passes the no-adapter-names / no-taxonomy grep + guard test from RFC-50's acceptance criteria.
6. **Component on both axes.** Every source and target adapter ships a WASM component implementing its axis world, co-packaged with its prose and published as one composite extension (RFC-48); there are no prose-only adapters, and `specify:adapter` is owned and published by the `specify` repo (§D).

