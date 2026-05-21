# Omnia reference material

Reference documentation for the Omnia target adapter at [`targets/omnia/`](../../targets/omnia/). In Specify 2.0 (RFC-25) Omnia is a **target adapter** — `shape`, `build`, `merge` — not a slash-command plugin. The bodies of the retired `omnia-crate-writer`, `omnia-test-writer`, `omnia-guest-writer`, and `omnia-code-reviewer` skills now live inside [`targets/omnia/briefs/build.md`](../../targets/omnia/briefs/build.md); the briefs reference the prose in this folder.

## Briefs

| Brief | Purpose |
|-------|---------|
| [`shape.md`](../../targets/omnia/briefs/shape.md) | Idiom guidance (provider DI, WASM guardrails, error variants, validation placement) consumed by core synthesis. |
| [`build.md`](../../targets/omnia/briefs/build.md) | Crate / test / guest generation plus code review, run by `/spec:build`. |
| [`merge.md`](../../targets/omnia/briefs/merge.md) | Pre-merge gate (cargo + clippy + test + wasm32 build) run by `/spec:merge`. |

## References

- [Guardrails](references/guardrails.md) — forbidden crates, std APIs, WASM constraints, serde / timestamp / DST idioms.
- [Capabilities](references/capabilities.md) — provider trait signatures and adapter triggers.
- [Guest patterns](references/guest-patterns.md) — HTTP / Messaging / WebSocket guest export patterns.
- [Guest wiring](references/guest-wiring.md) — crate → guest injection contract.
- [Providers](references/providers/) — per-trait deep dives (blobstore, broadcast, config, document-store, http-request, identity, publish, state-store).
- [Runtime](references/runtime.md) — `omnia::runtime!` macro, WASI host options, `.env.example` shape.
- [Agent teams](references/agent-teams.md) — multi-agent review pattern used by the build brief's code-review pass.
- [Codex](references/codex/) — stable codex rules (`OMNIA-001`, `OMNIA-002`, `RUST-001`, `SEC-001`).
