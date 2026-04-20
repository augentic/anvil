# RFC-3: Initiative Planning

> Status: Draft · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-2](archive/rfc-2-execution.md)

## Abstract

Extend RFC-2's `/spec:plan` into a **registry-aware** authoring skill that scales unchanged from a single repo to 100+ repos.

Two new, optional files scope the initiative. `.specify/registry.yaml` is the **platform catalogue** — the repos that comprise the system and the schemas they use. `.specify/initiative.md` is the **operator-authored brief** — intent as prose, seed inputs as YAML frontmatter. The registry answers "what's in the platform?"; the brief answers "what am I trying to do this cycle, and from what material?". Either or both may be absent: a bare `/spec:plan` with CLI-only inputs remains valid, and a single-repo system needs no registry at all.

Internally, `/spec:plan` runs a fixed three-phase flow: *analyse inputs* → *(sync peers)* → *generate plan*. The middle phase runs automatically when `registry.yaml` declares more than one project, and clones every listed repo into `.specify/workspace/<project>/` for read-only inventory. Discovery dispatches every input to `/spec:analyze` — a new skill introduced by this RFC — which branches internally on `kind` (`legacy-code` or `documentation`) and emits a single merged `discovery.md`; any other kind is a hard error. `/spec:extract` (unchanged from RFC-2) moves to `/spec:define` time, where it runs per-change against the scope each change owns. `/spec:plan` remains the single operator-facing entry point — the invocation shape, core loop, and single-writer invariant are unchanged.

There is no pipeline declaration file. RFC-3 does not move `pipeline.plan` anywhere: it eliminates the configuration surface entirely for v1 so the fixed shape is the only shape. `schema.yaml` retains its Layer 1 role (`define/build/merge`); no planning config lives there. Configurability is re-addable in a later RFC without migration (see *Alternatives Considered*).

RFC-3 addresses initiative planning across repos. Cross-repo spec references and contract validation are an execution-time concern that sits downstream of the planning flow introduced here. They are captured as Layer 3 and detailed in a follow-up revision.

In parallel, RFC-3 scales `/spec:plan` *down* into large monoliths by splitting the code-handling pipeline across two skills — a cheap whole-source analysis via `/spec:analyze` at plan-authoring time, and a deep per-slice extraction via `/spec:extract` at `/spec:define` time — and by adding an optional `scope` field to plan entries so each change's define runs against only the files that change owns. See §*Large-Monolith Decomposition*.

## Planning Model Overview

Specify planning model

`/spec:plan` runs a fixed internal flow. For single-repo initiatives the flow is *analyse inputs → generate plan*; for multi-repo initiatives (registry declares more than one project) a *sync peers* phase is inserted between them. The three diagram phases correspond to:

1. **Analyse inputs.** Read seed material — legacy code, documentation — and extract candidate capabilities, constraints, and open questions. Dispatches every input to `/spec:analyze`, which branches internally on the declared `kind` (`legacy-code` → module inventory; `documentation` → capabilities / constraints / open questions). Emits `discovery.md`.
2. **Sync peers.** *(Runs if* `registry.yaml` *declares more than one project.)* Clone every project declared in `registry.yaml` into `.specify/workspace/<project>/` (local, read-only cache), then inventory each repo's existing `.specify/` tree (baseline specs, in-flight plans, schema). Emits `workspace.md`.
3. **Generate plan.** Combine the input inventory (`discovery.md`) and — when present — the peer inventory (`workspace.md`) into the **Plan**: the ordered, dependency-aware list of changes RFC-2 drains with `specify initiative next`. Emits `plan.yaml`.

The `Plan` box in this diagram is the same `Plan` box on the left of RFC-2's execution diagram. Planning produces it; execution consumes it and amends it back.

The flow is fixed: phases run in order, and the *sync peers* phase is present if and only if `registry.yaml` declares more than one project. There is no operator-visible step ID surface, no configuration file, and no auto-insert policy to diagnose. The diagram labels (`discovery`, `workspace`, `propose`) and artifact filenames (`discovery.md`, `workspace.md`) retain those names for continuity; the RFC prose uses the activity-oriented descriptions above.

### Diagram labels → skills and CLI


| Diagram label                    | Phase / skill                            | CLI                                                                         |
| -------------------------------- | ---------------------------------------- | --------------------------------------------------------------------------- |
| `plan` (centre)                  | `/spec:plan` (registry- and brief-aware) | `specify initiative {init, create, amend, validate}` (unchanged from RFC-2) |
| `registry.yaml` (read)           | —                                        | `specify initiative registry {show, validate}` *(TBD)*                      |
| `initiative.md` (read)           | —                                        | `specify initiative brief {init, show}` *(TBD)*                             |
| Step ① — analyse inputs          | Discovery → `/spec:analyze`              | — (phase reads `--from` / `--against` / `--source` + brief inputs)          |
| Step ② — sync peers              | Workspace (CLI-driven)                   | `specify initiative workspace {sync, status}` *(TBD)*                       |
| Step ③ — generate plan           | Propose                                  | `specify initiative create` (per accepted slice; unchanged from RFC-2)      |
| `Inputs` box (legacy code, docs) | —                                        | — (filesystem paths under `--from` / `--source` or `initiative.md:inputs`)  |
| `Workspace` box (cloned repos)   | —                                        | `.specify/workspace/<project>/`                                             |
| `Plan` box (output)              | —                                        | `.specify/plan.yaml` (RFC-2 format, unchanged)                              |


## Motivation

RFC-2 assumes you already know the changes. For three common cases, you don't:

- **Legacy modernisation.** Changes must be *derived* from legacy code and documentation.
- **Greenfield across multiple repos.** Backend, frontend, and shared-types need coordinated changes, but the per-repo plans don't exist yet.
- **Platform initiatives.** A feature like "add OAuth login" must be decomposed across repos before any per-repo loop can run.

RFC-2's Layer 3 `/spec:plan` skill addresses the first case for a single repo. The multi-repo case has no equivalent — no declared scope of "the repos this initiative spans", no shared workspace for cross-repo analysis. The gap isn't a new skill; it's a platform-scoped `registry.yaml` of peer projects (alongside a per-initiative `initiative.md` brief), and a *sync peers* phase that `/spec:plan` runs automatically when the registry declares more than one project.

## Dependency on RFC-1 and RFC-2

- **RFC-1 (CLI):** registry parsing, initiative-brief parsing, clone orchestration, workspace layout, and plan writes all go through `specify` subcommands. No hand-edited plans. Cloning is deterministic CLI work — `/spec:plan` shells out to `specify initiative workspace sync` rather than delegating to a cloning skill.
- **RFC-2 (Plans):** the Plan format is unchanged. The `/spec:plan` skill — its invocation, its core loop, and its single-writer invariant — is unchanged. RFC-3 adds registry and initiative-brief awareness, a fixed internal *sync peers* phase, and a documentation-analysis skill; the operator-facing contract is the same.
- **No planning configuration.** RFC-2 Layer 3 placed `pipeline.plan` inside `schema.yaml`. RFC-3 does not relocate that declaration to a new file — it eliminates it. `/spec:plan`'s internal flow is fixed for v1. `schema.yaml` retains its Layer 1 role (`define/build/merge`). Stack-specific planning helpers (e.g. slice-heuristic prose) may still ship in schema directories and be consumed directly by the *generate plan* phase.

RFC-3 is structured in three layers; each layer describes a feature added to the planning flow (or, for Layer 3, the per-repo execution loop), not a separate skill or invocation path. **Layer 1** makes `/spec:plan` registry-aware and fixes the discovery-dispatch rule for code vs. documentation inputs. **Layer 2** adds the *sync peers* phase. **Layer 3** adds federation at execution time on top of the workspace Layer 2 materialises. A parallel §*Large-Monolith Decomposition* adds the two-skill analyze/extract split (plan-time vs define-time) and the `scope` field; it composes with all three layers but sits outside their progression.

---

## Layer 1: Registry- and brief-aware `/spec:plan`

Layer 1 adds two optional files to `/spec:plan`'s input surface:

- `.specify/registry.yaml` — **platform catalogue** (`projects[]`: the repos that comprise the system).
- `.specify/initiative.md` — **operator-authored brief** (frontmatter `name` + `inputs[]`; body = prose describing intent).

Both absent → the skill behaves exactly as today, running *analyse inputs → generate plan* against the current repo with CLI-supplied inputs only.

### When are `registry.yaml` and `initiative.md` required?

Both files are **optional**. They cover orthogonal concerns:


| File            | Carries                                                                | When required                                                                                                                                              |
| --------------- | ---------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `registry.yaml` | Platform catalogue (`projects[]`).                                     | Only when the platform spans more than one repo. Absent or single-entry → single-repo flow; multi-entry → *sync peers* phase (Layer 2) activates.          |
| `initiative.md` | Current initiative's name + seed inputs (frontmatter) + intent (body). | Whenever inputs would otherwise be supplied entirely via CLI flags, or the operator wants a durable home for the initiative's framing. Optional even then. |


The multi-repo toggle is `**len(projects) > 1`** on `registry.yaml`, not the file's presence. A single-entry registry runs the same flow as no registry at all; `initiative.md`'s presence has no bearing on whether peers are synced.

The RFC-2 readiness gate — "at least one of `--from`, `--against`, or `--source` must be supplied" — widens under Layer 1 to **"...or `initiative.md:inputs` is non-empty."** A bare `/spec:plan <name>` with no CLI inputs but a populated `initiative.md:inputs` is valid; a bare `/spec:plan <name>` with neither is still a hard exit, as today.

### The Registry

```yaml
# .specify/registry.yaml
version: 1
projects:
  - name: traffic
    url: .
    schema: omnia@v1
```

`projects[]` enumerates the repos that comprise the platform. The registry exists to answer the platform question ("what repos are in scope, and what's already specified in them?"); it is deliberately free of initiative-specific state — no `name`, no `inputs`, no cycle-scoped fields. This keeps the file stable across initiatives and makes a later move to a shared registry (§*Alternatives Considered — Registry repo*) a mechanical lift rather than a schema rethink.

### The Initiative Brief

```markdown
---
# .specify/initiative.md
name: traffic-modernisation
inputs:
  - path: ./inputs/legacy-traffic/
    kind: legacy-code
  - path: ./inputs/ops-runbook.pdf
    kind: documentation
---

# Traffic modernisation

Move the legacy traffic system onto Omnia, preserving the ops-runbook's
escalation paths and the existing Kafka ingress contract. Priority is parity
over feature work; the big open questions are around state migration.
```

`initiative.md` is the operator-authored starting point for one planning cycle. Frontmatter carries the structured bits the CLI and `/spec:analyze` need (`name`, `inputs[]`); the body carries the operator's framing for the agent dialogue `/spec:plan` drives.

The `inputs[]` list and its closed `kind` vocabulary are unchanged from the earlier draft — only their home has moved. The body prose is **not** an input to `/spec:analyze` in v1; if it later becomes useful to feed in, the closed `kind` vocabulary forces that addition through a schema + RFC update (e.g. a new `kind: initiative-brief`) rather than a silent behaviour change.

`specify initiative brief init <name>` scaffolds `initiative.md` from a template at the start of a cycle; `specify initiative archive` sweeps it into the archive alongside `plan.yaml` when the cycle ends. Neither command is operator-facing beyond that scaffolding — once `initiative.md` exists, it's a plain-text file the operator edits directly.

### Discovery dispatch

The *analyse inputs* phase routes each input to a skill based on its declared `kind`:


| `kind`          | Dispatch target | Purpose                                                                                         |
| --------------- | --------------- | ----------------------------------------------------------------------------------------------- |
| `legacy-code`   | `/spec:analyze` | Produce a module-level inventory (entry points, dependencies, candidate capability hints).      |
| `documentation` | `/spec:analyze` | Extract capabilities, constraints, and open questions from prose, PDFs, runbooks, and API docs. |


The `kind` vocabulary is a **closed enum** for v1. An input with any other `kind` is a hard error at the *analyse inputs* phase; extending the vocabulary requires a new skill and an RFC update. This keeps discovery auditable ("which skill produced this line of `discovery.md`?") and forces any future ambiguity in source material to be resolved at the RFC level rather than by a generic fallback.

`/spec:analyze` is **new in RFC-3** and is the sole plan-time discovery skill. It accepts both code and documentation inputs, branches internally on `kind`, and contributes a merged inventory to `discovery.md`. For code inputs the output is module-level (entry points, dependencies, candidate capability hints) — output size scales with module count, not LOC, so it handles monoliths without overflow. For documentation inputs the output is capabilities, constraints, and open questions. Both branches share one output artifact (`discovery.md`), one idempotency contract, and one fixture tree; per-kind prompts and merge rules are deferred to a follow-up skill RFC. RFC-3 fixes only the skill boundary (one plan-time skill, one artifact) and the closed `kind` vocabulary.

`/spec:extract` (RFC-2 Layer 3, unchanged) moves to `/spec:define` time. It produces `specs/<change>/` + `design.md` for a single change, scoped by the change's `scope` field. See §*Large-Monolith Decomposition* for the plan-time / define-time split.

### `--source` flags and the brief

`/spec:plan`'s existing `--source <key>=<path-or-url>` / `--from` / `--against` flags continue to work unchanged. They're additive with `initiative.md:inputs` — the *analyse inputs* phase reads both and merges them into `discovery.md`. Inputs supplied via CLI flags carry an explicit kind (e.g. `--source legacy=./old/:legacy-code`) so they route through the same closed-enum dispatch as brief inputs.

### The flow

```
analyse inputs ──▶ generate plan ──▶ plan.yaml
```

Unchanged from today. The *sync peers* phase is absent because `registry.yaml` either is absent or declares a single project.

---

## Large-Monolith Decomposition

A deep extraction over a whole monolith via `/spec:extract` is intractable: for 100k+ LOC with dozens of modules, the output overflows context, the resulting specs are unwieldy, and the human has no chance to draw slice boundaries before the extractor commits to them. RFC-3 solves this by keeping plan-time discovery cheap (`/spec:analyze` emits a module inventory, not specs) and deferring deep extraction (`/spec:extract`) to `/spec:define` time, where it runs per-slice against the scope each change owns.

This section is orthogonal to the Layer 1 / 2 / 3 progression. It extends Layer 1's discovery dispatch and the Plan schema, composes with Layer 2's *sync peers* phase (per-peer decomposition works identically), and does not interact with Layer 3. It introduces no new operator-facing skill: `/spec:plan` remains the sole human entry point.

### Plan-time analysis, define-time extraction

The monolith case is handled by splitting plan-time and define-time across two different skills:

1. `**/spec:analyze` (plan-time, whole monolith).** Reads the whole code tree and emits a module-level inventory — entry points, outbound dependencies, import-graph edges between modules, candidate capability hints derived from docstrings, endpoint names, and READMEs — not full specs. Output size grows with module count, not LOC. The code-branch output contract (artifact shape, idempotency, header rules) matches the documentation-branch contract so both feed `discovery.md` in the same shape, and the propose brief does not care which kind of input produced a given section.
2. `**/spec:extract` (define-time, one slice).** Runs per-change under `/spec:execute`, scoped to the files the change owns via its `scope` field. Produces `specs/<change>/` + `design.md` for that slice only. No single invocation ever sees more than one slice's worth of source.

The split is on the skill boundary, not a mode flag: the discovery brief under `/spec:plan` calls `/spec:analyze` on every input; the define pipeline under `/spec:execute` calls `/spec:extract` on the current change's scope. Neither skill has a mode flag that changes its output shape — `/spec:analyze` always emits `discovery.md`, `/spec:extract` always emits `specs/` + `design.md`. The detailed code-branch algorithm inside `/spec:analyze` — module-detection rules, entry-point discovery, hint derivation — is deferred to a follow-up skill RFC; RFC-3 fixes only the skill boundary and the output contract.

### The `scope` field

A new optional field on plan change entries, parallel to `sources`:

```yaml
changes:
  - name: ingest-pipeline
    sources: [monolith]
    scope:
      monolith:
        include:
          - src/ingest/**
          - src/kafka/**
        exclude:
          - src/ingest/_deprecated/**
    status: pending
```

- `scope.<source>.{include,exclude}` are glob lists resolved relative to the top-level `sources[<source>]` path.
- A change with no `scope` behaves as today — `/spec:define` hands the whole source to `/spec:extract`. Backwards compatible.
- A change with a `scope` hands only the filtered subset to `/spec:extract`.

The field lands as a structural extension to `plan.schema.json` and to the RFC-1 Plan library types (an additional `BTreeMap<String, Scope>` on `PlanChange`). No semantic changes to `sources`, `depends-on`, or `affects`; no change to the status state machine.

**Manifest escape hatch.** For tangled monoliths where globs cannot cleanly separate slices, `scope.<source>.manifest` points at a per-slice manifest under `.specify/plans/<initiative>/slices/<change>.md`:

```yaml
    scope:
      monolith:
        manifest: .specify/plans/<initiative>/slices/ingest-pipeline.md
```

Manifest shape (files, optional line-range subsets) is TBD and ships empty in v1. Path-based `include`/`exclude` is the 95% case; the manifest pointer exists so the RFC does not paint itself into a corner on tangled layouts.

### Validation

`specify initiative validate` (RFC-1) gains three scope-aware checks:

- **Path existence** — every glob root in `include` must resolve under `sources[<key>]`. *Error.*
- **Overlap warning** — a file claimed by more than one change's scope is flagged. Not an error; shared code is a real case (see *Alternatives Considered*). *Warning.*
- **Orphan warning** — a file under any `sources[<key>]` claimed by zero changes is flagged. Useful as a "did I cover everything?" audit before merging. *Warning.*

### Propose-brief slicing heuristics

The Omnia propose brief today decomposes `discovery.md` under the heuristic "one WASM crate per change". Monolith decomposition adds a slicing pass in front of that heuristic. The primary rule is **entry-point clustering** — group modules by which entry point they transitively reach. Tie-breakers are directory cohesion, external-dependency clustering, and a LOC / complexity budget that forces over-size candidates to split and under-size candidates to merge into a neighbour.

The brief presents candidate slices — each with include paths, `depends-on` edges, and a capability-hint summary — to the human via the existing accept / edit / reject / abort loop. Edit may move paths between slices before the slice is committed via `specify initiative create --scope-include ... --scope-exclude ...`.

The algorithm is schema-owned; other schemas ship their own slicing rules. RFC-3 fixes only the contract: take `discovery.md`, produce slices with `scope` pre-filled.

### Working-directory additions

Extends the Layer 2 layout:

```
.specify/plans/<initiative>/
├── discovery.md        # /spec:analyze output (cheap, whole-monolith)
├── workspace.md        # Layer 2 only
├── proposal.md         # slice decisions
└── slices/             # NEW — one manifest per tangled slice (optional)
    └── ingest-pipeline.md
```

Archived with the rest by `specify initiative archive`; no code change required.

### Cross-slice shared files

A file legitimately used by multiple slices (e.g. `src/common/validation.ts`) is represented by an explicit **shared-infrastructure change** that all consuming slices list in `depends-on`:

```yaml
changes:
  - name: shared-validation
    sources: [monolith]
    scope:
      monolith:
        include: [src/common/validation/**]
    status: pending

  - name: ingest-pipeline
    sources: [monolith]
    depends-on: [shared-validation]
    scope:
      monolith:
        include: [src/ingest/**]
    status: pending
```

This keeps every file owned by exactly one slice, preserves the single-pass-per-slice property of extraction, and makes the dependency explicit in the plan topology — every consumer listing the shared slice in `depends-on` makes the coupling visible to the human reading the plan. When a shared-change is overkill, the overlap *warning* in `specify initiative validate` permits the overlap and duplicate extraction is accepted as the cost. A future `read-only-in-scope` designation — letting multiple slices read a file without re-extracting it — is deferred.

### Staged rollout

RFC-3 lands monolith decomposition in three stages; each stage is independently useful and independently landable.

**Stage A — the `scope` field.** Add `scope.<source>.{include,exclude}` to `plan.yaml`; plumb through `/spec:execute` → `/spec:define` → `/spec:extract`; extend `specify initiative validate`. Humans author scopes by hand. Unblocks real monolith work without touching discovery or propose.

**Stage B — `/spec:analyze` code branch.** Ship the code-reading branch of `/spec:analyze` (Stage A can rely on the documentation branch and on hand-authored scopes; monolith authoring is unblocked once the code branch lands). Update the discovery brief to call `/spec:analyze` for `legacy-code` inputs. `discovery.md` now scales to monoliths; propose still emits scope-less slices that humans scope after the fact.

**Stage C — automatic slicing in propose.** Schema-specific slicing heuristics pre-fill `scope` on proposed slices; humans tune via the edit path. Manifest escape hatch for tangled cases.

Stages compose with RFC-3's other features: Layer 2's *sync peers* phase runs unchanged (peer inventory is orthogonal to within-peer decomposition); Layer 3's federation is unchanged (cross-repo reference resolution is orthogonal to slice scope).

### Non-goals

- **Symbol-level scope** (function / class granularity). File-level is sufficient for real monoliths; the parse-and-index infrastructure required to resolve symbols across languages is large and language-specific. Deferred indefinitely; the manifest pointer provides file-level granularity for tangled cases without requiring a cross-language symbol index.
- **Auto-sub-slicing at define time.** If a slice proves too big during `/spec:define`, the existing amend edge (RFC-2) lets the define phase call `specify initiative create` to split off a neighbour; RFC-3 only confirms this composes with `scope` — the parent change's scope is reduced via `specify initiative amend`, and the new change takes the removed paths. No new driver-level sub-slicing is introduced.
- **Replacing `/spec:extract`.** `/spec:analyze` is a plan-time complement, not a replacement. Deep extraction remains the tool that produces `specs/`, now scoped per-slice at `/spec:define` time.

---

## Layer 2: Multi-repo planning with the workspace

Layer 2 is the case the diagram describes: `registry.yaml` declares more than one repo, the *sync peers* phase clones them and inventories their specs, and *generate plan* consumes both the input inventory and the peer inventory.

A `registry.yaml` with `len(projects) > 1` is **required** in this layer — the *sync peers* phase has nothing to do without peers to inventory, and is only present in the flow when the registry declares them. A single-repo run (no registry, or a registry with a single project) never reaches this requirement. `initiative.md` is orthogonal: it carries the cycle's name and inputs in Layer 2 exactly as it does in Layer 1.

### The Registry (multi-project)

```yaml
# .specify/registry.yaml
version: 1
projects:
  - name: traffic
    url: git@github.com:org/traffic.git
    schema: omnia@v1

  - name: command-centre
    url: git@github.com:org/command-centre.git
    schema: omnia@v1
```

The accompanying `initiative.md` is identical in shape to the Layer 1 example — it declares the initiative's `name` and `inputs[]` in frontmatter, and the operator's framing in the body. Peer projects are declared in `registry.yaml`; seed material for this cycle is declared in `initiative.md`. The two files never overlap.

### The flow

```
analyse inputs ──▶ sync peers ──▶ generate plan ──▶ plan.yaml
```

Nothing about the `/spec:plan` invocation changes — the skill runs the fixed flow, and `len(projects) > 1` on `registry.yaml` decides whether the *sync peers* phase is included.

### The sync-peers phase

Executed between *analyse inputs* and *generate plan*:

1. **Sync.** Shells out to `specify initiative workspace sync` to clone or fetch every `projects[]` entry into `.specify/workspace/<project-name>/` (local, read-only cache). Local projects (`url: .` or relative paths) are symlinked, not cloned. Cloning is a deterministic CLI operation; no cloning skill is involved.
2. **Inventory.** Walks each peer's `.specify/` tree — baseline specs, active plans, schema — and emits `.specify/plans/<initiative-name>/workspace.md` (a peer-by-peer summary of what's already specified and what's in flight).

`workspace.md` becomes a new input to *generate plan*, alongside `discovery.md`. No writes ever land in peer clones during planning.

### The workspace layout

```
.specify/
  registry.yaml         # required when len(projects) > 1
  initiative.md         # operator-authored brief (optional in Layer 1, typical in Layer 2)
  workspace/
    traffic/            # clone of org/traffic (read-only)
    command-centre/     # clone of org/command-centre
  plans/
    <initiative-name>/
      discovery.md      # from analyse inputs (Layer 1)
      workspace.md      # from sync peers (Layer 2, NEW)
      proposal.md       # from generate plan (Layer 1)
  plan.yaml             # authored plan (RFC-2 format)
```

`.specify/workspace/` is `.gitignore`d by default and rebuilt by `specify initiative workspace sync`.

### Plan output shape

The *generate plan* phase emits a **single cross-repo `plan.yaml*`* in the initiating repo. Entries whose work spans peer projects reference them by registry name in `sources` / `affects`; execution of those entries requires Layer 3.

Per-repo `plan.yaml`s linked by a feature manifest — staged under `.specify/plans/<initiative-name>/<peer>/` and delivered out-of-band — is a plausible alternative output shape, but it is deferred (see *Alternatives Considered*). RFC-3 ships with exactly one shape.

### CLI surface additions


| Operation                        | CLI                                                    |
| -------------------------------- | ------------------------------------------------------ |
| Scaffold / show initiative brief | `specify initiative brief {init, show}` *(TBD)*        |
| Read / verify registry           | `specify initiative registry {show, validate}` *(TBD)* |
| Clone / refresh workspace        | `specify initiative workspace sync` *(TBD)*            |
| Inspect workspace state          | `specify initiative workspace status` *(TBD)*          |


These are all machinery the *sync peers* / *generate plan* phases shell out to; none are operator-facing entry points. `/spec:plan` remains the only command humans invoke.

### `--dry-run` and `--extend` under Layer 2

- `**--dry-run`.** The *sync peers* phase's read side may run (inventory whatever is already cloned) but MUST NOT clone new repos, write to `.specify/workspace/`, or write `workspace.md`. Mirrors the *analyse inputs* dry-run rule.
- `**--extend`.** When `.specify/workspace/` is already present, the *sync peers* phase may skip re-sync; `workspace.md` is regenerated from the existing cache. Refresh-on-`--extend` policy is *(TBD)*.

The single-writer invariant is unaffected: *sync peers* writes `workspace.md` under `.specify/plans/<name>/` and clones into `.specify/workspace/`; neither path touches `.specify/plan.yaml`.

---

## Layer 3: Federation at Execution Time

Layer 3 is the smallest possible addition to RFC-2's per-repo execution loop once the workspace exists:

- **Cross-repo spec references.** `@peer:capability` syntax in spec bodies. The CLI resolves against `.specify/workspace/<peer>/specs/`.
- **Contract reconciliation.** `specify federation validate` compares provider / consumer contracts declared across repos and flags mismatches across the workspace.
- **Peer status aggregation.** Read-only roll-up of peer change statuses into the initiating repo.

Layer 3 reads the same `.specify/workspace/` that Layer 2 materialises, so no new cloning, config, or peer discovery is required.

*(Detail TBD — ported from the federation draft.)*

---

## Relation to RFC-2

- RFC-2's `/spec:plan` skill is unchanged in invocation, core loop, working directory, and single-writer invariant. RFC-3's contribution is registry and initiative-brief awareness, a fixed internal *sync peers* phase that runs automatically when `registry.yaml` declares more than one project, and a new `/spec:analyze` skill that owns plan-time discovery for both code and documentation inputs; the skill picks up all three automatically. `/spec:extract` is otherwise unchanged from RFC-2 but moves from plan-time to `/spec:define` time, where it runs per-change against the change's `scope` field.
- RFC-2 Layer 3 placed `pipeline.plan` inside `schema.yaml`. RFC-3 does not relocate that declaration to any new file — it removes the configuration surface. `schema.yaml` returns to its Layer 1 role (per-repo `define/build/merge`). Stack-specific planning helpers may still ship in schema directories and be consumed directly by the *generate plan* phase; that is a separate concern from a pipeline-shape declaration, which no longer exists.
- The Plan format gains one optional field: `scope` on change entries (`scope.<source>.{include,exclude}` globs, or `manifest` pointer) for large-monolith slicing. Changes without `scope` are unaffected and behave exactly as in RFC-2. RFC-3's only other semantic extension on the Plan itself is that `sources` and `affects` may reference peer projects by registry name (resolved via `registry.yaml`). See §*Large-Monolith Decomposition*.
- The `amend` edge in RFC-2's execution diagram — a phase discovering a neighbouring change and calling `specify initiative amend` — continues to work identically on RFC-3-produced plans.
- `specify initiative archive` sweeps `.specify/plans/<name>/` alongside `plan.yaml` as today; the new `workspace.md` artifact and `initiative.md` brief are archived with the rest, no code change required.

## Alternatives Considered

### Combined registry + initiative file

An earlier draft put `projects[]` and `inputs[]` in the same `.specify/registry.yaml`, keyed by an initiative `name:` at the top. Rejected: the file conflated three independent concerns — platform catalogue (long-lived, slow-changing), initiative identity (ephemeral), initiative seed material (ephemeral) — and forced one presence/absence signal to double as both "multi-repo mode" and "this initiative has inputs". The split lets `registry.yaml` describe the platform honestly (what repos exist, what they're called, what schemas they use), `initiative.md` describe one cycle's intent as prose alongside its structured seed inputs, and the multi-repo toggle become what it actually is: `len(projects) > 1`. It also gives operator intent a first-class home (the prose body of `initiative.md`), where previously it tended to be lost in commit messages or reinvented in `proposal.md`. The split is additionally forward-compatible with a later move to a shared registry repo (§*Registry repo*, below): only `registry.yaml` relocates; `initiative.md` stays with whichever repo drives the cycle.

### Per-initiative brief files (`initiatives/<name>.md`)

An alternative shape was `.specify/initiatives/<initiative-name>.md`, mirroring RFC-2's `changes/<change>/` pattern and permitting multiple briefs in flight simultaneously. Rejected for v1: RFC-2 already assumes one active plan per repo (single `plan.yaml`, single `.specify/plans/<name>/` working dir), and a singular `initiative.md` co-located with `plan.yaml` matches that shape. If multi-initiative-in-flight becomes a real need, promoting `initiative.md` → `initiatives/<name>/brief.md` is a mechanical lift, not a schema rethink. YAGNI.

### Configurable planning pipeline (`planning.yaml`)

An earlier draft introduced `.specify/planning.yaml` declaring the authoring pipeline (`discovery → [workspace?] → propose`) with a CLI-provided default keyed on `registry.yaml` presence. RFC-3 v1 ships without that file. The configurability it enabled — custom step order, alternative *generate plan* implementations, pinning against auto-insert policy changes — is not needed for the initiatives RFC-3 is designed to serve, and the diagnostic surface it required (`planning show`, "resolved pipeline and its source") is real ongoing cost. The fixed flow has one shape, one set of phase names, and no "which pipeline is in effect?" question. If experience later reveals a need for alternative pipelines, `planning.yaml` can be reintroduced in a follow-up RFC with default = this RFC's fixed flow and no migration for existing repos, because nothing in v1 records the flow explicitly anywhere.

### Registry repo

A separate dedicated registry repo creates a coordination bottleneck. Every change requires commits to the registry, and the registry becomes a merge-conflict magnet. The chosen model keeps `registry.yaml` in whichever repo initiates the initiative (typically a dedicated platform / coordination repo), with peers autonomous. If you later need a central dashboard or CI check, you can build it on top of RFC-3 artifacts without requiring a separate write path. The registry/brief split makes this move cheaper when it does come: only `registry.yaml` would relocate (it is already initiative-free); `initiative.md` stays with whichever repo drives the cycle.

### Cross-organisation coordination

If you're coordinating across *organisations* (not just repos), a registry repo makes more sense because you can't assume write access to peer repos. In that case, the registry holds the change manifests and peer spec snapshots, and the CLI treats them as read-only. Start with the in-initiator model for the single-organisation case.

### Plan-per-repo + feature manifest

RFC-3 emits a single cross-repo `plan.yaml` in the initiating repo. An alternative shape — each peer gets its own `plan.yaml` staged under `.specify/plans/<initiative-name>/<peer>/` in the initiator, linked by a top-level feature manifest tracking aggregate status — is plausible and probably the right long-term direction for peer autonomy and independent delivery. It is deferred because it adds three unsettled design problems at once (manifest format, cross-repo delivery mechanism, aggregate status tracking) and none of them are blocking for getting the single-plan case working. RFC-3 picks the smaller shape; a later RFC can add the manifest-linked shape without changing the Layer 1 / Layer 2 contract.

### Open `kind` vocabulary

An input `kind` not in the closed enum (`legacy-code`, `documentation`) is a hard error in v1. Warn-and-skip and "treat unknown as generic text" were both rejected. Warn-and-skip hides real configuration bugs behind a log line; a generic fallback pushes skill selection into the input author's head ("will my OpenAPI spec get extracted or analysed?") and makes `discovery.md` provenance opaque. The closed enum keeps the skill-selection contract explicit: extending it requires adding a skill and an RFC update, which is the right forcing function.

### `/rt:git-cloner` as the cloning delegate

The *sync peers* phase could delegate cloning to the existing `/rt:git-cloner` skill. RFC-3 does not: cloning is deterministic work with no judgment in it, which per RFC-1 belongs in the CLI (`specify initiative workspace sync`). Routing it through a skill would also couple a core Specify flow to the `rt` plugin, which is the wrong layering.

### `/spec:scope` as a separate skill

An earlier draft introduced `/spec:scope` as a dedicated code-only discovery skill, distinct from both `/spec:extract` and `/spec:analyze`. Rejected: it would fragment plan-time discovery across two skills (one for code, one for documentation) that emit the same artifact (`discovery.md`) and are invoked at the same phase. Folding both modalities into a single `/spec:analyze` with an internal kind branch gives one plan-time skill, one output contract, and one fixture tree. If the code and documentation branches ever diverge enough to justify splitting, the skill can be split later without changing the Plan schema, the discovery-dispatch vocabulary, or the propose-brief contract.

### `--mode=survey` flag on `/spec:extract`

An earlier draft gave `/spec:extract` a `--mode=survey` flag that produced discovery-shaped output instead of spec-shaped output. Rejected: it overloads one skill with two output contracts (specs + design in default mode, module inventory in survey mode), and it leaves the documentation-input case with a different skill, so the intended "one skill covering both tiers" simplification does not actually materialise. RFC-3 places the plan-time / define-time split on the skill boundary — `/spec:analyze` at plan time, `/spec:extract` at define time — rather than on a mode flag. The two skills have non-overlapping output artifacts (`discovery.md` vs `specs/` + `design.md`), which keeps each skill's contract sharp and avoids the category error of an "extract" that does not extract.

### `/spec:decompose` as the plan-time code-reading skill

An alternative naming for the plan-time code skill — `/spec:decompose` — was considered on the grounds that monolith breakdown is the skill's purpose. Rejected: the skill *inventories* code, it does not decompose it (slice proposal lives in the schema-owned propose brief). Naming it after the downstream outcome rather than its own output creates a readership trap where operators expect slice boundaries in `discovery.md` and are surprised to find a flat capability list. `/spec:analyze` names what the skill does; slice boundaries remain the propose brief's job.

### Symbol-level scope

A slice manifest could enumerate individual functions or classes, letting a slice claim "just these symbols from `main.ts`". Rejected for v1: the parse-and-index infrastructure required to resolve symbols across languages is large, language-specific, and likely to drift from whatever the extractor already does internally. File-level scope covers the overwhelming majority of real monoliths; tangled cases use the manifest pointer as a file-level escape hatch. A later RFC may revisit symbol-level if real data shows file-level is too coarse.

### Scope overlap as the default for shared files

An alternative to the dedicated shared-infrastructure change is to permit scope overlap and re-extract shared files in every consuming slice. RFC-3 permits this (as a *warning*, not an error) but prefers the explicit shared-change pattern because it surfaces the dependency in the plan topology — every consumer listing the shared slice in `depends-on` makes the coupling visible to the human reading the plan. A future `read-only-in-scope` designation that lets multiple slices read a file without re-extracting it is deferred.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) — CLI surface this RFC extends.
- [RFC-2: Execution](archive/rfc-2-execution.md) — consumer of the Plan this RFC produces; introduces the `/spec:plan` skill RFC-3 extends.

