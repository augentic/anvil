# Guest Writer Runbook

Operational detail for `omnia-guest-writer`. The SKILL.md keeps only the orientation surface (Critical Path + Reference table + Guardrails); everything procedural lives here.

## Overview

Generate a complete WASM guest project that wraps one or more domain crates containing business logic. The guest provides the wiring layer that:

- Exposes HTTP endpoints using `wasip3::http` types
- Handles message subscription using `omnia_wasi_messaging` types
- Handles WebSocket events using `omnia_wasi_websocket` types
- Configures provider traits for WASI adapters (Config, Publish, Identity, StateStore, TableStore, Blobstore, DocumentStore, etc.)
- Bridges domain logic to the Omnia WASI runtime

## Key Principle

The guest is a thin wrapper. It handles WASI/wasm32 boundary concerns such as HTTP routing, subscribing to message topics, handling WebSocket events, and provider setup. **ALL** business logic is delegated to project crates (in the crates/ directory).

## Process

All paths in this skill are relative to the project root, consistent with `crate-writer` and `test-writer`. The reference docs cited from each step are specifications, not examples — follow them exactly.

### Step 1: Generate project structure

Create the guest project at the project root with structure:

```text
./
├── .cargo/config.toml   # Registry + credential providers
├── .github/
│   └── workflows/
│       ├── audit.yaml   # Daily security audit
│       ├── ci.yaml      # CI on every push
│       ├── patch.yaml   # Create patch release
│       ├── publish.yaml # CI → Publish → Deploy pipeline
│       └── release.yaml # Create release
├── .vscode/settings.json # rust-analyzer wasm32 config
├── Cargo.toml           # Workspace and dependencies
├── Makefile.toml        # Build tasks (cargo-make)
├── clippy.toml          # Lint exceptions
├── deny.toml            # Dependency checks
├── rust-toolchain.toml  # Nightly + wasm32 target
├── rustfmt.toml         # Formatting rules
├── src/
│   └── lib.rs           # HTTP and Messaging Guest implementations
├── supply-chain/
│   └── audits.toml      # Cargo vet audits file
│   └── config.toml      # Cargo vet config file
│   └── imports.lock     # Cargo vet lock file
│   └── README.md        # Cargo vet instruction file
├── examples/
│   ├── <guest>.rs       # Local runtime via omnia::runtime!
│   └── .env.example     # Environment template
└── crates/              # (optional) local crates
```

See [project.md](project.md) for the complete layout and [configuration.md](configuration.md) for standard config file contents.

### Step 2: Generate Cargo.toml

All omnia packages are published to crates.io. **Configure `.cargo/config.toml` first** -- see [configuration.md](configuration.md) for the full configuration including registry URLs, credential providers, and net settings.

Then configure workspace dependencies based on domain crate requirements:

- Always include: `omnia-sdk`, `anyhow`, `bytes`, `tracing`
- If HTTP used: `omnia-wasi-http`, `axum` (with features `["json", "macros", "query"]`)
- If messaging used: `omnia-wasi-messaging`
- If WebSocket used: `omnia-wasi-websocket`
- If StateStore used: `omnia-wasi-keyvalue`
- If Identity used: `omnia-wasi-identity`
- If TableStore used: `omnia-wasi-sql`
- If Blobstore used: `omnia-wasi-blobstore`
- If DocumentStore used: `omnia-wasi-jsondb`

All `omnia-*` crates are published on **crates.io**. No private registry configuration is needed.

See [configuration.md](configuration.md) for dependency patterns and version resolution instructions.

### Step 3: Generate src/lib.rs

Generate the main guest module with manual wiring for full control over HTTP routing, messaging dispatch, WebSocket handling, and handler invocation:

1. **HTTP Guest** -- Axum router with routes using `{param}` syntax (Axum 0.8)
2. **Messaging Guest** -- Topic dispatcher that returns `Err` for unhandled topics
3. **WebSocket Guest** -- Event handler that delegates to domain crate handlers
4. **Handler invocation** -- use the builder API: `Type::handler(input)?.provider(&provider).owner("owner").await`
5. **Provider** -- trait implementations for WASI adapters

**Owner**: every handler requires an `owner` string identifying the Omnia component owner (e.g. `"at"`). See [omnia/providers/README.md](omnia/providers/README.md#owner) for details.

See also [handlers.md](handlers.md) for HTTP, messaging, and WebSocket patterns.

### Step 4: Generate Runtime Example

Create `examples/<guest>.rs` with `omnia::runtime!` macro to enable local development and testing.

See [omnia/runtime.md](omnia/runtime.md).

### Step 5: Generate Environment Template

Create `examples/.env.example` with all required config keys documented.

### Step 6: Generate GitHub Workflows

Create `.github/workflows/` with the standard CI/CD workflow files. All workflows delegate to reusable workflows in the `augentic/.github` repository.

Generate all 5 files: `audit.yaml`, `ci.yaml`, `patch.yaml`, `publish.yaml`, `release.yaml`.

For `publish.yaml`, configure the project-specific deployment parameters (`package`, `storage-account`, `resource-group`) based on the project context.

See [configuration.md](configuration.md#github-workflows) for templates and required secrets.

### Step 7: Generate Supply-Chain and Compliance Files

Generate dependency compliance configuration for Cargo Deny and Cargo Vet:

1. **`deny.toml`** -- Cargo Deny configuration for license, advisory, ban, and source checks. Use the standard template from [configuration.md](configuration.md#denytom). Customize `[sources].private` to match the project's private registry URL(s) from `.cargo/config.toml`.

2. **`supply-chain/README.md`** -- Instructions for updating vetted dependencies after code changes.

3. **`supply-chain/config.toml`** -- Cargo Vet configuration with standard imports from trusted audit sources (bytecode-alliance, embark-studios, google, isrg, mozilla, zcash).

4. **`supply-chain/audits.toml`** -- Empty scaffold (populated by cargo vet commands).

5. **`supply-chain/imports.lock`** --- Empty lock file (populated by cargo vet commands).

After generating all project files, run:

```bash
cargo vet regenerate imports
cargo vet regenerate exemptions
cargo vet regenerate unpublished
```

These commands populate workspace-specific exemptions, policies, trusted publishers, and import data in the supply-chain directory. They require `Cargo.toml` and `Cargo.lock` to exist.

See [configuration.md](configuration.md) for all templates and post-generation details.

## Examples

Refer to the crate-specific examples that demonstrate guest wiring patterns:

- [omnia/guest-wiring.md](omnia/guest-wiring.md) -- How to wire HTTP routes, messaging topics, and WebSocket events into the guest project
- [omnia/runtime.md](omnia/runtime.md) -- Runtime example generation pattern

Each example includes the expected directory structure, generated files, and key wiring patterns.

## Error Handling

### Common Issues and Resolutions

| Issue                        | Cause                                  | Resolution                                              |
| ---------------------------- | -------------------------------------- | ------------------------------------------------------- |
| `src/lib.rs` already exists  | Guest project previously generated     | Skip generation (idempotent check)                      |
| Missing route for endpoint   | Endpoint from domain crate not wired into Axum router | Check domain crate handler exports; add missing route to src/lib.rs |
| Missing messaging handler    | Topic subscription from domain crate not wired        | Check domain crate messaging handlers; add topic match arm          |
| Missing WebSocket handler    | WebSocket handler from domain crate not wired         | Check domain crate WebSocket exports; add handler delegation        |
| Provider missing trait impl  | New provider needed by domain crate    | Add trait implementation to Provider struct             |
| Cargo.toml dependency error  | Domain crate path incorrect            | Verify the domain crate path relative to guest project root |
| Build fails on wasm32 target | Non-WASM-compatible code in guest      | Check for std::env, std::fs, std::net usage             |

### Recovery Process

1. Run `cargo check` and capture errors
2. For missing routes: add endpoint to Axum router in `src/lib.rs`
3. For provider errors: add trait bounds and implementations
4. For build errors: verify wasm32 compatibility of all dependencies
5. Re-run `cargo check` after each fix

## Verification Checklist

Before completing, verify:

- [ ] All HTTP endpoints are routed in the Axum router
- [ ] All message subscriptions have handlers in MessagingGuest
- [ ] All WebSocket events have handlers in WebSocketGuest (if WebSocket is used)
- [ ] Provider implements all required traits
- [ ] All config keys are validated in Provider::new()
- [ ] Domain crate is properly imported in Cargo.toml
- [ ] Runtime example compiles with `cargo build --example`
- [ ] `.env.example` documents all required environment variables
- [ ] Config files present: `rustfmt.toml`, `rust-toolchain.toml`, `clippy.toml`, `.vscode/settings.json`, `Makefile.toml`
- [ ] GitHub workflows present: `audit.yaml`, `ci.yaml`, `patch.yaml`, `publish.yaml`, `release.yaml`
- [ ] `deny.toml` present with `[sources].private` matching `.cargo/config.toml` registry URL(s)
- [ ] `supply-chain/` directory present with `config.toml`, `audits.toml`, and `README.md`
- [ ] Handlers annotated with `#[omnia_wasi_otel::instrument]`
