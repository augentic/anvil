---
id: analyze
description: Documentation extraction prompt for /change:analyze (Omnia).
generates: .specify/plans/<name>/discovery.md
---

# Omnia documentation extraction for `/change:analyze`

This brief carries Omnia's documentation extraction prompt for the [`/change:analyze`](../../../analyze/SKILL.md) skill. The skill resolves this file and executes the extraction algorithm below against each `documentation` input.

The output is the unified fenced-YAML candidate block shape defined in [`analyze/SKILL.md` §Output contract](../../../analyze/SKILL.md). Each block is appended under the pre-existing `## Candidate inventory` heading in `discovery.md`. Do not write the heading itself; the discovery brief owns it.

**Fixed field order (hard contract):** inside each YAML block, fields MUST appear in the order `kind`, `sources`, `handler`, `touches`, `surfaces`, `declared-at`, `unresolved`. Omit fields that don't apply. Doc-derived blocks typically omit `handler` and `touches`.

---

## Documentation extraction

Extract candidate hints from documentation: prose, PDFs, runbooks, OpenAPI specs, AsciiDoc, Markdown design notes, exported tickets. Identify the candidates the docs describe, cite the artifact paths in `declared-at`, and emit `surfaces` reflecting how cleanly each candidate is specified. Also surface constraints and open questions the docs carry alongside candidate descriptions.

### 1. Inventory the input

If `$INPUT_PATH` is a directory, walk it and enumerate artifacts by recognisable type:

| Shape                  | Signals                                              |
| ---------------------- | ---------------------------------------------------- |
| Markdown / AsciiDoc    | `*.md`, `*.adoc` — follow `##` / `###` headings.     |
| OpenAPI                | `*.yaml` / `*.json` with `openapi:` or `swagger:`.   |
| Runbook / procedure    | Prose with imperative "Steps:" or numbered lists.    |
| PDF / exported brief   | `*.pdf` — extract text, reference sections by page.  |
| Ticket / RFC export    | Prose with explicit headings or labelled sections.   |

If `$INPUT_PATH` is a single file, treat it as the whole inventory. Artifacts that fail to parse (unreadable PDF, malformed OpenAPI) halt the brief with a diagnostic naming the offending file; never write a partial `$DISCOVERY`.

### 2. Identify candidates per artifact

A candidate is a cohesive behavior the docs describe — *"what does the system do for the user or operator?"* — not a file and not a heading. Clustering heuristics:

- **Runbooks / procedures.** Each operational procedure (rotate a key, drain a queue, roll back a deploy) is one candidate.
- **OpenAPI.** Group endpoints by tag or by shared path prefix. A `POST /users` + `GET /users/{id}` pair with a common `tag: users` is one candidate (`user-directory`), not two. Use `operationId` or the tag's slug as a name seed.
- **Markdown / AsciiDoc.** Top-level adapter headings (`## X`) typically map 1:1 to candidates. Cluster sub-sections into their parent when the prose clearly describes one candidate across them. Do not split a single candidate just because its doc has many sub-headings.
- **Unstructured prose / PDFs.** Cluster by described behavior. Prefer candidate names that match the noun-phrase the doc uses for the behavior (`user-onboarding`, not `flow-3`).

Candidate names are kebab-case, 2–4 tokens, noun-phrases describing the behavior.

### 3. Fill each candidate block

For every identified candidate, emit:

````markdown
### <candidate-name>

```yaml
kind: candidate
sources: [<source-key>]
surfaces:
  - <source-key>:<surface-id>
declared-at:
  - <artifact-path#fragment>
```
````

Per-field rules:

- **`kind`** — always `candidate`.
- **`sources`** — list containing the documentation source key. When `$SOURCE_KEY` is supplied by the caller, use it; otherwise derive from the input path.
- **`handler`** — omit for documentation-derived candidates unless the docs explicitly name a handler function or entry point.
- **`touches`** — omit for documentation-derived candidates unless the docs explicitly list source files.
- **`surfaces`** — list of observable surfaces this candidate covers, namespaced `<source-key>:<surface-id>`. Derive surface ids from the doc's described triggers, routes, commands, or events. Sort alphabetically.
- **`declared-at`** — literal artifact paths relative to `$INPUT_PATH`. Cite deep-links where the artifact supports them:
  - OpenAPI — JSON pointer per RFC 6901, e.g. `api-spec.yaml#/paths/~1users/post`.
  - Markdown / AsciiDoc — heading slug, e.g. `runbook.md#rotate-the-upstream-ingest-key` (GitHub slug rules: lowercase, hyphen-separated, punctuation stripped).
  - PDF — page fragment, e.g. `ops-runbook.pdf#page=12` (hint, not a guarantee).

  Sort alphabetically. Non-empty.
- **`unresolved`** — omit unless the candidate's boundary is too ambiguous to accept without operator review, in which case set `true`.

### 4. Extract constraints and open questions

Documentation inputs often carry rules and unresolved decisions alongside candidate descriptions. Collect them into two appendix blocks AFTER the last candidate in `$DISCOVERY`:

```markdown
## Constraints (from documentation)

- <constraint text> (source: <artifact path[#fragment]>)
- <constraint text> (source: <artifact path[#fragment]>)

## Open questions (from documentation)

- <question text> (source: <artifact path[#fragment]>)
- <question text> (source: <artifact path[#fragment]>)
```

Rules:

- Every entry cites the artifact path (and optional fragment) it came from, so humans can audit the extraction.
- Sort entries alphabetically by their leading text within each block.
- Omit a block entirely if it would be empty — do not emit an empty `## Constraints` heading.

### 5. Idempotency

Follow the rules pinned in [`analyze/SKILL.md` §Idempotency](../../../analyze/SKILL.md):

- Candidates sorted alphabetically by name.
- Fixed field order inside each YAML block (§Step 3 above).
- `sources`, `surfaces`, `declared-at` sorted alphabetically.
- No timestamps, environment variables, absolute paths, or run IDs in `$DISCOVERY`.
- Re-running against unchanged inputs produces byte-identical output.

When `$SOURCE_KEY` is supplied, the skill (not this brief) prepends the `<!-- source-key: $SOURCE_KEY -->` marker before each `### <name>` heading this run produces. Do not emit the marker from the brief.

### 6. Error and empty-input handling

- **Unreadable artifact** — halt with a diagnostic naming the file; never write a partial `$DISCOVERY`.
- **Empty candidate inventory** (input parseable but no candidates discernable) — write `$DISCOVERY` with no `###` blocks, plus a single `## Open questions (from documentation)` block containing exactly:

  ```markdown
  - No candidates extracted from `<$INPUT_PATH>` — is this the correct input?
  ```

  No `## Constraints` block in this case.
- **Missing `$INPUT_PATH`** — the skill rejects this before dispatching into the brief; if somehow reached, halt with a diagnostic.

### Worked example

Fixture tree: [`./fixtures/analyze/documentation/`](./fixtures/analyze/documentation/).

Invocation (run from the fixture directory):

```
/change:analyze documentation ./inputs/ ./expected/
```

Input: a single runbook under [`inputs/ops-runbook.md`](./fixtures/analyze/documentation/inputs/ops-runbook.md) describing two operational procedures plus one deferred decision.

Expected output: two candidate blocks (alphabetical: `drain-backpressure-queue`, `rotate-upstream-ingest-key`), each with the unified fenced-YAML shape, plus a `## Constraints` block and a `## Open questions` block. See [`expected/discovery.md`](./fixtures/analyze/documentation/expected/discovery.md) for the byte-stable target.
