# Omnia reference material

Reference documentation for the Omnia target adapter at [`targets/omnia/`](../../targets/omnia/). In Specify 2.0 (RFC-25) Omnia is a **target adapter** — `shape`, `build`, `merge` — not a slash-command plugin.

The orchestration of the retired `omnia-crate-writer`, `omnia-test-writer`, `omnia-guest-writer`, and `omnia-code-reviewer` skills now lives in [`targets/omnia/briefs/build.md`](../../targets/omnia/briefs/build.md) and four phase sub-briefs under [`targets/omnia/briefs/build/`](../../targets/omnia/briefs/build/). The depth (templates, hard rules, mapping tables, mock-provider patterns, specialist prompts, codex rules) and worked examples live in this folder.

## Briefs

| Brief | Purpose |
|-------|---------|
| [`shape.md`](../../targets/omnia/briefs/shape.md) | Idiom guidance (provider DI, WASM guardrails, error variants, validation placement) consumed by core synthesis. |
| [`build.md`](../../targets/omnia/briefs/build.md) | Orchestrator: bindings, mode detection, phase order, verify-repair loop, stop-hint contract. |
| [`build/crate.md`](../../targets/omnia/briefs/build/crate.md) | Phase 2: generate or update the Rust crate. |
| [`build/test.md`](../../targets/omnia/briefs/build/test.md) | Phase 3: generate or update the test suite. |
| [`build/guest.md`](../../targets/omnia/briefs/build/guest.md) | Phase 4 (create mode only): scaffold the WASM guest wrapper. |
| [`build/review.md`](../../targets/omnia/briefs/build/review.md) | Phase 6: agent-team code review and remediation cycle. |
| [`merge.md`](../../targets/omnia/briefs/merge.md) | Pre-merge gate (cargo + clippy + test + wasm32 build) run by `/spec:merge`. |

## References

### Authority and hard constraints

- [`hard-rules.md`](references/hard-rules.md) — full hard-rules set and authority hierarchy.
- [`guardrails.md`](references/guardrails.md) — forbidden crates, std APIs, WASM constraints, serde / timestamp / DST idioms.
- [`wasm-constraints.md`](references/wasm-constraints.md) — translating `[runtime]` constraints to Omnia/WASM patterns.

### Capabilities and providers

- [`capabilities.md`](references/capabilities.md) — provider trait signatures and adapter triggers (all nine providers).
- [`capability-mapping.md`](references/capability-mapping.md) — mapping from Specify artifact adapters to Omnia provider traits.
- [`providers/`](references/providers/) — per-trait deep dives (blobstore, broadcast, config, document-store, http-request, identity, publish, state-store).

### Crate writer depth

- [`sdk-api.md`](references/sdk-api.md) — `Handler<P>`, `Context`, `Reply`, `IntoBody`, `Client`, `Error`; Input Type Decision Tree; Response Types.
- [`cargo-toml.md`](references/cargo-toml.md) — workspace and crate `Cargo.toml` templates.
- [`error-handling.md`](references/error-handling.md) — domain error enums, `omnia_sdk::Error` mapping, troubleshooting.
- [`cross-cutting-matrices.md`](references/cross-cutting-matrices.md) — Side-Effect / Outbound-Message / Transaction-Boundary matrices.
- [`update-patterns.md`](references/update-patterns.md) — strategy patterns per update category.
- [`change-classification.md`](references/change-classification.md) — classifying artifact-vs-code diffs.
- [`repair-patterns.md`](references/repair-patterns.md) — common verify-loop repair recipes.
- [`checklists.md`](references/checklists.md) — pre-generation and verification checklists.
- [`todo-markers.md`](references/todo-markers.md) — TODO marker rules, adapter overrides, cache-aside patterns.
- [`output-documents.md`](references/output-documents.md) — `Migration.md`, `Architecture.md`, `CHANGELOG.md`, `.env.example` shapes.

### Test writer depth

- [`mock-provider.md`](references/mock-provider.md) — Static and Replay MockProvider patterns per provider trait.
- [`spec-to-test-mapping.md`](references/spec-to-test-mapping.md) — how spec scenarios map to test functions; `REQ-XXX` traceability.

### Guest writer depth

- [`configuration.md`](references/configuration.md) — guest workspace `Cargo.toml`, `.cargo/config.toml`, `deny.toml`, the five GitHub workflows, `.env.example` shape (templates).
- [`handlers.md`](references/handlers.md) — HTTP routing, message subscriptions, WebSocket events, `lib.rs` wiring patterns.
- [`guest-patterns.md`](references/guest-patterns.md) — HTTP / Messaging / WebSocket guest export patterns.
- [`guest-wiring.md`](references/guest-wiring.md) — crate → guest injection contract.
- [`runtime.md`](references/runtime.md) — `omnia::runtime!` macro, WASI host options, `.env.example` shape.
- [`project-layout.md`](references/project-layout.md) — directory layout for the guest project.

### Code reviewer depth

- [`review-categories.md`](references/review-categories.md) — full SEC/COR/QUA/UNI check library and codex `rule_id` mapping.
- [`review-team-protocol.md`](references/review-team-protocol.md) — verbatim specialist spawn prompts, antagonist protocol, synthesis rules.
- [`review-auto-fix.md`](references/review-auto-fix.md) — `fix` scope, per-category success-rate table, regression guard.
- [`review-output-template.md`](references/review-output-template.md) — `REVIEW.md` template and finding-ID conventions.
- [`agent-teams.md`](references/agent-teams.md) — shared multi-agent review pattern (specialists + antagonist + lead synthesis).
- [`codex/`](references/codex/) — stable codex rules (`OMNIA-001`, `OMNIA-002`, `RUST-001`, `SEC-001`).

### Worked examples

- [`examples/crates/`](references/examples/crates/) — single-handler, multi-handler, anti-patterns; per-capability walkthroughs under `capabilities/`; per-update-category walkthroughs under `updates/`.
- [`examples/tests/`](references/examples/tests/) — per-provider testing patterns (HTTP, StateStore, Publisher, Blobstore, DocumentStore).
