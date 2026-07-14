# Plugins

Specify ships a small Cursor plugin marketplace so operators can invoke workflow phases as slash commands. Each plugin bundles ultrathin skill wrappers, optional rules, and (when needed) plugin-local references.

The product is still the `specify` runtime and the adapters in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters). Skills do not orchestrate, synthesize, or generate code — they invoke CLI verbs and relay output. See [AGENTS.md § Skill / CLI responsibility split](../../../AGENTS.md#skill--cli-responsibility-split).

## Plugin model

Plugins are installed from the Cursor marketplace (Settings > Plugins > search for "Augentic"). Each plugin may bundle:

- **Skills** — slash-command wrappers (e.g. `/spec:plan`) that invoke one `specify` verb.
- **Rules** — `.mdc` files that provide context to the agent.
- **References** — markdown documents that skills may link for operator-facing depth (plugin-local only; never into `docs/`).

## Project guidance

Installing plugins from the marketplace gives you each plugin's rules and skills. Keep project-wide and cross-plugin guidance in the repository's root `AGENTS.md`; `specify init` generates the Specify context there when the file is absent. Use plugin rules only for plugin-specific behavior.

## Plugin overview

| Plugin | Prefix | Purpose | Reference |
| ------ | ------ | ------- | --------- |
| **Specify** | `/spec:` | Workflow wrappers: `init`, `plan`, `refine`, `build`, `merge`, `finalize`, `drop` | [Slice Skills](../slice-skills/index.md), [Change Skills](../change-skills/index.md) |
| **Capture** | `/capture:` | Runtime capture for legacy TypeScript migration workflows | [Capture](../../../plugins/capture/README.md) |

The Omnia and Vectis target adapters are not Cursor plugins — they live under [`adapters/targets/`](../targets/index.md) and contribute their `guidance`, `build`, and `merge` operations to the workflow. See [Omnia target](../targets/omnia.md) and [Vectis target](../targets/vectis.md).

## How plugins compose with target adapters

The **Specify** plugin provides the operator slash commands. **Target adapters** own `guidance`, `build`, and `merge` — the build prompts drive implementation work directly:

- **Omnia target** drives crate, test, guest, and review phases inline from [`adapters/targets/omnia/prose/prompts/build.md`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/prompts/build.md).
- **Vectis target** drives composition, core, iOS, and Android phases inline from [`adapters/targets/vectis/prose/prompts/build.md`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/build.md).
- **Contracts target** runs OpenAPI, AsyncAPI, and JSON Schema sub-flows inside [`adapters/targets/contracts/prose/prompts/build.md`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/prose/prompts/build.md).

The Capture plugin is target-independent — it supports legacy runtime capture regardless of the target platform.

## Artifact flow

```text
/spec:plan     →  surveys each bound source, proposes slices[]
/spec:refine   →  extracts evidence per source, synthesizes proposal/spec/design/tasks via core
/spec:build    →  drives the target adapter's build operation (Omnia, Vectis, Contracts, ...)
/spec:merge    →  applies deltas to baseline (target-agnostic)
```

Target-adapter build prompts read the artifacts produced by `/spec:refine` and generate code. The artifacts are the interface between core synthesis and the target adapter.

Contributor notes for the marketplace layout and `--plugin-dir` preview: [Cursor operator plugins](../../contributing/operator-plugins.md).
