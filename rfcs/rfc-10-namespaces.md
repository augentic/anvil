# RFC-10: Namespaces

> Status: Draft

## Abstract

Reorganise the plugins and document the CLI namespace doctrine so that **scope of impact** is the single organising principle for the workflow surface. Today the `spec` plugin lumps every workflow skill — from "bootstrap a project" to "drive a single change" to "coordinate a multi-repo initiative" — into one slash-command palette, and there is no written rule that ties skill namespaces to CLI subcommand groups.

This RFC adopts a two-axis model:

- **Skills are namespaced by layer / scope of impact.** Three workflow plugins — `spec` (L0, project-meta), `change` (L1, single-change-in-single-repo), `plan` (L2, multi-change / multi-repo) — replace the current grab-bag `spec` plugin. The output-domain plugins (`omnia`, `vectis`, `contracts`, `rt`) are unchanged; they sit inside L1 as code-generation specialists invoked from `/change:build`. A fourth new plugin (`client`) is carved out for client-facing skills that sit outside the engineering-workflow layer model — initially housing `sow-writer`.
- **CLI subcommand groups are namespaced by domain.** A domain is a coupled set of artefacts under `.specify/` operated on as a unit. Most domains have one artefact (e.g. `registry` ↔ `registry.yaml`); some have several. The `change` group already follows this pattern (`change merge`, `change task`, `change journal`, `change outcome` are sub-groups under `.specify/changes/<name>/`). This RFC extends that pattern to the initiative domain: `specify initiative` is folded into `specify plan` because `plan.yaml` and `initiative.md` describe one initiative — the code already crosses the artefact boundary in both directions.

The two surfaces stay related through a **predictable layer-to-domain mapping** rather than literal name equality. There is deliberately no "internal" namespace.

The change is structural for skills (plugin reorg, slash-command renames) and minimally invasive for the CLI (one group merges into another with no behavioural change). Skill bodies, schemas, and CLI verb behaviour are unchanged. Migration is limited to the marketplace manifest, plugin directory layout, documentation cross-references, the user-facing slash-command namespace, and the `specify initiative` → `specify plan` consolidation.

## Motivation

### The namespace problem

Today the `spec` plugin bundles skills whose scopes of impact differ by an order of magnitude:

| Layer   | Scope of impact                            | Skills                                      |
| ------- | ------------------------------------------ | ------------------------------------------- |
| Layer 0 | the whole project                          | `init`, `extract`                           |
| Layer 1 | one change in one repo                     | `define`, `build`, `merge`, `drop`          |
| Layer 2 | many changes across many repos             | `analyze`, `plan`, `execute`, `initiative`  |

A daily developer scrolling `/spec:*` sees `init` (one-time bootstrap) and `extract` (one-time reverse engineering) and `initiative` (cross-repo umbrella) interleaved with `define` and `build`. The cost is cognitive, not technical: nothing breaks, but every operator has to learn that `/spec:*` is a grab-bag rather than a coherent surface.

A second, less visible problem is that there is no written rule about how the skill palette relates to the CLI subcommand surface. The CLI is already noun-grouped (`change`, `plan`, `initiative`, `registry`, `workspace`, `schema`) and that grouping is good — but nothing in the docs explains why it differs from the slash-command grouping, or what the *relationship* between the two surfaces should look like.

### Why now

- The plugins are not yet broadly adopted. There is no muscle memory or downstream documentation to break, so the migration cost is bounded to docs in this repository plus the marketplace manifest.
- The skill set is approaching steady state at the level of "what a workflow looks like end to end." Future skills will plausibly extend each layer (more L1 phase verbs, more L2 orchestration verbs) rather than introduce new layers. Splitting now anchors those extensions.
- The CLI surface already groups verbs by artefact noun. The skill layer is the only place where the layers remain conflated, and it's the only place where the namespace doctrine isn't written down.

### Non-goals

- **No CLI verb behavioural changes.** The only CLI change is the consolidation of `specify initiative` under `specify plan` — three verb renames (`initiative create` → `plan brief create`, `initiative show` → `plan brief show`, `initiative finalize` → `plan finalize`) with identical semantics. No flags, exit codes, or JSON shapes change. See §"Merging `initiative` into `plan`".
- **No skill rewrites.** SKILL.md bodies, references, and authority hierarchies are unchanged.
- **No new skills.** Reorganisation only.
- **No reshuffle of `omnia`, `vectis`, `contracts`, `rt`.** Those plugins are output-domain L1 specialists; their boundary is unrelated to this reorg.
- **No "internal" namespace.** See §"Why there is no internal namespace" below.
- ~~**No `/plan:initiative` skill rename.** The skill name stays as-is; a follow-up to `/plan:run` or `/plan:drive` is out of scope.~~ **Superseded.** A pre-RFC-10 progressive-disclosure pass folded the former `/spec:initiative` skill into `/spec:plan` as a flag-gated `--orchestrate` mode — the umbrella sequence now lives at `plugins/spec/skills/plan/orchestration.md`. RFC-10 still applies to the outer plugin reorg (`spec` → `spec` + `change` + `plan` + `client`), which would land `/spec:plan --orchestrate` as `/plan:plan --orchestrate` (or a follow-up rename). The "skills: `analyze`, `plan`, `execute`, `initiative`" entry in §"Skill plugin map" should be read as "skills: `analyze`, `plan` (with `--orchestrate` mode), `execute`".

## Detailed Design

### The two-axis model

Skills and CLI subcommands answer different questions, and they are best namespaced by different things.

- **Skills** answer "what workflow am I doing right now?" — they are verbs the operator invokes from a palette. The natural namespace key is **scope of impact** (the layer).
- **CLI subcommands** answer "what domain am I touching?" — they are deterministic primitives a skill or human composes into a workflow. The natural namespace key is **the domain being mutated**: a coupled set of artefacts under `.specify/` operated on as a unit. Today: `change`, `plan` (post-merge: also covering the initiative brief and finalize), `registry`, `workspace`, `schema`.

The CLI's domain-grouping is correct and stable, and a single skill routinely shells into many domain groups. `/spec:define` today calls `change create`, `change journal append`, `change validate`, `plan amend`, sometimes `registry add`. Forcing CLI groups to mirror skill groups would break that — you would need a `define` CLI group that is a thin façade over five other groups, doubling the surface.

When a domain spans multiple artefacts, the CLI sub-groups them rather than splintering into multiple top-level groups. This is already how `change` works (`change merge { run, preview, conflict-check }`, `change task { progress, mark }`, `change journal { append, show }`, `change outcome { set, show }`). The same pattern applies to the initiative domain (`plan brief { create, show }`).

The consistency this RFC pursues is therefore **predictable layer-to-domain mapping**, not literal name equality. A user who knows a skill's layer should be able to predict which CLI groups it touches, and vice versa.

### Skill plugin map (layer-named)

| Plugin   | Layer           | Skills                                     | One-line summary                                                 |
| -------- | --------------- | ------------------------------------------ | ---------------------------------------------------------------- |
| `spec`   | L0              | `init`, `extract`                          | Establish or repopulate the `.specify/` tree                     |
| `change` | L1              | `define`, `build`, `merge`, `drop`         | Drive a single change through its lifecycle                      |
| `plan`   | L2              | `analyze`, `plan` (with `--orchestrate` mode), `execute` | Coordinate engineering work above a single change                |
| `client` | out-of-layer    | `sow-writer`                               | Generate client-facing artifacts (SoWs, etc.) from project specs |
| `omnia` / `vectis` / `contracts` / `rt` | L1 specialists | (unchanged) | Per-stack code generation, invoked from `/change:build`          |

The plugin name in each workflow case is the strongest noun for the layer:

- L0 = `spec` because the artefact L0 establishes *is* `.specify/`.
- L1 = `change` because the unit of work *is* a change.
- L2 = `plan` because the artefact L2 coordinates *is* `plan.yaml`.

That makes every slash command read correctly out loud: "spec-init", "change-define", "change-build", "plan-execute".

`extract` moves into `spec` because it is L0 work: it populates baseline specs from an existing source codebase, exactly as `init` populates the rest of `.specify/`. Both are "establish or repopulate the project tree" verbs.

The output-domain plugins (`omnia`, `vectis`, `contracts`, `rt`) are not workflow plugins — they are L1 specialists invoked from `/change:build`. They keep their existing names and boundaries; this RFC does not touch them.

The `client` plugin is also not a workflow plugin: it consumes existing project artifacts to produce commercial deliverables. Its audience (delivery lead, partnership owner) and reading-direction (specs in, prose out) differ from the engineering-workflow layers, so it sits alongside the workflow plugins rather than inside the layer stack. See §"Decision: `sow-writer` lives in its own `client` plugin" for the full rationale.

### CLI namespace map (domain-named)

The CLI grouping is preserved and documented here for reference. The "Layer" column is descriptive — it follows from where the domain's artefacts live under `.specify/` — and is used to relate CLI groups back to the skill plugin map.

| CLI group     | Layer         | Domain                                                                          | Notes                                                                |
| ------------- | ------------- | ------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `init` (top)  | L0            | the whole `.specify/` tree                                                      | top-level verb; no group                                             |
| `status` (top)| cross-cutting | every layer                                                                     | dashboard read-only                                                  |
| `registry`    | L0            | `.specify/registry.yaml`                                                        | platform catalogue                                                   |
| `schema`      | L0            | schema cache + briefs                                                           | internal plumbing; no skill exposure                                 |
| `change`      | L1            | `.specify/changes/<name>/` (multi-artefact)                                     | sub-grouped: `merge`, `task`, `journal`, `outcome`                   |
| `vectis`      | L1            | (Crux project tree)                                                             | output-domain CLI for Crux scaffolding                               |
| `plan`        | L2            | the initiative — `.specify/plan.yaml` + `.specify/initiative.md` + `.specify/plan.lock` (multi-artefact) | sub-grouped: `brief`, `lock`; absorbs former `specify initiative` group |
| `workspace`   | L2            | `.specify/workspace/`                                                           | multi-repo materialisation                                           |

`.specify/plan.lock` is a runtime lock — it is acquired and released by `/plan:execute` rather than authored by an operator — but it lives in the same domain as `plan.yaml` and `initiative.md` and is owned by the same CLI group, so the domain row counts it.

The full `plan` verb surface after consolidation:

```
specify plan create <name> [--source <key>=<path-or-url> ...]
specify plan validate
specify plan doctor
specify plan next
specify plan status
specify plan add <name> [...]
specify plan amend <name> [...]
specify plan transition <name> <target> [--reason <text>]
specify plan archive [--force]
specify plan finalize [--clean] [--dry-run]                # was: specify initiative finalize
specify plan brief { create <name> | show }                # was: specify initiative { create, show }
specify plan lock { acquire | release | status }
```

The rule that generates this table is short enough to belong in `AGENTS.md`:

> **CLI subcommand groups own one *domain* each — a coupled set of artefacts under `.specify/` operated on as a unit. Domains with more than one artefact use sub-groups (`change merge`, `change task`, `plan brief`, `plan lock`). The domain's layer follows from where its artefacts live in `.specify/`.**

### Merging `initiative` into `plan`

The previous draft of this RFC kept `specify initiative` as a separate top-level CLI group on the basis of "one group per artefact." That rule was descriptive of the v1 CLI but not actually load-bearing — `change` already violates a strict reading of it (`change merge`, `change task`, `change journal`, `change outcome` are sub-groups within the changes domain). This RFC generalises the rule: **one group per domain, sub-grouped when the domain has multiple artefacts.**

`plan.yaml` and `initiative.md` are two artefacts of one domain — the initiative — and the code already crosses the artefact boundary in both directions:

- **`specify plan archive` co-moves `initiative.md`.** `Plan::archive` in `crates/change/src/plan.rs` looks for `.specify/initiative.md` and sweeps it into the archive directory alongside `plan.yaml`.
- **`specify initiative finalize` archives `plan.yaml`.** The finalize implementation loads `plan.yaml`, classifies plan-entry terminal status, and archives both files together.
- **`specify plan status`'s help text is "initiative progress report".** The help string already conflates the two.
- **The `/spec:plan --orchestrate` mode** (formerly the `/spec:initiative` skill, folded into `/spec:plan` in a pre-RFC-10 progressive-disclosure pass) drives them as one workflow (brief → registry validate → plan → execute → push → workspace merge → finalize).

Keeping the two CLI groups separate forces operators to memorise that finalize lives under `initiative` while the rest of initiative coordination lives under `plan`. Folding them into one group reflects how the code already behaves.

#### Why `plan` won the umbrella name

Two shapes were on the table:

- **Promote `initiative` to umbrella.** Operators say "I'm running an initiative", so `specify initiative {add, amend, transition, next, status, validate, doctor, archive, finalize, brief, lock}` matches the spoken language. Cost: every daily verb gets six extra characters (`specify plan add` → `specify initiative add`), and the daily verbs are used ~20× more often than `finalize` and `brief show`.
- **Keep `plan` as umbrella, fold `initiative` in.** Daily verbs stay at their current path. Two verbs move (`brief`, `finalize`); ten verbs do not. Cost: `specify plan finalize` reads slightly less naturally than `specify initiative finalize` for the rare-but-load-bearing closure step.

The second shape wins on keystroke economics. The unnatural reading at finalize time is documented away by treating `plan` as the noun for the whole initiative domain (which it already is in `plan status`'s help text).

#### Verb rename map

| Today                          | After                          |
| ------------------------------ | ------------------------------ |
| `specify initiative create`    | `specify plan brief create`    |
| `specify initiative show`      | `specify plan brief show`      |
| `specify initiative finalize`  | `specify plan finalize`        |

JSON shapes, exit codes, error variants, and flag parsing are preserved verbatim. Skills that today call `specify initiative ...` (only `/plan:initiative` and the `/plan:execute` self-heal path) are updated to call the new verb names; nothing else moves.

#### No deprecation aliases

The old `specify initiative {create, show, finalize}` verbs are removed in the same change that adds the new `specify plan {brief, finalize}` verbs. There are no transitional aliases and no deprecation period. The plugins are not yet broadly adopted, no downstream consumer scripts call the old verbs, and the `make checks` invariant added in §"Migration plan" guarantees that no in-repo caller is left behind. Hard-breaking now is cheaper than carrying aliases that would otherwise need their own removal change later.

#### ~~Why not also rename `/plan:initiative`~~ Superseded — `/spec:initiative` was folded into `/spec:plan --orchestrate`

A pre-RFC-10 progressive-disclosure pass folded the standalone `/spec:initiative` skill into `/spec:plan` as the `--orchestrate` mode; the umbrella sequence now lives at `plugins/spec/skills/plan/orchestration.md`. After RFC-10 lands, the orchestration mode rides along with `/spec:plan` into `/plan:plan` (no separate skill, no separate slash-command suffix). The original RFC-10 worry — that renaming the skill would touch `marketplace.json`, every doc reference, and the dedicated SKILL.md — was discharged early as part of the same fold.

### Why there is no internal namespace

A category named "internal reusable utility actions that do not fit elsewhere" is tempting but anti-productive. Whenever a verb feels "internal", one of two things is true, and the right response in either case is *not* to invent an `/internal:*` namespace.

1. **It is a CLI sub-primitive that another verb composes.** Examples: `specify schema {resolve, check, pipeline}`, `specify change journal append`, `specify change task mark`, `specify change outcome set`, `specify plan lock {acquire, release, status}`. These are already nested under their domain's CLI group. They do not need a separate namespace; they need a "supporting" or "internal" tag in the CLI reference docs so operators know they are machine-callable rather than daily-driver verbs.
2. **It is a user-facing verb that has been misclassified.** `extract` is the worked example: it is not internal, it is L0 (it populates `.specify/` from existing code, parallel to `init`). The fix is to place it in L0, not to invent a fourth bucket for it.

If a verb is genuinely user-facing and genuinely fits no layer, that is a signal the layer model is missing something — investigate before adding an escape-hatch namespace. As of this RFC every existing skill and CLI verb finds a home under the model above.

### Plugin descriptions

Each plugin's `marketplace.json` description should encode the layer explicitly:

- `**spec**` — Project-meta (L0): bootstrap a project (`init`) and populate baseline specs from existing code (`extract`).
- `**change**` — Single-change loop (L1): drive a change through its lifecycle (`define` → `build` → `merge`, or `drop`).
- `**plan**` — Above-a-single-change (L2): analyse, author and execute a plan, and drive an initiative end-to-end.
- `**client**` — Client-facing deliverables (out-of-layer): generate Statements of Work and other commercial artifacts from project specs and plans (`sow-writer`).
- `**omnia / vectis / contracts / rt**` — Output-domain specialists invoked from `/change:build` (unchanged).

### Slash-command rename map

| Today              | After                                                                    |
| ------------------ | ------------------------------------------------------------------------ |
| `/spec:init`       | `/spec:init`                                                             |
| `/spec:extract`    | `/spec:extract`                                                          |
| `/spec:define`     | `/change:define`                                                         |
| `/spec:build`      | `/change:build`                                                          |
| `/spec:merge`      | `/change:merge`                                                          |
| `/spec:drop`       | `/change:drop`                                                           |
| `/spec:analyze`    | `/plan:analyze`                                                          |
| `/spec:plan`       | `/plan:plan`                                                             |
| `/spec:execute`    | `/plan:execute`                                                          |
| ~~`/spec:initiative`~~ `/spec:plan --orchestrate` | ~~`/plan:initiative`~~ `/plan:plan --orchestrate` (skill folded into `/spec:plan` pre-RFC-10; the slash-command path becomes the orchestration mode of `/plan:plan` after this reorg) |
| `/plan:sow-writer` | `/client:sow-writer`                                                     |

The cross-plugin invocation graph is unchanged. `/plan:execute` calls into `/change:define → /change:build → /change:merge` exactly as `/spec:execute` does today; the dependency simply crosses a renamed boundary.

### Repository layout

```
plugins/
├── spec/
│   ├── README.md
│   ├── references/
│   ├── rules/
│   └── skills/
│       ├── init/
│       └── extract/
├── change/
│   ├── README.md
│   └── skills/
│       ├── define/
│       ├── build/
│       ├── merge/
│       └── drop/
├── plan/
│   ├── README.md
│   └── skills/
│       ├── analyze/
│       ├── plan/
│       ├── execute/
│       └── initiative/
├── client/
│   ├── README.md
│   └── skills/
│       └── sow-writer/
├── omnia/      (unchanged)
├── vectis/     (unchanged)
├── contracts/  (unchanged)
├── rt/         (unchanged)
└── references/ (unchanged)
```

The shared `plugins/spec/references/` and `plugins/spec/rules/` directories follow the `spec` plugin and stay where they are. The top-level `plugins/references/` directory (holding `specify.md`, `agent-teams.md`, `review-checks.md`) is also unchanged. Skills in `change`, `plan`, and `client` continue to reach into those references via symlinks; no reference content moves.

Symlink targets do change, however, because the existing convention uses **relative** paths anchored to `plugins/spec/`:

```text
plugins/spec/skills/define/references             -> ../../references            # → plugins/spec/references/
plugins/spec/skills/extract/references/specify.md -> ../../../references/specify.md  # → plugins/references/specify.md
```

When `define/`, `build/`, `merge/`, and `drop/` move from `plugins/spec/skills/` to `plugins/change/skills/`, the bulk-link target `../../references` would resolve to `plugins/change/references/` (which does not exist). Each relocated skill's `references` symlink must be re-pointed to `../../../spec/references` so the resolved path remains `plugins/spec/references/`. The same pattern applies to `analyze`, `plan`, `execute`, and `initiative` if they grow `references` symlinks during the move (today only `analyze` carries a per-skill references directory and it is not a symlink, so it simply moves wholesale). `init` and `extract` stay in `plugins/spec/skills/`, so their symlinks are unaffected. The `client` plugin's `sow-writer` does not currently use a `references` symlink and is unaffected.

The migration plan lists this as an explicit step (§"Migration plan / Skill-side / 2") rather than relying on it being inferred from "skills move".

### Marketplace manifest

`.cursor-plugin/marketplace.json` gains the new plugin entries (`change` and `client`; `plan` already exists) and updates the `spec` entry to reflect its narrowed scope. The relative ordering follows the operator journey: bootstrap (`spec`), daily work (`change`), orchestration (`plan`), client deliverables (`client`), then the output-domain plugins (`omnia`, `vectis`, `contracts`, `rt`).

### Authority hierarchy and references

The authority hierarchy in `.cursor/rules/project.mdc` (SKILL.md > Specify artifacts > references > source > inference) is plugin-agnostic and unchanged. References shared across plugins stay under `plugins/spec/references/` and `plugins/references/`; access patterns are preserved.

### Documentation surface

Two parallel rename passes are required.

**Slash-command renames** — every `/spec:*` reference in the documentation corpus needs to be remapped exactly once:

- `.cursor/rules/project.mdc` and `AGENTS.md` (also gain the new namespace doctrine, see below)
- `plugins/spec/README.md` (rewritten to cover the narrowed `spec` plugin), `plugins/plan/README.md` (rewritten to cover the workflow plugin and drop `sow-writer`), and new `plugins/{change,client}/README.md` files
- `docs/reference/cli/*.md`, `docs/how-to/*.md`, `docs/tutorials/*.md`, `docs/explanation/*.md`
- `docs/orientation/*.md` and `docs/index.md`
- `docs/SUMMARY.md`
- **`docs/reference/initiative-skills/`** is renamed to `docs/reference/plan-skills/`. The directory holds `analyze.md`, `execute.md`, `initiative.md`, `plan.md`, and `index.md` — all four skills are L2 / `/plan:*` after this RFC, so the directory name should match the plugin (paralleling the existing `docs/reference/change-skills/`). Inbound links from `docs/SUMMARY.md`, `docs/index.md`, `docs/reference/index.md`, the tutorials, and the how-tos are updated as part of this rename.
- **`docs/reference/plugins/plan.md`** describes today's SoW-only `plan` plugin. After this RFC the `plan` plugin's identity inverts — it becomes the L2 workflow plugin — so this file is rewritten (not search-and-replaced) to describe the new plugin, and a new `docs/reference/plugins/client.md` is authored to cover the new `client` plugin and the relocated `sow-writer`. A new `docs/reference/plugins/change.md` and a rewritten `docs/reference/plugins/spec.md` are added/updated alongside.

**Schemas and briefs** — the `schemas/` tree contains brief markdown and fixture files that quote slash-command invocations as part of their authored prose:

- `schemas/omnia/briefs/{specs,proposal,plan/{analyze,discovery,propose}}.md`, `schemas/vectis/briefs/{build,plan/propose}.md` — operator-facing prose embedded in briefs.
- `schemas/{plan/plan.schema.json, schema.schema.json}` — `description` fields that quote slash-command names in JSON Schema metadata.
- `schemas/omnia/briefs/fixtures/**/*.{md,txt,json}` — fixture inputs (e.g. `define-invocation.txt`, fixture `README.md` files) that mention `/spec:define` etc.

These are remapped by the same find-and-replace pass and covered by the same `make checks` invariant.

**Skill fixtures and transcripts** — every transcript and golden-output fixture under `plugins/spec/skills/{execute,initiative,plan,analyze}/fixtures/` is part of the rename scope. Concretely:

- `plugins/spec/skills/execute/fixtures/{single-change,loop,self-heal,field-wiring,e2e-platform-v2,e2e-platform-v2-with-crash,multi-project,cross-project-contract-warning,dry-run,back-compat}/` transcripts, READMEs, `metadata*.yaml`, `journal*.yaml`, `expected-output.md`, and `expected-journal.yaml` files.
- `plugins/spec/skills/initiative/fixtures/{migrate-legacy,new-feature,update-existing}/` transcripts, READMEs, `expected/{plan.yaml.after,initiative.md.after,archive-summary.md}`, and `inputs/registry.yaml.before`.
- `plugins/spec/skills/plan/fixtures/{discovery,dry-run,propose,propose-vectis,registry-proposal}/` transcripts, READMEs, notes, and `expected-output.md`.
- `plugins/spec/skills/analyze/fixtures/scaffold-example/` README and inputs.

These files are docs-equivalent prose — they are pinned exemplars of agent transcripts and serve as living documentation rather than as snapshot inputs to a comparison harness. Renaming them in lockstep with the documentation pass is therefore safe; no fixture-comparison test breaks because no comparison test reads these strings as expected output. (Fixture *commands* invoked by skill tests, e.g. `specify plan ...` shell-outs in test harnesses, are covered by the CLI-side rename track.)

The remapping is mechanical — every renamed slash command in the table above can be substituted by find-and-replace across `docs/`, `schemas/`, `plugins/`, `.cursor/`, and `AGENTS.md`, then spot-checked. The `make checks` script gains an entry to flag remaining `/spec:{define,build,merge,drop,analyze,plan,execute,initiative}` and `/plan:sow-writer` references after the split, so doc drift is caught in CI. (`/spec:init` and `/spec:extract` are *not* in that flag list — they remain valid.)

**CLI verb renames** — every `specify initiative {create, show, finalize}` reference in the documentation, skill bodies, CLI surface tables, and fixture transcripts needs to be remapped to its `specify plan ...` equivalent:

- `AGENTS.md` CLI surface table (the bullet list under "CLI surface the skills depend on")
- `.cursor/rules/project.mdc` (lists `specify initiative` group)
- `docs/reference/cli/initiative.md` → **merged** into `docs/reference/cli/plan.md`. The standalone file is removed; inbound links are repointed to `docs/reference/cli/plan.md#brief` and `#finalize` anchors. (A redirect file would only postpone the link audit, and the file count is small enough to fix in one pass.)
- The `/plan:initiative` skill body (today calls `specify initiative finalize`)
- The `/plan:execute` skill body (self-heal path may invoke `specify initiative finalize`)
- The `plugins/spec/skills/{initiative,execute}/fixtures/**` transcripts that quote `specify initiative …` invocations (covered by the same fixture pass above).
- RFC-3a, RFC-9 §4A and §4C, and any other RFC that references `specify initiative finalize` by name (those references stay accurate but should be annotated with the new verb name once this RFC lands)
- `make checks` gains a second invariant flagging remaining `specify initiative {create,show,finalize}` references after the rename.

### Namespace doctrine in AGENTS.md

Add a short section to `AGENTS.md` (or a new `docs/explanation/namespaces.md` linked from it) capturing the rule once and for all:

> **Skills** are namespaced by layer / scope of impact: `spec` (L0, project-meta), `change` (L1, single change in single repo), `plan` (L2, multi-change / multi-repo). The output-domain plugins (`omnia`, `vectis`, `contracts`, `rt`) are L1 specialists invoked from `/change:build`. The `client` plugin sits outside the layer model — it consumes project artifacts to produce client-facing deliverables.
>
> **CLI subcommands** are namespaced by the **domain** they operate on — a coupled set of artefacts under `.specify/` operated on as a unit. Each CLI group's layer follows from where its domain sits in the project tree. Domains with multiple artefacts use sub-groups (`change merge`, `change task`, `plan brief`, `plan lock`). Skills compose CLI groups across layers; the CLI does not mirror skill boundaries.
>
> There is no "internal" namespace. Implementation primitives hide under their domain's CLI group; user-facing utilities belong in one of the three skill layers, or — if their audience is non-engineering — in the out-of-layer `client` plugin.

This is the canonical reference for any future namespace argument.

## Decision: `sow-writer` lives in its own `client` plugin

`sow-writer` is the only skill whose layer-fit is debatable.

- **Workflow planning** (`analyze`, `plan`, `execute`, `initiative`) is engineering coordination. Audience: tech lead, initiative driver, automation. Reads/writes `plan.yaml`, `initiative.md`, `registry.yaml`. Squarely L2.
- **Commercial planning** (`sow-writer`) is client-facing scoping. Audience: delivery lead or partnership owner. Reads artifacts and emits SoW prose. L2-shaped (above-a-single-change), but the audience is different.

The audience boundary (engineering vs. commercial) is sharper than any layer boundary, and it does not run parallel to the layer stack. `sow-writer` therefore moves to a new `client` plugin alongside the workflow plugins, leaving `plan` strictly engineering-facing. Concretely:

- `/plan:sow-writer` is renamed to `/client:sow-writer` (see §"Slash-command rename map").
- A new `plugins/client/` directory hosts the skill, with room to grow into adjacent client-facing skills (`proposal-writer`, `pricing-writer`, contract-statement generators, etc.) without polluting `plan`.
- The `client` plugin sits outside the layer model — its audience differs from any engineering-workflow layer. This parallels the output-domain plugins (`omnia`, `vectis`, `contracts`, `rt`), which also sit outside the layer model proper but anchor onto it (in their case, as L1 specialists invoked from `/change:build`). `client` anchors onto it the other way: it consumes artifacts produced by L1 and L2 to emit deliverables outside the engineering surface entirely.

Two alternatives were considered and rejected:

1. **Keep `sow-writer` in `plan`.** L2 read as "anything coordinating above a single change, including delivery output." Rejected: conflating engineering coordination with commercial output encourages future delivery skills to land in `plan` by accident, and it muddies the `plan` plugin description for daily users who never touch SoWs.
2. **Rename the workflow plugin** (e.g. `initiative` or `orchestrate`) so `plan` could keep the SoW skill and inherit the commercial connotations of "planning a project". Rejected: more invasive than option 1 above, and it would force the daily verbs (`/plan:execute`, `/plan:analyze`) to be renamed at the same time. The keystroke-economics argument that kept `plan` as the umbrella for `initiative` consolidation (see §"Why `plan` won the umbrella name") applies equally here.

## Smaller design observations

- **`contracts` is two things at once.** As an abstraction (machine-readable interfaces between projects), it is an L0 platform concern. As a plugin (`/contracts:writer`, etc.), it is an L1 specialist invoked from build. As a brief, it is L1 logic that runs in define pipelines. This RFC does not try to unify these — the *plugin* sits alongside `omnia`/`vectis`/`rt` as an output-domain L1 specialist, and the *abstraction* shows up in L0 prose without requiring its own L0 namespace.
- **`/change:build` reads as "build the change."** Acceptable and consistent with the existing CLI verb `specify change`. Considered and rejected: `/change:implement`. Symmetry with `define`/`build`/`merge`/`drop` outweighs the slight verb-noun ambiguity.
- **`analyze` and `extract` share DNA.** Both read source code; one emits capability summaries (`discovery.md`), the other emits full artifacts. They land in different plugins because their *callers* differ: `analyze` is invoked from plan authoring (L2); `extract` is a standalone bootstrap (L0). Future maintenance should keep that boundary — `analyze` should not grow into `extract`'s territory.
- **`/plan:initiative` is acceptable but slightly redundant.** The Layer 4 umbrella skill is bigger than its current name suggests, and the redundancy increases once `initiative` is no longer a CLI noun (since the CLI consolidation in §"Merging `initiative` into `plan`" leaves `initiative` only as a skill name and as a domain-level concept). A future rename to `/plan:run` or `/plan:drive` would read more naturally; that is out of scope for this RFC because it is a skill-level rename rather than a namespace reorganisation.
- **L0 has only two skills.** That is fine — L0 is *meant* to be small (bootstrap and reverse-direction setup are both rare-but-distinct operations). If L0 stays this small for several releases it is worth revisiting whether `spec` pays its keep as a separate marketplace entry, but conflating L0 verbs with L1's daily palette would muddy the daily palette. Keep them separate.
- **The `change` group is the precedent for `plan`'s sub-grouping.** `change merge`, `change task`, `change journal`, and `change outcome` are all sub-groups under the changes domain. Folding `initiative` into `plan` as `plan brief` and a top-level `plan finalize` follows the same pattern; it does not introduce a new shape. If a future verb (e.g. `plan workspace ...`) wants to live under `plan`, the same precedent applies.

## Alternatives considered

### A. Status quo with internal grouping

Keep all ten skills in `spec` and reorganise the README into the three layer buckets above. Zero migration cost. Rejected because it leaves the slash-command palette unchanged and therefore does not address the layer-mixing pain that motivates this RFC.

### B. Carve out only the L2 skills

Two plugins: `spec` (L0 + L1) and a new L2 plugin (`analyze`, `plan`, `execute`, `initiative`). Targets the largest single layer boundary. Rejected because it leaves L0 (`init`, `extract`) in the same plugin as L1's daily phase loop — the original grab-bag at smaller scale.

### C. Two-plugin split: `plan` and `exec`

The original framing for this work proposed a `plan` plugin for planning skills and an `exec` plugin for define/build/merge. Rejected for three reasons:

1. **Name collision.** `plan` was already the SoW plugin; reusing the name conflates engineering coordination with commercial output. (This RFC instead lifts SoW into a dedicated out-of-layer `client` plugin per §"Decision: `sow-writer` lives in its own `client` plugin", removing the conflation entirely.)
2. **Unallocated skills.** `init`, `extract`, `drop` did not cleanly land in either bucket.
3. **`/exec:execute` is awkward** and `execute` is itself a planning-loop driver — it belongs with `plan`, not `exec`.

### D. Audience-named four-plugin split

An earlier draft of this RFC proposed `spec` (L0 init only), `audit` (L0 extract only), `change` (L1), `plan` (L2). It was framed by audience ("maintainer reconciling code with artifacts" for the audit bucket). Rejected because:

- Audience is less stable than scope of impact. The "maintainer reconciling code" audience is a use-case, not a structural property; if a future drift checker is callable from a build phase, it stops being a reconciliation tool and becomes an L1 specialist, but the namespace would not move with it.
- A dedicated `audit` plugin for a single skill (`extract`) is a one-skill marketplace entry. The L0 layer already houses `init`, which is the natural neighbour: both are "establish or repopulate `.specify/`" verbs. Folding `extract` into `spec` keeps the layer model intact and avoids the singleton plugin.

### E. Layer-prefixed namespaces (`/L0:init`, `/L1:define`, `/L2:execute`)

Use the layer numbers literally as the slash-command prefix. Rejected because layer numbers convey nothing about purpose to a new reader, and they age badly: a hypothetical Layer 3 would force a rename. The current proposal uses *named* prefixes (`spec` / `change` / `plan`) that happen to encode the layer through the strongest noun for that layer; the layer model stays as the *organising principle* without leaking into the user-facing string.

## Migration plan

The migration breaks naturally into two coordinated tracks: skill-side reorg and CLI-side consolidation. They land in a single change but are listed separately for clarity.

### Skill-side (slash-command reorg)

1. **Author the new plugin directories** under `plugins/change/` and `plugins/client/`, and update `plugins/spec/` and `plugins/plan/`. Move skill directories — no content edits. The `sow-writer/` directory moves from `plugins/plan/skills/` to `plugins/client/skills/`.
2. **Re-point per-skill `references` symlinks.** For each skill that moves out of `plugins/spec/skills/` (i.e. `define`, `build`, `merge`, `drop` → `plugins/change/skills/`; `analyze`, `plan`, `execute`, `initiative` → `plugins/plan/skills/`), recreate any `references` symlink so its target resolves to `plugins/spec/references/` (bulk-link skills) or `plugins/references/` (per-file skills). Concretely: replace `references -> ../../references` with `references -> ../../../spec/references`, and for any per-file links of the form `references/specify.md -> ../../../references/specify.md`, replace with `references/specify.md -> ../../../../references/specify.md`. Audit by running `find plugins/{change,plan,client}/skills -type l` after the move and confirming each symlink resolves.
3. **Update `marketplace.json`** with the new plugin entries (`change`, `client`) and revised descriptions for `spec` and `plan`. Reorder entries to match the operator journey (`spec` → `change` → `plan` → `client` → `omnia` → `vectis` → `contracts` → `rt`).
4. **Rename slash-command references** across `docs/`, `schemas/`, `.cursor/rules/`, `AGENTS.md`, the per-plugin READMEs, and the skill fixture trees enumerated in §"Documentation surface" (including `/plan:sow-writer` → `/client:sow-writer`).
5. **Rename the docs/reference subdirectories.** Move `docs/reference/initiative-skills/` to `docs/reference/plan-skills/`. Rewrite `docs/reference/plugins/plan.md` to describe the workflow plugin, author `docs/reference/plugins/{change,client}.md`, and refresh `docs/reference/plugins/spec.md`. Repoint `docs/SUMMARY.md`, `docs/index.md`, and `docs/reference/index.md`.
6. **Merge `docs/reference/cli/initiative.md` into `docs/reference/cli/plan.md`.** Delete the standalone file; repoint inbound links to the new anchors (`#brief`, `#finalize`).
7. **Add the namespace doctrine** to `AGENTS.md` (or `docs/explanation/namespaces.md`) verbatim from §"Namespace doctrine in AGENTS.md".
8. **Add a `make checks` invariant** that fails CI on any remaining `/spec:{define,build,merge,drop,analyze,plan,execute,initiative}` or `/plan:sow-writer` reference outside the migration map itself, and a parallel invariant for `specify initiative {create,show,finalize}`. (`/spec:init` and `/spec:extract` remain valid.) The check scans `docs/`, `schemas/`, `plugins/`, `.cursor/`, `AGENTS.md`, `README.md`, and the repo-root scripts.
9. **Update `make dev-plugins` and `make prod-plugins`** if their symlink targets enumerate plugin directories.
10. **Bump the marketplace version** in `.cursor-plugin/marketplace.json` and announce the rename in the changelog.

### CLI-side (`initiative` → `plan` consolidation)

1. **Fold `InitiativeAction` into `PlanAction`.** In `specify-cli/src/cli.rs`: remove the top-level `Initiative { action: InitiativeAction }` variant; add `PlanAction::Brief { action: PlanBriefAction }` (with `Create { name }` / `Show` variants) and `PlanAction::Finalize { clean: bool, dry_run: bool }`. Drop the `InitiativeAction` enum.
2. **Move dispatcher code.** Migrate `specify-cli/src/commands/initiative.rs` into `specify-cli/src/commands/plan/` (e.g. as `brief.rs` and `finalize.rs`) and wire the new `PlanAction::Brief` / `PlanAction::Finalize` arms in `commands/plan/mod.rs`. The underlying `InitiativeBrief::*` / `run_finalize` functions in the library crates are unchanged.
3. **Move `src/initiative_finalize.rs`** to `src/plan_finalize.rs` (or keep the filename if churn is undesirable; the CLI verb name is what matters). Update internal `pub use` references accordingly.
4. **Rename tests.** `tests/initiative.rs` → fold into `tests/plan.rs` or rename `tests/plan_initiative.rs`. Update fixture commands from `specify initiative ...` to `specify plan ...`.
5. **Add a `make checks` invariant** that fails CI on any remaining `specify initiative {create,show,finalize}` reference outside the migration map itself.
6. **Update skill bodies.** `/plan:initiative` SKILL.md and the `/plan:execute` self-heal path are the only call sites that invoke `specify initiative ...` today; rewrite both to use the new verbs.
7. **Update CLI reference docs.** Merge `docs/reference/cli/initiative.md` into `docs/reference/cli/plan.md`, or keep a one-line redirect file pointing at the new location.

### Combined posture

The migration is a single change with ~17 tasks across the two tracks (10 skill-side, 7 CLI-side). No CLI verb behaviour, no JSON shape, no schema, and no skill body logic moves; the work is dominated by mechanical renames, the symlink re-pointing pass, the docs-directory rename, two new `PlanAction` variants, and the namespace doctrine. It is suitable for a single `/spec:define → /spec:build → /spec:merge` cycle (using today's slash commands, before the rename takes effect).

## Recommendation

Adopt the three-plugin workflow split (`spec` / `change` / `plan`) with the skill mapping in §"Skill plugin map", carve out a new `client` plugin for `sow-writer` per §"Decision: `sow-writer` lives in its own `client` plugin", consolidate `specify initiative` into `specify plan` per §"Merging `initiative` into `plan`", document the CLI namespace rule in §"CLI namespace map", and write the doctrine into `AGENTS.md`. Defer the `/plan:initiative → /plan:run` rename to a follow-up, since it is a skill rename rather than a namespace reorganisation and can land independently.
