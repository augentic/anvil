# /spec:init

Initialise Specify in a project. Run once before any other `/spec:` skill.

## Synopsis

```text
/spec:init [<adapter>]
/spec:init hub
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<adapter>` | Required for regular projects | Adapter identifier or URL, e.g. `omnia` (bare name), `https://github.com/augentic/specify/adapters/omnia` (URL), or `file:///…` (local URI). Supports an `@ref` suffix for version pinning. Mutually exclusive with `--hub`. |
| `--hub` | -- | Scaffold a registry-only platform hub instead of a regular project. No adapter identifier is needed. |

## When to use

- Setting up a new project for spec-driven development.
- Re-initialising to change or update the adapter.
- Bootstrapping a registry-only platform hub for multi-repo coordination.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| Project config | `.specify/project.yaml` | Adapter identifier, domain description, project rules (regular); just `hub: true` (hub) |
| Adapter cache | `.specify/.cache/<adapter>/` | Cached adapter manifest and brief files (regular only) |
| Directory structure | `.specify/{slices,specs,archive}/` | Empty scaffold (regular only) |
| Agent context | `AGENTS.md` | Generated repository guidance when root `AGENTS.md` is absent |
| Context lock | `.specify/context.lock` | Fingerprint sidecar for `specify context check` |

## Behavior

1. Checks whether `.specify/` already exists. If so, warns and offers to reconfigure.
2. Runs `specify init <adapter>` (regular) or `specify init --hub` (hub); the CLI resolves the adapter and caches its brief files into `.specify/.cache/` (regular mode only).
3. The CLI scaffolds the directory structure, writes `project.yaml`, and generates `AGENTS.md` plus `.specify/context.lock` when root `AGENTS.md` is absent.
4. Existing root `AGENTS.md` files are preserved byte-for-byte; init reports the skip instead of overwriting them.
5. Detects existing source code in the project. If found, offers to create an `initial-baseline` change for `/spec:extract`.

## Lifecycle transitions

None -- init creates the project scaffold, not a slice.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| Adapter resolution failure | Invalid identifier or URL, network error, or missing `@ref` | Check identifier / URL and connectivity |
| `init-requires-adapter-or-hub` | `specify init` invoked with neither a adapter positional nor `--hub`, or with both | Pass exactly one of the two |
| `.specify/` already exists | Re-running init on an initialised project | Confirm reconfiguration or delete `.specify/` |

## Examples

```text
# Initialise with the Omnia adapter (bare name)
/spec:init omnia

# Initialise with the Omnia adapter (URL form)
/spec:init https://github.com/augentic/specify/adapters/omnia

# Initialise with a pinned Vectis adapter version
/spec:init https://github.com/augentic/specify/adapters/vectis@v1

# Bootstrap a registry-only platform hub
/spec:init hub
```

## See also

- [Prerequisites](../../orientation/prerequisites.md) -- what to install before init
- [Directory Layout](../directory-layout.md) -- what init creates
- [Configuration Files](../configuration.md) -- `project.yaml` format
