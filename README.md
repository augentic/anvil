# Emery

[CI](https://github.com/augentic/emery/actions/workflows/ci.yaml)
[License: MIT OR Apache-2.0](#license)
[Docs](https://emery.augentic.io/)

Spec-driven development in [Cursor](https://cursor.com): plan a change, approve it, then `refine → build → merge` each slice from durable artifacts — not chat history.

Emery uses **Source Adapters** (like `intent` or `typescript`) to ingest requirements and **Target Adapters** (like `contracts` or `omnia`) to generate outputs.

## Quick start

### Prerequisites

1. [Cursor](https://cursor.com) with the **Augentic** marketplace plugin installed (**Settings → Plugins** → search Augentic → install → restart). That gives you the `/emery:*` skills.
2. Optional: the `emery` CLI on `PATH` (the plugin can install it on `/emery:init`):

```bash
# Prebuilt binary
cargo binstall --git https://github.com/augentic/emery emery@0.28.0

# Homebrew
export HOMEBREW_GITHUB_API_TOKEN="$(gh auth token)"
brew tap augentic/tap && brew install emery

# Build from source
cargo install --git https://github.com/augentic/emery --locked
```

Verify installation:

```bash
emery --version
```

### First change

This path uses the **Contracts** target (`contracts@0.5.0`). The pin pulls a published adapter from GHCR; you do not need to clone `emery-adapters`.

**Option A: Cursor Agent**
In Cursor Agent chat, in a fresh or disposable repository:

```text
/emery:init contracts@0.5.0
/emery:plan first-contract source intent=intent@0.5.0:value:"Author an HTTP API contract for a health endpoint that returns status and version."
/emery:execute
/emery:finalize first-contract
```

**Option B: Terminal CLI**
The same steps run manually:

```bash
emery init contracts@0.5.0
emery plan author first-contract \
  --source intent=intent@0.5.0:value:"Author an HTTP API contract for a health endpoint that returns status and version."
# review change.md, discovery.md, plan.yaml  ← Gate 1
emery plan approve
emery plan execute
emery plan status     # must be drained
emery plan archive    # after you publish via git
```

What you should see after execute: slice artifacts under `.emery/slices/…`, generated files under `contracts/`, and merged baseline specs under `.emery/specs/`.

## How it works: The rhythm

```text
/emery:init  →  /emery:plan  →  Gate 1 (approve)  →  /emery:execute  →  /emery:finalize
                                         │
                                         └─ per slice: refine → build → merge
```

Gate 1 is the operator review step: nothing runs until you stamp the plan `approved`. A one-slice change uses the same steps as a twelve-slice migration. Code generation lives in target adapters, not in Cursor skills.

When `plan execute` parks, or you want to drive one slice without the drained loop, you can use the breakout skills: `/emery:refine`, `/emery:build`, `/emery:merge`, and `/emery:drop` (or their CLI equivalents).

## Documentation & Guides

- **Quick Start Tutorial:** [Guided Omnia walkthrough](docs/tutorials/quick-start.md) · [hosted](https://emery.augentic.io/tutorials/quick-start.html)
- **Command Lookup:** [Quick reference](docs/reference/quick-reference.md)
- **Core Concepts:** [What is Emery?](docs/orientation/index.md) · [Core concepts](docs/explanation/concepts.md) · [AGENTS.md § Workflow nouns](AGENTS.md#workflow-nouns)
- **Installation:** [Prerequisites](docs/orientation/prerequisites.md)
- **How-tos:**
  - [Drive a slice manually](docs/how-to/drive-slice-manually.md)
  - [Amend a plan at Gate 1](docs/how-to/amend-plan-at-gate-1.md)
  - [Drop down a layer](docs/how-to/drop-down-a-layer.md) (when automation fails)
  - [Bind multiple sources](docs/how-to/bind-multiple-sources.md)
  - [Resolve spec conflicts](docs/how-to/resolve-spec-conflicts.md)
- **Troubleshooting:** [GitHub Issues](https://github.com/augentic/emery/issues)
- **Full Developer Guide:** [emery.augentic.io](https://emery.augentic.io/) · [In-tree book source](docs/SUMMARY.md)

## Developing Emery (contributors)

The repository root is a Rust workspace producing the `emery` binary. The root `Makefile` forwards every goal to [cargo-make](Makefile.toml).

```bash
make test    # native integration suite
make check   # format, lint, tests, doctests, and docs
make ci      # full pre-commit gate, including vet and deny
```

Use `make eval` only for changes to engine prompts or answer schemas; it requires model credentials. `make lab -- ARGS` is the native mock-catalog lab shim, not the installed operator CLI.

Preview the working-tree Cursor skills against a local CLI:

```bash
cursor-agent --plugin-dir plugins/emery
```

Start with the [developer loop](docs/contributing/dev-loop.md), then [Cursor operator plugins](docs/contributing/operator-plugins.md) and [CONTRIBUTING.md](CONTRIBUTING.md). See also [GOVERNANCE.md](GOVERNANCE.md) and [Code of Conduct](CODE-OF-CONDUCT.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
