# Schemas and Plugins

Specify's core workflow -- define, build, merge -- is the same regardless of what you are building. **Schemas** and **plugins** determine *how* that workflow executes for your particular technology stack.

## Schemas

A schema is a configuration package that tells Specify how to generate artifacts and build code for a specific target platform. When you run `/spec:init`, you provide a schema URL that configures the project.

Each schema declares:

- **Brief pipelines** for each phase (define, build, merge) -- the ordered sequence of prompts that generate artifacts.
- **Domain context** -- terminology, constraints, and patterns specific to the target platform.
- **Specialist skill references** -- which plugin skills the build phase should delegate to.

### Available schemas

| Schema | URL | Target | Use case |
|--------|-----|--------|----------|
| Omnia | `https://github.com/augentic/specify/schemas/omnia` | Rust WASM (Omnia SDK) | Greenfield or migration to Omnia services |
| Vectis | `https://github.com/augentic/specify/schemas/vectis` | Rust + Swift + Kotlin (Crux) | Cross-platform mobile/desktop applications |
| Contracts | `https://github.com/augentic/specify/schemas/contracts` | JSON Schema / OpenAPI / AsyncAPI | Dedicated API contract changes |

Schema URLs support an optional `@ref` suffix to pin a version (e.g. `omnia@v1`).

### What schemas share

Both schemas share the same core five-artifact dependency chain:

1. **proposal** -- initial proposal document
2. **specs** -- behavioral specifications (requires proposal)
3. **contracts** -- API contract alignment validation (requires specs)
4. **design** -- technical design (requires proposal, contracts)
5. **tasks** -- implementation checklist (requires specs + design)

The `contracts` stage validates that the change's specs align with any baseline contracts at `.specify/contracts/` and produces a minimal delta for uncovered interactions. When no baseline contracts exist, it derives interface shapes from the specs. When the change describes no API interactions, it completes as a no-op.

The Vectis schema extends this with a **composition** stage between contracts and design that produces `composition.yaml` -- a structured YAML artifact describing the spatial layout of each screen (regions, groups, items, bindings, and event wiring). The composition brief requires specs and proposal as inputs, and the design brief reads the composition artifact to adopt the screen names and field names it proposes.

The artifact structure and lifecycle are schema-agnostic. Schemas fill in the brief *content* within each phase and may add schema-specific stages to the pipeline.

## Plugins

Specify ships as a Cursor plugin marketplace containing six plugins. Each plugin provides specialist skills and reference documentation.

| Plugin | Prefix | Purpose |
|--------|--------|---------|
| **Specify** | `/spec:` | Core workflow: define, build, merge, verify, explore, extract, plan, execute |
| **Omnia** | `/omnia:` | Rust WASM crate generation, testing, and review |
| **Vectis** | `/vectis:` | Cross-platform Crux app generation (Rust core, iOS/Android shells, design system) |
| **Interfaces** | `/interfaces:` | API contract generation, validation, and import (OpenAPI, AsyncAPI, JSON Schema) |
| **RT** | `/rt:` | Fixture capture and regression testing for migration |
| **Client** | `/client:` | Client-facing deliverables (SoW, proposals, pricing) |

### How schemas and plugins compose

The **Specify** plugin provides the workflow skeleton -- the `/spec:*` skills that every project uses. The schema determines which **specialist** plugin skills are invoked during the build phase.

For example, with the Omnia schema:

```text
/spec:define --> generates artifacts using Omnia brief pipelines (incl. contracts alignment)
/spec:build  --> delegates tasks to /omnia:crate-writer, /omnia:test-writer, etc.
/spec:merge  --> merges specs + contracts into baseline (schema-agnostic)
```

With the Vectis schema:

```text
/spec:define --> generates artifacts using Vectis brief pipelines (incl. contracts, composition.yaml)
/spec:build  --> delegates tasks to /vectis:core-writer, /vectis:ios-writer, etc.
/spec:merge  --> merges specs + contracts + composition into baseline
```

The define-build-merge loop is invariant. Swapping the schema swaps the brief content and the specialist skills -- nothing else changes.

### Workspace rules

Installing Augentic plugins from the Cursor marketplace gives you each plugin's rules and skills. For cross-plugin coordination, you can copy the workspace rule from the Specify repository (`.cursor/rules/project.mdc`) into your project's `.cursor/rules/` directory.

For more detail on each plugin's skills, see the [Plugins](../reference/plugins/index.md) reference.
