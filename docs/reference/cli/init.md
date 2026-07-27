# emery init

Scaffold the `.emery/` project structure and starter agent context.

## Synopsis

```bash
emery init <adapter> [--name <project-name>] [--description "<description>"] [--platforms <csv>]
emery init --workspace [--name <project-name>] [--description "<description>"]
emery init --upgrade
```

## Description

Two modes, picked by the presence of `--workspace`:

- **Regular** (positional `<adapter>`): scaffolds a single-project workspace. Creates `.emery/{slices,specs,archive}/`, resolves the adapter identifier into the out-of-tree per-project cache, writes `.emery/project.yaml` with `adapter:` set and a `rules:` entry per `pipeline.define` brief, records the running binary's version as `emery-version`, and generates root `AGENTS.md` plus `.emery/context.lock` when `AGENTS.md` is absent.
- **Workspace** (with `--workspace`): scaffolds a registry-only workspace. Creates `.emery/`, writes a sentinel `.emery/project.yaml { workspace: true, … }` (the `adapter:` field is omitted — its absence is what disables adapter resolution), creates an empty `registry.yaml { version: 1, projects: [] }` at the repo root, and generates workspace-shaped root `AGENTS.md` plus `.emery/context.lock` when `AGENTS.md` is absent. No adapter identifier is needed, no cache is needed, and phase-pipeline directories (`slices/`, `specs/`) are NOT scaffolded — the workspace disables those pipelines on itself. Slot materialization is operator-owned. `change.md` and `plan.yaml` are operator artifacts minted later by `/emery:plan` (via `emery plan author`, which scaffolds both files together). Refuses to run when `.emery/` already exists.

The two modes are mutually exclusive: `emery init` with both an adapter positional and `--workspace` exits `2` with clap's standard parse-error diagnostic. With neither, the typed `init-adapter-required` error (exit `2`) names the missing argument — there is no interactive prompt mode; every input arrives as a flag. When the resolved target requires `--platforms` and none was passed, the typed `project-platforms-required` names the flag and the default set.

Re-running `emery init` in an already-initialized project changes nothing and exits `0` with a message routing to `emery init --upgrade`. `emery init --upgrade` is the re-entry path: it bumps the `project.yaml.emery` pin over an existing project (preserving every operator artifact) and re-resolves the project's declared adapter.

A pinned package reference (`emery:omnia@1.0.0` or the `omnia@1.0.0` shorthand) that misses the global adapter store is **installed automatically**: the runtime pulls the component from the fixed first-party registry (`ghcr.io/augentic/emery-adapters/<name>:<version>`), writes it as `<store-root>/<name>@<version>.wasm` with a digest `.meta` sidecar recording the OCI repository and manifest digest, and verifies the entry after the write. The mapping is compiled into the binary — no project configuration can redirect it. A pull failure is the typed `adapter-install-failed` (exit `1`); a malformed artifact is `adapter-install-invalid`; a verify failure is `adapter-digest-mismatch`. The same pull-on-miss applies to every command that resolves a pin (source survey/extract, target build, merge), not just init. Bare names (resolved from the seeded project component cache — see [`emery adapter add`](adapter.md)) and local component paths never pull.

On success init prints a postflight report: what was scaffolded (or upgraded), the resolved adapter, the written config path, and the pinned `emery` version.

In both modes the command upserts `.emery/scratch/` and top-level `workspace/` into the project `.gitignore`.

If root `AGENTS.md` already exists, `emery init` preserves it byte-for-byte and skips context generation. Init inside `workspace/<peer>/` also skips nested `AGENTS.md` generation; workspace clones inherit context from their owning project.

This is the CLI command invoked by [`/emery:init`](../../../plugins/emery/skills/init/SKILL.md). The skill elicits any missing arguments conversationally (including the regular-vs-workspace topology question) and passes them as flags; the CLI itself has no interactive mode.

## Options

| Option | Description |
|--------|-------------|
| `<adapter>` (positional) | Adapter identifier: a first-party shorthand (bare `omnia` for a seeded cache entry, `omnia@1.0.0` for a registry pin), a package reference (`emery:omnia@1.0.0`), or a local `.wasm` component path. GitHub URLs are refused (`adapter-github-uri-unsupported`). Required unless `--workspace` or `--upgrade` is set. |
| `--name` | Project name (defaults to the project directory basename). For workspace mode, must be kebab-case (the CLI bakes it into `change.md`'s frontmatter). |
| `--description` | Free-form project description (tech stack, architecture, testing) |
| `--workspace` | Scaffold a registry-only workspace instead of a regular project. Refuses to run when `.emery/` already exists. Mutually exclusive with the `<adapter>` positional. |
| `--platforms` | Comma-separated target platform set (e.g. `core,ios,android`). Required when the target adapter declares `platforms.required`; `core` is mandatory in every set. |
| `--upgrade` | Re-enter an initialized project: bump the `emery` pin, re-scaffold preservation-safe files only, and re-resolve the declared adapter. Mutually exclusive with every other argument. |
| `--format` | Global output format: `json` for structured automation output |

## JSON output

When `--format json` is provided, returns:

- `mode` -- what this run did: `scaffolded`, `already-initialized`, or `upgraded`
- `config-path` -- path to the written `project.yaml`
- `adapter-name` -- resolved adapter name (or the literal string `workspace` in workspace mode)
- `cache-present` -- whether the resolved adapter's component-cache provenance sidecar (`components/<name>.meta.yaml`) was found (always `false` in workspace mode)
- `directories-created` -- list of directories created
- `scaffolded-rule-keys` -- per-brief rule keys added to `project.yaml` (always empty in workspace mode)
- `emery-version` -- version recorded in `project.yaml`
- `context-generated` -- `true` when init generated root `AGENTS.md` and `.emery/context.lock`
- `context-skipped` -- `true` when context generation was skipped
- `context-skip-reason` -- present when skipped (`existing-agents-md` or `workspace-clone`)


## See also

- [Configuration files](../../reference/configuration.md#projectyaml) and [Registry](../../reference/registry.md) -- when to choose a workspace vs platform-as-project
- [Configuration Files](../configuration.md) -- project.yaml and metadata format
- `AGENTS.md` context is generated during `emery init`; later inspection is direct file review.
- [Prerequisites](../../orientation/prerequisites.md) -- setup before first init
- [`emery registry`](registry.md) -- manage the workspace's registry catalogue
