# Onboard a Team Member

When a colleague joins a project that already uses Specify, they need the tooling installed and an understanding of the existing baseline.

## What they need to install

1. [Cursor IDE](https://cursor.com) with the Augentic plugin marketplace.
2. The `specify` CLI: `brew install augentic/tap/specify`
3. Schema-specific tooling (see [Prerequisites](../orientation/prerequisites.md)).

## What they get from the repo

The `.specify/` directory is committed to the repository. When they clone or pull, they get:

- **`project.yaml`** -- project configuration and schema reference.
- **`.cache/`** -- cached schema and brief files.
- **`specs/`** -- the accumulated baseline specifications.
- **`contracts/`** -- baseline API contracts (if any).

They can immediately run `specify status` to see active slices and the project dashboard.

## Working in parallel

Two developers can work on different changes simultaneously. Each change lives in its own directory under `.specify/slices/`:

```text
.specify/slices/
├── add-notifications/    # Developer A
└── improve-auth/         # Developer B
```

Use **git branches** -- each developer creates a branch for their change. The baseline at `.specify/specs/` is the shared truth.

## Handling baseline conflicts

If two changes modify the same capability, the second to merge may encounter a conflict:

1. Developer A merges `add-notifications` -- baseline updated.
2. Developer B tries to merge `improve-auth` -- `specify slice merge conflict-check` detects the baseline changed since define.

Resolution options:

- **Re-run define:** `/spec:define improve-auth` regenerates artifacts against the updated baseline.
- **Manual resolution:** Edit the delta spec to account for Developer A's changes.
- **Drop and redefine:** `/spec:drop` then `/spec:define` with a refreshed description.

## Orientation for the new team member

Point them to:

1. [Quick Start](../tutorials/quick-start.md) -- hands-on in 5 minutes.
2. [Your First Slice](../tutorials/first-change.md) -- the full tutorial with explanations.
3. The baseline at `.specify/specs/` -- read the existing specs to understand what the system does.

## See also

- [Troubleshooting](../appendices/troubleshooting.md) -- common issues and fixes
- [specify status](../reference/cli/status.md) -- project dashboard CLI
