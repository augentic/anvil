---
id: analyze
description: Per-kind clustering / extraction prompt for /spec:analyze (Omnia).
generates: .specify/plans/<name>/discovery.md
---

# Omnia per-kind prompts for `/spec:analyze`

This brief carries Omnia's clustering / extraction prompts for the
[`/spec:analyze`](../../../../plugins/spec/skills/analyze/SKILL.md)
skill. The skill resolves this file, dispatches on `$KIND`, and
executes exactly one of the branches below.

Both branches emit the same on-disk shape — one `### <name>` heading
per capability followed by a fenced YAML block. The normative
capability-summary contract lives in
[`analyze/SKILL.md` §Output contract](../../../../plugins/spec/skills/analyze/SKILL.md).
Do not invent new fields, drop required fields, or deviate from the
fixed field order.

**Fixed field order (hard contract):** inside each YAML block, fields
MUST appear in the order `summary`, `sources`, `depends-on`, `hints`,
`confidence`. Capabilities MUST be sorted alphabetically by name.
Within each capability, `sources`, `depends-on`,
`hints.entry_points`, and `hints.external_deps` MUST each be sorted
alphabetically.

---

## Documentation branch (`--kind documentation`)

Extract capability summaries from documentation: prose, PDFs,
runbooks, OpenAPI specs, AsciiDoc, Markdown design notes, exported
tickets. Identify the capabilities the docs describe, cite the
artifact paths as `sources:`, and emit a `confidence` marker
reflecting how cleanly each capability is specified. Also surface
constraints and open questions the docs carry alongside capability
descriptions.

### 1. Inventory the input

If `$INPUT_PATH` is a directory, walk it and enumerate artifacts by
recognisable type:

| Shape                  | Signals                                              |
| ---------------------- | ---------------------------------------------------- |
| Markdown / AsciiDoc    | `*.md`, `*.adoc` — follow `##` / `###` headings.     |
| OpenAPI                | `*.yaml` / `*.json` with `openapi:` or `swagger:`.   |
| Runbook / procedure    | Prose with imperative "Steps:" or numbered lists.    |
| PDF / exported brief   | `*.pdf` — extract text, reference sections by page.  |
| Ticket / RFC export    | Prose with explicit headings or labelled sections.   |

If `$INPUT_PATH` is a single file, treat it as the whole inventory.
Artifacts that fail to parse (unreadable PDF, malformed OpenAPI) halt
the brief with a diagnostic naming the offending file; never write a
partial `$DISCOVERY`.

### 2. Identify capabilities per artifact

A capability is a cohesive behavior the docs describe — *"what does
the system do for the user or operator?"* — not a file and not a
heading. Cluster heuristics:

- **Runbooks / procedures.** Each operational procedure (rotate a
  key, drain a queue, roll back a deploy) is one capability.
- **OpenAPI.** Group endpoints by tag or by shared path prefix. A
  `POST /users` + `GET /users/{id}` pair with a common `tag: users`
  is one capability (`user-directory`), not two. Use `operationId`
  or the tag's slug as a name seed.
- **Markdown / AsciiDoc.** Top-level capability headings (`## X`)
  typically map 1:1 to capabilities. Cluster sub-sections into their
  parent when the prose clearly describes one capability across
  them. Do not split a single capability just because its doc has
  many sub-headings.
- **Unstructured prose / PDFs.** Cluster by described behavior.
  Prefer capability names that match the noun-phrase the doc uses
  for the behavior (`user-onboarding`, not `flow-3`).

Capability names are kebab-case, 2–4 tokens, noun-phrases describing
the behavior.

### 3. Fill each capability block

For every identified capability, emit:

````markdown
### <capability-name>

```yaml
summary: <one-sentence imperative description>
sources:
  - <literal artifact path, optionally with fragment>
depends-on: [<other capability names>]
hints:
  entry_points: [<trigger / command / HTTP verb-path strings>]
  external_deps: [<named external systems>]
confidence: <high | medium | low>
```
````

Per-field rules:

- **`summary`** — single sentence, imperative voice, present tense.
  Describe the effect on the user or system, not the
  implementation. No trailing fluff.
- **`sources`** — literal artifact paths relative to `$INPUT_PATH`.
  Cite deep-links where the artifact supports them:
  - OpenAPI — JSON pointer per RFC 6901, e.g.
    `api-spec.yaml#/paths/~1users/post`.
  - Markdown / AsciiDoc — heading slug, e.g.
    `runbook.md#rotate-the-upstream-ingest-key` (GitHub slug rules:
    lowercase, hyphen-separated, punctuation stripped).
  - PDF — page fragment, e.g. `ops-runbook.pdf#page=12` (hint, not a
    guarantee).

  Sort alphabetically.
- **`depends-on`** — names of OTHER capabilities in this run's
  output that the docs explicitly reference (e.g. the runbook says
  "run `drain-backpressure-queue` before rotating the key"). If the
  docs do not explicitly link, leave the list empty. Sort
  alphabetically.
- **`hints.entry_points`** — named triggers the docs specify:
  commands (`rotate-ingest-key`), HTTP verb-paths (`POST /users`),
  events (`on-schedule:daily`), CLI subcommands. Prefer literal
  wording from the docs. Either omit the `entry_points` key entirely
  or emit a non-empty alphabetically-sorted list.
- **`hints.external_deps`** — named external systems the docs
  mention (`postgres`, `kafka`, `azure-key-vault`, `sendgrid`).
  Kebab-case. Either omit the `external_deps` key or emit a
  non-empty alphabetically-sorted list.
- **`confidence`** — certainty the capability is fully and correctly
  extracted:

  | value    | when                                                                                                                                        |
  | -------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
  | `high`   | Docs explicitly define the capability's boundary, at least one entry point, and the external systems it touches. Step-by-step, concrete.   |
  | `medium` | Capability is described but one of {boundary, entry point, external dep} is ambiguous or missing. A human should skim before accepting.    |
  | `low`    | Capability is implied or scattered across sections; the doc hand-waves structural details. Operator MUST review before the downstream slice. |

### 4. Extract constraints and open questions

Documentation inputs often carry rules and unresolved decisions
alongside capability descriptions. Collect them into two appendix
blocks AFTER the last capability in `$DISCOVERY`:

```markdown
## Constraints (from documentation)

- <constraint text> (source: <artifact path[#fragment]>)
- <constraint text> (source: <artifact path[#fragment]>)

## Open questions (from documentation)

- <question text> (source: <artifact path[#fragment]>)
- <question text> (source: <artifact path[#fragment]>)
```

Rules:

- Every entry cites the artifact path (and optional fragment) it
  came from, so humans can audit the extraction.
- Sort entries alphabetically by their leading text within each
  block.
- Omit a block entirely if it would be empty — do not emit an empty
  `## Constraints` heading.
- These blocks are documentation-branch-only. The code branch
  (landed in RFC-3a C21) does not emit them.

### 5. Idempotency

Follow the rules pinned in
[`analyze/SKILL.md` §Idempotency](../../../../plugins/spec/skills/analyze/SKILL.md):

- Capabilities sorted alphabetically by name.
- Fixed field order inside each YAML block (§Step 3 above).
- `sources`, `depends-on`, `hints.entry_points`,
  `hints.external_deps` sorted alphabetically.
- No timestamps, environment variables, absolute paths, or run IDs
  in `$DISCOVERY`.
- Re-running against unchanged inputs produces byte-identical
  output.

When `$SOURCE_KEY` is supplied, the skill (not this brief) prepends
the `<!-- source-key: $SOURCE_KEY -->` marker before each `### <name>`
heading this run produces. Do not emit the marker from the brief.

### 6. Error and empty-input handling

- **Unreadable artifact** — halt with a diagnostic naming the file;
  never write a partial `$DISCOVERY`.
- **Empty capability inventory** (input parseable but no
  capabilities discernable) — write `$DISCOVERY` with no `###`
  blocks, plus a single `## Open questions (from documentation)`
  block containing exactly:

  ```markdown
  - No capabilities extracted from `<$INPUT_PATH>` — is this the correct input?
  ```

  No `## Constraints` block in this case.
- **Missing `$INPUT_PATH`** — the skill rejects this before
  dispatching into the brief; if somehow reached, halt with a
  diagnostic.

### Worked example

Fixture tree:
[`../fixtures/plan/analyze/documentation/`](../fixtures/plan/analyze/documentation/).

Invocation (run from the fixture directory):

```
/spec:analyze --kind documentation ./inputs/ ./expected/
```

Input: a single runbook under
[`inputs/ops-runbook.md`](../fixtures/plan/analyze/documentation/inputs/ops-runbook.md)
describing two operational procedures plus one deferred decision.

Expected output: two capability summaries (alphabetical:
`drain-backpressure-queue`, `rotate-upstream-ingest-key`), each with
`confidence: high`, plus a `## Constraints` block and a
`## Open questions` block. See
[`expected/discovery.md`](../fixtures/plan/analyze/documentation/expected/discovery.md)
for the byte-stable target.

---

## Legacy-code branch (`--kind legacy-code`)

This branch infers capability boundaries from source-tree structure
and emits one capability summary per inferred capability. Output
lives at `$DISCOVERY` (capability summaries) and at
`<plan-dir>/analyze/<$SOURCE_KEY>/metadata.json` (per-source
structural facts).

### 1. Inputs

Expect `$INPUT_PATH` to be a local directory containing a source
tree (cloned or symlinked by the discovery brief). Skim the tree to
build the clustering signals below; do not require network access,
a build step, or a language runtime.

### 2. Clustering signals

Combine the following signals to draw capability boundaries. No
single signal is load-bearing; capabilities emerge from
concordance.

| Signal         | Weight | Details                                                                                                             |
| -------------- | ------ | ------------------------------------------------------------------------------------------------------------------- |
| Import graph   | Strong | Modules that cluster in a tight import SCC are likely one capability.                                               |
| Endpoint names | Strong | HTTP / CLI / message-handler names that share a prefix (e.g. `POST /users`, `POST /users/verify`) are one capability. |
| Docstrings     | Medium | First-paragraph summaries on modules, classes, or handlers that name a behaviour.                                   |
| Test names     | Medium | `describe('user registration', …)`, `test_user_registration_*` patterns bound behaviour.                            |
| READMEs        | Medium | Per-module or per-directory READMEs often list capabilities explicitly.                                             |

Apply Omnia's conventions:

- Prefer capabilities that align with the "one crate per capability"
  rule downstream: a cohesive HTTP surface + its domain model + its
  data access.
- Shared utilities that serve multiple capabilities become their
  own capability (kebab-case, e.g. `shared-validation`).
- Deprecated or vendored code goes out of scope (the propose brief
  later calls this out via `scope.<k>.exclude`).

Capability names are kebab-case, 2–4 tokens, noun-phrases describing
the behaviour.

### 3. Per-capability output

Emit one capability summary per inferred capability, in the on-disk
shape pinned by
[`analyze/SKILL.md` §Output contract](../../../../plugins/spec/skills/analyze/SKILL.md)
— `### <name>` heading followed by a fenced YAML block.

Fields, in strict order:

- **`summary`** — single-sentence imperative ("Create new user
  accounts with email verification."). Derived from the capability's
  dominant docstring, README entry, or endpoint-cluster description.
- **`sources`** — **file-hint list**: source files this capability
  substantively inhabits, relative to `$INPUT_PATH`, alphabetically
  sorted. This list becomes `scope.<key>.include` downstream (see
  [`rfc-3a-monoliths.md` §*Scoped extraction on monoliths*](../../../../rfcs/rfc-3a-monoliths.md)).
  Include every file that substantively implements the capability;
  exclude pure test files and imports-only glue.
- **`depends-on`** — names of OTHER capabilities this one imports or
  calls. Alphabetically sorted. Empty list allowed.
- **`hints.entry_points`** — HTTP routes, CLI commands,
  message-broker topics, scheduled triggers. Alphabetically sorted.
  Either omit the key entirely or emit a non-empty list.
- **`hints.external_deps`** — external systems (databases, queues,
  identity providers, third-party APIs) inferred from imports and
  configuration. Kebab-case names (`postgres`, `sendgrid`,
  `azure-ad`). Alphabetically sorted. Either omit the key entirely
  or emit a non-empty list.
- **`confidence`** — one of `high | medium | low`:

  | value    | when                                                                                                                                            |
  | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
  | `high`   | Clean file boundary (all sources under one subdirectory or a small explicit fan-out), a documented entry point, and a clear import-graph cluster. |
  | `medium` | Boundary is plausible but one signal is fuzzy — scattered files, ambiguous name, or weak imports. A human should skim before accepting.         |
  | `low`    | Capability is implied — files span multiple directories, entry points unclear, or the clustering required a judgement call. Propose flags it for review. |

Do NOT emit `## Constraints` or `## Open questions` appendix blocks
from this branch — those are documentation-only (see §*Documentation
branch* Step 4). The code branch has no equivalent source of prose
rules or deferred decisions.

### 4. Structural metadata (`metadata.json`)

In addition to capability summaries, write
`<plan-dir>/analyze/<$SOURCE_KEY>/metadata.json` with the shape
pinned in
[`analyze/SKILL.md` §Structural metadata](../../../../plugins/spec/skills/analyze/SKILL.md).
Omnia conventions for the numeric fields:

- **`language`** — detected primary source language, kebab-case
  (`typescript`, `javascript`, `rust`, `go`, `python`, `java`,
  `kotlin`, `csharp`). Prefer the language with the largest share
  of non-generated LOC.
- **`loc`** — total non-blank non-comment source lines. Exclude
  test files, vendored dependency directories (`node_modules`,
  `vendor`, `target`, `.venv`), and generated code (`*.gen.ts`,
  `*_pb.go`, etc.).
- **`module_count`** — total module count. Omnia convention:
  - TypeScript / JavaScript: source files under the primary source
    tree (exclude tests, `*.d.ts`, build output).
  - Rust: crates; for a single-crate repo, count top-level `mod`
    declarations.
  - Go: packages.
  - Python: modules (`.py` files and `__init__.py`-bearing
    directories).
  - Java / Kotlin / C#: top-level types (classes + interfaces)
    under the primary source tree.
- **`top_level_modules`** — immediate children of the source root
  that are directories, alphabetically sorted, relative paths
  (`src/auth`, `src/ingest`). Flat-layout projects with code at the
  root produce an empty array.

The documentation branch MUST NOT write this file — see
[`analyze/SKILL.md` §*Error handling*](../../../../plugins/spec/skills/analyze/SKILL.md).

### 5. Idempotency

Same rules as the documentation branch (§*Documentation branch*
Step 5) plus:

- `top_level_modules` in `metadata.json` is alphabetically sorted.
- Numeric fields (`loc`, `module_count`) are deterministic for a
  given input tree — re-running on unchanged sources produces
  byte-identical counts.
- No timestamps, environment variables, absolute paths, or run IDs
  in either output.

Reruns on unchanged inputs produce byte-identical `discovery.md`
and `metadata.json`.

### 6. Error and empty-input handling

- **`$INPUT_PATH` absent or unreadable** — hard exit before writing
  either output; never ship a partial `$DISCOVERY` or
  `metadata.json`.
- **Empty capability inventory** (tree parseable but no cohesive
  capabilities inferable) — still emit `$DISCOVERY` with no `###`
  blocks, AND emit `metadata.json` with accurate structural counts.
  An empty project is legal.
- **Unknown `language`** (detected value not in the Omnia
  convention list above) — use the detected value verbatim in
  `metadata.json` and let the propose brief's review list surface
  it; do NOT block the run.

### Worked example

Fixture tree:
[`../fixtures/plan/analyze/legacy-code/`](../fixtures/plan/analyze/legacy-code/).

Invocation (run from the fixture directory):

```
/spec:analyze --kind legacy-code --source-key monolith ./inputs/monolith/ ./expected/plans/legacy-code/
```

Input: a small TypeScript monolith under
[`inputs/monolith/`](../fixtures/plan/analyze/legacy-code/inputs/monolith/)
with four inferable capabilities spanning `src/users`, `src/auth`,
`src/common`, and `src/billing`.

Expected output: four capability summaries
(alphabetical: `billing-subscription`, `email-verification`,
`shared-validation`, `user-registration`) plus a structural-metadata
sidecar. The `user-registration` block reproduces the canonical
sample from
[`rfc-3a-monoliths.md` §*Plan-time analysis, define-time extraction*](../../../../rfcs/rfc-3a-monoliths.md)
in the on-disk shape. See
[`expected/discovery.md`](../fixtures/plan/analyze/legacy-code/expected/discovery.md)
and
[`expected/plans/legacy-code/analyze/monolith/metadata.json`](../fixtures/plan/analyze/legacy-code/expected/plans/legacy-code/analyze/monolith/metadata.json)
for the byte-stable targets.
