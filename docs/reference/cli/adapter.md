# specify adapter

Adapter resolution, validation, and brief pipeline queries.

## Subcommands

### specify adapter resolve

Resolve a adapter identifier or URL to its local cache path.

```bash
specify adapter resolve <adapter>
```

Returns the filesystem path to the cached adapter. Used by skills to locate brief files.

### specify adapter pipeline

Show the brief pipeline for a phase.

```bash
specify adapter pipeline <phase> [--change <slice-dir>] [--format json]
```

| Phase | Description |
|-------|-------------|
| `plan` | Briefs that drive change planning (discovery, propose, assignment) |
| `define` | Briefs that generate artifacts (proposal, specs, composition*, design, tasks) |
| `build` | Briefs that drive implementation |
| `merge` | Briefs that drive spec merging |

*\* `composition` is Vectis-adapter only.*

Returns the ordered list of briefs with their dependencies. Each brief entry includes its absolute `path`, `needs` edges, `generates` target, and current `present` flag.

When `--change` is provided, the `present` flag reflects whether each brief's output already exists in the given slice directory.

## See also

- [Adapters](../adapters/index.md) -- available first-party adapters and manifest structure
- [Configuration Files](../configuration.md) -- adapter references in project.yaml
