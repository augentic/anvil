# specify capability

Capability resolution, validation, and brief pipeline queries.

## Subcommands

### specify capability resolve

Resolve a capability identifier or URL to its local cache path.

```bash
specify capability resolve <capability>
```

Returns the filesystem path to the cached capability. Used by skills to locate brief files.

### specify capability check

Validate a capability manifest's structural integrity.

```bash
specify capability check [<capability>]
```

Checks that `capability.yaml` conforms to the capability JSON Schema, all referenced brief files exist, and the pipeline topology is acyclic.

### specify capability pipeline

Show the brief pipeline for a phase.

```bash
specify capability pipeline <phase> [--change <change-dir>] [--format json]
```

| Phase | Description |
|-------|-------------|
| `plan` | Briefs that drive initiative planning (discovery, propose, assignment) |
| `define` | Briefs that generate artifacts (proposal, specs, composition*, design, tasks) |
| `build` | Briefs that drive implementation |
| `merge` | Briefs that drive spec merging |

*\* `composition` is Vectis-capability only.*

Returns the ordered list of briefs with their dependencies. Each brief entry includes its absolute `path`, `needs` edges, `generates` target, and current `present` flag.

When `--change` is provided, the `present` flag reflects whether each brief's output already exists in the given change directory.

## See also

- [Capabilities](../capabilities/index.md) -- available first-party capabilities and manifest structure
- [Configuration Files](../configuration.md) -- capability references in project.yaml
