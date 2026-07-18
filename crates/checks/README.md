# checks

Repo invariants enforced as plain cargo tests: adapter/engine dependency boundary and plugin authoring shape. Docs/plugin link integrity is lychee's job (`cargo make links`, config in the repo-root `lychee.toml`). See [Consistency Checks](../../docs/contributing/checks.md).

```bash
cargo test -p checks
```
