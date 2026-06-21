# RFC-51: Adapter WIT — the typed contract package

> Status: Draft · Depends: RFC-47, RFC-48, RFC-49, RFC-50 · Framed by: [effect-oriented architecture](architecture.md)

## Abstract

This RFC authors the typed contract that replaces the loose `wasi:cli/run` surface: one versioned **WebAssembly Component Model** package, `augentic:specify@<semver>`. It defines every operation's request/report records, per-axis `target`/`source` interface signatures, and the worlds that export deterministic operations. 

It is the foundation of the effect-oriented architecture, establishing the shared envelope currency used by both deterministic tool paths and agent handoffs.

## Motivation

Currently, adapters are invoked via `wasi:cli/run` using argv conventions and JSON envelopes validated at runtime against embedded schemas. This causes:
- **Schema/code drift:** Requires manual parity testing between schemas and DTOs.
- **Untyped invocation:** Operation contract relies on argv and stdout parsing.
- **Broad capability grants:** Unnamed filesystem access via preopened directories.
- **Convention-based errors:** Failures cross as exit codes and stderr text.

Authoring a WIT package provides a single generated source of truth to resolve these issues.

## The Model

### 1. One shared WIT package
`augentic:specify@<semver>` defines an `interface types` carrying every operation's request/report records for both axes (build, merge, shape, survey, extract, evidence), plus shared `finding` and `adapter-error` shapes.

### 2. Interface declarations
The interface declares every operation. Worlds export only the deterministic subset.

```wit
package augentic:specify@0.1.0;

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
interface target {
  use types.{ build-request, build-report, merge-request, merge-report, adapter-error };
  build: func(req: build-request) -> result<build-report, adapter-error>;
  merge: func(req: merge-request) -> result<merge-report, adapter-error>;
}

world target-adapter {
  export target;
}
```

### 3. Versioning & Publishing
The `specify` repo owns and publishes the `augentic:specify@<semver>` WIT package via `wkg publish`. `specify-adapters` consumes it as a pinned dependency. The host advertises supported world versions, preventing runtime mismatches.

## Scope & Boundaries

**In scope:** 
- Authoring the `augentic:specify` WIT package.
- Wiring host-side `wasmtime::component::bindgen!` bindings.
- Asserting transitional parity against current envelope records.
- Publishing the package as a pinned dependency.

**Out of scope (handled by downstream RFCs):**
- **No behaviour change:** This RFC only lands the contract.
- **Typed agent envelopes & schema retirement:** Handled in [RFC-52](rfc-52-effect.md).
- **Typed tool dispatch:** Routing execution through generated bindings is in [RFC-53](rfc-53-orchestration.md).
- **Component-on-both-axes mandate:** Requiring every adapter to ship a component is in [RFC-55](rfc-55-runtime-move.md).
- **Brief execution:** Briefs remain markdown executed by an LLM. There is no WASM component running the prose.

## Phased Plan

**Phase 0 — Author WIT package + host bindings + publish**
Define `augentic:specify`, wire `wasmtime::component::bindgen!` host-side, assert generated types match current envelope records (transitional parity), and publish via `wkg publish`.

## Decisions to record

- **WIT package as schema source of truth:** Records are authored here; `*_JSON_SCHEMA` constants will be retired against them.
- **Data Model Refinements:**
  - The `claim` record has been radically simplified. The `claim-kind` enum (and its 14 variants) has been completely removed from the extraction contract. Extraction adapters are no longer responsible for categorizing claims or making downstream routing decisions; they simply extract facts, providing a `synopsis` and a `source`. The burden of semantic categorization and routing is shifted entirely to the synthesis engine.
  - The `claim.path` and `claim.payload` fields have been consolidated into a single `claim-source` variant (`path(string)`, `payload(string)`, `none`). This enforces a strict "pointer vs. data" tradeoff at the type level, preventing adapters from bloating the YAML with raw data when a file path would suffice, while still allowing in-memory or synthesized data to be passed directly.
  - The `example` claim kind concept has been renamed to `wiretap` in documentation, though it no longer exists as a distinct type in the WIT.
  - The `claim.id` field has been completely removed. Cross-source reconciliation is fundamentally a semantic operation performed by the synthesis engine, and forcing independent extraction adapters to generate deterministic join strings is an anti-pattern that leads to silent merge failures.
  - A new `claim.synopsis` field has been introduced as a required field on all claims. This replaces the scattered `statement` and `excerpt` fields across the various claim details, providing a unified, reconciliation-grade headline for the synthesis engine to use when semantically merging claims across sources.
  - The `authority` enum has been renamed to `trust-level` (with variants `directive`, `specification`, `observation`) to clarify its role in conflict resolution. These changes will require downstream updates to `schemas/evidence.schema.json` and the Rust engine.
- **Versioning & ownership:** `specify` publishes, `specify-adapters` consumes.
- **`shape` semantics:** Whether `shape` is a world export, manifest-declared file, or envelope.
- **Operation set vs declared tools:** Whether manifest's declared-tool set and operation set unify under one world.
- **Capability model:** Deferred to [RFC-52](rfc-52-effect.md).
- **Brief-typing surface:** Deferred to [RFC-53](rfc-53-orchestration.md).

## Acceptance Criteria

1. **Single typed contract:** `augentic:specify` WIT package defines every operation's request/report records and per-axis signatures.
2. **Host bindings + parity:** `wasmtime::component::bindgen!` is wired host-side and generated types match today's envelope records.
3. **Published + pinned:** Package is published via `wkg publish` and resolvable by `specify-adapters`.
4. **RFC-50 invariant intact:** The package carries no adapter name or taxonomy.
