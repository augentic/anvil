# framework_adapters_only

Tiny **adapters-only** framework-repo fixture (RFC-48 H1): an
`adapters/` tree with no `plugins/` directory and no
`.cursor-plugin/marketplace.json`. It mirrors `framework_minimal`
otherwise — one source adapter (`intent`) and one target adapter
(`omnia`), each with a minimal `adapter.yaml` and a brief — so the
framework profile still surfaces adapter and brief facts.

It exists so a framework lint over an adapters-only root can be
exercised: the plugin-bound `marketplace` and `prose` checkers must
no-op on their absent inputs, and the root still resolves as a valid
framework root because `adapters/` is present.

The `agent-teams.md` symlink, if needed, is **minted at test time**
rather than committed because relative symlinks survive `git` poorly
across operating systems.
