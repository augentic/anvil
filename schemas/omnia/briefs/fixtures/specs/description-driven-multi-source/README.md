# description-driven-multi-source

Demonstrates description-driven scope inference on a two-source change. The plan entry carries no `scope` field; the specs brief infers extract filters from the `description`'s file-path hints.

## Plan entry

See `plan-entry.yaml`.

## Expected behaviour

- For `monolith`: description mentions `src/ingest/` and `src/kafka/`, so the brief infers `--include src/ingest/** --include src/kafka/**` on `/spec:extract`.
- For `shared-lib`: no path hints in description for this source, so the brief runs extract on the full tree.
- Design sections are merged under `## Source: <key>` headings.
