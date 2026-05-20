# RFC-25: Workflow

> Status: Draft. Supersedes [RFC-20 (archived)](archive/rfc-20-survey.md) and [RFC-23 (archived)](archive/rfc-23-change-lifecycle.md). Ships as Specify 2.0. Compatible with [RFC-22](rfc-22-ledger.md) and [RFC-24](rfc-24-omnia.md) (target rename + `shape` ownership).
>
> Companion: [commands.md](commands.md) is the CLI floor. This document now contains the normative workflow, worked examples, and acceptance scenarios.

## Abstract

RFC-25 makes Specify 2.0 a single coherent workflow:

1. **Source adapters produce evidence.** They enumerate slice candidates at plan time and extract `Evidence` at slice time.
2. **Target adapters produce code.** They provide `shape` guidance plus `build` and `merge`; they do not synthesize `spec.md` or `design.md`.
3. **Core owns synthesis.** Core fuses an `EvidenceSet` into `proposal.md`, `spec.md`, `design.md`, and `tasks.md`, with provenance and conflict tags.
4. *Operators use one `/spec:` surface.* `/change:`* retires; `/spec:plan`, `/spec:execute`, and `/spec:finalize` become the default rhythm.
5. **Every change has a plan and Gate 1.** N=1 is degenerate, not special. Gate 1 is `plan.lifecycle == reviewed`.
6. **2.0 is a hard cut.** No compatibility aliases for old manifests, verbs, brief paths, or `/change:`*.

```text
source adapters --enumerate--> discovery.md / plan.yaml
        |
        `--extract--> evidence/*.yaml --> core synthesis --> proposal, spec, design, tasks
                                                     |
                                                     v
                                      target adapters (shape, build, merge) --> code
```

**Operator rhythm:** `/spec:plan` -> Gate 1 (`plan.lifecycle == reviewed`) -> `/spec:execute` -> `/spec:finalize`. Breakouts: `/spec:refine`, `/spec:build`, `/spec:merge`.

## How to read this RFC


| Audience                | Start here                                                               |
| ----------------------- | ------------------------------------------------------------------------ |
| Operator / skill author | §Operator workflow -> §Execution model -> [commands.md](commands.md)     |
| Source adapter author   | §Source adapter contract -> §Synthesis contract -> §Worked examples      |
| Target adapter author   | §Target adapter contract -> §Synthesis contract (`shape` only)           |
| CLI implementer         | §Normative decisions -> §Implementation contract -> §Implementation plan |
| Migrating from 1.x      | §Migration                                                               |


## Motivation

This RFC unifies two significant changes to Specify in a single release. The changes, source and target adapters, and plan-led workflow to reduce the Specify core to the orchestration of small point changes through to large-scale migrations.

**Adapter axis.** `/change:analyze` and `/change:survey` are one operation with two evidence sources; `/spec:define` and `/spec:extract` repeat the pattern at slice time. Legacy migration archaeology belongs in add-on source adapters, not core. Unqualified `adapter` only names outputs today, leaving no symmetrical term for inputs.

**Workflow axis.** The `/change:`* vs `/spec:`* split is a workflow seam, not an adapter seam. It forces a two-namespace operator surface, an orphan `/spec:define` path for trivial work, and a review pause enforced by skill exit instead of observable `plan.yaml` state.

**One redesign.** Qualify adapters by direction; collapse operator vocabulary to `/spec:`*; CLI-stamp Gate 1 as `plan.lifecycle == reviewed`; keep `change.md` and `plan.yaml` at every slice count.

## Normative decisions


| ID                                      | Decision                                                                                                                    | Implementation consequence                                                                                        |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **D1 Source/target split**              | Replace unqualified adapters with source adapters (`enumerate`, `extract`) and target adapters (`shape`, `build`, `merge`). | `sources/<name>/source.yaml`, `targets/<name>/target.yaml`, axis-aware resolver and cache.                        |
| **D2 Core synthesis**                   | Source adapters emit evidence only; target adapters supply `shape` only; core owns canonical artifacts.                     | `/spec:refine` runs extraction, synthesis, validation, and lifecycle transition.                                  |
| **D3 Multi-source slices**              | `planSlice.sources` is a list with cardinality >= 1.                                                                        | Combined evidence is first-class; pure intent/port/design are degenerate one-source cases.                        |
| **D4 Provenance and disagreement tags** | Requirements carry `ID:`, `Sources:`, and `Status:`.                                                                        | Core emits `agreed`, `unknown`, `conflict`, or `divergence`; tags never park the slice.                           |
| **D5 Always plan**                      | Every change runs through `enumerate` and `plan.yaml`, including N=1.                                                       | `/spec:define` retires; trivial work uses degenerate `intent.enumerate`.                                          |
| **D6 Gate 1 only**                      | Human review happens between planning and execution via `plan.lifecycle == reviewed`.                                       | No Gate 2 and no synthesis review state in v1.                                                                    |
| **D7 Supervised execute**               | `/spec:execute` is the only v1 driver and resumes from on-disk state.                                                       | No `--yes-plan`, `--one`, `--until`, `--dry-run`, or `--continue`.                                                |
| **D8 CLI owns workflow writes**         | CLI is the single writer for lifecycle and deterministic files.                                                             | Never hand-write `plan.yaml`, `.metadata.yaml`, archive paths, `discovery.md`, `sources.yaml`, or `targets.yaml`. |
| **D9 Hub routing is uniform**           | Loop and breakout verbs share the same hub root -> project slot routing.                                                    | Breakouts resolve the active slice project before phase work.                                                     |
| **D10 Hard cut at 2.0**                 | 1.x manifests, verbs, brief paths, and `/change:`* retire together.                                                         | Migration script performs mechanical renames; no compatibility aliases.                                           |


## Operator workflow

### What changes from 1.x


| Before (1.x)                                         | After (2.0)                                                                              |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `/change:draft`, `/change:survey`, `/change:analyze` | `/spec:plan` (`source.enumerate`)                                                        |
| `/spec:define`, `/spec:extract`                      | `/spec:refine` (`source.extract` + core synthesis)                                       |
| `/change:execute loop`                               | `/spec:execute`                                                                          |
| `/change:finalize`                                   | `/spec:finalize`                                                                         |
| `adapters/<name>/adapter.yaml`, `planSlice.adapter`  | `targets/<name>/target.yaml`, `planSlice.target`                                         |
| `specify adapter` *, `specify change`*               | `specify source *`, `specify target *`, `specify plan *`; see [commands.md](commands.md) |


### Commands

Default rhythm: `/spec:plan` -> review -> `/spec:execute` -> review on stops -> `/spec:finalize`.


| Stage      | Command                                        | Replaces                                             |
| ---------- | ---------------------------------------------- | ---------------------------------------------------- |
| Plan       | `/spec:plan <scope> [source <key>=<path> ...]` | `/change:draft`, `/change:survey`, `/change:analyze` |
| Drive plan | `/spec:execute`                                | `/change:execute loop`                               |
| Deliver    | `/spec:finalize <name>`                        | `/change:finalize`                                   |



| Breakout | Command        | When                                                                                   |
| -------- | -------------- | -------------------------------------------------------------------------------------- |
| Refine   | `/spec:refine` | Inspect or hand-edit after synthesis tags; manual slice work. Replaces `/spec:define`. |
| Build    | `/spec:build`  | Build failure park; explicit implementation.                                           |
| Merge    | `/spec:merge`  | Rare manual land before resuming execute.                                              |


N=1 uses the same rhythm as N=12. Re-entry needs no `--continue`; skills re-read `plan.yaml` and slice `.metadata.yaml`.

### Execution model

```text
PLAN (plan.yaml)          SLICE (.metadata.yaml)           STAGE
------------------------------------------------------------------
pending                   -                              /spec:plan
  | (operator)            -                              Gate 1: reviewed
reviewed                  -                              /spec:execute allowed
in-progress               refining                       extract + synthesize
  |                       refined                        spec.md (+ inline tags)
  |                       built                          /spec:build
  |                       merged                         /spec:merge -> entry done
drained                   -                              /spec:finalize
```

The plan transitions to `drained` once every entry has transitioned to `done`; `/spec:execute` exits at that point and `/spec:finalize` becomes legal.

`**/spec:plan**` runs pre-flight, scaffolds `change.md` and `plan.yaml`, runs hub registry validation and workspace sync when needed, enumerates each source, proposes candidate slices, assigns hub projects when needed, validates the plan, and stamps **Gate 1** with `specify plan transition <scope> reviewed`.

`**/spec:execute`** refuses unless the plan is `reviewed`, acquires the plan lock (hub root in hub mode), and loops: `specify plan next` -> hub project resolution and slot prep when needed -> `/spec:refine` if needed -> `/spec:build` -> `/spec:merge` -> residue commit and return to hub root when needed -> repeat until drained.

`**/spec:finalize`** requires all entries `done`, pushes branches, observes PRs until `MERGED`, then runs `specify plan finalize` to archive the plan.

**Breakouts** share skill bodies with the loop. `/spec:refine` requires an active `in-progress` entry from `plan next` and never writes `in-progress` itself. `/spec:build` refuses only on slice lifecycle, not on synthesis tags. `/spec:merge` is the only writer of per-entry `done`.


| Trigger                 | `/spec:execute` behavior     | Operator next                        |
| ----------------------- | ---------------------------- | ------------------------------------ |
| Build non-zero exit     | Stop with task id + log      | Fix; re-run execute or `/spec:build` |
| Merge baseline conflict | Stop with paths              | Resolve; re-run execute              |
| Plan drained            | Exit; `/spec:finalize` ready | Run finalize                         |


After Gate 1 without execute: `specify plan next`, then `/spec:refine`.

### The plan gate


| Gate       | Position                                    | Mechanism                                  |
| ---------- | ------------------------------------------- | ------------------------------------------ |
| **Gate 1** | After plan validate, before `/spec:execute` | `specify plan transition <scope> reviewed` |


Gate 1 is the successor to RFC-23's explicit human seam. No Gate 2 ships in v1; see §Non-goals.

### Planning at every scale

`/spec:plan` always enumerates. N=1 is degenerate: `intent.enumerate` produces one candidate. Headless trivial path: `specify plan create` + `plan add` + `plan transition reviewed` + `/spec:execute`.

```yaml
version: 1
name: add-search-filter
sources:
  intent:
    adapter: intent
    value: "Add a search filter to the user list."
slices:
  - name: add-search-filter
    target: omnia
    sources: [intent]
    candidate: add-search-filter
    status: pending
```

`change.md` is scaffolded at plan time and editable at Gate 1.

### Single-repo vs multi-repo


| `hub:`  | `/spec:plan` behavior                                                                   |
| ------- | --------------------------------------------------------------------------------------- |
| `false` | Single root; skip sync-workspace and assignment.                                        |
| `true`  | `registry.yaml`; sync-workspace before enumerate; per-candidate `--project` at propose. |


**One driving mode per project in v1.** Hub-registered projects are hub-driven only; `/spec:plan` from a project root while a hub plan is active is refused at plan-create.

### Hub routing

Plan artifacts live at the hub root; slice artifacts live in `.specify/workspace/<project>/`. Breakouts and `/spec:execute` share routing: plan lock at hub root -> resolve active slice project -> sync slot -> `chdir` -> phase work -> return.

## Concepts

### Adapter vocabulary


| Term                              | Meaning                                                                                                                    |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| **source adapter**                | Input role: `enumerate` + `extract`. Examples: `intent`, `documentation`, `legacy-code-typescript`, `openapi`.             |
| **target adapter**                | Output role: `shape` + `build` + `merge`. Examples: `omnia`, `vectis`, `contracts`. Replaces unqualified `adapter`.        |
| **plugin**                        | Shared implementation shape for either role.                                                                               |
| **candidate** / **candidate set** | Slice-sized unit from `enumerate`; blocks under `## Candidate inventory` in `discovery.md`.                                |
| **Evidence**                      | Per-binding result of `extract`; a structured document with `claims:`; persisted before synthesis.                         |
| **Evidence set**                  | All `Evidence` bound to one slice; synthesis input (`EvidenceSet`).                                                        |
| **provenance**                    | Source bindings behind one requirement (`Sources:` list).                                                                  |
| **conflict** / **divergence**     | Unresolvable vs authority-resolved disagreement; `[conflict]` / `[divergence]` tags.                                       |
| **authority**                     | Closed enum: `intent`, `external-contract`, `design-doc`, `behaviour` (highest first; see §Authority hierarchy). |


`provider` is reserved for Omnia DI. `profile` is retired. Unqualified `adapter` is removed. The slice-vs-change on-disk distinction in [project.mdc](../.cursor/rules/project.mdc) survives; only slash commands collapse to `/spec:`*.

### Workflow vocabulary


| Term                     | Meaning                                                                   |
| ------------------------ | ------------------------------------------------------------------------- |
| **change**               | On-disk umbrella: `change.md`, `plan.yaml`, archive; not a slash command. |
| **plan**                 | `/spec:plan`, `specify plan` *, Gate 1.                                   |
| **slice**                | One refine -> build -> merge unit.                                        |
| **refine** / **execute** | `/spec:refine` (per slice); `/spec:execute` (supervised driver).          |
| **gate**                 | CLI-stamped transition; v1: Gate 1 only.                                  |
| **breakout verb**        | `/spec:refine`, `/spec:build`, `/spec:merge`.                             |
| **active slice**         | Plan entry currently `in-progress`.                                       |
| **plan lifecycle**       | `pending -> reviewed -> in-progress -> drained`.                          |
| **per-entry lifecycle**  | `pending -> in-progress -> done`.                                         |
| **slice lifecycle**      | `refining -> refined -> built -> merged`.                                 |


## Implementation contract

### Types

Names used in function signatures and table cells throughout this RFC. Concrete shape is the canonical example or schema noted in the right column.


| Name             | Shape                                                                                                                     | Reference                                                              |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `source-binding` | Plan-level pair: source-key (kebab-case) -> path or value. Lives under `plan.yaml.sources`.                               | §Planning at every scale; §`planSlice.sources`.                        |
| `CandidateSet`   | Ordered list of candidate blocks emitted by `enumerate`; each has stable `id`, `sources[]`, optional `correlates-with[]`. | §Discovery handshake; `schemas/discovery/candidate-block.schema.json`. |
| `Evidence`       | Per-binding result of `extract`; persisted to `.specify/slices/<slice>/evidence/<source-key>.yaml`.                       | §`extract`; `schemas/evidence.schema.json`.                            |
| `EvidenceSet`    | All `Evidence` bound to one slice (one per entry in `planSlice.sources`); the input to synthesis.                         | §Synthesis contract.                                                   |
| `planSlice`      | One slice entry under `plan.yaml.slices[]`; carries `target`, `sources[]`, `project`, `status`.                           | §`planSlice.sources`; §On-disk and tooling.                            |


### Writer ownership

The CLI MUST be the single writer for deterministic workflow state:


| Artifact                          | Writer                                              |
| --------------------------------- | --------------------------------------------------- |
| `plan.yaml` lifecycle and entries | `specify plan` *                                    |
| `.metadata.yaml` lifecycle        | `specify slice` *                                   |
| Archive moves                     | `specify plan finalize`, `specify slice merge/drop` |
| `discovery.md` candidate blocks   | `/spec:plan` through CLI helpers                    |
| `sources.yaml` / `targets.yaml`   | CLI registry/catalogue commands                     |


Skills and adapters MAY author briefs, evidence content, artifacts, and implementation code when their contract allows it. They MUST NOT hand-edit lifecycle files or archive paths.

### Adapter implementation shape

Source: `sources/<name>/source.yaml`. Target: `targets/<name>/target.yaml`.

Shared rules: kebab-case `name` unique per axis; `axis: source | target`; closed `capabilities[]` (`enumerate`/`extract` for sources, `shape`/`build`/`merge` for targets); `briefs.<capability>` required; optional `tools[]` per RFC-15 into `.specify/.cache/{sources,targets}/<name>/`.

`detect[]` auto-detection from paths is deferred; operators bind explicitly (`source legacy=./repo`).

```yaml
# sources/<name>/source.yaml
name: legacy-code-typescript
version: 1
axis: source
capabilities: [enumerate, extract]
briefs:
  enumerate: briefs/enumerate.md
  extract:   briefs/extract.md
```

```yaml
# targets/<name>/target.yaml
name: omnia
version: 1
axis: target
capabilities: [shape, build, merge]
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
```

### Resolver and cache

```text
.specify/.cache/
|-- sources/{intent,documentation,legacy-code-typescript,...}/
`-- targets/{omnia,vectis,contracts,...}/
```

One resolver module (`crates/domain/src/plugin/`) routes by axis.

## Source adapter contract

### `enumerate(source-binding) -> CandidateSet`

Runs at plan time. `/spec:plan` writes candidate blocks to `discovery.md` using the RFC-20 grammar plus stable `id`, `sources[]`, and optional `correlates-with[]`.

### `extract(candidate, source-binding) -> Evidence`

Runs at slice time. `/spec:refine` persists `Evidence` under `.specify/slices/<slice>/evidence/<source-key>.yaml`.

```yaml
source: legacy-monolith
adapter: legacy-code-typescript
authority: behaviour
candidate: user-registration
claims:
  - kind: code-excerpt
    claim-id: users.register.email-validation
    path: src/users/register.ts
    lines: [12, 87]
    sha256: 6c25...
```

Closed `kind` enum: `intent-text`, `requirement-statement`, `acceptance-criterion`, `decision-record`, `document-section`, `diagram-reference`, `contract-reference`, `code-excerpt`, `type-definition`, `external-call`. New kinds require an RFC update. No raw source bodies by default.

Top-level `authority:` is required per `Evidence` unless provided by manifest `default-authority`. Optional `claim-id` on claim-shaped entries enables deterministic fusion; semantic correlation applies when absent. `Evidence` validates against `schemas/evidence.schema.json`; CLI writes paths and adapters return content via briefs/tools only.

### Default source adapters


| Adapter         | `default-authority` | Role                              |
| --------------- | ------------------- | --------------------------------- |
| `intent`        | `intent`            | Operator briefs and overrides.    |
| `documentation` | `design-doc`       | Written product/technical intent. |


Both are true source adapters with no special workflow rules. N=1 greenfield uses degenerate `intent.enumerate`.

### Discovery handshake

Each candidate block has stable `id`, `sources[]`, and optional `correlates-with[]`. Operator merges cross-source duplicates at propose time. Re-enumerating the same source replaces by `id`; enumerating a different source appends new ids. Schema: `schemas/discovery/candidate-block.schema.json`. `discovery.md` remains the plan-time source of truth; no `candidates.yaml` in v1.

Minimal candidate block, as written under `## Candidate inventory` in `discovery.md`:

```markdown
### user-registration

- id: user-registration
- sources: [legacy-monolith]
- correlates-with: [identity-design-notes#user-signup]
- summary: Registration endpoint accepting email + password with RFC-5322 validation.
- evidence-hint: src/users/register.ts (legacy-monolith)
```

`id` is the stable handle re-enumeration writes against. `sources[]` lists the source-bindings that surfaced this candidate. `correlates-with[]` is an operator-merge hint: it names sibling candidate ids from other sources that look like the same unit of work, and is what `specify plan add` consumes when the operator merges duplicates into one slice.

### `planSlice.sources`

`planSlice.sources` is a list of one or more binding keys:

```yaml
slices:
  - name: identity-user-registration
    target: omnia
    project: identity-svc
    sources: [legacy-monolith, identity-design-notes]
    status: pending
```


| Archetype         | `sources`                | Meaning                                     |
| ----------------- | ------------------------ | ------------------------------------------- |
| Pure greenfield   | `[intent]`               | `[]` normalizes to `[intent]`.              |
| Pure port         | `[<legacy>]`             | Code dictates behavior.                     |
| Pure design       | `[<doc>]`                | Docs dictate behavior.                      |
| Combined evidence | `[<doc>, <legacy>, ...]` | Authority hierarchy resolves disagreements. |


`specify plan add` enforces at most one `intent` per slice, at most one binding per key, and at least one binding total.

## Target adapter contract

Target adapters do not own `spec.md` or `design.md` synthesis. They may declare:

- `**shape**`: idiom guidance consumed by core synthesis.
- `**build**` / `**merge**`: implementation and landing briefs/tools.

`specify adapter pipeline {define,build,merge}` retires. Topology verbs can return when a third-party target needs custom ordering; see [commands.md](commands.md). RFC-24 "adapter-gated" becomes "target-gated"; `planSlice.adapter` becomes `planSlice.target`.

## Synthesis contract

Core owns `proposal.md`, `spec.md`, `design.md`, and `tasks.md`. Inputs are `EvidenceSet`, `planSlice`, and optional target `shape` brief. Agent authors from `plugins/spec/references/synthesis/`; CLI validates structure and stamps lifecycle.

`/spec:refine` pipeline:

1. Resolve target and sources.
2. Run serial `extract` per §Extraction reliability.
3. Synthesize in fixed substep order: `proposal` -> `specs` -> `design` -> `tasks`. Substeps are hand-coded in `/spec:refine` in v1 (no `specify slice synthesize` verb; see [commands.md](commands.md)).
4. Run `specify slice validate` ([commands.md](commands.md)).
5. Transition to `refined` via `specify slice transition <name> refined`.

Tags `[unknown]`, `[conflict]`, and `[divergence]` are review signals; they do not park the slice. Synthesis never aborts on tags; the slice lifecycle stays `refining -> refined -> built -> merged` regardless of tag content.

### Requirement block contract

Every requirement block requires:

```markdown
ID: REQ-001
Sources: [source-key]
Status: agreed
```

`Sources:` contains one or more source keys, highest authority first. `Status:` is one of `agreed`, `unknown`, `conflict`, or `divergence`.

### Authority hierarchy

Highest authority wins:

1. `intent`: operator override at slice time.
2. `external-contract`: published APIs, regulation.
3. `design-doc`: internal docs, RFCs.
4. `behaviour`: what legacy code does.


| Agreement                    | Output                                                    |
| ---------------------------- | --------------------------------------------------------- |
| One source                   | `Status: agreed`                                          |
| Multiple agree               | `Status: agreed`, all keys in `Sources:`                  |
| Disagree, one winner         | `Status: divergence`, `[divergence]`, loser as commentary |
| Disagree, tied top authority | `Status: conflict`, `[conflict]`, operator reconciles     |
| No contributing Evidence     | `Status: unknown`, `[unknown]`                            |


Substep order and lifecycle behavior live with the `/spec:refine` pipeline above.

### Extraction reliability


| Rule                         | Behavior                                                         |
| ---------------------------- | ---------------------------------------------------------------- |
| **Order**                    | Serial in `planSlice.sources` declaration order.                 |
| **Required**                 | Default; `optional: true` on binding allows fail-soft.           |
| **Hard failure**             | Required `extract` fails -> stay `refining`, no synthesis.       |
| **Soft failure**             | Optional fails -> warning, synthesis with remaining `Evidence`.  |
| **Empty / invalid Evidence** | Empty `claims: []` valid; invalid fails schema before synthesis. |


## Worked examples

These examples are non-normative walkthroughs of the source-extract and synthesis contracts above.

### Documentation extract to Evidence to spec

Input document:

```markdown
# Password reset

The account service should let a registered user request a password reset link by email.

Acceptance:
- Unknown email addresses receive the same outward response as known users.
- Reset links expire after 30 minutes.

Decision: use the existing transactional email provider rather than introducing a new notification service.
```

`documentation.extract` output:

```yaml
source: product-notes
adapter: documentation
authority: design-doc
candidate: password-reset
claims:
  - kind: requirement-statement
    claim-id: password-reset.request
    path: docs/account.md
    lines: [3, 3]
    sha256: 4f39...
    statement: "The account service should let a registered user request a password reset link by email."
  - kind: acceptance-criterion
    claim-id: password-reset.response-privacy
    path: docs/account.md
    lines: [6, 6]
    sha256: 91aa...
    criterion: "Unknown email addresses receive the same outward response as known users."
  - kind: acceptance-criterion
    claim-id: password-reset.expiry
    path: docs/account.md
    lines: [7, 7]
    sha256: 0d8b...
    criterion: "Reset links expire after 30 minutes."
  - kind: decision-record
    path: docs/account.md
    lines: [9, 9]
    sha256: f6c2...
    decision: "Use the existing transactional email provider rather than introducing a new notification service."
```

Resulting `spec.md` after core synthesis:

```markdown
### Requirement: Password reset request

ID: REQ-001
Sources: [product-notes]
Status: agreed

The system lets a registered user request a password reset link by email.

### Requirement: Password reset response privacy

ID: REQ-002
Sources: [product-notes]
Status: agreed

The system returns the same outward response for known and unknown email addresses.

### Requirement: Password reset expiry

ID: REQ-003
Sources: [product-notes]
Status: agreed

The system expires password reset links after 30 minutes.
```

### Per-requirement provenance variants

Single source:

```markdown
### Requirement: User registration accepts valid email

ID: REQ-001
Sources: [legacy-monolith]
Status: agreed

The system accepts a registration request when the email field is RFC-5322 valid ...
```

Combined evidence where sources agree:

```markdown
### Requirement: User registration accepts valid email

ID: REQ-001
Sources: [identity-design-notes, legacy-monolith]
Status: agreed

The system accepts a registration request when the email field is RFC-5322 valid ...
```

`[divergence]` from authority resolution:

```markdown
### Requirement: Reset link expiry [divergence]

ID: REQ-007
Sources: [identity-design-notes, legacy-monolith]
Status: divergence

The system expires password reset links after 30 minutes. (from identity-design-notes; design-doc)

Note: legacy-monolith observed 24-hour expiry; the design-doc authority overrides. Operator review recommended.
```

## On-disk and tooling

### `project.yaml`

```yaml
specify_version: 2.0.0
sources: [intent, documentation, legacy-code-typescript]
target: omnia
hub: false
```

`sources` lists available adapters; bindings live in `sources.yaml` ([RFC-21](rfc-21-catalogue.md)). v1 supports one `target` per project; `planSlice.target` must match for hub entries. `profile` and singular `adapter` are removed.

### `.specify/` layout

Regular project: `change.md`, `plan.yaml`, and `discovery.md` at root; `slices/<name>/` contains artifacts plus `evidence/<source-key>.yaml`.

Hub: plan and discovery artifacts at hub root; slices under `workspace/<project>/.specify/slices/`. Hub root `slices/` is unused.

### `surfaces.json` and `discovery-summary.md`

`surfaces.json` moves into `sources/legacy-code-typescript/`. `specify change survey` is deleted. `survey.md` becomes generic `discovery-summary.md` with `## Summary`, `## Source inventory`, and `## Candidate inventory`.

### CLI surface

Full v1 floor, cuts, and deferrals: [commands.md](commands.md).

Axis deltas: `specify source resolve`; `specify target resolve` (was `adapter resolve`); `specify plan transition ... reviewed`; `specify plan amend --add-source` / `--remove-source`; `specify change *` and `specify adapter pipeline` retire. Skill/slash retirements match §Operator workflow -> Commands.

### Skill / `SKILL.md` changes


| File                                                         | Action                                                  |
| ------------------------------------------------------------ | ------------------------------------------------------- |
| `plugins/spec/skills/plan/SKILL.md`                          | New, from `change/draft`.                               |
| `plugins/spec/skills/execute/SKILL.md`                       | New, from `change/execute loop`.                        |
| `plugins/spec/skills/refine/SKILL.md`                        | Renamed from `define/`; plan-resolved bindings.         |
| `plugins/spec/skills/{build,merge,finalize}/SKILL.md`        | Build/merge breakouts; finalize from `change/finalize`. |
| `plugins/spec/skills/init/SKILL.md`                          | Mention `/spec:plan`.                                   |
| `plugins/spec/skills/drop/SKILL.md`                          | Unchanged.                                              |
| `plugins/change/**`, `plugins/spec/skills/{define,extract}/` | Retired.                                                |


### Repository layout (monorepo v1)

```text
/
|-- plugins/
|   `-- spec/skills/{init,plan,refine,execute,build,merge,finalize,drop}/
|-- sources/
|   |-- intent/                       # source.yaml, briefs/
|   |-- documentation/
|   `-- legacy-code-typescript/       # surfaces.json moves here (was under change/)
|-- targets/                          # was adapters/
|   |-- omnia/                        # target.yaml, briefs/{shape,build,merge}.md
|   |-- vectis/
|   `-- contracts/
`-- schemas/                          # plugin, source, target, evidence, candidate-block
```

Deferred: other legacy-code languages; contract source adapters; per-adapter repo split.

## Implementation plan

Phase 1 (steps 1-13) lands the adapter model. Phase 2 (steps 14-17) lands workflow collapse in the same 2.0 release.


| Step | Decisions   | Deliverable                                                                                                              | Acceptance           |
| ---- | ----------- | ------------------------------------------------------------------------------------------------------------------------ | -------------------- |
| 1    | D1, D3, D6  | Schemas: `plugin`, `source`, `target`, `evidence`, `candidate-block`; plan `target` + `sources[]`; `reviewed` lifecycle. | #5g                  |
| 2    | D1, D3      | Domain rename `Adapter*` -> `Target*`; `Plan::resolve_sources`.                                                          | #5a                  |
| 3    | D1          | `crates/domain/src/plugin/` loader replaces `adapter/`.                                                                  | #2, #4               |
| 4    | D1, D5      | Ship `sources/intent/`, `sources/documentation/`.                                                                        | #1, #2               |
| 5    | D2, D4      | Core synthesis + `/spec:refine` pipeline; migrate define briefs -> synthesis + `shape`.                                  | #5, #5a-#5h          |
| 6    | D4          | `spec.md` provenance parser (`ID:`, `Sources:`, `Status:`).                                                              | #1, #5a-#5c          |
| 7    | D3          | Discovery `correlates-with`; stable-id replace.                                                                          | #5e                  |
| 8    | D1, D3, D10 | CLI: `source resolve`, plan amend sources; retire `change survey`, `adapter pipeline`.                                   | #3, #4, #7           |
| 9    | D1, D2      | Target brief migration; RFC-24 prose.                                                                                    | #5h                  |
| 10   | D1-D10      | Docs: AGENTS.md, project.mdc, decision-log, adapter-anatomy.                                                             | Documentation review |
| 11   | D1          | `discovery-summary.md` generic form.                                                                                     | #4                   |
| 12   | D1-D4, D9   | Adapter-axis acceptance lands before step 16.                                                                            | #1-#5h, #10          |
| 13   | D4          | RFC-19 journal events for extract and synthesis tags.                                                                    | #5b-#5d              |
| 14   | D6          | `reviewed` lifecycle + `plan transition reviewed` with no behavior change in draft.                                      | #1-#4                |
| 15   | D7, D9      | `/spec:execute` stop/resume; load-bearing workflow collapse step.                                                        | #8-#11               |
| 16   | D5-D7       | Document default `/spec:plan` -> execute -> finalize; scenario #1 release blocker.                                       | #1, #8, #9           |
| 17   | D10         | Delete `/change:*`, `/spec:define`; remove `plugins/change/`.                                                            | Full matrix          |


## Acceptance scenarios

Run these against the merged skills before implementation step 17. Each row stress-tests a place the redesign can fail.

**Scenario id convention.** Numeric ids (`#1`-`#12`) are independent scenarios. Letter-suffixed ids under a number (`#5`, `#5a`-`#5h`) share a theme — here, single-source and multi-source synthesis behavior. Implementation-plan acceptance columns and inline cross-references use these ids verbatim.

If any of #1-#4 fail the ergonomics test (operator confusion, lost time, surprised state), revisit §Planning at every scale before pushing through step 17.

**Release blocker:** scenario #1 (pure intent, one slice) must pass before step 16 lands. Single-release collapse means N=1 `/spec:plan` ergonomics surface to every operator at once.


| #   | Decisions  | Scenario                                                                                                                                                                                                         | What it stress-tests                                                                                                                                                                                                               |
| --- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | D3, D5, D6 | **Pure intent, one slice.** Operator runs `/spec:plan fix-typo "fix typo in user.rs"`.                                                                                                                           | Degenerate `intent.enumerate`; Gate 1 ergonomics on trivial work; `change.md` + `plan.yaml` justifiability at N=1; `Sources: [intent]` provenance.                                                                                 |
| 2   | D1, D3, D4 | **Documentation, one slice.** Operator binds a single docs path.                                                                                                                                                 | `documentation.enumerate` correctness at the new entry point; `Sources: [<doc-key>]` provenance.                                                                                                                                   |
| 3   | D3, D5, D6 | **Documentation, multi-slice.** Operator binds docs that map to N candidates.                                                                                                                                    | Propose/edit/reject loop; Gate 1 amendment flow.                                                                                                                                                                                   |
| 4   | D1, D3     | **Legacy-code, multi-slice.** Operator binds a legacy repo.                                                                                                                                                      | `legacy-code-typescript.enumerate`; survey/repair loop under the new skill; under-slicing failure mode; `Sources: [<legacy-key>]` provenance.                                                                                      |
| 5   | D2, D4     | **Intra-Evidence `[conflict]`.** Single-source slice where synthesis cannot reconcile contradictory `claims` within one `Evidence` document.                                                                     | `[conflict]` written into `spec.md`; lifecycle still transitions to `refined`; operator can hand-edit and run `/spec:build` without a parking-state ceremony.                                                                      |
| 5a  | D2-D4      | **Combined evidence (legacy-code + documentation), one slice.** Operator binds a legacy repo and a design-notes path on the same slice.                                                                          | Synthesis end-to-end: serial `extract` per binding; `EvidenceSet` cardinality 2; `Sources:` line carrying both keys; `claim-id` correlation produces deterministic fusion; lifecycle reaches `refined` cleanly when sources agree. |
| 5b  | D2, D4     | `**[divergence]` from authority resolution.** Combined-evidence slice where docs and legacy code disagree at different authority classes, for example docs say "30 minutes" expiry while code observed 24 hours. | `Status: divergence` written; design-doc winner becomes the operative requirement; behaviour preserved as inline commentary; lifecycle transitions to `refined`; operator may hand-edit before build.                    |
| 5c  | D2, D4     | `**[conflict]` from same-authority disagreement.** Combined-evidence slice where two `documentation` sources disagree on the same claim.                                                                         | `Status: conflict` written with both values preserved as inline commentary; lifecycle still transitions to `refined`; operator must reconcile by editing or amending bindings before the requirement is meaningful.                |
| 5d  | D2-D4      | **Optional binding fail-soft.** Combined-evidence slice with one `optional: true` binding whose `extract` fails.                                                                                                 | Synthesis proceeds with the surviving `Evidence`; structured warning emitted; `Sources:` lines reflect surviving contributors only.                                                                                                |
| 5e  | D3         | `**correlates-with` propose-time merge.** Two adapters surface the same candidate; operator merges them at propose.                                                                                              | `specify plan add` writes one slice with combined `sources:`; downstream extract runs against every contributing source.                                                                                                           |
| 5f  | D2, D3     | **Required-source extract failure.** Required binding's `extract` fails.                                                                                                                                         | Slice stays in `refining`, no synthesis runs, structured error names the source key.                                                                                                                                               |
| 5g  | D2, D8     | **Invalid Evidence schema rejection.** Adapter emits `Evidence` failing `evidence.schema.json`.                                                                                                                  | Validation fails before synthesis; structured error; slice stays in `refining`.                                                                                                                                                    |
| 5h  | D2         | **Target `shape` injection.** Synthesis consumes a non-empty `target.shape` brief.                                                                                                                               | Generated `spec.md` / `design.md` reflect target-idiom guidance; pure-intent fixture vs documentation fixture both pick up the same `shape`.                                                                                       |
| 6   | D9         | **Multi-repo assignment from a hub.** Operator runs `/spec:plan` in a hub.                                                                                                                                       | `hub:` discriminator; per-candidate `--project` at propose; workspace sync timing.                                                                                                                                                 |
| 7   | D3, D6     | **Operator amends one-slice plan into two slices at Gate 1.**                                                                                                                                                    | Plan amendment via `specify plan amend`; re-entry to Gate 1 after amend.                                                                                                                                                           |
| 8   | D7, D9     | **Step-through breakout mid-execute.** Operator starts `/spec:execute`; on the second slice they cancel, run `/spec:build` directly to investigate, then re-invoke `/spec:execute`.                              | Stop/resume contract; step-through verbs leave on-disk state consistent for `/spec:execute` to resume without flags.                                                                                                               |
| 9   | D7         | `**/spec:execute` parks on a build failure, operator fixes, resumes.** Slice's `cargo test` fails; operator patches the crate; runs `/spec:execute`.                                                             | Build-failure stop hint; build resumes from the failed task; loop continues to merge.                                                                                                                                              |
| 10  | D9         | **Hub `/spec:execute` across two projects.** Plan with slices targeting `project-a` and `project-b`; operator runs `/spec:execute` from the hub root.                                                            | Per-slice project routing; slot materialisation; `prepare-branch`; `chdir` + residue commit; plan-lock semantics at the hub root while phase work runs in slots.                                                                   |
| 11  | D7, D9     | **Hub breakout after build failure in a slot.** `/spec:execute` parks on `auth-rotate` in `project-a`; operator stays at hub root and runs `/spec:build`.                                                        | Project-routing rule for breakout verbs; active-slice resolution across the hub/slot boundary; correct `chdir` without operator intervention.                                                                                      |
| 12  | D9         | **Dual-driving refused.** Project registered in a hub; operator runs `/spec:plan` from the project root with a hub-driven plan active.                                                                           | One-driving-mode-per-project invariant.                                                                                                                                                                                            |


Adapter-axis scenarios #1-#5h and #10 land by step 12. Workflow-collapse scenarios, especially #1 and #8-#9, gate steps 15-16.

## Migration

Specify 2.0 is a hard cut from 1.x with no interim release. `migrate-to-2.0.sh` renames `project.yaml`, `registry.yaml`, `plan.yaml`, `sources.yaml`, cache, and archive fields; moves skills; bumps `specify_version`; and adds `reviewed` on first read. There is no `specify upgrade`; see [commands.md](commands.md). Dry-run against a 1.x fixture before tag.

Plugin authors: `adapter.yaml` fails on 2.0. Skill authors: use `/spec:execute`; stop conditions surface to the operator. Automation: Gate 1 is `plan.lifecycle == reviewed`; synthesis warnings come from `spec.md` or journal events.

## Alternatives considered

Full rationale: [decision-log.md](../docs/explanation/decision-log.md) when this RFC lands.

- Unified "lens" name: rejected; two qualified roles are clearer.
- Keep `/spec:extract`, `/change:analyze`, `/change:survey` as named skills: rejected because the names are the bifurcation.
- Per-source `spec.<source>.md`: rejected; one `spec.md` with `Sources:` lines.
- Source adapters emit artifacts / targets own define: rejected; blocks multi-source core synthesis.
- Target adapters in discovery: rejected; RFC-22 ledger territory.
- Defer multi-source synthesis: rejected; migration routinely combines code + docs.
- Adapter-only + workflow split across separate releases / separate RFC-25 + RFC-26: reversed; single 2.0 release, single migration script.
- `/spec:refine --loop`, `/spec:plan` as shim over `/change:`*, `/spec:define` kept, `/change:`* aliases: rejected.

## Non-goals

- 1.x backward compatibility.
- Plugin marketplace / runtime discovery.
- Per-handler provenance; per-Evidence confidence scores; auto-resolution of `[conflict]`.
- Cross-repo source sharing; bidirectional adapters; source adapters editing post-authoring artifacts.
- `/spec:execute` session tokens, `--continue`, or folding finalize into merge.
- Deleting `change.md` / `plan.yaml`; "manual mode" without execute.
- Gate 2, `/spec:execute` automation flags, parallel extract, per-claim authority overrides, multi-target projects, cross-mode hub+standalone driving; reinstate when a real consumer asks. [commands.md](commands.md) tracks the deferrals.

## Open questions

1. Default source adapters as true plugins vs CLI built-ins? **Preference:** in-repo plugins under `sources/`.
2. `extract` WASI sandboxing? **Preference:** RFC-15 `specify tool run` posture.
3. Required per-Evidence `authority:` vs manifest default? **Preference:** manifest default; explicit per-Evidence for mixed-class adapters later.
4. Required `claim-id` for `documentation` claim kinds? **Preference:** yes for `requirement-statement` / `acceptance-criterion`.
5. `[divergence]` override seam? **Preference:** edit `spec.md` in v1.
6. Slice dir at `plan add` vs extract start? **Preference:** at extract (Gate 1 plan-pure).
7. Lifecycle wire format? **Preference:** snake_case.

## Observability ([RFC-19](rfc-19-observability.md))


| Event                                                   | When                      |
| ------------------------------------------------------- | ------------------------- |
| `plan.transition.reviewed`                              | Gate 1 cleared            |
| `slice.transition.refined`                              | Synthesis completed       |
| `slice.extract.completed`                               | Per source key per slice  |
| `slice.synthesis.conflict` / `.divergence` / `.unknown` | Tags written to `spec.md` |


## References

- [RFC-19](rfc-19-observability.md) - journal events.
- [RFC-20 (archived)](archive/rfc-20-survey.md) - survey -> legacy source adapter.
- [RFC-21](rfc-21-catalogue.md) - `sources.yaml`.
- [RFC-22](rfc-22-ledger.md) - target-typed ledger entries.
- [RFC-23 (archived)](archive/rfc-23-change-lifecycle.md) - superseded lifecycle.
- [RFC-24](rfc-24-omnia.md) - target `shape`; Omnia as target adapter.
- [RFC-15 (archived)](archive/rfc-15-wasm-plugins.md) - WASI tools.
- [commands.md](commands.md) - CLI v1 floor.
- [specify-cli/AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) - exit codes.
- [project.mdc](../.cursor/rules/project.mdc) - artifact authority; synthesis hierarchy in §Synthesis contract.
- [AGENTS.md](../AGENTS.md) - plan-driven loop vocabulary.

