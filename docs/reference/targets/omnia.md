# Omnia Adapter

- **Identifier:** `omnia` (bundled, first-party)
- **URL:** `https://github.com/augentic/specify/adapters/targets/omnia`
- **Purpose:** Rust WASM development (greenfield or migration)
- **Target:** Rust WASM (Omnia SDK)

## Operations

The Omnia target declares exactly three operations — `shape`, `build`, `merge` — matching its [`adapter.yaml`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/adapter.yaml). Core `/spec:refine` synthesises the canonical artifacts (`proposal.md` / `spec.md` / `design.md` / `tasks.md`); the target adapter never writes them.

### shape

`shape` is idiom guidance read into context when core synthesis writes `proposal.md`, `spec.md`, and `design.md` for a `target: omnia` slice — see [`adapters/targets/omnia/prose/briefs/shape.md`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/briefs/shape.md). The brief is input to synthesis: it does not read sources or write artifacts. It carries the Omnia idioms the synthesiser must fold into the canonical artifacts — provider-based dependency injection (the closed provider-trait set), `wasm32-wasip2` guardrails (forbidden crates / std APIs, statelessness), `omnia_sdk::Error` variant conventions, edge-vs-core validation placement, and the required `design.md` heading order (domain model, provider trait dependencies, handler delegation, external surfaces, configuration, error mapping, validation placement, observability).

When a plan entry has `sources`, core synthesis reads `Evidence[]` from each bound source (e.g. `typescript`) and reconciles claims into `spec.md` requirements with `Sources:` provenance lines. The same `shape` guidance applies whether the slice's evidence is pure intent, documentation, or code.

The synthesis briefs treat baseline contracts at `contracts/` as read-only context. Implementation changes conform to existing contracts; new or changed interface shapes should be introduced through a dedicated `contracts@1.0.0` change before implementation depends on them. The contracts target adapter owns author/import/verify behavior through the format sub-flows in [`adapters/targets/contracts/prose/briefs/build.md`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/prose/briefs/build.md).

### build

The build brief drives implementation work directly through phase sub-briefs — there are no separate slash-command skills. The build orchestrator is [`adapters/targets/omnia/prose/briefs/build.md`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/briefs/build.md); the per-phase sub-briefs live under [`adapters/targets/omnia/prose/briefs/build/`](https://github.com/augentic/specify-adapters/tree/main/targets/omnia/prose/briefs/build/):

| Sub-brief | Purpose |
|-----------|---------|
| [`build/crate.md`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/briefs/build/crate.md) | Generate or update the Rust crate (provider DI, handler delegation, error variants). |
| [`build/test.md`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/briefs/build/test.md) | Generate or update the test suite (MockProvider patterns, scenario-to-test mapping). |
| [`build/guest.md`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/briefs/build/guest.md) | Scaffold the WASM guest wrapper (HTTP, messaging, WebSocket; create mode only). |
| [`build/review.md`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/briefs/build/review.md) | Agent-team code review (security, correctness, quality, antagonist) and remediation. |

The build brief reads `tasks.md` and walks the phases in order. The typical build order is: crate implementation, test generation, guest wiring (create mode), code review. `build` writes its outcome to `build/report.yaml`; the build orchestration's finalize tail owns the `built` transition.

### merge

The merge brief lands the built slice through `specify slice merge` per the shared [`/spec:merge`](../../../plugins/spec/skills/merge/SKILL.md) skill body — see [`adapters/targets/omnia/prose/briefs/merge.md`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/briefs/merge.md). Omnia adds no adapter-specific adoption mechanics on top of the standard delta merge; instead it enforces an Omnia-specific *pre-merge* gate before invoking the CLI: `cargo fmt --check`, `cargo check --workspace`, `cargo clippy -- -D warnings`, `cargo test`, and the definitive `cargo build --target wasm32-wasip2 --release`. `specify slice merge` is the writer of the `merged` lifecycle transition and the archive move.

## Reference material

Hard rules, capability/provider documentation, SDK templates, mock-provider patterns, review categories, and worked examples live under [`adapters/targets/omnia/prose/references/`](https://github.com/augentic/specify-adapters/tree/main/targets/omnia/prose/references/) — see the [`README`](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/prose/references/README.md) for the full index.

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
