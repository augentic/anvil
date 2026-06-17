## Summary

<!-- What changed and why -->

## Rust checklist

See [AGENTS.md § Rust quality](https://github.com/augentic/specify-cli/blob/main/AGENTS.md#rust-quality).

- [ ] `cargo make ci` (or documented subset) green
- [ ] No new `#[allow]`; any new `#[expect]` has `reason` and a refactor was attempted
- [ ] Test fn names short; no new archaeology in module docs
- [ ] Wire / error `code` changes called out below (if any)

## Test plan

<!-- How you verified -->
