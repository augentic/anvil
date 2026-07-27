# Requirement block

Every requirement in a spec file (`specs/<domain>/spec.md`) is one H3 block with three provenance lines plus a body. **The agent authors only the heading and body prose** (plus the requirement's `(source, id, kind)` claims and `agreement` verdict in the response); the `emery slice refine` synthesis kernel **renders the `ID:` / `Sources:` / `Status:` lines and the headline tag** from `model.yaml`. The provenance parser (consumed by `emery slice validate`) validates the rendered shape exactly — an operator hand-edit that stales a kernel-rendered line fails `slice-spec-provenance-stale`.

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

Every requirement **must** include ≥1 `#### Scenario:` heading — including `Status: unknown` / evidence-gap requirements. GIVEN is optional context before WHEN/THEN. Multiple `#### Scenario:` headings may appear within one block. Heading level is fixed at H4; see [`spec-format.md`](spec-format.md).

Invariants the kernel guarantees and the parser re-checks:

- **`ID:`** matches `^REQ-\d{3}$`. Unique across the slice. **New domains** receive slice-global `REQ-001..N` in declaration order. **Modified domains** assign additive ids from `max(baseline REQ)+1` per domain; set `baseline-id` when refining an existing baseline requirement — the kernel preserves that id under `## MODIFIED Requirements`.
- **`Sources:`** YAML-flow kebab-case keys resolving against `plan.yaml.slices[].sources[]`, highest-authority first. `[]` only when `Status: unknown`.
- **`Status:`** closed enum `agreed | unknown | conflict | divergence`.
- **Tag coherence:** headline tag matches `Status:` per [`tags.md`](tags.md). `Status: agreed` carries no tag.
- **Block boundary:** H3 starts a block; the next H2 or H3 closes it.

## Worked examples

Kernel-rendered output. Agent supplies heading, body, scenarios, claims, and `agreement`; kernel projects provenance lines and the headline tag. Other statuses follow the same template with tag/`Status:`/`Sources:` as in [`authority.md`](authority.md) and [`tags.md`](tags.md).

### `divergence`

```markdown
### Requirement: Session timeout [divergence]

ID: REQ-001
Sources: [docs, code]
Status: divergence

The system expires idle sessions after 30 minutes. (from docs; documentation)

Note: code observed 15-minute expiry; the documentation authority overrides. Operator review recommended.

#### Scenario: Idle session expires

- **WHEN** a session is idle for 30 minutes
- **THEN** the system expires the session
```

### `unknown` — evidence gap

```markdown
### Requirement: password reset behaviour [unknown]

ID: REQ-001
Sources: []
Status: unknown

A password reset flow exists; its behaviour is not evidenced.

#### Scenario: Password reset requested

- **WHEN** a user requests a password reset
- **THEN** behaviour is unspecified — operator must supply acceptance criteria
```

## Modified domains and merge-ready deltas

When the bound project already owns `specs/<domain>/spec.md`, the kernel renders merge-ready deltas:

- Net-new behaviour → `## ADDED Requirements` with ids continuing from the baseline max.
- Refining a baseline requirement → set `baseline-id: REQ-NNN`; kernel keeps the id under `## MODIFIED Requirements`.

Greenfield domains still render flat `### Requirement:` blocks. `emery slice merge` rejects flat deltas against a non-empty baseline (`merge-delta-headers-required`).

## Body conventions

- **One requirement, one behavioural assertion.** Split compound behaviours into separate blocks.
- **Verbatim source language where possible.** Quote documentation claims lightly normalised; paraphrase behavioural `excerpt` claims into present-tense system prose.
- **`Note:` lines carry commentary, never operative requirements.** For `disagreed`, winning value as body + loser as `Note:`; for tied conflict, only `Note:` lines plus an operator-reconciliation prompt.
- **No invented citations.** Only cite `(source, id, kind)` claims present in the inputs. Never author `Sources:` or `winner` markers.

## Failure modes the parser surfaces

These arise only from a **post-synthesis hand-edit** (`slice-spec-provenance-stale`) — fix by re-running `emery slice refine`:

| Symptom | Cause |
| ------- | ----- |
| `ID:` not matching `^REQ-\d{3}$` | Hand-edit corrupted a kernel-assigned id |
| `Sources:` key not in slice bindings | Hand-edit added a key the kernel never rendered |
| `Status:` outside the closed enum | Hand-edit replaced the kernel-rendered value |
| Tag disagrees with `Status:` | Hand-edit changed one without the other |
| `Sources: []` with non-`unknown` status | Non-`unknown` always has ≥1 source |
| Duplicate `REQ-NNN` ids | Hand-edit duplicated a kernel-assigned id |
