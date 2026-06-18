# Plugin Development

Specify ships as a Cursor plugin marketplace. Each plugin provides skills and optional reference documents. This page covers the development workflow, marketplace manifest structure, shared references, and testing.

> Domain-specific code generation lives in **target adapters** (`adapters/targets/<name>/`), not plugins. Omnia and Vectis are target adapters — see [`adapters/targets/omnia/`](https://github.com/augentic/specify-adapters/tree/main/adapters/targets/omnia/) and [`adapters/targets/vectis/`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/) — and their `shape` / `build` / `merge` briefs drive code generation directly without slash-command skills.

## Plugins overview

| Plugin | Directory | Prefix | Purpose |
|--------|-----------|--------|---------|
| Specify | `plugins/spec/` | `/spec:` | Core workflow (define, build, merge, verify, etc.) |
| Capture | `plugins/capture/` | `/capture:` | Runtime capture and regression testing |
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

The `check.ts` script validates that every plugin with a `.cursor-plugin/plugin.json` file is listed in the marketplace manifest, and that every listed plugin has a `skills/` directory.

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

### Testing adapter changes

Adapters are read from the filesystem at `/spec:init` time, not from the plugin cache. To iterate on adapters in a target project, symlink them:

```bash
SPECIFY_REPO="path/to/augentic/specify"
ln -sf "$SPECIFY_REPO/adapters" adapters
```

Adapter edits take effect immediately -- no cache clear or restart needed.

## Shared references

Agent-critical prose is **runtime-canonical** under [`plugins/spec/references/`](../../plugins/spec/references/). The Cursor plugin cache ships only `plugins/`, so skills must link to `references/…` siblings (or `../../references/…` from a skill directory), never to `docs/` via `../` escapes.

| Runtime file | Purpose |
|------|---------|
| [`plugins/spec/references/guardrails.md`](../../plugins/spec/references/guardrails.md) | Cross-cutting "do not / never / always" rules for skills |
| [`plugins/spec/references/specialist-usage.md`](../../plugins/spec/references/specialist-usage.md) | How specialists consume the four artifacts |
| [`plugins/spec/references/reconciliation.md`](../../plugins/spec/references/reconciliation.md) | Plan-time leads and slice-time evidence |
| [`plugins/spec/references/cli/plan-propose.md`](../../plugins/spec/references/cli/plan-propose.md) | `specify plan propose` contract for `/spec:plan` |

Contributor book and encyclopedic material stays in [`docs/`](../../docs/) (published at `https://specify.augentic.io/`). Use site URLs in optional "Reference documentation" tables; do not make guardrails or brief contracts depend on `docs/` paths at runtime.

Adapter briefs link to `references/spec-runtime/` inside each adapter (see [`adapters/shared/references/runtime/README.md`](../../adapters/shared/references/runtime/README.md)). Each adapter's `references/spec-runtime` is a single directory symlink to the shared bundle at `adapters/shared/references/runtime/`, which is itself a tree of symlinks to the plugin canonical files — so there is nothing to materialise or keep in sync. Edit the canonical file under `plugins/spec/references/`; `specify init` dereferences the symlinks into the cached adapter copy.

| Book-only (not agent runtime) | Purpose |
|------|---------|
| [`docs/reference/artifact-format.md`](../reference/artifact-format.md) | Full artifact format reference |
| [`docs/reference/cli-output-shapes.md`](../reference/cli-output-shapes.md) | JSON envelope shapes for CLI commands |
| [`docs/reference/review-team-protocol.md`](../reference/review-team-protocol.md) | Review-team protocol (canonical for mdBook; mirrored into `spec-runtime` for adapters) |

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

1. Run `make lint` to validate structural consistency.
2. Run `make use-local-plugins` and restart Cursor.
3. Open a target project and invoke the skill.
4. Verify the skill produces the expected artifacts and CLI interactions.
5. Check edge cases: missing inputs, invalid state, error recovery paths.

For generator skills (`crate-writer`, `core-writer`, etc.), verify that the generated code compiles and passes tests in the target project.
