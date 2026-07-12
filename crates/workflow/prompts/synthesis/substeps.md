# Substep contract

The synthesis response carries prose for four artifacts, authored in this fixed order: `proposal → specs → design → tasks`. Each section reads the prior sections plus the inputs-envelope `Evidence[]` (each source's inline `lead` + `claims`) and the resolved target `guidance` prompt; later sections never rewrite earlier ones. The `specify slice refine` persist tail projects the kernel-owned fields and persists every artifact; the agent never writes `ID:` / `Sources:` / `Status:` lines, `REQ` ids, `status`, `winner` markers, or rendered `Sources:` lists.

## 1. `proposal.md`

Carries the slice's *why*. Author from the lead `synopsis` (in `discovery.md`) plus the contributing Evidence:

- **Single-source intent** — `## Why` is a one- to three-paragraph restatement of the `intent` claim's `statement`. `## Domains` lists the operator's requested deliverable as a single kebab-case slug with a short scope summary; non-goals are inferred from the absence of language ("not …", "without …").
- **Single-source documentation** — `## Why` comes from `decision` and top-level `section` claims; `## Domains` lists the distinct deliverable surfaces identified from `requirement` claim subjects (one kebab-case slug per surface); non-goals are any `decision` claim that explicitly rules a path out.
- **Single-source code (port)** — `## Why` is "preserve observed legacy behaviour for `<lead>`"; `## Domains` lists the handler/endpoint families surfaced by `excerpt` / `call` claims (one kebab-case slug per family); non-goals call out behaviours the legacy code does *not* exhibit when a `documentation` source contradicts.
- **Combined evidence** — fold all contributing sources into one narrative. When sources disagree on why (rare), state the higher-authority position as the operative one and note the lower-authority position as commentary.

Required H2 sections, in order: `## Why`, `## Domains`, `## Non-goals`. Each `## Domains` bullet is `- <domain-slug> — <target-specific meaning and short scope summary>` and maps one-to-one to `specs/<domain>/spec.md`. No provenance lines on `proposal.md` — provenance lives in spec files.

## 2. `specs/<domain>/spec.md`

Behavioural requirements. **This is the artifact the provenance parser validates after the kernel renders it.** The response carries one spec body per `proposal.md` `## Domains` entry, keyed by `domain`; the synthesis kernel writes one file per domain at `specs/<domain>/spec.md`. The domain slug is kebab-case and maps directly from the `## Domains` bullet. The target guidance prompt explains how to choose domains for that target (Vectis feature, Omnia crate/service surface, contracts contract surface), but the file layout is workflow-owned and identical for every target. Root-level `spec.md` is not a valid refine artifact.

Author each requirement as prose only — heading and body. The kernel injects the `ID:` / `Sources:` / `Status:` lines and the headline tag from `model.yaml`; see [`requirement-block.md`](requirement-block.md) for the prose you write and the block the kernel renders.

Authoring loop (per requirement, in declaration order):

1. Group all claims across all Evidence by `id` (deterministic on `requirement` / `criterion` per the Evidence schema; see [`claim-reconciliation.md`](claim-reconciliation.md) for how `decision` / `section` / `excerpt` / `type` / `call` / spatial / `intent` claims contribute).
2. Record the contributing `(source, id, kind)` claims and an `agreement` verdict (`agreed` / `disagreed`) on the requirement. You classify agreement from Evidence semantics; the kernel resolves authority and derives `status` (see [`authority.md`](authority.md)).
3. Write the requirement prose (`title`, `statement`, `scenarios[]`, `notes`). Order requirements in the response by source order (top of the highest-authority Evidence document down; within one Evidence, keep claim order) — the kernel assigns `REQ-001`, `REQ-002`, … in that declaration order.
4. For each requirement the kernel derives a `[unknown]` / `[conflict]` / `[divergence]` tag, `specify slice validate` emits the matching `slice.synthesis.{unknown|conflict|divergence}` journal event with the requirement id.

Each spec file opens with a short `## Overview` paragraph (one to three sentences) summarising the domain's behavioural surface; the overview carries no provenance lines.

Each requirement may include one or more scenarios (rendered as `#### Scenario:` H4 headings after the body and before the next requirement). Scenarios use WHEN/THEN format (GIVEN is optional context). The `#### Scenario:` heading level is fixed — see [`spec-format.md`](spec-format.md) for the canonical heading conventions. Scenarios do not carry their own provenance lines.

## 3. `design.md`

Technical implementation guidance. Folds in the target `guidance` prompt (`adapters/targets/<target>/prose/prompts/guidance.md`) and any source claim that informs implementation but not behaviour. Required H2s, in order:

1. `## Domain model` — types, IDs, newtypes. Drawn from `type` claims on code Evidence and from `requirement` claim subjects on documentation Evidence; shaped by the target's `guidance` (Omnia provider DI, Vectis Crux idioms, contracts format choice).
2. `## APIs and integrations` — external surfaces (HTTP routes, message topics, WebSocket exports, contract endpoints). Drawn from `excerpt` and `call` claims and from `requirement` claims that name an external surface.
3. `## Configuration` — every config key the slice reads. For Omnia targets the guidance prompt enumerates the closed `Config::get` surface; for Vectis it lists tokens / asset bindings; for contracts it lists baseline-directory inputs.
4. `## Technical logic` — operation delegation, validation placement (edge vs core), error mapping. Folds in `excerpt` claims that show behaviour the requirements abstract over.
5. `## UI / layout` — required only when spatial Evidence (`region` / `container` / `leaf` claims from the `screenshots` source adapter) contributes. Carries the region / container / leaf tree per claim; the Vectis target's `build` prompt reads this section to regenerate `composition.yaml`. Targets that do not consume spatial Evidence omit this H2.
6. `## Observability` — metrics, traces, log shapes the target guidance prompt prescribes.

`decision` and `section` claims fold into the H2 they inform (a decision about transport routing lands in `## APIs and integrations`; a decision about error strategy lands in `## Technical logic`). Quote `decision` text verbatim where useful and cite the source key in parentheses: `(from product-notes)`.

`design.md` carries no provenance lines — every behavioural assertion that needs provenance lives in spec files.

## 4. `tasks.md`

Implementation sequencing as numbered markdown checkboxes grouped under `## N. <Group>` headings. Group order follows the target guidance prompt's `tasks.md` skeleton (the Omnia guidance prompt prescribes `crate → tests → guest wiring → review`; Vectis prescribes `core → tests → shells`; contracts prescribes `author → import → verify`).

Format — one `## N. <Group>` heading per stage, then one `- [ ] N.M <description>` checkbox per task:

```markdown
## 1. <Group>

- [ ] 1.1 <Imperative task description>
- [ ] 1.2 <Next task>
```

`specify slice validate` gates the shape: every task line MUST match `- [ ] X.Y <description>` (the dotted number is required) and sit under a `## ` heading — a bare `- <item>` or an un-numbered `- [ ] <item>` fails `tasks.use-checkbox-format` / `tasks.grouped-under-headings`. An optional `<!-- skill: plugin:skill-name -->` directive may trail a task when a build step should be executed by a named specialist skill; emit one only when the target guidance prompt asks for it. `tasks.md` carries no provenance and no narrative prose outside the checkbox list.

## What synthesis never does

- **Never edit `metadata.yaml`, `plan.yaml`, or `discovery.md`.** The CLI orchestrations own those.
- **Never rewrite an earlier response section.** Author `proposal` before specs; specs before `design`.
- **Never author kernel-owned fields.** `REQ` ids, `status`, `winner` markers, and rendered `Sources:` lists are the kernel's; the agent records `(source, id, kind)` claims and an `agreement` verdict and lets the kernel project the rest. A claim citing a `(source, id)` absent from Evidence fails projection with `slice-model-source-orphan`.
- **Never park the slice on uncertainty.** Record the `agreement` verdict and proceed; the kernel derives `[unknown]` / `[conflict]` / `[divergence]`.
