# Design format

`design.md` carries the technical shape the non-behavioural claims evidence. Include **only** the H2 sections that claims inform — omit empty sections entirely. When present, keep this relative order:

1. `## Overview` — what the system is and why, from `intent` and top-level `section` claims.
2. `## Domain model` — types and identifiers (`type` claims; quote `signature` verbatim).
3. `## APIs and integrations` — external surfaces (`call` / `contract` claims, surface-naming requirements).
4. `## Technical logic` — delegation, validation, errors; fold abstracting `excerpt` claims.
5. `## UI / layout` — only with spatial claims (`region` / `container` / `leaf`), rendered as one tree.
6. `## Observability` — only when claims evidence metrics, traces, or logs.

Fold `decision` and `section` claims into the H2 they inform; quote decisions as `(from <source>)`. Evidence `decision` claims are inputs that land here — there is no separate decision-record artifact.

No provenance lines in `design.md`; requirement-level honesty lives in `spec.md`'s tags ([tags.md](tags.md)). Where the claims are silent, say nothing — never pad a section with invented architecture. No timestamps or run identifiers; re-runs must be byte-identical.
