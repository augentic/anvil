# /spec:define

Create a new slice and generate all artifacts in one step.

## Synopsis

```text
/spec:define [description] [artifact-id?] [--source <key>=<path-or-url>...]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `description` | Yes | What you want to build or change |
| `artifact-id` | No | Regenerate a single artifact for an existing slice |
| `--source` | No | Named source paths for extraction (e.g. `legacy=./src/auth`) |

## When to use

- You have a clear idea of what to build and want a complete set of artifacts ready for implementation.
- You need to regenerate a single artifact for an existing slice (pass the artifact ID).
- You are defining a slice in a plan-driven change and the plan entry has `sources` for extraction.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| `proposal.md` | `.specify/slices/<name>/proposal.md` | Why the slice exists and what is in scope |
| `spec.md` (per capability) | `.specify/slices/<name>/specs/<capability>/spec.md` | Behavioral requirements with scenarios |
| `composition.yaml` (Vectis only) | `.specify/slices/<name>/composition.yaml` | Screen layout with regions, groups, bindings, and event wiring |
| `design.md` | `.specify/slices/<name>/design.md` | Domain model, APIs, integrations, configuration |
| `tasks.md` | `.specify/slices/<name>/tasks.md` | Implementation task list with checkboxes |

## Behavior

1. Creates the slice directory via `specify slice create <name>`.
2. Reads the capability's `pipeline.define` brief sequence.
3. Generates artifacts in dependency order: proposal, then specs (which may invoke `/spec:extract` if `sources` are present), then composition (Vectis only -- produces `composition.yaml`), then design, then tasks. The composition stage produces a YAML file rather than markdown; the skill dispatches on the `generates` extension in the brief frontmatter.
4. Scans `touched-specs` via `specify slice touched-specs`.
5. Transitions the slice to `defined`.
6. Writes phase outcome via `specify slice outcome set`.

## Lifecycle transitions

`created --> defining --> defined`

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| Name collision | A change with this name already exists | Choose a different name or use `--if-exists` |
| Capability not cached | `/spec:init` not run | Run `/spec:init` first |
| Source resolution failure | `--source` path does not exist | Check the path |
| Brief pipeline error | Capability brief has unresolvable dependency | Check capability configuration |

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
