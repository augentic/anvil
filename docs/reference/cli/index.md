# CLI Reference

The `emery` CLI owns all deterministic operations. Under the remediation programme (ADR-0008) the grammar is two verbs plus the auto-derived `completions` — deleted verbs are gone from the grammar, not hidden.

## Installation

```bash
brew tap augentic/tap
brew install emery

# or: cargo install --git https://github.com/augentic/emery --locked
```

## Conventions

- All commands return structured output on stdout and use exit codes for success/failure; `--format json` selects the JSON envelope (see [CLI output shapes](../cli-output-shapes.md)).
- Commands that modify `.emery/` state are idempotent where possible.
- Skills delegate to the CLI for all structural operations — they never hand-edit `.emery/` state directly.

## Commands

| Verb | Purpose |
|------|---------|
| [emery specify](specify.md) | Generate `spec.md` / `design.md` from the sources named on the invocation and commit them behind the generation pointer (ADR-0008, ADR-0009) |
| [emery show](show.md) | Print a reviewable document of the current generation to stdout |
| `emery completions <shell>` | Print a shell-completion script; auto-derived from the live clap surface |
