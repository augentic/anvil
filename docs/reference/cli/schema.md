# specify schema

Schema resolution, validation, and brief pipeline queries.

## Subcommands

### specify schema resolve

Resolve a schema URL to its local cache path.

```bash
specify schema resolve <schema-url>
```

Returns the filesystem path to the cached schema. Used by skills to locate brief files.

### specify schema check

Validate a schema's structural integrity.

```bash
specify schema check [<schema-url>]
```

Checks that `schema.yaml` conforms to the schema JSON Schema, all referenced brief files exist, and the pipeline topology is acyclic.

### specify schema pipeline

Show the brief pipeline for a phase.

```bash
specify schema pipeline <phase> [--change <change-dir>] [--format json]
```

| Phase | Description |
|-------|-------------|
| `plan` | Briefs that drive initiative planning (discovery, propose, assignment) |
| `define` | Briefs that generate artifacts (proposal, specs, composition*, design, tasks) |
| `build` | Briefs that drive implementation |
| `merge` | Briefs that drive spec merging |

*\* `composition` is Vectis-schema only.*

Returns the ordered list of briefs with their dependencies. Each brief entry includes its absolute `path`, `needs` edges, `generates` target, and current `present` flag.

When `--change` is provided, the `present` flag reflects whether each brief's output already exists in the given change directory.
