# emery specify

Generate `spec.md` / `design.md` from the sources named on the invocation and commit them as one generation.

## Synopsis

```bash
emery specify <adapter>... [--description <adapter>=<text>]
emery specify --config [<path>]
emery specify                       # discovers the project-root emery.toml
```

## Description

The one generate verb. Each run names its own sources: every positional `<adapter>` binds a **workspace-backed** source (the adapter reads a read-only view rooted at the project directory; the binding key is the adapter name), and every `--description <adapter>=<text>` (repeatable, `-d`) binds a **description-backed** source (the adapter extracts the inline text; no filesystem view is lent). Nothing about the binding list persists between runs — repeat the sources on every invocation (Makefile, skill, CI), or keep them in an operator-owned `emery.toml` selected with `--config [<path>]` (`-c`; the omitted value selects the project-relative `emery.toml`). A run naming no bindings at all discovers the project-root `emery.toml` as a fallback — discovery is a fallback, never merged with argv bindings.

Each run resolves its adapters before extracting; a local `.wasm` component loads through the deployment's `omnia:plugins/loader` capability, read fresh on every run — nothing is mirrored, so deleting the file makes the next run refuse typed. A binding's optional `digest` pin is verified host-side before validation, and every resolved digest rides the success envelope so the operator can commit it as the pin (trust-on-first-use). Extraction dispatches every binding over the `Source` capability, reconciles the typed claims under authority precedence (intent > documentation > behaviour), synthesises the two reviewable documents, and commits them as one generation behind the atomically swapped `current` pointer. Gaps stay `[unknown]`; disagreement surfaces inline as `[conflict]` / `[divergence]`. Re-running over identical sources is byte-stable and reports an empty re-mine diff in the success envelope — nothing is persisted for the diff. Review the committed set with [`emery show`](show.md).

`emery specify` without any source — and with no project-root `emery.toml` to discover — fails typed with `specify-source-required` (exit `1`); there is no interactive prompt mode, so every other input arrives as a flag. Binding the same `name` twice fails as `bad_request` (exit `1`); a `--description` entry without `<adapter>=` fails as `bad_request` (exit `1`); combining `--config` with positional adapters or `--description` fails as `bad_request` (exit `1`).

Resolution is **local-only** — there is no download path. A local `.wasm` component loads dynamically through the deployment loader (the journey host in [`examples/runtime.rs`](../../../examples/runtime.rs) loads the built mock component by path); a bare name or package pin still dispatches only guests declared in the runtime invocation until registry acquisition lands. A name or pin outside the declared set fails at dispatch. GitHub URLs are refused (`bad_request`).

This is the CLI command invoked by [`/emery:specify`](../../../plugins/emery/skills/specify/SKILL.md). The skill elicits any missing arguments conversationally and passes them as flags; the CLI itself has no interactive mode.

## The `emery.toml` config

`emery.toml` is operator-authored and operator-owned: the engine never writes it, and reads it when the `--config` flag names it or when a bindingless run discovers it at the project root. `--config` without a value names the project-relative `emery.toml`; an explicit value names another project-relative file (a missing explicit file is a read error, exit `3`, never a discovery miss). Each `[[source]]` entry binds one source, in declaration order; its `name` is the binding key, so one adapter may bind several roots. Exactly one content key per entry — `path` or `description`; omitted means the workspace lend at `.`. `path` (and a local component `adapter`) resolves relative to the file containing it, as Cargo resolves `path` dependencies. Duplicate names fail as `bad_request` (exit `1`), the same typed error argv raises.

Every filesystem input is normalized within the project preopen `.`. Absolute paths and relative paths that escape above it fail as `bad_request` (exit `1`); the engine never tries to infer a host path from the guest's ambient working directory. The `git` and `url` content keys are reserved: they parse but refuse typed (`bad_request`) until the remote read-view grant exists. The per-binding `digest` key pins a local component's exact bytes (`sha256:<hex>`), verified host-side before validation — a mismatch refuses `digest-mismatch` (exit `1`); on a registry adapter the key stays reserved, as does the per-binding `registry` override, until registry acquisition lands.

```toml
# Workspace lend of the invocation directory (the default: path = ".").
[[source]]
name = "docs"
adapter = "emery:documentation@1.2.0"

# Local path, resolved relative to this file.
[[source]]
name = "api-surface"
adapter = "typescript"
path = "packages/api/src"

# Inline description instead of a filesystem view — the file form of
# `--description`.
[[source]]
name = "intent"
adapter = "intent@1.0.0"
description = "Ship a location-independent spec generator."

# Local component, loaded fresh each run; the optional digest pins its
# exact bytes (an unpinned run reports the digest to commit here).
[[source]]
name = "custom"
adapter = "./adapters/custom.wasm"
digest = "sha256:9f2c44…"
```

## Options

| Option | Description |
|--------|-------------|
| `<adapter>...` (positional) | Source adapter identifiers: first-party shorthands, package references, or project-relative local `.wasm` component paths — each bound as a workspace-backed source. |
| `--description <adapter>=<text>` | Inline description-backed source binding (repeatable); `-d` for short. |
| `--config [<path>]` | Operator-owned config; the omitted value selects `emery.toml` (`-c` for short). Mutually exclusive with positional adapters and `--description`. |
| `--format` | Global output format: `json` for structured automation output. |

## JSON output

When `--format json` is provided, returns:

- `generation` — the committed generation id the `current` pointer names
- `requirements` — requirement blocks in the committed `spec.md`
- `sources` — number of sources extracted this run
- `diff` — the re-mine diff against the superseded generation; absent on a first run, empty on a byte-stable re-run
- `digests` — one `{ source, digest }` entry per loader-loaded local component: the resolved `sha256:<hex>` digest to commit as the binding's pin; absent when no binding loaded a component

## See also

- [`emery show`](show.md) renders the committed documents; see the [CLI reference](index.md).
