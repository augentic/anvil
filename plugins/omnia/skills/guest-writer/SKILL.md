---
name: omnia-guest-writer
description: Generate a Rust project that exposes HTTP endpoints, subscribes to message topics, and handles WebSocket events in order to surface business logic via the Omnia WASI runtime. Use when scaffolding or updating the WASM guest wrapper after `crate-writer` has produced the crates it surfaces; not for writing the crates themselves or for non-Omnia projects.
---

# Guest Generator Skill

> **The guest is a thin WASI/wasm32 wrapper.** It owns HTTP routing, topic dispatch, WebSocket exports, provider setup, and config validation; ALL business logic lives in domain crates under `crates/`.

## Critical Path

1. **Read references first** — load handler, provider/runtime, project, and configuration references; they are specifications, not examples.
2. **Keep the guest thin** — route HTTP, messaging, WebSocket, config, and provider setup through the WASI boundary; keep all business logic in domain crates.
3. **Generate project structure** — create the root Cargo workspace, `.cargo/`, tooling configs, `src/lib.rs`, examples, CI workflows, and supply-chain files from the documented layout.
4. **Wire runtime surfaces** — implement Axum routes, topic dispatch, WebSocket exports, provider traits, owner-bearing handler calls, and config validation.
5. **Add local runtime and environment** — generate `examples/<guest>.rs` with `omnia::runtime!` and an `.env.example` covering every required config key.
6. **Generate workflow and compliance files** — add standard GitHub workflows, deny/vet files, and run the cargo-vet regeneration commands once lockfiles exist.
7. **Verify WASM constraints** — run the build/check loop, fix missing routes or provider impls, and enforce no `std::env`, `std::fs`, `std::net`, or business logic in guest code.

## Orientation

The guest project is a single Rust workspace whose `src/lib.rs` exports HTTP, Messaging, and (optionally) WebSocket Guest implementations. Each surface is wired by hand against a domain crate's exported handler types: HTTP routes use Axum 0.8 brace syntax (`{param}`), the messaging dispatcher matches topics explicitly and returns `Err` for unhandled ones, and WebSocket handlers are exported via `omnia_wasi_websocket::export!`. Every handler invocation goes through the builder API with an owner: `.provider(&p).owner("o").await`.

Provider traits implement only the WASI adapters the domain crates actually consume (Config plus any of Publish, Identity, StateStore, TableStore, Blobstore, DocumentStore, etc.). Workspace dependencies are derived from those same adapters; all `omnia-*` crates are published on crates.io, no private registry needed. Config keys are validated up front in `Provider::new()` and documented in `examples/.env.example`.

The skill also generates the project's standard tooling: `.cargo/config.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `Makefile.toml`, `.vscode/settings.json`, the five GitHub workflows (`audit`, `ci`, `patch`, `publish`, `release`), and the Cargo Deny / Cargo Vet supply-chain files. After the project is laid down, `cargo vet regenerate {imports,exemptions,unpublished}` populates workspace-specific data — the regeneration step requires a `Cargo.lock` to exist.

Idempotency: an existing `src/lib.rs` short-circuits generation. The verification loop runs `cargo check`, fixes any missing routes / provider impls / wasm32-incompatible usage, and re-checks until clean.

See [`references/runbook.md`](references/runbook.md) for the operational detail (full process steps, error-handling table, recovery flow, verification checklist).

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Overview, key principle, full Process steps 1–7, error-handling table, recovery process, verification checklist |
| [`references/project.md`](references/project.md) | Directory layout and file organization for the guest project |
| [`references/handlers.md`](references/handlers.md) | HTTP routing, message subscriptions, WebSocket events, and `lib.rs` wiring patterns |
| [`references/configuration.md`](references/configuration.md) | Cargo workspace, `.cargo/config.toml`, `deny.toml`, GitHub workflow templates, `.env.example` shape |
| [`references/providers.md`](references/providers.md) | WASI adapter provider patterns at the guest-wiring boundary |
| [`references/guest-wiring.md`](references/guest-wiring.md) | How crates wire into the guest (mirror of crate-writer's guidance) |
| [`references/omnia/`](references/omnia/) | Provider deep-dives, runtime macro, guest wiring examples |

## Guardrails

- **NEVER put business logic in the guest.** All domain logic lives in project crates; the guest is wiring only.
- **ALWAYS gate the guest with `#![cfg(target_arch = "wasm32")]`** — wasm32 is the only supported target.
- **NEVER use `std::env`, `std::fs`, `std::net`, or `std::thread`.** All I/O routes through provider traits; configuration via `omnia_sdk::Config`. Async only — no blocking operations.
- **ALWAYS dispatch messaging handlers explicitly.** Match topics directly and return `Err` for any unhandled topic.
- **ALWAYS export WebSocket handlers via `omnia_wasi_websocket::export!`** and implement `omnia_wasi_websocket::incoming_handler::Guest`.
- **ALWAYS pass an owner.** Every handler invocation must include `.owner("...")` in the builder chain.
- **ALWAYS use the builder API**: `.provider(&p).owner("o").await` — never the legacy `.process(&p)` form.
- **ALWAYS use `{param}` brace syntax** for Axum 0.8 route params, never `:param`.
