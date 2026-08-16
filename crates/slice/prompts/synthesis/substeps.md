# Substep contract

Four artifacts in fixed order: `proposal → specs → design → tasks`. Each section reads prior sections plus each bound source's Evidence document (`lead` + `claims`, read from the working tree at the inputs-envelope `evidence-path`) and the resolved target `guidance`; later sections never rewrite earlier ones. The persist tail projects kernel-owned fields; the agent never writes `ID:` / `Sources:` / `Status:`, `REQ` ids, `status`, `winner` markers, or rendered `Sources:` lists.

## 1. `proposal.md`

Carries the slice's *why*, from lead `synopsis` plus Evidence:

- **Intent** — `## Why` restates the `intent` claim; `## Domains` is one kebab-case slug; non-goals from absence language ("not …").
- **Documentation** — `## Why` from `decision` / top-level `section` claims; `## Domains` from distinct `requirement` subjects; non-goals from decisions that rule a path out.
- **Code (port)** — `## Why` is "preserve observed legacy behaviour for `<lead>`"; domains from `excerpt` / `call` families; non-goals call out behaviours legacy does *not* exhibit when docs contradict.
- **Combined** — one narrative; on why-disagreement, higher authority operative, lower as commentary.

Required H2s in order: `## Why`, `## Domains`, `## Non-goals`. Domain bullets: `- <domain-slug> — <scope summary>` → one-to-one with `specs/<domain>/spec.md`. No provenance on `proposal.md`.

## 2. `specs/<domain>/spec.md`

Behavioural requirements — the artifact the provenance parser validates after kernel render. One staged `specs/<domain>/spec.md` per `## Domains` entry. Slug is kebab-case from the Domains bullet. Target guidance chooses domains; layout is workflow-owned. Root-level `spec.md` is invalid.

Author heading + body only; kernel injects provenance from `model.yaml` (see [`requirement-block.md`](requirement-block.md)).

Per requirement, in declaration order:

1. Group claims by `id` (`requirement` / `criterion`; see [`claim-reconciliation.md`](claim-reconciliation.md)).
2. Record `(source, id, kind)` claims + `agreement` (`agreed` / `disagreed`); kernel resolves authority / `status` ([`authority.md`](authority.md)).
3. Write `title`, `statement`, `scenarios[]`, `notes`. Order by highest-authority Evidence source order — kernel assigns `REQ-001…` in that order.
4. Kernel derives tags; `emery slice validate` emits `slice.synthesis.{unknown|conflict|divergence}` with the requirement id.

Open with a short `## Overview`. Every requirement needs ≥1 WHEN/THEN scenario as a `#### Scenario:` H4 (GIVEN optional) — including evidence-gap / `[unknown]`; see [`spec-format.md`](spec-format.md).

## 3. `design.md`

Technical guidance from target `guidance` plus non-behavioural claims. Include **only** H2s that Evidence or guidance informs — omit empty Domain model / APIs / Configuration / UI / Observability. When present, keep this relative order:

1. `## Domain model` — types/IDs (`type` claims; target-shaped).
2. `## APIs and integrations` — external surfaces (`excerpt` / `call` / surface-naming requirements).
3. `## Configuration` — keys the slice reads (per target guidance).
4. `## Technical logic` — delegation, validation, errors; fold abstracting `excerpt` claims.
5. `## UI / layout` — only with spatial Evidence (`region` / `container` / `leaf`); Vectis `build` reads for `composition.yaml`.
6. `## Observability` — metrics/traces/logs the guidance prescribes.

Fold `decision` / `section` into the H2 they inform; quote decisions as `(from <source>)`. No provenance lines.

## 4. `tasks.md`

Numbered checkboxes under `## N. <Group>` headings; group order from target guidance's skeleton:

```markdown
## 1. <Group>

- [ ] 1.1 <Imperative task description>
- [ ] 1.2 <Next task>
```

Every task MUST match `- [ ] X.Y <description>` under a `## ` heading (`tasks.use-checkbox-format` / `tasks.grouped-under-headings`). Optional `<!-- skill: plugin:skill-name -->` only when guidance asks. No provenance or narrative outside the list.

**Agent-completable.** Every task must be executable/verifiable by an agent (code, tooling, mocks, fixtures, validators, build, reviewer skills). Never depend on manual app testing, real credentials, visual inspection, device-only checks, app-store review, or asking the user to verify — encode the equivalent automated check instead.

## What synthesis never does

- **Never edit** `evidence/`, `dependencies/`, `metadata.yaml`, `plan.yaml`, or `leads.md`.
- **Never rewrite** an earlier artifact when authoring a later one.
- **Never author kernel-owned fields** — record claims + `agreement`; orphan `(source, id)` → `slice-model-source-orphan`.
- **Never park on uncertainty** — record the verdict; kernel derives `[unknown]` / `[conflict]` / `[divergence]`.
