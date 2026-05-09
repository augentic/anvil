# specify codex

Project-resolved review rule catalogue commands.

## Synopsis

```bash
specify codex list
specify codex show <rule-id>
specify codex validate
specify codex export --format json
```

## Description

`specify codex` resolves the active review rule set for the current project. Resolution is deterministic:

1. The foundational `default` capability's `codex/` directory.
2. The project capability's `codex/` directory.
3. Shared codex catalogs, reserved for a future configuration surface.
4. The repo-root `codex/` overlay.

Within each source, Markdown files are loaded in lexical path order. Duplicate rule ids across the resolved set are validation failures.

## Distribution

First-party codex rules ship beside first-party capabilities under `capabilities/<name>/codex/`. Regular `specify init <capability>` caches the selected capability into `.specify/.cache/<capability>/`; when the selected capability comes from a distribution tree that also contains a sibling `default` capability, init also caches that sibling into `.specify/.cache/default/`.

Project-aware codex commands then resolve `default` exactly like any other capability: cache first, then the project-local fallback. This keeps real projects independent from the plugin checkout after init while avoiding a separate packaged rule store.

## Subcommands

### specify codex list

Print a concise text list of resolved rules: id, severity, provenance, and title. With global `--format json`, emits the same rule summaries in the standard CLI JSON envelope.

### specify codex show

```bash
specify codex show UNI-002
```

Shows one resolved rule by stable id. Text output prints frontmatter summary, source path, provenance, and Markdown body. JSON output wraps the full exported rule as `rule`.

### specify codex validate

Validates every resolved codex file and the resolved set invariants. A clean codex exits `0`. Rule shape failures or duplicate ids exit with validation semantics (`2`) and include stable validation rule ids such as `codex.rule-id-unique`.

### specify codex export

```bash
specify codex export --format json
```

Exports the resolved codex as the consumer contract for future review tooling. The JSON envelope uses `schema-version: 3` and includes `rule-count` plus ordered `rules`.

Each rule includes frontmatter fields, Markdown `body`, `source-path`, `provenance-kind`, and provenance-specific fields:

- Capability rules include `capability-name` and `capability-version`.
- Catalog rules include `catalog-name`.
- Repo overlay rules set capability and catalog fields to `null`.
- Missing optional `review-mode` is emitted as `null`.

The export is a rule-catalog contract, not the future review finding schema. RM-04 owns finding-specific fields such as evidence, remediation, file references, and CI annotation shape; RM-11 consumes both the resolved codex export and the RM-04 finding schema when `specify review` lands.

## See also

- [specify capability](capability.md) -- capability resolution and cache behavior.
- [Capabilities](../capabilities/index.md) -- capability-owned codex directories and manifest boundaries.
