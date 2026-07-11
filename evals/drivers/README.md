# Eval replay drivers

Legacy operator replay scripts for multi-step scenarios not yet expressible as direct command steps. Canonical cases live under `quality/scenarios/`; new profile execution belongs under `quality/profiles/`, not here. The scripts shell out to the real `specify` CLI and delegate refine → build → merge to `specify plan execute`.

Each driver does the clerical work around the loop: build the branch-head binary and workflow guest (pinned as the core via `SPECIFY_CORE_PATH`, so nothing hydrates `specify:core` from the registry), init the sandbox from release-built adapter components in the sibling `augentic/specify-adapters` checkout (`cargo make release` there), write a project-root `omnia.toml` deployment manifest, author the plan (`specify plan author`), stamp Gate 1 on the operator's behalf (`specify plan transition <name> approved --actor agent`), and drive `specify plan execute`. Briefs live in [`fixtures/`](fixtures/), never inline in the scripts.

The judgment legs (`plan author`, `plan execute`) run through the composed runtime against the live cursor backend, so the `main` legs require `cursor-agent` on PATH and make real model calls. The `setup` legs are model-free and runnable anywhere the sibling checkout is release-built.

**Live legs are not CI.** Deterministic canonical profiles may run in CI; anything that calls the live model remains explicit. New runs write structured bundles under `quality/runs/`; `evals/runs/` is historical audit evidence.

## Layout

| Script | Scenario | Role |
| ------ | -------- | ---- |
| `lib.sh` | shared | `run` / `try`, build-under-test + manifest helpers, jq status helpers |
| `single-repo.sh` | shared | Single-repo setup / author / approve / execute + resume legs |
| `execute-fail-resume.sh` | `execute-fail-resume` | Park on build failure, operator fix, `resume` leg |
| `execute-pause-resume.sh` | `execute-pause-resume` | Operator interrupt mid-slice, breakout, `resume` leg |
| `workspace.sh` | workspace-* | Cross-repo workspace setup / author / approve / execute + resume |
| `contract-lifecycle.sh` | `contract-lifecycle` | Execute-phase loop only (plan setup stays agent-driven) |
| `guest-execute-loop.sh` | `guest-execute-loop` | Compatibility wrapper for `quality/profiles/guest-execute-loop.sh` |

Sandboxes materialise under `evals/.sandbox/<scenario>/` (gitignored). Override roots with `SPECIFY_FRAMEWORK`, `SPECIFY_SANDBOX`, `SPECIFY_BIN`, `SPECIFY_ADAPTERS`, or `SPECIFY_WS` when replaying on another machine.

## Quick replay

```bash
# One-time: release-build the adapter components in the sibling checkout
(cd ../specify-adapters && cargo make release)

# Single-repo fail-resume (recreates evals/.sandbox/execute-fail-resume/)
bash evals/drivers/execute-fail-resume.sh          # parks on the failure
#   ...operator fixes the slice per the stop hint...
bash evals/drivers/execute-fail-resume.sh resume   # drains

# Workspace scenarios (self-contained: scaffold workspace, author, execute)
bash evals/drivers/workspace.sh workspace-two-projects
bash evals/drivers/workspace.sh workspace-fail-resume          # + resume leg
bash evals/drivers/workspace.sh workspace-stale-recovery       # + resume leg

# Model-free setup legs (verify the provisioning verbs without a backend)
bash evals/drivers/execute-fail-resume.sh setup
bash evals/drivers/workspace.sh workspace-two-projects setup

# Contract lifecycle execute loop (plan must already be authored + approved)
bash evals/drivers/contract-lifecycle.sh execute
```

Do not copy these into `evals/.sandbox/` — that directory is for disposable project trees only.
