# Eval replay drivers

Operator replay scripts for multi-step execute scenarios. Pure bash (no Python), `jq` as the only structured-data dependency, bash 3.2-compatible. They shell out to the real `specify` CLI and always delegate driver locking to `specify plan lock --` — they do not reimplement lock semantics.

The loop is driven entirely by the CLI: [`lib.sh`](lib.sh) reads `specify plan status` for the next `action`/`slice`/`project` and `specify plan next` for each slice's source bindings, then dispatches the named phase to scenario hooks. Canned agent content (synthesis/evidence envelopes, crate/shell stubs, briefs, the vendored contract input tree) lives in [`fixtures/`](fixtures/), never inline in the scripts.

**Not CI.** These stay out of `.github/workflows/` so every scenario `negative-expectation: automated-runner-added` still holds. Run them manually or let a Cursor agent invoke them while filing a run record under `evals/runs/`.

## Layout

| Script | Scenario | Role |
| ------ | -------- | ---- |
| `lib.sh` | shared | `run` / `try` / `run_lock`, jq status helpers, and the `_drive` loop |
| `single-repo.sh` | shared | Single-repo setup/survey/propose + refine → build → merge hooks |
| `execute-fail-resume.sh` | `execute-fail-resume` | Park on build failure, breakout, resume |
| `execute-pause-resume.sh` | `execute-pause-resume` | Park after build prepare, breakout, resume |
| `workspace.sh` | workspace-* | Cross-repo workspace execute / fail / stale recovery |
| `contract-lifecycle.sh` | `contract-lifecycle` | Execute-phase loop only (plan setup stays agent-driven) |

Sandboxes materialise under `evals/.sandbox/<scenario>/` (gitignored). Override roots with `SPECIFY_FRAMEWORK`, `SPECIFY_SANDBOX`, `SPECIFY_BIN`, or `SPECIFY_WS` when replaying on another machine.

## Quick replay

```bash
make install-cli
export PATH="$HOME/.local/bin:$PATH"

# Single-repo fail-resume (recreates evals/.sandbox/execute-fail-resume/)
bash evals/drivers/execute-fail-resume.sh

# Workspace scenarios (self-contained: scaffold workspace, plan, execute to drained)
bash evals/drivers/workspace.sh workspace-two-projects
bash evals/drivers/workspace.sh workspace-fail-resume
bash evals/drivers/workspace.sh workspace-stale-recovery

# Contract lifecycle execute loop (run under a session lock; plan must already be approved)
specify plan lock -- bash evals/drivers/contract-lifecycle.sh execute
```

Do not copy these into `evals/.sandbox/` — that directory is for disposable project trees only.
