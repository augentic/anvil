---
id: build
description: Drive Vectis code generation for an active slice — regenerate `composition.yaml`, write the Crux shared core, write each in-scope shell, run host verification, and review.
---

# Vectis target — `build`

`/spec:build` reads this brief when the active in-progress slice declares `target: vectis`. The brief drives the host workflow that produces a buildable cross-platform application (Crux shared core + per-platform shells) from the slice's already-synthesised `spec.md` and `design.md`. It owns three new responsibilities the legacy skill-per-step layout did not pin down in one place:

1. **`composition.yaml` regeneration.** The wired `composition.yaml` is no longer a Specify artifact — synthesis does not write it. This brief regenerates it from `spec.md` + `design.md` (which already carry every upstream spatial / structural claim synthesis folded in from source adapters) at the start of each build, alongside the code it accompanies. `merge` lands the regenerated file together with the implementation code.
2. **Inline writer bodies.** The legacy `/vectis:core-writer`, `/vectis:test-writer`, `/vectis:ios-writer`, `/vectis:android-writer`, `/vectis:template-updater`, and the three reviewers are retired as separate skills; their operative content lives inline in the sections below.
3. **Operator-curated inputs are read, never authored.** `tokens.yaml` and `assets.yaml` are operator-curated and consumed as build inputs; the brief never invents or restates their contents.

The Vectis target stays three-capability (`shape` / `build` / `merge`) — there is **no** fourth `refine` slot. Composition regeneration is part of `build`, not a separate capability.

## Standard arguments

All inline writer sections below assume these symbols are resolved by `/spec:build` before the sub-agent fan-out:

| Symbol | Meaning |
| --- | --- |
| `SLICE_ID` | The active slice name (`specify plan next` output, or `specify slice` argument). |
| `SLICE_DIR` | `.specify/slices/<SLICE_ID>/`. |
| `FEATURE_NAME` | The single feature spec folder under `SLICE_DIR/specs/`. When the slice carries multiple features, iterate the per-feature steps below in declaration order. |
| `PROJECT_DIR` | The target project root (single-repo mode) or the resolved workspace slot (workspace mode). |
| `IOS_SHELL_DIR` | `${PROJECT_DIR}/iOS` (only when `ios` is in scope). |
| `ANDROID_SHELL_DIR` | `${PROJECT_DIR}/Android` (only when `android` is in scope). |
| `APP_NAME` | The Xcode target / Swift source folder name (derived from `design.md`'s `App` struct name). |

## Platform detection

Read `${SLICE_DIR}/proposal.md` `## Platforms` to determine scope. Valid Vectis platform tokens are `core`, `ios`, `android`, and the deferred `web`. Token / asset / layout work is **input context**, never a platform. Process platforms in dependency order:

1. `core` first — shells depend on the core.
2. `ios` and `android` shells — independent of each other; their **generation** phases can run in parallel; their **verify** phases are serial because they share the same Cargo workspace lock.
3. `web` — deferred.

If the proposal lists `core` only, skip the iOS and Android sections wholesale; this is a backend-only build.

## Step 1 — Regenerate `composition.yaml` from `spec.md` + `design.md`

Per RFC §Target-specific structured outputs, the wired `composition.yaml` is a build output, regenerated from the canonical artifacts on each `/spec:build` run. The substep runs before any shell writer fires.

Input sources, in priority order:

1. `${SLICE_DIR}/specs/<feature>/spec.md` — screen titles ("`Requirement: Todo List View`"), platform-specific behaviour sections, observable token / asset references.
2. `${SLICE_DIR}/design.md` — ViewModel variants, per-page view struct fields, `Event` variants, `Route` variants, capability matrix.
3. Sibling UI inputs (operator-curated, read-only): `${SLICE_DIR}/tokens.yaml` and `${SLICE_DIR}/assets.yaml` when present; otherwise `${PROJECT_DIR}/design-system/tokens.yaml` and `${PROJECT_DIR}/design-system/assets.yaml`. Used to validate token / asset references in the regenerated output; never to author requirements.
4. Optional `${SLICE_DIR}/composition.yaml` from a prior `/spec:build` run on the same slice (refining iteration). When present, preserve any operator-applied `# GAP` comments and re-validate against the updated artifacts.

### Steps

1. **Identify screens.** Walk every `### Requirement:` block in `spec.md`. A requirement is a screen when its title or body describes a view ("`Requirement: Todo List View`"), or when a scenario describes navigation to a destination ("WHEN user taps add THEN the app navigates to the add todo form"). Derive a kebab-case slug from the title (`Todo List View` → `todo-list`). Distinct ViewModel data shapes imply separate screens; transitions between loading / main / error are states within a screen.
2. **Adopt names from `design.md`.** ViewModel variant names, per-page view struct names, and field names come from `design.md`'s Domain Model section. Use them verbatim. If `design.md` does not document a screen the spec implies, this is a slice-definition gap — surface it as a `# GAP: design.md missing variant for <screen>` comment in the output and continue. The merge brief will catch it.
3. **Place items in regions.** For each screen, place screen title and navigation actions in `header`, primary content in `body` (choosing `list`, `grid`, `form`, or group-based layout based on the data shape described in `design.md`), secondary actions in `footer`, and a primary creation action as `fab` when one appears in the spec. Use `group` containers (`direction`, `gap`, `padding`, `align`, `justify`, `size`, `background`, `corner_radius`, `elevation`) to express layout intent the spec / design imply.
4. **Wire bindings.** For each screen entry, add:
    - `maps_to: "ViewModel::<ScreenName>(<ScreenName>View)"` (PascalCase from the slug).
    - `bind` on display and input items — the per-page view struct field name (from `design.md`).
    - `event` on interactive items — the `Event` variant the interaction triggers. Use `EventName` for no-arg, `EventName(arg)` for events that carry item-context fields or the `value` keyword.
    - `error` on `field` items when `design.md` describes validation for the input.
    - `*-when` conditional keys when the spec describes conditional visual states (`completed items show strikethrough` → `strikethrough-when: completed`).
5. **States and overlays.** For each screen, identify alternate states from the spec (loading, empty, error, saving) and add entries under `states` with `when:` predicates and replacement `body` content. Identify dialogs / sheets / snackbars from the spec and add entries under `overlays` with `kind`, `trigger` (the `Event` name that opens the overlay), optional `title`, and `content`.
6. **Per-platform overrides.** When `spec.md` platform-specific sections describe materially different layouts (not just behavioural differences), add a `platforms` map with per-platform region overrides on the affected screens.
7. **Naming proposals.** The names this step proposes — screen slugs, ViewModel variants, field names, event names — must match what `design.md` already documents. When `design.md` is silent, prefer the `design.md` conventions (snake_case fields, PascalCase ViewModel / Event names, kebab-case screen slugs). Never invent names that contradict `design.md`.
8. **Surface gaps.** Emit YAML comments (`# GAP: ...`) for any of: a spec-described data element with no natural visual representation; a spec-described interaction with no interactive item to wire; structurally recurring groups that look like a missing `component: <slug>` directive; a `bind` value that has no matching field on the per-page view struct described in `design.md`; an `event` value that has no matching variant in `design.md`.

Write the result to `${SLICE_DIR}/composition.yaml` via the **stage → validate → rename** sequence used by every Vectis producer:

```bash
COMP="${SLICE_DIR}/composition.yaml"
write_yaml "${COMP}.tmp"
specify tool run vectis -- validate composition "${COMP}.tmp"
mv "${COMP}.tmp" "${COMP}"
```

The validator (`specify tool run vectis -- validate composition <path>`) auto-invokes `tokens` and `assets` modes against any sibling `tokens.yaml` / `assets.yaml`. Errors are blocking: surface the report verbatim, delete the staging file, and exit non-zero — any prior `${SLICE_DIR}/composition.yaml` is preserved untouched. Warnings forward into the operator-facing summary. Clean runs proceed silently. On a validation error, fix `spec.md` / `design.md` (or the operator-curated `tokens.yaml` / `assets.yaml`) and re-run `/spec:build`; the regeneration step is idempotent against unchanged inputs.

When the slice has no UI surface at all (e.g. a core-only backend slice), this step writes no `composition.yaml`. Detect this by checking whether `proposal.md` lists any non-`core` platform; when only `core` is present, skip the step entirely.

## Step 2 — Composition validation gate (pre-shell)

After regenerating `composition.yaml`, re-run the deterministic validator against the merged input set:

```bash
specify tool run vectis -- validate composition
```

That single call covers:

1. **Composition schema validity** — `composition.yaml` conforms to the Vectis composition schema (regions, group hierarchy, allowed wiring keys, slug grammar, reserved-slug prohibitions).
2. **Wiring coverage** — every field in each per-page view struct (from `design.md`) appears as a `bind`; every shell-facing `Event` variant relevant to a screen has an `event` wiring; every `maps_to` resolves to a declared ViewModel variant; every overlay `trigger` matches an `event` name in the same screen; every `Navigate(X)` argument has a corresponding screen slug and `Route` variant.
3. **Structural identity** — every `component: <slug>` reused across screens has a structurally identical skeleton (with allowed `*-when`-gated sub-groups, state-replaced bodies, and per-instance `platforms.*` overrides).
4. **Auto-invoked `tokens` mode** — when a sibling `tokens.yaml` is present, every token reference in `composition.yaml` (and in `assets.yaml` when present) resolves against it.
5. **Auto-invoked `assets` mode** — when a sibling `assets.yaml` is present, every `image:` / `icon:` / `icon-button:` / `fab:` reference in `composition.yaml` resolves to a declared asset id, every declared asset file exists on disk, and per-platform raster densities / vector exports cover the targeted shell platforms.

**Severity handling.** Validation errors halt shell generation for the affected screens; the agent reports the errors and does not proceed until they are resolved. Warnings are logged and reported but do not block generation. The declared tool's exit semantics hold here: validation findings return non-zero with a report, warnings return zero with a report, and clean runs return zero silently. A tool invocation failure (missing sidecar, bad arguments, unreadable preopen) is a WASI tool failure; report it separately from the host prerequisite failures described in the verify sections below.

When `composition.yaml` is absent (core-only slice), `specify tool run vectis -- validate composition` exits cleanly without performing wired-mode checks; the brief proceeds directly to the core sub-agent.

## Step 3 — Sub-agent delegation contract

Each writer / verifier section below runs in its **own sub-agent** with a clean context window. `/spec:build` (this brief's caller) coordinates the sequence but does not execute step bodies inline. Without delegation, the orchestrator's context accumulates thousands of lines of writer instruction, generated code, and compiler output that are irrelevant to later phases.

### Handoff fields

**Inputs (orchestrator → sub-agent):**

| Field | Description |
| --- | --- |
| `task` | One of `core-writer`, `test-writer`, `ios-writer`, `android-writer`, `core-reviewer`, `ios-reviewer`, `android-reviewer`, `template-updater`. Maps to the matching section below. |
| `arguments` | Standard arguments: `SLICE_ID`, `FEATURE_NAME`, `PROJECT_DIR`, `IOS_SHELL_DIR`, `ANDROID_SHELL_DIR`, `APP_NAME`. |
| `mode` | `create`, `update`, or `repair`. The orchestrator decides this from on-disk inspection (see per-section "Detect mode" subheads) before spawning. |
| `skip_verification` | When `true`, the writer skips its inline build-verification step (ios-writer's `make build`, android-writer's `gradlew :app:assembleDebug`). The orchestrator sets this for the shell writers and runs verification in a dedicated sub-agent afterward (see "Why verify is serial" below). |
| `artifact_paths` | Paths to `spec.md`, `design.md`, `proposal.md`, regenerated `composition.yaml`, and sibling `tokens.yaml` / `assets.yaml` when present. |
| `orchestrated` | Reviewer sub-agents only. When `true`, the reviewer returns `design_findings` in its output instead of writing a follow-up Specify slice; the orchestrator consolidates findings across platforms after both reviews complete. Default `false`. |
| `extra_context` | Phase-specific: error output for `repair` mode, baseline test log for regression checks, prior phase warnings. |

**Outputs (sub-agent → orchestrator):**

| Field | Description |
| --- | --- |
| `status` | `success`, `failure`, or `pending`. |
| `files_modified` | Paths created or changed. |
| `verification` | Inline verification result, when the sub-agent ran one. |
| `errors` | Error details when status is `failure` or `pending`. |
| `warnings` | Non-blocking issues for downstream phases. |
| `design_findings` | Reviewer sub-agents only: classified findings (`code-fix` vs `spec-change`) with check IDs, severity, and suggested fixes. Empty list when nothing surfaced. |

### Why verify is serial; review is parallel

The iOS verify pipeline (`make build` → cargo-swift) and the Android verify pipeline (`make build` → uniffi typegen, `gradlew :shared:cargoBuild`) both invoke `cargo` against the same shared Rust workspace. Cargo uses a workspace-level lock file, so concurrent invocations serialise on the lock rather than running in parallel. The reviewers are pure code-analysis agent teams; they use different formatters (`swiftformat` vs Kotlin) and never invoke `cargo`, Gradle, or Xcode. With no shared mutable state and no build-tool contention, they are safe to run concurrently.

```text
both writers done (skip_verification: true)
    → iOS verify sub-agent       (serial — cargo workspace lock)
    → Android verify sub-agent   (serial — cargo workspace lock)
both verified
    ├── iOS review sub-agent     (parallel — code analysis only)
    └── Android review sub-agent (parallel — code analysis only)
both reviews done
    → consolidate design-level findings (see "Consolidate review findings" below)
```

If only one shell platform is in scope, still pass `skip_verification: true` to the writer and run the dedicated verify sub-agent afterward — the contract is consistent regardless of how many platforms are active.

## Step 4 — Crux shared core (`core-writer` work)

Detect mode by inspecting `${PROJECT_DIR}/shared/src/app.rs`:

- Missing → **create mode**: render the scaffold with `specify tool run vectis -- scaffold core <APP_NAME> [--caps <csv>]`, then enter update mode for feature-specific code.
- Present → **update mode**: diff the artifact-derived target against the existing implementation and apply targeted edits.

A repair sub-agent (invoked from the verify-repair loop) uses `mode: repair` plus the failing error output to apply the minimum change to fix the reported errors without re-running the full create / update process.

### Inline writer steps

1. **Read inputs.** `${SLICE_DIR}/specs/${FEATURE_NAME}/spec.md` (core body + platform sections), `${SLICE_DIR}/design.md` (Domain Model, Adapters, API Contracts, Implementation Constraints). Extract App name, Model, Events, ViewModel / Page / Route, capability set, and any HTTP / SSE / KV shapes.
2. **Detect mode** as above. In create mode, render the scaffold via `specify tool run vectis -- scaffold core <APP_NAME> --caps <comma-separated-caps> [--version-file <path>]` and run an explicit `cargo check --workspace` sanity gate before any further edits.
3. **Build an implementation inventory** of existing types and diff against the artifact-derived target — Added / Removed / Modified / Unchanged — per category in dependency order: capabilities → views → domain → model → events → API → logic.
4. **Apply structural edits** to `app.rs`: domain types → `Page` / `ViewModel` / `Route` → `Model` → `Event` / `Effect` → imports → `Cargo.toml` updates for new capabilities. Adopt screen names, ViewModel variants, per-page view structs, field names, and `Event` / `Route` variants verbatim from `design.md`.
5. **Apply logic edits** to `update()` and `view()`: per-`Event` match arms, business rules from the spec, model-to-ViewModel mapping for new pages. Consult the Crux 0.17 surface: return `Command<Effect, Event>` from `update()`; mark `Event` enums `#[repr(C)]`; never define a `Capabilities` struct (the 0.17 API uses `Effect` directly as an enum with `#[effect(facet_typegen)]`); never call `crux_core::cli::run()` (use `crux_core::type_generation::facet::TypeRegistry` instead); generate SSE inline as a custom adapter — not a published crate.
6. **Run `cargo check`** as a sanity gate. Full clippy / test / regression runs happen at the orchestration level (verify-repair loop below).
7. **Preserve helpers, comments, custom adapter modules, and `Cargo.lock`.** Never regenerate a file from scratch in update mode. Never hand-edit Cargo dependency versions — the scaffold tool owns version pins so `crux_core`'s bundled `uniffi_bindgen` matches the runtime `uniffi` crate. Never write tests in this sub-agent (test-writer owns them). Never generate shell code (the shell sub-agents own it).

## Step 5 — Crux tests (`test-writer` work)

Run after core-writer in the same slice. Detect mode from the existing `#[cfg(test)] mod tests` block in `app.rs`:

- No tests yet → **create mode**.
- Tests exist, spec changed → **update mode** (drift detection: diff spec scenarios against existing tests, add tests for new scenarios, update assertions for modified scenarios, flag stale tests for removed scenarios with `// STALE: scenario removed from spec`).
- Verify-repair failure → **repair mode** (sub-agent invoked with `mode: repair` plus failing test output).

### Inline writer steps

1. **Read inputs.** `${SPEC_PATH}`, `${DESIGN_PATH}`, `${APP_RS}`. Use spec-to-test mapping rules: one synchronous `#[test]` per scenario, named after the scenario, with a `/// Spec: <feature> > REQ-XXX > Scenario: <scenario>` traceability comment.
2. **Map scenarios deterministically.** Each `#### Scenario:` block produces exactly one test function. The `**WHEN**` clause becomes the test setup (model state, dispatched events). The `**THEN**` clause becomes assertions over `Command` effects and `view()` output. Stable `REQ-XXX` ID + scenario title is the drift-detection key.
3. **Write tests inside `#[cfg(test)] mod tests`** in `app.rs` (Crux convention — not a separate `tests/` directory). Preserve existing helpers, factory functions, and test style.
4. **Coverage requirements.** Every scenario; every shell-facing `Event` variant; every page transition (Loading → Main, Error → retry); every validation rule; every adapter's happy and error path; factory helpers for repeated setup.
5. **Crux test API.** Synchronous only — never `#[tokio::test]` or any async runtime. Call `update()` directly; inspect `Command` effects; resolve effects with simulated responses (`expect_one_effect()`, `expect_http()`, `resolve()`); assert on model and view-model state (`expect_one_event()`).
6. **Do not run `cargo test` in create or update mode** — orchestration owns it. In repair mode, run `cargo test` to get fresh errors and verify the fix before returning. Preserve test names, `/// Spec:` traceability comments, and assertion intent — only adjust the syntax used to express them.

## Step 6 — Core verify-repair loop (max 3 iterations)

Spawn this loop in its **own sub-agent** with `PROJECT_DIR`, the spec path, and (in update mode) a baseline test log captured before the writers ran. The sub-agent returns `status`, `iterations_used`, and any unresolved errors.

Capture the baseline before the writers (update mode only):

```bash
cd "$PROJECT_DIR" && cargo test 2>&1 | tee "/tmp/${SLICE_ID}-${FEATURE_NAME}-baseline.txt"
```

Each iteration runs all three checks; if any fail, apply the targeted fix and start a new iteration.

### Checks

```bash
cd "$PROJECT_DIR" && cargo fmt --check        # 1. Formatting (auto-fix with `cargo fmt`).
cd "$PROJECT_DIR" && cargo check              # 2. Compilation.
cd "$PROJECT_DIR" && cargo clippy --all-targets  # 2. Lint.
cd "$PROJECT_DIR" && cargo test               # 3. Tests.
```

### Failure classification → repair sub-agent routing

| Failure signal | Classification | Fix action |
| --- | --- | --- |
| Error in `#[cfg(test)] mod tests`, test helpers, or factories | Test issue | Spawn `test-writer` repair sub-agent with the error output. |
| Error in production code (`app.rs` outside `#[cfg(test)]`), missing types or methods | Code issue | Spawn `core-writer` repair sub-agent with the error output. |
| Assertion mismatch where *actual* looks correct per spec | Test issue | Spawn `test-writer` repair sub-agent — the expected value is wrong. |
| Assertion mismatch where *expected* matches spec | Code issue | Spawn `core-writer` repair sub-agent — the handler returns the wrong result. |
| Type mismatch between handler output and assertion | Per spec, route to the wrong-typed side. | Classify per spec, spawn the appropriate repair sub-agent. |
| API surface mismatch: wrong method on `Command`, incorrect `expect_*` chain, stale builder, wrong `resolve()` argument shape | Test issue | Spawn `test-writer` repair sub-agent (the Crux 0.17 API surface is non-trivial; the sub-agent reads the relevant Crux docs / template before fixing). |
| Unresolved import or missing crate in `Cargo.toml` | Workspace issue | Edit `Cargo.toml` directly (no sub-agent needed). |

### Repair discipline

- **Minimum change only.** Fix the reported error and nothing else.
- **Scope the diff.** Before committing a repair, verify the change is limited to files and functions identified in the error output.
- **One failure class per sub-agent.** When multiple failures are present, group them by classification (code vs test) and spawn one repair sub-agent per class.

### Regression check (update mode only)

After tests pass, compare results against the baseline from before the writers ran. For each test that passed before and now fails:

- If the test asserts behaviour the updated spec **explicitly changes** → expected behavioural change, not a regression.
- If the test asserts behaviour the spec does **not** change → true regression. Surface as a failure and route to the appropriate repair sub-agent.

### Loop control

Repeat until all three checks pass or 3 iterations are exhausted. If still failing after 3 iterations: **stop**. Do not mark the task complete. Report the remaining failures with full error output and escalate for guidance (the merge brief reads this as a `build` failure outcome).

## Step 7 — iOS shell (`ios-writer` work)

Skip this section when `ios` is not in `proposal.md` `## Platforms`. The Step 2 composition validation gate MUST have passed before this section runs.

Detect mode by inspecting `${IOS_SHELL_DIR}` for any `.swift` files:

- No Swift files → **create mode**: scaffold with `specify tool run vectis -- scaffold ios <APP_NAME> [--caps <csv>]`, then enter update mode.
- Swift files present → **update mode**: diff core types against existing Swift code and apply targeted edits.

Spawn the writer sub-agent with `mode: create|update` and `skip_verification: true`; the dedicated verify sub-agent (Step 8) runs afterward.

### Inline writer steps

1. **Read the input contract.** `app.rs`, `lib.rs`, `Cargo.toml`, the regenerated `composition.yaml`, sibling `tokens.yaml` / `assets.yaml` when present, and the `## iOS Shell Requirements` section of `spec.md` plus the `## iOS Shell Details` section of `design.md`.
2. **Diff core and UI artifacts.** Classify changes to `Effect`s, ViewModel variants, per-page view-struct fields, `Event`s, `Route`s, token categories, assets, components, and any legacy `VectisDesign` references (the latter are forbidden in 2.0 — remove on sight).
3. **Apply core / view updates.** Edit `Core.swift` (the Crux bridge — effect handlers, serialization protocol), `ContentView.swift` (root branching on the `ViewModel` enum), per-screen views under `iOS/<APP_NAME>/Views/`, navigation wiring, Inject hot-reload boilerplate, and build config with targeted changes only.
4. **Refresh generated UI surfaces.** Regenerate shell-local `iOS/<APP_NAME>/Theme/` (theme code derived from `tokens.yaml`, HIG fallback when `tokens.yaml` is absent), `iOS/<APP_NAME>/Components/` (one named SwiftUI view per `component: <slug>` directive in `composition.yaml`, PascalCased — `task-row` → `TaskRow`), and `iOS/<APP_NAME>/Resources/Assets.xcassets/` (one entry per `assets.yaml` declaration; SF Symbols resolve at the call site without copy). Preserve operator-owned files.
5. **Enforce shell boundaries.** Keep all business logic in the Rust core; the shell only renders views and performs platform I/O. Remove any legacy `import VectisDesign` — there is no shared Swift Package in 2.0; the writer emits shell-local theme + asset code exclusively.
6. **SwiftUI hazards to avoid.** Never place `TextField` or a small `Button` inside a `ScrollView` within a `NavigationStack` — the `UIScrollView` touch-delay mechanism suppresses taps. Always include `#Preview` blocks for new screens to keep Xcode previews working.

## Step 8 — iOS verify (max 3 iterations)

Spawn this loop in its own sub-agent with `IOS_SHELL_DIR` and `APP_NAME`. The sub-agent returns `status`, `iterations_used`, and any unresolved errors.

```bash
swiftformat "${IOS_SHELL_DIR}/${APP_NAME}/"        # 1. Format.
cd "$IOS_SHELL_DIR" && make build                  # 2. Build (typegen + package + xcodegen).
cd "$IOS_SHELL_DIR" && make sim-build              # 3. Simulator build.
```

If a step fails, fix the issue and re-run from step 1. Repeat until all three checks pass or 3 iterations are exhausted. If the same error recurs across iterations with no change in output, stop early. If still failing after 3 iterations: **stop**, report the remaining failures with full error output, and escalate.

If the iOS app panics with "UniFFI contract version mismatch", the installed `cargo-swift` version is incompatible with the active Vectis version pins — surface this to the operator (it is typically a template-drift fix; see Step 12).

## Step 9 — Android shell (`android-writer` work)

Skip this section when `android` is not in `proposal.md` `## Platforms`. The Step 2 composition validation gate MUST have passed before this section runs.

Detect mode by inspecting `${ANDROID_SHELL_DIR}/app/src/main/java/<package>/Core.kt`:

- Missing → **create mode**: scaffold with `specify tool run vectis -- scaffold android <APP_NAME> [--caps <csv>] [--android-package <package>]`, then enter Android host post-processing (Step 10 pre-flight), then update mode.
- Present → **update mode**: diff core types against existing Kotlin code and apply targeted edits.

Spawn the writer sub-agent with `mode: create|update` and `skip_verification: true`; the dedicated verify sub-agent (Step 10) runs afterward.

### Inline writer steps

1. **Read inputs.** `app.rs`, the regenerated `composition.yaml`, sibling `tokens.yaml` / `assets.yaml` when present, the `## Android Shell Requirements` section of `spec.md`, and the `## Android Shell Details` section of `design.md`. Extract App name, ViewModel / Effect / Event / Route variants, and the capability set.
2. **Build an inventory** of existing Kotlin code: effect handlers, ViewModel cases, screen composables, event dispatches, adapter clients (Ktor for HTTP / SSE, SharedPreferences for KV), DI modules (Koin when multiple non-Render effects are used).
3. **Diff Rust core types vs Kotlin inventory** by category (Effect → ViewModel → view-fields → Event → Route) and emit a summary edit plan.
4. **Apply changes.** Expand or strip CAP blocks in `Core.kt`, `AndroidManifest.xml`, and Gradle build files. Add or remove screen composables for each ViewModel variant under `Android/app/src/main/java/com/vectis/<app>/ui/screens/`. Update the root `when` over the `ViewModel` enum. Dispatch new `Event`s through `Core.update(...)`. Emit one named composable per `component: <slug>` directive in `composition.yaml` (PascalCased), with props inferred from variation across instances.
5. **Refresh generated UI surfaces.** Regenerate shell-local theme code under `Android/app/src/main/java/com/vectis/<app>/ui/theme/` (Material 3 fallback when `tokens.yaml` is absent), and drawable resources under `Android/app/src/main/res/drawable*/` (one entry per `assets.yaml` declaration; Material icons resolve at the call site without copy).
6. **Update build configuration** (`libs.versions.toml`, `build.gradle.kts`, manifest permissions, `network_security_config.xml`) to match the changed capability set. Remove any legacy `:vectis-design` Gradle module references — there is no shared Compose module in 2.0; the writer emits shell-local theme + drawable code exclusively. Replace any stale `import com.vectis.design.*` with `import com.vectis.<app>.ui.theme.*`.
7. **UniFFI bridging contract.** The `Application` class MUST set `System.setProperty("uniffi.component.shared.libraryOverride", "shared")` before any UniFFI class is loaded — without this the app fails with `UnsatisfiedLinkError` on launch. Imports for generated FFI types follow `import com.vectis.<app>.*` (not `com.vectis.design.*`). Rethrow `CancellationException` from coroutines — never swallow it.

## Step 10 — Android verify (max 3 iterations)

Spawn this loop in its own sub-agent with `ANDROID_SHELL_DIR`. The sub-agent returns `status`, `iterations_used`, and any unresolved errors.

### Pre-flight (fail fast on misconfiguration)

Run these before entering the loop. If any check fails, report the missing prerequisite and mark Android verification as **pending** rather than entering the build loop.

```bash
test -f "${ANDROID_SHELL_DIR}/local.properties"
grep -q "sdk.dir" "${ANDROID_SHELL_DIR}/local.properties"
grep -q "org.gradle.java.home" "${ANDROID_SHELL_DIR}/gradle.properties"  # Must point to Java 21.
rustup target list --installed | grep android
```

### Gradle wrapper bootstrap

Before any `./gradlew` invocation, verify `gradlew` exists and is executable and `gradle/wrapper/gradle-wrapper.jar` is present. If the wrapper is missing, bootstrap from a minimal init project:

```bash
tmp_dir=$(mktemp -d)
cd "$tmp_dir" && gradle wrapper && cd -
cp "$tmp_dir/gradlew" "$tmp_dir/gradlew.bat" "$ANDROID_SHELL_DIR/"
cp -r "$tmp_dir/gradle" "$ANDROID_SHELL_DIR/"
chmod +x "$ANDROID_SHELL_DIR/gradlew"
rm -rf "$tmp_dir"
```

If `gradle` itself is not installed, report the prerequisite (`brew install gradle`) and mark Android verification as pending.

### Build loop

```bash
cd "$ANDROID_SHELL_DIR" && make build                       # 1. Type generation + cross-compile.
cd "$ANDROID_SHELL_DIR" && ./gradlew :shared:cargoBuild     # 2. Rust library build.
cd "$ANDROID_SHELL_DIR" && ./gradlew :app:assembleDebug     # 3. APK build.
```

If a step fails, fix the issue and re-run. Repeat until all three checks pass or 3 iterations are exhausted. Stop early on identical-output regressions. If still failing after 3 iterations: **stop** and escalate. Java 25+ environments hit `IllegalArgumentException`; the fix is pinning `org.gradle.java.home` to Java 21 in `gradle.properties`.

## Step 11 — Reviewers (final pass)

After verify completes, spawn the relevant reviewer sub-agents. Each reviewer runs an agent team — three specialists plus an antagonist — through a bounded review-fix loop (max 3 iterations). The reviewer never edits beyond mechanical auto-fixes and reverts the entire batch if verification regresses.

### Core reviewer

Spawn after the core verify-repair loop succeeds. Scope: the Rust `shared` crate.

- **Specialists.** Structural (CRX-001..011 — missing `render()`, serde derives, input validation, `PendingOp` timestamps, ViewModel typing, unused deps), Logic (LOG-001..009 — state-machine completeness, op coalescing, concurrent conflicts, temporal ordering, rapid-action sequences, spec gaps, spec-to-test coverage, stale tests), Quality (GEN-001..012 — no `unwrap` / `expect` outside test setup, no debug output, no hardcoded secrets, error propagation, match exhaustiveness, function length). The lead also runs universal codex checks (UNI-001..021) with Rust / Crux heuristics.
- **Mechanical auto-fixes.** Missing serde derives, `render().and(...)` wraps, `.trim()` / empty input checks, unused deps. Revert the full batch if `cargo check` / `cargo clippy` / `cargo test` regress.
- **Logic findings stay non-mechanical.** Never auto-fix LOG-001..008 without explicit confirmation; surface them as design-level findings classified `code-fix` or `spec-change`.
- **Standalone vs orchestrated.** The core reviewer has no orchestrated mode — when design-level findings accumulate it always returns them for consolidation by the merge brief / operator.

### iOS reviewer

Spawn after iOS verify succeeds. Scope: every Swift file under `${IOS_SHELL_DIR}` plus read-only access to `${PROJECT_DIR}/shared/src/app.rs` and the wired UI input set.

- **Specialists.** Structural (IOS-001..019 — ViewModel / screen correspondence, effect handlers, token usage, ScrollView hazards, recurring-group component candidates), Quality (SWF-001..010 — concurrency, force unwraps, a11y labels, state management, previews, swiftformat), Integration (only on the first full-scope iteration — token / asset / composition cross-artifact checks). The lead runs universal codex checks (UNI-001..021) with Swift heuristics.
- **Mechanical auto-fixes.** Accessibility labels, design-token swaps, missing `#Preview`, Inject boilerplate. Revert the batch if `swiftformat` or the build regresses.
- **Orchestrated mode.** When the orchestrator passes `orchestrated: true`, the reviewer returns classified `design_findings` (`code-fix` vs `spec-change`) instead of writing a follow-up Specify slice. The orchestrator consolidates findings across iOS and Android into one cross-platform finding set.

### Android reviewer

Spawn after Android verify succeeds. Scope: every Kotlin file under `${ANDROID_SHELL_DIR}` plus read-only access to `${PROJECT_DIR}/shared/src/app.rs` and the wired UI input set.

- **Specialists.** Structural (AND-001..027 — screen / ViewModel correspondence, effect handlers, token usage, UniFFI library override, generated-type imports, coroutine safety, recurring-group component candidates), Quality (KTL-001..010 — force-unwraps, debug output, coroutine cancellation, Compose state, previews, a11y `contentDescription`), Integration (only on the first full-scope iteration — token / asset / composition cross-artifact checks). The lead runs universal codex checks (UNI-001..021) with Kotlin / Android heuristics.
- **Mechanical auto-fixes.** `contentDescription`, design-token swaps, missing `@Preview`, generated-FFI-type imports (`import com.vectis.<app>.*`), `CancellationException` rethrow, replacing stale `import com.vectis.design.*` with `import com.vectis.<app>.ui.theme.*`. Revert the batch if the Gradle build regresses.
- **Orchestrated mode.** Same `orchestrated: true` contract as the iOS reviewer.

### Consolidate review findings

When both shell reviews complete (or the single one when only one platform is in scope):

1. **Merge findings.** Combine `design_findings` from each reviewer into a single list. Deduplicate universal findings (UNI-prefixed) that both reviewers flagged with identical check IDs and matching evidence — keep the higher-severity instance. Platform-specific findings (IOS-, SWF-, AND-, KTL-, INT-prefixed) are always distinct.
2. **Empty list.** Skip the rest of this section.
3. **Validate classifications.** Each finding already carries `code-fix` or `spec-change`. Treat that as the source of truth. Resolve disagreements between platforms by applying: spec is clear but code is wrong → `code-fix`; spec is silent, ambiguous, or problematic → `spec-change`.
4. **Surface findings.** Findings flow to the operator alongside the build outcome. Cross-platform follow-up work is queued as a new slice via the operator's normal `/spec:plan` flow rather than letting reviewers spawn slices directly — the legacy "reviewer auto-creates a Specify change" path is retired in 2.0.

## Step 12 — Template / version-pin drift handling (`template-updater` path)

The Vectis scaffold tool (`specify tool run vectis -- scaffold ...`) is render-only and ships with embedded version pins. Upstream bumps (Crux core, uniffi, AGP / Gradle, cargo-swift, Xcode) can break a freshly rendered scaffold even when the rest of the slice is correct. Detect this when a verify-repair loop fails repeatedly with cargo / Gradle / Xcode errors that look like API renames, missing imports, or toolchain mismatches rather than feature-level bugs.

When detected:

1. **Confirm scope.** This is a template / pin issue when the failing error matches one of the known-drift patterns (renamed `crux_core` exports, AGP / Gradle major bump, uniffi bump that decouples from `crux_core::cli::bindgen`, cargo-swift incompatibility). Record the failing combo (caps + shells), the failing host step, and the load-bearing error line.
2. **Do not auto-fix in-band.** Template / pin drift is a workflow concern that lives in the `specify-cli` repo (templates + embedded `versions.toml`). The build brief surfaces the diagnosis and stops — the operator opens a separate slice rooted in the CLI repo (running the host workflow that was the `template-updater` skill: copy embedded `versions.toml` → scratch, edit pins, re-render the failing combo, run the cap matrix, propose a PR against `specify-cli`).
3. **Outcome reporting.** Record the failure as a build `failure` with a summary identifying it as template / pin drift so the operator can route to the right repair flow.

## Phase outcome contract

> See [Phase outcome contract](../../../plugins/spec/references/phase-outcome-contract.md).

The `build` phase concludes with exactly one of `success` / `failure` / `deferred`:

- **success** — every in-scope verify-repair loop returned `success` within its iteration budget, and the orchestrator has both regenerated `composition.yaml` (or skipped it for a core-only slice) and the implementation code under `${PROJECT_DIR}`. The slice lifecycle is ready to transition to `built`.
- **failure** — any verify-repair loop exhausted its iterations, or the composition validation gate (Step 2) failed and could not be repaired. Surface the load-bearing error line as `--summary` and the full output through `--context`; the merge brief refuses to run while the slice is in this state.
- **deferred** — a host prerequisite is missing (Java 21, Android SDK, Rust Android targets, `cargo-swift`, Gradle wrapper, Xcode CLT) or a known-drift template / pin issue surfaced (Step 12) and operator judgement is required. Surface the unresolved prerequisite or drift signal as `--summary`.

## Notes for downstream phases

- **`composition.yaml` is a build output.** It lives at `${SLICE_DIR}/composition.yaml` after this brief succeeds; the merge brief lands it into the baseline alongside the code. Operator-curated `tokens.yaml` / `assets.yaml` are also read by `merge`; the merge brief re-runs `specify tool run vectis -- validate composition` against the merged baseline so cross-artifact regressions are caught even when the current slice only touched code.
- **Do not write `composition.yaml` into `.specify/specs/`.** That is `specify slice merge`'s job, atomically, alongside the spec / design deltas.
- **Operator-curated inputs.** `tokens.yaml` and `assets.yaml` updates accompany the slice when the operator edits them; the merge brief promotes those edits into `design-system/tokens.yaml` / `design-system/assets.yaml` (or slice-local equivalents) using the same delta merge path as the spec deltas.
