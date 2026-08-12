# emery init

Scaffold the `.emery/` project structure and starter agent context.

## Synopsis

```bash
emery init <adapter> [--name <project-name>] [--description "<description>"] [--platforms <csv>]
emery init --upgrade
```

## Description

Scaffolds a single-project setup. Creates `.emery/{slices,specs,archive}/`, resolves the adapter identifier into the out-of-tree per-project cache, writes `.emery/project.yaml` with `adapter:` set and a `rules:` entry per `pipeline.define` brief, records the running binary's version as `emery-version`, and generates root `AGENTS.md` plus `.emery/context.lock` when `AGENTS.md` is absent. `change.md` and `plan.yaml` are operator artifacts minted later by `/emery:plan` (via `emery plan author`, which scaffolds both files together).

`emery init` without an adapter fails typed with `init-adapter-required` (exit `2`) — there is no interactive prompt mode; every input arrives as a flag. When the resolved target requires `--platforms` and none was passed, the typed `project-platforms-required` names the flag and the default set.

Re-running `emery init` in an already-initialized project changes nothing and exits `0` with a message routing to `emery init --upgrade`. `emery init --upgrade` is the re-entry path: it bumps the `project.yaml.emery` pin over an existing project (preserving every operator artifact) and re-resolves the project's declared adapter. The recorded binding is never rewritten: a bare record stays bare and a pinned record keeps its pin. An upgrade over a bare record with no cache seed refreshes the name to the newest published version (installing it into the global store when newer); a bare record with a live cache seed keeps resolving the seed. `--upgrade` never updates the installed `emery` binary itself; when the project's recorded pin is newer than the running binary, commands abort with `emery-version-too-old` (exit `3`) and the error's `hint:` line prints the literal reinstall command — update the binary through its install channel first, then re-run.

A pinned package reference (`emery:omnia@1.0.0` or the `omnia@1.0.0` shorthand) that misses the global adapter store is **installed automatically**: the runtime pulls the component from the fixed first-party registry (`ghcr.io/augentic/emery-adapters/<name>:<version>`), writes it as `<store-root>/<name>@<version>.wasm` with a digest `.meta` sidecar recording the OCI repository and manifest digest, and verifies the entry after the write. The mapping is compiled into the binary — no project configuration can redirect it. A pull failure is the typed `adapter-install-failed` (exit `1`, carrying the recoveries: name check, `emery adapter add`, explicit pin); a malformed artifact is `adapter-install-invalid`; a verify failure is `adapter-digest-mismatch`. The same pull-on-miss applies to every command that resolves a pin (source survey/extract, target build, merge), not just init.

A **bare name** (`emery init omnia`) persists bare on `project.yaml.adapter` and resolves **local-first**: a seeded project component cache entry (via [`emery adapter add`](adapter.md) or a local `.wasm` component at init) always wins — the co-dev path is never shadowed by a published component; otherwise init refreshes the name to the newest published version (the registry's newest exact-SemVer tag) and installs it into the global adapter store. After provisioning, later runs resolve the newest installed store version with no registry check; refresh explicitly with [`emery adapter upgrade`](adapter.md#emery-adapter-upgrade). The runtime logs each resolved adapter version to stderr. Local component paths never pull.

On success init prints a postflight report: what was scaffolded (or upgraded), the resolved adapter, the written config path, and the pinned `emery` version.

The command also upserts `.emery/scratch/` into the project `.gitignore`.

If root `AGENTS.md` already exists, `emery init` preserves it byte-for-byte and skips context generation.

This is the CLI command invoked by [`/emery:init`](../../../plugins/emery/skills/init/SKILL.md). The skill elicits any missing arguments conversationally and passes them as flags; the CLI itself has no interactive mode.

## Options

| Option | Description |
|--------|-------------|
| `<adapter>` (positional) | Adapter identifier: a first-party shorthand (bare `omnia` — resolves a seeded cache entry, else the newest published version; `omnia@1.0.0` for a registry pin), a package reference (`emery:omnia@1.0.0`), or a local `.wasm` component path. GitHub URLs are refused (`adapter-github-uri-unsupported`). Required unless `--upgrade` is set. |
| `--name` | Project name (defaults to the project directory basename). |
| `--description` | Free-form project description (tech stack, architecture, testing) |
| `--platforms` | Comma-separated target platform set (e.g. `core,ios,android`). Required when the target adapter declares `platforms.required`; `core` is mandatory in every set. |
| `--upgrade` | Re-enter an initialized project: bump the `emery` pin, re-scaffold preservation-safe files only, and re-resolve the declared adapter. Mutually exclusive with the other arguments. |
| `--format` | Global output format: `json` for structured automation output |

## JSON output

When `--format json` is provided, returns:

- `mode` -- what this run did: `scaffolded`, `already-initialized`, or `upgraded`
- `config-path` -- path to the written `project.yaml`
- `adapter-name` -- resolved adapter name
- `adapter-binding` -- the binding value recorded on `project.yaml.adapter` (the selector exactly as supplied, e.g. `omnia` or `emery:omnia@0.7.0`); absent in the no-op re-entry
- `cache-present` -- whether the resolved adapter's component-cache provenance sidecar (`components/<name>.meta.yaml`) was found
- `directories-created` -- list of directories created
- `scaffolded-rule-keys` -- per-brief rule keys added to `project.yaml`
- `emery-version` -- version recorded in `project.yaml`
- `context-generated` -- `true` when init generated root `AGENTS.md` and `.emery/context.lock`
- `context-skipped` -- `true` when context generation was skipped
- `context-skip-reason` -- present when skipped (`existing-agents-md`)


## See also

- [Configuration Files](../configuration.md) -- project.yaml and metadata format
- `AGENTS.md` context is generated during `emery init`; later inspection is direct file review.
- [Prerequisites](../../orientation/prerequisites.md) -- setup before first init
