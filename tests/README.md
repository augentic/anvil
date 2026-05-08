# Tests

This directory keeps the small amount of automation needed by the `specify`
repo itself.

## RM-01 Cross-Repo Test

[`cross_repo.ts`](cross_repo.ts) is the direct acceptance
proof for the roadmap's RM-01 item. It creates a fresh temp hub, two local
fixture projects, local bare remotes, and fake `gh`/SSH, then drives the real
`specify` binary through:

1. hub and registry setup,
2. workspace sync,
3. a three-entry contract-first plan,
4. routed slice execution with baseline/residue commits,
5. workspace push,
6. fake external PR merge,
7. `change finalize`.

Run it with:

```bash
make test
```

Set `SPECIFY_BIN=/path/to/specify` to use a freshly built CLI. The test skips
cleanly when no suitable binary is available.

## Support Files

- `fixtures/rm01/oauth-login.md` is the concise feature brief used by the test.
- `support/` contains small reusable helpers for local Git, fake `gh`, fixture
  projects, workspace sync, and invoking the `specify` CLI.

Capability-owned manual scenario documents still live beside their owners, for
example `capabilities/contracts/tests/`.

