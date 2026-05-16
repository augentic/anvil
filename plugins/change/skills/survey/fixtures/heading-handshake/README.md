# `heading-handshake`

Fixture proving the discovery-brief → survey heading handshake. The `## Candidate inventory` heading is emitted exactly once by the discovery brief; `/change:survey` appends candidate blocks under it without re-emitting the heading.

## Contents

- [`inputs/discovery.md`](inputs/discovery.md) — a discovery file with the `## Candidate inventory` heading already written by the discovery brief, plus one pre-existing documentation-derived candidate block from `/change:analyze`.
- [`inputs/surfaces.json`](inputs/surfaces.json) — a small `surfaces.json` representing one source with two surfaces.
- [`inputs/metadata.json`](inputs/metadata.json) — matching metadata for the source.
- [`expected/discovery.md`](expected/discovery.md) — the expected `discovery.md` after `/change:survey` appends one survey-derived candidate block under the pre-existing heading. The heading appears exactly once.

The fixture is byte-stable: the expected output is deterministic on unchanged inputs. The full acceptance fixture set lands in Change G.
