# Summary

<!-- What and why, in a sentence or two. -->

## Testing checklist

Placement rules: [docs/standards/testing.md](../docs/standards/testing.md). The default write path is crate or wire integration; a `src` unit test is the exception.

- [ ] Every new assertion names the layer that owns it: kernel unit (`src` `#[cfg(test)]`), crate integration (`crates/<name>/tests/`), or wire contract (`crates/transport/tests/`).
- [ ] Any new `src` `#[cfg(test)]` carries a one-line **Keep** or **Collapse** reason (CLI-unreachable branch, private kernel with no public projection, or dense pure matrix).
- [ ] No `pub` / `pub(crate)` was widened solely to make a test reachable, and no test-only trait pairs were added.
- [ ] If unit coverage was deleted or re-homed, the coverage brake ran before and after (`CRATE=<crate> cargo make cov`) and `TOTAL` held on still-live code.
- [ ] `cargo make ci` passes (or the PR states exactly which narrower checks ran and why).

## DCO

- [ ] All commits carry a `Signed-off-by` line (`git commit -s`).
