# terminal-status-not-met — `/change:finalize` halts when the plan still has eligible work

A two-entry plan for `dark-mode` where `dark-mode-contract` is `done` but `dark-mode-mobile` is still `in-progress`. The operator runs `/change:finalize dark-mode` expecting to close the change; the skill's plan-terminality check (step 2) rejects the run before any push happens.

This fixture pins the `non-terminal-entries` halt classification.

## Transcript

```text
$ /change:finalize dark-mode

Pre-flight
  change:       dark-mode (kebab-case ok)
  project root: /…/shop-platform/.specify/project.yaml
  plan.yaml:    present

---

## Step 2 — Plan terminality

Reading plan.yaml…

  | # | Entry              | Project        | Status       |
  |---|--------------------|----------------|--------------|
  | 1 | dark-mode-contract | —              | done         |
  | 2 | dark-mode-mobile   | vectis-mobile  | in-progress  |

Halt: non-terminal-entries.

  - dark-mode-mobile is still in-progress.

Next action: run /change:execute loop until every plan entry is
  terminal (done | failed | blocked | skipped), then re-run
  /change:finalize dark-mode.

Exit 1
```

## Invariants pinned

1. **Halt happens before any side-effect.** The skill never reaches step 3 (`specify workspace push`); no remote state is touched.
2. **The halt classification is `non-terminal-entries`, verbatim.** No paraphrase, no synonym.
3. **The diagnostic names the offending entry.** `dark-mode-mobile is still in-progress.` is the operator's actionable cue — they know which slice to drive.
4. **The next-action line points at `/change:execute loop`, not at `specify plan transition`.** Re-entering the executor is the canonical recovery; manual transitions are a fallback documented in the executor skill, not here.
5. **Exit code 1.** CI scripting can distinguish the `non-terminal-entries` halt (exit 1) from a successful run (exit 0).
