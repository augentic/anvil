# Drop Down a Layer

Specify is organized in three layers. Higher layers invoke lower layers, but you can always bypass a higher layer and work directly with the one below it. This is useful when automation does something unexpected, when you need fine-grained control, or when you want to debug state.

## From Layer 3 to Layer 2: manual change execution

If `/spec:execute` is stuck or you want to handle a specific change yourself:

```bash
# Take manual control of a plan entry
specify plan transition add-oauth in-progress

# Run the phases manually
/spec:define "Add OAuth2 provider integration"
/spec:build
/spec:merge

# Mark it done in the plan
specify plan transition add-oauth done
```

The plan is just a data file that tracks status. You can transition entries freely.

## From Layer 2 to Layer 1: direct CLI control

If a skill is not behaving as expected, you can inspect and manipulate state directly:

```bash
# Inspect a change
specify change status <name>

# Manually transition a change
specify change transition <name> defined

# Check task progress
specify task progress .specify/changes/<name>/

# Preview what merge would do
specify spec preview .specify/changes/<name>/

# Check for baseline conflicts
specify spec conflict-check .specify/changes/<name>/

# Validate artifacts
specify validate .specify/changes/<name>/
```

## Common scenarios

### Finish a change that `/spec:execute` abandoned

```bash
# Check what state the change is in
specify change status <name>

# If it's in 'building', resume build manually
/spec:build

# If build is done, merge
/spec:merge

# Update the plan entry
specify plan transition <name> done
```

### Adjust a plan entry

```bash
# Change dependencies
specify plan amend <name> --depends-on dep1,dep2

# Change the target project (multi-repo)
specify plan amend <name> --project api

# Skip an entry entirely
specify plan transition <name> skipped
```

### Reset and retry a failed entry

```bash
specify plan transition <name> pending
/spec:execute        # picks it up on the next cycle
```

## The principle

Every skill is built on CLI commands. If a skill does something you don't understand, the CLI gives you visibility into the same state the skill is reading. If a skill does something you don't want, the CLI lets you correct the state.

## See also

- [The Three-Layer Stack](../explanation/three-layer-stack.md) -- architectural explanation
- [CLI Reference](../reference/cli/index.md) -- all CLI commands
- [Recover from a Failed Change](recover-failed-change.md) -- focused recovery guide
