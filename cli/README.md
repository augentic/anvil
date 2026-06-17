# Specify CLI

## Installing the CLI

The `specify` binary is required by `spec` plugin skills. It is the single CLI for the whole workflow, including the framework authoring lint (`specify lint framework`). Install using:

```bash
cargo install --git https://github.com/augentic/specify-cli

# brew install augentic/tap/specify         # macOS + Linux (primary)
```

Once installed, keep the binary current with `specify upgrade`. It detects
its install channel (`cargo` / `brew` / `binary`), resolves the latest
release, and self-updates after `--yes` (or previews with `--dry-run`).

## Shell completions

`specify completions <shell>` writes a completion script to stdout for any
clap-supported shell (`bash`, `elvish`, `fish`, `powershell`, `zsh`). For
example:

```bash
specify completions zsh > "${fpath[1]}/_specify"   # zsh
specify completions bash > /etc/bash_completion.d/specify  # bash
```

The script is generated from the live clap surface, so it stays in sync
with every verb the binary exposes.
