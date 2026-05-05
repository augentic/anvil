# /spec:init

Initialise Specify in a project. Run once before any other `/spec:` skill.

## Synopsis

```text
/spec:init [<capability>]
/spec:init --hub
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<capability>` | Required for regular projects | Capability identifier or URL, e.g. `omnia` (bare name), `https://github.com/augentic/specify/capabilities/omnia` (URL), or `file:///…` (local URI). Supports an `@ref` suffix for version pinning. Mutually exclusive with `--hub`. |
| `--hub` | -- | Scaffold a registry-only platform hub instead of a regular project. No capability identifier is needed. |

## When to use

- Setting up a new project for spec-driven development.
- Re-initialising to change or update the capability.
- Bootstrapping a registry-only platform hub for multi-repo coordination.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| Project config | `.specify/project.yaml` | Capability identifier, domain description, project rules (regular); just `hub: true` (hub) |
| Capability cache | `.specify/.cache/<capability>/` | Cached capability manifest and brief files (regular only) |
| Directory structure | `.specify/{changes,specs,archive}/` | Empty scaffold (regular only) |

## Behavior

1. Checks whether `.specify/` already exists. If so, warns and offers to reconfigure.
2. Runs `specify init <capability>` (regular) or `specify init --hub` (hub); the CLI resolves the capability and caches its brief files into `.specify/.cache/` (regular mode only).
3. The CLI scaffolds the directory structure and writes `project.yaml`.
4. Detects existing source code in the project. If found, offers to create an `initial-baseline` change for `/spec:extract`.

## Lifecycle transitions

None -- init creates the project scaffold, not a change.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| Capability resolution failure | Invalid identifier or URL, network error, or missing `@ref` | Check identifier / URL and connectivity |
| `init-requires-capability-or-hub` | `specify init` invoked with neither a capability positional nor `--hub`, or with both | Pass exactly one of the two |
| `.specify/` already exists | Re-running init on an initialised project | Confirm reconfiguration or delete `.specify/` |

## Examples

```text
# Initialise with the Omnia capability (bare name)
/spec:init omnia

# Initialise with the Omnia capability (URL form)
/spec:init https://github.com/augentic/specify/capabilities/omnia

# Initialise with a pinned Vectis capability version
/spec:init https://github.com/augentic/specify/capabilities/vectis@v1

# Bootstrap a registry-only platform hub
/spec:init --hub
```

## See also

- [Prerequisites](../../orientation/prerequisites.md) -- what to install before init
- [Directory Layout](../directory-layout.md) -- what init creates
- [Configuration Files](../configuration.md) -- `project.yaml` format
