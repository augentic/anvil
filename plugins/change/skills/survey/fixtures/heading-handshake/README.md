# `heading-handshake`

Fixture proving the discovery-brief → survey heading handshake. The `## Candidate inventory` heading is emitted exactly once by the discovery brief; `/change:survey` appends candidate blocks under it without re-emitting the heading.

This fixture exercises the post-CLI discovery handshake, not the CLI ingest itself, so it deliberately ships only the staged candidate plus the discovery files — no `sources.yaml` and no source-tree stub. The shape of `inputs/staged/<source-key>.json` matches the staging convention used by the other survey fixtures so reviewers can read both fixture families with a single mental model.

## Contents

- [`inputs/discovery.md`](inputs/discovery.md) — a discovery file with the `## Candidate inventory` heading already written by the discovery brief, plus one pre-existing documentation-derived candidate block from `/change:analyze`.
- [`inputs/staged/legacy-api.json`](inputs/staged/legacy-api.json) — staged candidate `surfaces.json` representing one source with two surfaces; shape-consistent with the other fixtures even though no CLI invocation is exercised here.
- [`expected/discovery.md`](expected/discovery.md) — the expected `discovery.md` after `/change:survey` appends one survey-derived candidate block under the pre-existing heading. The heading appears exactly once.

The fixture is byte-stable: the expected output is deterministic on unchanged inputs.
