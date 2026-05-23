# Shared fixture-replay hook contract

Cross-target, build-time fixture-replay rules at the root of `adapters/targets/` — read by any target adapter that opts into the RFC-27 §D1 hook during `/spec:build`. This directory is a **convention folder** resolved outside `adapter.yaml` (same posture as [`../codex/`](../codex/)); it is not a target adapter and does not add a fourth operation.

## Relationship to `code-runtime`

The **wire format** for captured fixtures lives on the source axis:

- [`adapters/sources/code-runtime/references/fixture-format.md`](../../sources/code-runtime/references/fixture-format.md) — directory layout and behavioural JSON fields
- [`adapters/sources/code-runtime/briefs/extract.md`](../../sources/code-runtime/briefs/extract.md) — `kind: example` claim emission

This directory owns the **target-side hook contract**: when to run, how to record results, merge posture, and advisory v1 semantics. Test-harness depth (MockProvider, Crux effects, contract tool invocation) stays under each target adapter's `references/` and `briefs/build/replay.md`.

## Target adoption

| Target | Hook status | Entry point |
|---|---|---|
| **Omnia** | Implemented | [`../omnia/briefs/build/replay.md`](../omnia/briefs/build/replay.md) |
| **Vectis** | Not implemented (v1) | — |
| **Contracts** | Not implemented (v1) | — |
| **default** | Not implemented | — |

Targets that skip the hook produce no `fixture-replay` field and emit no journal event; omission is not an error.

## How to consume

1. Read [`hook-contract.md`](hook-contract.md) for skip rules, preconditions, advisory posture, recording, and merge summary behaviour.
2. Read [`journal-payload.md`](journal-payload.md) for the closed journal and aspirational `.metadata.yaml` shapes.
3. Implement a target-specific runner in `adapters/targets/<name>/briefs/build/replay.md` (or an inline build step) that links here and adds the runner command, paths, and harness references for that target.

## See also

- [`../codex/`](../codex/) — sibling shared convention for review rules (`UNI-*`)
- [Target adapters reference](../../../docs/reference/targets/index.md)
