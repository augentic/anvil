# Vectis target — `merge`

`/spec:merge` reads this brief when the slice it is merging declares `target: vectis`. The core merge work — deterministic spec-delta promotion, baseline coherence validation, lifecycle transition, and archive move — runs through `specify slice merge` per the shared `/spec:merge` skill. This brief adds the Vectis-specific adoption gates that run before the CLI invocation (preview confirmation), alongside it (the broader landing surface), and after it (host cap-matrix re-verification).

Two things make the Vectis `merge` brief different from the bare slice merge:

1. **`composition.yaml` is a build output that lands at merge time.** It is no longer a Specify artifact under `.specify/specs/`; the `build` brief regenerated it from `spec.md` + `design.md`, and `merge` promotes it into the baseline alongside the implementation code. The pre- and post-merge composition validators are the gate.
2. **The cap matrix is re-verified against the merged baseline.** A green slice build is necessary but not sufficient — the merge brief re-runs `cargo` / `make build` / `gradlew` against the merged tree because cross-slice regressions (UniFFI bridging drift, Java 21 / Gradle wrapper changes, cargo-swift drift, cap-marker expansion) only surface after deltas land.

## Prerequisites

Before merging, confirm:

- All task checkboxes in `${SLICE_DIR}/tasks.md` are complete.
- The slice lifecycle is `built` (the `build` phase returned `success`).
- The `build` phase regenerated `${SLICE_DIR}/composition.yaml` (or the slice is core-only and intentionally has none).
- `specify slice validate <SLICE_ID>` reports no unmet merge-phase needs.

Delta-spec merging, baseline coherence validation, lifecycle transition, and the archive move are delegated to the `specify` CLI. Follow the [`/spec:merge`](../../../../plugins/spec/skills/merge/SKILL.md) skill body for the driver-side flow: slice selection, prerequisite checks, the AskQuestion confirmation around the merge preview, baseline-drift handling, and result rendering. The Vectis adapter adds the two adapter-specific gates described below.

## Pre-merge — composition validation

Before invoking `specify slice merge`, re-run the deterministic validator against the staged slice contents so an invalid `composition.yaml` blocks the merge:

```bash
specify tool run vectis -- validate composition
```

The validator discovers `${SLICE_DIR}/composition.yaml` first (slice-local takes precedence) and auto-invokes `tokens` / `assets` modes against any sibling `tokens.yaml` / `assets.yaml`. Errors are blocking — surface the report verbatim and stop. Warnings forward into the operator-facing summary but do not block. When the slice is core-only (no `composition.yaml` in `${SLICE_DIR}`), the validator exits cleanly without performing the wired-mode checks.

A WASI tool invocation failure (missing sidecar, bad arguments, unreadable preopen) is a tool failure, not a validation finding; report it separately and stop.

## Merge invocation — broader landing surface

The merge surface is broader than spec / design / task deltas. In addition to the markdown deltas, `specify slice merge` promotes:

- `composition.yaml` from the slice — lands as the baseline UI input set for downstream shell generations (`.specify/specs/composition.yaml` or the platform-equivalent baseline path the project uses).
- `tokens.yaml`, `assets.yaml`, and any referenced asset files under `design-system/assets/**` (or slice-local `assets/`) when the slice carried operator-curated updates to those manifests. Token updates merge into `design-system/tokens.yaml`; asset updates merge into `design-system/assets.yaml` and `design-system/assets/**`.

Review every UI input delta alongside the spec / design / task changes in the `specify slice merge preview` output before confirming, so reviewers can see which downstream shell generations will be affected.

After `specify slice merge` exits zero, re-run the deterministic validator against the merged baseline:

```bash
specify tool run vectis -- validate composition
```

The validator discovers the now-merged baseline `composition.yaml` and auto-invokes `tokens` / `assets` modes against any sibling `tokens.yaml` / `assets.yaml`. Run this even when the current slice did not generate any platform code, because later shell work will consume the merged baseline input set. Validation findings prevent this brief from reporting a clean adoption gate and flow into the journal; warnings flow into the operator-facing summary; clean runs are silent.

## Post-merge — host cap-matrix re-verification

After `specify slice merge` exits zero (the slice's deltas have been promoted into the baseline and the lifecycle has transitioned to `merged`), verify the now-updated project root with host commands that match the assemblies present in the merged tree:

```bash
# core, when ${PROJECT_DIR}/shared exists
cd "$PROJECT_DIR" && cargo fmt --check
cd "$PROJECT_DIR" && cargo check
cd "$PROJECT_DIR" && cargo clippy --all-targets
cd "$PROJECT_DIR" && cargo test

# iOS, when ${PROJECT_DIR}/iOS exists
cd "$PROJECT_DIR/iOS" && make build
cd "$PROJECT_DIR/iOS" && make sim-build

# Android, when ${PROJECT_DIR}/Android exists
test -f "$PROJECT_DIR/Android/local.properties"
grep -q "org.gradle.java.home" "$PROJECT_DIR/Android/gradle.properties"
rustup target list --installed | grep android
cd "$PROJECT_DIR/Android" && make build
cd "$PROJECT_DIR/Android" && ./gradlew :shared:cargoBuild
cd "$PROJECT_DIR/Android" && ./gradlew :app:assembleDebug
```

Record every host step in a structured list with these fields:

- `name` — stable step id (`core.cargo-check`, `ios.make-build`, `android.gradlew-assembleDebug`, `android.preflight-java21`).
- `passed` — boolean.
- `failure_snippet` — empty when passed; otherwise the first useful stderr / stdout lines.

Host prerequisite failures (missing `cargo`, `gradle`, `xcodebuild`, Java 21, Android SDK / NDK, `cargo-swift`, Rust Android targets, an unusable Gradle wrapper) are host verification failures, not WASI tool failures. Name them as preflight steps (`android.preflight-java21`) so the journal makes the boundary clear.

When the slice modified neither the core nor a shell (e.g. a docs-only or UI-input-only slice that touched no Crux code), still run the applicable host checks against the merged tree — the cap matrix as a whole must remain green.

### Why post-merge, not pre-merge

The post-merge gate intentionally validates the merged baseline, not the staged delta. Shell verification (UniFFI bridging, Java 21 / Gradle wrapper, cargo-swift, cap-marker expansion) is only meaningful once the spec-level deltas are promoted and the writers have a stable baseline to build against. The `build` brief already verified the slice in isolation; this gate catches cross-slice regressions.

## Outcome contract

> See [Phase outcome contract](../../../../plugins/spec/references/phase-outcome-contract.md).

The Vectis adapter merge brief is the slice loop's first adapter-owned cap-matrix gate. Adapter diagnostics round-trip as opaque journal entries — the core does not parse them.

### success

`specify slice merge` exited zero, both composition validation gates had no blocking findings, and every required host verification step passed. The CLI atomically stamps `PhaseOutcome { phase: merge, outcome: success }` into `.metadata.yaml`, transitions the lifecycle to `merged`, and moves the slice directory into `.specify/archive/YYYY-MM-DD-<slice>/`.

The brief MUST NOT call `specify slice outcome set` on this path — the slice directory no longer exists under `.specify/slices/<slice>/` after archiving, so the call would fail with `not found`. The archived `.metadata.yaml` carries the success outcome.

`/spec:execute` translates `success` into a per-entry transition to `done` and proceeds to the next entry.

### failure

This branch covers three distinct failure modes:

1. **Pre-merge composition validation failed** or **`specify slice merge` exited non-zero** (a delta could not be applied, baseline coherence failed inside the merge call, the lifecycle gate refused the call). The filesystem is unchanged: no baseline was written and the slice directory was not moved.
2. **Post-merge composition validation failed.** The deltas have already landed in the baseline, but the merged UI input set is invalid or the declared WASI validator could not run.
3. **One or more host verification steps failed or could not run after a successful `specify slice merge`.** The deltas have already landed, but the merged baseline no longer builds end-to-end, or a host toolchain prerequisite is missing.

For modes 2 and 3, **the brief MUST NOT attempt to roll back the merge** — `specify slice merge` is not transactional with validation or host verification. Instead, journal the findings on the now-archived slice and surface the failure to the operator; the operator opens a follow-up slice (or `/spec:plan` a repair change) to fix the baseline.

Record the failure on the slice — first journal the diagnostic, then stamp the outcome (when the slice is still under `.specify/slices/`):

```bash
# Mode 1: pre-merge failure (slice is still active)
specify slice journal append <slice> merge failure \
  --summary "<which CLI step failed and the load-bearing stderr line>" \
  --context "<verbatim stderr / coherence-check tail / failing delta path>"

specify slice outcome set <slice> merge failure \
  --summary "<same load-bearing summary, written so it is useful as a /spec:drop reason>"
```

```bash
# Modes 2 and 3: post-merge validation or host-verification failure
# (slice is already archived)
specify slice journal append <slice> merge failure \
  --summary "<validator-or-host-step-name>: <one-line failure snippet>" \
  --context "<validator report/stderr or structured host step list with name, passed, failure_snippet>"

# Do NOT call `specify slice outcome set` — the slice directory has been moved
# to .specify/archive/<…>/ and the call fails with `not found`.
```

For mode 1, `/spec:execute` reads the `failure` outcome, translates it to per-entry `failed`, surfaces the journal entries, and stops the loop. For modes 2 and 3, `/spec:execute` reads the CLI-stamped `success` outcome and proceeds with the next entry; the operator separately triages the journal `failure` entry on the archived slice and queues a repair slice when ready.

`--summary` writing rules for post-merge findings: the load-bearing string is `"<validator-or-host-step-name>: <one-line failure snippet>"` (e.g. `"vectis.validate.composition: unresolved token colors.primary.dark"`, `"core.cargo-clippy: type mismatch in shared/src/app.rs:142"`, `"android.gradlew-assembleDebug: unresolved reference 'CoreFfi'"`). Keep it short enough to fit a CLI argument without truncation; route the validator report, invocation stderr, or full structured host step list through `--context` instead. When the declared validator itself fails to start, use `"vectis.validate.composition could not run: <stderr first line>"`. When a host prerequisite is missing, use the failing preflight step name such as `"android.preflight-java21: Java 21 not configured"`.

If the cap-matrix failure looks like a version-pin drift (AGP / Gradle / uniffi mismatch surfaced after pins changed in this slice), the matching repair flow is the template-updater host workflow against `specify-cli` (see [build.md](build.md) § Template / version-pin drift handling; symptom triage table: [`../references/known-drift.md`](../references/known-drift.md)). Record the failure here and surface it; the operator decides whether the next step is a template fix in the CLI repo, a pin rollback, or a follow-up slice.

### deferred

A merge prerequisite is unclear and `specify slice merge` was never invoked. Typical triggers:

- The user declined the AskQuestion confirmation around the merge preview.
- `specify slice merge` reported baseline drift (a sibling slice mutated the baseline after this slice was defined) that needs operator arbitration.
- `specify slice validate` surfaced unmet merge-phase needs that the brief cannot resolve unattended.

Plus Vectis-specific triggers around upstream-pin and shell-toolchain decisions:

- A breaking upstream change (new Crux core release with renamed exports, a uniffi bump that decouples from `crux_core::cli::bindgen`, an AGP / Gradle major bump, a cargo-swift bump) surfaced during this slice's build or after a successful merge. The mechanical repair path lives in the `specify-cli` template-updater workflow (symptom triage table: [`../references/known-drift.md`](../references/known-drift.md)), but the operator needs to confirm whether to take the upstream change at all before the brief commits.
- The operator wants to pin to a specific older version (holding back AGP 9.x because of `rust-android-gradle 0.9.6` drift, or holding back `crux_core` for a known-incompatible downstream consumer). The bump that verified is not the bump the operator wants to land.

Record the deferral on the slice — first journal the question, then stamp the outcome:

```bash
specify slice journal append <slice> merge question \
  --summary "<the question the operator must answer>" \
  --context "<verbatim conflict-check report / preview diff / lifecycle status / validator report / host step list>"

specify slice outcome set <slice> merge deferred \
  --summary "<same question, present-tense, self-contained>"
```

`/spec:execute` translates `deferred` into a per-entry `blocked`, surfaces the journal entries, and stops the loop. The brief MUST NOT silently fall through — recording the deferral is the only way the operator hears about it.

### Summary writing rules

The `--summary` strings ride into `/spec:drop reason` byte-for-byte when `/spec:execute` reclaims a `failure` or `deferred` slice. Keep them present-tense, self-contained, and short enough to fit a CLI argument without truncation. Route any validator report, host step list, verbatim stderr, or log tail through `--context` — that field is not forwarded to `--reason`.
