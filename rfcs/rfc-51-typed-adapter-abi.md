# RFC-51: Typed Adapter ABI via a WIT / Component-Model World

> Status: Draft - Depends: RFC-47 (adapter identity), RFC-48 (adapter packaging/registry), RFC-49 (adapter extraction to `specify-adapters`), RFC-50 (adapter-agnostic core)

## Abstract

Adapters are invoked today through the generic `wasi:cli/run` world: the host packs **argv**, reads an **exit code**, and exchanges data as **stdout/stderr JSON** plus **preopened directories**. Operation semantics live in argv conventions and in JSON envelopes validated *at runtime* against embedded `*.schema.json` constants.

This RFC proposes replacing that loose contract with a typed **WebAssembly Component Model** contract: one versioned **WIT package** defining every operation's request/report records, **per-axis worlds** that export the deterministic operations, and an **agent brief handoff that reuses the same WIT types** as its serialized envelope. The host calls deterministic adapters through generated, typed bindings; the schema-constant + parity-test machinery collapses into the WIT package as a single source of truth.

This is the *typed realization* of RFC-50's "uniform operation-envelope runtime": RFC-50 says the host's contract is a fixed envelope dispatched generically; this RFC says those envelopes are WIT records and the deterministic operations are WIT exports.

The contract reaches the prose as well. A **brief is the agent-side implementation of a WIT operation**: a `tool`-executed operation is implemented by a WASM component (a callable export), and an `agent`-executed operation is implemented by a markdown brief (an LLM-fulfilled "export"). Both satisfy the *same* WIT signature from `specify:adapter`; only the executor differs. A brief's **contract** — the signature it fulfils, the request fields its prose references, the report it returns, the capabilities it touches, and the example payloads it embeds — therefore binds to and is checked against the same WIT world, even though the brief *body* stays prose and its *execution* stays an LLM handoff.

**Explicit non-goal up front:** adapter **briefs do not become callable WIT functions.** They are markdown executed by an LLM; there is no component to instantiate and no export to invoke. WIT types the data crossing the boundary, makes the *deterministic* operations callable, and binds the *brief contract* to the world for static checking — but the agent execution stays a two-phase handoff and the prose body is never machine-executed.

## Motivation

RFC-50 routes every adapter behavior through one generic operation-envelope runtime and forbids adapter-specific code in the host. The contract those envelopes ride on is still untyped, and that creates five standing costs:

- **Schema/code drift.** The embedded `*_JSON_SCHEMA` constants in `engine/crates/schema/src/constants.rs` need a dedicated byte-parity test (`engine/crates/schema/tests/schemas.rs`) and wire fixtures to police drift between hand-maintained schemas and the DTOs that round-trip them. A typed boundary removes the drift surface instead of policing it.
- **Untyped invocation.** The `wasi:cli/run` path means the operation contract is "argv conventions + parse stdout as JSON." Mistakes surface at runtime as validation errors, not at the binding boundary.
- **Broad capability grants.** Adapters receive filesystem access via preopened directories and the `$CAPABILITY_DIR` env var rather than a *named* set of host capabilities. The world cannot tell, by inspection, what an adapter is allowed to touch.
- **Convention-based errors.** Failures cross as exit codes + stderr text rather than a typed `result<_, error>`.
- **Untyped brief contracts.** A brief's inputs (`$SLICE_NAME`, `inputs.artifacts.`*, `<lead>`), its output shape, and the capabilities it reads are expressed only as prose convention. Nothing links "this brief produces an `Evidence` document" to the schema that document is validated against, so a brief's placeholder references, embedded examples, and capability use can drift from the operation contract with nothing to catch it until runtime.

The Component Model is the native idiom here, not a new dependency: **WASI Preview 2 is itself defined in WIT**, and the host already compiles with the `component-model` feature (see [Starting state](#starting-state)). The interface-definition layer is already present; this RFC uses it for the adapter contract rather than only for WASI.

## Starting state

- **Runtime.** `wasmtime` and `wasmtime-wasi` are pinned at `45.0.0` with the `component-model`, `cranelift`, `runtime`, `cache`, and (for `wasmtime-wasi`) `p2` features enabled (`engine/Cargo.toml`). The Component Model machinery (records, variants, `result`, resources) is therefore already linked into the host binary.
- **Invocation ABI.** `engine/crates/registry/src/host.rs` instantiates each adapter `.wasm` as the prebuilt `wasi:cli/command` world (`Command::instantiate(...)`) and calls `command.wasi_cli_run().call_run(...)`. Arguments are passed via `WasiCtxBuilder::args` (argv[0] = tool name); data crosses via captured stdout/stderr and `preopened_dir` grants plus `$CAPABILITY_DIR`.
- **Operation set.** `SourceOperation` ∈ `{ survey, extract }` and `TargetOperation` ∈ `{ shape, build, merge }` (`engine/crates/workflow/src/adapter/operation.rs`). Each operation is `execution: tool` (single-phase WASI dispatch) or `execution: agent` (two-phase brief handoff, the default). No first-party `build`/`merge` *tool* is wired today — every first-party target is `execution: agent`; source `survey`/`extract` are agent-only.
- **Envelopes.** Request/report shapes are embedded JSON-Schema constants in `crates/schema/src/constants.rs` (`BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`, `EVIDENCE_JSON_SCHEMA`, `LEAD_JSON_SCHEMA`, …), byte-parity-tested against the on-disk `schemas/` tree.

## Scope

**In scope:** the host↔adapter wire ABI for deterministic operations; the typed envelope shapes for *every* operation on *both* axes; and the **typed brief contract** — binding each agent-executed brief to the WIT signature it fulfils, with its inputs, outputs, capabilities, and embedded examples checked against the world.

### Non-goals

- **Brief bodies stay markdown; brief execution stays an LLM handoff.** The agent path is not a WIT export and is not made callable. This RFC types the *envelopes* the briefs exchange and the *contract* each brief declares (signature, inputs, outputs, capabilities, examples); it does not type, generate, or machine-execute the prose body. Correctness of the instructions themselves remains the eval / review layer's job (see [Risks and invariants](#risks-and-invariants)).
- **No workflow or operation-set change.** The lifecycle, the `survey/extract/shape/build/merge` operation set, and adapter identity (RFC-47) are unchanged.
- **Source axis is types-only today.** Both source operations are agent-only, so the source world exports nothing callable; sources gain typed envelopes but no callable exports until a deterministic source tool exists.
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

// Every operation's signature — the shared contract a `tool` component
// EXPORTS and an `agent` brief BINDS TO (§F). The signature is identical
// either way; only the executor differs.
interface target-ops {
  use types.{ build-request, build-report, merge-request, merge-report, adapter-error };
  build: func(req: build-request) -> result<build-report, adapter-error>;
  merge: func(req: merge-request) -> result<merge-report, adapter-error>;
}

world target-tool {
  import wasi:filesystem/types@0.2.0;   // capability-scoped host access
  import host-config;                   // small host interface: read project.yaml, etc.
  export target-ops;                    // only the operations this component implements
}
```

A deterministic target `world` exports the subset of `target-ops` it implements; the host calls `instance.call_build(&mut store, &req)` and gets a typed `result<build-report, adapter-error>`. An operation a target fulfils with a brief is *not* exported by any world — it is bound by that brief's frontmatter and checked against the `target-ops` signature at authoring time (§F). The interface is therefore the union of operation signatures; `{world exports} ∪ {brief bindings}` must cover it exactly once. The `source` world is analogous; both source operations are agent-only today, so the source world exports nothing and `survey` / `extract` are brief-bound.

### C. The agent handoff reuses the WIT types

For `execution: agent`, there is no export to call. The host serializes the WIT `build-request` record into the two-phase brief handoff alongside the `implements:` contract id the brief declares (§F1), the agent runs the brief, and the host parses a `build-report` back and validates it at `finalize` against the operation's WIT-derived report type. Host-side Rust types come from `wasmtime::component::bindgen!`; the JSON the brief reads and writes is a projection of the *same* records, so there is one definition for both the typed (tool) path and the serialized (agent) path. Structurally the handoff becomes a typed call — "here is your `build-request` value, here is the signature you fulfil, return a `build-report`" — with an LLM rather than wasmtime as the interpreter and the `finalize` validation as the return-type check.

### D. Capabilities become named imports / resources

A world's `import`s name exactly the host capabilities the adapter may use, replacing the broad `$CAPABILITY_DIR` + preopened-directory grant. A `resource slice` / `resource project` can expose typed host methods (`read-artifact: func(path: string) -> result<string, adapter-error>`, `get-asset: func(id: string) -> option<asset>`) so the adapter reads through narrow host calls instead of walking a preopened tree. This narrows the blast radius and makes the data flow explicit.

The capability model targets **host/project data** — the slice tree, artifacts, and assets that flow *into* the operation. It deliberately does **not** govern an adapter reading its *own* bundled prose (briefs, phase sub-briefs, references): that corpus ships with the adapter and is read by whoever executes the brief, so there is nothing to narrow. Keeping these two access kinds distinct is what lets reference discovery stay lazy and open within the adapter bundle while host data stays narrowly typed (§G).

### E. Versioning

The WIT package is semver-versioned and ties into RFC-47 adapter identity and the `requires_specify` floor: the host advertises the world version(s) it supports, an adapter targets a world version, and a mismatch is a typed resolve error rather than a runtime surprise.

### F. Briefs as typed agent-fulfilled implementations

A brief fulfils a `target-ops` / `source-ops` signature the same way a component exports one. Binding the brief to that signature lets the brief's contract be checked against the world along four seams, none of which makes the prose callable:

```markdown
---
implements: specify:adapter/target-ops.build@1.0.0
consumes: build-request          # the request record this brief reads
produces: build-report           # the report record this brief returns
capabilities: [read-artifact, project-config]   # mirrors the world's imports
---

# Omnia target — build brief
...
```

**F1 — Signature binding.** `implements:` resolves against the WIT package at authoring time. A set-coverage check guarantees every agent-executed operation has exactly one binding brief and every brief binds a real operation — the `{world exports} ∪ {brief bindings}` coverage from §B. A brief for a nonexistent operation, or an operation with neither an export nor a binding brief, is a static failure rather than a runtime surprise.

**F2 — Typed input environment.** A brief's placeholders (`$SLICE_NAME`, `inputs.artifacts.proposal`, `inputs.artifacts.additional[]`, `<lead>`) are a projection of its `consumes` request record. The host builds a typed template environment from that record and flags any placeholder that resolves to neither a request-record field, a declared capability, nor a scratch lane. This is the closest the contract comes to type-checking the prose: it does not verify the instructions, but it verifies the *free variables* the instructions reference are real, typed fields of the request.

**F3 — Output contract and example validation.** The brief section that produces the operation's report is annotated with its `produces` type; its embedded fenced examples validate against the WIT-derived schema at authoring time, and the agent's actual output validates at `finalize` (§C). One WIT record is then enforced at three points — the brief's authoring example, the agent's runtime output, and the tool path's binding — so a brief's illustrative payloads can no longer drift from the shape the seam accepts.

**F4 — Capability binding.** A brief's `capabilities:` list mirrors the operation world's `import`s (§D, host-data only). A brief that references a host capability the world does not grant is a static failure. On the tool path this is sandbox-enforced; on the agent path the LLM can physically read more, so the declaration is an authoring-time + handoff-time contract checked by lint rather than sandbox-enforced — the same split this RFC draws everywhere between a component (enforced) and an LLM (declared + validated).

This rides existing machinery. Brief frontmatter is already parsed and stripped by the framework indexer, which already emits a typed `Brief` fact (`axis` / `adapter` / `operation` / `scope` / `sections`); the four seams are new declarative rules (`set-coverage`, `cross-reference`, `fenced-block`, `schema`, `field-grammar`, `presence`) over that fact, not a new engine. The schemas the example validation needs are the WIT-derived schemas Phase 2 already produces. The two-phase handoff envelope already points the agent at the request, the brief, and the report target; it gains the typed request value and the `implements` contract id (§C).

### G. Reference discovery stays lazy

The typed contract governs the **boundary** (request in, report out, host capabilities granted), not the **interior** navigation of the prose. Briefs are deliberately a lazily-discovered graph: a parent orchestrator brief links to phase sub-briefs and a "reference shelf" the agent loads on demand to stay within its context budget. Nothing in §F forces that graph to load eagerly, and nothing should.

- **Only the parent brief binds the signature.** Phase sub-briefs (`briefs/build/`**, `briefs/extract/**`) are internal decomposition of one operation's implementation, not separate operations; the indexer's `BriefScope::Parent` / `BriefScope::Phase` discriminant is where the binding stops. The agent still loads each phase at the marked step.
- **References are adapter-internal context, not boundary data.** They are read from the adapter's own bundle on demand (the agent path) or behind a coarse own-bundle grant (the tool path); §D's narrowing targets host/project data, so it does not constrain which references an operation may consult.
- **Typing adds integrity, not eagerness.** Lint can prove the discovery graph resolves (the existing `reference-resolves` / `links-registry` checks) so a dead reference link fails at authoring time rather than mid-run; the agent gains a *declared, bounded universe* it may reach without loading it; and the authoring-time checks stay incremental per file (placeholder and example checks apply to brief files and opt-in annotated blocks, never the whole reference corpus). Lazy discovery is preserved and is the point.

An optional further step makes discovery a structured *query* without making it eager: expose the reference shelf as a `resource references { list: func() -> list<reference-meta>; get: func(id: string) -> result<string, adapter-error>; }` on the tool path and a parallel `references:` frontmatter catalog on the agent path (same currency, two executors, per §C). Splitting cheap metadata from on-demand bodies keeps loading lazy by construction. This is polish, not load-bearing — the freeform shelf plus link resolution already yields a safe lazy graph.

## The hard boundary (non-goal)

A brief is a prompt executed by the LLM; **no WASM component runs the prose**, so there is no export to instantiate or invoke. But the brief is still the *implementation* of a WIT signature, so its contract binds to the world even though its body does not. The contract is therefore hybrid by design, not by accident:


| Operation execution                    | WIT role                                                                                                                                                                                                |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `execution: tool` (deterministic WASM) | Callable world export (`build`/`merge`/validators) + typed envelope                                                                                                                                     |
| `execution: agent` (markdown brief)    | Typed envelope + typed brief contract (signature binding, input environment, output examples, capabilities — §F); execution stays a two-phase host handoff and the prose body is never machine-executed |


The dominant mode today is `agent`, so most operations gain *typed envelopes and a typed contract* but not *callable exports*. The honest ceiling is that the contract types the data and the brief's shell, not the *semantics* of the instructions: an LLM can follow a perfectly-typed brief and still emit a structurally-valid-but-wrong report. Typing raises the floor — no orphan operations, no unresolved placeholders, no drifted examples, no ungranted capabilities, structurally-valid I/O — while correctness stays with the eval / review layer. That ceiling is a feature: it is exactly what lets the host stay agent-runtime-agnostic.

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

### Brief-typing phases (sequenced after Phase 2)

These bind the brief contract to the WIT world (§F–§G). Each is additive and authoring-time first; none changes the agent execution model.

#### Phase 4 — Bind briefs to the world

Add optional `implements` / `consumes` / `produces` frontmatter; resolve `implements` against the WIT package; add the §F1 set-coverage rule that agent-executed operations and binding briefs are one-to-one. Pure authoring-time; zero runtime change. Keep the frontmatter optional here so adoption across `specify-adapters` is incremental.

#### Phase 5 — Type the inputs and examples

Build the §F2 typed placeholder environment from `consumes` and flag unresolved placeholders; validate the §F3 embedded fenced examples against the WIT-derived schemas from Phase 2. Still authoring-time; the runtime `finalize` validation already exists.

#### Phase 6 — Bind capabilities and the typed handoff

Add the §F4 `capabilities` frontmatter checked against the operation world's `import`s (pairs with Phase 3), and carry the typed request value + `implements` contract id in the two-phase handoff envelope (§C) so the agent receives a fully-typed prompt.

#### Phase 7 — Skeleton / body split and conformance goldens (optional)

Graduate briefs from prose-with-frontmatter toward a structured skeleton (signature, I/O records, capabilities, required sections, per-section produced-type annotations) plus a prose body — the brief-side analog of the roadmap's parked "type-safe skill expression" idea. Add synthetic-request conformance goldens for the agent path (request → agent run → report validates), mirroring the tool path's bindings test.

## Decisions to record (open until reviewed)

- **WIT package as schema source of truth** — the fate of the `*_JSON_SCHEMA` constants, the byte-parity test, and the wire fixtures once shapes are generated from WIT.
- `**wasi:cli/run` → custom world migration** — a breaking extension ABI cut; confirm it stays contained to `specify extension run` and the `specify-adapters` `.wasm` build.
- **Capability model** — named `import`s / `resource`s vs. the current preopen grant; which host functions the world exposes.
- **Agent-handoff serialization** — the JSON projection of the WIT records used by the brief handoff.
- **Versioning** — how the world version relates to RFC-47 identity and `requires_specify`.
- `**shape` semantics** — whether `shape` is a world export, a host-read manifest-declared file, or an envelope.
- **Operation set vs declared tools** — whether the manifest's declared-tool set (`contract`, `vectis`) and the operation set unify under one world.
- **Brief frontmatter contract** — the `implements` / `consumes` / `produces` / `capabilities` frontmatter shape and its schema, and whether phase sub-briefs may carry their own narrower scoped annotations.
- **Coverage authority** — confirming the §F1 `{world exports} ∪ {brief bindings}` coverage is a `specify lint framework` authoring check with no lifecycle authority, consistent with the standards-vs-workflow split.
- **Reference catalog** — whether to ship the §G `resource references` / `references:` catalog now or leave the freeform link shelf plus `reference-resolves` as the v1 discovery surface.
- **Per-phase placeholder scoping** — whether §F2 placeholder typing scopes per phase sub-brief (each declaring the request subset it consumes) or only at the parent brief.

## Risks and invariants

- **Agent path unchanged.** Most operations remain a handoff; this is "type the envelopes, type the brief contract, and make the deterministic tools callable," not "turn briefs into functions." Typing raises the structural floor, not the correctness ceiling (see [The hard boundary](#the-hard-boundary-non-goal)).
- **Toolchain.** Components + `wit-bindgen` add build steps for adapter authors. Language-agnostic implementation is a benefit, but every adapter author needs a component toolchain (already true for the validators).
- **wasmtime feature maturity.** v45 ships stable Component Model support (records, variants, `result`, resources); confirm async-export and cross-boundary `resource` ergonomics before relying on them in Phase 3.
- **Cross-repo seam.** The adapter `.wasm` builds live in `specify-adapters`; the world re-export and the ABI cut must land in lockstep across both repos (workflow contract spans both).
- **RFC-50 invariant preserved.** The WIT package is generic — it carries no adapter *name* and no adapter *taxonomy*. The host still holds zero adapter-specific code; this RFC types the contract, it does not re-open the host to any adapter.
- **Lazy discovery preserved.** The brief-typing seams must not force eager loading of the reference shelf or phase sub-briefs (§G). The contract binds the boundary; the prose interior stays a lazily-walked graph and the authoring-time checks stay incremental per file. Regressing this would trade the brief tree's context-budget economy for nothing.
- **Frontmatter churn across adapters.** The brief-contract frontmatter lands in `specify-adapters`, so Phases 4–7 touch every first-party brief. Keeping the frontmatter optional through Phase 4 lets adoption stay incremental and `make lint` stay green per adapter.

## Acceptance criteria

1. **Single typed contract.** One `specify:adapter` WIT package defines every operation's request/report on both axes; no hand-rolled DTO or embedded JSON-Schema constant duplicates those shapes.
2. **Typed tool dispatch.** A deterministic adapter (`contract` / `vectis`) is invoked through generated bindings — no argv packing or stdout-JSON parsing on that path.
3. **No drift surface.** The `*_JSON_SCHEMA` constants + parity test are retired or regenerated from WIT.
4. **Named capabilities.** Typed-path adapters declare their host access in the world; no blanket `$CAPABILITY_DIR` grant for them.
5. **Brief bodies unchanged; contracts typed.** Brief bodies remain markdown and execution stays a handoff; each agent-executed brief binds the WIT signature it fulfils, and its inputs, outputs, capabilities, and embedded examples are checked against the world (§F).
6. **Operation coverage.** `{world exports} ∪ {brief bindings}` covers every operation on both axes exactly once, enforced as a `specify lint framework` check.
7. **Lazy discovery intact.** Reference shelves and phase sub-briefs are still loaded on demand; lint proves the discovery graph resolves, and no check requires loading the whole reference corpus (§G).
8. **RFC-50 invariant intact.** The host still passes the no-adapter-names / no-taxonomy grep + guard test from RFC-50's acceptance criteria.

