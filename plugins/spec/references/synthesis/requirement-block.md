# Requirement block

Every requirement in `spec.md` is one H3 block with three required provenance lines plus a body. The provenance parser (consumed by `specify slice validate`) enforces this shape exactly — any deviation fails the slice in `refining`.

## Canonical template

```markdown
### Requirement: <Human-readable name>[ <tag>]

ID: REQ-<NNN>
Sources: [<source-key>, <source-key>, …]
Status: <agreed|unknown|conflict|divergence>

<Requirement body — one or more paragraphs, optionally followed by `Note: …` lines for conflict/divergence commentary.>
```

Rules the parser enforces:

- **`ID:`** matches `^REQ-\d{3}$`. Zero-padded three-digit suffix, no gaps required, but each id MUST be unique inside one `spec.md`. Numbering starts at `REQ-001` per slice.
- **`Sources:`** is a YAML-flow list of kebab-case source keys. Every key MUST resolve against the slice's `plan.yaml.slices[].sources[]` bindings. `[]` is legal only when `Status: unknown`. Highest-authority key first.
- **`Status:`** is one of the closed enum `agreed | unknown | conflict | divergence`. Snake-case or any other casing fails.
- **Tag coherence:** the headline tag (`[unknown]` / `[conflict]` / `[divergence]`) MUST match `Status:` per [`tags.md`](tags.md). `Status: agreed` carries no tag; the other three Status values carry their matching tag verbatim.
- **Block boundary:** the H3 heading starts a new block; the next H2 or H3 closes it. Anything between provenance lines and the next heading is the body.

## Worked examples per Status

### `agreed` — single source

```markdown
### Requirement: User registration accepts valid email

ID: REQ-001
Sources: [legacy-monolith]
Status: agreed

The system accepts a registration request when the email field is RFC-5322 valid.
```

### `agreed` — multiple sources agree

```markdown
### Requirement: User registration accepts valid email

ID: REQ-001
Sources: [identity-design-notes, legacy-monolith]
Status: agreed

The system accepts a registration request when the email field is RFC-5322 valid.
```

### `divergence` — authority-resolved disagreement

```markdown
### Requirement: Reset link expiry [divergence]

ID: REQ-007
Sources: [identity-design-notes, legacy-monolith]
Status: divergence

The system expires password reset links after 30 minutes. (from identity-design-notes; documentation)

Note: legacy-monolith observed 24-hour expiry; the documentation authority overrides. Operator review recommended.
```

### `conflict` — tied-authority disagreement

```markdown
### Requirement: Reset link expiry [conflict]

ID: REQ-007
Sources: [product-notes, identity-design-notes]
Status: conflict

Note: product-notes says reset links expire after 30 minutes.
Note: identity-design-notes says reset links expire after 60 minutes.

Operator reconciliation required before /spec:build.
```

### `unknown` — no contributing Evidence

```markdown
### Requirement: Reset link single-use [unknown]

ID: REQ-008
Sources: []
Status: unknown

No contributing source supplied a claim for this requirement. Operator review required.
```

## Body conventions

- **One requirement, one behavioural assertion.** Split compound behaviours (`"…and the system also…"`) into separate `REQ-NNN` blocks so each carries its own provenance.
- **Verbatim source language where possible.** Quote requirement / criterion claims from `documentation` Evidence as written; lightly normalise capitalisation and terminal punctuation only. Behavioural `excerpt` claims paraphrase into present-tense system prose.
- **`Note:` lines carry commentary, never operative requirements.** A `[divergence]` block's body is the winning value; the loser sits in a `Note:` line below. A `[conflict]` block has only `Note:` lines plus an operator-reconciliation prompt.
- **No invented citations.** Do not add a `Sources:` key that did not contribute a claim. Do not promote a source key by hand to gain authority — emit the `[conflict]` and let the operator reconcile.

## Failure modes the parser surfaces

| Symptom                                                       | Fix                                                                                         |
| ------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `ID:` not matching `^REQ-\d{3}$` (e.g. `REQ-7`, `req-007`)    | Zero-pad to three digits; uppercase `REQ`.                                                  |
| `Sources:` key not in `plan.yaml.slices[].sources[]`          | Use one of the slice's bound source keys, or amend the plan to add the source.              |
| `Status:` outside the closed enum                             | Map to `agreed | unknown | conflict | divergence`.                                          |
| Tag in headline disagrees with `Status:`                      | Make them match per [`tags.md`](tags.md).                                                   |
| `Sources: []` with `Status:` anything other than `unknown`    | A non-`unknown` requirement always has at least one contributing source.                    |
| Duplicate `REQ-NNN` ids inside one `spec.md`                  | Renumber so each block is unique.                                                           |
