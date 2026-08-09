# Emery

[CI](https://github.com/augentic/emery/actions/workflows/ci.yaml)
[License: MIT OR Apache-2.0](#license)
[Docs](https://emery.augentic.io/)

Spec-driven development in [Cursor](https://cursor.com): plan a change, approve it, then `refine → build → merge` each slice from durable artifacts — not chat history.

Emery uses **Source Adapters** (like `intent` or `typescript`) to ingest requirements and **Target Adapters** (like `contracts` or `omnia`) to generate outputs.

## Quick start

Starting fresh? Follow the steps below. Migrating an existing codebase? See [Migrate a legacy service](docs/tutorials/migrate-a-legacy-service.md) · [hosted](https://emery.augentic.io/tutorials/migrate-a-legacy-service.html).

### Prerequisites

1. [Cursor](https://cursor.com) with the **Augentic** marketplace plugin installed (**Settings → Plugins** → search Augentic → install → restart). That gives you the `/emery:`* skills.
2. Optional: the `emery` CLI on `PATH` (the plugin can install it on `/emery:init`):

```bash
# prebuilt binary (verifies the Release archive's sha256, installs to ~/.local/bin)
curl -fsSL https://raw.githubusercontent.com/augentic/emery/main/scripts/install.sh | sh
```

```bash
# from source
cargo install --git https://github.com/augentic/emery --tag v0.32.0 --locked
```

Verify installation:

```bash
emery --version
```



### First change

This path uses the **Contracts** target. A bare adapter name resolves local-first and pulls the newest published version from GHCR when nothing local exists; you do not need to clone `emery-adapters`.

**Option A: Cursor Agent**
In Cursor Agent chat, in a fresh or disposable repository:

```text
/emery:init contracts
/emery:plan first-contract
```

When prompted, give a one-line intent such as: `Author an HTTP API contract for a health endpoint that returns status and version.` Then:

```text
/emery:execute
/emery:finalize first-contract
```

**Option B: Terminal CLI**
The same steps run manually:

```bash
emery init contracts
emery plan author first-contract \
  --intent "Author an HTTP API contract for a health endpoint that returns status and version."
# review change.md, discovery.md, plan.yaml
emery plan execute   # opens authorization epoch; drives the loop
emery plan status     # must be drained
emery plan archive    # after you publish via git
```

What you should see after execute: slice artifacts under `.emery/slices/…`, generated files under `contracts/`, and merged baseline specs under `.emery/specs/`.

## How it works: The rhythm

```text
/emery:init  →  /emery:plan  →  review  →  /emery:execute  →  /emery:finalize
                                    │
                                    └─ per slice: refine → build → merge
```

The pause after planning is the operator review step: nothing privileged runs until you invoke `emery plan execute` — that journals `plan.execute.started` and drives the loop under gap gates. A one-slice change uses the same steps as a twelve-slice migration. Code generation lives in target adapters, not in Cursor skills.

When `plan execute` parks, the stop card names the reason and the resume command — fix the input it points at and re-run `emery plan execute`; the loop resumes at the parked phase. Abandon a slice with `emery plan drop <entry>`.

## Documentation & Guides

- **Quick Start Tutorial:** [Guided Omnia walkthrough](docs/tutorials/quick-start.md) · [hosted](https://emery.augentic.io/tutorials/quick-start.html)
- **Migrate a legacy service:** [TypeScript → Omnia walkthrough](docs/tutorials/migrate-a-legacy-service.md) · [hosted](https://emery.augentic.io/tutorials/migrate-a-legacy-service.html)
- **Command Lookup:** [Quick reference](docs/reference/quick-reference.md)
- **Core Concepts:** [What is Emery?](docs/orientation/index.md) · [Core concepts](docs/explanation/concepts.md) · [AGENTS.md § Workflow nouns](AGENTS.md#workflow-nouns)
- **Installation:** [Prerequisites](docs/orientation/prerequisites.md)
- **How-tos:**
  - [Amend a plan before executing](docs/how-to/amend-a-plan.md)
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