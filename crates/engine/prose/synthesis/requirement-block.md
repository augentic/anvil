# Requirement block

Every requirement in `spec.md` is one H3 block: heading, three provenance lines, body, and at least one scenario. You author the body prose and scenarios; the heading name, heading tag, and provenance lines come **verbatim** from the requirement's reconciliation row. The heading name is the row's subject — the dotted-kebab claim id (`greeting.behaviour`), or `<id> acceptance criteria` for an appended gap row — never a prose title.

## Canonical template

```markdown
### Requirement: <subject>[ <tag>]

ID: REQ-<NNN>
Sources: [<source>, <source>, …]
Status: <agreed|unknown|conflict|divergence>

<Requirement body — one or more paragraphs, optionally followed by `Note: …` lines for conflict/divergence commentary.>

#### Scenario: <Scenario name>

- **WHEN** <trigger or input>
- **THEN** <expected behavior>
```

Every requirement **must** include ≥1 `#### Scenario:` heading — including `[unknown]` evidence-gap blocks. GIVEN is optional context before WHEN/THEN. Multiple scenarios may appear in one block. Heading levels are fixed; see [spec-format.md](spec-format.md).

Invariants the engine's parser re-checks fail-closed:

- **Heading name:** exactly the row's subject. Re-runs diff generations by this name, so a rewritten heading is refused as a renamed requirement.
- **`ID:`** matches `^REQ-[0-9]{3}$`, unique across the document, exactly the row's minted id.
- **`Sources:`** kebab-case keys in the row's order. `[]` only with `Status: unknown`.
- **`Status:`** the closed enum `agreed | unknown | conflict | divergence`, exactly the row's value.
- **Tag coherence:** the heading tag mirrors `Status:` per [tags.md](tags.md); `agreed` carries no tag.

## Worked `unknown` — evidence gap

```markdown
### Requirement: auth.password-reset acceptance criteria [unknown]

ID: REQ-002
Sources: []
Status: unknown

A password reset flow exists; its acceptance behaviour is not evidenced.

#### Scenario: Password reset requested

- **WHEN** a user requests a password reset
- **THEN** behaviour is unspecified — the operator must supply acceptance criteria
```

## Body conventions

- **One requirement, one behavioural assertion.** Split compound behaviours into separate blocks — but never invent a block without a reconciliation row.
- **Verbatim source language where possible.** Quote documentation claims lightly normalised; paraphrase behaviour `excerpt` claims into present-tense system prose.
- **`Note:` lines carry commentary, never operative requirements.** Winner as body plus one `Note:` per loser for `divergence`; only `Note:` lines for `conflict` (see [authority.md](authority.md)).
- **No invented citations.** Cite only claims present in the request's Evidence.
