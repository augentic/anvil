# RFC-3: Initiative Planning

> Status: Draft · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-2](archive/rfc-2-execution.md)

## Abstract

Extend RFC-2's `/spec:plan` into a **registry-aware** authoring skill that scales unchanged from a single repo to 100+ repos.

One new, optional file scopes the initiative: `.specify/registry.yaml` declares the projects in scope and the seed inputs to analyse. Its absence is the signal for "single-repo system" and requires no operator action.

Internally, `/spec:plan` runs a fixed three-phase flow: *analyse inputs* → *(sync peers)* → *generate plan*. The middle phase runs automatically when `registry.yaml` is present and clones every listed repo into `.specify/workspace/<project>/` for read-only inventory. Discovery dispatches to `/spec:extract` for `kind: legacy-code` inputs and to `/spec:analyze` (a new skill introduced by this RFC) for `kind: documentation` inputs; any other kind is a hard error. `/spec:plan` remains the single operator-facing entry point — the invocation shape, core loop, and single-writer invariant are unchanged.

There is no pipeline declaration file. RFC-3 does not move `pipeline.plan` anywhere: it eliminates the configuration surface entirely for v1 so the fixed shape is the only shape. `schema.yaml` retains its Layer 1 role (`define/build/merge`); no planning config lives there. Configurability is re-addable in a later RFC without migration (see *Alternatives Considered*).

RFC-3 addresses initiative planning across repos. Cross-repo spec references and contract validation are an execution-time concern that sits downstream of the planning flow introduced here. They are captured as Layer 3 and detailed in a follow-up revision.

## Planning Model Overview

Specify planning model

`/spec:plan` runs a fixed internal flow. For single-repo initiatives the flow is *analyse inputs → generate plan*; for multi-repo initiatives (registry present) a *sync peers* phase is inserted between them. The three diagram phases correspond to:

1. **Analyse inputs.** Read seed material — legacy code, documentation — and extract candidate capabilities, constraints, and open questions. Dispatches to `/spec:extract` for code and `/spec:analyze` for documentation based on each input's declared `kind`. Emits `discovery.md`.
2. **Sync peers.** *(Runs iff `registry.yaml` is present.)* Clone every project declared in `registry.yaml` into `.specify/workspace/<project>/` (local, read-only cache), then inventory each repo's existing `.specify/` tree (baseline specs, in-flight plans, schema). Emits `workspace.md`.
3. **Generate plan.** Combine the input inventory (`discovery.md`) and — when present — the peer inventory (`workspace.md`) into the **Plan**: the ordered, dependency-aware list of changes RFC-2 drains with `specify initiative next`. Emits `plan.yaml`.

The `Plan` box in this diagram is the same `Plan` box on the left of RFC-2's execution diagram. Planning produces it; execution consumes it and amends it back.

The flow is fixed: phases run in order, and the *sync peers* phase is present if and only if `registry.yaml` is. There is no operator-visible step ID surface, no configuration file, and no auto-insert policy to diagnose. The diagram labels (`discovery`, `workspace`, `propose`) and artifact filenames (`discovery.md`, `workspace.md`) retain those names for continuity; the RFC prose uses the activity-oriented descriptions above.

### Diagram labels → skills and CLI


| Diagram label                     | Phase / skill                                 | CLI                                                                         |
| --------------------------------- | --------------------------------------------- | --------------------------------------------------------------------------- |
| `plan` (centre)                   | `/spec:plan` (registry-aware)                 | `specify initiative {init, create, amend, validate}` (unchanged from RFC-2) |
| `registry.yaml` (read)            | —                                             | `specify initiative registry {show, validate}` *(TBD)*                      |
| Step ① — analyse inputs           | Discovery → `/spec:extract`, `/spec:analyze`  | — (phase reads `--from` / `--against` / `--source` + registry inputs)       |
| Step ② — sync peers               | Workspace (CLI-driven)                        | `specify initiative workspace {sync, status}` *(TBD)*                       |
| Step ③ — generate plan            | Propose                                       | `specify initiative create` (per accepted slice; unchanged from RFC-2)      |
| `Inputs` box (legacy code, docs)  | —                                             | — (filesystem paths under `--from` / `--source` or `registry.yaml:inputs`)  |
| `Workspace` box (cloned repos)    | —                                             | `.specify/workspace/<project>/`                                             |
| `Plan` box (output)               | —                                             | `.specify/plan.yaml` (RFC-2 format, unchanged)                              |


## Motivation

RFC-2 assumes you already know the changes. For three common cases, you don't:

- **Legacy modernisation.** Changes must be *derived* from legacy code and documentation.
- **Greenfield across multiple repos.** Backend, frontend, and shared-types need coordinated changes, but the per-repo plans don't exist yet.
- **Platform initiatives.** A feature like "add OAuth login" must be decomposed across repos before any per-repo loop can run.

RFC-2's Layer 3 `/spec:plan` skill addresses the first case for a single repo. The multi-repo case has no equivalent — no declared scope of "the repos this initiative spans", no shared workspace for cross-repo analysis. The gap isn't a new skill; it's an initiative-scoped `registry.yaml` and a *sync peers* phase that `/spec:plan` runs automatically when the registry is present.

## Dependency on RFC-1 and RFC-2

- **RFC-1 (CLI):** registry parsing, clone orchestration, workspace layout, and plan writes all go through `specify` subcommands. No hand-edited plans. Cloning is deterministic CLI work — `/spec:plan` shells out to `specify initiative workspace sync` rather than delegating to a cloning skill.
- **RFC-2 (Plans):** the Plan format is unchanged. The `/spec:plan` skill — its invocation, its core loop, and its single-writer invariant — is unchanged. RFC-3 adds registry awareness, a fixed internal *sync peers* phase, and a documentation-analysis skill; the operator-facing contract is the same.
- **No planning configuration.** RFC-2 Layer 3 placed `pipeline.plan` inside `schema.yaml`. RFC-3 does not relocate that declaration to a new file — it eliminates it. `/spec:plan`'s internal flow is fixed for v1. `schema.yaml` retains its Layer 1 role (`define/build/merge`). Stack-specific planning helpers (e.g. slice-heuristic prose) may still ship in schema directories and be consumed directly by the *generate plan* phase.

RFC-3 is structured in three layers; each layer describes a feature added to the planning flow (or, for Layer 3, the per-repo execution loop), not a separate skill or invocation path. **Layer 1** makes `/spec:plan` registry-aware and fixes the discovery-dispatch rule for code vs. documentation inputs. **Layer 2** adds the *sync peers* phase. **Layer 3** adds federation at execution time on top of the workspace Layer 2 materialises.

---

## Layer 1: Registry-aware `/spec:plan`

Layer 1 adds one optional file to `/spec:plan`'s input surface: `.specify/registry.yaml` (initiative scope). Absent → the skill behaves exactly as today, running *analyse inputs → generate plan* against the current repo with no remote peers.

### When is `registry.yaml` required?

`registry.yaml` is **optional**. Its absence is the canonical signal for a single-repo system; no operator action — no file, no flag, no template — is needed to opt out of multi-repo planning.


| Situation       | `registry.yaml`                                                                                   |
| --------------- | ------------------------------------------------------------------------------------------------- |
| Single repo     | **Absent.** `/spec:plan` treats the current repo as the one in-scope project implicitly.         |
| Multiple repos  | **Required.** The *sync peers* phase (Layer 2) reads it to clone peers; absence is a hard error. |


A single-repo degenerate form (`registry.yaml` present with one `projects[]` entry, `url: .`) is supported but redundant; its only practical use is pinning `inputs` in-tree.

The RFC-2 readiness gate — "at least one of `--from`, `--against`, or `--source` must be supplied" — widens under Layer 1 to **"...or `registry.yaml:inputs` is non-empty."** A bare `/spec:plan <name>` with no CLI inputs but a populated `registry.yaml:inputs` is valid; a bare `/spec:plan <name>` with neither is still a hard exit, as today.

### The Registry

```yaml
# .specify/registry.yaml
name: traffic-modernisation
version: 1

projects:
  - name: traffic
    url: .                # Layer 1 degenerate case: the only project is this repo
    schema: omnia@v1

inputs:
  - path: ./inputs/legacy-traffic/
    kind: legacy-code
  - path: ./inputs/ops-runbook.pdf
    kind: documentation
```

`projects` enumerates the repos in scope. `inputs` enumerates seed material the *analyse inputs* phase dispatches on. Both are optional.

### Discovery dispatch

The *analyse inputs* phase routes each input to a skill based on its declared `kind`:


| `kind`          | Dispatch target  | Purpose                                                              |
| --------------- | ---------------- | -------------------------------------------------------------------- |
| `legacy-code`   | `/spec:extract`  | Reconstruct language-agnostic specs + design from existing source.   |
| `documentation` | `/spec:analyze`  | Extract capabilities, constraints, and open questions from docs.     |


The `kind` vocabulary is a **closed enum** for v1. An input with any other `kind` is a hard error at the *analyse inputs* phase; extending the vocabulary requires a new skill and an RFC update. This keeps discovery auditable ("which skill produced this line of `discovery.md`?") and forces any future ambiguity in source material to be resolved at the RFC level rather than by a generic fallback.

`/spec:extract` already exists (RFC-2 Layer 3). `/spec:analyze` is **new in RFC-3**; its detailed design — prompts, artifact format, relationship to `discovery.md` — is deferred to a follow-up skill RFC. RFC-3 fixes only the contract: read documentation inputs and contribute candidate capabilities / constraints / open questions to `discovery.md` in the same shape `/spec:extract` uses.

### `--source` flags and the registry

`/spec:plan`'s existing `--source <key>=<path-or-url>` / `--from` / `--against` flags continue to work unchanged. They're additive with `registry.yaml:inputs` — the *analyse inputs* phase reads both and merges them into `discovery.md`. Inputs supplied via CLI flags carry an explicit kind (e.g. `--source legacy=./old/:legacy-code`) so they route through the same closed-enum dispatch as registry inputs.

### The flow

```
analyse inputs ──▶ generate plan ──▶ plan.yaml
```

Unchanged from today. The *sync peers* phase is absent because `registry.yaml` is absent.

---

## Layer 2: Multi-repo planning with the workspace

Layer 2 is the case the diagram describes: `registry.yaml` declares several repos, the *sync peers* phase clones them and inventories their specs, and *generate plan* consumes both the input inventory and the peer inventory.

`registry.yaml` is **required** in this layer — the *sync peers* phase has nothing to do without a registry, and is only present in the flow when the registry is. A single-repo run (no registry) never reaches this requirement.

### The Registry (multi-project)

```yaml
# .specify/registry.yaml
name: realtime
version: 1

projects:
  - name: traffic
    url: git@github.com:org/traffic.git
    schema: omnia@v1

  - name: command-centre
    url: git@github.com:org/command-centre.git
    schema: omnia@v1

inputs:
  - path: ./inputs/legacy-traffic/
    kind: legacy-code
  - path: ./inputs/ops-runbook.pdf
    kind: documentation
```

### The flow

```
analyse inputs ──▶ sync peers ──▶ generate plan ──▶ plan.yaml
```

Nothing about the `/spec:plan` invocation changes — the skill runs the fixed flow, and the presence of `registry.yaml` decides whether the *sync peers* phase is included.

### The sync-peers phase

Executed between *analyse inputs* and *generate plan*:

1. **Sync.** Shells out to `specify initiative workspace sync` to clone or fetch every `projects[]` entry into `.specify/workspace/<project-name>/` (local, read-only cache). Local projects (`url: .` or relative paths) are symlinked, not cloned. Cloning is a deterministic CLI operation; no cloning skill is involved.
2. **Inventory.** Walks each peer's `.specify/` tree — baseline specs, active plans, schema — and emits `.specify/plans/<initiative-name>/workspace.md` (a peer-by-peer summary of what's already specified and what's in flight).

`workspace.md` becomes a new input to *generate plan*, alongside `discovery.md`. No writes ever land in peer clones during planning.

### The workspace layout

```
.specify/
  registry.yaml         # required in Layer 2
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

The *generate plan* phase emits a **single cross-repo `plan.yaml`** in the initiating repo. Entries whose work spans peer projects reference them by registry name in `sources` / `affects`; execution of those entries requires Layer 3.

Per-repo `plan.yaml`s linked by a feature manifest — staged under `.specify/plans/<initiative-name>/<peer>/` and delivered out-of-band — is a plausible alternative output shape, but it is deferred (see *Alternatives Considered*). RFC-3 ships with exactly one shape.

### CLI surface additions


| Operation                 | CLI                                                    |
| ------------------------- | ------------------------------------------------------ |
| Read / verify registry    | `specify initiative registry {show, validate}` *(TBD)* |
| Clone / refresh workspace | `specify initiative workspace sync` *(TBD)*            |
| Inspect workspace state   | `specify initiative workspace status` *(TBD)*          |


These are all machinery the *sync peers* / *generate plan* phases shell out to; none are operator-facing entry points. `/spec:plan` remains the only command humans invoke.

### `--dry-run` and `--extend` under Layer 2

- **`--dry-run`.** The *sync peers* phase's read side may run (inventory whatever is already cloned) but MUST NOT clone new repos, write to `.specify/workspace/`, or write `workspace.md`. Mirrors the *analyse inputs* dry-run rule.
- **`--extend`.** When `.specify/workspace/` is already present, the *sync peers* phase may skip re-sync; `workspace.md` is regenerated from the existing cache. Refresh-on-`--extend` policy is *(TBD)*.

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

- RFC-2's `/spec:plan` skill is unchanged in invocation, core loop, working directory, and single-writer invariant. RFC-3's contribution is registry awareness, a fixed internal *sync peers* phase that runs automatically when `registry.yaml` is present, and a new `/spec:analyze` skill for documentation inputs; the skill picks up all three automatically.
- RFC-2 Layer 3 placed `pipeline.plan` inside `schema.yaml`. RFC-3 does not relocate that declaration to any new file — it removes the configuration surface. `schema.yaml` returns to its Layer 1 role (per-repo `define/build/merge`). Stack-specific planning helpers may still ship in schema directories and be consumed directly by the *generate plan* phase; that is a separate concern from a pipeline-shape declaration, which no longer exists.
- The Plan format is unchanged. RFC-3's only semantic extension on the Plan itself is that `sources` and `affects` may reference peer projects by registry name (resolved via `registry.yaml`).
- The `amend` edge in RFC-2's execution diagram — a phase discovering a neighbouring change and calling `specify initiative amend` — continues to work identically on RFC-3-produced plans.
- `specify initiative archive` sweeps `.specify/plans/<name>/` alongside `plan.yaml` as today; the new `workspace.md` artifact is archived with the rest, no code change required.

## Alternatives Considered

### Configurable planning pipeline (`planning.yaml`)

An earlier draft introduced `.specify/planning.yaml` declaring the authoring pipeline (`discovery → [workspace?] → propose`) with a CLI-provided default keyed on `registry.yaml` presence. RFC-3 v1 ships without that file. The configurability it enabled — custom step order, alternative *generate plan* implementations, pinning against auto-insert policy changes — is not needed for the initiatives RFC-3 is designed to serve, and the diagnostic surface it required (`planning show`, "resolved pipeline and its source") is real ongoing cost. The fixed flow has one shape, one set of phase names, and no "which pipeline is in effect?" question. If experience later reveals a need for alternative pipelines, `planning.yaml` can be reintroduced in a follow-up RFC with default = this RFC's fixed flow and no migration for existing repos, because nothing in v1 records the flow explicitly anywhere.

### Registry repo

A separate dedicated registry repo creates a coordination bottleneck. Every change requires commits to the registry, and the registry becomes a merge-conflict magnet. The chosen model keeps `registry.yaml` in whichever repo initiates the initiative (typically a dedicated platform / coordination repo), with peers autonomous. If you later need a central dashboard or CI check, you can build it on top of RFC-3 artifacts without requiring a separate write path.

### Cross-organisation coordination

If you're coordinating across *organisations* (not just repos), a registry repo makes more sense because you can't assume write access to peer repos. In that case, the registry holds the change manifests and peer spec snapshots, and the CLI treats them as read-only. Start with the in-initiator model for the single-organisation case.

### Plan-per-repo + feature manifest

RFC-3 emits a single cross-repo `plan.yaml` in the initiating repo. An alternative shape — each peer gets its own `plan.yaml` staged under `.specify/plans/<initiative-name>/<peer>/` in the initiator, linked by a top-level feature manifest tracking aggregate status — is plausible and probably the right long-term direction for peer autonomy and independent delivery. It is deferred because it adds three unsettled design problems at once (manifest format, cross-repo delivery mechanism, aggregate status tracking) and none of them are blocking for getting the single-plan case working. RFC-3 picks the smaller shape; a later RFC can add the manifest-linked shape without changing the Layer 1 / Layer 2 contract.

### Open `kind` vocabulary

An input `kind` not in the closed enum (`legacy-code`, `documentation`) is a hard error in v1. Warn-and-skip and "treat unknown as generic text" were both rejected. Warn-and-skip hides real configuration bugs behind a log line; a generic fallback pushes skill selection into the input author's head ("will my OpenAPI spec get extracted or analysed?") and makes `discovery.md` provenance opaque. The closed enum keeps the skill-selection contract explicit: extending it requires adding a skill and an RFC update, which is the right forcing function.

### `/rt:git-cloner` as the cloning delegate

The *sync peers* phase could delegate cloning to the existing `/rt:git-cloner` skill. RFC-3 does not: cloning is deterministic work with no judgment in it, which per RFC-1 belongs in the CLI (`specify initiative workspace sync`). Routing it through a skill would also couple a core Specify flow to the `rt` plugin, which is the wrong layering.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) — CLI surface this RFC extends.
- [RFC-2: Execution](archive/rfc-2-execution.md) — consumer of the Plan this RFC produces; introduces the `/spec:plan` skill RFC-3 extends.
