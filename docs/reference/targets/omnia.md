# Omnia Adapter

- **Identifier:** `omnia` (bundled, first-party)
- **URL:** `https://github.com/augentic/specify/adapters/targets/omnia`
- **Purpose:** Rust WASM development (greenfield or migration)
- **Target:** Rust WASM (Omnia SDK)

## Brief pipeline

### Define phase

| Brief | Output | Dependencies |
|-------|--------|-------------|
| `proposal.md` | `proposal.md` | -- |
| `specs.md` | `specs/<adapter>/spec.md` | proposal |
| `design.md` | `design.md` | proposal, specs |
| `tasks.md` | `tasks.md` | specs, design |

When a plan entry has `sources`, core synthesis reads `Evidence[]` from each bound source (e.g. `code-typescript`) and fuses claims into `spec.md` requirements with `Sources:` provenance lines.

The specs and design briefs read baseline contracts at `contracts/` as read-only context. Implementation changes conform to existing contracts; new or changed interface shapes should be introduced through a dedicated `contracts@v1` change before implementation depends on them. The contracts target adapter owns author/import/verify behavior through the format sub-flows in [`adapters/targets/contracts/briefs/build.md`](../../../adapters/targets/contracts/briefs/build.md).

### Build phase

The build brief drives implementation work directly through phase sub-briefs — there are no separate slash-command skills. The build orchestrator is [`adapters/targets/omnia/briefs/build.md`](../../../adapters/targets/omnia/briefs/build.md); the per-phase sub-briefs live under [`adapters/targets/omnia/briefs/build/`](../../../adapters/targets/omnia/briefs/build/):

| Sub-brief | Purpose |
|-----------|---------|
| [`build/crate.md`](../../../adapters/targets/omnia/briefs/build/crate.md) | Generate or update the Rust crate (provider DI, handler delegation, error variants). |
| [`build/test.md`](../../../adapters/targets/omnia/briefs/build/test.md) | Generate or update the test suite (MockProvider patterns, scenario-to-test mapping). |
| [`build/guest.md`](../../../adapters/targets/omnia/briefs/build/guest.md) | Scaffold the WASM guest wrapper (HTTP, messaging, WebSocket; create mode only). |
| [`build/review.md`](../../../adapters/targets/omnia/briefs/build/review.md) | Agent-team code review (security, correctness, quality, antagonist) and remediation. |

The build brief reads `tasks.md` and walks the phases in order. The typical build order is: crate implementation, test generation, guest wiring (create mode), code review.

### Merge phase

| Brief | Skills invoked |
|-------|---------------|
| `merge.md` | -- (drives git operations directly; runs `cargo check`, `cargo clippy`, `cargo test`, and `wasm32-wasip2` build via [`adapters/targets/omnia/briefs/merge.md`](../../../adapters/targets/omnia/briefs/merge.md)) |

## Reference material

Hard rules, capability/provider documentation, SDK templates, mock-provider patterns, review categories, and worked examples live under [`adapters/targets/omnia/references/`](../../../adapters/targets/omnia/references/) — see the [`README`](../../../adapters/targets/omnia/references/README.md) for the full index.

## Domain context

The Omnia adapter's briefs and references carry domain context about:

- Omnia SDK patterns (provider traits, side-effect abstractions).
- WASM constraints (no filesystem, no threading).
- Guest wiring conventions (HTTP handlers, message subscribers, WebSocket events).
- Testing patterns (MockProvider, Client-based integration tests).

## Project configuration

After `/spec:init omnia`, `project.yaml` carries:

```yaml
target: https://github.com/augentic/specify/adapters/targets/omnia
rules:
  - "Project-specific constraints go here"
```
