# Drop Down a Layer

Specify automation is layered: higher-level skills invoke lower-level ones, and you can always bypass a higher level and work directly with the level below. This is useful when automation does something unexpected, when you need fine-grained control, or when you want to debug state.

The change layer is three peer skills -- `/change:draft → /change:execute → /change:finalize` -- with an operator review pause between draft and execute. Each skill is composition over CLI verbs and (for execute) the spec-layer skills. If a skill halts or you want to drive a stage by hand, the same composition can be run directly.

## Drop from `/change:draft` to its CLI verbs

`/change:draft` mints `change.md` and `plan.yaml`, runs registry validation, walks the brief pipeline (discovery → optional sync-workspace → propose → optional assignment), and validates the plan. To drive each step by hand instead:

```bash
specify change draft <name> [--source <key>=<path-or-url>]   # mint change.md + plan.yaml
specify registry validate                                    # registry shape check (multi-repo)
/change:analyze <source>                                     # discovery brief, per input
specify workspace sync                                       # multi-repo only -- materialise tier-2 clones
# author plan entries by hand or re-run /change:draft's propose brief
specify plan add <entry-name> [--project <p>] [--depends-on a,b]
specify plan amend <entry-name> --project <p>                # multi-repo assignment
specify plan validate                                        # final shape check
```

This is the right escape hatch when the propose loop is producing the wrong slice boundaries, when the assignment step needs operator override on every entry, or when you just want to skip discovery for a small known change.

## Drop from `/change:execute` to per-slice spec skills

`/change:execute loop` runs `/spec:define → /spec:build → /spec:merge` per slice. To take manual control of a single plan entry:

```bash
specify plan transition <entry> in-progress

/spec:define "<the entry's description>"
/spec:build
/spec:merge

specify plan transition <entry> done
```

The plan is just a data file that tracks status. You can transition entries freely.

## Drop from `/change:finalize` to its CLI verbs

`/change:finalize` runs `specify workspace push`, observes PR state via `gh pr list`, and runs `specify change finalize`. To drive the post-execute tail by hand:

```bash
specify workspace push                       # publish specify/<change-name> branches as PRs
gh pr list -R <owner/repo> --head specify/<change-name>
gh pr merge <pr> -R <owner/repo> --squash    # or merge through the forge UI
specify change finalize                      # archive plan.yaml + change.md + .specify/plans/<name>/
```

Use this when the push step needs investigation (auth failures, branch divergence) or when you want to verify each PR's checks individually before re-running `/change:finalize`.

## From slice skills to direct CLI control

If a slice-layer skill is not behaving as expected, you can inspect and manipulate state directly:

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
/change:execute        # picks it up on the next cycle
```

## The principle

Every skill is built on CLI commands. If a skill does something you don't understand, the CLI gives you visibility into the same state the skill is reading. If a skill does something you don't want, the CLI lets you correct the state.

## See also

- [The Layered Stack](../explanation/layered-stack.md) -- architectural explanation
- [`/change:draft`, `/change:execute`, `/change:finalize`](../reference/change-skills/index.md) -- the three peer skills the lifecycle composes
- [CLI Reference](../reference/cli/index.md) -- all CLI commands
- [Recover from a Failed Change](recover-failed-change.md) -- focused recovery guide
