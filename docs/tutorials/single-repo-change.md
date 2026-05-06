# A Multi-Change Initiative

When a body of work spans multiple related changes -- a migration, a new feature set, or a modernisation effort -- you need a plan. This tutorial walks you through authoring a plan and executing it within a single repository.

**Prerequisites:** Familiarity with the [define-build-merge loop](first-change.md). A project with `/spec:init` already run.

> **Choosing your topology.** This tutorial is for **single-repo** work -- one project under `/spec:init` with phase pipelines enabled, no platform hub. If your initiative spans two or more repos (e.g. backend + mobile), you want the platform-hub topology instead -- see [Cross-Repo Initiatives](cross-repo-change.md). The platform-as-project shape (single-repo with `url: .` in `registry.yaml`) is also valid; see [Platform repo topologies](../explanation/platform-repo.md) for the comparison.

## When you need a plan

A plan is useful when:

- You have three or more related changes.
- Changes have dependencies (one must finish before another can start).
- You want to track progress across the entire initiative.
- You want to automate the change-by-change execution loop.

For one or two independent changes, the manual define-build-merge loop is simpler.

## 1. Author the plan

Suppose you want to migrate an auth service from a legacy codebase. Start by telling Specify what you are working from:

```text
/change:plan migrate-auth --source legacy=./src/auth
```

<details>
<summary>Expected output (summary)</summary>

```text
Planning migrate-auth...

Discovery:
  Analyzing ./src/auth...
  Found 4 capabilities: token-validation, session-management,
    oauth-integration, auth-middleware

Propose:
  1. extract-token-validation — Accept / Edit / Reject? Accept
  2. extract-session-management — Accept / Edit / Reject? Accept
  3. add-oauth-integration — Accept / Edit / Reject? Accept
  4. consolidate-auth-middleware — Accept / Edit / Reject? Accept

Validate:
  ✓ No duplicate names
  ✓ No dependency cycles
  ✓ All depends-on references valid

Plan created: plan.yaml (4 entries)
```

</details>

The plan skill runs a three-phase internal flow:

### Discovery

`/spec:analyze` reads the legacy source and produces a capability inventory in `discovery.md`. Each discovered capability gets a summary, source file hints, dependency edges, and a confidence marker.

You can review the discovery output at `.specify/plans/migrate-auth/discovery.md`.

### Propose

The skill proposes changes ("slices") based on the discovered capabilities. For each proposed slice, you can:

- **Accept** -- add it to the plan as-is.
- **Edit** -- modify the description, dependencies, or scope.
- **Reject** -- exclude it from the plan.

This is an interactive loop. The agent presents each slice and waits for your decision.

### Validate

After all slices are accepted or rejected, the skill runs `specify change plan validate` to check:

- No duplicate change names.
- No dependency cycles.
- All `depends-on` references point to existing entries.

## 2. Review the plan

After planning completes, inspect the result:

```bash
specify change plan status
```

This shows the entries in topological order:

```
migrate-auth
  pending  extract-token-validation    (depends-on: [])
  pending  extract-session-management  (depends-on: [])
  pending  add-oauth-integration       (depends-on: [extract-token-validation])
  pending  consolidate-auth-middleware  (depends-on: [extract-token-validation, extract-session-management])

  Summary: 4 pending, 0 in-progress, 0 done
```

You can also look at `plan.yaml` directly to see the full plan structure including descriptions, sources, and dependency edges.

## 3. Preview execution

Before committing to automated execution, preview what would happen:

```text
/change:execute --dry-run
```

<details>
<summary>Expected output</summary>

```text
Dry run:
  Next eligible: extract-token-validation (depends-on: [])
  Progress: 0/4 done, 4 pending
```

</details>

This reports the next eligible change and current progress without modifying anything.

## 4. Run one change

To execute a single change and stop:

```text
/change:execute
```

The driver:

1. Picks the next eligible entry (`extract-token-validation`, since it has no dependencies).
2. Transitions it to `in-progress`.
3. Runs `/spec:define` with the entry's description and source.
4. Runs `/spec:build`.
5. Runs `/spec:merge`.
6. Reads the phase outcome and transitions the plan entry to `done`.

After this, `specify change plan status` shows:

```
  done     extract-token-validation
  pending  extract-session-management
  pending  add-oauth-integration
  pending  consolidate-auth-middleware
```

## 5. Run until completion

To let Specify work through the remaining changes:

```text
/change:execute --loop
```

Loop mode repeats the pick-define-build-merge cycle until:

- **`all-done`** -- every entry is `done` or `skipped`.
- **`stuck`** -- no `pending` entry has all dependencies satisfied.
- **Interrupted** -- you press Ctrl+C for graceful shutdown.

## 6. Handle failures

If a change fails during execution:

1. `/change:execute` invokes `/spec:drop` to discard the failed change.
2. The plan entry transitions to `failed` with a reason.
3. In loop mode, the driver continues to the next eligible entry.

If all remaining entries depend on the failed one, execution reports `stuck`. You can then:

- Fix the issue and re-run: `specify change plan transition extract-token-validation pending` to reset it, then `/change:execute --loop`.
- Skip it: `specify change plan transition extract-token-validation skipped`.
- Amend the plan: `specify change plan amend consolidate-auth-middleware --depends-on extract-session-management` to remove the dependency.

## 7. Drop down a layer

You can always fall back to manual control. If `/change:execute` is stuck or you want to handle a specific change yourself:

```text
# Manually transition a plan entry
specify change plan transition add-oauth-integration in-progress

# Define, build, and merge manually
/spec:define "Add OAuth2 provider integration to the auth service"
/spec:build
/spec:merge

# Mark it done in the plan
specify change plan transition add-oauth-integration done
```

The plan is just a data file that tracks status. The Layer 1 CLI commands give you full control.

## What you learned

- `/change:plan` discovers capabilities and proposes changes with an interactive accept/edit/reject loop.
- The plan tracks changes, dependencies, and status in `plan.yaml`.
- `/change:execute` automates the define-build-merge loop per change.
- `--dry-run` previews, bare invocation runs one change, `--loop` runs until done.
- Failures transition plan entries to `failed`. You can reset, skip, or restructure.
- Layer 1 CLI commands are always available as manual fallback.

## Next

[Cross-Repo Initiatives](cross-repo-change.md) -- coordinate changes across multiple repositories.
