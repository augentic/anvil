# Plugins

Emery ships a small Cursor plugin marketplace so operators can invoke workflow phases as slash commands. Each plugin bundles ultrathin skill wrappers, optional rules, and (when needed) plugin-local references.

The product is still the `emery` runtime and the adapters in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters). Skills do not orchestrate, synthesize, or generate code — they invoke CLI verbs and relay output. See [AGENTS.md § Skill / CLI responsibility split](../../../AGENTS.md#skill--cli-responsibility-split).

## Plugin model

Plugins are installed from the Cursor marketplace (Settings > Plugins > search for "Augentic"). Each plugin may bundle:

- **Skills** — slash-command wrappers (e.g. `/emery:plan`) that invoke one `emery` verb.
- **Rules** — `.mdc` files that provide context to the agent.
- **References** — markdown documents that skills may link for operator-facing depth (plugin-local only; never into `docs/`).

## Project guidance

Installing plugins from the marketplace gives you each plugin's rules and skills. Keep project-wide and cross-plugin guidance in the repository's root `AGENTS.md`; `emery init` generates the Emery context there when the file is absent. Use plugin rules only for plugin-specific behavior.

## Plugin overview

| Plugin | Prefix | Purpose | Reference |
| ------ | ------ | ------- | --------- |
| **Emery** | `/emery:` | Workflow wrappers: `init`, `plan`, `execute`, `refine`, `build`, `merge`, `finalize`, `drop` | [Slice Skills](../slice-skills/index.md), [Change Skills](../change-skills/index.md) |

The Omnia and Vectis target adapters are not Cursor plugins — they live under [`targets/` in the adapters repo](../targets/index.md) and contribute their `guidance`, `build`, and `merge` operations to the workflow. See [Omnia target](../targets/omnia.md) and [Vectis target](../targets/vectis.md).

## How plugins compose with target adapters

The **Emery** plugin provides the operator slash commands. **Target adapters** own `guidance`, `build`, and `merge` — the build prompts drive implementation work directly:

- **Omnia target** drives crate, test, guest, and review phases inline from [`targets/omnia/prose/prompts/build.md`](https://github.com/augentic/emery-adapters/blob/main/targets/omnia/prose/prompts/build.md).
- **Vectis target** drives composition, core, iOS, and Android phases inline from [`targets/vectis/prose/prompts/build.md`](https://github.com/augentic/emery-adapters/blob/main/targets/vectis/prose/prompts/build.md).
- **Contracts target** runs OpenAPI, AsyncAPI, and JSON Schema sub-flows inside [`targets/contracts/prose/prompts/build.md`](https://github.com/augentic/emery-adapters/blob/main/targets/contracts/prose/prompts/build.md).

## Artifact flow

```text
/emery:plan     →  surveys each bound source, proposes slices[]
/emery:refine   →  extracts evidence per source, synthesizes proposal/spec/design/tasks via core
/emery:build    →  drives the target adapter's build operation (Omnia, Vectis, Contracts, ...)
/emery:merge    →  applies deltas to baseline (target-agnostic)
```

Target-adapter build prompts read the artifacts produced by `/emery:refine` and generate code. The artifacts are the interface between core synthesis and the target adapter.

Contributor notes for the marketplace layout and `--plugin-dir` preview: [Cursor operator plugins](../../contributing/operator-plugins.md).
