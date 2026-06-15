# intent.survey

Emit exactly one lead block under `## Lead inventory` in `discovery.md`. The `intent` source is degenerate by construction — it does not crawl, parse, or infer anything. It echoes the operator's intent string into the lead that backs the slice driving the plan.

## Inputs

- `Source` — the `plan.yaml.sources.<source>` binding bound to this adapter. `Source.value` carries the operator's free-form intent string. `Source.path` is absent for `intent` bindings; no filesystem root is preopened.
- `slice-name` — the kebab-case identifier `/spec:plan` derived for the lead's slice. Used verbatim as the lead `lead`.

## Output contract

Append (or replace by `lead`) one block under `## Lead inventory` in `discovery.md`:

```markdown
### <slice-name>

- lead: <slice-name>
- synopsis: <Source.value, one line, verbatim>
```

Rules:

- `lead` MUST equal `slice-name`. The bare-string `Slice.sources` shorthand `[<source>]` in `plan.yaml` only normalises cleanly when the lead matches the slice name.
- Do not emit `source`. The CLI stamps each lead's `source` from the survey binding (the key the source was registered under, typically `intent`); attribution is CLI-owned.
- `synopsis` MUST be the operator's intent string, verbatim. Collapse internal whitespace to single spaces; do not paraphrase, truncate, or annotate. A multi-line intent MAY stay multi-line when folding to one line would lose discriminating content the operator wrote.
- Do not emit `topics`. The optional per-lead `topics` field is for sources that read and classify material; `intent` infers nothing, so it always omits the bullet.

## Worked example

Input — `plan.yaml.sources.intent.value`:

```
Add a search filter to the user list.
```

Slice name: `add-search-filter`.

Output — block appended under `## Lead inventory` in `discovery.md`:

```markdown
### add-search-filter

- lead: add-search-filter
- synopsis: Add a search filter to the user list.
```

## Notes

- Re-running `intent.survey` against the same source replaces the lead by its `(source, lead)` pair. Editing the intent string and re-running yields the same lead with an updated synopsis.
- The single lead becomes the slice driving the plan; see [From sources to slices](../references/spec-runtime/reconciliation.md#plan-time-leads-become-slices) for how leads reconcile into slices.
- `discovery.md`'s `## Summary` and `## Source inventory` sections are owned by `/spec:plan`, not this brief; this brief only writes inside `## Lead inventory`.
