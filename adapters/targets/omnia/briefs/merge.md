# Omnia target — merge brief

> `/spec:merge` loads this brief when the active `in-progress` plan entry has `target: omnia`. The brief gates entry into `specrun slice merge`; the CLI owns delta-merge, baseline coherence, the lifecycle transition to `merged`, and the archive move. The Omnia target adds no adapter-specific adoption mechanics on top of that flow — every artefact under `specs/` is promoted by the standard delta merge, and there are no extra format validators or generated outputs to refresh at merge time. This brief instead enforces the Omnia-specific *pre-merge* gate: the generated crate compiles, its tests pass, and the WASM target builds.

## Inputs and bindings

```text
$SLICE_NAME     = active in-progress plan entry's slice name
$SLICE_DIR      = .specify/slices/$SLICE_NAME
$CRATE_NAME     = $SLICE_NAME with kebab → snake (or the slice's plan-level `crate:` override)
$CRATE_PATH     = crates/$CRATE_NAME
$WORKSPACE_ROOT = repo root (carries the Cargo workspace `Cargo.toml` and the guest `src/lib.rs`)
```

## Critical path

1. Confirm the slice lifecycle is `built` (`specrun slice transition` from the build phase). If not, emit a stop hint (§ Stop hint contract) with `failure-kind: lifecycle-refused`.
2. Confirm every checkbox in `$SLICE_DIR/tasks.md` is complete; otherwise defer.
3. Run the § Omnia pre-merge gate (cargo + clippy + test + wasm32 build).
4. Run `specrun slice merge` per the [`spec-merge`](../../../../plugins/spec/skills/merge/SKILL.md) skill body — preview, conflict-check, AskQuestion confirmation, run.
5. On `specrun slice merge` exit zero the CLI atomically stamps the merge outcome, transitions the slice to `merged`, and moves it into `.specify/archive/`. `/spec:merge` returns control.

## § Omnia pre-merge gate

Run these from `$WORKSPACE_ROOT` (or `$CRATE_PATH` where noted). All four MUST pass before invoking `specrun slice merge`. Any failure halts the merge attempt and emits a stop hint (§ Stop hint contract).

### 1. Format and lint

```bash
cd $CRATE_PATH && cargo fmt --check
cd $CRATE_PATH && cargo clippy --all-targets -- -D warnings
```

Formatting failures are repaired with `cargo fmt` and the gate re-run. Clippy failures route back to `/spec:build` — emit a stop hint with the clippy output and stop the merge.

### 2. Workspace check

```bash
cargo check --workspace
```

Catches missing workspace members, broken `Cargo.toml` paths, and provider-trait mismatches that the slice's standalone build did not surface. A failure here typically means the slice introduced or renamed a crate that the workspace root has not been updated to include; re-enter `/spec:build` to repair `$WORKSPACE_ROOT/Cargo.toml`.

### 3. Test suite

```bash
cd $CRATE_PATH && cargo test
```

The build phase's verify-repair loop already enforces a passing test suite. Re-running here catches drift caused by sibling slices landing between the build phase exit and the merge attempt. A regression routes back to `/spec:build`; emit a stop hint with the failing tests named.

### 4. WASM target build

```bash
cargo build --target wasm32-wasip2 --release --workspace
```

The wasm32-wasip2 build is the definitive deployment-target check. A native `cargo check` will accept code that uses forbidden std APIs or non-WASM-compatible crates; only the wasm32 build proves the slice compiles for the real target. A failure here is a guardrail violation that the build phase missed; re-enter `/spec:build` with the wasm32 error output. Reference [`../references/guardrails.md`](../references/guardrails.md) for the forbidden crate / API table.

## § Delegation to `specrun slice merge`

After the pre-merge gate passes, follow the [`spec-merge`](../../../../plugins/spec/skills/merge/SKILL.md) skill body for the driver-side flow: slice selection, prerequisite checks, the AskQuestion confirmation around the merge preview, baseline-drift handling, and result rendering. The skill orchestrates `specrun slice merge preview`, `specrun slice merge conflict-check`, `specrun slice merge run`, and `specrun slice validate`. Omnia adds no adapter-specific adoption mechanics — the standard delta merge promotes every artefact under `specs/` and there are no extra format validators or generated outputs to refresh at merge time.

## § Stop hint contract

> See [Phase outcome contract](../../../../plugins/spec/references/phase-outcome-contract.md).

When the pre-merge gate or the CLI delta merge fails, emit a structured stop hint as the body's final output:

- `slice` — slice name from `specrun plan next`.
- `phase` — `merge`.
- `failure-kind` — one of `pre-merge-gate`, `baseline-conflict`, `lifecycle-refused`.
- `paths` — for `baseline-conflict`: the conflicting baseline files reported by `specrun slice merge`. For `pre-merge-gate`: the captured stderr or log path from the failing cargo/clippy/test/wasm32 step.
- `next-action` — `resolve and re-run /spec:merge $SLICE` for conflicts; `re-run /spec:build $SLICE` for gate failures classified as build regressions.

Lifecycle invariants: `pre-merge-gate` and `baseline-conflict` leave the slice at `built` and the plan entry at `in-progress`. Omnia adds no post-merge validator — a successful `specrun slice merge` is terminal for this brief.

## References

- [`../references/guardrails.md`](../references/guardrails.md) — Forbidden crates / std APIs the wasm32 build proves are absent.
- [`../references/runtime.md`](../references/runtime.md) — Identity OAuth env vars + `omnia::runtime!` host enumeration the workspace check exercises.
- [`plugins/spec/skills/merge/SKILL.md`](../../../../plugins/spec/skills/merge/SKILL.md) — Driver-side merge flow this brief delegates to.
