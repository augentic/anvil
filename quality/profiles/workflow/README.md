# Workflow profile drivers

Profile-specific executors for multi-step scenarios that require controlled failures, interruptions, or multiple workspace slots. Canonical cases and assertion ids live under `quality/scenarios/`; these scripts only provide runtime control that generic command probes cannot express. They shell out to the real `specify` CLI and delegate refine → build → merge to `specify plan execute`. They are non-catalog bash operator evaluators, deliberately outside the two-axis Rust runner architecture (see the declared-vs-executed matrix in [`../../COVERAGE.md`](../../COVERAGE.md)). The uninterrupted guest execute loop is not driven from here: its live profiles run through the Rust runners — `cargo make quality -- run wasm-live` in this repo, `specify-dev quality` in `specify-adapters/harness/native` for `native-live`.

Each driver does the clerical work around the loop: build the branch-head binary and workflow guest (pinned as the core via `SPECIFY_CORE_PATH`, so nothing hydrates `specify:core` from the registry), init the sandbox from release-built adapter components in the sibling `augentic/specify-adapters` checkout (`cargo make release` there), write a project-root `omnia.toml` deployment manifest, author the plan (`specify plan author`), stamp Gate 1 on the operator's behalf (`specify plan transition <name> approved --actor agent`), and drive `specify plan execute`. Briefs live in [`fixtures/`](fixtures/), never inline in the scripts.

The judgment legs (`plan author`, `plan execute`) run through the composed runtime against the live cursor backend, so the `main` legs require `cursor-agent` on PATH and make real model calls. The `setup` legs are model-free and runnable anywhere the sibling checkout is release-built.

**Live legs are not CI.** Deterministic canonical profiles may run in CI; anything that calls the live model remains explicit. New runs write structured bundles under `quality/runs/`; `quality/runs/archive/` is historical audit evidence.

## Layout

| Script | Scenario | Role |
| ------ | -------- | ---- |
| `lib.sh` | shared | `run` / `try`, build-under-test + manifest helpers, jq status helpers |
| `single-repo.sh` | shared | Single-repo setup / author / approve / execute + resume legs |
| `execute-fail-resume.sh` | `execute-fail-resume` | Park on build failure, operator fix, `resume` leg |
| `execute-pause-resume.sh` | `execute-pause-resume` | Operator interrupt mid-slice, breakout, `resume` leg |
| `workspace.sh` | workspace-* | Cross-repo workspace setup / author / approve / execute + resume |
| `contract-lifecycle.sh` | `contract-lifecycle` | Execute-phase loop only (plan setup stays agent-driven) |

Sandboxes materialise under `quality/.sandbox/<scenario>/` (gitignored). Override roots with `SPECIFY_FRAMEWORK`, `SPECIFY_SANDBOX`, `SPECIFY_BIN`, `SPECIFY_ADAPTERS`, or `SPECIFY_WS` when replaying on another machine.

## Quick replay

```bash
# One-time: release-build the adapter components in the sibling checkout
(cd ../specify-adapters && cargo make release)

# Single-repo fail-resume (recreates quality/.sandbox/execute-fail-resume/)
bash quality/profiles/workflow/execute-fail-resume.sh          # parks on the failure
#   ...operator fixes the slice per the stop hint...
bash quality/profiles/workflow/execute-fail-resume.sh resume   # drains

# Workspace scenarios (self-contained: scaffold workspace, author, execute)
bash quality/profiles/workflow/workspace.sh workspace-two-projects
bash quality/profiles/workflow/workspace.sh workspace-fail-resume          # + resume leg
bash quality/profiles/workflow/workspace.sh workspace-stale-recovery       # + resume leg

# Model-free setup legs (verify the provisioning verbs without a backend)
bash quality/profiles/workflow/execute-fail-resume.sh setup
bash quality/profiles/workflow/workspace.sh workspace-two-projects setup

# Contract lifecycle execute loop (plan must already be authored + approved)
bash quality/profiles/workflow/contract-lifecycle.sh execute
```

Do not copy these into `quality/.sandbox/` — that directory is for disposable project trees only.
