# /spec:define

Create a new change and generate all artifacts in one step.

## Synopsis

```text
/spec:define [description] [artifact-id?] [--source <key>=<path-or-url>...]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `description` | Yes | What you want to build or change |
| `artifact-id` | No | Regenerate a single artifact for an existing change |
| `--source` | No | Named source paths for extraction (e.g. `legacy=./src/auth`) |

## When to use

- You have a clear idea of what to build and want a complete set of artifacts ready for implementation.
- You need to regenerate a single artifact for an existing change (pass the artifact ID).
- You are defining a change in a plan-driven initiative and the plan entry has `sources` for extraction.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| `proposal.md` | `.specify/changes/<name>/proposal.md` | Why the change exists and what is in scope |
| `spec.md` (per capability) | `.specify/changes/<name>/specs/<capability>/spec.md` | Behavioral requirements with scenarios |
| `design.md` | `.specify/changes/<name>/design.md` | Domain model, APIs, integrations, configuration |
| `tasks.md` | `.specify/changes/<name>/tasks.md` | Implementation task list with checkboxes |

## Behavior

1. Creates the change directory via `specify change create <name>`.
2. Reads the schema's `pipeline.define` brief sequence.
3. Generates artifacts in dependency order: proposal, then specs (which may invoke `/spec:extract` if `sources` are present), then design, then tasks.
4. Scans `touched-specs` via `specify change touched-specs`.
5. Transitions the change to `defined`.
6. Writes phase outcome via `specify change phase-outcome`.

## Lifecycle transitions

`created --> defined`

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| Name collision | A change with this name already exists | Choose a different name or use `--if-exists` |
| Schema not cached | `/spec:init` not run | Run `/spec:init` first |
| Source resolution failure | `--source` path does not exist | Check the path |
| Brief pipeline error | Schema brief has unresolvable dependency | Check schema configuration |

## Examples

```text
# Define a new feature
/spec:define "Add user authentication with JWT tokens"

# Define with legacy source for extraction
/spec:define "Migrate auth service" --source legacy=./services/auth

# Regenerate just the design for an existing change
/spec:define add-auth design
```

## See also

- [Artifact Format](../artifact-format.md) -- format of the generated artifacts
- [/spec:build](build.md) -- next step after define
- [/spec:extract](extract.md) -- how source extraction works when `--source` is provided
- [/spec:explore](explore.md) -- for thinking through requirements before defining
