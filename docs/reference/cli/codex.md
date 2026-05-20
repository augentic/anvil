# specify codex

Project-resolved review rule catalogue commands.

## Synopsis

```bash
specify codex export --format json
```

## Description

`specify codex export` resolves the active review rule set for the current project and emits the catalogue as JSON. Resolution is deterministic:

1. The foundational `default` adapter's `codex/` directory.
2. The project adapter's `codex/` directory.
3. Shared codex catalogs, reserved for a future configuration surface.
4. The repo-root `codex/` overlay.

Within each source, Markdown files are loaded in lexical path order. Duplicate rule ids across the resolved set are validation failures and surface through `export`'s exit-2 error envelope, which lists the offending rule ids on stable codes such as `codex.rule-id-unique`.

## Distribution

First-party codex rules ship beside first-party adapters under `adapters/<name>/codex/`. Regular `specify init <adapter>` caches the selected adapter into `.specify/.cache/<adapter>/`; when the selected adapter comes from a distribution tree that also contains a sibling `default` adapter, init also caches that sibling into `.specify/.cache/default/`.

`specify codex export` then resolves `default` exactly like any other adapter: cache first, then the project-local fallback. This keeps real projects independent from the plugin checkout after init while avoiding a separate packaged rule store.

## Subcommands

### specify codex export

```bash
specify codex export --format json
```

Exports the resolved codex as the consumer contract for future review tooling. The JSON envelope uses the standard CLI `envelope-version` and includes `rule-count` plus ordered `rules`.

Each rule includes frontmatter fields, Markdown `body`, `source-path`, and an internally-tagged `kind` plus provenance-specific fields:

- Adapter rules (`"kind": "adapter"`) include `name` and `version`.
- Catalog rules (`"kind": "catalog"`) include `name`.
- Repo overlay rules (`"kind": "repo"`) carry no provenance-specific fields.
- Missing optional `review-mode` is emitted as `null`.

The export is a rule-catalog contract, not the future review finding schema. RM-04 owns finding-specific fields such as evidence, remediation, file references, and CI annotation shape; RM-11 consumes both the resolved codex export and the RM-04 finding schema when `specify review` lands.

When resolution fails — bad frontmatter, duplicate ids, missing required headings — `export` exits `2` (validation-failed) and emits a `results: []` array in the standard envelope listing each `rule-id` plus a human `detail`. There is no separate `validate`, `list`, or `show` verb; consumers shell into `export --format json` and filter the `rules[]` array client-side.

## See also

- [specify adapter](adapter.md) -- adapter resolution and cache behavior.
- [Adapters](../adapters/index.md) -- adapter-owned codex directories and manifest boundaries.
