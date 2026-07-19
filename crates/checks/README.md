# checks

Repo invariants enforced as plain cargo tests: adapter/engine dependency boundary and plugin authoring shape. Developer Guide link integrity is mdBook's job (`cargo make links` / `mdbook build docs`, config in `docs/book.toml`). See [Consistency Checks](../../docs/contributing/checks.md).

```bash
cargo test -p checks
```
