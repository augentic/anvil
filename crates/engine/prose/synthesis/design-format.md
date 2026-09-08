# Design format

Hard-coded conventions of the rendered `design.md`. These are not configurable.

`design.md` carries the technical shape the non-behavioural claims evidence. The engine renders `# Design`, your `preamble` paragraphs, then one `## ` section per drafted entry, in this fixed order whatever order you answer in:

1. `overview` → `## Overview` — what the system is and why, from `intent` and top-level `section` claims and the rendered `spec.md`. Always required.
2. `domain-model` → `## Domain model` — types and identifiers. Reference every `type` claim exactly once as a `type` block; the engine inserts its `signature` verbatim as a fenced code block.
3. `apis` → `## APIs and integrations` — external surfaces (`call` / `contract` claims, surface-naming requirements).
4. `technical-logic` → `## Technical logic` — delegation, validation, errors; fold abstracting `excerpt` claims.
5. `ui-layout` → `## UI / layout` — only with spatial claims (`region` / `container` / `leaf`), rendered as one tree.
6. `observability` → `## Observability` — only when claims evidence metrics, traces, or logs.

## Section plan

The request lists every section with its presence — `required`, `permitted`, or `omit` — computed by the engine from the claim kinds present. Draft every `required` section, never an `omit` section, and a `permitted` section only when claims inform it. Each drafted section carries at least one block.

## Blocks

- **`{"text": "…"}`** — one Markdown paragraph. Cite decisions inline as `(from <source>)`, where `<source>` is exactly one bound source key; a citation naming an unbound key is refused. Refer to requirements by their `REQ-NNN` id inline — no `### Requirement:` or `#### Scenario:` heading and no provenance line belongs here.
- **`{"type": "<key>"}`** — a `type` claim by its id (or path when it has none), under `domain-model` only. Interleave with text blocks to place each signature; every listed type claim is referenced exactly once, or the draft is refused.

Fold `decision` and `section` claims into the section they inform. Where the claims are silent, say nothing — never pad a section with invented architecture. No timestamps or run identifiers; re-runs must be byte-identical.
