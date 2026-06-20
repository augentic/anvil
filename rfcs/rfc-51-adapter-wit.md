# RFC-51: Adapter WIT — the typed contract package

> Status: Draft · Depends: RFC-47 (adapter identity), RFC-48 (adapter packaging/registry), RFC-49 (adapter extraction to `specify-adapters`), RFC-50 (adapter-agnostic core) · Framed by: [the effect-oriented architecture](architecture.md) as the Stage 0–1 foundation · Hands off: capabilities + agent-envelope typing → [RFC-52](rfc-52-effect.md), typed tool dispatch + brief-typing → [RFC-53](rfc-53-orchestration.md), the component-on-both-axes mandate → [RFC-55](rfc-55-runtime-move.md)

## Position in the architecture

This RFC is the **foundation** of the [effect-oriented architecture](architecture.md): the one versioned WIT package — the stratum-1 transfer records, the per-axis interface and world signatures, and the host bindings — that every later stage imports. It is deliberately scoped to *authoring and publishing that contract*; it is behaviour-neutral by construction. The work that **consumes** the contract — and therefore changes a runtime path — is sequenced into the stages that own it:

- **Capabilities / resources** (the narrow host-data accessors) → [RFC-52](rfc-52-effect.md), which generalizes them from a raw grant into named **data effects**.
- **Typed agent envelopes + retiring the `*_JSON_SCHEMA` constants** → [RFC-52](rfc-52-effect.md): projecting the records into the prepare/finalize handoff and collapsing the byte-parity machinery into the package is the same work as naming the `judge` effect over those records.
- **Typed tool dispatch** (routing the deterministic `tool` operations through the generated bindings, retiring `wasi:cli/run` on that path) → [RFC-53](rfc-53-orchestration.md), where the component's world exports first become callable.
- **The typed brief contract and lazy discovery** → [RFC-53](rfc-53-orchestration.md): once a brief is the body of the `judge` effect, the signature is declared at the call-site rather than re-stated in frontmatter, so the heavier `implements` / `consumes` / `produces` / `capabilities` machinery is re-evaluated there.
- **The component-on-both-axes mandate** → [RFC-55](rfc-55-runtime-move.md): requiring every adapter — including the agent-only sources — to ship a WASM component only bites once guests are instantiated generically by the runtime move.

What remains in RFC-51 is durable and behaviour-neutral: author the `specify:adapter` package (records + interface/world signatures), wire the host bindings, assert parity with today's envelopes, and publish it as the single source of truth.

## Abstract

Adapters are invoked today through the generic `wasi:cli/run` world: the host packs **argv**, reads an **exit code**, and exchanges data as **stdout/stderr JSON** plus **preopened directories**. Operation semantics live in argv conventions and in JSON envelopes validated *at runtime* against embedded `*.schema.json` constants.

This RFC authors the typed contract that replaces that loose surface: one versioned **WebAssembly Component Model** package, `specify:adapter@<semver>`, defining every operation's request/report records, the per-axis `target` / `source` interface signatures, and the worlds that export the deterministic operations — wired host-side through `wasmtime::component::bindgen!` and published as a pinned dependency. It is the *typed realization* of RFC-50's "uniform operation-envelope runtime": RFC-50 says the host's contract is a fixed envelope dispatched generically; this RFC says those envelopes are WIT records and the deterministic operations are WIT exports.

The package is the **single source of truth** the rest of the architecture imports. Its records are designed as **shared envelope currency**: the same `build-request` / `build-report` shapes serve the deterministic tool path (a callable world export) and the agent handoff (an LLM running a brief, exchanging the records as serialized JSON). RFC-51 *defines* that currency; the work that consumes it — making tool dispatch callable, typing the handoff and retiring the schema constants, and mandating a component on both axes — is staged into RFC-52/53/55 (see [Position in the architecture](#position-in-the-architecture)).

**Explicit non-goal up front:** adapter **briefs do not become callable WIT functions.** They are markdown executed by an LLM; there is no component to instantiate and no export to invoke. WIT types the data crossing the boundary and makes the *deterministic* operations callable — but the agent execution stays a two-phase handoff and the prose body is never machine-executed.

## Motivation

RFC-50 routes every adapter behavior through one generic operation-envelope runtime and forbids adapter-specific code in the host. The contract those envelopes ride on is still untyped, and that creates four standing costs that the typed package is the precondition for removing — each removed in the stage that owns the consuming work:

- **Schema/code drift.** The embedded `*_JSON_SCHEMA` constants in `engine/crates/schema/src/constants.rs` need a dedicated byte-parity test (`engine/crates/schema/tests/schemas.rs`) and wire fixtures to police drift between hand-maintained schemas and the DTOs that round-trip them. A WIT package gives one generated source of truth; the actual retirement of the constants is [RFC-52](rfc-52-effect.md)'s, but it is only possible once this package exists.
- **Untyped invocation.** The `wasi:cli/run` path means the operation contract is "argv conventions + parse stdout as JSON." Routing `tool` dispatch through generated bindings ([RFC-53](rfc-53-orchestration.md)) is what removes it; this RFC authors the bindings it rides on.
- **Broad capability grants.** Adapters receive filesystem access via preopened directories and the `$CAPABILITY_DIR` env var rather than a *named* set of host capabilities; the named host-data effects that narrow this are [RFC-52](rfc-52-effect.md)'s.
- **Convention-based errors.** Failures cross as exit codes + stderr text rather than a typed `result<_, adapter-error>` — the variant this package defines.

The Component Model is the native idiom here, not a new dependency: **WASI Preview 2 is itself defined in WIT**, and the host already compiles with the `component-model` feature (see [Starting state](#starting-state)). The interface-definition layer is already present; this RFC uses it for the adapter contract rather than only for WASI.

## Starting state

- **Runtime.** `wasmtime` and `wasmtime-wasi` are pinned at `45.0.0` with the `component-model`, `cranelift`, `runtime`, `cache`, and (for `wasmtime-wasi`) `p2` features enabled (`engine/Cargo.toml`). The Component Model machinery (records, variants, `result`, resources) is therefore already linked into the host binary.
- **Invocation ABI.** `engine/crates/registry/src/host.rs` instantiates each adapter `.wasm` as the prebuilt `wasi:cli/command` world (`Command::instantiate(...)`) and calls `command.wasi_cli_run().call_run(...)`. Arguments are passed via `WasiCtxBuilder::args` (argv[0] = tool name); data crosses via captured stdout/stderr and `preopened_dir` grants plus `$CAPABILITY_DIR`.
- **Operation set.** `SourceOperation` ∈ `{ survey, extract }` and `TargetOperation` ∈ `{ shape, build, merge }` (`engine/crates/workflow/src/adapter/operation.rs`). Each operation is `execution: tool` (single-phase WASI dispatch) or `execution: agent` (two-phase brief handoff, the default). No first-party `build`/`merge` *tool* is wired today — every first-party target is `execution: agent`; source `survey`/`extract` are agent-only.
- **Envelopes.** Request/report shapes are embedded JSON-Schema constants in `crates/schema/src/constants.rs` (`BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`, `EVIDENCE_JSON_SCHEMA`, `LEAD_JSON_SCHEMA`, …), byte-parity-tested against the on-disk `schemas/` tree.

## Scope

**In scope:** authoring the `specify:adapter` WIT package — the `types` records for every operation on both axes, the `target` / `source` interface signatures, and the per-axis worlds; wiring the host-side `wasmtime::component::bindgen!` bindings; asserting transitional parity against today's envelope records; and publishing the package as a pinned, versioned dependency (§D).

### Non-goals

- **No behaviour change.** This RFC lands the contract and asserts it matches the current envelopes; it changes no runtime path. Every consumer is staged downstream.
- **Tool dispatch is not rerouted here.** Routing `execution: tool` (`contract`, `vectis`) through the generated bindings and retiring `wasi:cli/run` on that path is [RFC-53](rfc-53-orchestration.md).
- **Agent envelopes are not typed here, and the schema constants are not retired here.** Projecting the records into the prepare/finalize handoff and collapsing the `*_JSON_SCHEMA` constants + parity test into the package is [RFC-52](rfc-52-effect.md).
- **The component-on-both-axes mandate is not imposed here.** Requiring every adapter — including the agent-only sources — to ship a component is [RFC-55](rfc-55-runtime-move.md), where guests are instantiated generically.
- **Brief bodies stay markdown; brief execution stays an LLM handoff.** The agent path is not a WIT export and is not made callable. The deeper brief-to-signature contract is [RFC-53](rfc-53-orchestration.md); correctness of the instructions themselves remains the eval / review layer's job (see [Risks and invariants](#risks-and-invariants)).
- **No workflow or operation-set change.** The lifecycle, the `survey/extract/shape/build/merge` operation set, and adapter identity (RFC-47) are unchanged.
- **No compatibility shim.** Consistent with the project's pre-1.0 "hard cut" posture, the eventual `wasi:cli/run` → world migration ([RFC-53](rfc-53-orchestration.md)) is a clean ABI cut at `specify extension run`, not a dual-path bridge.

## The model

### A. One shared WIT package — the envelope currency

A single versioned package, `specify:adapter@<semver>`, defines an `interface types` carrying every operation's request/report records for both axes (build, merge, shape, survey, extract, evidence), plus the shared `finding` / `adapter-error` shapes. This package becomes the single source of truth for those shapes; the `*_JSON_SCHEMA` constants are generated from it (or retired in favour of it) in [RFC-52](rfc-52-effect.md), eliminating the drift the parity test currently guards.

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

RFC-51 authors these *signatures* — the records, the per-axis interfaces, and the worlds — and asserts the generated Rust types match the current envelopes. The host *dispatch* that actually calls `instance.call_build(&mut store, &req)` and retires the argv path is [RFC-53](rfc-53-orchestration.md); until then the deterministic operations keep running behind the existing `wasi:cli/run` contract while exchanging the same records. An operation a target fulfils with a brief is *not* exported by any world — its request and report still cross as these records, but via the agent handoff (§C). The `source` world is analogous; both source operations are agent-only today, so the source world exports nothing callable.

### C. The records are the agent handoff's envelope currency too

For `execution: agent` there is no export to call, but the contract still governs the data: the same WIT `build-request` / `build-report` records are designed to be the prepare/finalize handoff's serialized envelope, so there is one definition for both the typed (tool) path and the serialized (agent) path. Host-side Rust types come from `wasmtime::component::bindgen!`; the JSON the brief reads and writes is a projection of the *same* records. RFC-51 establishes that currency and the bindings behind it; *typing the live handoff against it* — and retiring the `*_JSON_SCHEMA` constants once it is the single source of truth — is [RFC-52](rfc-52-effect.md), because that work is the same as naming the `judge` effect over these records.

### D. Versioning, ownership & publishing

The `specify` repo **owns and publishes** the `specify:adapter@<semver>` WIT package: it is authored under `specify/wit/`, published to the registry from specify CI (`wkg publish`), and consumed by `specify-adapters` (and any third-party adapter) as a pinned dependency resolved through `wkg`. The dependency direction is strictly one-way — specify produces the contract, adapters consume it — so the package is the single upstream source for both repos' generated bindings.

The package is semver-versioned and ties into RFC-47 adapter identity and the `requires_specify` floor: the host advertises the world version(s) it supports, an adapter targets a world version, and a mismatch is a typed resolve error rather than a runtime surprise.

### E. Consumers of the contract → RFC-52/53/55

Earlier drafts continued here with the consuming work — typed tool dispatch, typed agent envelopes, the schema-constant retirement, the typed brief contract, and the component-on-both-axes mandate. Each is now **relocated to the stage that owns it** (see [Position in the architecture](#position-in-the-architecture)): tool dispatch and brief-typing to [RFC-53](rfc-53-orchestration.md), agent-envelope typing + constant retirement + the host-data capability model to [RFC-52](rfc-52-effect.md), and the component mandate to [RFC-55](rfc-55-runtime-move.md). RFC-51 keeps only the package, the bindings, and the publish.

## The hard boundary (non-goal)

A brief is a prompt executed by the LLM; **no WASM component runs the prose**, so there is no export to instantiate or invoke. The contract is therefore hybrid by design, not by accident:

| Operation execution                    | WIT role                                                                                                                                                                                                                              |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `execution: tool` (deterministic WASM) | Callable world export (`build`/`merge`/validators) + typed envelope; the callable dispatch lands in [RFC-53](rfc-53-orchestration.md)                                                                                                  |
| `execution: agent` (markdown brief)    | Typed envelope only — request in, report out; the live handoff is typed against the records in [RFC-52](rfc-52-effect.md). Execution stays a two-phase host handoff and the prose body is never machine-executed. Deeper brief-to-signature binding is [RFC-53](rfc-53-orchestration.md). |

The dominant mode today is `agent`, so most operations will gain *typed envelopes* but not *callable exports*. The honest ceiling is that the contract types the data crossing the seam, not the *semantics* of the instructions: an LLM can follow a perfectly-typed brief and still emit a structurally-valid-but-wrong report. Typing raises the floor — no orphan operations, structurally-valid I/O — while correctness stays with the eval / review layer. That ceiling is a feature: it is exactly what lets the host stay agent-runtime-agnostic.

## Phased plan

This RFC is a single behaviour-neutral phase; the consuming phases live in their owning RFCs. It **must keep `make lint` and `cargo make ci` green**.

### Phase 0 — Author the WIT package + host bindings + publish (no behavior change)

Define `specify:adapter` with the `types` interface, the per-axis `target` / `source` interfaces, and the worlds; wire `wasmtime::component::bindgen!` host-side. Assert the generated types match the current envelope records (transitional parity), so nothing changes at runtime yet. Publish the package from specify CI (`wkg publish`) so `specify-adapters` can resolve a pinned version (§D) — this establishes specify as the owner/publisher before any adapter consumes the contract.

### Beyond Phase 0 — the consuming stages

Each consuming track builds on the records and bindings this RFC lands, in the stage that owns it:

- **Typed agent envelopes + `*_JSON_SCHEMA` retirement, and the named host-data capability model** → [RFC-52](rfc-52-effect.md).
- **Typed tool dispatch (`wasi:cli/run` retired on the tool path) + the typed brief contract** → [RFC-53](rfc-53-orchestration.md).
- **The component-on-both-axes mandate** → [RFC-55](rfc-55-runtime-move.md).

## Decisions to record (open until reviewed)

- **WIT package as schema source of truth** — that the records are authored once here and the `*_JSON_SCHEMA` constants are generated from / retired against them (the retirement itself lands in [RFC-52](rfc-52-effect.md)).
- **Versioning, ownership & publishing** — that the `specify` repo owns and publishes `specify:adapter@<semver>` (`wkg publish`) while `specify-adapters` consumes a pinned version (one-way dependency); and how the world version relates to RFC-47 identity and `requires_specify`.
- **`shape` semantics** — whether `shape` is a world export, a host-read manifest-declared file, or an envelope.
- **Operation set vs declared tools** — whether the manifest's declared-tool set (`contract`, `vectis`) and the operation set unify under one world.
- **Capability model** — deferred to [RFC-52](rfc-52-effect.md): named host-data effects vs. the current preopen grant, and which host functions the world exposes.
- **Brief-typing surface** — deferred to [RFC-53](rfc-53-orchestration.md): how much of the `implements` / `consumes` / `produces` / `capabilities` brief contract, its coverage check, and the reference catalog survives once a brief is the body of the `judge` effect.

## Risks and invariants

- **Behaviour-neutral by construction.** RFC-51 lands a contract and asserts parity; if it changes a runtime path it has overreached. Every behaviour change is a downstream consumer's.
- **Cross-repo seam (one-directional).** The `specify` repo owns and publishes the `specify:adapter` WIT package; the adapter `.wasm` builds live in `specify-adapters` and consume a pinned published version. The ABI cut is a publish-then-pin sequence — specify ships the world version, `specify-adapters` bumps its pin and re-exports — rather than a symmetric lockstep edit, so the version bump and the consuming build must be sequenced deliberately.
- **Toolchain cost lands later.** Components + `wit-bindgen` add build steps for adapter authors, and the requirement that *every* adapter ship a component — including the agent-only source adapters (`intent`, `documentation`, `typescript`, `screenshots`, `captures`) — is the principal adoption cost. That mandate is imposed in [RFC-55](rfc-55-runtime-move.md), where guests are instantiated generically; this RFC only authors the world they will implement.
- **wasmtime feature maturity.** v45 ships stable Component Model support for the records, variants, and `result` this RFC uses; the `resource` ergonomics that the named host-data effects need are confirmed in [RFC-52](rfc-52-effect.md) before that stage relies on them.
- **RFC-50 invariant preserved.** The WIT package is generic — it carries no adapter *name* and no adapter *taxonomy*. The host still holds zero adapter-specific code; this RFC types the contract, it does not re-open the host to any adapter.

## Acceptance criteria

1. **Single typed contract.** One `specify:adapter` WIT package defines every operation's request/report records and the per-axis interface/world signatures on both axes; it is the authoritative shape no hand-rolled DTO duplicates.
2. **Host bindings + parity.** `wasmtime::component::bindgen!` is wired host-side and the generated types are asserted equal to today's envelope records; no runtime path changes (behaviour-neutral).
3. **Published + pinned.** The package is published from specify CI (`wkg publish`) and resolvable as a pinned dependency by `specify-adapters`; the world version ties into RFC-47 identity and `requires_specify`.
4. **RFC-50 invariant intact.** The host still passes the no-adapter-names / no-taxonomy grep + guard test from RFC-50's acceptance criteria; the package carries no adapter name or taxonomy.
5. **Consumers are sequenced, not duplicated.** Typed tool dispatch ([RFC-53](rfc-53-orchestration.md)), agent-envelope typing + `*_JSON_SCHEMA` retirement ([RFC-52](rfc-52-effect.md)), and the component-on-both-axes mandate ([RFC-55](rfc-55-runtime-move.md)) each reference this package as their single source of truth.
