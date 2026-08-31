# Emery - Agent Instructions

> **Remediation programme in flight.** This file maps the reduced Phase 0 tree — what exists after the freeze, never a spec of what to build. Feature work is frozen until the spec walking skeleton is green.
>
> The v1 implementation (survey + extract, plan/refine/execute, target adapters, the definition loop) is archived at git tag `v1`; retrieve it with `git worktree add ../emery-v1 v1`. First-party adapters live in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters) — cut to extract-only in Phase 4 (its `v1` tag keeps the survey-era tree).

## What this repository is now

A Rust workspace at the repository root producing the `emery` runtime binary, plus one Cursor plugin (`plugins/emery/` carrying the `/emery:specify` skill wrapper). The live CLI grammar is three verbs:

- `emery specify <adapter>... [--description <adapter>=<text>] [--config [<path>]]` — the spec generator: resolve the sources named on the invocation (a project-relative local `.wasm` loads through the deployment's `omnia:plugins/loader` capability, read fresh each run; an exact package reference fetches from the binding's `registry` override or the compiled-in `omnia.host` default; either way the binding's optional `digest` pin is verified host-side and the resolved digest rides the success envelope), extract each binding over the `Source` capability, reconcile under authority precedence, synthesise, and commit `spec.md` / `design.md` as one generation behind the swapped `current` pointer. `--config` without a value explicitly selects `emery.toml`; a run naming no bindings at all discovers the project-root `emery.toml` as a fallback — never merged with argv bindings. The binding list is per-run input, never persisted; a re-run reports the re-mine diff against the superseded generation in the success envelope — never persisted.
- `emery show <spec|design>` — print a reviewable document of the current generation to stdout; text mode is the document body alone, and the generation id rides the JSON envelope.
- `emery completions <shell>` — auto-derived from the clap surface.

Deleted verbs are deleted from the grammar, not hidden — there are no compatibility aliases or deprecated stubs. The HTTP surface is deleted outright — no listener, no MCP shelves — so C3 (no unauthenticated HTTP ingress) is satisfied by absence rather than refusal. Adapter references reach the model through the completion session's tool closure (`list_docs` / `read_doc`, answered in-process from the embedded corpus), and the committed generation is reviewed over `emery show`.

## Vocabulary

- **source adapter** — input role: one WebAssembly component exporting the WIT `source-adapter` world (`extract` + `metadata`; no manifest file). `extract` takes a typed `SourceInput` (`key`, workspace-or-value) and returns an Evidence document of typed claims — the spec IR (A8/A16; required extras are fail-closed engine-side). Survey, leads, and the target axis were deleted from the WIT contract (archived at `v1`); see [wit/emery.wit](wit/emery.wit).
- **engine** — this product: the engine guest (`emery:engine`), the surviving engine crates, and the engine side of the adapter contract.
- **capability** — an engine-side trait a provider carries (`Source`, `Model`, `Plugins`, `StateStore` / `BlobStore`).
- **contract** — the typed agreement (WIT, CLI, wire).
- **boundary** — the crossing you test or bind (entry points, host bindings).
- **plugin** (adapter vocabulary) vs **Cursor plugins** — do not confuse the adapter noun with `plugins/emery/`, the IDE distribution surface for the `/emery:specify` skill wrapper; the latter is invisible to the `emery` CLI.

Artifact authority is unchanged in spirit: when authoritative inputs are incomplete, preserve the gap as `[unknown]` rather than guessing.

## The Rust workspace

Leaf → root. Each publishing package is `emery-<crate>` on crates.io; Rust `use` paths follow the package name (`emery_adapter::`, `emery_engine::`, …). The root package and `emery-testkit` stay `publish = false`.

```text
adapter      # the adapter SDK — the SourceAdapter operations trait (extract + metadata + docs), the WIT package + source! export macro, the Source capability (wasm32 defaults over the engine guest's source::import wrappers; bare natively so tests script Source), WIT types, embedded prose registry + the reference tool closure (list_docs / read_doc over the embedded docs)
engine       # the spec generator — per-run source bindings (argv + emery.toml loaders + root discovery), specify + show handlers, extract leg (resolve + required-extras gate) over the provider's Source capability, reconcile/synthesise (embedded synthesis prose), the fail-closed spec AST, the generation-pointer output home; plus the ported kernels: emery_engine::resolve (selector parsing, loader-backed local-component loading over the Plugins capability, the adapter floor gate) and emery_engine::handler (preopen-relative path normalization, Render); plus the CLI surface (emery_engine::cli — clap grammar over Handler inputs: specify + show + completions, Client dispatch, output projection, the exit contract)
prose        # build-dependency crate — embed-time prompt-corpus walk + link check
testkit      # unpublished — scripted capability doubles for native tests: the FIFO request-recording `Scripted` model plus the StateStore/BlobStore pair (`Memory`, `Namespaced`); not a production crate
emery (root) # Omnia deployment unit under src/: wasm32 engine guest cdylib (src/lib.rs — bare model provider, wasi:cli/run) + shipped runtime (src/main.rs, one omnia::runtime! embedding $OUT_DIR/emery.cwasm; static, CWD-rooted deployment policy inline — the invocation directory mounts read-only as `.`, the wasi:keyvalue/wasi:blobstore hosts bind the generation state to the durable omnia-filesystem store (compiled-in root `.omnia/storage`), and the plugins block declares the source seam with the declarative locations list: the `.` path root, so local `.wasm` adapters load fresh each run, then the omnia.host registry endpoint with cache: Filesystem — the project CAS under the same durable store at `.omnia/storage/plugins/` — for exact package references; bare names still dispatch statically declared guests)
```

### Repository map

```text
src/               shipped binary (omnia::runtime!, static CWD-rooted deployment, filesystem-backed keyvalue/blobstore hosts) + wasm32 engine guest cdylib (bare model provider, wasi:cli/run)
crates/            the workspace crates above
examples/          mock source adapter + static runtime host (root-package examples)
wit/               the emery:adapter WIT package (source-adapter world) + README
plugins/emery/     Cursor plugin: /emery:specify skill wrapper, rules, manifest
docs/              Developer Guide (mdBook; reference + contributing + standards only)
```

### Exit codes

`exit_code` in [`crates/engine/src/cli.rs`](./crates/engine/src/cli.rs) maps `omnia_guest::Error` variants and is the single source of truth.

| Code | Name | When |
| ---- | ------------------------ | ------------------------------------------------------------------ |
| 0 | `EXIT_SUCCESS` | Command succeeded. |
| 1 | `BadRequest` | Operator or input refusal. The `error` field is `specify-source-required`, `adapter-cli-too-old`, a loader refusal (`digest-mismatch`, `invalid-digest`, `artifact-refused`, `seam-missing`, `already-active`, `location-unsupported`), or the Omnia default `bad_request`. |
| 2 | `NotFound` | Missing resource. The `error` field is `spec-not-generated` or the Omnia default `not_found`. Clap usage and unknown-verb also exit 2 (framework). |
| 3 | `ServerError` | Unclassified default: I/O, storage, leftover conversions. The `error` field is the Omnia default `server_error`. |
| 4 | `BadGateway` | Upstream, model, or component-acquisition failure. The `error` field is the Omnia default `bad_gateway` or the loader's `acquire-failed`. |

Omnia default codes are snake_case (`bad_request`, `not_found`, `server_error`, `bad_gateway`). The recovery and loader discriminants stay kebab-case so skills can branch on them.

## Testing philosophy

Emery strictly enforces a **root-led integration posture** (DWN-style):

- The root `tests/` scenario suites are the default home for every CLI-reachable behavior: `tests/specify.rs` (the `specify` → `show` product arc), `tests/command.rs` (the CLI wire contract), and `tests/plugin.rs` (plugin-rule mentions vs the shipped grammar) drive the in-process command router over scripted capabilities (`tests/support/mod.rs`) and read like usage documentation.
- Crate suites in `crates/<name>/tests/` survive only for independently useful library contracts (the adapter SDK, prose) or product invariants impractical to arrange through the entry points; unit tests are near-zero, reserved for genuinely CLI-unreachable branches.
- Default to deletion; do not widen public APIs to test private kernels. `make cov` (workspace-wide) is the brake; `CRATE=emery-<crate> make cov-crate` audits one leaf contract.
- One fast rung: the native suites (`make test`, per push) over scripted `Model` + `Source` + `Plugins` + storage (`StateStore`/`BlobStore`); engine state is asserted through the scripted store and the envelope, never the filesystem. The v1 eval and wasm-example rungs are archived at `v1`. No test builds or spawns the mock source component.

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
make source   # build the mock source example (wasm32-wasip2, release)
make runtime  # build the scripted-model runtime example
```

Local Cursor preview of the skill wrapper: `cursor-agent --plugin-dir plugins/emery` (see [docs/contributing/operator-plugins.md](docs/contributing/operator-plugins.md)).

## Gotchas

- **Path and package adapters load dynamically; bare names stay static.** `emery specify <adapter>` accepts a package reference (`emery:intent@1.0.0`), the first-party shorthand (`intent@1.0.0` — sugar for the `emery` namespace), a bare name, or a project-relative local `.wasm` path. A local `.wasm` loads through the deployment's `omnia:plugins/loader` capability on every run that names it — read fresh (nothing mirrors, a deleted file refuses typed). A package reference fetches from the binding's `registry` override or the compiled-in `omnia.host` default, cached in the project CAS (`.omnia/storage/plugins/`, inside the durable filesystem store), and registers under the package reference itself — no parallel `source:` id. Either load is optionally pinned by the binding's `digest`, with the resolved digest reported in the envelope (the journey host in `examples/runtime.rs` declares a path-only, cacheless `locations:` list and loads the built mock component by path). Bare names still dispatch only guests declared in the runtime invocation (`src/main.rs`); dispatch beyond the declared set fails at dispatch. GitHub URLs are refused.
- Never hand-edit `.emery/` state (the generation store); never `mkdir -p .emery/...`. Route through the CLI. The binding list is per-run input — argv, or an operator-owned `emery.toml` named by `--config` (or discovered at the project root by a run naming no bindings at all); the engine never writes it.
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
