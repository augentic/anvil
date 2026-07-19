# Consistency Checks

Repo invariants that are cheap to enforce in CI and expensive to notice later. Two enforcement surfaces: the typed invariants live as plain cargo tests in the lightweight [`crates/checks`](../../crates/checks/) package, and Developer Guide link integrity is the mdBook build (`mdbook-linkcheck2` via [`docs/book.toml`](../book.toml)). Both run inside `cargo make ci`.

```bash
cargo test -p checks          # adapter boundary + plugin authoring shape
cargo make links              # Developer Guide link integrity (mdbook build)
cargo make ci                 # the full gate
```

Skill frontmatter shape and the marketplace manifest are enforced by the typed `authoring` check in the `checks` package ([`crates/checks/authoring.rs`](../../crates/checks/authoring.rs)); docs house style is **not** a CI predicate, and ultrathin skill body style is guidance in [`docs/standards/cli-contract.md`](../standards/cli-contract.md).

## Who owns which invariant

| Invariant                                       | Owner                                                                                                            | When it runs                            |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| Adapter dependency boundary, plugin authoring   | `crates/checks`                                                                                                  | Every `cargo make ci`                   |
| Developer Guide link integrity                  | `mdbook-linkcheck2` over [`docs/book.toml`](../book.toml) — `cargo make links` locally, the `links` job in [`ci.yaml`](../../.github/workflows/ci.yaml) per push | Every `cargo make ci` and every push    |
| Embedded prompt-corpus links                    | `crates/prose` via `crates/slice/build.rs` + `crates/change/build.rs` — a dangling reference **fails the build** | Every compile                           |

The `checks` package stays a separate workspace member (not root-package tests) so the Wasmtime-heavy runtime graph stays out of the ordinary test build. Cross-crate test support comes from the `crates/native` and `crates/mock` dev-dependencies; fixtures live crate-locally under `crates/<name>/tests/fixtures/`.

## What the checks enforce

### `boundaries.rs`

No engine Cargo manifest (workspace root, `crates/`, `examples/`) may depend on a concrete adapter crate or reach into `specify-adapters` via a `path`/`git` source. The engine talks to adapters only through the WASM component seam.

### Links (mdBook)

Every relative link in the Developer Guide must resolve. Web links are skipped (`follow-web-links = false`); links that leave `docs/` (for example into `crates/` or `AGENTS.md`) are allowed (`traverse-parent-directories = true`). Chapters referenced as in-book targets must appear in `SUMMARY.md`; prefer file hrefs over bare directory paths.

Judgment prose under `crates/slice/prompts/` and `crates/change/prompts/` is out of scope: embed-time link-check in `crates/prose` fails the build on a dangling reference.

## Extending

Add a `[[test]]` target under [`crates/checks/Cargo.toml`](../../crates/checks/Cargo.toml) only when failure breaks a shipped artifact or an architecture contract and no off-the-shelf tool covers it. House-style preferences belong in `docs/standards/` as guidance, not here.

## CLI checks

```bash
cargo make ci     # fmt + clippy + all tests (incl. checks) + docs + vet + deny
cargo make check  # the pre-commit subset
```

Per-push CI ([`.github/workflows/ci.yaml`](../../.github/workflows/ci.yaml)) needs no sibling checkout.
