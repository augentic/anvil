# Anatomy of a Capability

A capability configures how Specify generates artifacts and builds code for a particular outcome domain. When a user runs `/spec:init <capability>`, the identifier they provide determines which brief pipelines run during define, build, and merge.

## Directory layout

Each first-party capability lives under `capabilities/<name>/`:

```text
capabilities/
├── capability.schema.json       # JSON Schema for capability.yaml
├── omnia/
│   ├── capability.yaml          # Pipeline declarations
│   ├── tools.yaml               # Optional WASI tool sidecar
│   └── briefs/
│       ├── proposal.md
│       ├── specs.md
│       ├── design.md
│       ├── tasks.md
│       ├── build.md
│       └── merge.md
├── vectis/
│   ├── capability.yaml
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
    ├── capability.yaml
    ├── tools.yaml               # Optional WASI tool sidecar
    └── briefs/...
```

## `capability.yaml`

The `capability.yaml` file is the capability's entry point. It declares the pipeline phases. It is validated against `capabilities/capability.schema.json` in this repository and against the equivalent schema bundled with the CLI.

Here is the Omnia capability as a concrete example:

```yaml
# yaml-language-server: $schema=../capability.schema.json
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
| `name` | string | yes | Capability identifier (e.g. `omnia`, `vectis`). Must match the directory name under `capabilities/`. |
| `version` | integer | yes | Capability version number (minimum 1). |
| `description` | string | yes | Human-readable description of the capability. |
| `pipeline` | object | yes | Pipeline phases with ordered brief references. |

The post-RFC manifest deliberately drops the legacy `domain` and `extends` fields. Tech-stack guidance, architectural notes, and testing context belong in capability references and skills, not in always-loaded manifest metadata.

### Optional tool sidecar

Capabilities may ship a `tools.yaml` sidecar next to `capability.yaml` to declare WASI helper tools for `specify tool`. The `capability.yaml` schema remains closed and unchanged; do not add a `tools:` field to it.

```yaml
# capabilities/contracts/tools.yaml
tools:
  - name: contract
    version: 1.0.0
    source: "https://github.com/augentic/specify-tools/releases/download/1.0.0/contract.wasm"
    sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    permissions:
      read:
        - "$PROJECT_DIR/contracts"
      write: []
```

Use absolute local paths or `file://` URIs for vendored and first-party development artifacts. Use `https://` for third-party or released artifacts. Released tool declarations should include `sha256`; first-party release declarations require it so cache fills verify the exact component bytes.

Capability-scope tools may use `$CAPABILITY_DIR` in permission paths to read capability-owned templates or resources. Project-scope declarations in `.specify/project.yaml` may not use `$CAPABILITY_DIR`.

See [specify tool](../reference/cli/tool.md), [Tool Declarations](../explanation/tool-declarations.md), and [RFC-15](../../rfcs/rfc-15-wasm-plugins.md) for the command surface, declaration precedence, cache layout, and security model.

### Pipeline structure

The `pipeline` object has three required phases -- `define`, `build`, and `merge`:

| Phase | When it runs | Minimum entries |
|-------|-------------|-----------------|
| `define` | `/spec:define` (artifact generation) | 1 |
| `build` | `/spec:build` (implementation) | 1 |
| `merge` | `/spec:merge` (baseline merge) | 1 |

> **No `pipeline.plan`.** Planning is orchestration, not capability-owned slice work; both `capabilities/capability.schema.json` (this repo) and `schemas/capability.schema.json` (CLI) reject `pipeline.plan` outright. Planning briefs live with the change-planning skill at [`plugins/change/skills/plan/briefs/<capability>/`](../../plugins/change/skills/plan/briefs/) — see [RFC-13 §3.11](../../rfcs/rfc-13-plan.md) for the migration that landed this rejection.

Each phase contains an ordered array of **pipeline entries**:

```yaml
- id: specs
  brief: briefs/specs.md
```

| Field | Description |
|-------|-------------|
| `id` | Brief identifier (must be unique across all phases within the capability). |
| `brief` | Relative path from the capability directory to the brief markdown file. |

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

The `needs` dependencies form a directed acyclic graph. The `/spec:define` skill resolves the topological order by calling `specify capability pipeline define --change <dir>`, which returns the briefs in dependency-safe order. Skills execute each brief in sequence, reading the artifacts generated by earlier briefs.

For example, the Omnia define pipeline resolves to:

```text
proposal → specs → design → tasks
```

The Vectis define pipeline adds `composition` between `specs` and `design`:

```text
proposal → specs → composition → design → tasks
```

## Capability resolution

Projects reference their capability in `.specify/project.yaml`:

```yaml
capability: https://github.com/augentic/specify/capabilities/omnia
```

Capability identifiers support a bare name, an `https://…` URL, a `file:///…` URI, and an optional `@ref` suffix to pin a specific git ref:

```text
https://github.com/augentic/specify/capabilities/omnia@v1
```

The CLI resolves the capability to a local directory by checking `.specify/.cache/` first (populated at `/spec:init` time) and fetching from the remote if needed. See [Configuration Files](../reference/configuration.md) for the full resolution algorithm.

## Adding or modifying a capability

1. **Create the directory** under `capabilities/<name>/` with a `capability.yaml` and a `briefs/` subdirectory.

2. **Declare the pipeline.** List the define, build, and merge phases with their brief entries.

3. **Write the briefs.** Each brief needs YAML frontmatter with at minimum `id` and `description`. The `needs` array declares dependencies on other briefs. The body contains the agent instructions for that pipeline stage.

4. **Validate.** Run `make checks` -- the `checks.ts` script verifies:
   - `capability.yaml` validates against `capabilities/capability.schema.json`
   - All `brief` paths resolve to existing files
   - Brief frontmatter `id` matches the pipeline entry `id`
   - All `needs` references point to declared brief IDs
   - The `needs` graph contains no cycles (Kahn's algorithm)

5. **Register in the README.** Update `capabilities/README.md` with the new capability's entry.
