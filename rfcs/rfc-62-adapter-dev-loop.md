# RFC-62: The Adapter Development Loop

> Status: Proposed · Depends: [RFC-61](rfc-61-omnia-migration.md) Steps 2–3 (the adapter guests and the live eval harness in `specify-adapters` this RFC tightens) · Owns: the prose-edit → live-model round trip for adapter authors

## Abstract

Under [RFC-61](rfc-61-omnia-migration.md) an adapter's briefs compile into its component as prompt bodies and its references ship as the embedded MCP shelf. Testing the impact of a prose change therefore pays a full round trip today: rebuild the adapter guest to `wasm32-wasip2`, re-launch the eval driver through cargo, and spawn one `cursor-agent` per `create` in the exercised operation. This RFC owns the adapter author's inner loop: a dev-only prose overlay that removes compilation from prose iteration entirely, a single-operation selector on the eval guest, a driver-level model override, and the layering rule that reserves live model calls for the one question they alone can answer — whether the model behaves differently under the new prose.

## Where the time goes

The harness exists (the flattened `evals` package in `specify-adapters`: the `eval-driver` + `eval-guest` example targets — `evals/{runtime,guest}.rs` — driven per scenario by the `#[ignore]`d tests in `evals/live.rs`) and the loop it runs has two cost classes:

1. **Rebuild tax.** A brief edit invalidates the prose registry — `specify-prose-registry` emits `cargo:rerun-if-changed` per embedded document — so the adapter core and shim rebuild to `wasm32-wasip2` on every edit, and each live-test invocation shells out to `cargo build` for the guests and the driver before spawning it, a native workspace freshness check. Tens of seconds per iteration, all of it waste for a prose-only change: no Rust changed.
2. **Model tax.** The cursor backend spawns a fresh, context-free `cursor-agent` per `create`. A full `build` through the contracts guest is several sub-flow legs plus a verify-repair pass, and the eval guest currently exposes no smaller unit than one whole operation (and only `target.build` at that).

The design goal: an author iterating on one brief pays exactly one model leg per iteration and zero compilation.

## The prose overlay

`specify-guest-kit` gains a **dev-only prose overlay** behind a cargo feature (working name `prose-overlay`), off by default and never enabled by the `refresh-guests` release builds that produce the committed `guest.wasm` artifacts:

- The registry lookup (`registry::find` / `registry::body`) and the MCP shelf's read path first consult an overlay directory under the guest's shared `"."` preopen — `.eval/prose/<adapter-relative path>` — and fall back to the embedded `DOCS` table on a miss. Both the prompt body a judgment leg assembles and the document the spawned agent fetches over MCP resolve through the same lookup, so one overlay covers both.
- Each eval runner seeds the overlay by copying the adapter's `briefs/` and `references/` trees into the scratch project (resolving the `references/spec-runtime` symlinks the same way the build-time embed does), and builds the guests with the feature enabled.
- The overlay is additive per path: a document absent from the overlay serves its embedded body, and a path missing from both keeps the registry's existing fail-loud contract.

With the overlay in place the prose loop collapses to *edit markdown → re-invoke the driver*. No registry codegen, no component build, no cargo — the wasm on disk is already correct because the Rust did not change.

## Layered iteration

Three layers, cheapest first; escalate only when the layer below cannot answer the question:

1. **Native, no model** (seconds): the wasm-free core's tests assert prompt assembly through `MockModel`, which records every `Request` — "did my brief edit land in the assembled `system` / `user` text" never needs a component or a model call.
2. **Composed, no model**: the non-ignored `wiring` tests in `evals/live.rs` (scenario seeding + manifest rendering for every scenario, run by ordinary CI) plus the replay-backend runtime tests prove manifest, mount, link, and MCP-route wiring without `cursor-agent`.
3. **Composed, live**: the eval runner with the overlay — the only layer that judges the prose's effect on model output, and under this RFC the only layer that costs a model leg.

## One operation per invocation

The eval guest's argv grows an operation selector. Today it accepts `<adapter-id> <slice> <inputs-dir>` and dispatches only `target.build`; it extends to select any seam operation — `survey`, `extract`, `guidance`, `build`, `merge` — so a source adapter's briefs are exercisable at all, and a target author studying one brief drives one operation rather than a whole multi-leg build. Each operation prints its typed answer as one JSON line and carries its outcome in the exit status, matching the existing `build` behavior.

## Driver ergonomics

- **Model override.** `Request.model` is always `None` from guest-kit's `judgment` helper, and RFC-61's invariant keeps model ids out of guests. The override therefore lands in the eval driver's backend binding: an environment variable (working name `SPECIFY_EVAL_MODEL`) the driver passes to the cursor backend, letting authors iterate on a fast model and confirm on the default — without the id ever entering a guest or the WIT contract.
- **Prebuilt driver.** Landed: the live tests spawn the built `eval-driver` example binary directly. The remaining cost is the per-invocation `cargo build` freshness check, which the prose overlay makes a no-op for prose-only edits.
- **Watch loop.** A `cargo make` task wraps the runner in a file watch over the adapter's prose trees, so the save-to-report loop is hands-off.

## Scope

- The `prose-overlay` feature in `specify-guest-kit` (registry + shelf read paths) and the runner support that seeds and enables it.
- The operation selector on the eval guest.
- The driver-level model override and the watch task. (Prebuilt-driver invocation and the wiring-smoke generalization landed with the `evals/live.rs` test runner: each live test spawns the built `eval-driver` example binary, and the non-ignored `wiring` tests smoke every scenario model-free.)

## Out of scope

- **Record/replay of live answers as a regression surface.** Deterministic coverage of judgment legs stays in native core tests per the testing posture; `ModelDefault` replay remains a component-test convenience, and the larger replay question stays parked in [roadmap.md](roadmap.md) (orchestration trace replay).
- **The eval sweep and grading posture** in [docs/contributing/evals.md](../docs/contributing/evals.md) — unchanged; this RFC accelerates authoring iteration, not the proof surface.
- **The production runtime binary.** The overlay is an eval-deployment affordance, never a mode of the `specify` runtime.
- **Session reuse or prompt caching in the cursor backend.** Session-less `create` is an RFC-61 invariant, not a cost to engineer away here.

## Acceptance criteria

1. A prose-only edit followed by a runner re-invocation compiles nothing: no registry codegen, no crate build, no component build.
2. Under the overlay, both the assembled prompt body and the MCP-served reference reflect the on-disk markdown for every overlaid path.
3. The overlay feature is compiled out of the `refresh-guests` artifacts, and committed run summaries grade embedded-build runs only.
4. The eval guest can drive each `source` and `target` seam operation singly, with a typed JSON answer line and a carrying exit status.
5. A model-id override reaches the cursor backend from the driver's environment without entering any guest or the WIT contract.
6. After the one-time build, a runner invocation spawns no cargo process.

## Risks and invariants

- **Dev/prod divergence.** An overlay run proves prose against the live files, not against the shipped component; graded evidence (the committed run summaries, the eval sweep) comes from embedded builds only. The overlay exists to iterate, never to certify.
- **Fail-loud lookup holds.** The overlay never silently serves an empty or partial document; a miss falls back to the embedded body, and a path absent from both keeps the registry's existing panic contract.
- **The seam does not move.** No change to `specify:adapter` — this is harness and guest-kit surface only, and the operation selector uses the seam operations as they exist.
- **Model ids stay out of guests.** The override is backend configuration in the driver, preserving RFC-61's contract-agnosticism invariant.
