# Repository Structure

```text
specify/
├── .cursor/
│   └── rules/                    # Project guidance for agents
├── .cursor-plugin/
│   └── marketplace.json          # Multi-plugin marketplace manifest
├── docs/                         # Extended documentation
│   ├── architecture.md           # Repository structure reference
│   └── plugins.md                # Full plugin and skill reference
├── plugins/
│   ├── references/               # Shared references (specify.md, agent-teams.md)
│   ├── spec/                     # Specify workflow plugin
│   │   ├── skills/               # Workflow skills (init, propose, apply, ...)
│   │   ├── references/           # Artifact templates and schema resolution
│   │   └── mcp.json              # MCP server definition
│   ├── omnia/                    # Omnia code generation plugin
│   │   ├── skills/               # Code generation skills (crate-writer, test-writer, ...)
│   │   ├── references/           # Guardrails, providers, guest wiring patterns
│   │   └── mcp.json
│   ├── rt/                       # RT migration plugin
│   │   ├── skills/               # Migration skills (code-analyzer, replay-writer, ...)
│   │   └── mcp.json
│   └── plan/                     # Plan requirements analysis plugin
│       ├── skills/               # Planning skills (epic-analyzer, sow-writer)
│       └── mcp.json
├── schemas/                      # Schema definitions (omnia, realtime)
│   ├── omnia/                    # Greenfield Rust WASM schema
│   └── realtime/                 # TypeScript migration schema (extends omnia)
└── scripts/                      # Validation and plugin management
    ├── checks.sh                 # Documentation and consistency checks
    ├── dev-plugins.sh            # Symlink local plugins for development
    └── prod-plugins.sh           # Restore marketplace plugins
```

## Artifact Boundaries

Specify artifacts have separate responsibilities:

- **`proposal.md`** -- Why the change exists and what is in scope
- **`spec.md`** -- Behavioral requirements, scenarios, error conditions, optional metrics
- **`design.md`** -- Domain model, APIs, integrations, configuration, technical logic
- **`tasks.md`** -- Implementation sequencing only

Behavioral specs should remain platform-neutral. Omnia trait selection, guest wiring, and WASM translation belong in specialist skills and references.

## File Locations

In downstream consumer projects:

- **Crates**: `$PROJECT_DIR/crates/<crate_name>/`
- **Metrics**: `$PROJECT_DIR/.metrics.json` when tracking is enabled

In this repository:

- **Working artifacts**: `$PROJECT_DIR/.specify/changes/<change-name>/`
- **Baseline specs**: `$PROJECT_DIR/.specify/specs/`
