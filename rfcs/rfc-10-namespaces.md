# RFC-10: Namespaces

> Status: Draft

## Abstract

Reorganise the plugins published from this repository so that each plugin maps to a single primary audience. Today the `spec` plugin houses ten skills whose users range from "developer running the daily phase loop" to "delivery lead authoring a Statement of Work" to "platform engineer bootstrapping a new project". This RFC splits those skills across four plugins — `spec`, `audit`, `change`, and `plan` — chosen so that anyone scanning the slash-command palette can predict which plugin owns a given task from the audience alone.

The change is structural: skill bodies, CLI verbs, schemas, and reference documents are unchanged. Migration is limited to the marketplace manifest, plugin directory layout, documentation cross-references, and the user-facing slash-command namespace.

## Motivation

### The namespace problem

The `spec` plugin currently bundles skills that serve distinct purposes:


| Purpose                                    | Skills                                       | Layer   |
| ------------------------------------------ | -------------------------------------------- | ------- |
| Configuration                              | `init`                                       | Layer 0 |
| Single-change, single-repo point solutions | `define`, `build`, `merge`, `drop`           | Layer 1 |
| Multi-change, multi-repo initiatives       | `initiative`, `plan`, `execute`, `[analyze]` | Layer 2 |
| Internal - used by other skills            | `extract`                                    |         |


A daily developer scrolling the `/spec:*` palette sees `extract` (one-time reverse engineering), `analyze` (planning aid), and `initiative` (cross-repo umbrella) interleaved with `define` and `build`. The cost is cognitive, not technical: nothing breaks, but every operator has to learn that `/spec:*` is a grab-bag rather than a coherent surface.

> **Note (2026-04-29).** The original RFC inventory included three additional Layer-1 skills — `status`, `explore`, and `verify` — that have since been removed from the `spec` plugin. The change reduced the audit bucket to a single skill (`extract`) and the meta bucket to a single skill (`init`); see §"Open question: should singletons still get their own plugin?" for the resulting design question.

Because the existing marketplace already segments by *output domain* (`omnia`, `vectis`, `contracts`, `rt`) and by *commercial output* (`plan` for SoW), the `spec` plugin is the only one that mixes audiences. Aligning `spec` with the rest of the marketplace closes that inconsistency.

### Why now

- The plugins are not yet broadly adopted. There is no muscle memory or downstream documentation to break, so the migration cost is bounded to docs in this repository plus the marketplace manifest.
- The skill set is approaching steady state at the level of "what a workflow looks like end to end." Future skills will plausibly extend each audience bucket (more audit verbs, more initiative-orchestration verbs) rather than introduce new audiences. Splitting now anchors those extensions.
- The CLI surface already groups verbs by audience (`specify change …`, `specify plan …`, `specify initiative …`, `specify registry …`). The skill layer is the only place where the audiences remain conflated.

### Non-goals

- **No CLI changes.** Every skill continues to shell out to the same `specify` verbs. The CLI noun groupings landed in v1 are unaffected.
- **No skill rewrites.** SKILL.md bodies, references, and authority hierarchies are unchanged.
- **No new skills.** Reorganisation only.
- **No reshuffle of `omnia`, `vectis`, `contracts`, `rt`.** Those plugins already pass the audience test.

## Detailed Design

### Plugin map


| Plugin   | Skills                                                   | Primary audience                           | Direction of work                                       |
| -------- | -------------------------------------------------------- | ------------------------------------------ | ------------------------------------------------------- |
| `spec`   | `init`                                                   | Anyone interacting with the project        | Project meta — bootstrap                                |
| `audit`  | `extract`                                                | Maintainer reconciling code with artifacts | Code → artifacts (backward)                             |
| `change` | `define`, `build`, `merge`, `drop`                       | Day-to-day developer                       | Spec → code (forward, per-change)                       |
| `plan`   | `analyze`, `plan`, `execute`, `initiative`, `sow-writer` | Initiative driver / delivery lead          | Above-a-single-change orchestration and delivery output |


After the split, `/spec:`* becomes a small, stable surface that any contributor is expected to know. `/change:`* is the daily palette. `/audit:`* and `/plan:`* are episodic palettes operators reach for in specific situations.

### Plugin descriptions

Each plugin's `marketplace.json` description should encode the audience explicitly so that browsing the marketplace surfaces the partition:

- `**spec**` — Project bootstrap: initialise a regular project or a registry-only platform hub.
- `**audit**` — Reconcile code with artifacts: extract Specify artifacts (specs, design) from existing source code.
- `**change**` — Drive a single change through its lifecycle: define artifacts, build the implementation, merge into baseline, or drop without merging.
- `**plan**` — Coordinate work above a single change: analyse, author and execute a plan, drive an initiative end-to-end, and produce Statements of Work from artifacts.

### Slash-command rename map


| Today              | After                                                                    |
| ------------------ | ------------------------------------------------------------------------ |
| `/spec:init`       | `/spec:init`                                                             |
| `/spec:extract`    | `/audit:extract`                                                         |
| `/spec:define`     | `/change:define`                                                         |
| `/spec:build`      | `/change:build`                                                          |
| `/spec:merge`      | `/change:merge`                                                          |
| `/spec:drop`       | `/change:drop`                                                           |
| `/spec:analyze`    | `/plan:analyze`                                                          |
| `/spec:plan`       | `/plan:plan`                                                             |
| `/spec:execute`    | `/plan:execute`                                                          |
| `/spec:initiative` | `/plan:initiative`                                                       |
| `/plan:sow-writer` | `/plan:sow-writer` (unchanged unless §"Open question" decides otherwise) |


The cross-plugin invocation graph is unchanged. `/plan:execute` calls into `/change:define → /change:build → /change:merge` exactly as `/spec:execute` does today; the dependency simply crosses a renamed boundary.

### Repository layout

```
plugins/
├── spec/
│   ├── README.md
│   ├── references/
│   ├── rules/
│   └── skills/
│       └── init/
├── audit/
│   ├── README.md
│   └── skills/
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
│       ├── initiative/
│       └── sow-writer/
├── omnia/      (unchanged)
├── vectis/     (unchanged)
├── contracts/  (unchanged)
├── rt/         (unchanged)
└── references/ (unchanged)
```

The shared `plugins/spec/references/` and `plugins/spec/rules/` directories follow the `spec` plugin and stay where they are. Skills in `audit`, `change`, and `plan` continue to reach into those references via the existing symlink convention; no reference content moves.

### Marketplace manifest

`.cursor-plugin/marketplace.json` gains the three new plugin entries (`audit`, `change`) and updates the `spec` and `plan` entries to reflect their narrowed scope. The relative ordering follows the operator journey: bootstrap (`spec`), daily work (`change`), situational reconciliation (`audit`), orchestration and delivery (`plan`), then the output-domain plugins (`omnia`, `vectis`, `contracts`, `rt`).

### Authority hierarchy and references

The authority hierarchy in `.cursor/rules/project.mdc` (SKILL.md > Specify artifacts > references > source > inference) is plugin-agnostic and unchanged. References shared across plugins stay under `plugins/spec/references/` and `plugins/references/`; access patterns are preserved.

### Documentation surface

Every `/spec:*` reference in the documentation corpus needs to be remapped exactly once:

- `.cursor/rules/project.mdc` and `AGENTS.md`
- `plugins/spec/README.md` (rewritten to cover the narrowed `spec` plugin) and new `plugins/{audit,change,plan}/README.md` files
- `docs/reference/cli/*.md`, `docs/how-to/*.md`, `docs/tutorials/*.md`, `docs/explanation/*.md`
- `docs/orientation/*.md` and `docs/index.md`
- `docs/SUMMARY.md`

The remapping is mechanical — every renamed slash command in the table above can be substituted by find-and-replace, then spot-checked. The `make checks` script gains an entry to flag remaining `/spec:{define,build,merge,drop,extract,analyze,plan,execute,initiative}` references after the split, so doc drift is caught in CI.

## Open question: should singletons still get their own plugin?

After the 2026-04-29 removal of `status`, `explore`, and `verify`, the `spec` plugin contains only `init` and the `audit` plugin contains only `extract`. Two plausible resolutions:

1. **Keep the singletons.** `spec` is still the natural home for project bootstrap, and a future skill (e.g. a project doctor or a hub-only inspection tool) is plausible. `audit` keeps headroom for code-as-input skills (e.g. a future contract drift checker that lives outside the change loop). Cost: two single-skill plugins in the marketplace.
2. **Fold `init` into `spec` by promoting `change` to absorb both.** Move `init` into `change` and drop the `spec` plugin entirely; move `extract` into `change` and drop `audit`. Reduces the plugin count from four to two (`change`, `plan`). Cost: `change` mixes "bootstrap a project" with "drive a single change," weakening the audience-purity argument that motivated the split.

Recommendation: keep the singletons. The cognitive cost of a one-skill plugin is low (it shows up in the marketplace as a small dedicated entry, exactly matching its purpose) and the cost of mixing audiences in `change` is the original problem this RFC is trying to fix. If the singletons stay isolated for a release or two and no new skills land, revisit the question then.

## Open question: where does `sow-writer` live?

`sow-writer` is the only skill whose audience-fit is debatable.

- **Workflow planning** (`analyze`, `plan`, `execute`, `initiative`) is engineering coordination. Audience: tech lead, initiative driver, automation. Reads/writes `plan.yaml`, `initiative.md`, `registry.yaml`.
- **Commercial planning** (`sow-writer`) is client-facing scoping. Audience: delivery lead or partnership owner. Reads artifacts and emits SoW prose.

Three resolutions are on the table:

1. **Keep `sow-writer` in `plan`.** Justified when the same role authors both `plan.yaml` and the SoW. In that case "above-a-single-change planning artifacts" is a single audience and the bundling is correct. This is the default assumed by the plugin map above.
2. **Move `sow-writer` to its own plugin** (e.g. `delivery` or `sow`). Strict audience purity. Cost: one small plugin in the marketplace. Benefit: room to grow into `proposal-writer`, `pricing-writer`, etc., without polluting `plan`.
3. **Rename the workflow plugin** (e.g. `initiative` or `orchestrate`) and let `plan` keep the SoW skill. Inverts the question: does the workflow side or the commercial side keep the `plan` name? This is more invasive than option 2.

The recommendation is to defer this question to whoever owns the delivery workflow. If the same role authors both, choose option 1; otherwise choose option 2. Either way the rest of this RFC is unaffected.

## Smaller design observations

- `**audit` is now a singleton.** With `verify` removed, `audit` contains only `extract`. The plugin name still fits — `extract` treats code as ground truth and produces artifacts from it — but the namespace is one skill wide. See §"Open question: should singletons still get their own plugin?" for the trade-off.
- `**/change:build` reads as "build the change."** Acceptable and consistent with the existing CLI verb `specify change`. Considered and rejected: `/change:implement`. Symmetry with `define`/`build`/`merge`/`drop` outweighs the slight verb-noun ambiguity.
- `**analyze` and `extract` share DNA.** Both read source code; one emits capability summaries (`discovery.md`), the other emits full artifacts. They land in different plugins under this scheme because their *callers* differ: `analyze` is invoked from plan authoring; `extract` is a standalone bootstrap. Future maintenance should keep that boundary — `analyze` should not grow into `extract`'s territory.
- `**/plan:initiative` is acceptable but slightly redundant.** The Layer 4 umbrella skill is bigger than its current name suggests. A future rename to `/plan:run` or `/plan:drive` would read more naturally; that is out of scope for this RFC because it is a skill-level rename rather than a plugin reorganisation.

## Alternatives considered

### A. Status quo with internal grouping

Keep all ten skills in `spec` and reorganise the README into the four buckets above. Zero migration cost. Rejected because it leaves the slash-command palette unchanged and therefore does not address the audience-mixing pain that motivates this RFC.

### B. Carve out only the initiative skills

Two plugins: `spec` (everything single-change plus bootstrap) and a new `initiative` plugin (`analyze`, `plan`, `execute`, `initiative`). Targets the largest single audience boundary. Rejected because it leaves `extract` and the per-change phase loop in the same plugin as `init` — two distinct audiences still co-located.

### C. Two-plugin split: `plan` and `exec`

The original framing for this work proposed a `plan` plugin for planning skills and an `exec` plugin for define/build/merge. Rejected for three reasons:

1. **Name collision.** `plan` is already the SoW plugin; reusing the name conflates engineering coordination with commercial output.
2. **Unallocated skills.** `init`, `extract`, `drop` do not cleanly land in either bucket.
3. `**/exec:execute` is awkward** and `execute` is itself a planning-loop driver — it belongs with `plan`, not `exec`.

### D. Audience-driven three-way split

`spec` (daily verbs), `audit` (situational inspection), `plan` (initiative-level). Functionally the same as this RFC's proposal minus the `change` carve-out. Rejected because the day-to-day developer is by far the largest audience and deserves its own dedicated palette; mixing daily verbs with bootstrap skills under `spec` re-creates the palette-grab-bag pattern at smaller scale.

## Migration plan

1. **Author the new plugin directories** under `plugins/audit/`, `plugins/change/`, and update `plugins/plan/`. Move skill directories — no content edits.
2. **Update `marketplace.json`** with the new plugin entries and revised descriptions.
3. **Rename slash-command references** across `docs/`, `.cursor/rules/`, `AGENTS.md`, and per-plugin READMEs using the rename map in §"Slash-command rename map".
4. **Add a `make checks` invariant** that fails CI on any remaining `/spec:{define,build,merge,drop,extract,analyze,plan,execute,initiative}` reference outside the migration map itself.
5. **Update `make dev-plugins` and `make prod-plugins`** if their symlink targets enumerate plugin directories.
6. **Bump the marketplace version** in `.cursor-plugin/marketplace.json` and announce the rename in the changelog.

The migration is a single change with five-to-six tasks under it. No CLI verbs, no schemas, and no skill bodies need to move; the work is dominated by mechanical renames and is suitable for a single `/spec:define → /spec:build → /spec:merge` cycle (using today's slash commands, before the rename takes effect).

## Recommendation

Adopt the four-plugin split (`spec` / `audit` / `change` / `plan`) with the skill mapping in §"Plugin map". Resolve the `sow-writer` placement question per §"Open question" before merging the change. Defer the `/plan:initiative → /plan:run` rename to a follow-up, since it is a skill rename rather than a plugin reorganisation and can land independently.