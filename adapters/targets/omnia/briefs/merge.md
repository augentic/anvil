# Omnia target — merge brief

> `/spec:merge` loads this brief when the active `in-progress` plan entry has `target: omnia`. The brief gates entry into `specify slice merge`; the CLI owns delta-merge, baseline coherence, the lifecycle transition to `merged`, and the archive move. The Omnia target adds no adapter-specific adoption mechanics on top of that flow — every artefact under `specs/` is promoted by the standard delta merge, and there are no extra format validators or generated outputs to refresh at merge time. This brief instead enforces the Omnia-specific *pre-merge* gate: the generated crate compiles, its tests pass, and the WASM target builds.

## Inputs and bindings

```text
$SLICE_NAME     = active in-progress plan entry's slice name
$SLICE_DIR      = .specify/slices/$SLICE_NAME
$CRATE_NAME     = $SLICE_NAME with kebab → snake (or the slice's plan-level `crate:` override)
$CRATE_PATH     = crates/$CRATE_NAME
$WORKSPACE_ROOT = repo root (carries the Cargo workspace `Cargo.toml` and the guest `src/lib.rs`)
```

## Critical path

1. Confirm the slice lifecycle is `built` (`specify slice transition` from the build phase). If not, defer — see § Outcome signalling.
2. Confirm every checkbox in `$SLICE_DIR/tasks.md` is complete; otherwise defer.
3. Run the § Omnia pre-merge gate (cargo + clippy + test + wasm32 build).
4. Run `specify slice merge` per the [`spec-merge`](../../../../plugins/spec/skills/merge/SKILL.md) skill body — preview, conflict-check, AskQuestion confirmation, run.
5. On `specify slice merge` exit zero the CLI atomically stamps the merge outcome, transitions the slice to `merged`, and moves it into `.specify/archive/`. `/spec:merge` returns control.

## § Omnia pre-merge gate

Run these from `$WORKSPACE_ROOT` (or `$CRATE_PATH` where noted). All four MUST pass before invoking `specify slice merge`. Any failure halts the merge attempt and emits a `failure` outcome via § Outcome signalling.

### 1. Format and lint

```bash
cd $CRATE_PATH && cargo fmt --check
cd $CRATE_PATH && cargo clippy --all-targets -- -D warnings
```

Formatting failures are repaired with `cargo fmt` and the gate re-run. Clippy failures route back to `/spec:build` — emit a `failure` outcome with the clippy output and stop the merge.

### 2. Workspace check

```bash
cargo check --workspace
```

Catches missing workspace members, broken `Cargo.toml` paths, and provider-trait mismatches that the slice's standalone build did not surface. A failure here typically means the slice introduced or renamed a crate that the workspace root has not been updated to include; re-enter `/spec:build` to repair `$WORKSPACE_ROOT/Cargo.toml`.

### 3. Test suite

```bash
cd $CRATE_PATH && cargo test
```

The build phase's verify-repair loop already enforces a passing test suite. Re-running here catches drift caused by sibling slices landing between the build phase exit and the merge attempt. A regression routes back to `/spec:build`; emit a `failure` outcome with the failing tests named.

### 4. WASM target build

```bash
cargo build --target wasm32-wasip2 --release --workspace
```

The wasm32-wasip2 build is the definitive deployment-target check. A native `cargo check` will accept code that uses forbidden std APIs or non-WASM-compatible crates; only the wasm32 build proves the slice compiles for the real target. A failure here is a guardrail violation that the build phase missed; re-enter `/spec:build` with the wasm32 error output. Reference [`../references/guardrails.md`](../references/guardrails.md) for the forbidden crate / API table.

## § Delegation to `specify slice merge`

After the pre-merge gate passes, follow the [`spec-merge`](../../../../plugins/spec/skills/merge/SKILL.md) skill body for the driver-side flow: slice selection, prerequisite checks, the AskQuestion confirmation around the merge preview, baseline-drift handling, and result rendering. The skill orchestrates `specify slice merge preview`, `specify slice merge conflict-check`, `specify slice merge run`, and `specify slice validate`. Omnia adds no adapter-specific adoption mechanics — the standard delta merge promotes every artefact under `specs/` and there are no extra format validators or generated outputs to refresh at merge time.

## § Outcome signalling

Phase outcomes follow the shared phase contract in [`plugins/spec/references/phase-outcome-contract.md`](../../../../plugins/spec/references/phase-outcome-contract.md). The three terminal branches:

### success — merge applied, slice archived

`specify slice merge run` exited zero. The CLI atomically stamps `PhaseOutcome { phase: merge, outcome: success }` into `.metadata.yaml`, transitions the lifecycle to `merged`, and moves the slice directory into `.specify/archive/YYYY-MM-DD-<slice>/`.

The brief MUST NOT call `specify slice outcome set` on this path — the slice directory no longer exists under `.specify/slices/<slice>/` after archiving, so the call would fail with `not found`. `/spec:execute` translates `success` into a plan-entry transition to `done` and proceeds to the next entry.

### failure — merge halted, baselines untouched

Triggered by any pre-merge gate failure (§ Omnia pre-merge gate) or a non-zero exit from `specify slice merge run`. The filesystem is unchanged: no baseline was written, and the slice directory was not moved.

Record the failure on the slice — first journal the diagnostic, then stamp the outcome:

```bash
specify slice journal append $SLICE_NAME merge failure \
  --summary "<which gate or CLI step failed; the load-bearing stderr line>" \
  --context "<verbatim stderr or test / clippy / wasm32 build output>"

specify slice outcome set $SLICE_NAME merge failure \
  --summary "<same load-bearing summary, present-tense and self-contained>"
```

`/spec:execute` translates `failure` into a plan-entry transition to `failed`, surfaces the journal entries, and stops the loop. The brief MUST NOT retry automatically — the failing gate or delta needs operator attention before a repeat attempt is safe.

### deferred — human judgement required

Typical triggers:

- Slice lifecycle is not `built` (e.g. `refining`, `refined`, `building`); the build phase still has work.
- `tasks.md` has unchecked boxes.
- The user declined the AskQuestion confirmation around the merge preview.
- `specify slice merge conflict-check` reported baseline drift (a sibling slice mutated the baseline after this slice was defined) that needs operator arbitration.
- `specify slice validate` surfaced unmet `merge`-phase needs that the brief cannot resolve unattended.

```bash
specify slice journal append $SLICE_NAME merge question \
  --summary "<the question the operator must answer>" \
  --context "<verbatim conflict-check report / preview diff / lifecycle status>"

specify slice outcome set $SLICE_NAME merge deferred \
  --summary "<same question, present-tense, self-contained>"
```

`/spec:execute` translates `deferred` into a plan-entry transition to `blocked`, surfaces the journal entries, and stops the loop. The brief MUST NOT silently fall through — recording the deferral is the only way the operator hears about it.

### Summary writing rules

The `--summary` strings ride into `/spec:drop reason` byte-for-byte when `/spec:execute` reclaims a `failure` or `deferred` slice. Keep them present-tense, self-contained, and short enough to fit a CLI argument without truncation. Route verbatim stderr or log tails through `--context` — that field is not forwarded to `--reason`.

## References

- [`../references/guardrails.md`](../references/guardrails.md) — Forbidden crates / std APIs the wasm32 build proves are absent.
- [`../references/runtime.md`](../references/runtime.md) — Identity OAuth env vars + `omnia::runtime!` host enumeration the workspace check exercises.
- [`plugins/spec/skills/merge/SKILL.md`](../../../../plugins/spec/skills/merge/SKILL.md) — Driver-side merge flow this brief delegates to.
- [`plugins/spec/references/phase-outcome-contract.md`](../../../../plugins/spec/references/phase-outcome-contract.md) — Outcome / journal / verbatim-summary rules.
