# Plugin Development

Specify ships as a Cursor plugin marketplace. Each plugin provides skills and optional reference documents. This page covers the development workflow, marketplace manifest structure, shared references, and testing.

> Domain-specific code generation lives in **target adapters** (`adapters/targets/<name>/`), not plugins. Omnia and Vectis are target adapters — see [`adapters/targets/omnia/`](https://github.com/augentic/specify-adapters/tree/main/targets/omnia/) and [`adapters/targets/vectis/`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/) — and their `guidance` / `build` / `merge` prompts drive code generation directly without slash-command skills.

## Plugins overview

| Plugin  | Directory          | Prefix      | Purpose                                                        |
| ------- | ------------------ | ----------- | -------------------------------------------------------------- |
| Specify | `plugins/spec/`    | `/spec:`    | Core workflow (define, build, merge, verify, etc.)             |
| Capture | `plugins/capture/` | `/capture:` | Runtime capture and regression testing                         |

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

| Field                   | Description                                         |
| ----------------------- | --------------------------------------------------- |
| `metadata.pluginRoot`   | Base directory for all plugins (always `"plugins"`) |
| `plugins[].name`        | Plugin identifier used in the Cursor UI             |
| `plugins[].source`      | Subdirectory name under `pluginRoot`                |
| `plugins[].description` | Human-readable description                          |

The `check.ts` script validates that every plugin with a `.cursor-plugin/plugin.json` file is listed in the marketplace manifest, and that every listed plugin has a `skills/` directory.

## Dev/prod workflow

Cursor Agent loads working-tree plugins directly with `--plugin-dir`.

### Iteration loop

1. Edit skills, rules, or references in `plugins/`.
2. Start Cursor Agent with the plugin directory.
3. Test in a target project.
4. Repeat from step 1.

```bash
cursor-agent --plugin-dir plugins/spec
```

Pass `--plugin-dir` once per local plugin when testing more than one. Omit it to use the published plugins.

### Testing adapter changes

Adapters are read from the filesystem at `/spec:init` time, not from the plugin cache. To iterate on adapters in a target project, symlink them:

```bash
SPECIFY_REPO="path/to/augentic/specify"
ln -sf "$SPECIFY_REPO/adapters" adapters
```

Adapter edits take effect immediately -- no cache clear or restart needed.

## Shared references

The Cursor plugin cache ships only `plugins/`, so any prose a skill links at runtime must live under the plugin (`references/…` siblings, or `../../references/…` from a skill directory) — never in `docs/` via `../` escapes. The spec plugin keeps a single runtime reference: [`plugins/spec/references/guardrails.md`](../../plugins/spec/references/guardrails.md), the cross-cutting "do not / never / always" rules the seven wrappers link to.

Everything else lives with its owner:

- **Core judgment prose** (lead reconciliation, the synthesis playbook, spec formatting, Decision Record authoring) is embedded in the `specify` binary from `crates/workflow/prompts/` — it is not plugin material.
- **Adapter prompts and references** live in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters): each adapter's `references/spec-runtime` is a directory symlink to that repo's `codex/references/runtime/` shared bundle, embedded into the published adapter components at build time. That bundle is self-contained; there is no mirror to maintain against this repository.
- **Contributor book and encyclopedic material** stays in [`docs/`](../../docs/) (published at `https://specify.augentic.io/`). Use site URLs in optional "Reference documentation" tables; do not make guardrails or brief contracts depend on `docs/` paths at runtime.

| Book-only (not agent runtime)                                                    | Purpose                                                                                          |
| -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| [`docs/reference/artifact-format.md`](../reference/artifact-format.md)           | Full artifact format reference                                                                   |
| [`docs/reference/cli-output-shapes.md`](../reference/cli-output-shapes.md)       | JSON envelope shapes for CLI commands                                                            |
| [`docs/reference/review-team-protocol.md`](../reference/review-team-protocol.md) | Review-team protocol (canonical for mdBook; forked into the adapters repo's shared bundle)       |

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

1. Run `cargo test --test framework` to validate structural consistency.
2. Start `cursor-agent --plugin-dir plugins/<name>`.
3. Open a target project and invoke the skill.
4. Verify the skill produces the expected artifacts and CLI interactions.
5. Check edge cases: missing inputs, invalid state, error recovery paths.

For generator skills (`crate-writer`, `core-writer`, etc.), verify that the generated code compiles and passes tests in the target project.
