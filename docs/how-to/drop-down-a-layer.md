# Drop Down a Layer

Specify is organized in four layers. Higher layers invoke lower layers, but you can always bypass a higher layer and work directly with the one below it. This is useful when automation does something unexpected, when you need fine-grained control, or when you want to debug state.

## From Layer 4 to Layer 3: skip the umbrella

If `/change:plan --orchestrate` halts on the registry-amendment-required path, on `stuck`, or on a step you want to drive yourself, the same composition can be run by hand:

```bash
specify slice create <name>           # Step 1 — brief
specify registry validate                  # Step 2 — registry
/change:plan <name> --from <docs>            # Step 3 — plan
/change:execute --loop                       # Step 4 — execute
specify workspace push                     # Step 5 — push
specify workspace merge                    # Step 6 — land (or merge by hand on the forge)
specify change finalize                # Step 7 — finalize
```

The umbrella is **idempotent on re-entry** — re-running `/change:plan --orchestrate <name>` after dropping down picks up from the first step that still has work to do. See [`/change:plan --orchestrate` — Re-entry / idempotency](../../plugins/change/skills/plan/re-entry.md).

## From Layer 3 to Layer 2: manual change execution

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

## From Layer 2 to Layer 1: direct CLI control

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

- [The Layered Stack](../explanation/three-layer-stack.md) -- architectural explanation
- [/change:plan --orchestrate](../../plugins/change/skills/plan/orchestration.md) -- the Layer 4 umbrella mode (formerly the `/spec:initiative` skill)
- [CLI Reference](../reference/cli/index.md) -- all CLI commands
- [Recover from a Failed Change](recover-failed-change.md) -- focused recovery guide
