# Running Acceptance

The acceptance surface is intentionally small. RM-01 is covered by one direct
Deno test at [`tests/cross_repo.ts`](../../tests/cross_repo.ts).
It uses local fixture repositories and fake `gh`/SSH helpers, then drives the
real `specify` CLI through the cross-repo happy path.

## Targets

- `make checks` runs static repository checks.
- `make test` runs the RM-01 test.

## What RM-01 Proves

The test creates a fresh temp workspace with:

- a registry-only `shop-platform` hub,
- `shop-backend` and `shop-mobile` fixture repos backed by local bare remotes,
- fake `gh` and fake SSH,
- the OAuth login fixture brief.

It then asserts the durable RM-01 behavior directly: registry setup, workspace
sync, a three-entry contract-first plan, routed execution on `specify/oauth-login`
branches, baseline/residue commit split, workspace push, fake external merge,
`change finalize`, archived plan state, and `plan-not-found` on a second finalize.

The test deliberately does not keep a backend registry, recorded trace layer, or
separate per-stage smoke targets. Rust CLI mechanics remain owned by
`specify-cli/tests/cross_repo.rs`; this repo keeps only the smallest cross-repo
workflow proof needed for RM-01.

## Setting `SPECIFY_BIN`

`make test` resolves `specify` in this order:

1. `$SPECIFY_BIN`
2. `specify` on `PATH`

If neither is available, or the binary predates the RM-01 surface, the Deno test
prints a skip message and exits cleanly.

```bash
# In the specify-cli checkout:
cargo build --release

# In this repo:
export SPECIFY_BIN=/absolute/path/to/specify-cli/target/release/specify
make test
```

Set `SPECIFY_ACCEPTANCE_PRESERVE=1` to keep the temp fixture directory after a
passing run. Failed runs are preserved automatically and print their location.
