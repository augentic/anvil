---
id: merge
description: Merge the Vectis slice into the repository
needs: [build]
---

Before merging, confirm all task checkboxes in `tasks.md` are complete and the slice status is `complete`. Delta-spec merging, baseline coherence validation, the lifecycle transition, and the archive move are delegated to the `specify` CLI: `specify slice merge preview`, `specify slice merge conflict-check`, `specify slice merge run`, and `specify slice validate`.

Follow the [`specify-merge`](../../../plugins/spec/skills/merge/SKILL.md) skill for the driver-side flow — slice selection, prerequisite checks, the AskQuestion confirmation around the merge preview, baseline-drift handling, and result rendering. The Vectis capability adds **one capability-specific gate** on top of that flow: the post-merge cap-matrix re-verification via the standalone `specify-vectis` binary (RFC-13 §4.3a + §"Merge and adoption contract"). The `specify slice merge run` step promotes spec deltas (markdown) and composition deltas (YAML) under `.specify/specs/` and `.specify/specs/composition.yaml`; this brief then re-runs `specify-vectis verify` against the project root to confirm the resulting baseline still scaffolds and compiles end-to-end.

The `specify slice merge run` command merges both spec deltas and composition deltas in a single operation. The merge surface is broader than spec / design / task deltas: per RFC-11 §I "Merge handoff", `composition.yaml`, `tokens.yaml`, `assets.yaml`, and any referenced asset files under `design-system/assets/**` (or slice-local `assets/`) are reviewable lifecycle artifacts when they appear in a slice. `composition.yaml` continues to merge into the Specify baseline; token and asset updates merge into `design-system/tokens.yaml`, `design-system/assets.yaml`, and `design-system/assets/**` respectively. Review every UI input delta alongside the spec / design / task changes in the `specify slice merge preview` output before confirming the merge so reviewers can understand which downstream shell generations will be affected.

After `specify slice merge run` succeeds, re-run the deterministic UI input validator against the merged baseline:

```bash
specify-vectis validate composition
```

The validator discovers the now-merged baseline `composition.yaml` and auto-invokes `tokens` / `assets` modes against any sibling `tokens.yaml` / `assets.yaml`. Run this even when the current slice did not generate any platform code, because later shell work may consume the merged baseline input set (RFC-11 §I "Merge handoff"). The same exit semantics apply: errors block merge finalisation, warnings flow into the operator-facing summary, clean runs are silent. When `composition.yaml` is absent from the merged baseline (no UI input set in the project), the validator exits cleanly without performing wired-mode checks.

## Capability-specific adoption gate

After `specify slice merge run` exits zero (i.e. the slice's `specs/` and `composition.yaml` deltas have been promoted into the baseline and the lifecycle has transitioned to `merged`), shell out to `specify-vectis verify` against the now-updated project:

```bash
specify-vectis verify --dir "$PROJECT_ROOT" --format json > /tmp/vectis-verify.json
case $? in
  0) ;;  # clean — every assembly compiles end-to-end
  *) ;;  # non-zero — record `failure` (see §failure below)
esac
```

`specify-vectis verify` is the canonical end-to-end gate for the Vectis cap matrix. It runs the per-assembly pipeline — `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo deny check`, `cargo vet`, Swift codegen, Kotlin codegen, plus shell-specific compile steps when iOS or Android are present — and emits a structured JSON envelope listing each step. Failures include the first N lines of combined stdout/stderr per failing step, which the brief threads into `--context` on the journal entry below.

When the slice modified neither the core nor a shell (e.g. a docs-only or UI-input-only slice that touched no Crux code), still run the verifier after merge — the cap matrix as a whole must remain green, and the binary is cheap on a clean baseline. The post-merge gate intentionally validates the merged baseline, not the staged delta, because shell verification (UniFFI bridging, Java 21 / Gradle wrapper, cargo-swift, cap-marker expansion) is only meaningful once the spec-level deltas are promoted and the writers have a stable baseline to scaffold against.

If the operator wants to re-confirm an individual scaffold or rebuild the full cap matrix before merge, the equivalent pre-merge gate is `specify-vectis verify --dir "$PROJECT_ROOT"` invoked through `/spec:build`. This brief intentionally re-runs the same binary post-merge so cross-cap regressions (e.g. an `sse` slice that lands cleanly in isolation but breaks the `http,kv,time,platform,sse` combo's `cargo deny` gate) surface against the merged tree, not the pre-merge slice.

### Cap-matrix re-verification

`specify-vectis verify`'s default `--dir` invocation walks the on-disk project — core (`shared/`) is always verified; the `iOS/` and `Android/` assemblies are auto-detected and verified when their directories exist. The brief MUST run the binary against the project root after the merge call, not against a scratch scaffold; the goal is to confirm that the *merged* baseline still produces a buildable Vectis project, not that an arbitrary fresh scaffold from current pins does.

When the verifier reports a failure, the JSON envelope's `assemblies.{core,ios,android}.steps[]` array identifies the first failing step per assembly (each step carries `name` + `passed` + `error`). Use the assembly + step name as the load-bearing string in the journal `--summary`; route the full envelope (or the failing step's stderr tail) through `--context`.

Token, asset, and layout regressions surface through `specify-vectis validate composition` before the cap-matrix verifier runs. If a shell-local theme or asset emission problem survives into merge despite the validator, it will surface here as a downstream shell failure (e.g. the iOS shell's `make build` fails because a generated asset catalog is malformed). Treat that as the same `failure` mode below.

If the cap-matrix failure looks like a version-pin drift (e.g. `cargo deny check` flagged a new RUSTSEC advisory, AGP / Gradle / uniffi mismatch surfaced after an `update-versions` ran in this slice), `specify-vectis update-versions --verify` is the matching diagnostic — but it is *not* the merge brief's job to run. Record the failure and surface it; the operator decides whether the next step is `/vectis:template-updater` (template fix), a pin rollback, or a follow-up slice.

## Outcome signalling (Merge and adoption contract)

The Vectis capability merge brief is the slice loop's first capability-owned cap-matrix gate. RFC-13 §"Merge and adoption contract" pins the protocol: the brief decides go/no-go and signals it through `specify slice outcome set` plus `specify slice journal append`. The core proceeds with archival on `success` and halts on `failure` / `deferred`, surfacing the journal entries to the operator. Capability diagnostics round-trip as opaque journal entries — the core does not parse them.

The shared phase contract (outcome values, journal kinds, the verbatim-`summary` rule, plan-mutation rules) is authored once at [`plugins/spec/references/phase-outcome-contract.md`](../../../plugins/spec/references/phase-outcome-contract.md). The three terminal branches below are the merge-phase deltas; the brief MUST pick exactly one before returning control.

### success — merge applied, cap matrix clean, slice archived

`specify slice merge run` exited zero AND `specify-vectis verify` exited `0`. The CLI atomically stamps `PhaseOutcome { phase: merge, outcome: success }` into `.metadata.yaml`, transitions the lifecycle to `merged`, and moves the slice directory into `.specify/archive/YYYY-MM-DD-<slice>/`.

The brief MUST NOT call `specify slice outcome set` on this path — the slice directory no longer exists under `.specify/slices/<slice>/` after archiving, so the call would fail with `not found`. The archived `.metadata.yaml` carries the success outcome; `/change:execute` reads it via `specify slice outcome show <slice>`, which falls back to the archive when the active directory is absent. See [`phase-outcome-contract.md`](../../../plugins/spec/references/phase-outcome-contract.md) §"Merge success path is CLI-stamped".

`/change:execute` translates `success` into a plan-entry transition to `done` and proceeds to the next entry.

### failure — merge halted or cap matrix rejected

This branch covers three distinct failure modes:

1. **`specify slice merge run` exited non-zero** (a delta could not be applied, baseline coherence failed inside the merge call, the lifecycle gate refused the call). The filesystem is unchanged: no baseline was written and the slice directory was not moved. Same shape as the omnia merge brief's `failure` branch.
2. **`specify-vectis verify` returned non-zero after a successful `specify slice merge run`.** The deltas have already landed in the baseline (the CLI stamped `success` atomically), but the merged baseline no longer scaffolds and compiles end-to-end. Typical sub-cases:
   - Core `cargo check` / `clippy` / `cargo deny check` / `cargo vet` failed against the merged `shared/` crate.
   - A specific platform shell (iOS `xcodebuild` / `make build`, Android `:shared:cargoBuild` / `:app:assembleDebug`) failed its compile pipeline.
   - `specify-vectis update-versions --verify` was run earlier in the slice and at the time succeeded, but the post-merge baseline now disagrees (e.g. a sibling slice's `Cargo.toml` edit moved a transitive pin into a window that fails `cargo deny`).
3. **`specify-vectis update-versions --verify` failed during the slice's build phase to find a verifying combination at all** — the slice attempted a version bump and no cap-matrix combo passed. This case typically surfaces *before* `specify slice merge run` is called (the build brief will have refused to mark tasks complete), but a slice that papered over the failure manually surfaces it here as a `failure` after `merge run` succeeds and post-merge `specify-vectis verify` re-runs the matrix. **The brief MUST NOT attempt to roll back the merge** — `specify slice merge run` is not transactional with the verifier. Instead, journal the verifier's findings on the now-archived slice and surface the failure to the operator; the operator opens a follow-up slice (or `/spec:drop --reason …` and re-defines) to repair the baseline.

In all three modes, record the failure on the slice — first journal the diagnostic, then stamp the outcome (when the slice is still under `.specify/slices/`):

```bash
# Mode 1: pre-merge failure (slice is still active)
specify slice journal append <slice> merge failure \
  --summary "<which CLI step failed and the load-bearing stderr line>" \
  --context "<verbatim stderr / coherence-check tail / failing delta path>"

specify slice outcome set <slice> merge failure \
  --summary "<same load-bearing summary, written so it is useful as a /spec:drop --reason>"
```

```bash
# Modes 2 and 3: post-merge cap-matrix failure (slice is already archived)
specify slice journal append <slice> merge failure \
  --summary "<assembly>.<step-name>: <one-line restatement of the first failing step's error>" \
  --context "<verbatim contents of /tmp/vectis-verify.json>"

# Do NOT call `specify slice outcome set` — the slice directory has been moved
# to .specify/archive/<…>/ and the call fails with `not found`. The archived
# `.metadata.yaml` carries the CLI-stamped `success` outcome from the
# `specify slice merge run` step; the journal `failure` entry is what surfaces
# the post-merge cap-matrix regression to the operator.
```

For mode 1, `/change:execute` reads the `failure` outcome and translates it into a plan-entry transition to `failed`, surfaces the journal entries to the operator, and stops the loop. For modes 2 and 3, `/change:execute` reads the CLI-stamped `success` outcome and proceeds with the next plan entry; the operator separately triages the journal `failure` entry on the archived slice and queues a repair slice when ready (typically `/vectis:template-updater` for upstream-pin drift, or a fresh `/spec:define` + `/spec:build` for handler regressions). In none of the modes does the brief retry the merge automatically — the failing delta or invalid baseline state needs human attention before a repeat attempt is safe.

`--summary` writing rules for verifier findings: the load-bearing string is `"<assembly>.<step-name>: <one-line restatement of the first failing step's stderr>"` (e.g. `"core.cargo-clippy: type mismatch in shared/src/app.rs:142"` or `"android.gradlew-assembleDebug: unresolved reference 'CoreFfi'"`). Keep it short enough to fit a CLI argument without truncation; route the full JSON envelope or stderr tail through `--context` instead. When `specify-vectis verify` itself fails to start (missing prerequisite — exit `2` — or any other invocation error), use `"specify-vectis verify could not run: <stderr first line>"` and put the full stderr on `--context`.

### deferred — human judgement required

A merge prerequisite is unclear and `specify slice merge run` was never invoked. Typical triggers (same as omnia):

- The user declined the AskQuestion confirmation around the merge preview.
- `specify slice merge conflict-check` reported baseline drift (a sibling slice mutated the baseline after this slice was defined) that needs operator arbitration.
- `specify slice status` reports a lifecycle other than `complete` (e.g. `building`, `defining`) and the user declined to proceed.
- `specify slice validate` surfaced unmet `merge`-phase needs that the brief cannot resolve unattended.

Plus Vectis-specific triggers around upstream-pin and shell-toolchain decisions:

- A breaking external change (a new Crux core release with renamed exports, a uniffi bump that decouples from `crux_core::cli::bindgen`, an AGP / Gradle major bump, a cargo-swift bump) surfaced during this slice's build phase or after merge. The mechanical fix path may be `/vectis:template-updater`, but the operator needs to confirm whether to take the upstream change at all before the brief commits to a path forward.
- `specify-vectis update-versions --verify` succeeded against the embedded defaults but the operator wants to pin to a specific older version (e.g. holding back AGP 9.x because of `rust-android-gradle 0.9.6` drift, or holding back `crux_core` for a known-incompatible downstream consumer). The bump that the verifier accepted is not the bump the operator wants to land.

Record the deferral on the slice — first journal the question, then stamp the outcome:

```bash
specify slice journal append <slice> merge question \
  --summary "<the question the operator must answer>" \
  --context "<verbatim conflict-check report / preview diff / lifecycle status / specify-vectis verify or update-versions output>"

specify slice outcome set <slice> merge deferred \
  --summary "<same question, present-tense, self-contained>"
```

`/change:execute` translates `deferred` into a plan-entry transition to `blocked`, surfaces the journal entries, and stops the loop. The brief MUST NOT silently fall through — recording the deferral is the only way the operator hears about it.

### Summary writing rules

The `--summary` strings ride into `/spec:drop --reason` byte-for-byte when `/change:execute` reclaims a `failure` or `deferred` slice (see [`phase-outcome-contract.md`](../../../plugins/spec/references/phase-outcome-contract.md) §"Verbatim-`summary` rule"). Keep them present-tense, self-contained, and short enough to fit a CLI argument without truncation. Route any verbatim stderr, verifier JSON, or log tail through `--context` instead — that field is not forwarded to `--reason`.
