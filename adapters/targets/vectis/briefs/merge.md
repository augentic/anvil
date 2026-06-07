# Vectis target — `merge`

`/spec:merge` reads this brief when the slice it is merging declares `target: vectis`. The core merge work — deterministic spec-delta promotion, baseline coherence validation, lifecycle transition, and archive move — runs through `specify slice merge` per the shared `/spec:merge` skill. This brief adds the Vectis-specific adoption gates that run before the CLI invocation (preview confirmation), alongside it (the broader landing surface), and after it (host cap-matrix re-verification).

Two things make the Vectis `merge` brief different from the bare slice merge:

1. **`composition.yaml` is a build output that lands at merge time.** It is not a Specify artifact under `.specify/specs/`; the `build` brief regenerates it from `spec.md` + `design.md`, and `merge` promotes it into the baseline alongside the implementation code. The pre- and post-merge composition validators are the gate.
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

The validator discovers the now-merged baseline `composition.yaml` and auto-invokes `tokens` / `assets` modes against any sibling `tokens.yaml` / `assets.yaml`. Run this even when the current slice did not generate any platform code, because later shell work will consume the merged baseline input set. Validation findings trigger a stop hint with `failure-kind: post-merge-validator`; warnings flow into the operator-facing summary; clean runs are silent.

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

Host prerequisite failures (missing `cargo`, `gradle`, `xcodebuild`, Java 21, Android SDK / NDK, `cargo-swift`, Rust Android targets, an unusable Gradle wrapper) are host verification failures, not WASI tool failures. Name them as preflight steps (`android.preflight-java21`) so the stop hint makes the boundary clear.

When the slice modified neither the core nor a shell (e.g. a docs-only or UI-input-only slice that touched no Crux code), still run the applicable host checks against the merged tree — the cap matrix as a whole must remain green.

### Why post-merge, not pre-merge

The post-merge gate intentionally validates the merged baseline, not the staged delta. Shell verification (UniFFI bridging, Java 21 / Gradle wrapper, cargo-swift, cap-marker expansion) is only meaningful once the spec-level deltas are promoted and the writers have a stable baseline to build against. The `build` brief already verified the slice in isolation; this gate catches cross-slice regressions.

## Stop hint contract

> See [Phase outcome contract](../references/spec-runtime/phase-outcome-contract.md).

When the pre-merge gate, the CLI delta merge, or the post-merge hook fails, emit a structured stop hint as the body's final output:

- `slice` — slice name from `specify plan next`.
- `phase` — `merge`.
- `failure-kind` — one of `pre-merge-gate`, `baseline-conflict`, `lifecycle-refused`, `post-merge-validator`.
- `paths` — for `baseline-conflict`: the conflicting baseline files reported by `specify slice merge`. For `pre-merge-gate` / `post-merge-validator`: the captured validator report, structured host step list, or stderr log path.
- `next-action` — `resolve and re-run /spec:merge $SLICE` for conflicts; `re-run /spec:build $SLICE` for gate failures classified as build regressions; `queue repair slice` for `post-merge-validator` drift (composition validation or cap-matrix failure after a successful `specify slice merge`).

Lifecycle invariants: `pre-merge-gate` and `baseline-conflict` leave the slice at `built` and the plan entry at `in-progress`. `post-merge-validator` runs after `specify slice merge` succeeded, so the slice is already `merged` and the plan entry is already `done` — the hint is observability, not a park. The brief MUST NOT attempt to roll back the merge on a post-merge failure.

For cap-matrix failures that look like version-pin drift (AGP / Gradle / uniffi mismatch surfaced after pins changed in this slice), the matching repair flow is the template-updater host workflow against `specify-cli` (see [build.md](build.md) § Template / version-pin drift handling; symptom triage table: [`../references/known-drift.md`](../references/known-drift.md)). Record the failure in the stop hint and surface it; the operator decides whether the next step is a template fix in the CLI repo, a pin rollback, or a follow-up slice.
