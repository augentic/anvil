# /spec:init

Initialise Specify in a project. Run once before any other `/spec:` skill.

## Synopsis

```text
/spec:init [schema-url]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `schema-url` | Recommended | Schema URL, e.g. `https://github.com/augentic/specify/schemas/omnia`. Supports `@ref` suffix for version pinning. Required for regular projects unless the skill can infer an appropriate default. |

## When to use

- Setting up a new project for spec-driven development.
- Re-initialising to change or update the schema.

## Artifacts produced

| Artifact | Location | Content |
|----------|----------|---------|
| Project config | `.specify/project.yaml` | Schema URL, domain description, project rules |
| Schema cache | `.specify/.cache/<schema>/` | Cached schema and brief files |
| Directory structure | `.specify/{changes,specs,archive}/` | Empty scaffold |

## Behavior

1. Checks whether `.specify/` already exists. If so, warns and offers to reconfigure.
2. Runs `specify init --schema-uri <uri>`; the CLI resolves the schema and caches its brief files into `.specify/.cache/`.
3. The CLI scaffolds the directory structure and writes `project.yaml`.
4. Detects existing source code in the project. If found, offers to create an `initial-baseline` change for `/spec:extract`.

## Lifecycle transitions

None -- init creates the project scaffold, not a change.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| Schema resolution failure | Invalid URL, network error, or missing `@ref` | Check URL and connectivity |
| `.specify/` already exists | Re-running init on an initialised project | Confirm reconfiguration or delete `.specify/` |

## Examples

```text
# Initialise with the Omnia schema
/spec:init https://github.com/augentic/specify/schemas/omnia

# Initialise with a pinned Vectis schema version
/spec:init https://github.com/augentic/specify/schemas/vectis@v1
```

## See also

- [Prerequisites](../../orientation/prerequisites.md) -- what to install before init
- [Directory Layout](../directory-layout.md) -- what init creates
- [Configuration Files](../configuration.md) -- `project.yaml` format
