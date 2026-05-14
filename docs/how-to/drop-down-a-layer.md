# Drop Down a Layer

Specify automation is layered: higher-level skills invoke lower-level ones, and you can always bypass a higher level and work directly with the level below. This is useful when automation does something unexpected, when you need fine-grained control, or when you want to debug state.

## Skip the orchestrate umbrella

If `/change:plan <name> orchestrate` halts on the registry-amendment-required path, on `stuck`, or on a step you want to drive yourself, the same composition can be run by hand:

```bash
specify slice create <name>           # Step 1 — brief
specify registry validate                  # Step 2 — registry
/change:plan <name> from <docs>            # Step 3 — plan
/change:execute loop                       # Step 4 — execute
specify workspace push                     # Step 5 — push
gh pr merge <pr> --squash                  # Step 6 — land each PR (or use the forge UI)
specify change finalize                # Step 7 — finalize
```

The umbrella is **idempotent on re-entry** — re-running `/change:plan <name> orchestrate` after dropping down picks up from the first step that still has work to do. See [`/change:plan <name> orchestrate` — Re-entry / idempotency](../../plugins/change/skills/plan/re-entry.md).

## From plan-driven execution to single-slice loops

If `/change:execute` is stuck or you want to handle a specific change yourself:

```bash
# Take manual control of a plan entry
specify change plan transition add-oauth in-progress

# Run the phases manually
/spec:define "Add OAuth2 provider integration"
/spec:build
/spec:merge

# Mark it done in the plan
specify change plan transition add-oauth done
```

The plan is just a data file that tracks status. You can transition entries freely.

## From slice skills to direct CLI control

If a skill is not behaving as expected, you can inspect and manipulate state directly:

```bash
# Inspect a slice
specify slice status <name>

# Manually transition a slice
specify slice transition <name> defined

# Check task progress
specify slice task progress <name>

# Preview what merge would do
specify slice merge preview <name>

# Check for baseline conflicts
specify slice merge conflict-check <name>

# Validate artifacts
specify slice validate <name>
```

## Common scenarios

### Finish a slice that `/change:execute` abandoned

```bash
# Check what state the slice is in
specify slice status <name>

# If it's in 'building', resume build manually
/spec:build

# If build is done, merge
/spec:merge

# Update the plan entry
specify change plan transition <name> done
```

### Adjust a plan entry

```bash
# Change dependencies
specify change plan amend <name> --depends-on dep1,dep2

# Change the target project (multi-repo)
specify change plan amend <name> --project api

# Skip an entry entirely
specify change plan transition <name> skipped
```

### Reset and retry a failed entry

```bash
specify change plan transition <name> pending
/change:execute        # picks it up on the next cycle
```

## The principle

Every skill is built on CLI commands. If a skill does something you don't understand, the CLI gives you visibility into the same state the skill is reading. If a skill does something you don't want, the CLI lets you correct the state.

## See also

- [The Layered Stack](../explanation/layered-stack.md) -- architectural explanation
- [/change:plan <name> orchestrate](../../plugins/change/skills/plan/orchestration.md) -- the cross-repo umbrella mode
- [CLI Reference](../reference/cli/index.md) -- all CLI commands
- [Recover from a Failed Change](recover-failed-change.md) -- focused recovery guide
