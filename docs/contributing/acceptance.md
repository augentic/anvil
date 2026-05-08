# Running The Acceptance Suite

> Audience: contributors and release managers working in `augentic/specify`.
> Source of truth for the design: [RM-01 Acceptance Framework](../../rfcs/rm-01-acceptance-framework.md) and the [implementation plan](../../rfcs/rm-01-acceptance-framework-implementation-plan.md).
> Companion: [Consistency Checks](checks.md) (the `make checks` reference).

The acceptance framework lives under [`acceptance/`](../../acceptance/README.md). It is the layer that proves skill markdown, capability briefs, fixtures, and workflow orchestration still drive the `specify` CLI substrate correctly. Rust CLI mechanics are owned by `specify-cli` and are not duplicated here.

This page is the operator-facing guide: which target to run when, how to set `SPECIFY_BIN`, what the smoke catalog covers, how to read a failure, and how the CI tier matrix slices the suite by touched files.

## Running The Suite

The framework exposes a small `make` surface. Every target either runs to a clean exit or skips gracefully when its prerequisite tools (notably the `specify` CLI) are missing.

| Target                                  | Tier   | Roughly | What it proves                                                                 |
| --------------------------------------- | ------ | ------- | ------------------------------------------------------------------------------ |
| `make checks`                           | 0      | < 2s    | Static framework checks (markdown links, scenario frontmatter, recorded-trace headers, retired-verb hygiene). Required on every PR. |
| `make acceptance-smoke`                 | 1      | < 1s    | Narrow contracts scenario via the `fixture` backend; proves the runner end-to-end without live model behavior. |
| `make acceptance-stub-smoke`            | 1      | < 1s    | Deterministic stub backend driving the contracts-describe slice loop end-to-end through the real `specify` CLI. |
| `make acceptance-cross-repo-recorded-smoke` | 1   | ~ 1s    | Re-runs the checked-in RM-01 trace (`acceptance/recorded/rm01-cross-repo/baseline.jsonl`) against the live binary; pins CLI substrate behavior. |
| `make acceptance-cross-repo-setup-smoke`    | 2   | ~ 1s    | Hub + two registered projects + workspace sync; setup-* assertions only. |
| `make acceptance-cross-repo-plan-smoke`     | 2   | ~ 1s    | Setup → 3-entry deterministic plan; setup-* + plan-* assertions. |
| `make acceptance-cross-repo-execute-smoke`  | 2   | ~ 2.5s  | Adds the deterministic execute loop; setup-* + plan-* + execute-* assertions. |
| `make acceptance-cross-repo-finalize-smoke` | 2   | ~ 4s    | Adds workspace push, fake-gh PR mark-merged, change finalize, idempotency probe. |
| `make acceptance-cross-repo-contracts-build-smoke` | 3 | ~ 3s | Contract slice emits an OpenAPI 3.1 + JSON Schema bundle the contract WASI tool can validate. |
| `make acceptance-cross-repo-omnia-build-smoke`     | 3 | ~ 3s | Backend slice emits a Rust crate skeleton (Cargo.toml + src/{lib,providers}.rs) into the routed clone. |
| `make acceptance-cross-repo-vectis-build-smoke`    | 3 | ~ 3s | Mobile slice emits a Vectis composition + SwiftUI shell into the routed mobile clone. |
| `make acceptance-cross-repo-define-smoke`          | 4 | ~ 4s | Operator-driven define run (`/spec:define` via the `agent` backend). Requires `OPERATOR_RESULTS` env, or the runner's `cursor-sdk` flag (see [`agent` backend docs](../../acceptance/runner/backends/README.md)). |

Aggregators:

| Target                                       | Runs                                                                                          | Use when                                                          |
| -------------------------------------------- | --------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `make acceptance-cross-repo`                 | All nine cross-repo smokes (setup, plan, execute, finalize, define, contracts/omnia/vectis-build, recorded). `define` skips when `OPERATOR_RESULTS` is unset. | Manual cross-repo sweep; release rehearsals; operator-driven runs. |
| `make acceptance-cross-repo-deterministic`   | Eight cross-repo smokes minus `define`.                                                       | Unattended runs and CI; never blocks on operator-supplied JSON.    |
| `make acceptance-all`                        | `acceptance-smoke` + `acceptance-stub-smoke` + `acceptance-cross-repo`.                        | Full pre-tag rehearsal of every layer at once.                     |

Aggregators print a single PASS / SKIP / FAIL summary table at the end, never fail-fast, and capture per-target stdout/stderr to a temp logs directory. On any failure the captured log is re-emitted to the aggregator console so the operator does not have to open the file.

## When To Run What

A small Deno helper at [`scripts/acceptance-tier.ts`](../../scripts/acceptance-tier.ts) reads the changed-file list from `git diff --name-only` and prints the recommended `make` targets for that change. The mapping mirrors §C16 of the implementation plan:

| Touched paths                                                                 | Recommended targets (in addition to `make checks`)                                       |
| ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `acceptance/runner/**`, `acceptance/assertions/**`                            | All Tier 1 + Tier 2 + every Tier 3 build smoke (a runner change can affect everything).  |
| `capabilities/contracts/**`, `plugins/contract/**`, `acceptance/recorded/**`  | Tier 1 + `acceptance-cross-repo-contracts-build-smoke`.                                  |
| `capabilities/omnia/**`, `plugins/omnia/**`                                   | Tier 1 + `acceptance-cross-repo-omnia-build-smoke`.                                      |
| `capabilities/vectis/**`, `plugins/vectis/**`                                 | Tier 1 + `acceptance-cross-repo-vectis-build-smoke`.                                     |
| `plugins/spec/**`, `plugins/change/**`                                        | Tier 1 + Tier 2 (deterministic flows: setup, plan, execute, finalize).                   |
| `Makefile`, `scripts/checks.ts`, `scripts/acceptance-{tier,aggregate}.ts`, `acceptance/**` (general) | Tier 0 + Tier 1.                                                                         |
| Anything else (docs, RFCs, READMEs unrelated to acceptance)                    | Tier 0 only (`make checks`).                                                             |

Use it directly:

```bash
make acceptance-tiers                                  # selection only (one target per line)
make acceptance-tiers TIER_ARGS='--explain'            # selection plus per-file rationale on stderr
make acceptance-tiers TIER_ARGS='--files "Makefile capabilities/contracts/tests/describe.md"'
```

To execute the recommended targets:

```bash
make $(make acceptance-tiers)
```

The selector emits `make checks` for empty diffs (the Tier 0 floor); it never selects nothing.

## Setting `SPECIFY_BIN`

Every cross-repo smoke shells out to `specify`. The drivers resolve the binary in this order:

1. `$SPECIFY_BIN` if set and executable.
2. `specify` on `PATH`.
3. Otherwise the smoke prints `[skip]` and exits 0.

The framework binary is built from [`augentic/specify-cli`](https://github.com/augentic/specify-cli):

```bash
# In the specify-cli checkout:
cargo build --release

# In this repo:
export SPECIFY_BIN=/absolute/path/to/specify-cli/target/release/specify
make acceptance-cross-repo
```

The system `specify` (e.g. v0.1.0 from a stock install) predates the RFC-9 surface (`init --hub`, `change plan {create, next, transition}`, `workspace prepare-branch`, `change finalize`); the smokes detect this and skip rather than fail. Set `SPECIFY_BIN` to the freshly built release binary for real coverage.

## Operator-driven Runs

The `agent` backend is reserved for live `/spec:define` execution. `acceptance-cross-repo-define-smoke` invokes that backend and expects either an operator-supplied results JSON or a Cursor SDK path:

```bash
make acceptance-cross-repo-define-smoke OPERATOR_RESULTS=/path/to/operator-results.json
```

A worked example lives at [`acceptance/suites/rm01-cross-repo/operator-results.example.json`](../../acceptance/suites/rm01-cross-repo/operator-results.example.json). Without `OPERATOR_RESULTS` (or `--cursor-sdk`) the smoke skips with exit 0; the operator-driven path is intentionally opt-in.

`make acceptance-cross-repo` includes the define smoke; `make acceptance-cross-repo-deterministic` deliberately omits it so unattended CI never depends on operator-supplied artifacts.

## Failure Attribution (Fault-domain Taxonomy)

When a smoke fails the runner classifies the failure into one of the fault domains documented in [RM-01 Acceptance Framework §"Risks"](../../rfcs/rm-01-acceptance-framework.md). Use this taxonomy when reading aggregator output:

| Domain                         | What to look for                                                                                  | Likely owner / next step                                       |
| ------------------------------ | ------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `cli-substrate`                | Live `specify` exited non-zero where the recorded trace exited zero, or surfaced an unknown verb. | `specify-cli` regression — open an issue in `augentic/specify-cli`. |
| `skill-orchestration`          | Stub / scripted backend drove the wrong slice, transition, or branch order.                        | Skill markdown change in `plugins/spec/**` or `plugins/change/**`. |
| `capability-brief`             | A brief omitted / mis-named the artifacts a capability requires.                                  | The capability under `capabilities/<cap>/`.                    |
| `specialist-generation`        | A `*-build` smoke produced a missing or malformed crate / contract / mobile residue.              | The matching plugin under `plugins/<omnia|vectis|contract>/`.  |
| `runner-setup`                 | Hub / projects / fake forge configuration failed before any assertion ran.                        | `acceptance/runner/{hub,projects,workspace,fake-gh}.ts`.       |
| `external-fake`                | Fake `gh` / local bare remotes diverged from the production forge contract.                       | `acceptance/runner/fake-gh.ts` and `acceptance/runner/git.ts`. |
| `live-agent-nondeterminism`    | Recorded vs live trace diverged in shape (not exit code) — likely model wording drift.            | Re-record the trace; see "Regenerating Recorded Traces" below. |

The aggregator preserves every per-target log under `${TMPDIR}/specify-acceptance-<label>-<hash>/<target>.log` so the failing run is reviewable without re-running.

## Regenerating Recorded Traces

`acceptance/recorded/<suite>/*.jsonl` traces back the `recorded` backend (Tier 1). They are deterministic CLI substrate regression pins, not goldens of model wording. The full regen procedure lives in [`acceptance/runner/backends/README.md` §"Regenerating A Recorded Trace"](../../acceptance/runner/backends/README.md). In short:

1. Run a trusted live source (`scripted-execute`, `scripted-finalize`, or `agent`) with the run dir preserved.
2. Concatenate the captured tool-calls, prepend a `recorded-trace-header` line carrying `schemaVersion: 1`, `sourceBackend`, `sourceRunId`, `sourceTimestamp`, `scenarioId`, and `scenarioId`.
3. Replace the absolute hub-dir prefix in every `cwd` with the literal `<hubDir>` so the trace is portable.
4. Drop the new file in place; `make checks` validates the header on every PR.

When you commit a recorded trace, quote the `sourceRunId` and `sourceTimestamp` from the header in the commit message. `scripts/checks.ts` prints a non-fatal `WARN:` line on every PR that touches a trace, reminding the operator to disclose the source run; the warning is suppressed when `git` is unavailable (shallow clones / no `--allow-run`) so it never blocks a push.

## GitHub Actions Workflow

The repository ships [`.github/workflows/acceptance.yml`](../../.github/workflows/acceptance.yml) alongside the existing `ci.yaml` (which owns the Tier 0 `make checks` job for every PR). The acceptance workflow is split into five jobs that mirror the tier matrix:

- `tier-0` — re-runs `make checks` so the workflow is self-contained when triggered via `workflow_dispatch`.
- `tier-1-and-2` — narrow + cross-repo deterministic smokes on every PR. Builds `specify-cli` from `main` (override via the `SPECIFY_CLI_REF` env), exports `SPECIFY_BIN`, and runs `make acceptance-smoke` + `make acceptance-stub-smoke` + `make acceptance-cross-repo-deterministic`.
- `tier-3-{contracts,omnia,vectis}` — specialist build smokes. Each is gated by a `dorny/paths-filter` step so only the changed area runs; a runner / assertion change triggers all three (matching the catch-all rule in `scripts/acceptance-tier.ts`).

Failure artifacts are uploaded with `actions/upload-artifact@v4` and `if: failure()` so the aggregator's temp logs survive the runner. Successful runs do not retain artifacts.

The workflow is **not** required to be wired up as a status check — the `make` targets are the source of truth and the workflow can be paused via `paths:` or by removing the `pull_request` trigger if it interferes with other CI processes. Tier 4 (`acceptance-cross-repo-define-smoke` with `OPERATOR_RESULTS`) stays out of CI by design: it requires operator-supplied JSON or a Cursor SDK runtime that no shared runner provides.

## Further Reading

- [Acceptance Framework overview](../../acceptance/README.md) — directory layout and ownership rules.
- [RM-01 Acceptance Framework Design](../../rfcs/rm-01-acceptance-framework.md) — full risk register and tier rationale.
- [RM-01 Acceptance Framework Implementation Plan](../../rfcs/rm-01-acceptance-framework-implementation-plan.md) — chunk-by-chunk plan; §C16 owns this page.
- [Consistency Checks](checks.md) — the `make checks` reference; check 15 documents recorded-trace freshness.
