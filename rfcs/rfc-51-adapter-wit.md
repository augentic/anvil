# RFC-51: Adapter WIT — the typed contract package

> Status: Draft · Depends (landed; durable spec in [engine/DECISIONS.md](../engine/DECISIONS.md)): RFC-47 (adapter identity / semver), RFC-48 (packaging & transport), RFC-49 (cross-repo seam), RFC-50 (name-free host) · Framed by: [effect-oriented architecture](architecture.md)

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
`augentic:specify@<semver>` defines an `interface types` carrying the cross-cutting data types shared across both axes (`error`, `artifact`, `revision`, `edit`, `changeset`). The per-operation I/O and judgment records live in the axis interfaces: `input`, `working-tree`, `finding`, `severity`, `outcome`, `report` in `interface target`; `lead`, `weight`, `backing`, `claim`, `evidence` in `interface source`.

### 2. Interface declarations

The package is authored in [`wit/specify.wit`](../wit/specify.wit) — this RFC points to that file rather than duplicating it. In summary:

- `interface types` carries the cross-cutting data types (`error`, `artifact`, `revision`, `edit`, `changeset`). The single error variant is `error` (`invalid-request` | `io` | `internal`); `finding` / `severity` / `outcome` and the `working-tree` record live in `interface target`, and `weight` in `interface source`.
- `interface target` declares the deterministic target operations (`guidance`, `build`, `merge`); `interface source` declares the source operations (`survey`, `extract`). (`guidance` is the WIT export name for the operation the adapter brief layer still calls `shape`; reconciling the brief filenames and the `TargetOperation` enum to `guidance` is tracked downstream.)
- `interface references` is the **reference shelf** — a stateless `resolve(id) → bytes` export the native tool-use loop ([RFC-53](rfc-53-tool-server.md)) calls to follow a brief's internal references (its surface and capability semantics are [RFC-52](rfc-52-effect.md)'s).
- `world target-adapter` exports `target` and `world source-adapter` exports `source` (`survey` + `extract`); both worlds also export `references`. The judgment operations may still be satisfied by the native tool-use loop over the adapter's brief (agent-only) even though the world exports the interface.

Host-capability imports (project / slice handles) are deferred to [RFC-52](rfc-52-effect.md); until then `tool` operations reach host data through the existing preopen / `$CAPABILITY_DIR` grant.

### 3. Versioning & Publishing
The `specify` repo owns and publishes the `augentic:specify@<semver>` WIT package via `wkg publish`. `specify-adapters` consumes it as a pinned dependency. The host advertises supported world versions, preventing runtime mismatches.

## Scope & Boundaries

**In scope:** 
- Authoring the `augentic:specify` WIT package (see [`wit/specify.wit`](../wit/specify.wit)).
- Landing the simplified claim / `weight` data model the records encode, including the downstream `schemas/evidence.schema.json` and Rust engine updates (see [Decisions to record](#decisions-to-record)).
- Wiring host-side `wasmtime::component::bindgen!` bindings against the package.
- Publishing the package as a pinned dependency.

**Out of scope (handled by downstream RFCs):**
- **Contract + data model only:** RFC-51 lands the typed records and the simplified claim / `weight` model they encode. It does not change *how* operations are invoked, retire any `*_JSON_SCHEMA`, or alter synthesis behaviour beyond consuming the new claim shape.
- **Typed agent envelopes & `*_JSON_SCHEMA` retirement:** Handled in [RFC-52](rfc-52-effect.md). RFC-51 aligns `schemas/evidence.schema.json` to the new model but does not retire the embedded-schema drift surface.
- **Typed tool dispatch:** Routing execution through generated bindings is in [RFC-54](rfc-54-orchestration.md).
- **Component-on-both-axes mandate:** Requiring every adapter to ship a component is in [RFC-56](rfc-56-runtime-move.md).
- **Brief execution:** Briefs remain markdown executed by an LLM. There is no WASM component running the prose.

## Phased Plan

**Phase 0 — Author WIT package + host bindings + publish**
Author `augentic:specify` ([`wit/specify.wit`](../wit/specify.wit)), wire `wasmtime::component::bindgen!` host-side, and publish via `wkg publish`.

**Phase 1 — Land the data model**
Migrate `schemas/evidence.schema.json` and the Rust engine (the `Evidence` / claim DTOs, the `authority` → `weight` rename, and the dependent `slice/model`, `slice/provenance`, and `plan` schemas) to the records the package encodes, and update the `specify-adapters` source `extract` briefs that author Evidence. Assert the generated bindings match the authored records.

## Decisions to record

- **WIT package as schema source of truth:** Records are authored here and become the source of truth; the `*_JSON_SCHEMA` constants are retired against them in [RFC-52](rfc-52-effect.md).
- **Data Model Refinements:**
  - The `claim` record has been radically simplified. The `claim-kind` enum (and its 14 variants) has been completely removed from the extraction contract. Extraction adapters are no longer responsible for categorizing claims or making downstream routing decisions; they simply extract facts, providing a `synopsis` and a `source`. The burden of semantic categorization and routing is shifted entirely to the synthesis engine.
  - The `claim.path` and `claim.payload` fields have been consolidated into a single optional `backing` variant (`payload(string)` | `path(string)`), carried as `backing: option<backing>`; an absent `backing` is the third state (no pointer and no inline data). This enforces a strict "pointer vs. data" tradeoff at the type level, preventing adapters from bloating the YAML with raw data when a file path would suffice, while still allowing in-memory or synthesized data to be passed directly.
  - The `example` claim kind concept has been renamed to `wiretap` in documentation, though it no longer exists as a distinct type in the WIT.
  - The `claim.id` field has been completely removed. Cross-source reconciliation is fundamentally a semantic operation performed by the synthesis engine, and forcing independent extraction adapters to generate deterministic join strings is an anti-pattern that leads to silent merge failures.
  - A new `claim.synopsis` field has been introduced as a required field on all claims. This replaces the scattered `statement` and `excerpt` fields across the various claim details, providing a unified, reconciliation-grade headline for the synthesis engine to use when semantically merging claims across sources.
  - The `authority` enum has been renamed to `weight` (with variants `directive`, `specification`, `observation`) to clarify its role in conflict resolution. These changes will require downstream updates to `schemas/evidence.schema.json` and the Rust engine.
- **Content-addressed, node-independent build I/O:** The build contract no longer passes a bare `project-path` string. Build inputs cross as the typed `input` variant, carrying a `string` per kind (`proposal`, `design`, `tasks`, `spec`, `other`); a build's result is a `changeset` — a portable delta of adds, modifies, and deletes against a base `revision` — which the caller's native orchestration layer extracts from the working tree (a `git diff` against `base`) and feeds to `merge`, replacing the earlier `produced` / `built` file lists. The mutable project tree itself is handed to `build` (and to `merge`, as the baseline being folded into) as a `working-tree` *capability* rather than a path; neither operation returns the delta, so the `report` carries only judgment. The records this introduces (`revision`, `edit`, `changeset`) are part of this package — `artifact` is the content-addressed handle used for `edit.content` inside a `changeset`, not for build inputs; the capability's runtime semantics (the `wasi:filesystem` descriptor and the agent `local-path` bridge) are [RFC-52](rfc-52-effect.md)'s, consistent with this RFC deferring host-capability imports there.
- **Working-tree capability shape — a `working-tree` record, not a resource:** The materialized tree is served by a **custom git-aware `wasi:filesystem` backend** (native; [RFC-55](rfc-55-working-tree.md)) and Omnia adds no bespoke Specify host, so the capability is stock pieces bundled as a `working-tree` *record* (`{ base: revision, root: descriptor }`) in `interface target`: `build` / `merge` take it as `tree`, the borrowed `wasi:filesystem` `descriptor` (`root`) is opened over the tree the host materialized from `base`, and the caller's native orchestration layer extracts the `changeset` (`git diff` against `base`) — no host-implemented resource. `revision` / `edit` / `changeset` are package records in `interface types`; `local-path` is a property the host reports for the spawned-agent bridge, gated on a disk-backed backend ([RFC-52](rfc-52-effect.md)). The remaining follow-on is the `wasmtime::component::bindgen!` host-binding update.
- **Versioning & ownership:** `specify` publishes, `specify-adapters` consumes.
- **Capability model:** Deferred to [RFC-52](rfc-52-effect.md).
- **Brief-typing surface:** Deferred to [RFC-54](rfc-54-orchestration.md).

## Acceptance Criteria

1. **Single typed contract:** the `augentic:specify` package ([`wit/specify.wit`](../wit/specify.wit)) defines every operation's request/report records and per-axis signatures.
2. **Host bindings:** `wasmtime::component::bindgen!` is wired host-side and the generated types match the records authored in the package.
3. **Data model landed:** `schemas/evidence.schema.json` and the Rust engine encode the simplified claim / `weight` model; the dependent `slice/model`, `slice/provenance`, and `plan` schemas and the `specify-adapters` `extract` briefs are aligned to it.
4. **Published + pinned:** Package is published via `wkg publish` and resolvable by `specify-adapters`.
5. **RFC-50 invariant intact:** The package carries no adapter name or taxonomy.
