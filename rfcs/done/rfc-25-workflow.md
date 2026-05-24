# RFC-25: Workflow

> **Implemented.** The in-force contract lives in [`specify-cli/docs/standards/workflow.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md); cite that document by `§`-anchor from code and skill briefs. This RFC is retained for historical motivation and design context.
>
> Status: Implemented. Supersedes [RFC-20 (archived)](archive/rfc-20-survey.md) and [RFC-23 (archived)](archive/rfc-23-change-lifecycle.md). Ships as Specify 2.0. Compatible with [RFC-22](rfc-22-ledger.md) and [RFC-24](rfc-24-omnia.md) (target rename + `shape` ownership).

## Abstract

RFC-25 makes Specify 2.0 a single coherent workflow:

1. **Source adapters produce evidence.** They enumerate slice candidates at plan time and extract `Evidence` at slice time.
2. **Target adapters produce code.** They provide `shape` guidance plus `build` and `merge`; they do not synthesize `spec.md` or `design.md`.
3. **Core owns synthesis at both layers.** At plan time, core fuses `Candidate[]` into `slices[]` rows in `plan.yaml` (`/spec:plan`'s `propose` sub-step). At slice time, core fuses `Evidence[]` into `proposal.md`, `spec.md`, `design.md`, and `tasks.md`. Both layers are agent-default with operator override; uncertainty produces review tags rather than parking the workflow.
4. *Operators use one `/spec:` surface.* `/change:`* retires; `/spec:plan`, `/spec:execute`, and `/spec:finalize` become the default rhythm.
5. **Every change has a plan and Gate 1.** N=1 is degenerate, not special. Gate 1 is `plan.lifecycle == reviewed`.
6. **2.0 is a hard cut.** No compatibility aliases for old manifests, verbs, brief paths, or `/change:`*.

```text
source adapters --enumerate--> discovery.md --core propose--> plan.yaml
        |
        `--extract--> evidence/*.yaml --core synthesize--> proposal, spec, design, tasks
                                                     |
                                                     v
                                      target adapters (shape, build, merge) --> code
```

**Operator rhythm:** `/spec:plan` -> Gate 1 (`plan.lifecycle == reviewed`) -> `/spec:execute` -> `/spec:finalize`. Breakouts: `/spec:refine`, `/spec:build`, `/spec:merge`.

## How to read this RFC


| Audience                | Start here                                                               |
| ----------------------- | ------------------------------------------------------------------------ |
| Operator / skill author | §Operator workflow -> §Execution model -> §CLI surface                   |
| Source adapter author   | §Source adapter contract -> §Synthesis contract -> §Worked examples      |
| Target adapter author   | §Target adapter contract -> §Synthesis contract (`shape` only)           |
| CLI implementer         | §Normative decisions -> §Implementation contract -> §Implementation plan |
| Migrating from 1.x      | §Migration                                                               |


## Motivation

This RFC unifies two significant changes to Specify in a single release. The changes, source and target adapters, and plan-led workflow to reduce the Specify core to the orchestration of small point changes through to large-scale migrations.

**Adapter axis.** `/change:analyze` and `/change:survey` are one operation with two evidence sources; `/spec:define` and `/spec:extract` repeat the pattern at slice time. Legacy migration archaeology belongs in add-on source adapters, not core. Unqualified `adapter` only names outputs today, leaving no symmetrical term for inputs.

**Workflow axis.** The `/change:`* vs `/spec:`* split is a workflow seam, not an adapter seam. It forces a two-namespace operator surface, an orphan `/spec:define` path for trivial work, and a review pause enforced by skill exit instead of observable `plan.yaml` state.

**One redesign.** Qualify adapters by direction; collapse operator vocabulary to `/spec:`*; make Gate 1 the operator-stamped, CLI-written `plan.lifecycle == reviewed`; keep `change.md` and `plan.yaml` at every slice count.

## Normative decisions


| ID                                      | Decision                                                                                                                    | Implementation consequence                                                                                                                                                   |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **D1 Source/target split**              | Replace unqualified adapters with source adapters (`enumerate`, `extract`) and target adapters (`shape`, `build`, `merge`). | `adapters/sources/<name>/adapter.yaml`, `adapters/targets/<name>/adapter.yaml`, axis-aware resolver and cache.                                                                                 |
| **D2 Core synthesis**                   | Source adapters emit evidence only; target adapters supply `shape` only; core owns canonical artifacts.                     | `/spec:refine` runs extraction, synthesis, validation, and lifecycle transition.                                                                                             |
| **D3 Multi-source slices**              | `Slice.sources` is a list with cardinality >= 1.                                                                            | Combined evidence is first-class; pure intent/port/design are degenerate one-source cases.                                                                                   |
| **D4 Provenance and disagreement tags** | Requirements carry `ID:`, `Sources:`, and `Status:`.                                                                        | Core emits `agreed`, `unknown`, `conflict`, or `divergence`; tags never park the slice.                                                                                      |
| **D5 Always plan**                      | Every change runs through `enumerate` and `plan.yaml`, including N=1.                                                       | `/spec:define` retires; trivial work uses degenerate `intent.enumerate`.                                                                                                     |
| **D6 Gate 1 only**                      | Human review happens between planning and execution via `plan.lifecycle == reviewed`.                                       | No Gate 2 and no synthesis review state in v1.                                                                                                                               |
| **D7 Supervised execute**               | `/spec:execute` is the only v1 driver and resumes from on-disk state.                                                       | No `--yes-plan`, `--one`, `--until`, `--dry-run`, or `--continue`.                                                                                                           |
| **D8 CLI owns workflow writes**         | CLI is the single writer for lifecycle and deterministic files.                                                             | Never hand-write `plan.yaml`, `.metadata.yaml`, archive paths, `discovery.md`, `sources.yaml`, or `targets.yaml`.                                                            |
| **D9 Workspace routing is uniform**     | Loop and breakout verbs share the same workspace root -> project slot routing.                                              | Breakouts resolve the active slice project before phase work.                                                                                                                |
| **D10 Hard cut at 2.0**                 | 1.x manifests, verbs, brief paths, and `/change:`* retire together.                                                         | Migration script performs mechanical renames; no compatibility aliases.                                                                                                      |
| **D11 Automated propose**               | `/spec:plan`'s `propose` sub-step fuses candidates into slices automatically; uncertain merges tag and proceed to Gate 1.   | Agent writes merged `slices[]` without operator merge ceremony; operator override is `specify plan amend` at Gate 1; mirrors slice-time synthesis tag-and-proceed behaviour. |


## Operator workflow

### What changes from 1.x


| Before (1.x)                                         | After (2.0)                                                                |
| ---------------------------------------------------- | -------------------------------------------------------------------------- |
| `/change:draft`, `/change:survey`, `/change:analyze` | `/spec:plan` (`source.enumerate`)                                          |
| `/spec:define`, `/spec:extract`                      | `/spec:refine` (`source.extract` + core synthesis)                         |
| `/change:execute loop`                               | `/spec:execute`                                                            |
| `/change:finalize`                                   | `/spec:finalize`                                                           |
| `adapters/<name>/adapter.yaml`, `Slice.adapter`      | `adapters/targets/<name>/adapter.yaml`, `Slice.target`                              |
| `specify adapter` *, `specify change`*               | `specify source *`, `specify target *`, `specify plan *`; see §CLI surface |


### Commands

Default rhythm: `/spec:plan` -> review -> `/spec:execute` -> review on stops -> `/spec:finalize`.


| Stage      | Command                                       | Replaces                                             |
| ---------- | --------------------------------------------- | ---------------------------------------------------- |
| Plan       | `/spec:plan <name> [source <key>=<path> ...]` | `/change:draft`, `/change:survey`, `/change:analyze` |
| Drive plan | `/spec:execute`                               | `/change:execute loop`                               |
| Deliver    | `/spec:finalize <name>`                       | `/change:finalize`                                   |



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
reviewed                  refining                       extract + synthesize
  |                       refined                        spec.md (+ inline tags)
  |                       built                          /spec:build
  |                       merged                         /spec:merge -> entry done
reviewed (all done)       -                              /spec:finalize
```

The plan lifecycle has two stored states: `pending` (default after `plan create`) and `reviewed` (operator-stamped at Gate 1). It does not move further during execution. "Currently executing" and "drained" are computed from per-entry `status`: any entry `in-progress` means execution is live; all entries `done` means the plan is drained and `/spec:finalize` is legal. `/spec:execute` exits when no `pending` or `in-progress` per-entry remains.

`**/spec:plan**` runs pre-flight, scaffolds `change.md` and `plan.yaml`, runs workspace registry validation and `workspace sync` when needed, enumerates each source, runs `propose` to fuse candidates into `slices[]` automatically (see §Synthesis contract -> Plan-time fusion), assigns workspace projects when needed, and validates the plan. It exits at `pending` and prints the literal `specify plan transition <name> reviewed` command in its closing message; the operator stamps Gate 1 explicitly. `/spec:plan` never writes `reviewed` itself.

`**/spec:execute`** refuses unless the plan is `reviewed`, acquires the plan lock (workspace root in workspace mode), and loops: `specify plan next` -> workspace project resolution and `workspace sync` of the active slot when needed -> `/spec:refine` if needed -> `/spec:build` -> `/spec:merge` -> residue commit and return to workspace root when needed -> repeat until no per-entry `pending` or `in-progress` remains.

`**/spec:finalize`** requires all per-entry `status: done`, pushes branches, observes PRs until `MERGED`, then runs `specify plan finalize` to archive the plan.

**Breakouts** share skill bodies with the loop. `/spec:refine` requires an active `in-progress` entry from `plan next` and never writes `in-progress` itself. `/spec:build` refuses only on slice lifecycle, not on synthesis tags. `/spec:merge` is the only writer of per-entry `done`.


| Trigger                 | `/spec:execute` behavior     | Operator next                        |
| ----------------------- | ---------------------------- | ------------------------------------ |
| Build non-zero exit     | Stop with task id + log      | Fix; re-run execute or `/spec:build` |
| Merge baseline conflict | Stop with paths              | Resolve; re-run execute              |
| All per-entry `done`    | Exit; `/spec:finalize` ready | Run finalize                         |


After Gate 1 without execute: `specify plan next`, then `/spec:refine`.

### The plan gate


| Gate       | Position                                    | Mechanism                                               |
| ---------- | ------------------------------------------- | ------------------------------------------------------- |
| **Gate 1** | After plan validate, before `/spec:execute` | Operator runs `specify plan transition <name> reviewed` |


Gate 1 is the successor to RFC-23's explicit human seam: the operator is the only writer of `reviewed`, and `/spec:plan` exits at `pending` with the literal command in its closing hint. No Gate 2 ships in v1; see §Non-goals.

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
    status: pending
```

`change.md` is scaffolded at plan time and editable at Gate 1.

### Single-repo vs multi-repo


| `workspace:` | `/spec:plan` behavior                                                                     |
| ------------ | ----------------------------------------------------------------------------------------- |
| `false`      | Single root; skip `workspace sync` and assignment.                                        |
| `true`       | `registry.yaml`; `workspace sync` before enumerate; per-candidate `--project` at propose. |


**One driving mode per project in v1.** Workspace-registered projects are workspace-driven only; `/spec:plan` from a project root while a workspace plan is active is refused at plan-create.

### Workspace routing

Plan artifacts live at the workspace root; each project's slot lives at `.specify/workspace/<project>/` and carries its own `.specify/slices/<name>/` tree. Breakouts and `/spec:execute` share routing: plan lock at workspace root -> resolve active slice project -> `workspace sync` -> `chdir` -> phase work -> return.

### Plan lock

The plan lock is the file-level mutex that prevents two `/spec:execute` runs (or an execute-plus-breakout pair) from racing on the same plan. It is a sidecar lockfile at `.specify/plan.lock` (in single-repo mode) or at `<workspace-root>/.specify/plan.lock` (in workspace mode), acquired with an exclusive advisory file lock (`flock(LOCK_EX | LOCK_NB)` on POSIX, `LockFileEx` on Windows) and released on process exit. The lockfile body carries the holder's pid, hostname, and acquisition timestamp for diagnostics; the lock identity is the file lock itself, not the body. Acquisition is non-blocking — a second `/spec:execute` (or a breakout) that finds the lock held exits immediately with structured error `plan-lock-busy` carrying the holder's pid; the operator either waits or, if the holder is dead, removes the stale lockfile by hand. v1 has no `specify plan lock {acquire,release,status}` verb (see §CLI surface) — the lock is purely internal to `/spec:execute` and the breakout verbs. Stale-lock detection (pid liveness, watchdog auto-release) is deferred until a real consumer asks; an operator-removed lockfile is the v1 escape hatch.

## Concepts

### Adapter vocabulary


| Term                          | Meaning                                                                                                                                                    |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **source adapter**            | Input role: `enumerate` + `extract`. Examples: `intent`, `documentation`, `code-typescript`, `openapi`.                                                    |
| **target adapter**            | Output role: `shape` + `build` + `merge`. Examples: `omnia`, `vectis`, `contracts`. Replaces unqualified `adapter`.                                        |
| **plugin**                    | Shared shape for either adapter role; schema `adapter.schema.json`, loader `crates/domain/src/adapter/`, audience tag for source + target adapter authors. |
| **candidate**                 | Slice-sized unit from `enumerate`; blocks under `## Candidate inventory` in `discovery.md`.                                                                |
| **evidence**                  | Per-source result of `extract`; a structured document with `claims:`; persisted before synthesis.                                                          |
| **provenance**                | Sources behind one requirement (`Sources:` list).                                                                                                          |
| **conflict** / **divergence** | Unresolvable vs authority-resolved disagreement; `[conflict]` / `[divergence]` tags.                                                                       |
| **authority**                 | Closed enum: `intent`, `documentation`, `behaviour` (highest first; see §Authority hierarchy).                                                             |


`provider` is reserved for Omnia DI. `profile` is retired. Unqualified `adapter` is removed. The slice-vs-change on-disk distinction in [project.mdc](../.cursor/rules/project.mdc) survives; only slash commands collapse to `/spec:`*.

### Workflow vocabulary


| Term                     | Meaning                                                                                                                                          |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| **change**               | On-disk umbrella: `change.md`, `plan.yaml`, archive; not a slash command.                                                                        |
| **plan**                 | `/spec:plan`, `specify plan` *, Gate 1.                                                                                                          |
| **propose**              | `/spec:plan` sub-step that fuses `Candidate[]` from `enumerate` into `slices[]` rows in `plan.yaml`. Agent-default; operator override at Gate 1. |
| **slice**                | One refine -> build -> merge unit.                                                                                                               |
| **refine** / **execute** | `/spec:refine` (per slice); `/spec:execute` (supervised driver).                                                                                 |
| **gate**                 | CLI-stamped transition; v1: Gate 1 only.                                                                                                         |
| **breakout verb**        | `/spec:refine`, `/spec:build`, `/spec:merge`.                                                                                                    |
| **active slice**         | Plan entry currently `in-progress`.                                                                                                              |
| **plan lifecycle**       | `pending -> reviewed`. Two stored states; "drained" and "currently executing" are computed from per-entry `status`.                              |
| **per-entry lifecycle**  | `pending -> in-progress -> done`.                                                                                                                |
| **slice lifecycle**      | `refining -> refined -> built -> merged`.                                                                                                        |


## Implementation contract

### Types

Names used in function signatures and table cells throughout this RFC. Concrete shape is the canonical example or schema noted in the right column.


| Name        | Shape                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Reference                                                        |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `Source`    | Plan-level entry under `plan.yaml.sources`: source-key (kebab-case) -> adapter + path or value.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | §Planning at every scale; §`Slice.sources`.                      |
| `Candidate` | Slice-sized unit emitted by `enumerate`; one markdown block under `## Candidate inventory` in `discovery.md` with stable `id` and `sources[]`.                                                                                                                                                                                                                                                                                                                                                                                                                                           | §Discovery handshake; `schemas/discovery/candidate.schema.json`. |
| `Evidence`  | Per-source result of `extract`; persisted to `.specify/slices/<slice>/evidence/<source-key>.yaml`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | §`extract`; `schemas/evidence.schema.json`.                      |
| `Slice`     | One slice entry under `plan.yaml.slices[]`; carries `target`, `project`, `status`, structured `sources[]` (each entry pairs a source `key` referencing `plan.yaml.sources.<key>` with the `candidate` id from `discovery.md` that contributed to the slice; readable string shorthand `<key>` normalises to `{ key: <key>, candidate: <slice.name> }`), and optional `divergence` (closed enum: `none` (default; absent in YAML) | `likely` (set by `propose`) | `accepted` | `rejected` (operator-set via `plan amend --divergence`); advisory metadata in v1 — see §Plan-time fusion). | §`Slice.sources`; §On-disk and tooling.                          |


### Writer ownership

The CLI MUST be the single writer for deterministic workflow state:


| Artifact                          | Writer                                              |
| --------------------------------- | --------------------------------------------------- |
| `plan.yaml` lifecycle and entries | `specify plan` *                                    |
| `.metadata.yaml` lifecycle        | `specify slice` *                                   |
| Archive moves                     | `specify plan finalize`, `specify slice merge/drop` |
| `discovery.md`                    | `/spec:plan` through CLI helpers                    |
| `sources.yaml` / `targets.yaml`   | CLI registry/catalogue commands                     |


Adapters retain ownership of the briefs they ship. Skills and adapters retain write authority over evidence content, artifacts, and implementation code when their contract allows it. They MUST NOT hand-edit lifecycle files or archive paths.

`specify plan add` and `specify plan amend` are `plan.yaml`-only operations: they MUST NOT create, move, or delete any path under `.specify/slices/`. Slice directories are created by `specify slice create`, which `/spec:refine` invokes immediately before serial `extract` (see §`/spec:refine` pipeline). Between Gate 1 and the first `/spec:execute` (or `/spec:refine` breakout) the on-disk shape is plan-only — `.specify/slices/` is empty — so plan edits at Gate 1 never produce orphan slice directories.

### Adapter implementation shape

Source: `adapters/sources/<name>/adapter.yaml`. Target: `adapters/targets/<name>/adapter.yaml`.

Shared rules: kebab-case `name` unique per axis; `axis: source | target`; closed `operations[]` (`enumerate`/`extract` for sources, `shape`/`build`/`merge` for targets); `briefs.<operation>` required; optional `tools[]` per RFC-15 into `.specify/.cache/adapters/{sources,targets}/<name>/`.

`detect[]` auto-detection from paths is deferred; operators bind explicitly (`source legacy=./repo`).

```yaml
# adapters/sources/<name>/adapter.yaml
name: code-typescript
version: 1
axis: source
operations: [enumerate, extract]
briefs:
  enumerate: briefs/enumerate.md
  extract:   briefs/extract.md
```

```yaml
# adapters/targets/<name>/adapter.yaml
name: omnia
version: 1
axis: target
operations: [shape, build, merge]
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
```

### Resolver and cache

```text
.specify/.cache/
|-- adapters/sources/{intent,documentation,code-typescript,...}/
`-- adapters/targets/{omnia,vectis,contracts,...}/
```

One resolver module (`crates/domain/src/adapter/`) routes by axis.

### Wire format

Lifecycle and per-entry state values are kebab-case in every external surface: on-disk YAML, structured CLI output (`--format json`), RFC-19 journal event payloads, and error discriminants. The only states with a separator are per-entry `in-progress` and the slice transition target `dropped --reason ...`; both emit as written here, with no snake_case alias.

Rust enum variants stay snake_case internally and reach the wire via `#[serde(rename = "...")]`, identical to the `specify_version` -> `specify-version` precedent in §Implementation plan. Consumers parsing CLI output or journal events MUST treat kebab-case as canonical; snake_case lifecycle values are not produced anywhere and parsers MAY reject them.

## Source adapter contract

### `enumerate(Source) -> Candidate[]`

Runs at plan time. `/spec:plan` writes `discovery.md` — `## Summary`, `## Source inventory`, and `## Candidate inventory` — using the candidate grammar below plus stable `id` and `sources[]`.

### `extract(Candidate, Source) -> Evidence`

Runs at slice time. `/spec:refine` persists `Evidence` under `.specify/slices/<slice>/evidence/<source-key>.yaml`.

```yaml
source: legacy-monolith
adapter: code-typescript
authority: behaviour
candidate: user-registration
claims:
  - kind: excerpt
    claim-id: users.register.email-validation
    path: src/users/register.ts#L12-L87
```

Closed `kind` enum: `intent`, `requirement`, `criterion`, `decision`, `section`, `diagram`, `contract`, `excerpt`, `type`, `call`, `region`, `container`, `leaf`. New kinds require an RFC update. The three spatial kinds (`region`, `container`, `leaf`) are co-introduced with the first-party `screenshots` source adapter (see §Default source adapters) and carry layout-region, container-grouping, and individual-element claims respectively; their schema shape lives in `schemas/evidence.schema.json` alongside the textual kinds. No raw source bodies by default.

Top-level `authority:` is required per `Evidence`. `claim-id` is required on `requirement` and `criterion` claim kinds (deterministic fusion); other kinds may carry it as well. `Evidence` validates against `schemas/evidence.schema.json`; CLI writes paths and adapters return content via briefs/tools only.

Claim `path:` carries an optional GitHub-style anchor: `<path>` for whole-file claims, `<path>#L<n>` for a single line, `<path>#L<start>-L<end>` for a range. The schema enforces the grammar; consumers that need numeric bounds parse the anchor.

### Sandboxing

Source adapter operations (`enumerate` and `extract`) run under the RFC-15 `specify tool run` posture: WASI Preview 2 modules with directory preopens, no inherited host environment, no runtime network access, and a fixed working directory. Source adapter tools MUST be declared in the manifest's `tools[]` (per §Adapter implementation shape) and run through the same host runner that targets and other RFC-15 tools use.

What changes for source adapters is the **operator-bound source path**, which is neither `$PROJECT_DIR` nor `$CAPABILITY_DIR`. When a source binding under `plan.yaml.sources.<key>` carries a `path:`, the CLI exposes that path as a third runtime root, `$SOURCE_DIR`, and pre-opens it **read-only** for the duration of the operation. Bindings that carry `value:` instead (e.g. `intent`) do not produce a `$SOURCE_DIR` preopen.

Per-operation filesystem grant for source adapters:


| Root              | Mode       | Contents                                                                            |
| ----------------- | ---------- | ----------------------------------------------------------------------------------- |
| `$SOURCE_DIR`     | read-only  | The operator-bound source path; absent for `value:`-style bindings.                 |
| `$CAPABILITY_DIR` | read-only  | `.specify/.cache/adapters/sources/<adapter>/` — adapter-owned cache, per RFC-15.             |
| `$SCRATCH_DIR`    | write-only | `.specify/.cache/adapters/sources/<adapter>/<slice>/` — per-slice scratch for the run.       |
| `$PROJECT_DIR`    | none       | Source-adapter tools do not get the project root; lifecycle state stays off-limits. |


Access outside these roots is denied. Symlinks are resolved during canonicalization, identical to RFC-15: a symlink inside `$SOURCE_DIR` pointing outside it is denied even if its textual path looks contained. The minimal environment exposes `$PROJECT_DIR`, `$CAPABILITY_DIR`, `$SCRATCH_DIR`, and (when present) `$SOURCE_DIR`; nothing else is inherited.

Denied filesystem access is surfaced as a structured `source-extract-path-denied` (or `source-enumerate-path-denied`) error from the host runner; the adapter MUST NOT swallow it. See §Extraction reliability for how a path-denied error interacts with the slice lifecycle.

This deliberately excludes `$PROJECT_DIR` from source-adapter grants: source adapters analyse external material (legacy code, documentation, intent) and write Evidence through CLI-mediated paths, never directly into project lifecycle state. Write access to the canonical `evidence/<source-key>.yaml` location is the CLI's responsibility — adapters return content via brief output that the CLI persists.

### Default source adapters


| Adapter         | Authority emitted | Role                                                                                                                                                                                   |
| --------------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `intent`        | `intent`          | Operator briefs and overrides.                                                                                                                                                         |
| `documentation` | `documentation`   | Operator-provided written product/technical intent.                                                                                                                                    |
| `screenshots`   | `documentation`   | Vision-assisted spatial inference over a directory of screen images; emits `region` / `container` / `leaf` Evidence claims for downstream targets that need layout structure (Vectis). |


All three ship as in-repo plugins under `adapters/sources/intent/`, `adapters/sources/documentation/`, and `adapters/sources/screenshots/` with the same `adapter.yaml` + `briefs/` shape as every other source adapter. The adapter loader (`crates/domain/src/adapter/`) MUST resolve them through the same code path as a third-party source adapter; there is no `if name == "intent" { ... }` branch in core and no built-in fallback when the manifests are missing. Renaming or removing a manifest takes the corresponding adapter out of the resolver's set, identical behaviour to a third-party adapter.

`screenshots` houses the body of the legacy Vectis `image-layout-inferer` skill, restructured as a source adapter: `enumerate` identifies candidate screens from the bound directory and writes one block per screen under `## Candidate inventory`; `extract` emits structured spatial Evidence per candidate (the new `region` / `container` / `leaf` claim kinds). The migration script retires `plugins/vectis/skills/image-layout-inferer/` and the hand-authored `layout.yaml` Specify artifact in 2.0 — see §Migration and §Note to the implementing agent. The v1 `enumerate` and `extract` briefs are the current `image-layout-inferer` prompt verbatim, just resliced into the two source-adapter operations; v1 does not redesign the inference algorithm.

The first-party `code-typescript` source adapter ships alongside the three above; its body is the retired `change survey` TypeScript enumerator rehomed under `adapters/sources/code-typescript/`. Authority emitted is `behaviour`; enumeration grammar stays adapter-internal (§`discovery.md` consolidation) and unchanged from its 1.x form. Other code languages remain deferred per §Repository layout.

N=1 greenfield uses degenerate `intent.enumerate` via this normal resolution path.

### Discovery handshake

`discovery.md` is the single plan-time discovery artifact. 1.x `survey.md` retires; its `Summary` and `Source inventory` sections fold into `discovery.md` rather than a sibling file. Required sections, in order:

1. `## Summary` — one-line counts (`Sources`, `Candidates`); adapter-specific tallies are permitted but unspecified here.
2. `## Source inventory` — one row per bound source (key, adapter, path or value).
3. `## Candidate inventory` — one block per candidate (see below).

N=1 `intent.enumerate` may leave `Summary` and `Source inventory` minimal; the file still exists at plan time.

Each candidate has stable `id` and `sources[]`. The `/spec:plan` agent merges cross-source duplicates at `propose` time (see §Synthesis contract -> Plan-time fusion); the operator overrides via `specify plan amend` at Gate 1. Re-enumerating the same source replaces by `id`; enumerating a different source appends new ids. Schema: `schemas/discovery/candidate.schema.json`. No `candidates.yaml` in v1.

Minimal candidate block under `## Candidate inventory`:

```markdown
### user-registration

- id: user-registration
- sources: [legacy-monolith]
- summary: Registration endpoint accepting email + password with RFC-5322 validation.
```

`id` is the stable handle re-enumeration writes against. `sources[]` lists the sources that surfaced this candidate. Cross-source merge decisions are recorded in `plan.yaml.slices[].sources[]` rather than on the candidate block — see §Synthesis contract -> Plan-time fusion.

### `Slice.sources`

`Slice.sources` is a list of one or more `{ key, candidate }` bindings. Each entry pairs a source `key` (referencing a top-level binding under `plan.yaml.sources.<key>`) with the `candidate` id from `discovery.md` that contributed to the slice. The (key, candidate) tuple is what `/spec:refine` feeds into per-source `extract`, and is what survives re-enumeration:

```yaml
slices:
  - name: identity-user-registration
    target: omnia
    project: identity-svc
    sources:
      - key: identity-design-notes
        candidate: user-registration
      - key: legacy-monolith
        candidate: user-registration
    status: pending
```

Bare-string shorthand: an entry MAY be written as a plain `<key>` when the candidate id equals the slice's `name`. The reader normalises `<key>` to `{ key: <key>, candidate: <slice.name> }`; the CLI always writes the structured form. This keeps the degenerate intent case readable:

```yaml
sources: [intent]   # sugar for [{ key: intent, candidate: <slice.name> }]
```


| Archetype         | `sources`                                        | Notes                                                                               |
| ----------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------- |
| Pure greenfield   | `[intent]` (shorthand)                           | Empty/missing list normalises to this; candidate id equals slice name.              |
| Pure port         | One binding `{ key: <legacy>, candidate: <id> }` | Code dictates behavior; `<id>` is the candidate id from the legacy enumerate.       |
| Pure design       | One binding `{ key: <doc>, candidate: <id> }`    | Docs dictate behavior; `<id>` is the candidate id from the documentation enumerate. |
| Combined evidence | Multiple bindings, one per contributing source   | Per-source candidate ids may differ; authority hierarchy resolves disagreements.    |


`specify plan add` enforces at most one entry with `key: intent` per slice, at most one entry per `key`, and at least one source total.

### Worked multi-source `plan.yaml`

A complete plan covering two slices over a legacy code source plus a design-notes source — illustrating the relationship between the plan-level `sources:` map and per-slice `slices[].sources[]` bindings:

```yaml
version: 1
name: identity-revamp
sources:
  identity-design-notes:
    adapter: documentation
    path: ./design-notes/identity
  legacy-monolith:
    adapter: code-typescript
    path: ./vendor/legacy-monolith
slices:
  - name: identity-user-registration
    target: omnia
    project: identity-svc
    sources:
      - key: identity-design-notes
        candidate: user-registration
      - key: legacy-monolith
        candidate: user-registration
    status: pending
  - name: identity-password-reset
    target: omnia
    project: identity-svc
    sources:
      - key: identity-design-notes
        candidate: password-reset
      - key: legacy-monolith
        candidate: account-pwd-reset
    divergence: likely
    status: pending
```

The first slice has matching candidate ids across both sources — `propose` fused them into one row without further annotation. The second slice has differently-named candidates that `propose` judged to refer to the same unit of work (likely-divergence call-out lives in `change.md`'s `## Likely divergences` block); the slice carries `divergence: likely` so the operator's Gate-1 acknowledgement (or rejection) is recorded against the slice. Both slices route to the same workspace project (`identity-svc`); a workspace plan with slices targeting different projects would carry distinct `project:` values per slice.

## Target adapter contract

Target adapters do not own `spec.md` or `design.md` synthesis. They may declare:

- `**shape**`: idiom guidance consumed by core synthesis.
- `**build**` / `**merge**`: implementation and landing briefs/tools.

`specify adapter pipeline {define,build,merge}` retires. RFC-24 "adapter-gated" becomes "target-gated"; `Slice.adapter` becomes `Slice.target`.

### Target-specific structured outputs

Target adapters MAY produce target-specific structured manifests (e.g. Vectis `composition.yaml`) as part of `build`. Such manifests are not synthesised by core, do not require a fourth target-adapter capability, and are landed by `merge` alongside implementation code. The build brief reads `spec.md` + `design.md` (which already carry any upstream spatial or structural claims that core synthesis folded in from source adapters) and writes the manifest in the same pass as the code it accompanies. This keeps the three-capability model intact and removes the apparent need for a refine-time slot for target-only artifacts: by the time `build` runs, the canonical artifacts already carry every claim the target needs to wire its structured outputs together. See §Migration for the Vectis-specific consequence (`composition.yaml` regenerates on the first 2.0 `/spec:execute`).

## Synthesis contract

Core owns automated fusion at two layers, both agent-default with operator override:

- **Plan-time fusion** runs inside `/spec:plan`'s `propose` sub-step. Inputs: `Candidate[]` from each source's `enumerate`. Output: `slices[]` rows in `plan.yaml` with merged `sources[]`, and `slices[].divergence: likely` set on slices whose merged candidates have materially disagreeing summaries. Operator override: `specify plan amend` at Gate 1.
- **Slice-time fusion** runs inside `/spec:refine`. Inputs: `Evidence[]`, `Slice`, and optional target `shape` brief. Outputs: `proposal.md`, `spec.md`, `design.md`, and `tasks.md`. Operator override: hand-edit `spec.md` after tags surface.

Agent authors from `plugins/spec/references/synthesis/`; CLI validates structure and stamps lifecycle. Both layers follow the same rule: uncertainty produces review tags, never parks the workflow.

### Plan-time fusion: `/spec:plan`'s `propose` sub-step

`/spec:plan` runs `propose` after `enumerate` and before plan validate / Gate 1. The agent reads the full `## Candidate inventory` in `discovery.md` and writes `slices[]` rows:

1. Identify candidates that name the same unit of work across sources, using each candidate's `id`, `summary`, and `sources[]`.
2. Drive `specify plan add` per proposed slice, passing every contributing `(source-key, candidate-id)` pair. The CLI writes the slice row with one structured `{ key, candidate }` entry per contributing source under `slices[].sources[]`; per-entry lifecycle starts at `pending` (unchanged from §Workflow vocabulary). The merge record lives entirely in `plan.yaml.slices[].sources[]` — each entry's `candidate` field is the back-reference to its `discovery.md` block, so no cross-reference is written back into `discovery.md`.
3. Annotate uncertain merges with `tentative: true` on the contributing candidate blocks in `discovery.md`, and call them out in a `## Tentative merges` block in `change.md` with prose reasoning.
4. When merged candidates share an `id` but their `summary` strings materially disagree (different numeric values, conflicting verbs, mutually exclusive nouns), set `divergence: likely` on the slice entry in `plan.yaml` and call the contributing candidate-pair summaries out in a `## Likely divergences` block in `change.md` with the values shown side by side. This signals that slice-time synthesis is expected to surface `[divergence]` once `Evidence` lands.

Tentative annotations are review signals: the plan still progresses to validate and Gate 1, and the per-entry lifecycle is unaffected. The operator reconciles tentative merges by editing `change.md` or running `specify plan amend` (split, merge, relabel, rebind sources) before stamping `reviewed`. Hard tie-breakers (e.g. two candidates share a name across docs and legacy with divergent summaries) emit both rows annotated `tentative: true` rather than failing the plan.

The `divergence:` field on slice entries follows the same review-signal posture: the plan still progresses to validate and Gate 1, and slice-time synthesis is not gated on operator response. The field is a closed enum with values `none` (default; absent in YAML), `likely` (set by `propose`), `accepted` (operator acknowledges the predicted divergence and wants execute to proceed), and `rejected` (operator disagrees with the prediction and intends to split, rebind, or otherwise amend the plan). The operator progresses the state via `specify plan amend <slice> --divergence accepted` (or `rejected`); see §Types and §CLI surface. The field is advisory metadata in v1 — no halt or park is wired against any value — and exists so the slice carries a durable record of "operator was warned at Gate 1" that future workflow gates can consume without a schema change. The pair-level detail lives in the `## Likely divergences` block of `change.md`; the slice-level field is the load-bearing state.

Authority hierarchy does not apply at `propose`; without `Evidence`, candidate fusion runs on headlines alone. Authority activates at slice-time synthesis (see §Authority hierarchy).

### `/spec:refine` pipeline

1. Resolve target and sources. Each `slices[].sources[]` binding supplies the `(source-key, candidate-id)` tuple that drives one `extract` call; the source key resolves to the top-level `plan.yaml.sources.<key>` binding, and the candidate id resolves to the matching block in `discovery.md`.
2. Create the slice directory via `specify slice create <name> --target <target>`; this is the first step that materialises any path under `.specify/slices/<name>/`. The CLI stamps `.metadata.yaml` at `refining` as part of create.
3. Run serial `extract` per §Extraction reliability, one call per `slices[].sources[]` binding, passing the binding's `candidate` id as the `Candidate` argument.
4. Synthesize in fixed substep order: `proposal` -> `specs` -> `design` -> `tasks`. Substeps are hand-coded in `/spec:refine` in v1 (no `specify slice synthesize` verb; see §CLI surface).
5. Run `specify slice validate` (§CLI surface).
6. Transition to `refined` via `specify slice transition <name> refined`.

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
2. `documentation`: operator-provided written product/technical intent (internal docs, RFCs, product notes). Matches the `documentation` source adapter; distinct from synthesized `design.md` and the refine substep `design`.
3. `behaviour`: what legacy code does.


| Agreement                    | Output                                                    |
| ---------------------------- | --------------------------------------------------------- |
| One source                   | `Status: agreed`                                          |
| Multiple agree               | `Status: agreed`, all keys in `Sources:`                  |
| Disagree, one winner         | `Status: divergence`, `[divergence]`, loser as commentary |
| Disagree, tied top authority | `Status: conflict`, `[conflict]`, operator reconciles     |
| No contributing Evidence     | `Status: unknown`, `[unknown]`                            |


Substep order and lifecycle behavior live with the `/spec:refine` pipeline above.

The override seam for an authority-resolved `[divergence]` (or any other tag) is hand-editing `spec.md` after `/spec:refine` transitions the slice to `refined` and before `/spec:build` begins. The operator removes the `[divergence]` tag from the requirement header, flips `Status: divergence` to `Status: agreed`, edits the body to the chosen value, and proceeds. `/spec:refine` is not idempotent against hand-edits — re-running it discards manual reconciliation — so operators reconcile once and move forward, or amend the plan (e.g. `--remove-source`) and re-refine cleanly. The `divergence:` field on the slice entry is the Gate-1 audit trail for "operator was warned about likely divergence on this slice"; it does not bypass or shortcut the spec.md edit.

### Extraction reliability


| Rule                         | Behavior                                                                                                                                                                                                                                                                                                                    |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Order**                    | Serial in `Slice.sources` declaration order.                                                                                                                                                                                                                                                                                |
| **Failure**                  | Any `extract` fails -> stay `refining`, no synthesis. Operator amends the plan to drop the source if they want to proceed without it.                                                                                                                                                                                       |
| **Path-denied**              | A read or write outside the source-adapter sandbox grants (see §Sandboxing) fails the `extract` with structured error `source-extract-path-denied`. Counts as a failure for the row above; the slice stays `refining`. Resolution paths: rebind the source via `plan amend` to include the needed root, or drop the source. |
| **Empty / Invalid Evidence** | Empty `claims: []` valid; invalid fails schema before synthesis.                                                                                                                                                                                                                                                            |


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
authority: documentation
candidate: password-reset
claims:
  - kind: requirement
    claim-id: password-reset.request
    path: docs/account.md#L3
    statement: "The account service should let a registered user request a password reset link by email."
  - kind: criterion
    claim-id: password-reset.response-privacy
    path: docs/account.md#L6
    criterion: "Unknown email addresses receive the same outward response as known users."
  - kind: criterion
    claim-id: password-reset.expiry
    path: docs/account.md#L7
    criterion: "Reset links expire after 30 minutes."
  - kind: decision
    path: docs/account.md#L9
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

The system expires password reset links after 30 minutes. (from identity-design-notes; documentation)

Note: legacy-monolith observed 24-hour expiry; the documentation authority overrides. Operator review recommended.
```

## On-disk and tooling

### `project.yaml`

```yaml
specify-version: 2.0.0
sources: [intent, documentation, code-typescript]
target: omnia
workspace: false
```

`sources` lists available adapters; configured sources live in `sources.yaml` ([RFC-21](rfc-21-catalogue.md)). v1 supports one `target` per project; `Slice.target` must match for workspace entries. `profile` and singular `adapter` are removed.

### `.specify/` layout

Regular project: `change.md`, `plan.yaml`, and `discovery.md` at root; `slices/<name>/` contains artifacts plus `evidence/<source-key>.yaml`.

Workspace: plan and discovery artifacts at the workspace root (in the workspace's own `.specify/`); each project's slot lives at `.specify/workspace/<project>/` and carries its own `.specify/slices/<name>/` tree. The workspace's own `.specify/slices/` is unused.

Slice directories appear lazily: `specify plan add` writes only into `plan.yaml`, and `slices/<name>/` is created by `specify slice create` at the start of `/spec:refine` (see §`/spec:refine` pipeline). At Gate 1 the slice tree is empty regardless of slice count; the on-disk shape an operator reviews is plan-only.

### `discovery.md` consolidation

`specify change survey` and per-plan `survey.md` retire. Enumeration metadata and candidates share one file: `discovery.md` with `## Summary`, `## Source inventory`, and `## Candidate inventory` (§Discovery handshake). Code source adapters validate their staged enumeration output through source-local tooling before `/spec:plan` appends candidates; the schema for that intermediate output is an adapter-internal concern.

### CLI surface

Specify 2.0 v1 target surface. Global on all rows: `--format text|json` (`SPECIFY_FORMAT`).

The v1 floor: the CLI is the single writer of files the skills must not hand-edit (`project.yaml`, `plan.yaml`, `.metadata.yaml`, archive paths), plus a small set of computations and side effects the agent shouldn't reimplement. Everything else — status, show, list, diagnostic helpers — is cut. Operators read YAML and Markdown files directly; skills do the same. Verbs return when a real caller asks for them.


| Command                            | Positionals          | Flags                                                                                                                                 |
| ---------------------------------- | -------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `specify init`                     | `<target>`           | `--name`, `--domain`                                                                                                                  |
| `specify init`                     |                      | `--workspace`, `--name`, `--domain`                                                                                                   |
| `specify source resolve`           | `<name>`             | `--project-dir`                                                                                                                       |
| `specify target resolve`           | `<value>`            | `--project-dir`                                                                                                                       |
| `specify plan create`              | `<name>`             | `--source`                                                                                                                            |
| `specify plan add`                 | `<name>`             | `--depends-on`, `--sources`, `--description`, `--project`, `--target`, `--context`                                                    |
| `specify plan amend`               | `<name>`             | `--depends-on`, `--sources`, `--add-source`, `--remove-source`, `--description`, `--project`, `--target`, `--context`, `--divergence` |
| `specify plan transition`          | `<name>`, `<target>` | `--reason`                                                                                                                            |
| `specify plan next`                |                      |                                                                                                                                       |
| `specify plan finalize`            | `<name>`             | `--clean`, `--dry-run`                                                                                                                |
| `specify slice create`             | `<name>`             | `--target`, `--if-exists`                                                                                                             |
| `specify slice transition`         | `<name>`, `<target>` | `--reason`                                                                                                                            |
| `specify slice validate`           | `<name>`             |                                                                                                                                       |
| `specify slice merge`              | `<name>`             | `--dry-run`, `--check-only`                                                                                                           |
| `specify workspace sync`           | `[<project>…]`       |                                                                                                                                       |
| `specify workspace push`           | `[<project>…]`       | `--dry-run`                                                                                                                           |
| `specify workspace prepare-branch` | `<project>`          | `--change`, `--source`, `--output`                                                                                                    |
| `specify tool run`                 | `<name>`, `[args…]`  | arguments after `--`                                                                                                                  |


`<target>` for `specify plan transition`: plan lifecycle `reviewed`; per-entry `done`. `pending` is written by `plan add` / `plan amend`, and `in-progress` is written only by `plan next`. `plan next` returns the active `in-progress` entry before selecting a new `pending` entry, and reports drained only when no active or pending entries remain. v1 has no per-entry `blocked`, `failed`, or `skipped` state; build failures and merge conflicts leave the active entry `in-progress`. `<target>` for `specify slice transition`: `refining`, `refined`, `built`, `dropped` (`--reason` only for `dropped`; the `merged` state is stamped by `specify slice merge`, never `slice transition`). Repeatable flags: `plan create --source`, `plan add` / `amend` `--depends-on` / `--sources` / `--add-source` / `--remove-source` / `--context`, `workspace prepare-branch` `--source` / `--output`. `plan add` / `plan amend` `--sources` and `--add-source` take `<key>=<candidate-id>` arguments — the source key references a top-level `plan.yaml.sources.<key>` binding and the candidate id references a `## Candidate inventory` block in `discovery.md`; the bare `<key>` shorthand is accepted only when the candidate id equals the slice's own `name` (typical for `intent`). `--remove-source` takes `<key>` alone (one binding per key per slice). `plan amend --add-source` / `--remove-source` only succeed while the slice's per-entry lifecycle is `pending` and the plan lifecycle is at most `reviewed`; rebinding an already-extracted slice requires `slice transition dropped` and re-add. `plan amend --divergence <accepted|rejected>` writes `slices[].divergence` and is accepted at any per-entry lifecycle state; the field is advisory metadata in v1 (no halt/park is wired against any value) and records operator acknowledgement (or rejection) of the `propose`-time `likely` prediction. `none` cannot be set explicitly — absence is none — and `likely` is reserved for the `propose` sub-step.

Axis deltas from 1.x: `specify source resolve`; `specify target resolve` (was `adapter resolve`); `specify plan transition ... reviewed`; `specify plan amend --add-source` / `--remove-source`; `specify change *` and `specify adapter pipeline` retire. Skill/slash retirements match §Operator workflow -> Commands.

#### What was cut and why

**Reads — operator or agent opens the file directly.**

`specify status`, `specify plan show`, `specify plan status`, `specify slice status`, `specify workspace status`, `specify registry show`, `specify source list`, `specify target list`, `specify tool list`, `specify tool show`, `specify slice journal show`, `specify slice outcome show`, `specify slice task progress`. Every one of these formatted a YAML or Markdown file that anyone can `cat`. No skill needs the CLI to read state back to it; the agent reads `.specify/project.yaml`, `.specify/plan.yaml`, `.specify/slices/<name>/.metadata.yaml` directly.

**Validation folded into the write verb.**

- `specify plan validate` — `plan add` and `plan amend` refuse to write an invalid plan; first-use validation is the seam.
- `specify source validate`, `specify target validate` — `source resolve` and `target resolve` validate the manifest on load.
- `specify registry validate` — `workspace sync` and `/spec:plan` refuse to operate on a malformed registry.
- `specify context check` — not needed without `context generate`.

**Folded into a parent verb.**

- `specify slice drop` -> `specify slice transition <name> dropped --reason "..."`.
- `specify slice outcome set` — not needed in v1. Slice lifecycle alone tells `/spec:execute` where to resume; the chat session and on-disk artifacts carry the failure diagnostic. Persisted phase outcomes are observability and belong with RFC-19. Reinstate when crash-recovery diagnostics must survive an agent restart.
- `specify slice journal append` — defer to RFC-19; nothing in v1 signals through the journal.
- `specify context generate` — `specify init` writes the initial `AGENTS.md` and `.specify/context.lock`. Drift detection (`--check`) is a CI affordance; ship when a CI integration asks for it.
- `specify tool fetch` — `specify tool run` fetches `.wasm` on first call.

**No skill caller in v1 — topology and helpers hand-coded in skills until a real caller appears.**

- `specify slice synthesize`, `specify target build`, `specify target merge` — synthesis and target brief topology for the two or three known target adapters (omnia, vectis, contracts) is hand-coded in `/spec:refine`, `/spec:build`, `/spec:merge`. Reinstate when a third-party target ships with custom brief ordering.
- `specify slice touched-specs` — `/spec:merge` diffs the slice's `specs/` against the baseline inline.
- `specify slice overlap` — parallel-slice safety; single-operator v1 has no parallel slices to coordinate.
- `specify slice task progress`, `specify slice task mark` — `/spec:build` greps `- [ ]` in `tasks.md` and edits the checkbox in place.
- `specify compatibility check` — defer until a real cross-project consumer exists.

**Deferred — separate consumer ask.**

- `slice transition refined_provisional` — the second structural gate (operator review of synthesis output as a parking state). Multi-source synthesis ships in v1; `/spec:refine` surfaces `[conflict]` / `[divergence]` / `[unknown]` inline in `spec.md` as review signals and `/spec:build` does not refuse on those tags. The `divergence:` enum on slice entries already carries the Gate-1 acknowledgement signal a future park would consume, with `surfaced` / `confirmed` / `resolved` reserved as forward-compatible values, so the parking state can be wired in without a schema change when a real consumer demands review-then-promote ergonomics, automation hooks, or CI gating around synthesis output.
- `--parallel-extract` flag (or implicit parallelism) on `/spec:refine`. v1 runs `extract` serially in `planSlice.sources` declaration order for deterministic goldens; parallel extraction returns when extract latency becomes a real workflow cost.
- `plan.yaml.slices[].authority-override` and per-claim authority overrides. v1 uses adapter-class defaults; per-slice and per-claim overrides return when editing `spec.md` after `[divergence]` is no longer an adequate operator seam.

**Operator-curated YAML — hand-edit, validation on first use.**

`specify registry add`, `specify registry remove`. `AGENTS.md` does not forbid hand-editing `registry.yaml` (the off-limits list is `.metadata.yaml`, archive paths, and `.specify/` scaffolding). Operators edit `registry.yaml` directly; `workspace sync` and `/spec:plan` validate at first use.

**Permanent surface for transient or never-existing need.**

- `specify upgrade` — migration ships as `migrate-to-2.0.sh` with the release notes.
- `specify plan archive` — covered by `plan finalize`.
- `specify plan lock {acquire, release, status}` — internal to `/spec:execute` and the breakout verbs.
- `specify tool gc` — `rm -rf .specify/.cache/` until cache pressure is a real workflow.
- `specify codex export` — moves into a `codex` target adapter under `specify target *`.

**Borderline — ship if trivial, otherwise defer.**

`specify completions <shell>` — no skill caller, but `clap_complete` is one line and shell completion is the most-expected nicety in a CLI. Ship when the `clap_complete` dependency is paid for any reason.

**Retired pre-redesign surface (verbs that never reach v1):**

`specify adapter *`, `specify change *`, `specify change survey`, `specify plan doctor`.

#### When verbs come back

Add a verb when at least one of these is true:

1. A skill body is reimplementing nontrivial domain logic that should live in the CLI.
2. A documented external consumer (CI, hosted runner, third-party adapter) needs the structured shape.
3. The on-disk file the verb writes is documented as off-limits to hand-editing.

Speculation — "we might need this someday" — is not on the list.

### Skill / `SKILL.md` changes


| File                                                         | Action                                                  |
| ------------------------------------------------------------ | ------------------------------------------------------- |
| `plugins/spec/skills/plan/SKILL.md`                          | New, from `change/draft`.                               |
| `plugins/spec/skills/execute/SKILL.md`                       | New, from `change/execute loop`.                        |
| `plugins/spec/skills/refine/SKILL.md`                        | Renamed from `define/`; plan-resolved sources.          |
| `plugins/spec/skills/{build,merge,finalize}/SKILL.md`        | Build/merge breakouts; finalize from `change/finalize`. |
| `plugins/spec/skills/init/SKILL.md`                          | Mention `/spec:plan`.                                   |
| `plugins/spec/skills/drop/SKILL.md`                          | Unchanged.                                              |
| `plugins/change/`**, `plugins/spec/skills/{define,extract}/` | Retired.                                                |


### Repository layout (monorepo v1)

```text
/
|-- plugins/
|   `-- spec/skills/{init,plan,refine,execute,build,merge,finalize,drop}/
|-- adapters/sources/
|   |-- intent/                       # adapter.yaml, briefs/
|   |-- documentation/
|   |-- screenshots/                  # was plugins/vectis/skills/image-layout-inferer/
|   `-- code-typescript/              # adapter-internal enumeration schema (was under change/)
|-- adapters/targets/                          # was adapters/
|   |-- omnia/                        # adapter.yaml, briefs/{shape,build,merge}.md
|   |-- vectis/                       # target-only after the source/target split
|   `-- contracts/
`-- schemas/                          # plugin, source, target, evidence, candidate
```

Deferred: other code languages; contract source adapters; per-adapter repo split.

## Implementation plan

Subagent-sized decomposition, wave ids (`Wn.k`), parallelism notes, and a live progress snapshot live in the companion [rfc-25-plan.md](rfc-25-plan.md).

Phase 1 (steps 1-13) lands the adapter model. Phase 2 (steps 14-17) lands workflow collapse in the same 2.0 release.

### Note to the implementing agent

This RFC renames or reshapes several names that are already deeply embedded in the Specify codebase. Treat every rename as a cross-cutting refactor, not a documentation edit: chase each old name through `crates/`, `tests/`, `schemas/`, `plugins/`, `docs/`, `AGENTS.md`, fixtures, golden files, and the sibling `augentic/specify-cli` repo before declaring a step done. The renames currently in scope include, but are not limited to:

- `hub` -> `workspace` (project-yaml discriminator, CLI flags, error codes such as `init-requires-adapter-or-hub` and `hub-cannot-be-project`, doc prose, fixture names, `init_hub` test helpers).
- `specify_version` -> `specify-version` in YAML surfaces only (kebab-case on disk, snake_case Rust field names stay snake_case — only `#[serde(rename = "specify-version")]` and the on-disk emit change).
- `Adapter*` types -> `Target*` (per implementation step 2); the `plugins/adapter/` loader -> `plugins/plugin/` (per step 3).
- `change survey` and `adapter pipeline` CLI verbs are retired (per step 8).
- `/change:*` and `/spec:define` skills are deleted (per step 17).
- `slices[].sources` reshapes from `string[]` (source keys) to `{ key, candidate }[]`; the standalone `slices[].candidate` field is removed and its value folds into each binding (per §`Slice.sources`). The schema accepts a bare `<key>` string as shorthand for `{ key: <key>, candidate: <slice.name> }`; the CLI always writes the structured form. `plan add` / `plan amend` `--sources` and `--add-source` flags take `<key>=<candidate-id>` arguments, with bare `<key>` accepted only under the same shorthand rule.
- Plan lifecycle collapses from `pending -> reviewed -> in-progress -> drained` to `pending -> reviewed`. Drop any code, schema enum, error discriminant, fixture, or doc reference to plan-level `in-progress` and `drained`; both are computed from per-entry `status` at read time. `specify plan transition` accepts only the plan-level target `reviewed` (per-entry `done` is unchanged); `/spec:plan` MUST NOT write `reviewed` itself — the operator runs the transition.
- Vectis source/target split: `plugins/vectis/skills/image-layout-inferer/` moves to `adapters/sources/screenshots/` and is restructured as a source adapter (`adapter.yaml` with `axis: source`, `operations: [enumerate, extract]`, plus `briefs/{enumerate,extract}.md`). `Evidence` schema gains the `region` / `container` / `leaf` claim kinds; `schemas/evidence.schema.json` and any fixture/golden files under `tests/` and the sibling `augentic/specify-cli` repo update in the same change. Baseline `layout.yaml` paths retire as a Specify artifact — operators re-emit equivalent data via `screenshots.extract` (or a hand-rolled local source adapter); `composition.yaml` is no longer a Specify artifact and is regenerated by `adapters/targets/vectis/build` on the first 2.0 `/spec:execute`. `tokens.yaml` and `assets.yaml` are unchanged: they remain operator-curated configuration consumed by the Vectis target's `build`.

For each rename: update the symbol, the JSON Schema, the YAML on-disk form, every test fixture and golden file, every error-code discriminant, every doc reference (including this RFC's siblings and the parent `AGENTS.md`), and the CLI `--help` text in the same change. Where the old name appears in archived RFCs under `rfcs/archive/`, leave it alone — archives are historical record. When in doubt, run `rg '<old-name>'` across both repos before opening the PR.


| Step | Decisions   | Deliverable                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Acceptance           |
| ---- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------- |
| 1    | D1, D3, D6  | Land the JSON Schemas this RFC references (`schemas/adapter.schema.json`, `schemas/source.schema.json`, `schemas/target.schema.json`, `schemas/evidence.schema.json`, `schemas/discovery/candidate.schema.json`, plus the `plan.yaml` schema's `target` field and structured `slices[].sources[]` shape and `pending`/`reviewed` plan-lifecycle enum). None of these exist in the tree today; this step ships them and wires them into `specify slice validate` / `plan add` / `plan amend` first-use validation. | #5g                  |
| 2    | D1, D3      | Domain rename `Adapter*` -> `Target*`; `Plan::resolve_sources`.                                                                                                                                                                                                                                                                                                                                                                                                                                                   | #5a                  |
| 3    | D1          | `crates/domain/src/adapter/` axis-aware loader replaces the legacy 1.x adapter loader.                                                                                                                                                                                                                                                                                                                                                                                                                            | #2, #4               |
| 4    | D1, D5      | Ship `adapters/sources/intent/`, `adapters/sources/documentation/`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | #1, #2               |
| 5    | D2, D4      | Core synthesis + `/spec:refine` pipeline; migrate define briefs -> synthesis + `shape`.                                                                                                                                                                                                                                                                                                                                                                                                                           | #5, #5a-#5h          |
| 6    | D4          | `spec.md` provenance parser (`ID:`, `Sources:`, `Status:`).                                                                                                                                                                                                                                                                                                                                                                                                                                                       | #1, #5a-#5c          |
| 7    | D3, D11     | Discovery stable-id replace; `/spec:plan` `propose` sub-step (agent-driven candidate fusion, `tentative: true` annotations + `## Tentative merges` block in `change.md`, `slices[].divergence: likely` on materially-disagreeing summary pairs + `## Likely divergences` block in `change.md` with side-by-side values).                                                                                                                                                                                          | #5e                  |
| 8    | D1, D3, D10 | CLI: `source resolve`, plan amend sources; retire `change survey`, `adapter pipeline`.                                                                                                                                                                                                                                                                                                                                                                                                                            | #3, #4, #7           |
| 9    | D1, D2      | Target brief migration; RFC-24 prose.                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | #5h                  |
| 10   | D1-D10      | Docs: AGENTS.md, project.mdc, decision-log, adapter-anatomy.                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Documentation review |
| 11   | D1          | `discovery.md` three-section form (`Summary`, `Source inventory`, `Candidate inventory`).                                                                                                                                                                                                                                                                                                                                                                                                                         | #4                   |
| 12   | D1-D4, D9   | Adapter-axis acceptance lands before step 16.                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | #1-#5h, #10          |
| 13   | D4          | RFC-19 journal events for extract and synthesis tags.                                                                                                                                                                                                                                                                                                                                                                                                                                                             | #5b-#5d              |
| 14   | D6          | Plan lifecycle is `pending -> reviewed` (two stored states); `plan transition reviewed` is operator-only and is the sole plan-level transition target. `/spec:plan` exits at `pending` with the literal stamp command in its closing hint and never writes `reviewed` itself.                                                                                                                                                                                                                                     | #1-#4                |
| 15   | D7, D9      | `/spec:execute` stop/resume; load-bearing workflow collapse step.                                                                                                                                                                                                                                                                                                                                                                                                                                                 | #8-#11               |
| 16   | D5-D7       | Document default `/spec:plan` -> execute -> finalize; scenario #1 release blocker.                                                                                                                                                                                                                                                                                                                                                                                                                                | #1, #8, #9           |
| 17   | D10         | Delete `/change:*`, `/spec:define`; remove `plugins/change/`.                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Full matrix          |


### Suggested PR train

Not binding; sequence by what unblocks the most tests soonest.

1. `augentic/specify-cli`: schemas, the `Adapter*` → `Target*` rename, the `crates/domain/src/adapter/` axis-aware loader, and CLI verbs (steps 1–3, 6, 8, 13, 14). This unblocks plan/slice/source/target writes for everything downstream.
2. `augentic/specify`: `adapters/sources/`, `adapters/targets/`, `/spec:*` skill bodies, synthesis pipeline, discovery propose, docs (steps 4, 5, 7, 9–11, 16).
3. Cutover: `migrate-to-2.0.sh` plus deletion of `/change:*` and `/spec:define` and removal of `plugins/change/` (steps 15, 17). Lands last so step 1 has time to settle.

## Acceptance scenarios

Run these against the merged skills before implementation step 17. Each row stress-tests a place the redesign can fail.

**Scenario id convention.** Numeric ids (`#1`-`#12`) are independent scenarios. Letter-suffixed ids under a number (`#5`, `#5a`-`#5j`) share a theme — here, single-source and multi-source synthesis behavior. Sub-ids are non-dense: gaps in the `5x` series (`5d`, `5i`) are intentional — former rows were folded into adjacent scenarios during drafting and the ids are preserved so cross-references stay stable across revisions. Implementation-plan acceptance columns and inline cross-references use these ids verbatim.

If any of #1-#4 fail the ergonomics test (operator confusion, lost time, surprised state), revisit §Planning at every scale before pushing through step 17.

**Release blocker:** scenario #1 (pure intent, one slice) must pass before step 16 lands. Single-release collapse means N=1 `/spec:plan` ergonomics surface to every operator at once.


| #   | Decisions  | Scenario                                                                                                                                                                                                         | What it stress-tests                                                                                                                                                                                                                                                                                                                                 |
| --- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | D3, D5, D6 | **Pure intent, one slice.** Operator runs `/spec:plan fix-typo "fix typo in user.rs"`.                                                                                                                           | Degenerate `intent.enumerate`; Gate 1 ergonomics on trivial work; `change.md` + `plan.yaml` justifiability at N=1; `Sources: [intent]` provenance; `/spec:plan` exits at `pending` and prints the literal `specify plan transition fix-typo reviewed` command — the operator runs it, then `/spec:execute`. The skill never auto-stamps `reviewed`.  |
| 2   | D1, D3, D4 | **Documentation, one slice.** Operator binds a single docs path.                                                                                                                                                 | `documentation.enumerate` correctness at the new entry point; `Sources: [<doc-key>]` provenance.                                                                                                                                                                                                                                                     |
| 3   | D3, D5, D6 | **Documentation, multi-slice.** Operator binds docs that map to N candidates.                                                                                                                                    | Propose/edit/reject loop; Gate 1 amendment flow.                                                                                                                                                                                                                                                                                                     |
| 4   | D1, D3     | **code, multi-slice.** Operator binds a legacy repo.                                                                                                                                                             | `code-typescript.enumerate`; enumerate/repair loop under `/spec:plan`; under-slicing failure mode; `Sources: [<legacy-key>]` provenance.                                                                                                                                                                                                             |
| 5   | D2, D4     | **Intra-Evidence `[conflict]`.** Single-source slice where synthesis cannot reconcile contradictory `claims` within one `Evidence` document.                                                                     | `[conflict]` written into `spec.md`; lifecycle still transitions to `refined`; operator can hand-edit and run `/spec:build` without a parking-state ceremony.                                                                                                                                                                                        |
| 5a  | D2-D4      | **Combined evidence (code + documentation), one slice.** Operator binds a legacy repo and a design-notes path on the same slice.                                                                                 | Synthesis end-to-end: serial `extract` per source; two-entry `Evidence[]`; `Sources:` line carrying both keys; `claim-id` correlation produces deterministic fusion; lifecycle reaches `refined` cleanly when sources agree.                                                                                                                         |
| 5b  | D2, D4     | `**[divergence]` from authority resolution.** Combined-evidence slice where docs and legacy code disagree at different authority classes, for example docs say "30 minutes" expiry while code observed 24 hours. | `Status: divergence` written; documentation authority wins as the operative requirement; behaviour preserved as inline commentary; lifecycle transitions to `refined`; operator may hand-edit before build.                                                                                                                                          |
| 5c  | D2, D4     | `**[conflict]` from same-authority disagreement.** Combined-evidence slice where two `documentation` sources disagree on the same claim.                                                                         | `Status: conflict` written with both values preserved as inline commentary; lifecycle still transitions to `refined`; operator must reconcile by editing or amending sources before the requirement is meaningful.                                                                                                                                   |
| 5e  | D3, D11    | **Cross-source propose-time merge.** Two adapters surface the same candidate; the `/spec:plan` agent merges them automatically at `propose`.                                                                     | `specify plan add` writes one slice with combined `sources:` without operator ceremony; uncertain merges annotated `tentative: true` and surfaced in `change.md`; operator overrides via `specify plan amend` at Gate 1 if the merge is wrong; downstream `extract` runs against every contributing source.                                          |
| 5f  | D2, D3     | **Extract failure.** A bound source's `extract` fails.                                                                                                                                                           | Slice stays in `refining`, no synthesis runs, structured error names the source key.                                                                                                                                                                                                                                                                 |
| 5g  | D2, D8     | **Invalid Evidence schema rejection.** Adapter emits `Evidence` failing `evidence.schema.json`.                                                                                                                  | Validation fails before synthesis; structured error; slice stays in `refining`.                                                                                                                                                                                                                                                                      |
| 5h  | D2         | **Target `shape` injection.** Synthesis consumes a non-empty `target.shape` brief.                                                                                                                               | Generated `spec.md` / `design.md` reflect target-idiom guidance; pure-intent fixture vs documentation fixture both pick up the same `shape`.                                                                                                                                                                                                         |
| 5j  | D1, D2     | **Source-adapter sandbox path-denied.** A source adapter's `extract` (or `enumerate`) attempts a read outside its bound `$SOURCE_DIR` / `$CAPABILITY_DIR` / `$SCRATCH_DIR` grants.                               | Host runner denies the access and surfaces structured error `source-extract-path-denied` (or `source-enumerate-path-denied`); slice stays `refining`; no Evidence is written; operator can rebind via `plan amend` or drop the source. WASI preopens are the only grant; lifecycle state (`.specify/project.yaml`, `.metadata.yaml`) is unreachable. |
| 6   | D9         | **Multi-repo assignment from a workspace.** Operator runs `/spec:plan` in a workspace.                                                                                                                           | `workspace:` discriminator; per-candidate `--project` at propose; `workspace sync` timing.                                                                                                                                                                                                                                                           |
| 7   | D3, D6     | **Operator amends one-slice plan into two slices at Gate 1.**                                                                                                                                                    | Plan amendment via `specify plan amend`; re-entry to Gate 1 after amend.                                                                                                                                                                                                                                                                             |
| 8   | D7, D9     | **Step-through breakout mid-execute.** Operator starts `/spec:execute`; on the second slice they cancel, run `/spec:build` directly to investigate, then re-invoke `/spec:execute`.                              | Stop/resume contract; step-through verbs leave on-disk state consistent for `/spec:execute` to resume without flags.                                                                                                                                                                                                                                 |
| 9   | D7         | `**/spec:execute` parks on a build failure, operator fixes, resumes.** Slice's `cargo test` fails; operator patches the crate; runs `/spec:execute`.                                                             | Build-failure stop hint; build resumes from the failed task; loop continues to merge.                                                                                                                                                                                                                                                                |
| 10  | D9         | **Workspace `/spec:execute` across two projects.** Plan with slices targeting `project-a` and `project-b`; operator runs `/spec:execute` from the workspace root.                                                | Per-slice project routing; slot materialisation; `prepare-branch`; `chdir` + residue commit; plan-lock semantics at the workspace root while phase work runs in slots.                                                                                                                                                                               |
| 11  | D7, D9     | **Workspace breakout after build failure in a slot.** `/spec:execute` parks on `auth-rotate` in `project-a`; operator stays at workspace root and runs `/spec:build`.                                            | Project-routing rule for breakout verbs; active-slice resolution across the workspace/slot boundary; correct `chdir` without operator intervention.                                                                                                                                                                                                  |
| 12  | D9         | **Dual-driving refused.** Project registered in a workspace; operator runs `/spec:plan` from the project root with a workspace-driven plan active.                                                               | One-driving-mode-per-project invariant.                                                                                                                                                                                                                                                                                                              |


Adapter-axis scenarios #1-#5h and #10 land by step 12. Workflow-collapse scenarios, especially #1 and #8-#9, gate steps 15-16.

## Migration

Specify 2.0 is a hard cut from 1.x with no interim release. `migrate-to-2.0.sh` renames `project.yaml`, `registry.yaml`, `plan.yaml`, `sources.yaml`, cache, and archive fields; moves skills; bumps `specify-version`; rewrites `plan.yaml.slices[].sources` from any 1.x form into the v2 structured `{ key, candidate }[]` shape (lifting any standalone `slices[].candidate` value into each binding); removes `plugins/vectis/skills/image-layout-inferer/` (its body lifts into `adapters/sources/screenshots/`); retires baseline `layout.yaml` paths and warns when it finds an existing `composition.yaml` (now a build output, regenerated by `adapters/targets/vectis/build` on the first 2.0 `/spec:execute`); and adds `reviewed` on first read. There is no `specify upgrade`; see §CLI surface. Dry-run against a 1.x fixture before tag.

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
- Gate 2, `/spec:execute` automation flags, parallel extract, per-claim authority overrides, multi-target projects, cross-mode workspace+standalone driving; reinstate when a real consumer asks. §CLI surface tracks the deferrals.

## Open questions

## Observability ([RFC-19](rfc-19-observability.md))


| Event                                                   | When                                                               |
| ------------------------------------------------------- | ------------------------------------------------------------------ |
| `plan.transition.reviewed`                              | Gate 1 cleared                                                     |
| `plan.propose.divergence`                               | `propose` sets `slices[].divergence: likely` on a slice            |
| `plan.amend.divergence`                                 | Operator transitions `slices[].divergence` (payload: `from`, `to`) |
| `slice.transition.refined`                              | Synthesis completed                                                |
| `slice.extract.completed`                               | Per source key per slice                                           |
| `slice.synthesis.conflict` / `.divergence` / `.unknown` | Tags written to `spec.md`                                          |


## References

- [RFC-19](rfc-19-observability.md) - journal events.
- [RFC-20 (archived)](archive/rfc-20-survey.md) - survey -> legacy source adapter.
- [RFC-21](rfc-21-catalogue.md) - `sources.yaml`.
- [RFC-22](rfc-22-ledger.md) - target-typed ledger entries.
- [RFC-23 (archived)](archive/rfc-23-change-lifecycle.md) - superseded lifecycle.
- [RFC-24](rfc-24-omnia.md) - target `shape`; Omnia as target adapter.
- [RFC-15 (archived)](archive/rfc-15-wasm-plugins.md) - WASI tools.
- [specify-cli/AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) - exit codes.
- [project.mdc](../.cursor/rules/project.mdc) - artifact authority; synthesis hierarchy in §Synthesis contract.
- [AGENTS.md](../AGENTS.md) - plan-driven loop vocabulary.
- [rfc-25-plan.md](rfc-25-plan.md) - wave decomposition, progress snapshot, outstanding 2.0 cutover work.

