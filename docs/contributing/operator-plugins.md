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

The marketplace manifest at `.cursor-plugin/marketplace.json` lists exactly these plugins. Shape is enforced by the typed structs in `crates/checks/authoring.rs` (the `checks` package; see [Consistency Checks](checks.md) for what CI actually enforces).

## Preview a working-tree plugin

The `/spec:*` skills are ultrathin wrappers: they check `specify --version`, elicit arguments, invoke one CLI verb, and relay stdout. Preview the working-tree plugin separately from building the CLI that skills call.

From the repository root:

```bash
cursor-agent --plugin-dir plugins/spec
```

That loads `plugins/spec/` instead of the marketplace copy. Pass `--plugin-dir` once per local plugin when testing more than one (e.g. add `plugins/capture`). Omit it to use the published marketplace plugins. Then run `/spec:init`, `/spec:plan`, and the other skills in chat as usual.

Skills need a real binary on `PATH` (`cargo make specify` is the lab/mock shim, not this):

```bash
cargo install --path . --locked
specify --version
```

Reinstall when you change the CLI. Guest verbs that call the model also need `cursor-agent` on `PATH` and logged in.

| You're changing…                 | Do this                                                                         |
| -------------------------------- | ------------------------------------------------------------------------------- |
| `plugins/spec/skills/*/SKILL.md` | `cursor-agent --plugin-dir plugins/spec` (no rebuild)                           |
| Rust CLI / guest orchestrations  | `cargo install --path . --locked`, then use skills or call `specify …` directly |
| Shape/authoring invariants only  | `cargo test -p checks` (no agent needed)                                        |

For a dry run without Cursor, call the same verbs the skills wrap (`specify init`, `specify plan author`, …) after install.

## Publishing a new Cursor plugin

1. Add the plugin under `plugins/<name>/` and list it in `marketplace.json`.
2. Merge to `main`.
3. In the Cursor plugin marketplace dashboard: refresh, set the new plugin to **Required**, save.
4. Restart Cursor locally.

## Related

- [Skill / CLI responsibility split](../../AGENTS.md#skill--cli-responsibility-split) — wrappers stay ultrathin
- [Consistency Checks](checks.md) — skill and marketplace predicates
- [Adapter anatomy](../explanation/adapter-anatomy.md) — Cursor `plugin.json` vs adapter components
