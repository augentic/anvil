# Skills Test Coverage

This page is a coverage audit of every plugin skill against the manual
acceptance scenarios in this repo. It exists to answer one question: for each
`SKILL.md`, is there at least one scenario that exercises the skill's primary
path end to end?

> **2.0 status.** The matrix below tracks 2.0 verbs (`/spec:plan`,
> `/spec:refine`, `/spec:execute`, `/spec:build`, `/spec:merge`, `/spec:drop`,
> `/spec:finalize`) and the deterministic-boundary harness that ships with
> them. The retired 1.x verbs (`change-*`, `specify-define`, `specify-extract`,
> `change-analyze`) appear as historical rows because the cross-repo and plan
> scenario packs they exercised still drive the same skill bodies under the
> renamed verbs. Per-fixture replay coverage is tracked alongside the manual
> scenarios.

The audit is hand-curated and is intended to be re-run whenever a new skill
or scenario lands. The deterministic-boundary fixtures under
[`tests/fixtures/{sources,targets,skills}/`](../../tests/fixtures/) drive
[`tests/cross_repo.ts`](../../tests/cross_repo.ts) on every `make test`; the
manual scenarios under [`tests/cross-repo/`](../../tests/cross-repo/) and
[`tests/plan/`](../../tests/plan/) cover the LLM-driven body bytes that the
harness intentionally does not pin.

## Inputs

- **Skills:** every `plugins/<plugin>/skills/<skill>/SKILL.md` and the
  `argument-hint` declared in its YAML frontmatter (the documented primary
  trigger).
- **Plan-generation scenarios:** [`tests/plan/`](../../tests/plan/) -- two
  manual scenarios that cover plan authoring only (no execution, no push, no
  finalize).
- **Cross-repo scenarios:** [`tests/cross-repo/`](../../tests/cross-repo/) --
  one end-to-end manual scenario that drives `plan` → `execute` → `push` →
  `finalize` across a hub plus two routed projects.
- **Target-local scenarios:**
  [`targets/contracts/tests/`](../../targets/contracts/tests/) --
  five scenarios covering the contracts target's own `refine → build →
  merge` slice loop (the only target-local pack in the repo today).

> **Scenario fixtures (`tests/scenarios/*.json`).** The cleanup-plan brief
> mentions `tests/scenarios/*.json` fixtures, but no such directory or file
> exists in the repository at audit time. The repository's test packs are the
> markdown scenarios listed above; their YAML frontmatter (validated by
> [`scripts/checks/scenarios.ts`](../../scripts/checks/scenarios.ts)) is the
> closest analogue to a fixture. The matrix below uses the markdown scenarios
> as the unit of coverage. See [Plan amendments](#plan-amendments) for
> follow-up.

## How to read the matrix

- **Skill** -- relative link to the `SKILL.md`.
- **Primary trigger** -- the `argument-hint` from the skill's frontmatter, or
  a one-line description of the entrypoint when the skill has no
  `argument-hint`.
- **Plan trace(s)** -- markdown scenarios under `tests/plan/` or
  `tests/cross-repo/` that exercise this skill (directly or as a documented
  sub-step of a larger flow).
- **Scenario fixture(s)** -- target-local scenarios under
  `targets/<name>/tests/` that exercise this skill.
- **Status:**
  - ✓ -- at least one scenario *directly* asserts an artifact, state
    transition, or behavior produced by this skill's primary path.
  - **partial** -- the skill is reached as a sub-step of a larger flow, but
    no scenario asserts the skill's primary artifact in isolation.
  - **gap** -- no scenario reaches this skill on its primary path.

## Coverage matrix

| Skill | Primary trigger / argument-hint | Plan trace(s) | Scenario fixture(s) | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| [`specify-execute`](../../plugins/spec/skills/execute/SKILL.md) | `/spec:execute` (no argument-hint; loop semantics documented in body) | [`tests/cross-repo/scenario.md`](../../tests/cross-repo/scenario.md) (`execute-loop-all-done`) | [`tests/fixtures/skills/execute/*/`](../../tests/fixtures/skills/execute/) | ✓ | Cross-repo scenario asserts the loop terminates; fixture-shape harness covers `input/plan.yaml` + `expected.md` for breakout, build-failure, and workspace cases. |
| [`specify-plan`](../../plugins/spec/skills/plan/SKILL.md) | `<change-name>` | [`tests/plan/single-project.md`](../../tests/plan/single-project.md), [`tests/plan/contract-routing.md`](../../tests/plan/contract-routing.md), [`tests/cross-repo/scenario.md`](../../tests/cross-repo/scenario.md) | -- | ✓ | Single-project covers local-only plans; contract-routing covers cross-repo plan authoring; cross-repo/scenario.md drives the full plan → execute → finalize path. |
| [`client-sow-writer`](../../plugins/client/skills/sow-writer/SKILL.md) | `<slice-dir>` | -- | -- | gap | No scenario produces a SoW from a completed slice. |
| [`contract-asyncapi`](../../targets/contracts/briefs/build.md#asyncapi-sub-flow) | `[slice-dir]` (AsyncAPI authoring) | -- | -- | gap | All five contract scenarios author HTTP/JSON-Schema artifacts; none produce an AsyncAPI document. |
| [`contract-json-schema`](../../targets/contracts/briefs/build.md#json-schema-sub-flow) | `[slice-dir]` (standalone JSON Schema) | -- | [`describe.md`](../../targets/contracts/tests/describe.md), [`design.md`](../../targets/contracts/tests/design.md), [`update.md`](../../targets/contracts/tests/update.md), [`import.md`](../../targets/contracts/tests/import.md), [`source.md`](../../targets/contracts/tests/source.md) | ✓ | Every contracts scenario asserts `contracts/schemas/*.yaml` artifacts and runs `contract-validator-clean`. |
| [`contract-openapi`](../../targets/contracts/briefs/build.md#openapi-sub-flow) | `[slice-dir]` (OpenAPI 3.1) | -- | [`describe.md`](../../targets/contracts/tests/describe.md), [`design.md`](../../targets/contracts/tests/design.md), [`update.md`](../../targets/contracts/tests/update.md), [`import.md`](../../targets/contracts/tests/import.md), [`source.md`](../../targets/contracts/tests/source.md) | ✓ | Every contracts scenario asserts `contracts/http/*.yaml` artifacts. |
| `omnia` target — `build` brief ([`targets/omnia/briefs/build.md`](../../targets/omnia/briefs/build.md)) | crate / test / guest writers + code reviewer, run by `/spec:build` | [`tests/cross-repo/scenario.md`](../../tests/cross-repo/scenario.md) | -- | partial | Cross-repo's backend slice routes to an `omnia@v1` project so the build brief is reached during `/spec:execute`, but no assertion targets the produced crate, tests, guest, or review output (only `execute-loop-all-done`). The bodies of the retired `omnia-{crate,test,guest}-writer` and `omnia-code-reviewer` skills live inside this brief; gap coverage from previous releases (no scenario reviews / asserts generated artifacts) is preserved by this row. |
| [`rt-replay-writer`](../../plugins/rt/skills/replay-writer/SKILL.md) | `<crate-name>` | -- | -- | gap | No scenario captures legacy fixtures or generates regression tests under `tests/data/replay/`. |
| [`rt-wiretapper`](../../plugins/rt/skills/wiretapper/SKILL.md) | `<legacy-dir>` | -- | -- | gap | No scenario instruments a legacy TypeScript repo. |
| `change-analyze` (retired in 2.0; plan-time source enumeration inlined into `/spec:plan`) | n/a — inlined into `/spec:plan` | [`tests/plan/single-project.md`](../../tests/plan/single-project.md), [`tests/plan/contract-routing.md`](../../tests/plan/contract-routing.md) | -- | partial | Plan scenarios assert `discovery.md` exists; per-source enumeration is a sub-step of `/spec:plan` and is not invocable as a standalone slash command in 2.0. |
| [`specify-build`](../../plugins/spec/skills/build/SKILL.md) | `[slice-name]` | [`tests/cross-repo/scenario.md`](../../tests/cross-repo/scenario.md) | [`describe.md`](../../targets/contracts/tests/describe.md), [`design.md`](../../targets/contracts/tests/design.md), [`update.md`](../../targets/contracts/tests/update.md), [`import.md`](../../targets/contracts/tests/import.md), [`source.md`](../../targets/contracts/tests/source.md), [`tests/fixtures/skills/build/`](../../tests/fixtures/skills/build/) | partial | All scenarios with `stages: [refine, build, merge]` reach build; the harness covers `success`, `breakout-from-execute`, and `failure-replay` fixture shapes. No scenario targets build-only resumption against a live LLM body. |
| [`specify-refine`](../../plugins/spec/skills/refine/SKILL.md) | n/a — orchestrates `extract` per source | [`tests/cross-repo/scenario.md`](../../tests/cross-repo/scenario.md) | [`describe.md`](../../targets/contracts/tests/describe.md), [`design.md`](../../targets/contracts/tests/design.md), [`update.md`](../../targets/contracts/tests/update.md), [`import.md`](../../targets/contracts/tests/import.md), [`source.md`](../../targets/contracts/tests/source.md), [`tests/fixtures/skills/refine/`](../../tests/fixtures/skills/refine/) | ✓ | Every contracts scenario uses `entrypoint: /spec:refine`; cross-repo drives refine inside its `/spec:execute` loop; the harness schema-validates Evidence inputs and parses synthesised `expected/spec.md` for closed `Status:` enum across `single-source-intent`, `combined-docs-and-legacy`, `conflict`, `divergence`, and `unknown` cases. |
| [`specify-drop`](../../plugins/spec/skills/drop/SKILL.md) | `[slice-name]` | -- | -- | gap | No scenario exercises drop semantics (slice discarded without merging deltas to baseline). |
| `specify-extract` (retired in 2.0; logic lifted into per-source `extract` briefs invoked by `/spec:refine`) | `<source-path> <slice-dir>` (now a sub-step of `/spec:refine`) | -- | [`source.md`](../../targets/contracts/tests/source.md), [`tests/fixtures/sources/{intent,documentation,code-typescript,screenshots}/`](../../tests/fixtures/sources/) | ✓ | The `contracts-source` scenario asserts artifacts derived from source code; the deterministic-boundary harness schema-validates every Evidence golden against `evidence.schema.json`. |
| [`specify-init`](../../plugins/spec/skills/init/SKILL.md) | `<target>` (with `--workspace` flag) | [`tests/plan/single-project.md`](../../tests/plan/single-project.md), [`tests/plan/contract-routing.md`](../../tests/plan/contract-routing.md), [`tests/cross-repo/scenario.md`](../../tests/cross-repo/scenario.md) | -- | ✓ | Single-project covers `specify init <target>`; contract-routing and cross-repo cover both `--workspace` and target-bound init. |
| [`specify-merge`](../../plugins/spec/skills/merge/SKILL.md) | `[slice-name]` | [`tests/cross-repo/scenario.md`](../../tests/cross-repo/scenario.md) | [`describe.md`](../../targets/contracts/tests/describe.md), [`design.md`](../../targets/contracts/tests/design.md), [`update.md`](../../targets/contracts/tests/update.md), [`import.md`](../../targets/contracts/tests/import.md), [`source.md`](../../targets/contracts/tests/source.md), [`tests/fixtures/skills/merge/`](../../tests/fixtures/skills/merge/) | partial | Reached through `stages: [refine, build, merge]`; the harness covers `success` and `conflict-replay` fixture shapes. No scenario targets merge in isolation against a live LLM body. |
| [`specify-finalize`](../../plugins/spec/skills/finalize/SKILL.md) | `<change-name>` | [`tests/cross-repo/scenario.md`](../../tests/cross-repo/scenario.md) | [`tests/fixtures/skills/finalize/`](../../tests/fixtures/skills/finalize/) | ✓ | Cross-repo scenario drives finalize end-to-end; fixture-shape harness covers single-repo and multi-project-workspace transcripts. |
| `vectis` target — `build` brief ([`targets/vectis/briefs/build.md`](../../targets/vectis/briefs/build.md)) | core / test / iOS / Android writers + reviewers + template-updater, run by `/spec:build` | [`tests/cross-repo/scenario.md`](../../tests/cross-repo/scenario.md) | -- | partial | Cross-repo's mobile slice routes to a `vectis@v1` project so the build brief is reached during `/spec:execute`, but no assertion targets the produced Crux core, iOS / Android shells, generated tests, reviewer output, or template-drift recovery. The bodies of the retired `vectis-{core,test,ios,android}-writer`, `vectis-{core,ios,android}-reviewer`, and `vectis-template-updater` skills live inside this brief; gap coverage from previous releases (no scenario reviews / asserts generated artifacts, no template-drift scenario) is preserved by this row. |
| `vectis` target — `merge` brief ([`targets/vectis/briefs/merge.md`](../../targets/vectis/briefs/merge.md)) | host cap-matrix re-verification (`cargo` / `make build` / `gradlew`), run by `/spec:merge` | [`tests/cross-repo/scenario.md`](../../tests/cross-repo/scenario.md) | -- | partial | Reached as part of the mobile slice's `stages: [define, build, merge]` chain; no scenario asserts the post-merge host-cap-matrix re-verification or the deferred-merge failure-mode handling. |

**Totals (17 skill rows + 3 target-adapter rows)**

| Status | Count | Skills |
| --- | --- | --- |
| ✓ | 8 | `specify-execute`, `specify-plan`, `contract-json-schema`, `contract-openapi`, `specify-refine`, `specify-extract`, `specify-init`, `specify-finalize`. |
| partial | 6 | `change-analyze`, `omnia` target build brief, `specify-build`, `specify-merge`, `vectis` target build brief, `vectis` target merge brief. |
| gap | 5 | `client-sow-writer`, `contract-asyncapi`, `rt-replay-writer`, `rt-wiretapper`, `specify-drop`. |

The four retired Omnia skills (`omnia-crate-writer`, `omnia-test-writer`, `omnia-guest-writer`, `omnia-code-reviewer`) collapsed into a single target-adapter `build` brief; they are tracked as one row above. The eight retired Vectis skills (`vectis-{core,test,ios,android}-writer`, `vectis-{core,ios,android}-reviewer`, `vectis-template-updater`) collapsed into the target-adapter `build` brief; their Vectis-specific post-merge checks moved to the `merge` brief. The retired `vectis-image-layout-inferer` skill body moved to the [`screenshots` source adapter](../../sources/screenshots/adapter.yaml); its source-adapter coverage is exercised by the `tests/fixtures/sources/screenshots/` deterministic-boundary harness.

## Gaps

Each row below describes a skill whose primary path has neither a plan trace
nor a scenario fixture. The row is detailed enough for a follow-up to file an
issue without re-running this audit.

| Skill | Missing coverage | What a future test should exercise |
| --- | --- | --- |
| [`client-sow-writer`](../../plugins/client/skills/sow-writer/SKILL.md) | No scenario produces a SoW. | Scaffold a completed slice (proposal + design + tasks + spec deltas), invoke `/client:sow-writer <slice-dir>`, and assert that a SoW markdown is written to the configured output path with the expected sections (scope, deliverables, acceptance, risks). |
| [`contract-asyncapi`](../../targets/contracts/briefs/build.md#asyncapi-sub-flow) | No AsyncAPI scenario. | Sister scenario to `adapters/contracts/tests/describe.md` that requests an evented contract (channels, messages, bindings) and asserts `contracts/asyncapi/*.yaml` plus `contract-validator-clean`. |
| `omnia` target — `build` brief ([`targets/omnia/briefs/build.md`](../../targets/omnia/briefs/build.md)) | Reached but not asserted; no scenario reviews / asserts generated Omnia artifacts. | Targeted fixture under `tests/fixtures/targets/omnia/` runs the build brief end-to-end against a fixed Specify slice and asserts the produced `Cargo.toml`, `src/lib.rs`, provider trait composition, error mapping, guest wiring (HTTP / topic / WebSocket), MockProvider integration tests with `REQ-XXX` traceability, and the code reviewer's adversarial-review summary (both default and `fix` positional). Covers create + update modes for the crate-writer, drift detection for the test-writer, and code-review remediation for the auto-fix path. |
| [`rt-replay-writer`](../../plugins/rt/skills/replay-writer/SKILL.md) | No replay scenario. | Scenario that supplies captured fixtures under `tests/data/replay/` for a generated crate, invokes `/rt:replay-writer <crate-name>`, and asserts replay tests compile and pass. |
| [`rt-wiretapper`](../../plugins/rt/skills/wiretapper/SKILL.md) | No wiretap scenario. | Scenario that clones a small fixture TypeScript service, runs `/rt:wiretapper <legacy-dir>`, and asserts wiretap code compiles, fixture JSON appears for declared entry points, and adapters wire the entrypoint without breaking the original `tsc` build. |
| `change-analyze` (retired in 2.0; plan-time adapter inference inlined into `/spec:plan`) | Reached only via plan pipeline. | Direct invocation scenario: `/spec:plan source documentation=<path>` against a documentation tree vs. `/spec:plan source code-typescript=<path>` against a code tree, asserting the emitted source-adapter summaries differ on the `kind` axis and that `discovery.md` is structured per the schema. |
| [`specify-build`](../../plugins/spec/skills/build/SKILL.md) | Reached only as part of `[define, build, merge]`. | Mid-flight build scenario: pre-create a slice with proposal + spec but no implementation, run `/spec:build <slice-name>`, and assert the build resumes and completes without re-running define. |
| [`specify-drop`](../../plugins/spec/skills/drop/SKILL.md) | No drop scenario. | Scenario that creates a slice, drops it via `/spec:drop <slice-name>`, and asserts (a) baseline specs are unchanged, (b) the slice directory is moved to `archive/dropped/`, and (c) any plan entry transitions to `dropped`. |
| [`specify-merge`](../../plugins/spec/skills/merge/SKILL.md) | Reached only via stages chain. | Targeted merge-only scenario: pre-stage a slice with completed build artifacts, run `/spec:merge <slice-name>`, and assert the baseline diff and the move-to-archive transition. |
| `vectis` target — `build` brief ([`targets/vectis/briefs/build.md`](../../targets/vectis/briefs/build.md)) | Reached but not asserted; no scenario reviews / asserts generated Vectis artifacts or exercises template drift. | Targeted fixture under [`tests/fixtures/targets/vectis/`](../../tests/fixtures/targets/vectis/) runs the build brief end-to-end against a fixed Specify slice and asserts the produced Crux shared crate (`Cargo.toml`, `src/app.rs`, adapter wiring, UniFFI exports), the regenerated `composition.yaml` (per [`expected/composition.yaml`](../../tests/fixtures/targets/vectis/task-list/expected/composition.yaml)), the iOS SwiftUI shell (`iOS/project.yml`, generated views, asset catalog), the Android Compose shell (Gradle build, generated composables, drawables), the spec-traced Crux tests (synchronous `#[test]` per scenario with `/// Spec:` comments), the core / iOS / Android reviewer summaries, and the template-drift recovery path against a deliberately broken Vectis template (e.g. a stale Crux pin). |
| `vectis` target — `merge` brief ([`targets/vectis/briefs/merge.md`](../../targets/vectis/briefs/merge.md)) | Reached only via stages chain; host cap-matrix re-verification is not asserted. | Targeted scenario that pre-stages a Vectis slice with a clean build, runs `/spec:merge <slice-name>`, and asserts the post-merge `cargo check --workspace`, `cargo clippy --workspace`, `cargo test --workspace`, `cd iOS && make build`, and `cd Android && make build` invocations all exit clean against the merged baseline, plus the deferred-merge transition when one of those host caps regresses. |

## Plan amendments

The CL-S06 brief asks the auditor to *open issues in the parent repo* for
each gap. This run cannot do that:

1. **Auth context.** The runner does not have authenticated `gh` access for
   the upstream repository, and creating issues from an automated audit run
   would mutate an external system without explicit user direction.
2. **Tracking surface.** The repository's working tracking surface for skill
   coverage is this audit page; opening one issue per gap would duplicate the
   matrix without adding fidelity.

Recommended amendments to the cleanup plan for the next maintainer:

- Replace step 3 of CL-S06 ("Open issues in the parent repo for any skill
  without at least one trace covering the primary path") with: "Document
  every gap in `docs/contributing/skills-test-coverage.md`. A separate,
  authenticated follow-up may bulk-open issues from the [Gaps](#gaps)
  section."
- Replace the second acceptance criterion ("An issue is open for every gap")
  with: "Every gap row in the matrix contains enough detail
  (`Skill | Missing coverage | What a future test should exercise`) for a
  follow-up to file an issue without re-running the audit."
- Acknowledge that `tests/scenarios/*.json` does not exist; the brief should
  call out `tests/plan/*.md`, `tests/cross-repo/*.md`, and
  `adapters/<cap>/tests/*.md` (the markdown scenario packs the repo
  actually ships) instead.

When the gap list is bulk-filed as issues by a follow-up, link each issue
back to the matching row of this matrix so the audit stays the source of
truth.
