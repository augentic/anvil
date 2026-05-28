---
id: CORE-006
title: Adapter Manifest Version
severity: important
trigger: An `adapters/{sources,targets}/<name>/adapter.yaml` declares a `version:` field whose value does not equal the canonical `1`, so the workflow loader cannot rely on the wire shape pinned by the v1 manifest schema.
deterministic_hints:
  - kind: path-pattern
    value: "adapters/sources/*/adapter.yaml"
    description: Source adapter manifests; `version:` must equal the literal `1` pinned by `source.schema.json`.
  - kind: path-pattern
    value: "adapters/targets/*/adapter.yaml"
    description: Target adapter manifests; `version:` must equal the literal `1` pinned by `target.schema.json`.
  - kind: constant-eq
    value: adapter-manifest-version-equals-v1
    description: For each `AdapterManifest` fact in the candidate set, assert that the stringified `version` field equals `1`. One finding per non-conforming manifest, with the `(actual, expected)` pair surfaced as structured evidence.
---

## Rule

Every `adapters/{sources,targets}/<name>/adapter.yaml` declares its manifest wire shape through the top-level `version:` field. v1 of the Specify adapter contract pins this field to the literal `1`; a manifest that ships any other value (`2`, `"0.9"`, an absent field) cannot be safely loaded by the v1 dispatcher because field semantics, operation enums, and brief shapes are wire-coupled to that version discriminant.

The deterministic-hint interpreter consumes the `AdapterManifest` facts the framework-profile indexer already produced (`crates/specify-lints/src/lint/index/adapter.rs::extract`, whose `version` field stringifies both integer (`1`) and quoted-string (`"2.1"`) YAML forms verbatim), so the rule cost is one string-equality compare per candidate manifest at lint time. The single source discriminator hardcodes both the field (`AdapterManifest.version`) and the expected constant (`"1"`); a richer config shape (`{field: …, expected: …}`) is deferred until a second consumer arrives.

No imperative `Check` row is retired by this rule: the workflow contract pins `version: 1` but no existing predicate enforces it on disk. CORE-006 is the first declarative enforcement of the invariant and the smoke-test landing path for the `constant-eq` deterministic hint kind (the migration-cadence fallback permitted when no imperative row maps cleanly to a hint kind). Every adapter manifest in the framework repo already declares `version: 1`, so the rule fires zero findings against the current tree and surfaces only on drift.

## Look For

- A newly added adapter manifest copy-pasted from an external scaffold that ships `version: 2` (or any other non-`1` value) without coordinating a v2 dispatcher.
- A manifest scaffolded without the `version:` field at all, relying on schema-default behaviour that the v1 loader does not provide.
- A migration that quoted the version as a string (`version: "1.0"`) for YAML-style consistency; the v1 contract expects the bare `1` (integer or quoted string equivalent), and any decoration that changes the stringified value trips the rule.

## Fix

Set the manifest's top-level `version:` field to the literal `1`:

```yaml
name: <adapter-name>
version: 1
```

If the manifest genuinely needs a different version, the change belongs in a coordinated CLI release — bump the dispatcher's accepted version set, widen `CORE-006`'s expected constant in the same PR, and update `source.schema.json` / `target.schema.json` accordingly. CORE-006 is the canary, not the policy authority; the policy lives in the schema and the dispatcher.
