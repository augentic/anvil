# Shared replay hook contract

Cross-target, build-time `replay` rules under `adapters/shared/` — read by any target adapter that opts into the the capture-backed replay workflow hook during `/spec:build`. This directory is shared support material, not a target adapter, and does not add a fourth operation.

## Relationship to `captures`

The **wire format** for runtime captures lives on the source axis:

- [`adapters/sources/captures/prose/references/capture-format.md`](https://github.com/augentic/specify-adapters/blob/main/sources/captures/prose/references/capture-format.md) — directory layout and behavioural JSON fields
- [`sources/captures/prose/prompts/extract.md`](https://github.com/augentic/specify-adapters/blob/main/sources/captures/prose/prompts/extract.md) — `kind: example` claim emission

This directory owns the **target-side hook contract**: when to run, how to record results, merge posture, and advisory v1 semantics. Test-harness depth (MockProvider, Crux effects, contract tool invocation) stays under each target adapter's `references/` and `prompts/build/replay.md`.

## Target adoption

| Target | Hook status | Entry point |
|---|---|---|
| **Omnia** | Implemented | [`targets/omnia/prose/prompts/build/replay.md`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/prompts/build/replay.md) |
| **Vectis** | Not implemented (v1) | — |
| **Contracts** | Not implemented (v1) | — |
| **default** | Not implemented | — |

Targets that skip the hook produce no `replay` field and emit no journal event; omission is not an error.

## How to consume

1. Read [`hook-contract.md`](hook-contract.md) for skip rules, preconditions, advisory posture, recording, and merge summary behaviour.
2. Read [`journal-payload.md`](journal-payload.md) for the closed journal and aspirational `metadata.yaml` shapes.
3. Implement a target-specific runner in `targets/<name>/prose/prompts/build/replay.md` (or an inline build step) that links here and adds the runner command, paths, and harness references for that target.

## See also

- [`../../rules/universal/`](../../rules/universal/) — sibling shared review rules (`UNI-*`)
- [Target adapters reference](../../../../../docs/reference/targets/index.md)
