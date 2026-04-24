# Omnia Plugin

Generate and review Rust WASM crates targeting the [Omnia](https://omnia.host) runtime.

## Skills

### /omnia:crate-writer

Generate or update Rust crates from Specify artifacts.

**Invocation:** Via skill directive tag in `tasks.md` (`<!-- skill: omnia:crate-writer -->`) or directly.

**Inputs:**
- `crate-name` -- derives the change directory and crate path.
- Reads `spec.md`, `design.md`, and baseline specs.

**Outputs:**
- Rust crate at `crates/<name>/` with `Cargo.toml`, `lib.rs`, domain modules, and documentation.
- Guest wiring if a guest project exists.

**Modes:**
- **Create** -- generates a new crate from scratch.
- **Update** -- reads existing crate code and applies targeted changes based on delta specs.

The skill uses a provider-based dependency injection pattern. Side effects (HTTP, messaging, state store, transactions) are expressed through provider traits, making crates testable with mock providers.

### /omnia:test-writer

Generate or update test suites for Omnia crates.

**Invocation:** Via skill directive tag (`<!-- skill: omnia:test-writer -->`) or directly.

**Inputs:**
- `crate-name` -- the crate to test.
- Reads `spec.md`, `design.md`, and the crate's source code.

**Outputs:**
- Test files under `tests/` with `MockProvider` setup.
- Spec-to-test mapping for traceability.

Maps each requirement scenario to a test case. Uses the `MockProvider` pattern to isolate side effects.

### /omnia:guest-writer

Generate the WASM guest wrapper that exposes HTTP endpoints, messaging subscriptions, and WebSocket event handlers.

**Invocation:** Via skill directive tag (`<!-- skill: omnia:guest-writer -->`) or directly.

**Inputs:**
- Project context derived from the crate structure.

**Outputs:**
- Guest project tree: `lib.rs`, `Cargo.toml`, CI workflows, supply-chain configuration (`deny.toml`, `cargo-vet`).

The guest is the entry point that the Omnia runtime calls. It wires incoming requests to the crate's business logic.

### /omnia:code-reviewer

Review generated Rust WASM crates for correctness and Omnia compliance.

**Invocation:** Via skill directive tag (`<!-- skill: omnia:code-reviewer -->`) or directly after generation.

**Inputs:**
- `crate-path` -- path to the crate to review.
- Optional `--fix` flag for auto-correction.

**Outputs:**
- `REVIEW.md` with categorised findings.
- Optional code fixes and `cargo check` verification.

Uses an agent team pattern: three specialist reviewers (structural, logic, quality) run concurrently, followed by an antagonist that challenges findings for false positives. The lead synthesises results with confidence scoring.
