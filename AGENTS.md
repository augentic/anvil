# Emery - Agent Instructions

> **Remediation programme in flight.** This file maps the reduced Phase 0 tree — what exists after the freeze, never a spec of what to build. Feature work is frozen until the spec walking skeleton is green.
>
> The v1 implementation (survey + extract, plan/refine/execute, target adapters, the definition loop) is archived at git tag `v1`; retrieve it with `git worktree add ../emery-v1 v1`. First-party adapters live in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters) — cut to extract-only in Phase 4 (its `v1` tag keeps the survey-era tree).

## What this repository is now

A Rust workspace at the repository root producing the `emery` runtime binary, plus one Cursor plugin (`plugins/emery/` carrying the `/emery:specify` skill wrapper). The live CLI grammar is three verbs:

- `emery specify <adapter>... [--description <adapter>=<text>] [--config [<path>]]` — the spec generator: resolve the sources named on the invocation (a project-relative local `.wasm` loads through the deployment's `omnia:plugins/loader` capability, read fresh each run; an exact package reference fetches from the binding's `registry` override or the compiled-in `omnia.host` default; either way the binding's optional `digest` pin is verified host-side and the resolved digest rides the success envelope), extract each binding over the `Source` capability, reconcile under authority precedence, synthesise, and commit `spec.md` / `design.md` as one revision, atomically swapping the current revision id. `--config` without a value explicitly selects `emery.toml`; a run naming no bindings at all discovers the project-root `emery.toml` as a fallback — never merged with argv bindings. The binding list is per-run input, never persisted; a re-run reports the re-mine diff against the superseded revision in the success envelope — never persisted.
- `emery show <spec|design>` — print a reviewable document of the current revision to stdout; text mode is the document body alone, and the revision id rides the JSON envelope.
- `emery completions <shell>` — auto-derived from the clap surface.

Deleted verbs are deleted from the grammar, not hidden — there are no compatibility aliases or deprecated stubs. The HTTP surface is deleted outright — no listener, no MCP shelves — so C3 (no unauthenticated HTTP ingress) is satisfied by absence rather than refusal. Adapter references reach the model through the completion session's tool closure (`list_docs` / `read_doc`, answered in-process from the embedded corpus), and the committed revision is reviewed over `emery show`.

## Vocabulary

- **source adapter** — input role: one WebAssembly component exporting the WIT `source-adapter` world (`extract` + `metadata`; no manifest file). `extract` takes a typed `SourceInput` (`key`, workspace-or-value) and returns an Evidence document of typed claims — the spec IR (A8/A16; the claim gate — id grammar + required per-kind extras — is one `emery_source::claims` rule, run in the SDK's answer tail so a miss is repaired in-adapter, and re-run fail-closed engine-side because the engine cannot trust the guest). Survey, leads, and the target axis were deleted from the WIT contract (archived at `v1`); see [wit/emery.wit](wit/emery.wit).
- **engine** — this product: the engine guest (`emery:engine`), the surviving engine crates, and the engine side of the adapter contract.
- **capability** — an engine-side trait a provider carries (`Source`, `Model`, `Plugins`, `StateStore` / `BlobStore`).
- **contract** — the typed agreement (WIT, CLI, wire).
- **boundary** — the crossing you test or bind (entry points, host bindings).
- **plugin** (adapter vocabulary) vs **Cursor plugins** — do not confuse the adapter noun with `plugins/emery/`, the IDE distribution surface for the `/emery:specify` skill wrapper; the latter is invisible to the `emery` CLI.

Artifact authority is unchanged in spirit: when authoritative inputs are incomplete, preserve the gap as `[unknown]` rather than guessing.

## The Rust workspace

Leaf → root. Each publishing package is `emery-<crate>` on crates.io; Rust `use` paths follow the package name (`emery_source::`, `emery_adapter::`, `emery_engine::`, `emery_cli::`, …). The root package and the `component` rung package stay `publish = false`; the mock adapter is a root-package example, not a crate. Scripted capability doubles come from omnia's `omnia-test` crate (dev-dependency, native only), not a local crate. `prose` and `source` are the leaves both the engine and the adapters consume; `adapter` is the guest-only SDK over them, and no production crate depends on it. `cli` is a transport façade over `engine`: the engine depends on nothing CLI-shaped, and only the root's wasm32 arm and the root suites depend on `cli`.

```text
prose        # embedded prompt corpora — the runtime registry (Doc, find/body, the registry! macro over the generated DOCS table) with no dependencies; feature `emit` (build-dependencies only) adds the embed-time corpus walk + link check
source       # the emery:adapter/source contract, both sides — the WIT package + one wit-bindgen generation (wasm32; the export bindings the SDK wires into, and the engine guest's caller side in wire::import), the Evidence DTOs (types), the import-side Source capability (wasm32 defaults over wire::import; bare natively so tests script Source), DispatchError, AdapterIdentity, SOURCE_INTERFACE, and the deterministic claim gate (claims: the dotted-kebab id grammar, ClaimKind::required_extras, Evidence::validate)
adapter      # the adapter SDK — the SourceAdapter operations trait (extract + metadata + docs), the source! export macro, the judgment loop (judgment / repaired with bounded repair), Context, the reference tool closure (list_docs / read_doc over the embedded docs), the Evidence answer schema + tail; re-exports the source contract so an adapter depends on one crate
engine       # the transport-neutral spec generator — the `Specify { bindings }` and `Show { document }` operations (serde DTOs implementing omnia's Handler<P>, returning `Serialize`-only bodies), the binding DTO + its rules (emery_engine::sources — SourceBinding / BindingContent and validate: non-empty, unique keys, selector parse, digest/registry gating, preopen-relative roots), extract leg (load + the contract's claim gate re-run over the wire as `bad_request`) over the provider's Source capability, reconcile/synthesise (embedded synthesis prose), the fail-closed spec AST, the revision store (the private `store` module; its `Diff` DTO is exported through `emery_engine::specify`, and the `CURRENT` keyvalue key and `CONTAINER` blobstore name at the crate root); plus the ported kernels: emery_engine::AdapterSelector + the private `load` module (selector parsing, the per-run `Loader` memo over the Plugins capability for local components and registry packages, the adapter `emery-version` gate) and emery_engine::preopen_path (preopen-relative path normalization); plus the `Provider` alias bound. No clap, no toml, no terminal text, no exit codes — omnia's HTTP overlay routes over it unchanged
cli          # the emery command façade over the engine — the clap grammar (App / SpecifyArgs / ShowArgs / DocumentArg, help prose, completions), the binding carriers (argv positionals + --description, the --config emery.toml file, project-root discovery → Vec<SourceBinding>), Client dispatch, the Text trait (text mode) + Serialize (JSON) projector, the failure envelope + hint table, the buffered Response, and the exit contract (exit_code)
emery (root) # Omnia deployment unit under src/: wasm32 engine guest cdylib (src/lib.rs wasm32 arm — bare model provider, wasi:cli/run over emery_cli::run) + shipped runtime (src/lib.rs native arm, one omnia::runtime! embedding $OUT_DIR/emery.cwasm, exporting `main` / `manifest` / `Hooks`; src/main.rs calls `emery::main()`; static, CWD-rooted deployment policy inline — the invocation directory mounts read-only as `.`, the wasi:keyvalue/wasi:blobstore hosts bind the revision state to the durable omnia-filesystem store (compiled-in root `.omnia/storage`), and the plugins block declares the source seam with the declarative locations list: the `.` path root, so local `.wasm` adapters load fresh each run, then the omnia.host registry endpoint for exact package references, read fresh (no project cache); bare names still dispatch statically declared guests)
```

### Repository map

```text
src/               lib.rs: the wasm32 engine guest cdylib (bare model provider, wasi:cli/run) and, natively, the shipped omnia::runtime! (static CWD-rooted deployment, filesystem-backed keyvalue/blobstore hosts); main.rs: the binary entry calling emery::main()
crates/            the workspace crates above
examples/          the mock greeting adapter (`adapter/` — a root-package `[[example]]` cdylib: SourceAdapter + embedded prose; `docs/` is its fixture tree) + cursor-bound runtime host (`runtime.rs`, root-package example) + the component rung (`component/` — an unpublished workspace package, the adapters repo's `examples/conformance` shape: build.rs drives `omnia_test::build::Components` to compile `--example adapter` to wasm32-wasip2 and generate the `MOCK_ADAPTER` path; the lib's `run` overlays the shipped deployment (`omnia_test::host::Deployment::from(emery::manifest())` — the scenario's project mount, path root, argv, and declared adapter guests) and drives it through `emery::Hooks` over `omnia_test::host::Backends<ScriptedModel>`, `engine` hands back the embedded engine bytes; `tests/component.rs` is the suite)
wit/               the emery:adapter WIT package (source-adapter world) + README
plugins/emery/     Cursor plugin: /emery:specify skill wrapper, rules, manifest
docs/              Developer Guide (mdBook; reference + contributing + standards only)
```

### Exit codes

`exit_code` in [`crates/cli/src/lib.rs`](./crates/cli/src/lib.rs) maps `omnia_guest::Error` variants and is the single source of truth.

| Code | Name | When |
| ---- | ------------------------ | ------------------------------------------------------------------ |
| 0 | `EXIT_SUCCESS` | Command succeeded. |
| 1 | `BadRequest` | Operator or input refusal. The `error` field is `specify-source-required`, `unsupported-version`, a loader refusal (`refused`, `already-active`), or the Omnia default `bad_request`. |
| 2 | `NotFound` | Missing resource. The `error` field is `spec-not-generated` or the Omnia default `not_found`. Clap usage and unknown-verb also exit 2 (framework). |
| 3 | `ServerError` | Unclassified default: I/O, storage, leftover conversions. The `error` field is the Omnia default `server_error` or the loader's `internal`. |
| 4 | `BadGateway` | Upstream, model, or component-acquisition failure. The `error` field is the Omnia default `bad_gateway` or the loader's `unavailable`. |

Omnia default codes are snake_case (`bad_request`, `not_found`, `server_error`, `bad_gateway`). The recovery and loader discriminants stay kebab-case so skills can branch on them.

## Testing philosophy

Emery strictly enforces a **root-led integration posture** (DWN-style):

- The root `tests/` scenario suites are the default home for every CLI-reachable behavior: `tests/specify.rs` (the `specify` → `show` product arc), `tests/command.rs` (the CLI wire contract), and `tests/plugin.rs` (plugin-rule mentions vs the shipped grammar) drive the in-process command façade (`emery_cli::run`) over scripted capabilities (`tests/support/mod.rs`, assembling omnia-test's `guest::{Scripted, ScriptedLoader, Memory, Namespaced}` with the local `Source` double through `omnia_test::delegate!`) and read like usage documentation. `examples/component/tests/component.rs` is the component rung, its own unpublished package so the fixture build stays out of the shipped binary's build script: the shipped deployment (`emery::manifest()`, overlaid by the package's `run` harness through `omnia_test::host::Deployment`) plus the built mock adapter under the real omnia runtime, owning only the wasm boundary itself — `wasi:cli/run`, the seam lowering, the real path loader and digest pin, the reference-tool streams — observed through exit status and storage handles.
- Crate suites in `crates/<name>/tests/` survive only for independently useful library contracts (the adapter SDK, prose) or product invariants impractical to arrange through the entry points; unit tests are near-zero, reserved for genuinely CLI-unreachable branches.
- Default to deletion; do not widen public APIs to test private kernels. `make cov` (workspace-wide) is the brake; `CRATE=emery-<crate> make cov-crate` audits one leaf contract.
- One `make test` invocation (per push) carries both rungs: the native suites over scripted `Model` + `Source` + `Plugins` + storage (`StateStore`/`BlobStore`), with engine state asserted through the scripted store and the envelope, never the filesystem; and the component rung, the only place a wasm component is built (`examples/component/build.rs` over `omnia_test::build::Components`, incremental after the first build) — no test spawns a process. omnia-test's `Scripted` and `ScriptedModel` fail a scenario that leaves scripted turns unconsumed or runs past its script, so every scenario scripts exactly what the run consumes. The v1 eval and wasm-example rungs are archived at `v1`.

See [`docs/standards/testing.md`](docs/standards/testing.md).

## Commands

All from the repository root, driven by `make` ([`Makefile`](./Makefile) → mise). Run the full local gate before committing; do not rely on bare `cargo test` or `cargo clippy`.

```bash
make ci       # fmt + lint + test + test-docs + doc + links + vet + deny
make check    # the pre-commit subset (make fmt writes formatting)
make links    # Developer Guide link integrity (mdbook build docs, mdbook-linkcheck2)
make test     # cargo nextest run --locked --workspace --all-features --no-tests=pass, under -Dwarnings
make lint     # clippy --workspace --all-targets --all-features -- -D warnings
make fmt      # nightly cargo fmt --all
make source   # build the mock adapter example (wasm32-wasip2, release) for the live journey
make runtime  # build the cursor-bound runtime example
make sweep    # drop target/ artifacts untouched for a week (cargo-sweep); cargo never collects them itself
```

Local Cursor preview of the skill wrapper: `cursor-agent --plugin-dir plugins/emery` (see [docs/contributing/operator-plugins.md](docs/contributing/operator-plugins.md)).

## Gotchas

- **Path and package adapters load dynamically; bare names stay static.** `emery specify <adapter>` accepts a package reference (`emery:intent@1.0.0`), the first-party shorthand (`intent@1.0.0` — sugar for the `emery` namespace), a bare name, or a project-relative local `.wasm` path. A local `.wasm` loads through the deployment's `omnia:plugins/loader` capability on every run that names it — read fresh (nothing mirrors, a deleted file refuses typed). A package reference fetches from the binding's `registry` override or the compiled-in `omnia.host` default on every run that names it — read fresh, no project cache — and registers under the package reference itself — no parallel `source:` id. Either load is optionally pinned by the binding's `digest`, with the resolved digest reported in the envelope (the journey host in `examples/runtime.rs` declares a path-only `locations:` list and loads the built mock component by path). Bare names still dispatch only guests declared in the runtime invocation (`src/lib.rs`); dispatch beyond the declared set fails at dispatch. GitHub URLs are refused.
- Never hand-edit `.emery/` state (the revision store); never `mkdir -p .emery/...`. Route through the CLI. The binding list is per-run input — argv, or an operator-owned `emery.toml` named by `--config` (or discovered at the project root by a run naming no bindings at all); the engine never writes it.
- `make links` enforces Developer Guide link integrity — renaming docs paths requires updating links in the same change.
- Crossing a major is a hard cut: no silent compatibility aliases and no migration framework. Pre-1.0, a major bump means regenerating with a fresh `emery specify`.
- Brevity caps (identifier ≤ 25, comment density) are review-only; see [coding-standards.md](docs/standards/coding-standards.md). WIT doc/comment caps are the same rule.

## Related coding standards

- The external Rust baseline is the [Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/guidelines/index.html); [docs/standards/](docs/standards/) carries the house deltas (overrides win).
- Cross-cutting rules: [style.md](docs/standards/style.md). Lints, comments, DTOs, module layout: [coding-standards.md](docs/standards/coding-standards.md). Operation/router shape and exit mapping: [handler-shape.md](docs/standards/handler-shape.md). Workspace layout and deployment split: [architecture.md](docs/standards/architecture.md). Skill/CLI split and the JSON envelope: [cli-contract.md](docs/standards/cli-contract.md). Docs house style: [doc-authoring.md](docs/standards/doc-authoring.md).
- Lint suppressions: refactor first; `#[expect(lint, reason = "…")]` at the smallest scope; state in the PR why a refactor was infeasible.
- When you remove a symbol, `rg <SymbolName> -- AGENTS.md docs/` and update every hit in the same PR.
- Markdown changes follow [doc-authoring.md](docs/standards/doc-authoring.md); do not hard-wrap prose solely for column width.

Run `make ci` before committing. If it cannot run, say exactly why and which checks were run instead.
