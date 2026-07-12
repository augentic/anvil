# RFC-62: Self-Contained Engine Testing

> **Status: Draft.** Specify is tested as a self-contained workflow engine against its own WIT contract. Tests do not resolve, build, inspect, or otherwise depend on `specify-adapters`. Owns: the engine test boundary, the combined fixture adapter, and the local developer loop

## Abstract

Specify's test surface has grown into a scenario catalogue, profile matrix, live-quality runner, report format, archived run records, reference fixtures, cross-repository developer script, and multiple execution harnesses. That machinery is disproportionate to the engine contract and obscures the fast Rust-native integration tests that should provide most confidence.

This RFC replaces that model with three small layers:

1. fast Rust-native integration tests for workflow behavior;
2. one composed WebAssembly smoke test for the WIT and runtime boundary;
3. one explicit live-model workflow test for model connectivity, reconciliation, and synthesis.

A single Specify-owned fixture adapter supplies both source and target behavior. Its logic is ordinary native Rust. A thin `wasm32-wasip2` shim exports both WIT interfaces from one component. The ordinary developer loop remains self-contained, model-free, and free of Wasmtime.

## Problem

The current quality architecture models testing as a reusable product of its own:

- `quality/scenarios/` defines a canonical YAML catalogue;
- `crates/scenario` defines profiles, assertions, grading, reports, bundles, and semantic judgment;
- `harness/quality` interprets that vocabulary for repeated live trials;
- `harness/replay` reuses the quality executor for composed tests;
- `quality/runbooks/`, `quality/profiles/`, and `quality/runs/` maintain parallel operator and audit surfaces;
- `quality/fixtures/reference/` retains families that its own README classifies as non-executable;
- `scripts/dev.rs` makes a sibling adapter checkout part of the normal engine developer loop;
- documentation assigns part of Specify's confidence to tests outside this repository.

This creates four problems.

First, engine correctness is no longer self-contained. Specify owns a generic workflow over the `specify:adapter` WIT package and must prove that workflow with this repository alone.

Second, most workflow behavior is native Rust but the test architecture emphasizes profiles and harnesses. The workflow crate already keeps WIT bindings outside its core and expresses source, target, model, filesystem, and resolver dependencies as capabilities. Native integration tests should be the primary end-to-end surface.

Third, the catalogue is substantially larger than its automated execution surface. Prompt-driven runbooks, Bash evaluators, semantic rubrics, and historical reports should not masquerade as one executable contract.

Fourth, the local loop is slower and harder to understand than the engine warrants. A contributor should not need a sibling checkout, adapter builds, live credentials, a scenario profile, or a composed runtime to prove an ordinary workflow change.

## Boundary

Specify is a single-repository engine. For testing purposes it has no knowledge of any external adapter repository.

The engine owns:

- the source and target WIT interfaces;
- adapter resolution and dispatch by configured component identity;
- plan authoring and lead reconciliation;
- evidence extraction and slice synthesis;
- build orchestration;
- deterministic merge and lifecycle behavior;
- model invocation through the model capability;
- the workflow guest's thin WIT and command shims.

The engine does not own:

- the behavior or output quality of any real adapter;
- compatibility with an external repository's HEAD, layout, fixtures, or release build;
- adapter-specific prompts, references, generated projects, or semantic rubrics;
- a general evaluation framework for other products;
- live transport and credential behavior of the shipped model backend — the composed smoke proves the guest-to-host model wiring with deterministic answers; the cursor-bound client is owned upstream by omnia.

External adapters prove their own behavior against the published WIT package. Specify proves that any conforming component can be loaded and called.

## Decision

### Rust-native integration is primary

Workflow tests run against the native `workflow` crate through its public operations and capability traits. A native fixture provider supplies:

- the fixture adapter's source implementation;
- the same fixture adapter's target implementation;
- upstream scripted model support;
- temporary filesystem state;
- the real workflow operations, artifact parsers, schemas, lifecycle transitions, and merge implementation.

The native path executes the same non-WASM Rust code used by the workflow guest. The guest remains binding and projection glue only.

Tests are organized by engine concern:

- `workflow` proves the complete `init → author → approve → execute` loop;
- `reconciliation` proves that surveyed leads become complete, non-duplicated slice assignments;
- `synthesis` proves that extracted evidence and target guidance become valid slice artifacts with provenance;
- `merge` proves baseline delta application, conflict handling, and lifecycle effects;
- `adapter` proves source and target operation dispatch;
- `judgment` proves model request construction, schema gating, repair, and typed failure propagation.

Focused integration tests cover failures that the happy path cannot distinguish. The same assertion is not repeated at every layer.

### One fixture adapter core implements both axes

Replace the separate, guest-only echo implementations with one test-support crate containing native Rust logic for both axes.

The fixture is small but observable:

- `survey` returns controlled leads suitable for single-slice and multi-slice reconciliation, including the adversarial lead set the live test requires;
- `extract` returns controlled evidence with stable authority, claim identifiers, and provenance anchors, including one authority disagreement and one deliberate evidence gap;
- source and target metadata return stable, axis-appropriate records;
- `guidance` returns a short deterministic instruction;
- `build` writes or reports one observable fixture output and returns a valid report;
- `merge` implements the WIT export even where the engine merge path remains deterministic;
- each operation can return a typed failure without requiring another component.

Native tests call this implementation through `SourceSeam` and `TargetSeam`; they do not duplicate fixture behavior in mocks.

### Add a combined WIT world

Retain the axis-specific production worlds and add the combined world already sketched in `wit/specify.wit`:

```wit
world adapter {
  export source;
  export target;
}
```

Existing source-only and target-only components remain valid. External adapters are not required to implement both interfaces. The combined world is additive and lets one self-contained component prove both workflow imports.

The fixture package produces one `fixture_adapter.wasm` artifact. Its `wasm32` module contains only:

- `wit_bindgen` generation for the combined world;
- conversion between generated WIT values and native fixture types;
- delegation to the native fixture implementation;
- the minimal HTTP export required by deployment validation.

If host dispatch requires axis-qualified identities, the same component artifact may be bound to `source:fixture` and `target:fixture`. This remains one implementation and one compiled component. Requiring one runtime instance or adding runtime alias support is out of scope.

### One native full-loop test

The central test:

1. initializes a temporary project with the fixture target;
2. binds the fixture source;
3. surveys controlled leads;
4. invokes the scripted model to reconcile leads into slices;
5. approves and executes the plan;
6. extracts fixture evidence;
7. invokes the scripted model to synthesize proposal, spec, design, and tasks;
8. calls fixture target guidance and build;
9. merges the slice;
10. asserts drained completion, correct lifecycle states, visible baseline output, complete provenance, and the fixture build output.

Model responses are concise checked-in test inputs, not a replay catalogue or scenario format.

Focused native tests retain coverage of malformed model responses, repair exhaustion, adapter failures, interruption, and merge conflicts where existing integration tests do not already own them.

### One composed smoke test

WebAssembly testing is a boundary test, not a second workflow matrix.

One composed test builds `specify.wasm` and `fixture_adapter.wasm`, hosts them in one temporary deployment, binds both adapter axes, and uses deterministic model answers. It asserts only facts unique to the composed boundary:

- the combined component validates and loads;
- source and target exports dispatch;
- metadata is readable through both axes;
- the workflow guest calls the model host;
- writable preopens and component cache are wired;
- the loop reaches the same externally visible terminal state as the native test.

The composed test does not depend on the scenario crate, live-quality runner, semantic judge, report bundles, external components, or a sibling checkout.

### One explicit live-model test

Deterministic answers prove orchestration and validation, but not that a configured model can produce useful reconciliation and synthesis.

Provide one ignored, operator-invoked native integration test using the same fixture workflow. It calls the configured live model, reconciles a small non-trivial lead set, synthesizes one or two slices, and runs deterministic schema, coverage, provenance, and lifecycle validators.

The lead set must earn the word non-trivial, because validator-clean output over an easy input demonstrates schema conformance rather than judgment. It contains at least one cross-source overlap that a correct reconciliation merges into a single slice, one authority disagreement that a correct synthesis surfaces as a divergence or conflict tag, and one evidence gap that a correct synthesis marks unknown rather than inventing. The deterministic validators then discriminate: coverage catches an unmerged overlap, provenance catches an invented requirement, and the tag checks catch a suppressed disagreement.

The pass condition is structural and behavioral, not a second model judging the first. It runs once by default. The test reports the repair count of each judgment leg without asserting on it: a leg drifting from zero repairs toward the repair budget is the early warning that a prompt or schema change degraded the model's first answer, visible before it becomes a failure. Repeated trials, semantic scoring, judge models, token reports, immutable run archives, and generalized rubric machinery remain out of scope.

The live test binds the model capability natively. The shipped binary's WASI model host path is proven by the composed smoke with deterministic answers; live transport and credential behavior of the cursor backend belongs upstream, exactly as adapter output quality belongs to adapter authors.

Manual means scheduled by convention, not never. Operators run `cargo make test-live` before tagging a release and after any change to the judgment prompts or the answer schemas. The cadence is documented in the contributor guide, not automated: ordinary CI still never calls a live model.

## Developer loop

The command surface has three local rungs:

```shell
cargo make test       # fast native integration tests; model-free and no Wasmtime
cargo make test-wasm  # build two guests and run the composed smoke test
cargo make test-live  # run the explicit live-model workflow test
```

`cargo make check` remains the pre-commit gate: formatting, linting, native tests, documentation, and a compile-only `wasm32-wasip2` check. It does not host components or call a live model.

`cargo make ci` remains the full self-contained repository gate. It requires no sibling checkout, external component build, or model credentials.

The default loop compiles only default workspace members, avoids Wasmtime and component builds, avoids CLI subprocess matrices where native operations expose the behavior, and avoids a bespoke developer script where cargo-make is sufficient.

The implementing pull request records cold and warm timings before and after migration. The native loop must be materially faster than the current cross-repository `dev check`.

## Resulting repository shape

Retain:

- `crates/workflow/tests/` for native workflow integration;
- existing crate integration tests for artifacts, schemas, transport, and errors;
- `tests/framework/` for remaining shipped prose, schema, and manifest checks;
- one fixture-adapter package with native core and combined-world shim;
- one small composed test package;
- minimal model inputs colocated with their tests;
- merge goldens where exact structure remains the contract.

Remove:

- `crates/scenario`;
- `harness/quality`;
- the scenario/profile/assertion/report/bundle abstractions;
- `quality/scenarios/`, `runbooks/`, `profiles/`, `rubrics/`, `runs/`, and non-executable reference fixtures;
- scenario and run-record framework checks;
- cross-repository paths and adapter-specific commands from `scripts/dev.rs`;
- documentation that assigns engine correctness to a sibling repository;
- scheduled workflows used only by removed quality machinery.

Move the deterministic model answers beside their consuming tests, then remove `quality/` entirely.

Remove the resolver's sibling `specify-adapters` development probe. Bare-name resolution may use only the documented local project cache, configured store, or an explicitly supplied component path.

## Migration plan

### Phase 1: Establish native confidence

1. Extract echo behavior into a native fixture-adapter core.
2. Implement native `SourceSeam` and `TargetSeam` bridges.
3. Add upstream scripted model support to the owning integration-test package.
4. Add the native full-loop characterization test.
5. Add only the focused failure cases not already covered.
6. Record coverage and loop timings.

No old harness is removed until this phase proves the relied-upon workflow behavior.

### Phase 2: Collapse the WebAssembly fixture

1. Publish the additive combined `adapter` world.
2. Replace `echo_source.wasm` and `echo_target.wasm` with `fixture_adapter.wasm`.
3. Keep all behavior in the native core and make the guest a delegation shim.
4. Replace `harness/replay`'s scenario interpreter with one direct composed test.
5. Keep axis-specific worlds covered by WIT compilation; do not add duplicate runtime loops.

### Phase 3: Replace live quality

1. Add the ignored native live-model test over the adversarial fixture lead set.
2. Expose it as `cargo make test-live`.
3. Retain the temporary path on failure.
4. Report per-leg repair counts in the test output.
5. Document the operator cadence: before a release tag and after judgment-prompt or answer-schema changes.
6. Remove repeated trials, judge scoring, bundles, verification hooks, and profile selection.

### Phase 4: Delete the testing framework

1. Remove `crates/scenario`, `harness/quality`, and `quality/`.
2. Remove scenario framework tests and workspace dependencies.
3. Rename and simplify `harness/replay` around the composed boundary.
4. Delete or localize the cross-repository developer script.
5. Remove the resolver sibling fallback.
6. simplify Cargo Make tasks and CI workflows.

### Phase 5: Rewrite documentation

1. Rewrite `docs/standards/testing.md` around native integration, one WASM smoke, and one live test.
2. Rewrite `docs/contributing/dev-loop.md` as a self-contained loop.
3. Fold durable quality guidance into those documents and remove profile documentation.
4. Update `AGENTS.md`, `README.md`, architecture, release docs, and framework checks so no engine test instruction requires `specify-adapters`.
5. Document the combined WIT world without changing the obligations of source-only or target-only adapters.

## Acceptance criteria

1. A fresh Specify checkout runs its full gate without a sibling checkout or external adapter component.
2. `cargo make test` proves the complete workflow natively, including model-backed reconciliation and synthesis through deterministic model support.
3. The native full-loop calls both source operations and target guidance/build, then observes merge and drained completion.
4. One fixture core supplies both axes without native/WASM behavior duplication.
5. One `fixture_adapter.wasm` exports both interfaces through the combined world.
6. One composed smoke proves both interfaces can be loaded and called.
7. One explicit live test proves real model connectivity and validator-clean reconciliation and synthesis over a lead set containing at least one cross-source overlap, one authority disagreement, and one evidence gap, and reports per-leg repair counts.
8. Default tests do not compile Wasmtime, build components, call a live model, inspect another repository, or spawn a cross-repository harness.
9. `crates/scenario`, `harness/quality`, and `quality/` are removed.
10. No test documentation assigns engine correctness to an external adapter repository.
11. The resolver has no hard-coded sibling adapter checkout.
12. Coverage of still-live workflow code does not fall.
13. Cold and warm loop timings improve materially.
14. `cargo make ci` and `cargo make test-wasm` pass in this repository alone.

## Risks and mitigations

- **The combined world may look mandatory.** Keep the source-only and target-only worlds and document the combined world as additive.
- **Native tests may miss conversion defects.** The composed smoke calls every source and target operation through generated bindings.
- **One happy path may hide failures.** Keep focused native cases for typed adapter failures, malformed model answers, repair exhaustion, interruption, and merge conflict.
- **Scripted answers may overstate model quality.** The explicit live test exercises the same workflow over an adversarial lead set and accepts only validator-clean artifacts.
- **The live test may be flaky or expensive.** Keep it manual, single-trial, small, and structurally graded.
- **A single pass may hide prompt drift.** The bounded repair loop can absorb a degrading first answer silently; reported per-leg repair counts surface the drift before it exhausts the budget.
- **A manual test can rot.** The documented cadence ties `cargo make test-live` to release tagging and to judgment-prompt or answer-schema changes, so the rung runs when its answer can change.
- **Deleting reports may lose context.** Git preserves removed records; tests retain executable inputs and failures rather than a second audit system.
- **Test support may leak into production APIs.** Use existing public capabilities; do not widen workflow APIs only for tests.

## Non-goals

- Evaluating arbitrary adapter quality.
- Testing generated applications or adapter-specific outputs.
- Maintaining a cross-repository compatibility matrix.
- Providing a scenario DSL, profile runner, report bundle, rubric engine, or evaluation archive.
- Running live models in ordinary CI.
- Replacing focused integration tests with one monolithic test.
- Requiring external adapters to combine interfaces.
- Changing the generic runtime to guarantee one in-memory fixture instance.

## Open questions

1. Should the combined world be named `adapter` or `test-adapter`?
2. Which public operation is the narrowest native full-loop entry point without widening an API?
3. Can upstream model test support provide scripted answers directly to native workflow tests?
4. Is the composed smoke fast enough for every push after timings are measured?
5. Should `scripts/dev.rs` be deleted or reduced to local aliases?
