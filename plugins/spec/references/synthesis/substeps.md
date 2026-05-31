# Substep contract

`/spec:refine` invokes synthesis in this fixed order: `proposal → specs → design → tasks`. Each substep reads the prior substeps' artifacts plus the full `Evidence[]` and the target `shape` brief; later substeps never rewrite earlier ones.

## 1. `proposal.md`

Carries the slice's *why*. Author from the lead `summary` (in `discovery.md`) plus the contributing Evidence:

- **Single-source intent** — `## Why` is a one- to three-paragraph restatement of the `intent` claim's `statement`. `## Units` lists the operator's requested deliverable as a single kebab-case slug with a short scope summary; non-goals are inferred from the absence of language ("not …", "without …").
- **Single-source documentation** — `## Why` comes from `decision` and top-level `section` claims; `## Units` lists the distinct deliverable surfaces identified from `requirement` claim subjects (one kebab-case slug per surface); non-goals are any `decision` claim that explicitly rules a path out.
- **Single-source code (port)** — `## Why` is "preserve observed legacy behaviour for `<lead>`"; `## Units` lists the handler/endpoint families surfaced by `excerpt` / `call` claims (one kebab-case slug per family); non-goals call out behaviours the legacy code does *not* exhibit when a `documentation` source contradicts.
- **Combined evidence** — fold all contributing sources into one narrative. When sources disagree on why (rare), state the higher-authority position as the operative one and note the lower-authority position as commentary.

Required H2 sections, in order: `## Why`, `## Units`, `## Non-goals`. Each `## Units` bullet is `- <unit-slug> — <target-specific meaning and short scope summary>` and maps one-to-one to `specs/<unit>/spec.md`. No provenance lines on `proposal.md` — provenance lives in spec files.

## 2. `specs/<unit>/spec.md`

Behavioural requirements. **This is the only synthesised artifact the provenance parser validates.** Write one spec file per `proposal.md` `## Units` entry at `specs/<unit>/spec.md`. The unit slug is kebab-case and maps directly from the `## Units` bullet. The target shape brief explains how to choose units for that target (Vectis feature, Omnia crate/service surface, contracts contract surface), but the file layout is workflow-owned and identical for every target. Root-level `spec.md` is not a valid refine artifact.

Every requirement block follows [`requirement-block.md`](requirement-block.md) verbatim: `ID:`, `Sources:`, `Status:`, with a tag in the headline when `Status` is anything other than `agreed`.

Authoring loop:

1. Group all claims across all Evidence by `id` (deterministic on `requirement` / `criterion` per the Evidence schema; see [`claim-reconciliation.md`](claim-reconciliation.md) for how `decision` / `section` / `excerpt` / `type` / `call` / spatial / `intent` claims contribute).
2. For each reconciled group, apply [`authority.md`](authority.md)'s decision table to pick `Status:`.
3. Emit one H3 requirement block per group, numbering `REQ-001`, `REQ-002`, … in source order (top of the highest-authority Evidence document down). Within one Evidence, keep claim order.
4. For each block that carries a `[unknown]` / `[conflict]` / `[divergence]` tag, `specrun slice validate` emits the matching `slice.synthesis.{unknown|conflict|divergence}` journal event with the requirement id.

Each spec file opens with a short `## Overview` paragraph (one to three sentences) summarising the unit's behavioural surface; the overview carries no provenance lines.

Each requirement block may include one or more `#### Scenario:` H4 headings after the requirement body and before the next `### Requirement:` heading. Scenarios use WHEN/THEN format (GIVEN is optional context). The `#### Scenario:` heading level is fixed — see [`spec-format.md`](../spec-format.md) for the canonical heading conventions. Scenarios do not carry their own provenance lines.

## 3. `design.md`

Technical implementation guidance. Folds in the target `shape` brief (`adapters/targets/<target>/briefs/shape.md`) and any source claim that informs implementation but not behaviour. Required H2s, in order:

1. `## Domain model` — types, IDs, newtypes. Drawn from `type` claims on code Evidence and from `requirement` claim subjects on documentation Evidence; shaped by the target's `shape` (Omnia provider DI, Vectis Crux idioms, contracts format choice).
2. `## APIs and integrations` — external surfaces (HTTP routes, message topics, WebSocket exports, contract endpoints). Drawn from `excerpt` and `call` claims and from `requirement` claims that name an external surface.
3. `## Configuration` — every config key the slice reads. For Omnia targets the shape brief enumerates the closed `Config::get` surface; for Vectis it lists tokens / asset bindings; for contracts it lists baseline-directory inputs.
4. `## Technical logic` — handler delegation, validation placement (edge vs core), error mapping. Folds in `excerpt` claims that show behaviour the requirements abstract over.
5. `## UI / layout` — required only when spatial Evidence (`region` / `container` / `leaf` claims from the `screenshots` source adapter) contributes. Carries the region / container / leaf tree per claim; the Vectis target's `build` brief reads this section to regenerate `composition.yaml`. Targets that do not consume spatial Evidence omit this H2.
6. `## Observability` — metrics, traces, log shapes the target shape brief prescribes.

`decision` and `section` claims fold into the H2 they inform (a decision about transport routing lands in `## APIs and integrations`; a decision about error strategy lands in `## Technical logic`). Quote `decision` text verbatim where useful and cite the source key in parentheses: `(from product-notes)`.

`design.md` carries no provenance lines — every behavioural assertion that needs provenance lives in spec files.

## 4. `tasks.md`

Implementation sequencing as plain markdown checkboxes. Order follows the target's `shape` brief's `tasks.md` skeleton (the Omnia shape brief prescribes `crate → tests → guest wiring → review`; Vectis prescribes `core → tests → shells`; contracts prescribes `author → import → verify`).

Format:

```markdown
- [ ] <Imperative task description>
- [ ] <Next task>
```

One bullet per task. Nesting (`  - [ ]`) is allowed for sub-tasks but discouraged — prefer flat lists per H2 section. `tasks.md` carries no provenance and no narrative prose outside the checkbox list.

## What synthesis never does

- **Never edit `.metadata.yaml`, `plan.yaml`, or `discovery.md`.** The skill body's CLI calls own those.
- **Never rewrite an earlier substep.** `proposal.md` is final before spec files open; spec files are final before `design.md` opens.
- **Never invent provenance.** A `Sources:` key that did not contribute a claim is a parser failure.
- **Never park the slice on uncertainty.** Surface `[unknown]` / `[conflict]` / `[divergence]` and proceed.
- **Never call a `specrun slice synthesize` verb.** It does not exist; substeps are hand-coded in the skill body.
