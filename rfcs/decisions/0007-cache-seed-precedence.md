# ADR-0007: Project cache seeds answer exact pins

> Status: Accepted
> Date: 2026-08-18

## Context

The launcher's pinned-resolution leg (`launcher::resolver::resolve_pinned`) originally consulted only the global store: an exact pin (`emery:omnia@1.0.0`) was answered by the store entry, with verify-on-read against the recorded byte digest, installing on miss. The project component cache (`<project-cache>/components/<name>.wasm`, seeded by `emery adapter add` or a local component at init) answered only bare names.

That split broke the co-development loop the cache exists for: a definition pinning a development version (`emery:target@0.0.0`) could never be answered by the seeded component — the launcher went to the registry for a version that does not exist there. The Phase 0 wasm-example repair (remediation plan T14) hit exactly this. Recorded here because pin resolution is a supply-chain-relevant policy, not lab tooling.

## Options

- **A. Cache seeds answer pins too.** The seed wins every dispatch for that adapter name; the pin's digest verification applies only to store-resolved components.
- **B. Keep pins store-only; make examples use bare names.** Fails because bare names fall through to the compiled first-party catalog, which has no row for development adapters — the example (and any co-dev on an unpublished adapter version) cannot run.
- **C. Verify the seed against the pinned version.** Requires version metadata on seeded bytes; a development seed is by definition not the published version, so this reintroduces the same dead end as B.

## Decision

**Option A.** Cache hits always win, for pinned and bare dispatches alike. `resolve_pinned` consults the project cache seed before the store; only a seedless pin reaches the store/registry leg with its digest verification. The seed is an operator act on this project (`emery adapter add`), so the operator has already chosen to trust those bytes over any published version.

Every settled identity is logged to stderr (host version + adapter version + origin), and a non-durable settle — a bare dispatch or a seed-answered pin — is journaled as `adapter.identity.settled` when a change journal exists, so a seed shadowing a pin is always auditable.

## Deletions

Nothing is deleted. Concept-count effect: none — no new operator-visible noun; the existing "cache hits always win" rule loses its pinned-dispatch exception, which is a simplification of the resolution story.

## Consequences

- Co-dev seeds are never shadowed by published components; the wasm examples and adapter development loops work with exact development pins.
- A stale (or hostile) cache seed overrides a pinned identity silently at resolve time — the journal fact is the audit trail, not a gate. Removing the seed restores pin verification. Accepted: the seed already required write access to the project cache, which is the same trust boundary.

## Revisit trigger

The remediation programme's seam rework (ADR-0002 Wasm-primary, ADR-0006): if the walking skeleton re-cuts adapter resolution, the replacement must either conserve this behavior or supersede this ADR. Independently: any incident where a seed-answered pin caused a wrong-component run in a non-development project reopens this in favor of Option C.
