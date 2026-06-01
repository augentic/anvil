# Vectis target — build brief

`/spec:build` reads this brief when the active in-progress slice declares `target: vectis`. The brief drives the host workflow that produces a buildable cross-platform application (Crux shared core + per-platform shells) from the slice's already-synthesised `spec.md` and `design.md`. It owns three responsibilities the legacy skill-per-step layout did not pin down in one place:

1. **`composition.yaml` regeneration.** Synthesis does not write `composition.yaml`. This brief regenerates it from `spec.md` + `design.md` (which already carry every upstream spatial / structural claim synthesis folded in from source adapters) at the start of each build, alongside the code it accompanies. `merge` lands the regenerated file together with the implementation code.
2. **Inline phase sub-briefs.** The legacy `/vectis:core-writer`, `/vectis:test-writer`, `/vectis:ios-writer`, `/vectis:android-writer`, `/vectis:core-reviewer`, `/vectis:ios-reviewer`, `/vectis:android-reviewer` are retired as separate skills; their bodies now live in phase sub-briefs under [`build/`](build/).
3. **Operator-curated inputs are read, never authored.** `tokens.yaml`, `assets.yaml`, and `components.yaml` are operator-curated and consumed as build inputs; the brief never invents or restates their contents. The component catalog (`.specify/design-system/components.yaml`) is the third design-system input, joining `tokens.yaml` and `assets.yaml`. When present, the build reads confirmed entries and factors shared component code per in-scope shell tree; when absent, no component factoring occurs.

The Vectis target stays three-capability (`shape` / `build` / `merge`) — there is **no** fourth `refine` slot. Composition regeneration is part of `build`.

## Inputs

The brief runs against the build request the CLI prepared at `.specify/slices/<slice>/build/request.yaml`; consume its `inputs` manifest rather than relying on convention. Every artifact path resolves against `inputs.root` (the slice tree).

- `inputs.artifacts.proposal` (`proposal.md`) — `## Platforms` scope (`core` / `ios` / `android`) and screen / interaction intent.
- `inputs.artifacts.specs[]` (`specs/<unit>/spec.md`) — behavioural requirements per unit: screen titles, scenarios, platform-specific behaviour, validation rules.
- `inputs.artifacts.design` (`design.md`) — domain model: ViewModel / `Event` / `Route` variants, per-page view structs, capability matrix.
- `inputs.artifacts.tasks` (`tasks.md`) — phase-completion tracking.
- `inputs.artifacts.additional[]` — the three design-system inputs declared by [`adapter.yaml`](../adapter.yaml), **all optional** (`required: false`), each with an explicit absent-fallback:
  - `tokens.yaml` — design tokens; absent → HIG (iOS) / Material 3 (Android) theme fallback in the shell writers.
  - `assets.yaml` — asset inventory; the composition validator's `tokens` / `assets` modes run only when the respective file is present.
  - `components.yaml` — the opt-in component catalog (surfaced as `CATALOG_PATH`); absent → no component factoring.

## Standard arguments

All phase sub-briefs assume these symbols are resolved by `/spec:build` before the sub-agent fan-out:

| Symbol | Meaning |
| --- | --- |
| `SLICE_ID` | The active slice name (`specrun plan next` output, or `specrun slice` argument). |
| `SLICE_DIR` | `.specify/slices/<SLICE_ID>/`. |
| `UNIT_NAME` | The single unit spec folder under `SLICE_DIR/specs/`. When the slice carries multiple units, iterate the per-unit phase sub-briefs in declaration order. |
| `PROJECT_DIR` | The target project root (single-repo mode) or the resolved workspace slot (workspace mode). |
| `IOS_SHELL_DIR` | `${PROJECT_DIR}/iOS` (only when `ios` is in scope). |
| `ANDROID_SHELL_DIR` | `${PROJECT_DIR}/Android` (only when `android` is in scope). |
| `APP_NAME` | The Xcode target / Swift source folder name (derived from `design.md`'s `App` struct name). |
| `CATALOG_PATH` | `${PROJECT_DIR}/.specify/design-system/components.yaml` when present. Optional — absent means no component factoring. |

## Platform detection

Read `${SLICE_DIR}/proposal.md` `## Platforms` to determine scope. Valid Vectis platform tokens are `core`, `ios`, `android`, and the deferred `web`. Token / asset / layout work is **input context**, never a platform. Process platforms in dependency order:

1. `core` first — shells depend on the core.
2. `ios` and `android` shells — independent of each other; their **generation** phases can run in parallel; their **verify** phases are serial because they share the same Cargo workspace lock.
3. `web` — deferred.

If the proposal lists `core` only, skip the iOS and Android phase sub-briefs wholesale; this is a backend-only build.

## Phase order

1. Load [`build/composition.md`](build/composition.md) — regenerate `composition.yaml` from `spec.md` + `design.md` and run the deterministic validator gate.
2. Load [`build/core/write.md`](build/core/write.md) — generate / update the Crux shared core.
3. Load [`build/test.md`](build/test.md) — generate / update Crux tests, then run the core verify-repair loop (max 3 iterations).
4. (When `ios` is in scope) Load [`build/ios/write.md`](build/ios/write.md) — generate / update the SwiftUI shell, then its verify loop.
5. (When `android` is in scope) Load [`build/android/write.md`](build/android/write.md) — generate / update the Compose shell, then its verify loop.
6. Load [`build/core/review.md`](build/core/review.md) and, when in-scope, [`build/ios/review.md`](build/ios/review.md) and [`build/android/review.md`](build/android/review.md). Reviewers run in parallel.
7. Run § Consolidate review findings.
8. Mark `tasks.md` checkboxes complete as each phase lands, then write the build report (§ Build report). The brief never transitions the slice — the CLI's `--phase finalize` validates the report and owns the `Refined → Built` transition.

## § Sub-agent delegation contract

Each writer / verifier / reviewer phase sub-brief runs in its **own sub-agent** with a clean context window. `/spec:build` coordinates the sequence but does not execute phase bodies inline.

**Inputs (orchestrator → sub-agent):** `task` (one of `core-writer`, `test-writer`, `ios-writer`, `android-writer`, `core-reviewer`, `ios-reviewer`, `android-reviewer`), `arguments` (standard arguments above), `mode` (`create`, `update`, or `repair` — decided by the orchestrator from on-disk inspection), `skip_verification` (true for shell writers; verification runs in a dedicated sub-agent afterward), `artifact_paths` (paths to `spec.md`, `design.md`, `proposal.md`, regenerated `composition.yaml`, sibling `tokens.yaml` / `assets.yaml` when present, and `components.yaml` when `CATALOG_PATH` exists), `orchestrated` (reviewer sub-agents only; signals that the reviewer is running inside a build phase so its `design_findings` should flow into § Consolidate review findings — reviewers always return `design_findings` for the parent to consolidate, never auto-spawn follow-up slices), `extra_context` (phase-specific: error output for `repair` mode, baseline test log for regression checks, prior phase warnings).

**Outputs (sub-agent → orchestrator):** `status` (`success` / `failure` / `pending`), `files_modified`, `verification` (inline result when the sub-agent ran one), `errors`, `warnings`, `design_findings` (reviewers only; empty list when nothing surfaced).

### Why verify is serial; review is parallel

The iOS verify pipeline (`make build` → cargo-swift) and the Android verify pipeline (`make build` → uniffi typegen, `gradlew :shared:cargoBuild`) both invoke `cargo` against the same shared Rust workspace. Cargo uses a workspace-level lock file, so concurrent invocations serialise on the lock rather than running in parallel. The reviewers are pure code-analysis agent teams; they use different formatters (`swiftformat` vs Kotlin) and never invoke `cargo`, Gradle, or Xcode. With no shared mutable state and no build-tool contention, they are safe to run concurrently.

## § Consolidate review findings

When all in-scope reviews complete:

1. **Merge findings.** Combine `design_findings` from each reviewer into a single list. Deduplicate universal findings (UNI-prefixed) that both reviewers flagged with identical check IDs and matching evidence — keep the higher-severity instance. Platform-specific findings (CRX-, LOG-, GEN-, IOS-, SWF-, AND-, KTL-, INT-prefixed) are always distinct.
2. **Empty list.** Skip the rest of this section.
3. **Validate classifications.** Each finding already carries `code-fix` or `spec-change`. Treat that as the source of truth. Resolve disagreements between platforms by applying: spec is clear but code is wrong → `code-fix`; spec is silent, ambiguous, or problematic → `spec-change`.
4. **Surface findings.** Findings flow to the operator alongside the build outcome. Cross-platform follow-up work is queued as a new slice via the operator's normal `/spec:plan` flow rather than letting reviewers spawn slices directly — the legacy "reviewer auto-creates a Specify change" path is retired in 2.0.

## § Deterministic review

The per-platform reviewers above ([`build/core/review.md`](build/core/review.md), [`build/ios/review.md`](build/ios/review.md), [`build/android/review.md`](build/android/review.md)) carry the model-assisted surface — specialist + antagonist judgment per [`agent-teams.md`](../references/agent-teams.md). `specrun lint --format json` is the **deterministic complement**. It resolves applicable rules via `specrun rules export`, evaluates declarative `deterministic_hints`, and emits findings in the same `LintFinding` shape (`rule-id`, `fingerprint`, severity, `evidence`) operators already see in that export. The two surfaces are layered, not alternatives — model-assisted judgment sits on top of the deterministic scan.

Per [Standards layer](../../../../docs/explanation/standards-layer.md), deterministic findings may block CI but never transition plan entries, slices, or changes. CI wiring is consumer-project policy, not adapter policy; this brief acknowledges the surface and links out for the contract.

## § Template / version-pin drift handling

The Vectis scaffold tool (`specrun tool run vectis -- scaffold ...`) is render-only and ships with embedded version pins. Upstream bumps (Crux core, uniffi, AGP / Gradle, cargo-swift, Xcode) can break a freshly rendered scaffold even when the rest of the slice is correct. Detect this when a verify-repair loop fails repeatedly with cargo / Gradle / Xcode errors that look like API renames, missing imports, or toolchain mismatches rather than feature-level bugs. When detected, do **not** auto-fix in-band: record the failing combo (caps + shells), the failing host step, and the load-bearing error line, then mark the build outcome as `deferred` with a template-drift signal. The operator opens a separate slice rooted in the CLI repo to bump the embedded `versions.toml`.

Symptom triage table: [`../references/known-drift.md`](../references/known-drift.md) — start here before escalating; if the reproduced failure matches a listed item, the operator can route directly to that item's playbook in the host-side template-updater workflow.

## § Phase outcome contract

> See [Phase outcome contract](../../../../plugins/spec/references/phase-outcome-contract.md).

The `build` phase concludes with exactly one of `success` / `failure` / `deferred`:

- **success** — every in-scope verify-repair loop returned `success` within its iteration budget, and the orchestrator has both regenerated `composition.yaml` (or skipped it for a core-only slice) and the implementation code under `${PROJECT_DIR}`. Write a `status: success` build report (§ Build report); finalize owns the lifecycle transition.
- **failure** — any verify-repair loop exhausted its iterations, or the composition validation gate ([build/composition.md](build/composition.md)) failed and could not be repaired. Surface the load-bearing error line as `--summary` and the full output through `--context`, and write a `status: failure` build report with the blocking findings mapped where possible; the merge brief refuses to run while the slice is in this state.
- **deferred** — a host prerequisite is missing (Java 21, Android SDK, Rust Android targets, `cargo-swift`, Gradle wrapper, Xcode CLT) or a known-drift template / pin issue surfaced and operator judgement is required. Surface the unresolved prerequisite or drift signal as `--summary` and write a `status: failure` build report (the report carries only `success` / `failure`; `deferred` is the operator-facing stop signal, not a built slice).

## Build report

When the algorithm resolves, write a schema-valid build report to `.specify/slices/<slice>/build/report.yaml` (the handoff envelope's `report` field). This is the brief's final deliverable. The brief never transitions the slice lifecycle — the CLI's `--phase finalize` validates the report and owns the `Refined → Built` transition.

```yaml
version: 1
slice: <slice-name>     # matches the build request's `slice`
target: vectis@v1       # this adapter at its manifest version
status: success         # or: failure
findings: []            # structured diagnostics; default []
```

**Success vs failure findings rule.** A `status: success` report carries an empty `findings[]` or only non-blocking findings (`suggestion` / `optional`); the CLI rejects a `success` report carrying any blocking (`critical` / `important`) finding. A `status: failure` report populates `findings[]` with the blocking violations the target can map from the composition validator gate and the per-platform verify-repair output, and leaves `findings: []` when no specifics are mappable.

- **Clean build** — composition regenerated and the validator gate ([build/composition.md](build/composition.md)) passed (or was skipped for a core-only slice), every in-scope verify-repair loop (core, iOS, Android) returned `success` within its budget, and § Consolidate review findings produced no blocking findings → `status: success`, `findings: []` (or only advisory `suggestion` / `optional` findings).
- **Unresolved build** — a verify-repair loop exhausted its iterations, the composition validator gate failed unrepaired, or a host prerequisite / known-drift signal forced a `deferred` outcome → `status: failure` with blocking findings mapped where possible.

Each `findings[]` item validates against `schemas/diagnostics/diagnostic.schema.json` (the structured-diagnostic shape distributed with the CLI; required fields include `id`, `title`, `severity`, `source`, `artifact`, `evidence`, `impact`, `remediation`, `fingerprint`). Map vectis's composition-validator, cargo / Gradle / Xcode verify, and review findings into that shape, carrying detail under `evidence.kind: structured` with `target-adapter: vectis`.

## Notes for downstream phases

- **`composition.yaml` is a build output.** It lives at `${SLICE_DIR}/composition.yaml` after this brief succeeds; the merge brief lands it into the baseline alongside the code. Operator-curated `tokens.yaml` / `assets.yaml` are also read by `merge`; the merge brief re-runs `specrun tool run vectis -- validate composition` against the merged baseline so cross-artifact regressions are caught even when the current slice only touched code.
- **Do not write `composition.yaml` into `.specify/specs/`.** That is `specrun slice merge`'s job, atomically, alongside the spec / design deltas.
- **Operator-curated inputs.** `tokens.yaml` and `assets.yaml` updates accompany the slice when the operator edits them; the merge brief promotes those edits into `design-system/tokens.yaml` / `design-system/assets.yaml` (or slice-local equivalents) using the same delta merge path as the spec deltas. The component catalog (`CATALOG_PATH`) is project-level and not slice-local; it is read as-is at build time and does not participate in the merge delta path.
