# CLI Reference

The `emery` CLI owns all deterministic operations. Under the remediation programme (ADR-0008) the grammar is two verbs plus the auto-derived `completions` — deleted verbs are gone from the grammar, not hidden.

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/augentic/emery/main/scripts/install.sh | sh

# or: cargo install --git https://github.com/augentic/emery --locked
```

## Conventions

- All commands return structured output on stdout and use exit codes for success/failure; `--format json` selects the JSON envelope (see [CLI output shapes](../cli-output-shapes.md)).
- Commands that modify `.emery/` state are idempotent where possible.
- Skills delegate to the CLI for all structural operations — they never hand-edit `.emery/` state directly.

## Commands

| Verb | Purpose |
|------|---------|
| [emery init](init.md) | Project scaffold over the authored source bindings; `--upgrade` re-entry bumps the version pin |
| `emery specify` | Generate `spec.md` / `design.md` from the bound sources and commit them behind the generation pointer (ADR-0008, ADR-0009) |
| `emery completions <shell>` | Print a shell-completion script; auto-derived from the live clap surface |
