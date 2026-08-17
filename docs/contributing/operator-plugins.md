# Cursor operator plugins

Emery's product is the Rust `emery` runtime and the source/target adapters in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters). The `plugins/` tree is only the Cursor distribution surface: ultrathin slash-command wrappers that invoke CLI verbs and relay their output.

Do not put orchestration, synthesis, validation, or code-generation prose in skill bodies. That work lives in guest orchestrations, embedded judgment prompts (`crates/slice/prompts/`, `crates/change/prompts/`), and adapter `prose/prompts/` in the adapters repo.

## What ships

| Plugin | Directory | Prefix | Role |
| ------ | --------- | ------ | ---- |
| Emery | `plugins/emery/` | `/emery:` | Workflow wrappers: `init`, `plan`, `correct`, `refine`, `execute`, `status`, `finalize`, plus `/emery:system-*` (`system-survey`, `system-plan`, `system-review`) |

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

The marketplace manifest at `.cursor-plugin/marketplace.json` lists exactly these plugins. Keep the manifest in sync with the directories under `plugins/` when adding or renaming a plugin.

## Preview a working-tree plugin

The `/emery:*` skills are ultrathin wrappers: they check `emery --version`, elicit arguments, invoke one CLI verb, and relay stdout. Preview the working-tree plugin separately from building the CLI that skills call.

From the repository root:

```bash
cursor-agent --plugin-dir plugins/emery
```

That loads `plugins/emery/` instead of the marketplace copy. Omit it to use the published marketplace plugin. Then run `/emery:init`, `/emery:plan`, and the other skills in chat as usual.

Skills need a real binary on `PATH` (`cargo make eval` is the native lab shim, not this):

```bash
cargo install --path . --locked
emery --version
```

Reinstall when you change the CLI. Guest verbs that call the model also need `cursor-agent` on `PATH` and logged in.

| You're changing…                 | Do this                                                                         |
| -------------------------------- | ------------------------------------------------------------------------------- |
| `plugins/emery/skills/*/SKILL.md` | `cursor-agent --plugin-dir plugins/emery` (no rebuild)                           |
| Rust CLI / guest orchestrations  | `cargo install --path . --locked`, then use skills or call `emery …` directly |

For a dry run without Cursor, call the same verbs the skills wrap (`emery init`, `emery plan author`, …) after install.

## Publishing a new Cursor plugin

1. Add the plugin under `plugins/<name>/` and list it in `marketplace.json`.
2. When `plugins/` or marketplace content changes, bump `.cursor-plugin/marketplace.json` `metadata.version` and each `plugins/*/.cursor-plugin/plugin.json` `version` in the same PR (plugin SemVer is independent of the host CLI).
3. Merge to `main`.
4. In the Cursor plugin marketplace dashboard: refresh, set the new plugin to **Required**, save.
5. Restart Cursor locally.

## Related

- [Skill / CLI responsibility split](../../AGENTS.md#skill--cli-responsibility-split) — wrappers stay ultrathin
- [Adapter anatomy](../explanation/adapter-anatomy.md) — Cursor `plugin.json` vs adapter components
