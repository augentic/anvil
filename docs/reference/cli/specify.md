# emery specify

Generate `spec.md` / `design.md` from the sources named on the invocation and commit them as one generation (ADR-0008 §3, ADR-0009).

## Synopsis

```bash
emery specify <adapter>... [--value <adapter>=<text>]
emery specify --sources <path>
```

## Description

The one generate verb. Each run names its own sources: every positional `<adapter>` binds a **workspace-backed** source (the adapter reads a read-only view rooted at the project directory; the binding key is the adapter name), and every `--value <adapter>=<text>` (repeatable) binds a **value-backed** source (the adapter extracts the inline text; no filesystem view is lent). Nothing about the binding list persists between runs — repeat the sources on every invocation (Makefile, skill, CI), or keep them in an operator-owned `sources.toml` passed explicitly with `--sources <path>`.

Each run resolves its adapters before extracting; a local `.wasm` component mirrors into the out-of-tree project cache as a side effect, so the selector stays resolvable after the original file is removed. Extraction dispatches every binding over the adapter seam, reconciles the typed claims under authority precedence (intent > documentation > behaviour), synthesises the two reviewable documents, and commits them as one generation behind the atomically swapped `current` pointer. Gaps stay `[unknown]`; disagreement surfaces inline as `[conflict]` / `[divergence]`. Re-running over identical sources is byte-stable and reports an empty re-mine diff in the success envelope (ADR-0010) — nothing is persisted for the diff. Review the committed set with [`emery show`](show.md).

`emery specify` without any source fails typed with `specify-source-required` (exit `2`) — there is no interactive prompt mode; every input arrives as a flag. Binding the same key twice fails `specify-source-duplicate`; a `--value` entry without `<adapter>=` fails as an argument error (exit `2`); combining `--sources` with positional adapters or `--value` fails as an argument error (exit `2`).

Resolution is **local-only** — there is no download path. Until dynamic loading returns, adapter admission is static: `emery specify` dispatches only guests declared in the runtime invocation (the journey host's mock `source` in [`examples/runtime.rs`](../../../examples/runtime.rs) is the in-tree pattern). A name or pin outside the declared set fails at the dispatch seam. GitHub URLs are refused (`adapter-github-uri-unsupported`).

This is the CLI command invoked by [`/emery:specify`](../../../plugins/emery/skills/specify/SKILL.md). The skill elicits any missing arguments conversationally and passes them as flags; the CLI itself has no interactive mode.

## The `--sources` file

`sources.toml` is operator-authored and operator-owned: the engine never writes it, never discovers it implicitly, and reads it only when `--sources <path>` names it. Each `[sources.<key>]` table binds one source; the table key becomes the seam binding key, so one adapter may bind several roots. Exactly one location key per entry — `path` or `value`; omitted means the workspace lend at `.`. `path` (and a local component `adapter`) resolves relative to the file containing it, as Cargo resolves `path` dependencies, so the file works from any invocation directory. The `git` and `url` location keys are reserved: they parse but refuse typed (`source-remote-unsupported`) until the remote read-view grant exists.

```toml
# Workspace lend of the invocation directory (the default: path = ".").
[sources.docs]
adapter = "emery:documentation@1.2.0"

# Local path, resolved relative to this file.
[sources.api-surface]
adapter = "typescript"
path = "packages/api/src"

# Inline value instead of a filesystem view — the file form of `--value`.
[sources.intent]
adapter = "intent@1.0.0"
value = "Ship a location-independent spec generator."
```

## Options

| Option | Description |
|--------|-------------|
| `<adapter>...` (positional) | Source adapter identifiers: first-party shorthands, package references, or local `.wasm` component paths — each bound as a workspace-backed source. |
| `--value <adapter>=<text>` | Inline value-backed source binding (repeatable). |
| `--sources <path>` | Operator-owned `sources.toml` carrying the whole binding list; mutually exclusive with positional adapters and `--value`. |
| `--format` | Global output format: `json` for structured automation output. |

## JSON output

When `--format json` is provided, returns:

- `generation` — the committed generation id the `current` pointer names
- `requirements` — requirement blocks in the committed `spec.md`
- `sources` — number of sources extracted this run
- `diff` — the re-mine diff against the superseded generation; absent on a first run, empty on a byte-stable re-run

## See also

- [`emery show`](show.md) renders the committed documents; see the [CLI reference](index.md).
