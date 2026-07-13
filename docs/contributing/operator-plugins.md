# Cursor operator plugins

Specify's product is the Rust `specify` runtime and the source/target adapters in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters). The `plugins/` tree is only the Cursor distribution surface: ultrathin slash-command wrappers that invoke CLI verbs and relay their output.

Do not put orchestration, synthesis, validation, or code-generation prose in skill bodies. That work lives in guest orchestrations, embedded judgment prompts (`crates/slice/prompts/`, `crates/change/prompts/`), and adapter `prose/prompts/` in the adapters repo.

## What ships

| Plugin | Directory | Prefix | Role |
| ------ | --------- | ------ | ---- |
| Specify | `plugins/spec/` | `/spec:` | Workflow wrappers: `init`, `plan`, `refine`, `build`, `merge`, `drop`, `finalize` |
| Capture | `plugins/capture/` | `/capture:` | Runtime capture for migration workflows |

Layout:

```text
plugins/<name>/
├── .cursor-plugin/
│   └── plugin.json
├── skills/
│   └── <skill-name>/
│       └── SKILL.md
└── rules/                      # optional .mdc context
```

The marketplace manifest at `.cursor-plugin/marketplace.json` lists exactly these plugins. Shape is documented by `schemas/authoring/marketplace.schema.json` and `schemas/authoring/skill.schema.json` (editor contracts; see [Consistency Checks](checks.md) for what CI actually enforces).

## Preview a working-tree plugin

```bash
cursor-agent --plugin-dir plugins/spec
```

Pass `--plugin-dir` once per local plugin when testing more than one. Omit it to use the published marketplace plugins.

## Publishing a new Cursor plugin

1. Add the plugin under `plugins/<name>/` and list it in `marketplace.json`.
2. Merge to `main`.
3. In the Cursor plugin marketplace dashboard: refresh, set the new plugin to **Required**, save.
4. Restart Cursor locally.

## Related

- [Skill / CLI responsibility split](../../AGENTS.md#skill--cli-responsibility-split) — wrappers stay ultrathin
- [Consistency Checks](checks.md) — skill and marketplace predicates
- [Adapter anatomy](../explanation/adapter-anatomy.md) — Cursor `plugin.json` vs adapter components
