# Design format

Hard-coded conventions the fail-closed design parser enforces. These are not configurable.

`design.md` carries the technical shape the non-behavioural claims evidence. Open with a short `# Design` title and overview paragraph; the parser treats everything before the first H2 as preamble. The rest of the document is H2 sections drawn from one closed vocabulary, spelled exactly and kept in this relative order:

1. `## Overview` — what the system is and why, from `intent` and top-level `section` claims and the validated `spec.md`. Always present.
2. `## Domain model` — types and identifiers (`type` claims). Quote every `signature` **verbatim**; the parser looks for it, whitespace-insensitive, and refuses a paraphrase.
3. `## APIs and integrations` — external surfaces (`call` / `contract` claims, surface-naming requirements).
4. `## Technical logic` — delegation, validation, errors; fold abstracting `excerpt` claims.
5. `## UI / layout` — only with spatial claims (`region` / `container` / `leaf`), rendered as one tree.
6. `## Observability` — only when claims evidence metrics, traces, or logs.

## Section plan

The request lists every section with its presence — `required`, `permitted`, or `omit` — computed by the engine from the claim kinds present. Render every `required` section, never render an `omit` section, and render a `permitted` section only when claims inform it. No other H2 exists: an unknown heading, a duplicate, an out-of-order section, or a section with no body is refused.

## Invariants the parser re-checks

- **Requirement blocks stay in `spec.md`.** No `### Requirement:` or `#### Scenario:` heading appears in `design.md`, and no provenance lines; requirement-level honesty lives in `spec.md`'s tags ([tags.md](tags.md)). Refer to requirements by their `REQ-NNN` id inline instead.
- **Citations name bound sources.** Quote decisions as `(from <source>)`, where `<source>` is exactly one bound source key. A citation naming a key that is not bound is refused. Evidence `decision` claims are inputs that land here — there is no separate decision-record artifact.
- **Signatures are verbatim.** Every `type` claim's `signature` appears under `## Domain model` exactly as extracted.

Fold `decision` and `section` claims into the H2 they inform. Where the claims are silent, say nothing — never pad a section with invented architecture. No timestamps or run identifiers; re-runs must be byte-identical.
