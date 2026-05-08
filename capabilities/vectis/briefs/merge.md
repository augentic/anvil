---
id: merge
description: Merge the Vectis slice into the repository
needs: [build]
---

Before merging, confirm all task checkboxes in `tasks.md` are complete and the slice status is `complete`. Delta-spec merging, baseline coherence validation, the lifecycle transition, and the archive move are delegated to the `specify` CLI: `specify slice merge preview`, `specify slice merge conflict-check`, `specify slice merge run`, and `specify slice validate`.

Follow the [`specify-merge`](../../../plugins/spec/skills/merge/SKILL.md) skill for the driver-side flow — slice selection, prerequisite checks, the AskQuestion confirmation around the merge preview, baseline-drift handling, and result rendering. The Vectis capability adds two capability-specific gates after that flow: deterministic UI-input validation through the declared Vectis WASI tool, then host verification through the same ordinary build commands / verify sub-agent outputs used by the build brief. The `specify slice merge run` step promotes spec deltas (markdown) and composition deltas (YAML) under `.specify/specs/` and `.specify/specs/composition.yaml`; this brief validates and verifies the resulting baseline explicitly.

The `specify slice merge run` command merges both spec deltas and composition deltas in a single operation. The merge surface is broader than spec / design / task deltas: per RFC-11 §I "Merge handoff", `composition.yaml`, `tokens.yaml`, `assets.yaml`, and any referenced asset files under `design-system/assets/**` (or slice-local `assets/`) are reviewable lifecycle artifacts when they appear in a slice. `composition.yaml` continues to merge into the Specify baseline; token and asset updates merge into `design-system/tokens.yaml`, `design-system/assets.yaml`, and `design-system/assets/**` respectively. Review every UI input delta alongside the spec / design / task changes in the `specify slice merge preview` output before confirming the merge so reviewers can understand which downstream shell generations will be affected.

After `specify slice merge run` succeeds, re-run the deterministic UI input validator against the merged baseline:

```bash
specify tool run vectis-validate -- composition
```

The validator discovers the now-merged baseline `composition.yaml` and auto-invokes `tokens` / `assets` modes against any sibling `tokens.yaml` / `assets.yaml`. Run this even when the current slice did not generate any platform code, because later shell work may consume the merged baseline input set (RFC-11 §I "Merge handoff"). Validation findings prevent this brief from reporting a clean adoption gate and flow into the journal; warnings flow into the operator-facing summary; clean runs are silent. A tool invocation failure (missing sidecar, bad arguments, unreadable preopen) is a WASI tool failure, not a host prerequisite failure. When `composition.yaml` is absent from the merged baseline (no UI input set in the project), the validator exits cleanly without performing wired-mode checks.

## Capability-specific adoption gate

After `specify slice merge run` exits zero (i.e. the slice's `specs/` and `composition.yaml` deltas have been promoted into the baseline and the lifecycle has transitioned to `merged`), verify the now-updated project with host commands or a verify sub-agent that returns the same structured step list. There is no canonical Vectis verifier JSON envelope; the merge brief owns the journal shape.

Run the host checks that match the assemblies present in the merged tree:

```bash
# core, when $PROJECT_ROOT/shared exists
cd "$PROJECT_ROOT" && cargo fmt --check
cd "$PROJECT_ROOT" && cargo check
cd "$PROJECT_ROOT" && cargo clippy --all-targets
cd "$PROJECT_ROOT" && cargo test

# iOS, when $PROJECT_ROOT/iOS exists
cd "$PROJECT_ROOT/iOS" && make build
cd "$PROJECT_ROOT/iOS" && make sim-build

# Android, when $PROJECT_ROOT/Android exists
test -f "$PROJECT_ROOT/Android/local.properties"
grep -q "org.gradle.java.home" "$PROJECT_ROOT/Android/gradle.properties"
rustup target list --installed | grep android
cd "$PROJECT_ROOT/Android" && make build
cd "$PROJECT_ROOT/Android" && ./gradlew :shared:cargoBuild
cd "$PROJECT_ROOT/Android" && ./gradlew :app:assembleDebug
```

Record every host step in a structured list with these fields:

- `name` — stable step id such as `core.cargo-check`, `ios.make-build`, `android.gradlew-assembleDebug`, or `android.preflight-java21`.
- `passed` — boolean.
- `failure_snippet` — omitted or empty when passed; otherwise the first useful stderr/stdout lines.

Host prerequisite failures (missing `cargo`, `gradle`, `xcodebuild`, Java 21, Android SDK / NDK, `cargo-swift`, Android Rust targets, or an unusable Gradle wrapper) are host verification failures. They are not WASI tool failures and should be named as preflight steps (for example `android.preflight-java21`) so the journal makes the boundary clear.

When the slice modified neither the core nor a shell (e.g. a docs-only or UI-input-only slice that touched no Crux code), still run the applicable host checks after merge — the cap matrix as a whole must remain green. The post-merge gate intentionally validates the merged baseline, not the staged delta, because shell verification (UniFFI bridging, Java 21 / Gradle wrapper, cargo-swift, cap-marker expansion) is only meaningful once the spec-level deltas are promoted and the writers have a stable baseline to build against.

If the operator wants to re-confirm a scaffold before merge, the render step is `specify tool run vectis-scaffold -- ...` followed by the same explicit host post-processing / verification steps. This brief intentionally re-runs host verification post-merge so cross-cap regressions surface against the merged tree, not the pre-merge slice.

### Cap-matrix re-verification

The host verification gate walks the on-disk project — core (`shared/`) is verified when present; the `iOS/` and `Android/` assemblies are verified when their directories exist. The brief MUST verify the project root after the merge call, not a scratch scaffold; the goal is to confirm that the *merged* baseline still produces a buildable Vectis project, not that an arbitrary fresh scaffold from current pins does.

When host verification reports a failure, use the first failing step's `name` and `failure_snippet` as the load-bearing string in the journal `--summary`; route the full structured step list through `--context`.

Token, asset, and layout regressions surface through `specify tool run vectis-validate -- composition` before host verification runs. If a shell-local theme or asset emission problem survives into merge despite the validator, it will surface here as a downstream shell failure (e.g. the iOS shell's `make build` fails because a generated asset catalog is malformed). Treat that as the same `failure` mode below.

If the cap-matrix failure looks like a version-pin drift (e.g. AGP / Gradle / uniffi mismatch surfaced after pins changed in this slice), the matching diagnostic belongs to `/vectis:template-updater`, not this merge brief. Record the failure and surface it; the operator decides whether the next step is a template fix, a pin rollback, or a follow-up slice.

## Outcome signalling (Merge and adoption contract)

The Vectis capability merge brief is the slice loop's first capability-owned cap-matrix gate. RFC-13 §"Merge and adoption contract" pins the protocol: the brief decides go/no-go before `specify slice merge run`, and records any post-merge adoption-gate regression through `specify slice journal append` after the CLI has archived the slice. Capability diagnostics round-trip as opaque journal entries — the core does not parse them.

The shared phase contract (outcome values, journal kinds, the verbatim-`summary` rule, plan-mutation rules) is authored once at [`plugins/spec/references/phase-outcome-contract.md`](../../../plugins/spec/references/phase-outcome-contract.md). The three terminal branches below are the merge-phase deltas; the brief MUST pick exactly one before returning control.

### success — merge applied, cap matrix clean, slice archived

`specify slice merge run` exited zero, `specify tool run vectis-validate -- composition` had no blocking findings, and every required host verification step passed. The CLI atomically stamps `PhaseOutcome { phase: merge, outcome: success }` into `.metadata.yaml`, transitions the lifecycle to `merged`, and moves the slice directory into `.specify/archive/YYYY-MM-DD-<slice>/`.

The brief MUST NOT call `specify slice outcome set` on this path — the slice directory no longer exists under `.specify/slices/<slice>/` after archiving, so the call would fail with `not found`. The archived `.metadata.yaml` carries the success outcome; `/change:execute` reads it via `specify slice outcome show <slice>`, which falls back to the archive when the active directory is absent. See [`phase-outcome-contract.md`](../../../plugins/spec/references/phase-outcome-contract.md) §"Merge success path is CLI-stamped".

`/change:execute` translates `success` into a plan-entry transition to `done` and proceeds to the next entry.

### failure — merge halted or cap matrix rejected

This branch covers three distinct failure modes:

1. **`specify slice merge run` exited non-zero** (a delta could not be applied, baseline coherence failed inside the merge call, the lifecycle gate refused the call). The filesystem is unchanged: no baseline was written and the slice directory was not moved. Same shape as the omnia merge brief's `failure` branch.
2. **`specify tool run vectis-validate -- composition` returned blocking findings or failed to run after a successful `specify slice merge run`.** The deltas have already landed in the baseline (the CLI stamped `success` atomically), but the merged UI input set is invalid or the declared WASI validator could not run.
3. **One or more host verification steps failed or could not run after a successful `specify slice merge run`.** The deltas have already landed in the baseline, but the merged baseline no longer builds end-to-end, or the required host toolchain prerequisite is missing. Typical sub-cases:
   - Core `cargo fmt --check` / `cargo check` / `cargo clippy --all-targets` / `cargo test` failed against the merged `shared/` crate.
   - A specific platform shell (iOS `make build` / `make sim-build`, Android `make build` / `:shared:cargoBuild` / `:app:assembleDebug`) failed its compile pipeline.
   - A host prerequisite check failed (Java 21, Android SDK / NDK, Rust Android targets, Gradle wrapper, cargo-swift, Xcode command line tools).

For modes 2 and 3, **the brief MUST NOT attempt to roll back the merge** — `specify slice merge run` is not transactional with validation or host verification. Instead, journal the findings on the now-archived slice and surface the failure to the operator; the operator opens a follow-up slice (or `/spec:drop reason …` and re-defines) to repair the baseline.

In all three modes, record the failure on the slice — first journal the diagnostic, then stamp the outcome (when the slice is still under `.specify/slices/`):

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
# to .specify/archive/<…>/ and the call fails with `not found`. The archived
# `.metadata.yaml` carries the CLI-stamped `success` outcome from the
# `specify slice merge run` step; the journal `failure` entry is what surfaces
# the post-merge cap-matrix regression to the operator.
```

For mode 1, `/change:execute` reads the `failure` outcome and translates it into a plan-entry transition to `failed`, surfaces the journal entries to the operator, and stops the loop. For modes 2 and 3, `/change:execute` reads the CLI-stamped `success` outcome and proceeds with the next plan entry; the operator separately triages the journal `failure` entry on the archived slice and queues a repair slice when ready (typically `/vectis:template-updater` for upstream-pin drift, or a fresh `/spec:define` + `/spec:build` for handler regressions). In none of the modes does the brief retry the merge automatically — the failing delta or invalid baseline state needs human attention before a repeat attempt is safe.

`--summary` writing rules for post-merge findings: the load-bearing string is `"<validator-or-host-step-name>: <one-line failure snippet>"` (e.g. `"vectis-validate.composition: unresolved token colors.primary.dark"`, `"core.cargo-clippy: type mismatch in shared/src/app.rs:142"`, or `"android.gradlew-assembleDebug: unresolved reference 'CoreFfi'"`). Keep it short enough to fit a CLI argument without truncation; route the validator report, invocation stderr, or full structured host step list through `--context` instead. When the declared validator itself fails to start, use `"vectis-validate.composition could not run: <stderr first line>"`; when a host prerequisite is missing, use the failing preflight step name such as `"android.preflight-java21: Java 21 not configured"`.

### deferred — human judgement required

A merge prerequisite is unclear and `specify slice merge run` was never invoked. Typical triggers (same as omnia):

- The user declined the AskQuestion confirmation around the merge preview.
- `specify slice merge conflict-check` reported baseline drift (a sibling slice mutated the baseline after this slice was defined) that needs operator arbitration.
- `specify slice status` reports a lifecycle other than `complete` (e.g. `building`, `defining`) and the user declined to proceed.
- `specify slice validate` surfaced unmet `merge`-phase needs that the brief cannot resolve unattended.

Plus Vectis-specific triggers around upstream-pin and shell-toolchain decisions:

- A breaking external change (a new Crux core release with renamed exports, a uniffi bump that decouples from `crux_core::cli::bindgen`, an AGP / Gradle major bump, a cargo-swift bump) surfaced during this slice's build phase or after merge. The mechanical fix path may be `/vectis:template-updater`, but the operator needs to confirm whether to take the upstream change at all before the brief commits to a path forward.
- A version workflow or template-updater run succeeded against newer embedded defaults but the operator wants to pin to a specific older version (e.g. holding back AGP 9.x because of `rust-android-gradle 0.9.6` drift, or holding back `crux_core` for a known-incompatible downstream consumer). The bump that verified is not the bump the operator wants to land.

Record the deferral on the slice — first journal the question, then stamp the outcome:

```bash
specify slice journal append <slice> merge question \
  --summary "<the question the operator must answer>" \
  --context "<verbatim conflict-check report / preview diff / lifecycle status / validator report / host step list / template-updater output>"

specify slice outcome set <slice> merge deferred \
  --summary "<same question, present-tense, self-contained>"
```

`/change:execute` translates `deferred` into a plan-entry transition to `blocked`, surfaces the journal entries, and stops the loop. The brief MUST NOT silently fall through — recording the deferral is the only way the operator hears about it.

### Summary writing rules

The `--summary` strings ride into `/spec:drop reason` byte-for-byte when `/change:execute` reclaims a `failure` or `deferred` slice (see [`phase-outcome-contract.md`](../../../plugins/spec/references/phase-outcome-contract.md) §"Verbatim-`summary` rule"). Keep them present-tense, self-contained, and short enough to fit a CLI argument without truncation. Route any validator report, host step list, verbatim stderr, or log tail through `--context` instead — that field is not forwarded to `--reason`.
