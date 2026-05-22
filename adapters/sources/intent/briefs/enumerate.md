# intent.enumerate

Emit exactly one candidate block under `## Candidate inventory` in `discovery.md`. The `intent` source is degenerate by construction — it does not crawl, parse, or infer anything. It echoes the operator's intent string into the candidate that backs the slice driving the plan.

## Inputs

- `Source` — the `plan.yaml.sources.<source-key>` binding bound to this adapter. `Source.value` carries the operator's free-form intent string. `Source.path` is absent for `intent` bindings; no filesystem root is preopened.
- `slice-name` — the kebab-case identifier `/spec:plan` derived for the candidate's slice. Used verbatim as the candidate `id`.

## Output contract

Append (or replace by `id`) one block under `## Candidate inventory` in `discovery.md`:

```markdown
### <slice-name>

- id: <slice-name>
- sources: [<source-key>]
- summary: <Source.value, one line, verbatim>
```

Rules:

- `id` MUST equal `slice-name`. The bare-string `Slice.sources` shorthand `[<source-key>]` in `plan.yaml` only normalises cleanly when the candidate id matches the slice name.
- `sources` MUST be a one-element list carrying the source key the binding was registered under (typically `intent`, but operators MAY bind a second intent source under a different key).
- `summary` MUST be the operator's intent string on a single line. Collapse internal whitespace to single spaces; do not paraphrase, truncate, or annotate. If the operator supplied multi-line prose, fold it to one line.
- Do not set `tentative`. The intent source surfaces exactly one candidate per slice; cross-source merge ambiguity is a `/spec:plan` propose-time concern, not an enumerate concern.

## Worked example

Input — `plan.yaml.sources.intent.value`:

```
Add a search filter to the user list.
```

Slice name: `add-search-filter`.

Output — block appended under `## Candidate inventory` in `discovery.md`:

```markdown
### add-search-filter

- id: add-search-filter
- sources: [intent]
- summary: Add a search filter to the user list.
```

## Notes

- Re-running `intent.enumerate` against the same source replaces the candidate by `id`. Editing the intent string and re-running yields the same candidate id with an updated summary.
- `discovery.md`'s `## Summary` and `## Source inventory` sections are owned by `/spec:plan`, not this brief; this brief only writes inside `## Candidate inventory`.
