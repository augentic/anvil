# Eval replay drivers

Operator replay scripts for multi-step execute scenarios. They shell out to the real `specify` CLI and always delegate driver locking to `specify plan lock --` — they do not reimplement lock semantics.

**Not CI.** These stay out of `.github/workflows/` so every scenario `negative-expectation: automated-runner-added` still holds. Run them manually or let a Cursor agent invoke them while filing a run record under `evals/runs/`.

## Layout

| Script | Scenario | Role |
| ------ | -------- | ---- |
| `execute_loop.py` | shared | Single-repo refine → build → merge loop |
| `execute_fail_resume.py` | `execute-fail-resume` | Park on build failure, breakout, resume |
| `execute_pause_resume.py` | `execute-pause-resume` | Park after build prepare, breakout, resume |
| `workspace.py` | workspace-* | Cross-repo workspace execute / fail / stale recovery |
| `contract_lifecycle.sh` | `contract-lifecycle` | Execute-phase loop only (plan setup stays agent-driven) |

Sandboxes materialise under `evals/.sandbox/<scenario>/` (gitignored). Override roots with `SPECIFY_FRAMEWORK`, `SPECIFY_SANDBOX`, `SPECIFY_BIN`, or `SPECIFY_WS` when replaying on another machine.

## Quick replay

```bash
make install-cli
export PATH="$HOME/.local/bin:$PATH"

# Single-repo fail-resume (recreates evals/.sandbox/execute-fail-resume/)
python3 evals/drivers/execute_fail_resume.py

# Workspace stale recovery (after agent setup per evals/scenarios/workspace-stale-recovery.md)
python3 evals/drivers/workspace.py workspace-stale-recovery

# Contract lifecycle execute loop (plan must already be approved)
bash evals/drivers/contract_lifecycle.sh execute
```

Do not copy these into `evals/.sandbox/` — that directory is for disposable project trees only.
