# Consistency Checks

Repo invariants that are cheap to enforce in CI and expensive to notice later: the adapter/engine dependency boundary, and relative link integrity under `docs/` / `plugins/`. They live as plain cargo tests in the lightweight [`tests/`](../../tests/) package (`checks`) and run inside `cargo make ci`.

```bash
cargo test -p checks          # boundaries + docs/plugin links
cargo make ci                 # the full gate
```

Skill frontmatter shape and the marketplace manifest are enforced by the typed `authoring` check in the `checks` package ([`tests/authoring.rs`](../../tests/authoring.rs)); docs house style is **not** a CI predicate, and ultrathin skill body style is guidance in [`docs/standards/cli-contract.md`](../standards/cli-contract.md).

## Who owns which invariant

| Invariant                                      | Owner                                                                                                            | When it runs                                          |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| Adapter dependency boundary, docs/plugin links | `tests/` (`checks` package)                                                                                      | Every `cargo make ci`                                 |
| Embedded prompt-corpus links                   | `crates/prose` via `crates/slice/build.rs` + `crates/change/build.rs` — a dangling reference **fails the build** | Every compile                                         |
| Published docs book links                      | mdbook-linkcheck2 in [`.github/workflows/docs.yaml`](../../.github/workflows/docs.yaml)                          | Push to `main` for `docs/**` only — **not** a PR gate |

The `checks` package stays a separate workspace member (not root-package tests) so the Wasmtime-heavy runtime graph stays out of the ordinary test build. `tests/fs_git.rs` and shared `tests/fixtures/` trees sit alongside and are pulled in by other crates via `#[path]` / relative paths; crate-local fixtures live under `crates/<name>/tests/fixtures/`.

## What the checks enforce

### `boundaries.rs`

No engine Cargo manifest (workspace root, `crates/`, `harness/`) may depend on a concrete adapter crate or reach into `specify-adapters` via a `path`/`git` source. The engine talks to adapters only through the WASM component seam.

### `links.rs`

Every relative link under `plugins/` and `docs/` must resolve on disk. External links and fenced/inline code are skipped. Missing `.svg` embeds under `docs/` fail too. This is the only PR-time gate for `docs/` links — mdBook linkcheck runs post-merge only.

Judgment prose under `crates/slice/prompts/` and `crates/change/prompts/` is out of scope: embed-time link-check in `crates/prose` fails the build on a dangling reference.

## Extending

Add a `[[test]]` target under [`tests/Cargo.toml`](../../tests/Cargo.toml) only when failure breaks a shipped artifact or an architecture contract. House-style preferences belong in `docs/standards/` as guidance, not here.

## CLI checks

```bash
cargo make ci     # fmt + clippy + all tests (incl. checks) + docs + vet + deny
cargo make check  # the pre-commit subset
```

Per-push CI ([`.github/workflows/ci.yaml`](../../.github/workflows/ci.yaml)) needs no sibling checkout. WASM boundary execution lives in the weekly/path-filtered [`.github/workflows/wasm.yaml`](../../.github/workflows/wasm.yaml).
