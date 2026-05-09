# Re-entry / idempotency (orchestration mode)

Running `/change:plan <name> orchestrate` a second time after a halt is the canonical resume mechanism. The orchestration mode inspects on-disk state and resumes at the first incomplete step, without prompting and without re-doing earlier work.

## Resume table

| State at re-entry | Resume step |
|---|---|
| change brief absent (`change.md`) | step 1 |
| brief present, `plan.yaml` absent | step 3 (with `/change:plan` default mode running fresh) |
| `plan.yaml` present, any entry not in `{done, failed, skipped}` | step 4 (`/change:execute loop` resumes — self-heal reclaims any `in-progress` left by a prior crash) |
| every plan entry terminal, no PRs pushed yet (no `specify/<name>` branch on any remote) | step 5 |
| PRs pushed, not all `MERGED` | step 6 lists PRs and stops for operator merge |
| every PR `MERGED`, plan still on disk | step 7 |
| plan archived (`plan-not-found`) | report "change already finalized" and exit 0 |

## Idempotency invariants

The orchestration mode never re-creates a brief, re-runs discovery, or re-pushes a clone whose remote is already up to date. Resume is purely additive — every shell-out underneath is itself idempotent:

- `specify change create` refuses on populated brief;
- `specify change plan create` refuses on populated plan;
- `specify workspace push` reports `up-to-date` for clones it already pushed;
- step 6 reads remote PR state and reports already-merged PRs without invoking merge automation;
- `specify change finalize` refuses on `plan-not-found`.

## Step 3 re-entry against a populated plan

When step 3 runs against a populated `plan.yaml`, the orchestration forwards `--extend` to `/change:plan` (default mode) so the plan skill appends new slices instead of refusing on a populated plan. Operators who want a fresh plan archive the old one first (`specify change plan archive`) and re-run.

The plan skill's own `--extend` semantics apply (see [SKILL.md](SKILL.md) §"Modes → `--extend`"):

- step 2 (`specify change plan create`) is skipped;
- step 3(a) (discovery) is skipped when `discovery.md` already exists;
- step 3(c) (propose) silently skips draft slices whose names collide with existing entries;
- pre-existing entries are never modified.
