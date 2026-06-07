# Omnia build — guest writer

Loaded by [../build.md](../build.md) phase 4 on **first build only** (when no `src/lib.rs` exists at the workspace root). Subsequent builds skip this step; route / topic / WebSocket wiring updates are folded into [crate writer](crate.md)'s four-category cadence.

The guest is a thin WASI/wasm32 wrapper — it owns HTTP routing, topic dispatch, WebSocket exports, provider setup, and config validation; ALL business logic lives in domain crates under `crates/`.

## Hard rules

- **Never put business logic in the guest.** All domain logic lives in the project's domain crates; the guest is wiring only.
- **Gate the guest with `#![cfg(target_arch = "wasm32")]`** — wasm32 is the only supported target.
- **Forbid `std::env`, `std::fs`, `std::net`, `std::thread::spawn`** in guest code. All I/O routes through provider traits; configuration via `omnia_sdk::Config`. Async only — no blocking operations.
- **Dispatch messaging handlers explicitly.** Match topics directly and return `Err` for any unhandled topic.
- **Export WebSocket handlers via `omnia_wasi_websocket::export!`** and implement `omnia_wasi_websocket::incoming_handler::Guest`.
- **Always pass an owner.** Every handler invocation must include `.owner("...")` in the builder chain.
- **Use the builder API** — `.provider(&p).owner("o").await`, never the legacy `.process(&p)` form.
- **Axum 0.8 route params use `{param}` brace syntax**, never `:param`.

The full constraint list lives at [`guardrails.md`](../../references/guardrails.md) and [`wasm-constraints.md`](../../references/wasm-constraints.md).

## Process

1. **Lay down the root workspace** per [`configuration.md`](../../references/configuration.md) — `Cargo.toml` (workspace), `.cargo/config.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `Makefile.toml`, `.vscode/settings.json`. The configuration reference carries the full template bodies, including the five GitHub workflows (`audit`, `ci`, `patch`, `publish`, `release`) and the supply-chain files (`deny.toml`, `cargo-vet` config).
2. **Generate `src/lib.rs`** with Axum HTTP routing, message-topic dispatcher (`match topic { … }`), and WebSocket export hooks. Pattern catalogue: [`handlers.md`](../../references/handlers.md) and [`guest-patterns.md`](../../references/guest-patterns.md).
3. **Implement the `Provider` struct** that satisfies the consumed `omnia-wasi-*` adapter traits. Validate every required `Config::get` key in `Provider::new()` and document each in `examples/.env.example`. The crate → guest injection contract is at [`guest-wiring.md`](../../references/guest-wiring.md).
4. **Author `examples/<guest-name>.rs`** with the `omnia::runtime!({ main: true, hosts: { … } });` block enumerating every WASI host the guest consumes. See [`runtime.md`](../../references/runtime.md) for the macro, host options, and `.env.example` shape.
5. **Author the supply-chain files** per [`configuration.md`](../../references/configuration.md): `deny.toml`, `cargo-vet` config (`exemptions.lock`, `imports.lock`, `audits.toml`). After the workspace builds for the first time and produces `Cargo.lock`, run `cargo vet regenerate {imports,exemptions,unpublished}`.
6. **Author the five GitHub workflows** — full templates in [`configuration.md`](../../references/configuration.md): `audit`, `ci`, `patch`, `publish`, `release`.
7. **Apply the project layout** described in [`project-layout.md`](../../references/project-layout.md).
8. **Verify with `cargo check`** — fix any missing route / provider impl / wasm32-incompatible usage and re-check until clean. The parent brief's verify-repair loop runs after this step.

## When `WasiIdentity` is consumed

Identity needs OAuth2 credentials wired through `Config`. Add `IDENTITY_CLIENT_ID`, `IDENTITY_CLIENT_SECRET`, `IDENTITY_TOKEN_URL` to `.env.example` and assert their presence in `Provider::new()`. See [`providers/identity.md`](../../references/providers/identity.md) for the full integration pattern.
