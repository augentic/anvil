# emery init

Scaffold `.emery/` with the project's authored source bindings (ADR-0009 §1).

## Synopsis

```bash
emery init <adapter>... [--value <adapter>=<text>] [--name <project-name>] [--description "<description>"]
emery init --upgrade
```

## Description

Scaffolds a single-project setup: resolves each requested source adapter on the source axis, mirrors local components into the out-of-tree per-project cache, and writes `.emery/project.yaml` carrying the project identity, the running binary's version as the `emery:` pin, and one `sources:` entry per binding.

Two binding forms exist:

- Each positional `<adapter>` records a **workspace-backed** binding: the adapter reads a read-only view rooted at the project directory. The binding key is the resolved adapter name.
- Each `--value <adapter>=<text>` (repeatable) records a **value-backed** binding: the adapter extracts the inline text and no filesystem view is lent. This is how an operator directive (an intent source) binds.

`emery init` without any source fails typed with `init-source-required` (exit `2`) — there is no interactive prompt mode; every input arrives as a flag. Binding the same adapter twice fails `init-source-duplicate`; a `--value` entry without `<adapter>=` fails as an argument error (exit `2`).

Re-running `emery init` in an already-initialized project changes nothing and exits `0` with a message routing to `emery init --upgrade`. `emery init --upgrade` is the re-entry path: it bumps the `project.yaml.emery` pin over an existing project and re-ensures every recorded binding. Recorded bindings are never rewritten: a bare record stays bare and a pinned record keeps its pin. `--upgrade` never updates the installed `emery` binary itself; when the project's recorded pin is newer than the running binary, commands abort with `emery-version-too-old` (exit `3`) and the error's `hint:` line prints the literal reinstall command.

Resolution is **local-only** — there is no download path. A seeded project component cache entry always wins, for pinned and bare identities alike; a pinned package reference (`emery:intent@1.0.0` or the `intent@1.0.0` shorthand) otherwise resolves its verified global-store entry (`<store-root>/<name>@<version>.wasm`, verify-on-read against its digest `.meta` sidecar — an unverifiable entry is `adapter-sidecar-missing` / `adapter-digest-mismatch`). Nothing local is the typed `adapter-not-found` (exit `1`); the explicit install verb arrives with the distribution surface. A bare first-party name (`documentation`, `intent`, `typescript`) with no cache seed resolves the component embedded in the release binary (the pins live in `scripts/first-party.txt`). GitHub URLs are refused (`adapter-github-uri-unsupported`).

This is the CLI command invoked by [`/emery:init`](../../../plugins/emery/skills/init/SKILL.md). The skill elicits any missing arguments conversationally and passes them as flags; the CLI itself has no interactive mode.

## Options

| Option | Description |
|--------|-------------|
| `<adapter>...` (positional) | Source adapter identifiers: first-party shorthands, package references, or local `.wasm` component paths — each bound as a workspace-backed source. |
| `--value <adapter>=<text>` | Inline value-backed source binding (repeatable). |
| `--name` | Project name (defaults to the project directory basename). |
| `--description` | Free-form project description. |
| `--upgrade` | Re-enter an initialized project: bump the `emery` pin and re-ensure every recorded binding. Mutually exclusive with the other arguments. |
| `--format` | Global output format: `json` for structured automation output. |

## JSON output

When `--format json` is provided, returns:

- `mode` — what this run did: `scaffolded`, `already-initialized`, or `upgraded`
- `config-path` — path to the written `project.yaml`
- `sources` — the bound source keys, in binding order
- `emery-version` — version pinned in `project.yaml`

## See also

- `emery specify` consumes the authored bindings; see the [CLI reference](index.md).
