# Emery - Agent Instructions

> **Remediation programme in flight.** This file maps the reduced Phase 0 tree — what exists after the freeze, never a spec of what to build. Feature work is frozen until the spec walking skeleton is green.
>
> The v1 implementation (survey + extract, plan/refine/execute, target adapters, the definition loop) is archived at git tag `v1`; retrieve it with `git worktree add ../emery-v1 v1`. First-party adapters live in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters) — re-seamed extract-only in Phase 4 (its `v1` tag keeps the survey-era tree).

## What this repository is now

A Rust workspace at the repository root producing the `emery` runtime binary, plus one Cursor plugin (`plugins/emery/` carrying the `/emery:specify` skill wrapper). The live CLI grammar is three verbs:

- `emery specify <adapter>... [--value <adapter>=<text>] [--sources <path>]` — the spec generator (ADR-0008 §3, ADR-0009): resolve the sources named on the invocation (mirroring a local `.wasm` into the project cache), extract each binding over the adapter seam, reconcile under authority precedence, synthesise, and commit `spec.md` / `design.md` as one generation behind the swapped `current` pointer. The binding list is per-run input, never persisted; a re-run reports the re-mine diff against the superseded generation in the success envelope — never persisted (ADR-0010).
- `emery show <spec|design>` — print a reviewable document of the current generation to stdout; text mode is the document body alone, and the generation id rides the JSON envelope.
- `emery completions <shell>` — auto-derived from the clap surface.

Deleted verbs are deleted from the grammar, not hidden — there are no compatibility aliases or deprecated stubs. The guest's mutating HTTP catch-all remains a typed refusal (C3, `crates/transport/src/http.rs`); the pre-bound listener serves MCP shelves only — the adapter reference shelves plus the engine's read-only spec shelf (`/mcp/emery/spec`, the current generation and its id for IDEs and agents).

## Vocabulary

- **source adapter** — input role: one WebAssembly component exporting the WIT `source-adapter` world (`extract` + `metadata`; no manifest file). `extract` takes a typed `SourceInput` (`key`, workspace-or-value) and returns an Evidence document of typed claims — the spec IR (A8/A16; required extras are fail-closed engine-side, ADR-0009 §3). Survey, leads, and the target axis were deleted from the seam (archived at `v1`); see [wit/emery.wit](wit/emery.wit).
- **engine** — this product: the engine guest (`emery:engine`), the surviving engine crates, and the seam opposite adapters.
- **plugin** (adapter vocabulary) vs **Cursor plugins** — do not confuse the adapter noun with `plugins/emery/`, the IDE distribution surface for the `/emery:specify` skill wrapper; the latter is invisible to the `emery` CLI.

Artifact authority is unchanged in spirit: when authoritative inputs are incomplete, preserve the gap as `[unknown]` rather than guessing.

## The Rust workspace

Leaf → root. Each publishing package is `emery-<crate>` on crates.io; Rust `use` paths follow the package name (`emery_error::`, `emery_engine::`, …). The root package and `emery-testkit` stay `publish = false`.

```text
error        # leaf — thiserror + serde-saphyr only
diagnostics  # neutral Diagnostic substrate + emery_diagnostics::digest (SHA-256)
artifacts    # artifact types + parsers (evidence, validate registry); no engine deps
adapter      # the adapter SDK — the SourceAdapter operations trait (extract + metadata + docs), the WIT package + source! export macro, the Source capability (wasm32 defaults over the engine guest's source::import seam wrappers; bare natively so tests script the seam), seam DTOs, embedded prose registry
engine       # the spec generator — per-run source bindings (argv + sources.toml loaders), specify + show operations, extract leg (ensure + required-extras gate) over the provider's Source capability, reconcile/synthesise (embedded synthesis prose), the generation-pointer output home; plus the ported kernels: emery_engine::resolve (resolver::Component, ensure, metadata::runner) and emery_engine::handler (preopen-relative ExecutionPaths/Locations, Render, Error)
transport    # typed command router over Invoker: specify + show + completions, exhaustive TryFrom conversions, projectors, exit contract, HTTP surface (read-only MCP spec shelf + C3 refusal)
prose        # build-dependency crate — embed-time prompt-corpus walk + link check
testkit      # unpublished — scripted StateStore/BlobStore doubles (`Memory`, `Namespaced`) for native tests; not a production crate
emery (root) # Omnia deployment unit under src/: wasm32 engine guest cdylib (src/lib.rs — bare model provider, wasi:cli/run, spec shelf + HTTP refusal) + shipped runtime (src/main.rs, one omnia::runtime! embedding $OUT_DIR/emery.cwasm; static, CWD-rooted deployment policy inline — the invocation directory mounts read-only as `.`, and the wasi:keyvalue/wasi:blobstore hosts bind engine state (component cache and store included) to the durable omnia-filesystem store (default `.omnia/storage`); adapter guests are declared in the runtime invocation; dynamic resolution is deferred)
```

### Repository map

```text
src/               shipped binary (omnia::runtime!, static CWD-rooted deployment, filesystem-backed keyvalue/blobstore hosts) + wasm32 engine guest cdylib (bare model provider, wasi:cli/run, spec shelf + HTTP refusal)
crates/            the workspace crates above
examples/          source adapter (guest + adapter) + runtime host (root-package examples; ADR-0009 §5) + profile host (project-id-keyed storage; docs/reference/deployment-profiles.md)
wit/               the emery:adapter WIT package (source-adapter world) + README
plugins/emery/     Cursor plugin: /emery:specify skill wrapper, rules, manifest
docs/              Developer Guide (mdBook; reference + contributing + standards only)
```

### Exit codes

`Exit::from(&Error)` in [`crates/transport/src/command/output.rs`](./crates/transport/src/command/output.rs) is the single source of truth.

| Code | Name | When |
| ---- | ------------------------ | ------------------------------------------------------------------ |
| 0 | `EXIT_SUCCESS` | Command succeeded. |
| 1 | `EXIT_GENERIC_FAILURE` | Any `Error` variant not listed below (I/O, YAML, `spec-not-generated`, …). |
| 2 | `EXIT_VALIDATION_FAILED` | Validation findings, `Error::Validation`, `Error::Argument`. |
| 3 | `EXIT_VERSION_TOO_OLD` | `Error::AdapterCliTooOld` — an adapter's declared `emery` floor is newer than the binary. |

## Testing philosophy

Emery strictly enforces a **root-led integration posture** (DWN-style):

- The root `tests/` scenario suites are the default home for every CLI- or MCP-reachable behavior: `tests/specify.rs` (the `specify` → `show` product arc), `tests/command.rs` (the CLI wire contract), and `tests/shelf.rs` (the MCP spec shelf and the C3 refusal) drive the in-process command router and HTTP listener over scripted capabilities (`tests/support/mod.rs`) and read like usage documentation.
- Crate suites in `crates/<name>/tests/` survive only for independently useful library contracts (the adapter SDK, artifacts, diagnostics, error, prose) or product invariants impractical to arrange through the entry seams; unit tests are near-zero, reserved for genuinely CLI-unreachable branches.
- Default to deletion; do not widen public APIs to test private kernels. `cargo make cov` (workspace-wide) is the brake; `CRATE=emery-<crate> cargo make cov-crate` audits one leaf contract.
- One fast rung: the native suites (`cargo make test`, per push) over scripted `Model` + `Source` + storage (`StateStore`/`BlobStore`); engine state is asserted through the scripted store and the envelope, never the filesystem. The wasm32 guest is linted under the guest deny-list (`cargo make lint`'s wasm leg, which subsumes the old compile check); the v1 eval and wasm-example rungs are archived at `v1`. No test builds or spawns the mock source component.

See [`docs/standards/testing.md`](docs/standards/testing.md).

## Commands

All from the repository root, driven by `cargo make` ([`Makefile.toml`](./Makefile.toml)). Run the full local gate before committing; do not rely on bare `cargo test` or `cargo clippy`.

```bash
cargo make ci     # fmt + lint + test + test-docs + doc + links + vet + deny
cargo make check  # the pre-commit subset (cargo make fmt fixes formatting)
cargo make links  # Developer Guide link integrity (mdbook build docs, mdbook-linkcheck2)
cargo make test   # cargo nextest run --locked --workspace --all-features --no-tests=pass, under -Dwarnings
cargo make lint   # clippy --workspace --all-targets --all-features -- -D warnings
cargo make fmt    # nightly cargo fmt --all
cargo make source         # build the mock source example (wasm32-wasip2, release)
cargo make runtime        # build the scripted-model runtime example
cargo make profile        # build the project-id-keyed deployment-profile example
```

Local Cursor preview of the skill wrapper: `cursor-agent --plugin-dir plugins/emery` (see [docs/contributing/operator-plugins.md](docs/contributing/operator-plugins.md)).

## Gotchas

- **Adapter admission is static.** `emery specify <adapter>` accepts a package reference (`emery:intent@1.0.0`), the first-party shorthand (`intent@1.0.0`), a bare name, or a local `.wasm` path — but until the dynamic resolver returns, dispatch lands only on guests declared in the runtime invocation (`src/main.rs`; the journey host declares its mock `source` the same way in `examples/runtime.rs`). A local `.wasm` still mirrors into the project cache on the first `specify` that names it; extract dispatch beyond the declared set fails at the seam. There is no download path (ADR-0002 §2), and GitHub URLs are refused (`adapter-github-uri-unsupported`).
- Never hand-edit `.emery/` state (the component cache, the generation store); never `mkdir -p .emery/...`. Route through the CLI. The binding list is per-run input — argv or an operator-owned `sources.toml` named by `--sources`; the engine never writes or discovers it.
- `cargo make links` enforces Developer Guide link integrity — renaming docs paths requires updating links in the same change.
- Crossing a major is a hard cut: no silent compatibility aliases and no migration framework. Pre-1.0, a major bump means regenerating with a fresh `emery specify`.
- Brevity caps (identifier ≤ 25, comment density) are review-only; see [coding-standards.md](docs/standards/coding-standards.md). WIT doc/comment caps are the same rule.

## Related coding standards

- The external Rust baseline is the [Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/guidelines/index.html); [docs/standards/](docs/standards/) carries the house deltas (overrides win).
- Cross-cutting rules: [style.md](docs/standards/style.md). Lints, comments, DTOs, module layout: [coding-standards.md](docs/standards/coding-standards.md). Operation/router shape and exit mapping: [handler-shape.md](docs/standards/handler-shape.md). Workspace layout and deployment split: [architecture.md](docs/standards/architecture.md). Skill/CLI split and the JSON envelope: [cli-contract.md](docs/standards/cli-contract.md). Docs house style: [doc-authoring.md](docs/standards/doc-authoring.md).
- Lint suppressions: refactor first; `#[expect(lint, reason = "…")]` at the smallest scope; state in the PR why a refactor was infeasible.
- When you remove a symbol, `rg <SymbolName> -- AGENTS.md docs/` and update every hit in the same PR.
- Markdown changes follow [doc-authoring.md](docs/standards/doc-authoring.md); do not hard-wrap prose solely for column width.

Run `cargo make ci` before committing. If it cannot run, say exactly why and which checks were run instead.
