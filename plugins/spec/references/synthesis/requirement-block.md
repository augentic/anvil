# Requirement block

Every requirement in a spec file (`specs/<domain>/spec.md`) is one H3 block with three provenance lines plus a body. **The agent authors only the heading and body prose** (plus the requirement's `(source, id, kind)` claims and `agreement` verdict in the response); `specify slice synthesize` **renders the `ID:` / `Sources:` / `Status:` lines and the headline tag** from `model.yaml`. The provenance parser (consumed by `specify slice validate`) validates the rendered shape exactly — an operator hand-edit that stales a kernel-rendered line fails `slice-spec-provenance-stale`.

## Canonical template (kernel-rendered)

```markdown
### Requirement: <Human-readable name>[ <tag>]

ID: REQ-<NNN>
Sources: [<source>, <source>, …]
Status: <agreed|unknown|conflict|divergence>

<Requirement body — one or more paragraphs, optionally followed by `Note: …` lines for conflict/divergence commentary.>

#### Scenario: <Scenario name>

- **WHEN** <trigger or input>
- **THEN** <expected behavior>
```

The `#### Scenario:` heading is optional per requirement block — include it when the requirement has meaningful acceptance criteria. GIVEN is optional context that precedes the WHEN/THEN pair. Multiple `#### Scenario:` headings may appear within one requirement block. The heading level is fixed at H4; see [`spec-format.md`](../spec-format.md) for the canonical heading conventions.

Invariants the kernel guarantees and the parser re-checks:

- **`ID:`** matches `^REQ-\d{3}$`. Zero-padded three-digit suffix; each id is unique across the whole slice. **New domains** (no baseline `specs/<domain>/spec.md`) receive slice-global `REQ-001..N` in declaration order. **Modified domains** (baseline exists) assign additive requirements from `max(baseline REQ)+1` per domain; set `baseline-id` in the synthesis response when refining an existing baseline requirement — the kernel preserves that id and renders the block under `## MODIFIED Requirements`.
- **`Sources:`** is a YAML-flow list of kebab-case source keys, every key resolving against the slice's `plan.yaml.slices[].sources[]` bindings, highest-authority key first. `[]` appears only when `Status: unknown`.
- **`Status:`** is one of the closed enum `agreed | unknown | conflict | divergence`.
- **Tag coherence:** the headline tag (`[unknown]` / `[conflict]` / `[divergence]`) matches `Status:` per [`tags.md`](tags.md). `Status: agreed` carries no tag; the other three Status values carry their matching tag verbatim.
- **Block boundary:** the H3 heading starts a new block; the next H2 or H3 closes it. Anything between provenance lines and the next heading is the body.

## Worked examples per Status

These show the kernel's **rendered output**. In the response the agent supplies only the heading text, body prose, scenarios, and the requirement's `(source, id, kind)` claims plus its `agreement` verdict; the kernel projects the `ID:` / `Sources:` / `Status:` lines and the headline tag.

### `agreed` — single source with scenario

```markdown
### Requirement: User registration accepts valid email

ID: REQ-001
Sources: [legacy-monolith]
Status: agreed

The system accepts a registration request when the email field is RFC-5322 valid.

#### Scenario: Valid email accepted

- **WHEN** a registration request arrives with email `user@example.com`
- **THEN** the system creates the account and returns 201
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

## Modified domains and merge-ready deltas

When the bound project already owns `specs/<domain>/spec.md`, the kernel renders merge-ready deltas — not flat requirement blocks:

- Net-new behaviour in the slice → `## ADDED Requirements` with ids continuing from the baseline max (`REQ-004` after `REQ-001..003`, etc.).
- Refining an existing baseline requirement → set `baseline-id: REQ-NNN` on that requirement in the synthesis response; the kernel keeps the id and renders the block under `## MODIFIED Requirements`.

Greenfield domains (no baseline file) still render flat `### Requirement:` blocks. `specify slice merge` rejects flat deltas against a non-empty baseline (`merge-delta-headers-required`) so requirement changes cannot be silently dropped.

## Body conventions

- **One requirement, one behavioural assertion.** Split compound behaviours (`"…and the system also…"`) into separate `REQ-NNN` blocks so each carries its own provenance.
- **Verbatim source language where possible.** Quote requirement / criterion claims from `documentation` Evidence as written; lightly normalise capitalisation and terminal punctuation only. Behavioural `excerpt` claims paraphrase into present-tense system prose.
- **`Note:` lines carry commentary, never operative requirements.** For a `disagreed` requirement, write the winning value as the body and preserve each loser as a `Note:` line; for a tied conflict, write only `Note:` lines plus an operator-reconciliation prompt. (The kernel renders the `[divergence]` / `[conflict]` tag from the verdict and resolved authority.)
- **No invented citations.** Only cite a `(source, id, kind)` claim that the inputs-envelope Evidence actually carries — a claim referencing an absent `(source, id)` fails projection with `slice-model-source-orphan`. Never try to author `Sources:` lists or `winner` markers; the kernel projects them.

## Failure modes the parser surfaces

The kernel renders the provenance lines, so these arise only from a **post-synthesis hand-edit** (caught by `slice-spec-provenance-stale`) — the fix is to re-run `specify slice synthesize` rather than to hand-correct the line:

| Symptom                                                       | Cause                                                                                       |
| ------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `ID:` not matching `^REQ-\d{3}$` (e.g. `REQ-7`, `req-007`)    | A hand-edit corrupted a kernel-assigned id.                                                 |
| `Sources:` key not in `plan.yaml.slices[].sources[]`          | A hand-edit added a key the kernel never rendered.                                          |
| `Status:` outside the closed enum                             | A hand-edit replaced the kernel-rendered value.                                             |
| Tag in headline disagrees with `Status:`                      | A hand-edit changed one without the other.                                                  |
| `Sources: []` with `Status:` anything other than `unknown`    | A non-`unknown` requirement always renders at least one contributing source.                |
| Duplicate `REQ-NNN` ids inside the slice                      | A hand-edit duplicated a kernel-assigned id.                                                |
