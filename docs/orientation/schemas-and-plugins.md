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

Schema URLs support an optional `@ref` suffix to pin a version (e.g. `omnia@v1`).

### What schemas share

Both schemas use the same four-blueprint dependency order:

1. **proposal** -- initial proposal document
2. **specs** -- behavioral specifications (requires proposal)
3. **design** -- technical design (requires proposal)
4. **tasks** -- implementation checklist (requires specs + design)

The artifact structure and lifecycle are schema-agnostic. Schemas fill in the brief *content* within each phase, not the phase structure itself.

## Plugins

Specify ships as a Cursor plugin marketplace containing five plugins. Each plugin provides specialist skills and reference documentation.

| Plugin | Prefix | Purpose |
|--------|--------|---------|
| **Specify** | `/spec:` | Core workflow: define, build, merge, verify, explore, extract, plan, execute |
| **Omnia** | `/omnia:` | Rust WASM crate generation, testing, and review |
| **Vectis** | `/vectis:` | Cross-platform Crux app generation (Rust core, iOS/Android shells, design system) |
| **RT** | `/rt:` | Repository cloning, fixture capture, regression testing for migration |
| **Plan** | `/plan:` | Statement of Work generation |

### How schemas and plugins compose

The **Specify** plugin provides the workflow skeleton -- the `/spec:*` skills that every project uses. The schema determines which **specialist** plugin skills are invoked during the build phase.

For example, with the Omnia schema:

```text
/spec:define --> generates artifacts using Omnia brief pipelines
/spec:build  --> delegates tasks to /omnia:crate-writer, /omnia:test-writer, etc.
/spec:merge  --> merges specs into baseline (schema-agnostic)
```

With the Vectis schema:

```text
/spec:define --> generates artifacts using Vectis brief pipelines
/spec:build  --> delegates tasks to /vectis:core-writer, /vectis:ios-writer, etc.
/spec:merge  --> merges specs into baseline (schema-agnostic)
```

The define-build-merge loop is invariant. Swapping the schema swaps the brief content and the specialist skills -- nothing else changes.

### Workspace rules

Installing Augentic plugins from the Cursor marketplace gives you each plugin's rules and skills. For cross-plugin coordination, you can copy the workspace rule from the Specify repository (`.cursor/rules/project.mdc`) into your project's `.cursor/rules/` directory.

For more detail on each plugin's skills, see the [Plugins](../reference/plugins/index.md) reference.
