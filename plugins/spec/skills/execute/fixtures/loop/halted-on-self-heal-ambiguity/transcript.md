# halted-on-self-heal-ambiguity — `--loop` halts before the iteration begins

The prior `/spec:execute` run left `shopping-cart` as `in-progress` in `plan.yaml`, and its `.metadata.yaml` carries a contradictory pair (`status: defining` + `outcome.phase: merge` / `outcome: success`). A new `/spec:execute --loop` invocation enters self-heal (step 3 of the `--loop` algorithm), detects the contradiction, refuses to speculate, and halts before the outer iteration loop runs at all.

The `checkout-api` entry is structurally blocked by `shopping-cart` anyway, so the halt costs nothing — but the semantics matter: even if the plan had other independent entries with all-`done` dependencies, self-heal halt stops the whole loop. Ambiguity is a signal that the on-disk state is inconsistent; continuing with other work while that inconsistency sits unresolved risks compounding the damage.

## Driver timeline

```text
$ /spec:execute --loop

# step 1 (project resolution): silent on success.
# step 2: acquire the driver lock. No diagnostic on success.

# step 3: self-heal (writing path).
#   Scans plan.yaml, finds shopping-cart with status: in-progress.
#   Reads .specify/changes/shopping-cart/.metadata.yaml.
#   Classifies: outcome.phase == merge, outcome: success BUT
#   status: defining. Contradiction — no lifecycle transition
#   reaches this state. HALT.
#   - Does NOT call specify initiative transition.
#   - Does NOT call /spec:drop.
#   - Does NOT append a type: recovery journal entry.

Self-heal halted: shopping-cart has outcome=success phase=merge but LifecycleStatus=defining. Manual triage required.

# step 4: SKIPPED. The iteration loop never runs.

# step 5: emit terminal summary (Completion: halted).

# step 6: release the driver lock. Unconditional — halt still runs
# the finally-edge.

# step 7: exit 1.
```

## Terminal summary (as rendered by `/spec:execute`)

```text
## /spec:execute — platform-v2 — terminated

### Final state
Progress: done 0, in-progress 1, pending 1, blocked 0, failed 0, skipped 0 (total 2)

Completion: halted

Next action: Manually triage the halted change: inspect .specify/changes/shopping-cart/.metadata.yaml against plan.yaml, repair the contradiction, then re-run /spec:execute --loop.
```

## Invariants pinned

1. **Self-heal halt is the only path to `Completion: halted` under `--loop`.** Individual mid-loop failures / deferrals transition the plan entry to `failed` / `blocked` and the loop continues; `specify initiative next` skips those entries. Only a self-heal ambiguity halt reaches `halted`.
2. **Halted runs still emit the terminal summary.** The summary is emitted in step 5 regardless of whether the loop body ran or not.
3. **Halted runs still release the lock.** Step 6 runs unconditionally. The halt's observable effect is the exit code and the terminal summary, not a stranded lock file.
4. **Plan and journal untouched on halt.** `plan.yaml.after` is byte-identical to `plan.yaml.before`. No `journal.yaml` entry is authored — halt emits no recovery entries. `shopping-cart`'s journal (whatever the crashed run left there) is preserved unchanged.
5. **Progress line reflects the pre-halt state.** `in-progress 1` is the entry self-heal halted on. `pending 1` is `checkout-api`, which was never touched.
6. **Exit code 1.** Not 0. Halted is an actionable diagnostic, not a partial success.
