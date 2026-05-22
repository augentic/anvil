---
name: rt-replay-writer
description: Wire captured replay fixtures into the workflow — fixture-to-Evidence extraction lives at the `code-runtime` source adapter and build-time fixture replay lives at the Omnia target's `build` brief. Use when an operator asks about replay-aware target work, not for capturing fresh fixtures (that is `wiretapper`) and not for hand-authoring tests against `tests/data/replay/` (now consumed by `code-runtime` as a source-adapter input).
argument-hint: "[slice-name]"
---

# Replay Writer Skill

The replay-writer skill is the operator's named entry point for replay-aware target work. As of Specify 2.1 the substantive work lives at first-class adapter homes: fixture-to-Evidence extraction at the `code-runtime` source adapter, build-time fixture replay at the Omnia target's `build` brief. This SKILL.md routes the operator to whichever surface their question lands on; it no longer authors `tests/data/replay/` and no longer runs `cargo test` directly.

## Critical Path

1. **Fixture-to-Evidence extraction.** Captured fixtures under `tests/data/replay/<handler>/<scenario>.json` are consumed by the `code-runtime` source adapter, which walks the bound fixture tree, enumerates one candidate per handler entry point, and extracts `kind: example` claims (authority `behaviour`) with `fixture-digest: sha256:<hex>` anchors. Operator binds the directory at plan time:

   ```yaml
   # plan.yaml fragment
   sources:
     runtime:
       adapter: code-runtime
       path: ./fixtures/replay
   ```

   See [`adapters/sources/code-runtime/briefs/extract.md`](../../../../adapters/sources/code-runtime/briefs/extract.md) for the claim shape, the required fields (`claim-id`, `path`, `fixture-digest`, `statement`), the 64 KiB inline cap, and the determinism rules. The fixture-tree layout itself stays under [`references/fixture-format.md`](references/fixture-format.md) — `code-runtime`'s extract brief links to that same file rather than re-stating the format.

2. **Build-time fixture replay (optional).** Generated crates run their replay tests against the same fixture tree during the Omnia target's `build` phase. The hook is **OPTIONAL in v1**: targets that omit it produce no `fixture-replay` field, and omission is not an error.

   See [`adapters/targets/omnia/briefs/build.md`](../../../../adapters/targets/omnia/briefs/build.md) § Fixture replay for the optional step, the `.metadata.yaml` write contract (`passed` / `failed` / `skipped` / `ran-at` / `runner`), and the `slice.fixture-replay.completed` journal event payload.

3. **`merge` is advisory, not gating.** When the optional hook runs, `merge` surfaces a one-line summary in its closing message (e.g. `fixture-replay: 47 passed, 0 failed, 2 skipped`) but does **not** auto-refuse on `failed > 0`. The operator decides whether to land. Stricter posture wires through a custom target adapter fork (refuse from the fork's own `merge.md`) or through a CI policy reading `specify slice outcome show <slice> --format json`.

## What this skill no longer does

- **Author tests against `tests/data/replay/`.** That directory is a source-adapter input now. The Omnia target's `build/test.md` sub-brief is the authority on per-crate test generation; the test-writer skill body remains the authority on test depth (MockProvider construction, spec-to-test mapping).
- **Run `cargo test` directly.** The verify-repair loop lives in [`adapters/targets/omnia/briefs/build.md`](../../../../adapters/targets/omnia/briefs/build.md) and re-enters phase sub-briefs on failure.
- **Hold the slice lifecycle.** Transitions are owned by `specify slice transition`, `specify slice outcome set`, and `specify slice merge`.

## References

- [`adapters/sources/code-runtime/briefs/enumerate.md`](../../../../adapters/sources/code-runtime/briefs/enumerate.md) — handler-grain candidate enumeration.
- [`adapters/sources/code-runtime/briefs/extract.md`](../../../../adapters/sources/code-runtime/briefs/extract.md) — `kind: example` claim emission; absorbs the body of this skill's old `extract` half.
- [`adapters/targets/omnia/briefs/build.md`](../../../../adapters/targets/omnia/briefs/build.md) — build orchestrator that hosts the optional fixture-replay step and the `.metadata.yaml` write contract.
- [Fixture format](references/fixture-format.md) — replay-fixture file shape (TestDef, `setup`, `samples/`, `INSTRUCTIONS.md`); cited by `code-runtime`'s extract brief.
- [Crate layout](references/crate-layout.md) — generated-crate paths the replay tests run against.
- Sibling skill: [wiretapper](../wiretapper/SKILL.md) — TypeScript instrumentation that produces the fixture tree this skill points at. Unaffected by the source/target split; source-side instrumentation stays an RT plugin concern.
