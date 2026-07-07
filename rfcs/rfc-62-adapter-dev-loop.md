# RFC-62: The Adapter Development Loop

> Status: Implemented (2026-07) · Depends: RFC-61 (landed; removed from the tree — the adapter guests and the live eval harness in `specify-adapters` this RFC tightens) and RFC-64 (one component, no manifest — landed) · Owns: the prose-edit → live-model round trip for adapter authors

## Abstract

An adapter's prompts compile into its component as judgment-leg prompt bodies and its references ship as the embedded MCP shelf (RFC-61, with the `briefs/` → `prompts/` rename recorded in the adapters repo's `DECISIONS.md`). Testing the impact of a prose change therefore pays a full round trip today: rebuild the adapter guest to `wasm32-wasip2`, re-launch the eval driver through cargo, and spawn one `cursor-agent` per `create` in the exercised operation. This RFC owns the adapter author's inner loop: a dev-only prose overlay that removes compilation from prose iteration entirely, a single-operation selector on the eval guest, a driver-level model override, and the layering rule that reserves live model calls for the one question they alone can answer — whether the model behaves differently under the new prose.

## Where the time goes

The harness exists (the flattened `evals/` package at the `specify-adapters` workspace root: the `eval-driver` + `eval-guest` example targets — `evals/{runtime,guest}.rs` — driven per scenario by the `#[ignore]`d tests in `evals/live.rs`, over the shared host harness crate at `crates/harness`) and the loop it runs has two cost classes:

1. **Rebuild tax.** A prompt edit invalidates the prose registry — the `prose` crate's build-time codegen (`prose::emit_core`, emitting `$OUT_DIR/registry_docs.rs`) prints `cargo:rerun-if-changed` per embedded document — so the adapter core and shim rebuild to `wasm32-wasip2` on every edit, and each live-test invocation shells out to `cargo build` three times (the adapter guest, the eval guest, the driver) through `harness::cargo` before spawning the driver, a native workspace freshness check. Tens of seconds per iteration, all of it waste for a prose-only change: no Rust changed.
2. **Model tax.** The cursor backend spawns a fresh, context-free `cursor-agent` per `create`. A full `build` through the contracts guest is several sub-flow legs plus a verify-repair pass, and the eval guest currently exposes no smaller unit than one whole operation (and only `target.build` at that).

The design goal: an author iterating on one prompt pays exactly one model leg per iteration and zero compilation.

## The prose overlay

The `adapter` crate (`crates/adapter`, the shared guest support every core and shim builds on) gains a **dev-only prose overlay** behind a cargo feature (working name `prose-overlay`), off by default and never enabled by the `build-guests-release` builds that produce the published components (RFC-64: no committed wasm — the release artifacts are pushed by the publish workflow, never checked in):

- The registry lookup (`registry::find` / `registry::body`, wrapped per core by `adapter::embed_registry!`) and the MCP shelf's read path (`adapter::shelf`) first consult an overlay directory under the guest's shared `"."` preopen — `.eval/prose/<adapter-relative path>` — and fall back to the embedded `DOCS` table on a miss. Both the prompt body a judgment leg assembles and the document the spawned agent fetches over MCP resolve through the same lookup, so one overlay covers both.
- Each eval runner seeds the overlay by copying the adapter's `prose/` trees into the scratch project under the registry's key convention (keys omit the `prose/` prefix: `prompts/build.md`, `references/openapi/verifier.md`, …), resolving the `references/spec-runtime` symlinks the same way the build-time embed does. `harness::copy_tree` is the natural home for the seeding leg, but it does not follow directory symlinks today — the seeding path must resolve them, matching `prose`'s walk.
- The overlay is additive per path: a document absent from the overlay serves its embedded body, and a path missing from both keeps the registry's existing fail-loud contract.

With the overlay in place the prose loop collapses to *edit markdown → re-invoke the driver*. No registry codegen, no component build, no cargo — the wasm on disk is already correct because the Rust did not change. To make that literal, the live runner's three `cargo build` legs become skippable when the overlay is active and the built artifacts already exist (the freshness question is answered by construction: prose does not change Rust), so a re-invocation spawns no cargo process at all.

## Layered iteration

Three layers, cheapest first; escalate only when the layer below cannot answer the question:

1. **Native, no model** (seconds): the wasm-free core's tests assert prompt assembly through `adapter::MockModel`, which records every `Request` — "did my prompt edit land in the assembled `system` / `user` text" never needs a component or a model call.
2. **Composed, no model**: the non-ignored `wiring` tests in `evals/live.rs` (scenario seeding + manifest rendering for every scenario, run by ordinary CI) plus the composed-deployment suite in the root `tests/` package (`adapter-tests`, which deploys the built guests in-process via `omnia-testkit` with the model backend stubbed to fail any completion) prove manifest, mount, link, and MCP-route wiring without `cursor-agent`.
3. **Composed, live**: the eval runner with the overlay — the only layer that judges the prose's effect on model output, and under this RFC the only layer that costs a model leg.

## One operation per invocation

The eval guest's argv grows an operation selector. Today it accepts `<adapter-id> <slice> <inputs-dir>` and dispatches only `target.build`; it extends to select any judgment-bearing seam operation — `survey`, `extract`, `guidance`, `build`, `merge` — so a source adapter's prompts are exercisable at all, and a target author studying one prompt drives one operation rather than a whole multi-leg build. Each operation prints its typed answer as one JSON line and carries its outcome in the exit status, matching the existing `build` behavior. (`describe`, the sixth seam operation since RFC-64, is deterministic and model-free — the composed `tests/` suite covers it and the eval harness has nothing to add.)

## Driver ergonomics

- **Model override.** `Request.model` is always `None` from the `adapter` crate's `judgment` helper, and the RFC-61 invariant keeps model ids out of guests. The cursor backend deliberately reads no environment configuration, so the override lands in the eval driver's own runtime assembly: an environment variable (working name `SPECIFY_EVAL_MODEL`) the driver reads and applies as a thin model-context decorator around the cursor backend, filling `model` on requests that carry `None`. `cursor-agent --model` already exists downstream of `Request.model`, so nothing else moves — and the id never enters a guest or the WIT contract.
- **Prebuilt driver.** Landed: the live tests spawn the built `eval-driver` example binary directly. The remaining per-invocation `cargo build` freshness checks are removed by the overlay-active skip above.
- **Watch loop.** A `cargo make` task wraps the runner in a file watch over the adapter's `prose/` trees, so the save-to-report loop is hands-off.

## Scope

- The `prose-overlay` feature in the `adapter` crate (registry + shelf read paths) and the runner support that seeds and enables it, including symlink-resolving overlay seeding and the overlay-active skip of the runner's cargo legs. (Landed.)
- The operation selector on the eval guest. (Landed.)
- The driver-level model override and the watch task. (Landed: `SPECIFY_EVAL_MODEL` in the eval driver, the `eval-watch` cargo-make task. Prebuilt-driver invocation and the wiring-smoke generalization landed with the `evals/live.rs` test runner: each live test spawns the built `eval-driver` example binary, and the non-ignored `wiring` tests smoke every scenario model-free.)

## Out of scope

- **Record/replay of live answers as a regression surface.** Deterministic coverage of judgment legs stays in native core tests per the testing posture (`TESTING.md` in `specify-adapters`); the composed `tests/` suite stubs the model outright, and the larger replay question stays parked in [roadmap.md](roadmap.md) (orchestration trace replay via `ModelDefault` at the `wasi-model` boundary).
- **The eval sweep and grading posture** in [docs/contributing/evals.md](../docs/contributing/evals.md) — unchanged; this RFC accelerates authoring iteration, not the proof surface.
- **The production runtime binary.** The overlay is an eval-deployment affordance, never a mode of the `specify` runtime.
- **Session reuse or prompt caching in the cursor backend.** Session-less `create` is an RFC-61 invariant, not a cost to engineer away here.

## Acceptance criteria

1. A prose-only edit followed by a runner re-invocation compiles nothing: no registry codegen, no crate build, no component build.
2. Under the overlay, both the assembled prompt body and the MCP-served reference reflect the on-disk markdown for every overlaid path — including paths reached through the `references/spec-runtime` symlinks.
3. The overlay feature is compiled out of the `build-guests-release` artifacts the publish workflow pushes, and committed run summaries (`evals/<adapter>/runs/`) grade embedded-build runs only.
4. The eval guest can drive each judgment-bearing `source` and `target` seam operation singly, with a typed JSON answer line and a carrying exit status.
5. A model-id override reaches the cursor backend from the driver's environment without entering any guest or the WIT contract, and without the `omnia-cursor` backend itself growing environment configuration.
6. After the one-time build, a runner invocation with the overlay active spawns no cargo process.

## Risks and invariants

- **Dev/prod divergence.** An overlay run proves prose against the live files, not against the shipped component; graded evidence (the committed run summaries, the eval sweep) comes from embedded builds only. The overlay exists to iterate, never to certify.
- **Fail-loud lookup holds.** The overlay never silently serves an empty or partial document; a miss falls back to the embedded body, and a path absent from both keeps the registry's existing panic contract.
- **The seam does not move.** No change to `specify:adapter` — this is harness and `adapter`-crate surface only, and the operation selector uses the seam operations as they exist.
- **Model ids stay out of guests.** The override is driver-side configuration around the backend, preserving RFC-61's contract-agnosticism invariant and the cursor backend's no-environment-configuration posture.
- **The stale-artifact trap on the skip path.** Skipping the cargo legs is only sound for prose-only edits; a Rust edit with the skip active runs yesterday's component. The skip therefore activates only with the overlay (where prose is what's under test), and the escape hatch is simply re-running without it — the runner never guesses at Rust freshness itself.
