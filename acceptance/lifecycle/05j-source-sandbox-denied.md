---
id: source-sandbox-denied
owner: lifecycle
kind: suite
backend: fixture
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - out-of-sandbox-access-denied
  - project-dir-not-preopened
  - slice-stays-refining
  - operator-can-rebind-or-drop
expected-artifacts:
  - plan.yaml
---

# Source-adapter sandbox path-denied

Scenario ID: `source-sandbox-denied`

> **Automated (`backend: fixture`).** This scenario's structural assertions are deterministic and are proven by a fixture-driven test in the deterministic surface — no manual sweep run is required. See [Automated coverage](#automated-coverage).

## Intent

Prove the source-adapter sandbox holds: a `survey` or `extract` that attempts to read or write outside its bound `$SOURCE_DIR` / `$CAPABILITY_DIR` / `$SCRATCH_DIR` grants fails closed. `$PROJECT_DIR` is never a visible preopen, so lifecycle state is unreachable. For a `tool`-execution adapter the host runner denies the WASI access directly; for an `agent`-execution adapter, Evidence staged outside the granted `$SCRATCH_DIR` is rejected at finalize with `extract-evidence-missing`. Either way the slice stays `refining`, no Evidence is persisted, and the operator can rebind via `plan amend` or drop the source.

## Automated coverage

The sandbox assertions are deterministic CLI/host behavior — no LLM prose is involved — so they are proven by a fixture-driven test rather than the manual sweep:

- Test: `sandbox_denies_out_of_scope` in [`augentic/specify-cli` `tests/source_extract.rs`](https://github.com/augentic/specify-cli/blob/main/tests/source_extract.rs).
- Runs under `cargo make test` (and `cargo nextest run --test source_extract`) on every commit, as part of the deterministic surface in [`docs/contributing/acceptance.md`](../../docs/contributing/acceptance.md).

Assertion → coverage map:

- `plan-exists`: the test seeds a plan with a path-bound source before extracting.
- `project-dir-not-preopened`: the prepare-phase handoff envelope carries no `project-dir`, and every granted root (`briefs-dir`, `source-dir`, `scratch-dir`, `evidence-dir`) is a strict descendant of the project root — the project root itself is never a grant.
- `out-of-sandbox-access-denied`: Evidence staged outside the granted `$SCRATCH_DIR` (at the project root that `$PROJECT_DIR: none` makes unreachable) is denied — finalize fails closed with `extract-evidence-missing` (exit code 1).
- `slice-stays-refining`: no Evidence lands on the slice path and no cache event is emitted, so the slice never transitions out of `refining`.
- `operator-can-rebind-or-drop`: the recovery mechanic (rebind via `specify plan amend`, or `specify slice drop`) is covered by the general plan-amend / slice-drop command tests, not specific to this scenario.

## Reproducing by hand (optional)

The fixture test is the source of truth; the manual steps below only reproduce it for inspection. Follow the **single-project setup** in [`shared/setup.md`](../shared/setup.md) with `specify init omnia@v1`, bind a source adapter configured to attempt an out-of-sandbox access during `survey`/`extract`, plan a one-slice change named `escape-attempt`, stamp Gate 1, then `/spec:refine` and capture the denied access. Recover with `specify plan amend` to rebind the source, or drop it.
