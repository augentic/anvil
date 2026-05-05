# Capabilities and Plugins

Specify's core workflow -- define, build, merge -- is the same regardless of what you are building. **Capabilities** and **plugins** determine *how* that workflow executes for your particular technology stack.

## Capabilities

A capability is a versioned Specify extension that tells the core how to generate artifacts and build code for a specific outcome domain. When you run `/spec:init`, you provide a capability identifier (a bare name, an `https://…` URL, or a `file:///…` URI) that configures the project.

Each capability declares:

- **Brief pipelines** for each phase (define, build, merge) -- the ordered sequence of prompts that generate artifacts.
- **Domain context** (carried in the briefs and skills) -- terminology, constraints, and patterns specific to the target outcome domain.
- **Specialist skill references** -- which plugin skills the build phase should delegate to.

### Available capabilities

| Capability | Identifier or URL | Target | Use case |
|------------|-------------------|--------|----------|
| Omnia | `https://github.com/augentic/specify/capabilities/omnia` | Rust WASM (Omnia SDK) | Greenfield or migration to Omnia services |
| Vectis | `https://github.com/augentic/specify/capabilities/vectis` | Rust + Swift + Kotlin (Crux) | Cross-platform mobile/desktop applications |
| Contracts | `https://github.com/augentic/specify/capabilities/contracts` | JSON Schema / OpenAPI / AsyncAPI | Dedicated API contract changes |

Capability identifiers support an optional `@ref` suffix to pin a version (e.g. `omnia@v1`).

### Implementation capability artifact chain

Omnia and Vectis share the same core implementation artifact chain:

1. **proposal** -- initial proposal document
2. **specs** -- behavioral specifications (requires proposal)
3. **design** -- technical design (requires proposal + specs)
4. **tasks** -- implementation checklist (requires specs + design)

The specs and design briefs read any baseline contracts at `contracts/` as context. Implementation changes conform to those baseline contracts; new or changed interface shapes are introduced through dedicated `contracts@v1` changes instead of being derived inline.

The Vectis capability extends this with a **composition** stage between specs and design that produces `composition.yaml` -- a structured YAML artifact describing the spatial layout of each screen (regions, groups, items, bindings, and event wiring). The composition brief requires specs and proposal as inputs, and the design brief reads the composition artifact to adopt the screen names and field names it proposes.

The Contracts capability is different: define captures proposal, specs, and build tasks, while `/spec:build` authors or imports the contract artifacts and then verifies them.

The artifact structure and lifecycle are capability-agnostic. Capabilities fill in the brief *content* within each phase and may add capability-specific stages to the pipeline.

## Plugins

Specify ships as a Cursor plugin marketplace containing six plugins. Each plugin provides specialist skills and reference documentation.

| Plugin | Prefix | Purpose |
|--------|--------|---------|
| **Specify** | `/spec:` | Core workflow: define, build, merge, verify, explore, extract, plan, execute |
| **Omnia** | `/omnia:` | Rust WASM crate generation, testing, and review |
| **Vectis** | `/vectis:` | Cross-platform Crux app generation (Rust core, iOS/Android shells, design system) |
| **Contract** | `/contract:` | API contract generation, validation, and import (OpenAPI, AsyncAPI, JSON Schema) |
| **RT** | `/rt:` | Fixture capture and regression testing for migration |
| **Client** | `/client:` | Client-facing deliverables (SoW, proposals, pricing) |

### How capabilities and plugins compose

The **Specify** plugin provides the workflow skeleton -- the `/spec:*` skills that every project uses. The capability determines which **specialist** plugin skills are invoked during the build phase.

For example, with the Omnia capability:

```text
/spec:define --> generates artifacts using Omnia brief pipelines
/spec:build  --> delegates tasks to /omnia:crate-writer, /omnia:test-writer, etc.
/spec:merge  --> merges specs into baseline (capability-agnostic)
```

With the Vectis capability:

```text
/spec:define --> generates artifacts using Vectis brief pipelines (incl. composition.yaml)
/spec:build  --> delegates tasks to /vectis:core-writer, /vectis:ios-writer, etc.
/spec:merge  --> merges specs + composition into baseline
```

The define-build-merge loop is invariant. Swapping the capability swaps the brief content and the specialist skills -- nothing else changes.

### Workspace rules

Installing Augentic plugins from the Cursor marketplace gives you each plugin's rules and skills. For cross-plugin coordination, you can copy the workspace rule from the Specify repository (`.cursor/rules/project.mdc`) into your project's `.cursor/rules/` directory.

For more detail on each plugin's skills, see the [Plugins](../reference/plugins/index.md) reference.
