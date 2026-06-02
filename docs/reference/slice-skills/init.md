# /spec:init

Initialise Specify in a project. Run once before any other `/spec:` skill.

## Synopsis

```text
/spec:init [<adapter>]
/spec:init workspace
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<target>` | Required for regular projects | Target identifier or URL, e.g. `omnia` (bare name), `https://github.com/augentic/specify/adapters/targets/omnia` (URL), or `file:///…` (local URI). Supports an `@ref` suffix for version pinning. Mutually exclusive with `--workspace`. |
| `--workspace` | -- | Scaffold a registry-only workspace instead of a regular project. No adapter identifier is needed. |

## When to use

- Setting up a new project for spec-driven development.
- Re-initialising to change or update the adapter.
- Bootstrapping a registry-only workspace for multi-repo coordination.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| Project config | `.specify/project.yaml` | Adapter identifier, domain description, project rules (regular); just `workspace: true` (workspace) |
| Adapter cache | `.specify/.cache/manifests/targets/<adapter>/` | Cached target adapter manifest and brief files (regular only) |
| Directory structure | `.specify/{slices,specs,archive}/` | Empty scaffold (regular only) |
| Agent context | `AGENTS.md` | Generated repository guidance when root `AGENTS.md` is absent |
| Context lock | `.specify/context.lock` | Fingerprint sidecar for init-time `AGENTS.md` generation |

## Behavior

The authoritative step-by-step lives in the [`/spec:init` skill body](../../../plugins/spec/skills/init/SKILL.md); the operator summary follows.

1. Checks whether `.specify/` already exists. If so, warns and offers to reconfigure.
2. Runs `specrun init <adapter>` (regular) or `specrun init --workspace` (workspace); the CLI resolves the adapter and caches its brief files into `.specify/.cache/` (regular mode only).
3. The CLI scaffolds the directory structure, writes `project.yaml`, and generates `AGENTS.md` plus `.specify/context.lock` when root `AGENTS.md` is absent.
4. Existing root `AGENTS.md` files are preserved byte-for-byte; init reports the skip instead of overwriting them.
5. Detects existing source code in the project. If found, the operator can bind it as a `code-typescript` (or future-language) source on the first `/spec:plan` invocation.

## Lifecycle transitions

None -- init creates the project scaffold, not a slice.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| Adapter resolution failure | Invalid identifier or URL, network error, or missing `@ref` | Check identifier / URL and connectivity |
| Clap parse error (exit 2) | `specrun init` invoked with neither a adapter positional nor `--workspace`, or with both | Pass exactly one of the two |
| `.specify/` already exists | Re-running init on an initialised project | Confirm reconfiguration or delete `.specify/` |

## Examples

```text
# Initialise with the Omnia adapter (bare name)
/spec:init omnia

# Initialise with the Omnia target (URL form)
/spec:init https://github.com/augentic/specify/adapters/targets/omnia

# Initialise with a pinned Vectis target version
/spec:init https://github.com/augentic/specify/adapters/targets/vectis@v1

# Bootstrap a registry-only workspace
/spec:init workspace
```

## See also

- [Prerequisites](../../orientation/prerequisites.md) -- what to install before init
- [Directory Layout](../directory-layout.md) -- what init creates
- [Configuration Files](../configuration.md) -- `project.yaml` format
