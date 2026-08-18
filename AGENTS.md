# Emery - Agent Instructions

> **Remediation programme in flight ([ADR-0008](rfcs/decisions/0008-spec-generator-programme.md)).** Before starting any task, read [`CONSTITUTION.md`](CONSTITUTION.md) (standing invariants) and [`rfcs/remediation-plan.md`](rfcs/remediation-plan.md) (the plan of record). The product yardstick is [`rfcs/product.md`](rfcs/product.md); the destination is [`rfcs/target-architecture.md`](rfcs/target-architecture.md) — cite the spec-generator sections, not the deferred annex. Decisions flow through [`rfcs/decisions/`](rfcs/decisions/). Feature work is frozen until the spec walking skeleton is green.
>
> **This file maps the reduced Phase 0 tree** — what exists after the freeze, never a spec of what to build. The v1 implementation (survey + extract, plan/refine/execute, target adapters, the definition loop) is archived at git tag `v1`; retrieve it with `git worktree add ../emery-v1 v1`. First-party adapter prose lives in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters), itself tagged `v1` and untouched by this phase.

## What this repository is now

A Rust workspace at the repository root producing the `emery` runtime binary, plus one Cursor plugin (`plugins/emery/` carrying the `/emery:init` skill wrapper). The live CLI grammar is three verbs:

- `emery init <adapter>` — scaffold `.emery/`, resolve/cache the adapter, write `project.yaml` (`--upgrade` is the re-entry path; `--platforms <csv>` declares the platform set).
- `emery specify` — reserved by ADR-0008 §3 for the Phase 3 spec generator; today a typed stub failing `specify-not-implemented` (exit 1). It carries no orchestration, no output-home scaffolding, no artifacts.
- `emery completions <shell>` — auto-derived from the clap surface.

Deleted verbs are deleted from the grammar, not hidden — there are no compatibility aliases or deprecated stubs. The guest's mutating HTTP catch-all remains a typed refusal (C3, `crates/transport/src/http.rs`); the pre-bound listener serves adapter MCP shelves only.

## Vocabulary

- **source adapter** — input role: one WebAssembly component exporting the WIT `source-adapter` world (`extract` + `metadata`; no manifest file). `extract` takes a typed `SourceInput` (`key`, workspace-or-value) and returns an Evidence document of typed claims. Survey, leads, and the target axis were deleted from the seam (archived at `v1`). The WIT doc comment in [`wit/emery.wit`](wit/emery.wit) records the Phase 3 design inputs (extract returns the spec IR; required-extras validation is fail-closed) — recorded, not implemented.
- **engine** — this product: the engine guest (`emery:engine`), the surviving engine crates, and the seam opposite adapters.
- **plugin** (adapter vocabulary) vs **Cursor plugins** — do not confuse the adapter noun with `plugins/emery/`, the IDE distribution surface for the `/emery:init` skill wrapper; the latter is invisible to the `emery` CLI.

Artifact authority is unchanged in spirit: when authoritative inputs are incomplete, preserve the gap as `[unknown]` rather than guessing.

## The Rust workspace

Leaf → root. Each publishing package is `emery-<crate>` on crates.io; short `use` paths come from each crate's `[lib] name` (the root binary and `guest` / `launcher` / `mock` stay `publish = false`).

```text
error        # leaf — thiserror + serde-saphyr only
diagnostics  # neutral Diagnostic substrate + diagnostics::digest (SHA-256)
artifacts    # artifact types + parsers (evidence, atomic writer, validate registry); no engine deps
adapter      # the adapter SDK — the Source operations trait (extract + metadata), the WIT package + source! export macro, seam DTOs, embedded prose registry
project      # foundation — init (+ project::agents), adapter resolution (resolver::Component, ensure kernels, Locations/ExecutionPaths), config/Layout, platform enum, handler plumbing (Anchor, Ctx, Render, Error); plus residual v1 modules (plan model, snapshot/workspace kernels, journal, seam DTOs) left compiling for the Phase 3 cut
transport    # typed command router over Invoker: init + specify + completions, exhaustive TryFrom conversions, projectors, exit contract, HTTP refusal (C3)
launcher     # native-only deployment policy (publish = false) — anchoring, mounts, HTTP listener + /mcp/<axis>/<name> routing, fail-closed adapters-only GuestResolver, pull-on-miss install from GHCR
prose        # build-dependency crate — embed-time prompt-corpus walk + link check
native       # the native host — validated source-adapter Catalog, DynModel, the seam Provider, cli-gated command execution
guest        # the engine guest as a library (wasm32-only) — WIT bindings, WIT-backed Provider, guest::export!
mock         # dev-only mock crate (publish = false) — mock source adapter core, catalog registry, session helpers, mock::invoke
emery (root) # Omnia deployment unit under src/: guest cdylib (src/lib.rs) + shipped runtime (src/main.rs, one omnia::runtime! embedding $OUT_DIR/emery.bin)
```

### Repository map

```text
src/               shipped binary (omnia::runtime!) + wasm32 engine guest cdylib (guest::export!)
crates/            the workspace crates above
wit/               the emery:adapter WIT package (source-adapter world) + README
plugins/emery/     Cursor plugin: /emery:init skill wrapper, rules, manifest
docs/              Developer Guide (mdBook; reference + contributing + standards only)
rfcs/              product yardstick, target architecture, remediation plan, decisions/
```

### Exit codes

`Exit::from(&Error)` in [`crates/transport/src/command/output.rs`](./crates/transport/src/command/output.rs) is the single source of truth.

| Code | Name | When |
| ---- | ------------------------ | ------------------------------------------------------------------ |
| 0 | `EXIT_SUCCESS` | Command succeeded. |
| 1 | `EXIT_GENERIC_FAILURE` | Any `Error` variant not listed below (I/O, YAML, `specify-not-implemented`, …). |
| 2 | `EXIT_VALIDATION_FAILED` | Validation findings, `Error::Validation`, `Error::Argument`. |
| 3 | `EXIT_VERSION_TOO_OLD` | `Error::CliTooOld` — `project.yaml.emery` is newer than the binary — or `Error::AdapterCliTooOld`. |

## Testing philosophy

Emery strictly enforces an **aggressive integration-first posture**:

- Design against the public surface: if a behavior is reachable through a CLI input or `pub` fn and observable at a public boundary (stdout JSON, exit code, filesystem), write the integration test in `crates/<name>/tests/`; the unit test is redundant. Wire-contract coverage lives in `crates/transport/tests/`.
- Default to deletion; do not widen public APIs to test private kernels. `CRATE=<crate> cargo make cov` is the brake.
- One rung: native integration over `crates/mock`'s adapter catalog and the offline `crates/native` provider (`cargo make test`, per push). The wasm32 guest is compile-checked (`cargo check --lib -p emery --target wasm32-wasip2`); the v1 eval and wasm-example rungs are archived at `v1`.

See [`docs/standards/testing.md`](docs/standards/testing.md).

## Commands

All from the repository root, driven by `cargo make` ([`Makefile.toml`](./Makefile.toml)). Run the full local gate before committing; do not rely on bare `cargo test` or `cargo clippy`.

```bash
cargo make ci     # fmt + lint + test + test-docs + doc + links + vet + deny
cargo make check  # the pre-commit subset (cargo make fmt fixes formatting)
cargo make links  # Developer Guide link integrity (mdbook build docs, mdbook-linkcheck2)
cargo make test   # cargo nextest run --locked --workspace --all-features --no-tests=pass, under -Dwarnings
cargo make lint   # cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo make fmt    # nightly cargo fmt --all
```

Local Cursor preview of the skill wrapper: `cursor-agent --plugin-dir plugins/emery` (see [docs/contributing/operator-plugins.md](docs/contributing/operator-plugins.md)).

## Gotchas

- **Residual v1 modules compile but are unreferenced.** `crates/project` still carries the plan model, snapshot/workspace kernels, journal, and seam DTOs the pruned CLI never reaches; they fall at Phase 3 with the spine cut. Do not repair, extend, or design new seams into them — anything broken on the deletion frontier is deleted, never patched.
- **Adapter resolution is local-first.** `emery init <adapter>` accepts a package reference (`emery:intent@1.0.0`), the first-party shorthand (`intent@1.0.0`), a bare name, or a local `.wasm` path. A pin installs on first use from the compiled GHCR mapping (`ghcr.io/augentic/emery-adapters/<name>:<version>`) into the global store (`$EMERY_HOME/store/`, else `~/.emery/store/`); a bare name resolves the project cache seed, else the newest store version, else pull-latest. Cache hits always win. GitHub URLs are refused (`adapter-github-uri-unsupported`).
- Never hand-edit `project.yaml` or the component cache; never `mkdir -p .emery/...`. Route through the CLI.
- `cargo make links` enforces Developer Guide link integrity — renaming docs paths requires updating links in the same change.
- Crossing a major is a hard cut: no silent compatibility aliases and no migration framework. Pre-1.0, a major bump means re-init.
- Brevity caps are mechanically enforced by root-crate tests (`tests/ident_brevity.rs`, `tests/doc_brevity.rs`, part of `cargo make test`).

## Related coding standards

- The external Rust baseline is the [Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/guidelines/index.html); [docs/standards/](docs/standards/) carries the house deltas (overrides win).
- Cross-cutting rules: [style.md](docs/standards/style.md). Lints, comments, DTOs, module layout: [coding-standards.md](docs/standards/coding-standards.md). Operation/router shape and exit mapping: [handler-shape.md](docs/standards/handler-shape.md). Workspace layout and deployment split: [architecture.md](docs/standards/architecture.md). Skill/CLI split and the JSON envelope: [cli-contract.md](docs/standards/cli-contract.md). Docs house style: [doc-authoring.md](docs/standards/doc-authoring.md).
- Lint suppressions: refactor first; `#[expect(lint, reason = "…")]` at the smallest scope; state in the PR why a refactor was infeasible.
- When you remove a symbol, `rg <SymbolName> -- AGENTS.md docs/` and update every hit in the same PR.
- Markdown changes follow [doc-authoring.md](docs/standards/doc-authoring.md); do not hard-wrap prose solely for column width.

Run `cargo make ci` before committing. If it cannot run, say exactly why and which checks were run instead.
