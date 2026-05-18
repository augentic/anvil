# Anatomy of a Adapter

A adapter configures how Specify generates artifacts and builds code for a particular outcome domain. When a user runs `/spec:init <adapter>`, the identifier they provide determines which brief pipelines run during define, build, and merge.

## Directory layout

Each first-party adapter lives under `adapters/<name>/`:

```text
adapters/
├── adapter.schema.json       # JSON Schema for adapter.yaml
├── default/
│   ├── adapter.yaml          # Foundational pipeline declarations
│   ├── briefs/
│   └── codex/                   # Universal review rules
├── omnia/
│   ├── adapter.yaml          # Pipeline declarations
│   ├── tools.yaml               # Optional WASI tool sidecar
│   └── briefs/
│       ├── proposal.md
│       ├── specs.md
│       ├── design.md
│       ├── tasks.md
│       ├── build.md
│       └── merge.md
├── vectis/
│   ├── adapter.yaml
│   ├── tools.yaml               # Optional WASI tool sidecar
│   └── briefs/
│       ├── proposal.md
│       ├── specs.md
│       ├── composition.md       # Vectis-specific stage
│       ├── design.md
│       ├── tasks.md
│       ├── build.md
│       └── merge.md
└── contracts/
    ├── adapter.yaml
    ├── tools.yaml               # Optional WASI tool sidecar
    └── briefs/...
```

## `adapter.yaml`

The `adapter.yaml` file is the adapter's entry point. It declares the pipeline phases. It is validated against `adapters/adapter.schema.json` in this repository and against the equivalent schema bundled with the CLI.

Here is the Omnia adapter as a concrete example:

```yaml
# yaml-language-server: $schema=../adapter.schema.json
name: omnia
version: 1
description: Omnia Rust WASM workflow

pipeline:
  define:
    - id: proposal
      brief: briefs/proposal.md
    - id: specs
      brief: briefs/specs.md
    - id: design
      brief: briefs/design.md
    - id: tasks
      brief: briefs/tasks.md
  build:
    - id: build
      brief: briefs/build.md
  merge:
    - id: merge
      brief: briefs/merge.md
```

### Top-level fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Adapter identifier (e.g. `omnia`, `vectis`). Must match the directory name under `adapters/`. |
| `version` | integer | yes | Adapter version number (minimum 1). |
| `description` | string | yes | Human-readable description of the adapter. |
| `pipeline` | object | yes | Pipeline phases with ordered brief references. |

The post-RFC manifest deliberately drops the legacy `domain` and `extends` fields. Tech-stack guidance, architectural notes, and testing context belong in adapter references and skills, not in always-loaded manifest metadata.

### Optional codex directory

Adapters may ship a `codex/` directory next to `adapter.yaml` to distribute review rules owned by that adapter. The directory is a repository convention, not a manifest field: do not add `codex:` to `adapter.yaml`.

Codex files are Markdown documents with RM-03 frontmatter (`id`, `title`, `severity`, `trigger`) and a self-contained `## Rule` section. The foundational `default` adapter owns adapter-independent universal rules; domain adapters may add smaller packs for their own review concerns.

### Optional tool sidecar

Adapters may ship a `tools.yaml` sidecar next to `adapter.yaml` to declare WASI helper tools for `specify tool`. The `adapter.yaml` schema remains closed and unchanged; do not add a `tools:` field to it.

```yaml
# adapters/contracts/tools.yaml
tools:
  - "specify:contract@0.3.0"
```

Use scalar `specify:<tool>@<semver>` package requests for first-party released WASI tools. Use object declarations with absolute local paths or `file://` URIs for vendored and first-party development artifacts; those object declarations may include `sha256` pins when cache verification is useful.

Adapter-scope tools may use `$ADAPTER_DIR` in permission paths to read adapter-owned templates or resources. Project-scope declarations in `.specify/project.yaml` may not use `$ADAPTER_DIR`.

See [specify tool](../reference/cli/tool.md), [Tool Declarations](../explanation/tool-declarations.md), and [RFC-15](../../rfcs/archive/rfc-15-wasm-plugins.md) for the command surface, declaration precedence, cache layout, and security model.

### Pipeline structure

The `pipeline` object has three required phases -- `define`, `build`, and `merge`:

| Phase | When it runs | Minimum entries |
|-------|-------------|-----------------|
| `define` | `/spec:define` (artifact generation) | 1 |
| `build` | `/spec:build` (implementation) | 1 |
| `merge` | `/spec:merge` (baseline merge) | 1 |

> **No `pipeline.plan`.** Planning is orchestration, not adapter-owned slice work; both `adapters/adapter.schema.json` (this repo) and `schemas/adapter.schema.json` (CLI) reject `pipeline.plan` outright. Planning briefs live with the change-draft skill at [`plugins/change/skills/draft/briefs/<adapter>/`](../../plugins/change/skills/draft/briefs/) — see [RFC-13 §"Platform components are not adapters"](../../rfcs/archive/rfc-13-extensibility.md#platform-components-are-not-adapters) for the boundary that lands this rejection.

Each phase contains an ordered array of **pipeline entries**:

```yaml
- id: specs
  brief: briefs/specs.md
```

| Field | Description |
|-------|-------------|
| `id` | Brief identifier (must be unique across all phases within the adapter). |
| `brief` | Relative path from the adapter directory to the brief markdown file. |

## Brief files

Each brief is a markdown file with YAML frontmatter. The frontmatter declares metadata; the body contains the generation or implementation instructions that the agent follows.

### Brief frontmatter

```yaml
---
id: specs
description: Generate behavioral specifications from the proposal
generates: "specs/**/*.md"
needs: [proposal]
---
```

| Field | Required | Description |
|-------|----------|-------------|
| `id` | yes | Must match the `id` in the corresponding pipeline entry. |
| `description` | yes | What this brief produces or does. |
| `generates` | no | Output filename or glob pattern (e.g. `proposal.md`, `specs/**/*.md`). |
| `needs` | no | Array of brief IDs that must complete before this brief runs. |
| `tracks` | no | Brief ID whose output file tracks build progress (build briefs only). |

### Execution order

The `needs` dependencies form a directed acyclic graph. The `/spec:define` skill resolves the topological order by calling `specify adapter pipeline define --change <dir>`, which returns the briefs in dependency-safe order. Skills execute each brief in sequence, reading the artifacts generated by earlier briefs.

For example, the Omnia define pipeline resolves to:

```text
proposal → specs → design → tasks
```

The Vectis define pipeline adds `composition` between `specs` and `design`:

```text
proposal → specs → composition → design → tasks
```

## Adapter resolution

Projects reference their adapter in `.specify/project.yaml`:

```yaml
adapter: https://github.com/augentic/specify/adapters/omnia
```

Adapter identifiers support a bare name, an `https://…` URL, a `file:///…` URI, and an optional `@ref` suffix to pin a specific git ref:

```text
https://github.com/augentic/specify/adapters/omnia@v1
```

The CLI resolves the adapter to a local directory by checking `.specify/.cache/` first (populated at `/spec:init` time) and fetching from the remote if needed. See [Configuration Files](../reference/configuration.md) for the full resolution algorithm.

## Adding or modifying a adapter

1. **Create the directory** under `adapters/<name>/` with a `adapter.yaml` and a `briefs/` subdirectory. Add `codex/` only when the adapter owns review rules.

2. **Declare the pipeline.** List the define, build, and merge phases with their brief entries.

3. **Write the briefs.** Each brief needs YAML frontmatter with at minimum `id` and `description`. The `needs` array declares dependencies on other briefs. The body contains the agent instructions for that pipeline stage.

4. **Validate.** Run `make checks` -- the `checks.ts` script verifies:
   - `adapter.yaml` validates against `adapters/adapter.schema.json`
   - All `brief` paths resolve to existing files
   - Brief frontmatter `id` matches the pipeline entry `id`
   - All `needs` references point to declared brief IDs
   - The `needs` graph contains no cycles (Kahn's algorithm)

5. **Register in the README.** Update `adapters/README.md` with the new adapter's entry and mention any codex pack the adapter owns.
