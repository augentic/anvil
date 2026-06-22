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

The package is authored in [`wit/specify.wit`](../wit/specify.wit) — this RFC points to that file rather than duplicating it. In summary:

- `interface types` carries every operation's request/report records for both axes, plus the shared `finding`, `severity`, `trust-level`, and `adapter-error` shapes.
- `interface target` declares the deterministic target operations (`shape`, `build`, `merge`); `interface source` declares the source operations (`survey`, `extract`).
- Each world exports only the deterministic subset its component actually implements: `world target-adapter` exports `target`, while `world source-adapter` exports nothing today because both source operations are still brief-bound (agent-only).

Host-capability imports (project / slice handles) are deferred to [RFC-52](rfc-52-effect.md); until then `tool` operations reach host data through the existing preopen / `$CAPABILITY_DIR` grant.

### 3. Versioning & Publishing
The `specify` repo owns and publishes the `augentic:specify@<semver>` WIT package via `wkg publish`. `specify-adapters` consumes it as a pinned dependency. The host advertises supported world versions, preventing runtime mismatches.

## Scope & Boundaries

**In scope:** 
- Authoring the `augentic:specify` WIT package (see [`wit/specify.wit`](../wit/specify.wit)).
- Landing the simplified claim / `trust-level` data model the records encode, including the downstream `schemas/evidence.schema.json` and Rust engine updates (see [Decisions to record](#decisions-to-record)).
- Wiring host-side `wasmtime::component::bindgen!` bindings against the package.
- Publishing the package as a pinned dependency.

**Out of scope (handled by downstream RFCs):**
- **Contract + data model only:** RFC-51 lands the typed records and the simplified claim / `trust-level` model they encode. It does not change *how* operations are invoked, retire any `*_JSON_SCHEMA`, or alter synthesis behaviour beyond consuming the new claim shape.
- **Typed agent envelopes & `*_JSON_SCHEMA` retirement:** Handled in [RFC-52](rfc-52-effect.md). RFC-51 aligns `schemas/evidence.schema.json` to the new model but does not retire the embedded-schema drift surface.
- **Typed tool dispatch:** Routing execution through generated bindings is in [RFC-53](rfc-53-orchestration.md).
- **Component-on-both-axes mandate:** Requiring every adapter to ship a component is in [RFC-55](rfc-55-runtime-move.md).
- **Brief execution:** Briefs remain markdown executed by an LLM. There is no WASM component running the prose.

## Phased Plan

**Phase 0 — Author WIT package + host bindings + publish**
Author `augentic:specify` ([`wit/specify.wit`](../wit/specify.wit)), wire `wasmtime::component::bindgen!` host-side, and publish via `wkg publish`.

**Phase 1 — Land the data model**
Migrate `schemas/evidence.schema.json` and the Rust engine (the `Evidence` / claim DTOs, the `authority` → `trust-level` rename, and the dependent `slice/model`, `slice/provenance`, and `plan` schemas) to the records the package encodes, and update the `specify-adapters` source `extract` briefs that author Evidence. Assert the generated bindings match the authored records.

## Decisions to record

- **WIT package as schema source of truth:** Records are authored here and become the source of truth; the `*_JSON_SCHEMA` constants are retired against them in [RFC-52](rfc-52-effect.md).
- **Data Model Refinements:**
  - The `claim` record has been radically simplified. The `claim-kind` enum (and its 14 variants) has been completely removed from the extraction contract. Extraction adapters are no longer responsible for categorizing claims or making downstream routing decisions; they simply extract facts, providing a `synopsis` and a `source`. The burden of semantic categorization and routing is shifted entirely to the synthesis engine.
  - The `claim.path` and `claim.payload` fields have been consolidated into a single optional `source` variant (`path(string)` | `payload(string)`), carried as `option<source>`; an absent `source` is the third state (no pointer and no inline data). This enforces a strict "pointer vs. data" tradeoff at the type level, preventing adapters from bloating the YAML with raw data when a file path would suffice, while still allowing in-memory or synthesized data to be passed directly.
  - The `example` claim kind concept has been renamed to `wiretap` in documentation, though it no longer exists as a distinct type in the WIT.
  - The `claim.id` field has been completely removed. Cross-source reconciliation is fundamentally a semantic operation performed by the synthesis engine, and forcing independent extraction adapters to generate deterministic join strings is an anti-pattern that leads to silent merge failures.
  - A new `claim.synopsis` field has been introduced as a required field on all claims. This replaces the scattered `statement` and `excerpt` fields across the various claim details, providing a unified, reconciliation-grade headline for the synthesis engine to use when semantically merging claims across sources.
  - The `authority` enum has been renamed to `trust-level` (with variants `directive`, `specification`, `observation`) to clarify its role in conflict resolution. These changes will require downstream updates to `schemas/evidence.schema.json` and the Rust engine.
- **Content-addressed, node-independent build I/O:** The build contract no longer passes a bare `project-path` string or inline document bodies. Inputs are `artifact` handles (a content-addressed reference whose body is pulled lazily, never inlined); a build's result is a `change-set` — a portable delta of adds, modifies, and deletes against a base `revision` — carried on the build `report` and consumed by `merge-request`, replacing the earlier `produced` / `built` file lists. The mutable project tree itself is handed to `build` as a `working-tree` *capability* rather than a path. The records this introduces (`revision`, `edit`, `change-set`) are part of this package; the capability's runtime semantics (the `wasi:filesystem` descriptor and the agent `local-path` bridge) are [RFC-52](rfc-52-effect.md)'s, consistent with this RFC deferring host-capability imports there.
- **Versioning & ownership:** `specify` publishes, `specify-adapters` consumes.
- **Capability model:** Deferred to [RFC-52](rfc-52-effect.md).
- **Brief-typing surface:** Deferred to [RFC-53](rfc-53-orchestration.md).

## Acceptance Criteria

1. **Single typed contract:** the `augentic:specify` package ([`wit/specify.wit`](../wit/specify.wit)) defines every operation's request/report records and per-axis signatures.
2. **Host bindings:** `wasmtime::component::bindgen!` is wired host-side and the generated types match the records authored in the package.
3. **Data model landed:** `schemas/evidence.schema.json` and the Rust engine encode the simplified claim / `trust-level` model; the dependent `slice/model`, `slice/provenance`, and `plan` schemas and the `specify-adapters` `extract` briefs are aligned to it.
4. **Published + pinned:** Package is published via `wkg publish` and resolvable by `specify-adapters`.
5. **RFC-50 invariant intact:** The package carries no adapter name or taxonomy.
