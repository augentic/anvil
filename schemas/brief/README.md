# `brief/schema.json`

Canonical JSON Schema (2020-12) for the YAML **frontmatter block** at
the top of every brief markdown file under
`schemas/*/briefs/**/*.md`. A brief describes one step of a schema's
pipeline; the frontmatter declares the step's identity (`id`,
`description`) and its graph edges (`needs`, `generates`, `tracks`).

## Shape

Every brief starts with a YAML frontmatter block delimited by `---`
lines:

```markdown
---
id: discovery
description: Read inputs and emit a neutral capability inventory.
generates: .specify/plans/<name>/discovery.md
needs: [propose]
---

Free-form markdown body follows.
```

The schema validates the YAML *between* the delimiters. The markdown
body is not constrained by this schema.

## Fields

| Field | Required | Shape | Notes |
|---|---|---|---|
| `id` | yes | non-empty string | Referenced by `needs` in sibling briefs and by `pipeline.*` entries in the owning `schema.yaml`. |
| `description` | yes | non-empty string | One-sentence summary surfaced when listing pipeline steps. |
| `needs` | no | unique array of strings | Briefs whose outputs this brief consumes. Forms the pipeline DAG. |
| `generates` | no | non-empty string | Output path (optionally glob-shaped) produced by the brief. |
| `tracks` | no | non-empty string | Identifier of an artefact this brief iterates over (e.g. `tasks` for a build brief). Signals per-item progress. |

`additionalProperties` is `false`: unknown keys cause validation to
fail. Keeping the frontmatter vocabulary small is the point — every
field here is load-bearing for how the pipeline runner schedules
briefs.

## Editor integration

VS Code, Zed, Cursor, and any other editor that speaks the
[YAML Language Server](https://github.com/redhat-developer/yaml-language-server)
protocol can lint brief frontmatter as-you-type by pointing at this
schema with a top-of-file modeline:

```markdown
# yaml-language-server: $schema=https://github.com/augentic/specify/raw/main/schemas/brief/schema.json
---
id: discovery
description: …
---
```

The comment MUST precede the opening `---` delimiter so the language
server sees it before the frontmatter block. Both the
`yaml-language-server:` prefix and the raw-content URL form are
required by the YAML Language Server convention; see its
[schema-association docs](https://github.com/redhat-developer/yaml-language-server#using-inlined-schema)
for the full syntax.

For offline work, point `$schema` at the checked-in file via a
`file://` URL or a workspace-relative path, depending on what your
editor's YAML integration accepts.

## Mirror

A byte-identical copy lives at
[`augentic/specify-cli/schemas/brief/schema.json`](https://github.com/augentic/specify-cli/tree/main/schemas/brief/schema.json).
When you edit the canonical file here, mirror the change to
`specify-cli` in the same commit pair — the two files are covered by
the `diff` byte-equality check in the RFC-2 cleanup acceptance tests.

## See also

- [`../schema.schema.json`](../schema.schema.json) — schema for the
  `schema.yaml` that declares which briefs make up a schema's
  pipeline; `pipeline.*` entries reference brief `id` values.
- [`../omnia/briefs/`](../omnia/briefs/) and
  [`../vectis/briefs/`](../vectis/briefs/) — live brief corpora this
  schema validates in practice.
