# Consistency Checks

Repo invariants that are cheap to enforce in CI and expensive to notice later. Two enforcement surfaces: the typed invariants live as plain cargo tests in the lightweight [`crates/checks`](../../crates/checks/) package, and docs/plugin link integrity is delegated to [lychee](https://github.com/lycheeverse/lychee) via [`lychee.toml`](../../lychee.toml). Both run inside `cargo make ci`.

```bash
cargo test -p checks          # adapter boundary + plugin authoring shape
cargo make links              # docs/plugin link integrity (lychee)
cargo make ci                 # the full gate
```

Skill frontmatter shape and the marketplace manifest are enforced by the typed `authoring` check in the `checks` package ([`crates/checks/authoring.rs`](../../crates/checks/authoring.rs)); docs house style is **not** a CI predicate, and ultrathin skill body style is guidance in [`docs/standards/cli-contract.md`](../standards/cli-contract.md).

## Who owns which invariant

| Invariant                                       | Owner                                                                                                            | When it runs                            |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| Adapter dependency boundary, plugin authoring   | `crates/checks`                                                                                                  | Every `cargo make ci`                   |
| Docs/plugin link integrity                      | lychee over [`lychee.toml`](../../lychee.toml) — `cargo make links` locally, the `links` job in [`ci.yaml`](../../.github/workflows/ci.yaml) per push | Every `cargo make ci` and every push    |
| Embedded prompt-corpus links                    | `crates/prose` via `crates/slice/build.rs` + `crates/change/build.rs` — a dangling reference **fails the build** | Every compile                           |

The `checks` package stays a separate workspace member (not root-package tests) so the Wasmtime-heavy runtime graph stays out of the ordinary test build. Cross-crate test support comes from the `crates/linked` and `crates/fixture` dev-dependencies; fixtures live crate-locally under `crates/<name>/tests/fixtures/`.

## What the checks enforce

### `boundaries.rs`

No engine Cargo manifest (workspace root, `crates/`, `examples/`) may depend on a concrete adapter crate or reach into `specify-adapters` via a `path`/`git` source. The engine talks to adapters only through the WASM component seam.

### Links (lychee)

Every relative link under `plugins/` and `docs/` must resolve on disk. Web links and fenced/inline code are skipped (`offline = true`); per-file carve-outs live in [`lychee.toml`](../../lychee.toml). This one checker is the PR-time gate for `docs/` links — the published book runs no separate link check.

Judgment prose under `crates/slice/prompts/` and `crates/change/prompts/` is out of scope: embed-time link-check in `crates/prose` fails the build on a dangling reference.

## Extending

Add a `[[test]]` target under [`crates/checks/Cargo.toml`](../../crates/checks/Cargo.toml) only when failure breaks a shipped artifact or an architecture contract and no off-the-shelf tool covers it. House-style preferences belong in `docs/standards/` as guidance, not here.

## CLI checks

```bash
cargo make ci     # fmt + clippy + all tests (incl. checks) + docs + vet + deny
cargo make check  # the pre-commit subset
```

Per-push CI ([`.github/workflows/ci.yaml`](../../.github/workflows/ci.yaml)) needs no sibling checkout.
