# RFC-51: Typed Adapter ABI via a WIT / Component-Model World

> Status: Draft - Depends: RFC-47 (adapter identity), RFC-48 (adapter packaging/registry), RFC-49 (adapter extraction to `specify-adapters`), RFC-50 (adapter-agnostic core)

## Abstract

Adapters are invoked today through the generic `wasi:cli/run` world: the host packs **argv**, reads an **exit code**, and exchanges data as **stdout/stderr JSON** plus **preopened directories**. Operation semantics live in argv conventions and in JSON envelopes validated *at runtime* against embedded `*.schema.json` constants. This RFC proposes replacing that loose contract with a typed **WebAssembly Component Model** contract: one versioned **WIT package** defining every operation's request/report records, **per-axis worlds** that export the deterministic operations, and an **agent brief handoff that reuses the same WIT types** as its serialized envelope. The host calls deterministic adapters through generated, typed bindings; the schema-constant + parity-test machinery collapses into the WIT package as a single source of truth.

This is the *typed realization* of RFC-50's "uniform operation-envelope runtime": RFC-50 says the host's contract is a fixed envelope dispatched generically; this RFC says those envelopes are WIT records and the deterministic operations are WIT exports.

**Explicit non-goal up front:** adapter **briefs do not become WIT functions.** They are markdown executed by an LLM; there is no component to bind. WIT types the data crossing the boundary and makes the *deterministic* operations callable — the agent execution stays a two-phase handoff.

## Motivation

RFC-50 routes every adapter behavior through one generic operation-envelope runtime and forbids adapter-specific code in the host. The contract those envelopes ride on is still untyped, and that creates four standing costs:

- **Schema/code drift.** The embedded `*_JSON_SCHEMA` constants in `engine/crates/schema/src/constants.rs` exist alongside a dedicated byte-parity test (`engine/crates/schema/tests/schemas.rs`) and wire fixtures precisely to police drift between hand-maintained schemas and the DTOs that round-trip them. A typed boundary removes the drift surface instead of policing it.
- **Untyped invocation.** The `wasi:cli/run` path means the operation contract is "argv conventions + parse stdout as JSON." Mistakes surface at runtime as validation errors, not at the binding boundary.
- **Broad capability grants.** Adapters receive filesystem access via preopened directories and the `$CAPABILITY_DIR` env var rather than a *named* set of host capabilities. The world cannot tell, by inspection, what an adapter is allowed to touch.
- **Convention-based errors.** Failures cross as exit codes + stderr text rather than a typed `result<_, error>`.

The Component Model is the native idiom here, not a new dependency: **WASI Preview 2 is itself defined in WIT**, and the host already compiles with the `component-model` feature (see [Starting state](#starting-state)). The interface-definition layer is already present; this RFC uses it for the adapter contract rather than only for WASI.

## Starting state

- **Runtime.** `wasmtime` and `wasmtime-wasi` are pinned at `45.0.0` with the `component-model`, `cranelift`, `runtime`, `cache`, and (for `wasmtime-wasi`) `p2` features enabled — `engine/Cargo.toml:201`. The Component Model machinery (records, variants, `result`, resources) is therefore already linked into the host binary.
- **Invocation ABI.** `engine/crates/registry/src/host.rs` instantiates each adapter `.wasm` as the prebuilt `wasi:cli/command` world (`Command::instantiate(...)`) and calls `command.wasi_cli_run().call_run(...)`. Arguments are passed via `WasiCtxBuilder::args` (argv[0] = tool name); data crosses via captured stdout/stderr and `preopened_dir` grants plus `$CAPABILITY_DIR`.
- **Operation set.** `SourceOperation` ∈ `{ survey, extract }` and `TargetOperation` ∈ `{ shape, build, merge }` (`engine/crates/workflow/src/adapter/operation.rs`). Each operation is `execution: tool` (single-phase WASI dispatch) or `execution: agent` (two-phase brief handoff, the default). No first-party `build`/`merge` *tool* is wired today — every first-party target is `execution: agent`; source `survey`/`extract` are agent-only.
- **Envelopes.** Request/report shapes are embedded JSON-Schema constants in `crates/schema/src/constants.rs` (`BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`, `EVIDENCE_JSON_SCHEMA`, `LEAD_JSON_SCHEMA`, …), byte-parity-tested against the on-disk `schemas/` tree.

## Scope

**In scope:** the host↔adapter wire ABI for deterministic operations, and the typed envelope shapes for *every* operation on *both* axes.

### Non-goals

- **Briefs stay markdown.** The agent path is not a WIT export and is not made callable. This RFC types the *envelopes* the briefs exchange; it does not touch the prompts.
- **No workflow or operation-set change.** The lifecycle, the `survey/extract/shape/build/merge` operation set, and adapter identity (RFC-47) are unchanged.
- **Source axis is types-only today.** Both source operations are agent-only, so the source world exports nothing callable; sources gain typed envelopes but no callable exports until a deterministic source tool exists.
- **No compatibility shim.** Consistent with the project's pre-1.0 "hard cut" posture, the `wasi:cli/run` → world migration is a clean ABI cut at `specify extension run`, not a dual-path bridge.

## The model

### A. One shared WIT package — the envelope currency

A single versioned package, `specify:adapter@<semver>`, defines an `interface types` carrying every operation's request/report records for both axes (build, merge, shape, survey, extract, evidence), plus the shared `finding` / `adapter-error` shapes. This package becomes the single source of truth for those shapes; the `*_JSON_SCHEMA` constants are generated from it (or retired in favour of it), eliminating the drift the parity test currently guards.

### B. Per-axis worlds export only the deterministic operations

```wit
package specify:adapter@1.0.0;

// Shared envelope currency — used by BOTH the tool path and the agent handoff.
interface types {
  enum severity { critical, important, suggestion, optional }
  record finding { rule-id: string, severity: severity, detail: string }

  record build-request { slice: string, project-root: string, inputs: list<tuple<string, string>> }
  enum build-status { success, failure }
  record build-output { platform: string, path: string }
  record build-report {
    slice: string,
    status: build-status,
    findings: list<finding>,
    outputs: list<build-output>,
  }
  // merge-request / merge-report, shape-*, and the source-side
  // survey / extract / evidence records live here too.

  variant adapter-error { invalid-request(string), io(string), internal(string) }
}

// Deterministic operations — exported ONLY by `execution: tool` components.
interface target-ops {
  use types.{ build-request, build-report, merge-request, merge-report, adapter-error };
  build: func(req: build-request) -> result<build-report, adapter-error>;
  merge: func(req: merge-request) -> result<merge-report, adapter-error>;
}

world target-tool {
  import wasi:filesystem/types@0.2.0;   // capability-scoped host access
  import host-config;                   // small host interface: read project.yaml, etc.
  export target-ops;
}
```

A deterministic target exports `target-ops`; the host calls `instance.call_build(&mut store, &req)` and gets a typed `result<build-report, adapter-error>`. A pure-agent target exports *nothing callable* — see (C). The `source` world is analogous, but with no callable operations today (survey/extract are agent-only), so it reduces to the shared `types`.

### C. The agent handoff reuses the WIT types

For `execution: agent`, there is no export to call. The host serializes the WIT `build-request` record into the two-phase brief handoff, the agent runs the brief, and the host parses a `build-report` back. Host-side Rust types come from `wasmtime::component::bindgen!`; the JSON the brief reads and writes is a projection of the *same* records, so there is one definition for both the typed (tool) path and the serialized (agent) path.

### D. Capabilities become named imports / resources

A world's `import`s name exactly the host capabilities the adapter may use, replacing the broad `$CAPABILITY_DIR` + preopened-directory grant. A `resource slice` / `resource project` can expose typed host methods (`read-artifact: func(path: string) -> result<string, adapter-error>`, `get-asset: func(id: string) -> option<asset>`) so the adapter reads through narrow host calls instead of walking a preopened tree. This narrows the blast radius and makes the data flow explicit.

### E. Versioning

The WIT package is semver-versioned and ties into RFC-47 adapter identity and the `requires_specify` floor: the host advertises the world version(s) it supports, an adapter targets a world version, and a mismatch is a typed resolve error rather than a runtime surprise.

## The hard boundary (non-goal, restated precisely)

A brief is a prompt executed by the LLM; **no WASM component runs it**, so WIT has nothing to bind there. The contract is therefore hybrid by necessity:

| Operation execution | WIT role |
| --- | --- |
| `execution: tool` (deterministic WASM) | Callable world export (`build`/`merge`/validators) + typed envelope |
| `execution: agent` (markdown brief) | Typed envelope only; execution stays a two-phase host handoff |

The dominant mode today is `agent`, so most operations gain *typed envelopes* but not *callable exports*. That is the honest ceiling of this RFC, and it is a feature: it is exactly what lets the host stay agent-runtime-agnostic.

## Phased plan

Each phase is independently mergeable and **must keep `make lint` and `cargo make ci` green**. The ABI break is contained at the single `specify extension run` chokepoint.

### Phase 0 — Author the WIT package + host bindings (no behavior change)

Define `specify:adapter` with the `types` interface and the per-axis worlds; wire `wasmtime::component::bindgen!` host-side. Assert the generated types match the current envelope records (transitional parity), so nothing changes at runtime yet.

### Phase 1 — Typed exports for the existing tool components

Re-export a world from the deterministic components (`contract`, `vectis`) instead of `wasi:cli/run`, and route the `execution: tool` dispatch through the generated bindings. Lowest-risk first step: these are real callable exports that exist today behind the argv contract.

### Phase 2 — Type the agent envelopes

Project the WIT `types` records into the survey/extract/shape/build/merge handoffs the host already drives; retire or regenerate the `*_JSON_SCHEMA` constants + parity test from the WIT package so there is a single source of truth.

### Phase 3 — Capability / resource model

Replace `$CAPABILITY_DIR` + broad preopens with named world `import`s and `resource` handles. Gate this on confirming the pinned wasmtime's resource ergonomics (see Risks).

## Decisions to record (open until reviewed)

- **WIT package as schema source of truth** — the fate of the `*_JSON_SCHEMA` constants, the byte-parity test, and the wire fixtures once shapes are generated from WIT.
- **`wasi:cli/run` → custom world migration** — a breaking extension ABI cut; confirm it stays contained to `specify extension run` and the `specify-adapters` `.wasm` build.
- **Capability model** — named `import`s / `resource`s vs. the current preopen grant; which host functions the world exposes.
- **Agent-handoff serialization** — the JSON projection of the WIT records used by the brief handoff.
- **Versioning** — how the world version relates to RFC-47 identity and `requires_specify`.
- **`shape` semantics** — whether `shape` is a world export, a host-read manifest-declared file, or an envelope.
- **Operation set vs declared tools** — whether the manifest's declared-tool set (`contract`, `vectis`) and the operation set unify under one world.

## Risks and invariants

- **Agent path unchanged.** Most operations remain a handoff; manage expectations that this is "type the envelopes + make the deterministic tools callable," not "turn briefs into functions."
- **Toolchain.** Components + `wit-bindgen` add build steps for adapter authors. Language-agnostic implementation is a benefit, but every adapter author needs a component toolchain (already true for the validators).
- **wasmtime feature maturity.** v45 ships stable Component Model support (records, variants, `result`, resources); confirm async-export and cross-boundary `resource` ergonomics before relying on them in Phase 3.
- **Cross-repo seam.** The adapter `.wasm` builds live in `specify-adapters`; the world re-export and the ABI cut must land in lockstep across both repos (workflow contract spans both).
- **RFC-50 invariant preserved.** The WIT package is generic — it carries no adapter *name* and no adapter *taxonomy*. The host still holds zero adapter-specific code; this RFC types the contract, it does not re-open the host to any adapter.

## Acceptance criteria

1. **Single typed contract.** One `specify:adapter` WIT package defines every operation's request/report on both axes; no hand-rolled DTO or embedded JSON-Schema constant duplicates those shapes.
2. **Typed tool dispatch.** A deterministic adapter (`contract` / `vectis`) is invoked through generated bindings — no argv packing or stdout-JSON parsing on that path.
3. **No drift surface.** The `*_JSON_SCHEMA` constants + parity test are retired or regenerated from WIT.
4. **Named capabilities.** Typed-path adapters declare their host access in the world; no blanket `$CAPABILITY_DIR` grant for them.
5. **Briefs unchanged.** Briefs remain markdown; the agent handoff exchanges WIT-typed envelopes.
6. **RFC-50 invariant intact.** The host still passes the no-adapter-names / no-taxonomy grep + guard test from RFC-50's acceptance criteria.
