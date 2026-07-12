# Plugins

Specify ships as a Cursor plugin marketplace. Each plugin provides specialist skills and reference documentation for a specific domain.

## Plugin model

Plugins are installed from the Cursor marketplace (Settings > Plugins > search for "Augentic"). Each plugin bundles:

- **Skills** — agent-driven orchestrators invoked with a slash-command prefix (e.g. `/spec:plan`).
- **Rules** — `.mdc` files that provide context to the agent.
- **References** — markdown documents that skills read for domain knowledge.

This repository is the Cursor-shaped distribution of the skills. The plugin manifests, slash-command routing, and `<!-- skill: plugin:skill -->` delegation directives are Cursor-specific.

## Workspace rules

Installing plugins from the marketplace gives you each plugin's rules and skills. For cross-plugin coordination, copy the workspace rule from the Specify repository (`.cursor/rules/project.mdc`) into your project's `.cursor/rules/` directory.

## Plugin overview

| Plugin       | Prefix      | Purpose                                                                                                            | Reference            |
| ------------ | ----------- | ------------------------------------------------------------------------------------------------------------------ | -------------------- |
| **Specify**  | `/spec:`    | Workflow orchestration: `init`, `plan`, `refine`, `execute`, `build`, `merge`, `finalize`, `drop`.                  | [Slice Skills](../slice-skills/index.md) |
| **Capture**  | `/capture:` | Runtime capture for legacy TypeScript migration workflows                                                          | [Capture](../../../plugins/capture/README.md) |
| **Client**   | `/client:`  | Client-facing deliverables (SoW, proposals, pricing)                                                               | [Client](client.md)  |

The Omnia and Vectis target adapters are not Cursor plugins — they live under [`adapters/targets/`](../targets/index.md) and contribute their `guidance`, `build`, and `merge` operations to the workflow. See [Omnia target](../targets/omnia.md) and [Vectis target](../targets/vectis.md).

## How plugins compose with target adapters

The **Specify** plugin provides the workflow skeleton. **Target adapters** (under `adapters/targets/<name>/`) own the `guidance`, `build`, and `merge` operations — the build prompts drive implementation work directly:

- **Omnia target** drives crate, test, guest, and review phases inline from [`adapters/targets/omnia/prose/prompts/build.md`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/prompts/build.md).
- **Vectis target** drives composition, core, iOS, and Android phases inline from [`adapters/targets/vectis/prose/prompts/build.md`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/build.md).
- **Contracts target** runs OpenAPI, AsyncAPI, and JSON Schema sub-flows inside [`adapters/targets/contracts/prose/prompts/build.md`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/prose/prompts/build.md).

The Capture and Client plugins are target-independent — Capture supports legacy runtime capture regardless of the target platform; Client supports operator-facing deliverables.

## Artifact flow

```text
/spec:plan     →  surveys each bound source, proposes slices[]
/spec:refine   →  extracts evidence per source, synthesizes proposal/spec/design/tasks via core
/spec:build    →  drives the target adapter's build operation (Omnia, Vectis, Contracts, ...)
/spec:merge    →  applies deltas to baseline (target-agnostic)
```

Target-adapter build prompts read the artifacts produced by `/spec:refine` and generate code. The artifacts are the interface between core synthesis and the target adapter.
