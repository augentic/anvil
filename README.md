# Emery

[CI](https://github.com/augentic/emery/actions/workflows/ci.yaml)
[License: MIT OR Apache-2.0](#license)
[Docs](https://emery.augentic.io/)

Spec-driven development in [Cursor](https://cursor.com): plan a change, approve it, then `refine → build → merge` each slice from durable artifacts — not chat history.

**Operators:** install the Augentic Cursor plugin and the `emery` CLI, then run `/emery:init` → `/emery:plan` → `/emery:execute` → `/emery:finalize`.

**Contributors:** this repository is the Rust workspace that builds the `emery` binary and the ultrathin `/emery:`* skill wrappers. Source and target adapters live in [augentic/emery-adapters](https://github.com/augentic/emery-adapters).

## Choose your path


| I want to…                     | Go to                                                                                                                        |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| Run my first change            | [Install](#install) → [First change](#first-change)                                                                          |
| Understand the model           | [What is Emery?](https://emery.augentic.io/orientation/index.html) · [in-tree](docs/orientation/index.md)                |
| Look up a command              | [Quick reference](https://emery.augentic.io/reference/quick-reference.html) · [in-tree](docs/reference/quick-reference.md) |
| Recover when execute parks     | [Breakout skills](#breakout-skills) · [Drive a slice manually](docs/how-to/drive-slice-manually.md)                          |
| Contribute to the CLI / engine | [Developing Emery](#developing-emery-contributors)                                                                       |
| Author or debug an adapter     | [augentic/emery-adapters](https://github.com/augentic/emery-adapters)                                                    |




## The rhythm

```text
/emery:init  →  /emery:plan  →  Gate 1 (approve)  →  /emery:execute  →  /emery:finalize
                                         │
                                         └─ per slice: refine → build → merge
```

Default workflow: init → plan → Gate 1 → execute → finalize

Gate 1 is the operator review step: nothing runs until you stamp the plan `approved`. A one-slice change uses the same steps as a twelve-slice migration.

Example session (contracts target)

```text
$ emery --version
emery 0.x.x

$ /emery:init contracts@0.5.0
# → scaffolds .emery/; pulls the Contracts adapter

$ /emery:plan first-contract source intent=intent@0.5.0:value:"…"
# → writes change.md, discovery.md, plan.yaml
# → plan.lifecycle: pending   ← stop and review

$ /emery:execute
# → confirms Gate 1, then refine → build → merge
# → inspect contracts/ and .emery/specs/

$ emery plan status
# → drained

$ /emery:finalize first-contract
# → archives the plan after you publish via git
```



## Install



### 1. Cursor plugin

In Cursor: **Settings → Plugins**, search for **Augentic**, install the marketplace, and restart Cursor. That installs the Emery plugin (`/emery:`* skills). Every skill is an ultrathin wrapper around one `emery` CLI verb.

### 2. `emery` CLI (optional)

While the plugin will install the `emery` binary on `/emery:init`, it can be manually installed using:

```bash
# Or cargo-binstall (prebuilt; no local compile)
cargo binstall --git https://github.com/augentic/emery emery@0.28.0

# Or from source (needs a Rust toolchain + wasm32-wasip2)
cargo install --git https://github.com/augentic/emery --locked

# Homebrew
export HOMEBREW_GITHUB_API_TOKEN="$(gh auth token)"
brew tap augentic/tap && brew install emery
```

```bash
emery --version
```

All install routes and adapter-specific tooling: [Prerequisites](docs/orientation/prerequisites.md) · [hosted](https://emery.augentic.io/orientation/prerequisites.html).

## First change

This README’s golden path uses the **Contracts** target (`contracts@0.5.0`) — fewer platform prerequisites than Omnia or Vectis. The pin downloads a published adapter from GHCR; you do **not** need to clone [emery-adapters](https://github.com/augentic/emery-adapters) to use it.

For a full Omnia walkthrough (Rust + `wasm32-wasip2`), see the [quick-start tutorial](docs/tutorials/quick-start.md).

Each step shows the skill first, then the CLI form.

**1. Initialize** — in Cursor Agent chat, in a fresh or disposable repository:

```text
/emery:init contracts@0.5.0
```

```bash
emery init contracts@0.5.0
```

What you should see: a `.emery/` tree (`project.yaml`, `slices/`, `specs/`, `archive/`) and, when absent, a generated `AGENTS.md`.

**2. Plan** a change from a one-line intent:

```text
/emery:plan first-contract source intent=intent@0.5.0:value:"Author an HTTP API contract for a health endpoint that returns status and version."
```

```bash
emery plan author first-contract --source intent=intent@0.5.0:value:"Author an HTTP API contract for a health endpoint that returns status and version."
```

What you should see: `change.md`, `discovery.md`, and `plan.yaml` at the project root; `plan.lifecycle: pending`. Inspect those files before continuing — this is Gate 1 review.

**3. Approve and execute.** The skill asks for your explicit Gate 1 approval, stamps it, then drives the loop:

```text
/emery:execute
```

```bash
emery plan approve         # Gate 1: operator approval
emery plan execute         # refine → build → merge per slice
```

What you should see: slice artifacts under `.emery/slices/…`, generated contract files under `contracts/`, and merged baseline specs under `.emery/specs/`.

**4. Finalize.** Commit and publish through your normal Git workflow, then close the change:

```text
/emery:finalize first-contract
```

```bash
emery plan status     # must be `drained`
emery plan archive    # Gate 2
```

What you should see: `plan status` reports `drained`; archive moves the plan out of the active set.

Guided walkthrough of every artifact and transition: [quick-start tutorial](docs/tutorials/quick-start.md) (Omnia) · [hosted](https://emery.augentic.io/tutorials/quick-start.html). Command lookup: [Quick Reference](docs/reference/quick-reference.md).

### Breakout skills

When the execute loop parks, or you want to drive one slice by hand:


| Skill          | CLI equivalent            |
| -------------- | ------------------------- |
| `/emery:refine` | `emery slice refine`    |
| `/emery:build`  | `emery slice build`     |
| `/emery:merge`  | `emery slice merge run` |
| `/emery:drop`   | `emery slice drop`      |


Code generation lives in target adapters, not in Cursor skills. Vocabulary: [AGENTS.md § Workflow nouns](AGENTS.md#workflow-nouns); longer read: [Core concepts](docs/explanation/concepts.md).

## Stuck?


| Symptom                      | What to check                                                                                                                  |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `/emery:*` skills missing     | Augentic marketplace installed? Restart Cursor after install.                                                                  |
| `emery: command not found` | CLI on `PATH`? Re-run [Install](#install) and `emery --version`.                                                             |
| `plan execute` refuses       | Plan still `pending`? Run `emery plan approve` (or confirm Gate 1 in `/emery:execute`).                                       |
| Adapter / pin errors         | Use a pinned id (`contracts@0.5.0`) so the runtime can pull from GHCR; see [Prerequisites](docs/orientation/prerequisites.md). |
| Execute parked mid-slice     | Run the matching [breakout](#breakout-skills); see [Drive a slice manually](docs/how-to/drive-slice-manually.md).              |


Questions and bugs: [GitHub Issues](https://github.com/augentic/emery/issues).

## Adapters

You consume adapters as Wasm packages (for example `contracts@0.5.0`). Clone [emery-adapters](https://github.com/augentic/emery-adapters) only when authoring or debugging an adapter.


| Target      | Use case                                                      |
| ----------- | ------------------------------------------------------------- |
| `omnia`     | [Omnia](https://omnia.host) Rust WASM services                |
| `vectis`    | Cross-platform [Crux](https://redbadger.github.io/crux/) apps |
| `contracts` | API/interface contract work                                   |


Source adapters turn intent, documentation, legacy TypeScript, screenshots, or runtime captures into Evidence. Target adapters consume the resulting specs and build implementation outputs.

## Developing Emery (contributors)

The repository root is a Rust workspace producing the `emery` binary. The root `Makefile` forwards every goal to [cargo-make](Makefile.toml), so `make test` and `cargo make test` are interchangeable; this README uses the shorter form.

The default contributor loop is self-contained and model-free:

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

Start with the [developer loop](docs/contributing/dev-loop.md), then [Cursor operator plugins](docs/contributing/operator-plugins.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Documentation


| Resource                  | Link                                                                                                        |
| ------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Hosted Developer Guide    | [emery.augentic.io](https://emery.augentic.io/)                                                         |
| In-tree book source       | [docs/SUMMARY.md](docs/SUMMARY.md)                                                                          |
| Quick-start tutorial      | [docs/tutorials/quick-start.md](docs/tutorials/quick-start.md)                                              |
| Quick reference           | [docs/reference/quick-reference.md](docs/reference/quick-reference.md)                                      |
| Orientation               | [docs/orientation/index.md](docs/orientation/index.md)                                                      |
| Agent instructions        | [AGENTS.md](AGENTS.md)                                                                                      |
| Contributing / governance | [CONTRIBUTING.md](CONTRIBUTING.md) · [GOVERNANCE.md](GOVERNANCE.md) · [Code of Conduct](CODE-OF-CONDUCT.md) |




## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.