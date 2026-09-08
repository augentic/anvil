# Requirement block

Every requirement in `spec.md` is one block the engine renders from its row and your draft: heading, three provenance lines, body, templated notes, and at least one scenario. You draft the body paragraphs and the scenarios for each subject; everything else is rendered.

## What the engine renders

```markdown
### Requirement: <subject>[ <tag>]

ID: REQ-<NNN>
Sources: [<source>, <source>, …]
Status: <agreed|unknown|conflict|divergence>

<your body paragraphs>

Note: <templated loser and gap lines, where the row has them>

#### Scenario: <your scenario name>

- **GIVEN** <your context, optional>
- **WHEN** <your trigger or input>
- **THEN** <your expected behaviour>
```

## What you draft

One entry per row, keyed by the row's `subject` exactly as listed:

- **`body`** — one or more Markdown paragraphs, each a string. One requirement, one behavioural assertion. Empty for a `conflict` row (see [authority.md](authority.md)); at least one paragraph otherwise.
- **`scenarios`** — at least one, each with a `name`, optional `given` lines, a `when`, and a `then`, all single lines. Draft from the `criterion` claims covering the row; for an uncovered row, a scenario that states what is checked without inventing the outcome.

The engine refuses a draft that omits a row, drafts a subject that is not a row, drafts a subject twice, gives a conflict row a body, gives any other row none, omits a scenario, or opens a paragraph line with `#`, `ID:`, `Sources:`, `Status:`, or `Note:`.

## Body conventions

- **Verbatim source language where possible.** Quote documentation claims lightly normalised; paraphrase behaviour claims into present-tense system prose.
- **No commentary about provenance.** Winners, losers, and gaps are the engine's notes; do not restate them.
- **No invented citations.** Cite only claims present in the request's Evidence.
