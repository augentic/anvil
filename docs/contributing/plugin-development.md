# Plugin Development

Specify ships as a Cursor plugin marketplace containing six plugins. Each plugin provides skills and optional reference documents. This page covers the development workflow, marketplace manifest structure, shared references, and testing.

## Plugins overview

| Plugin | Directory | Prefix | Purpose |
|--------|-----------|--------|---------|
| Specify | `plugins/spec/` | `/spec:` | Core workflow (define, build, merge, verify, etc.) |
| Omnia | `plugins/omnia/` | `/omnia:` | Rust WASM crate generation and review |
| Vectis | `plugins/vectis/` | `/vectis:` | Cross-platform Crux app generation |
| Contract | `plugins/contract/` | `/contract:` | API contract generation and validation (OpenAPI, AsyncAPI, JSON Schema) |
| RT | `plugins/rt/` | `/rt:` | Fixture capture and regression testing |
| Client | `plugins/client/` | `/client:` | Client-facing deliverables (SoW, proposals, pricing summaries) |

Each plugin directory follows the same structure:

```text
plugins/<name>/
├── .cursor-plugin/
│   └── plugin.json             # Per-plugin manifest (optional)
├── skills/
│   ├── <skill-name>/
│   │   ├── SKILL.md
│   │   └── references/         # Optional supporting docs
│   └── ...
└── references/                 # Plugin-level shared references (optional)
```

## Marketplace manifest

The top-level marketplace manifest at `.cursor-plugin/marketplace.json` declares all plugins:

```json
{
  "name": "augentic",
  "owner": { "name": "augentic", "email": "info@augentic.io" },
  "metadata": {
    "description": "Spec-driven code generation plugins for Augentic.",
    "version": "0.24.1",
    "pluginRoot": "plugins"
  },
  "plugins": [
    {
      "name": "spec",
      "source": "spec",
      "description": "Skills for Specify workflow orchestration..."
    }
  ]
}
```

| Field | Description |
|-------|-------------|
| `metadata.pluginRoot` | Base directory for all plugins (always `"plugins"`) |
| `plugins[].name` | Plugin identifier used in the Cursor UI |
| `plugins[].source` | Subdirectory name under `pluginRoot` |
| `plugins[].description` | Human-readable description |

The `checks.ts` script validates that every plugin with a `.cursor-plugin/plugin.json` file is listed in the marketplace manifest, and that every listed plugin has a `skills/` directory.

## Dev/prod workflow

Cursor's plugin cache is populated from the server when missing and left alone when present. The local plugin script exploits this by clearing the cache and repopulating it with your local files.

### Iteration loop

1. Edit skills, rules, or references in `plugins/`.
2. Run `make use-local-plugins` to copy local files into the Cursor cache.
3. **Restart Cursor** (a window reload is not sufficient).
4. Test in a target project.
5. Repeat from step 1.

```bash
make use-local-plugins    # copy local plugins into Cursor cache
```

When finished, revert to published plugins:

```bash
make use-team-plugins   # clear cache; Cursor refetches from server on restart
```

### Testing capability changes

Capabilities are read from the filesystem at `/spec:init` time, not from the plugin cache. To iterate on capabilities in a target project, symlink them:

```bash
SPECIFY_REPO="path/to/augentic/specify"
ln -sf "$SPECIFY_REPO/capabilities" capabilities
```

Capability edits take effect immediately -- no cache clear or restart needed.

## Shared references

Files in `plugins/references/` are shared across plugins:

| File | Purpose |
|------|---------|
| `specify.md` | Master reference: artifact format, lifecycle states, naming conventions, delta-merge rules, hard constraints |
| `agent-teams.md` | Multi-agent review pattern (structural, logic, quality specialists + antagonist) |
| `review-checks.md` | Transitional pointer to universal codex rules used by existing reviewer skills |

Skills reference these files via symlinks. For example, a skill at `plugins/omnia/skills/crate-writer/` might symlink `references/specify.md` to `../../../references/specify.md`. This keeps relative paths in the skill body short while sharing a single source of truth.

The `checks.ts` script validates that all symlinks under `plugins/` resolve to valid targets.

## Publishing a new plugin

New plugins added to `marketplace.json` require a one-time server-side setup:

1. Push the plugin directory to `main` and merge.
2. Open the Cursor plugin marketplace dashboard.
3. Refresh the marketplace (even if auto-refresh is enabled).
4. Set the new plugin to **Required**.
5. Click **Save**.
6. Restart Cursor locally to pick up the new plugin.

After this initial setup, the plugin participates in the normal dev/prod workflow.

## Testing a skill change

There is no automated test harness for skills (they are markdown documents interpreted by an LLM). Testing is manual:

1. Run `make checks` to validate structural consistency.
2. Run `make use-local-plugins` and restart Cursor.
3. Open a target project and invoke the skill.
4. Verify the skill produces the expected artifacts and CLI interactions.
5. Check edge cases: missing inputs, invalid state, error recovery paths.

For generator skills (`crate-writer`, `core-writer`, etc.), verify that the generated code compiles and passes tests in the target project.
