# Substep contract

`/spec:refine` invokes synthesis in this fixed order: `proposal → specs → design → tasks`. Each substep reads the prior substeps' artifacts plus the full `Evidence[]` and the target `shape` brief; later substeps never rewrite earlier ones.

## 1. `proposal.md`

Carries the slice's *why*. Author from the candidate `summary` (in `discovery.md`) plus the contributing Evidence:

- **Single-source intent** — `proposal.md` is a one- to three-paragraph restatement of the `intent` claim's `statement`. Scope is "what the operator asked for"; non-goals are inferred from the absence of language ("not …", "without …").
- **Single-source documentation** — motivation comes from `decision` and top-level `section` claims; scope is the union of `requirement` claim subjects; non-goals are any `decision` claim that explicitly rules a path out.
- **Single-source code (port)** — motivation is "preserve observed legacy behaviour for `<candidate-id>`"; scope is the set of handlers / endpoints surfaced by `excerpt` / `call` claims; non-goals call out behaviours the legacy code does *not* exhibit when a `documentation` source contradicts.
- **Combined evidence** — fold all contributing sources into one narrative. When sources disagree on motivation (rare), state the higher-authority motivation as the operative one and note the lower-authority position as commentary.

Required H2 sections, in order: `## Motivation`, `## Scope`, `## Non-goals`. No provenance lines on `proposal.md` — provenance lives in `spec.md`.

## 2. `spec.md`

Behavioural requirements. **This is the only synthesised artifact the provenance parser validates.** Every requirement block follows [`requirement-block.md`](requirement-block.md) verbatim: `ID:`, `Sources:`, `Status:`, with a tag in the headline when `Status` is anything other than `agreed`.

Authoring loop:

1. Group all claims across all Evidence by `claim-id` (deterministic on `requirement` / `criterion` per the Evidence schema; see [`claim-fusion.md`](claim-fusion.md) for how `decision` / `section` / `excerpt` / `type` / `call` / spatial / `intent` claims contribute).
2. For each fused group, apply [`authority.md`](authority.md)'s decision table to pick `Status:`.
3. Emit one H3 requirement block per group, numbering `REQ-001`, `REQ-002`, … in source order (top of the highest-authority Evidence document down). Within one Evidence, keep claim order.
4. For each block that carries a `[unknown]` / `[conflict]` / `[divergence]` tag, the skill body emits the matching `slice.synthesis.{unknown|conflict|divergence}` journal event with the requirement id.

`spec.md` also opens with a short `## Overview` paragraph (one to three sentences) summarising the slice's behavioural surface; the overview carries no provenance lines.

Acceptance scenarios, when needed, live under a `## Scenarios` H2 *after* all requirement blocks. Scenarios cite requirements by id (`Given REQ-001 …`) and do not carry their own provenance.

## 3. `design.md`

Technical implementation guidance. Folds in the target `shape` brief (`targets/<target>/briefs/shape.md`) and any source claim that informs implementation but not behaviour. Required H2s, in order:

1. `## Domain model` — types, IDs, newtypes. Drawn from `type` claims on code Evidence and from `requirement` claim subjects on documentation Evidence; shaped by the target's `shape` (Omnia provider DI, Vectis Crux idioms, contracts format choice).
2. `## APIs and integrations` — external surfaces (HTTP routes, message topics, WebSocket exports, contract endpoints). Drawn from `excerpt` and `call` claims and from `requirement` claims that name an external surface.
3. `## Configuration` — every config key the slice reads. For Omnia targets the shape brief enumerates the closed `Config::get` surface; for Vectis it lists tokens / asset bindings; for contracts it lists baseline-directory inputs.
4. `## Technical logic` — handler delegation, validation placement (edge vs core), error mapping. Folds in `excerpt` claims that show behaviour the requirements abstract over.
5. `## UI / layout` — required only when spatial Evidence (`region` / `container` / `leaf` claims from the `screenshots` source adapter) contributes. Carries the region / container / leaf tree per claim; the Vectis target's `build` brief reads this section to regenerate `composition.yaml`. Targets that do not consume spatial Evidence omit this H2.
6. `## Observability` — metrics, traces, log shapes the target shape brief prescribes.

`decision` and `section` claims fold into the H2 they inform (a decision about transport routing lands in `## APIs and integrations`; a decision about error strategy lands in `## Technical logic`). Quote `decision` text verbatim where useful and cite the source key in parentheses: `(from product-notes)`.

`design.md` carries no provenance lines — every behavioural assertion that needs provenance lives in `spec.md`.

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
- **Never rewrite an earlier substep.** `proposal.md` is final before `spec.md` opens; `spec.md` is final before `design.md` opens.
- **Never invent provenance.** A `Sources:` key that did not contribute a claim is a parser failure.
- **Never park the slice on uncertainty.** Surface `[unknown]` / `[conflict]` / `[divergence]` and proceed.
- **Never call a `specify slice synthesize` verb.** It does not exist; substeps are hand-coded in the skill body.
