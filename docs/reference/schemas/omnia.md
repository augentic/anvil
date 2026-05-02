# Omnia Schema

- **URL:** `https://github.com/augentic/specify/schemas/omnia`
- **Purpose:** Rust WASM development (greenfield or migration)
- **Target:** Rust WASM (Omnia SDK)

## Brief pipeline

### Define phase

| Brief | Output | Dependencies |
|-------|--------|-------------|
| `proposal.md` | `proposal.md` | -- |
| `specs.md` | `specs/<capability>/spec.md` | proposal |
| `design.md` | `design.md` | proposal, specs |
| `tasks.md` | `tasks.md` | specs, design |

When a plan entry has `sources`, the specs brief invokes `/spec:extract` to derive requirements from legacy code.

The specs and design briefs read baseline contracts at `.specify/contracts/` as read-only context. Implementation changes conform to existing contracts; new or changed interface shapes should be introduced through a dedicated `contracts@v1` change before implementation depends on them. See [Contract Plugin](../plugins/contract.md) for skill details.

### Build phase

| Brief | Skills invoked |
|-------|---------------|
| `build.md` | `/omnia:guest-writer`, `/omnia:crate-writer`, `/omnia:test-writer`, `/omnia:code-reviewer` |

The build brief reads `tasks.md` and delegates to specialist skills based on skill directive tags. The typical build order is: guest wiring, crate implementation, test generation, code review.

### Merge phase

| Brief | Skills invoked |
|-------|---------------|
| `merge.md` | -- (drives git operations directly) |

## Specialist skills

| Skill | Purpose |
|-------|---------|
| `/omnia:crate-writer` | Generate or update Rust crates with provider-based DI |
| `/omnia:test-writer` | Generate test suites with MockProvider pattern |
| `/omnia:guest-writer` | Generate WASM guest wrapper (HTTP, messaging, WebSocket) |
| `/omnia:code-reviewer` | Agent team review (structural, logic, quality + antagonist) |

See [Omnia Plugin](../plugins/omnia.md) for full skill documentation.

## Domain context

The Omnia schema injects domain context about:

- Omnia SDK patterns (provider traits, side-effect abstractions).
- WASM constraints (no filesystem, no threading).
- Guest wiring conventions (HTTP handlers, message subscribers, WebSocket events).
- Testing patterns (MockProvider, Client-based integration tests).

## Project configuration

After `/spec:init` with the Omnia schema, `project.yaml` can include:

```yaml
schema: https://github.com/augentic/specify/schemas/omnia
domain: |
  Describe your service's domain, purpose, and constraints here.
  This context is available to all briefs during artifact generation.
rules:
  - "Project-specific constraints go here"
```
