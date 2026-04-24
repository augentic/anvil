# RFC-3a: Monolith Migration Planning

> Status: Implemented · Depends: [RFC-1](rfc-1-cli.md), [RFC-2](rfc-2-execution.md)
>
> **Supersession note.** The `scope` and `affects` structured fields on plan entries, introduced by §*The `scope` field* and §*How `scope` travels through the pipeline*, have been removed. Scope and delta-targeting intent are now carried in the `description` field as prose and inferred by the define skill at execution time. The three-hop flag-forwarding pipeline (plan.yaml → execute → define → extract) is replaced by description-driven inference in the specs brief. Extract's native `--include`/`--exclude`/`--manifest` flags are preserved; the change is about *who decides* those values. See the plan schema README for current field documentation.

## Abstract

Extend RFC-2's `/spec:plan` into a **registry-aware** authoring skill that scales unchanged from a single repo to 100+ repos.

Two new, optional files scope the initiative. `.specify/registry.yaml` is the **platform catalogue** — the repos that comprise the system and the schemas they use. `.specify/initiative.md` is the **operator-authored brief** — intent as prose, seed inputs as YAML frontmatter. The registry answers "what's in the platform?"; the brief answers "what am I trying to do this cycle, and from what material?". Either or both may be absent: a bare `/spec:plan` with CLI-only inputs remains valid, and a single-repo system needs no registry at all.

Internally, `/spec:plan` runs a fixed three-phase flow: *analyse inputs* → *(sync peers)* → *generate plan*. The middle phase runs automatically when `registry.yaml` declares more than one project, and clones every listed repo into `.specify/workspace/<project>/` for read-only inventory. Discovery dispatches every input to `/spec:analyze` — a new skill introduced by this RFC — which branches internally on `kind` (`legacy-code` or `documentation`) and emits a single merged `discovery.md` of **capability summaries** (name, one-line summary, `sources:` file-hint list, `depends-on:`, confidence); any other kind is a hard error. `/spec:extract` (unchanged from RFC-2) moves to `/spec:define` time, where it runs per-change against the scope each change owns. `/spec:plan` remains the single operator-facing entry point — the invocation shape, core loop, and single-writer invariant are unchanged.

There is no pipeline declaration file. RFC-3 does not move `pipeline.plan` anywhere: it eliminates the configuration surface entirely for v1 so the fixed shape is the only shape. `schema.yaml` retains its Layer 1 role (`define/build/merge`); no planning config lives there. Configurability is re-addable in a later RFC without migration (see *Alternatives Considered*).

RFC-3 addresses initiative planning across repos. Cross-repo spec references and contract validation are an execution-time concern that sits downstream of the planning flow introduced here. They are explicitly out of scope for RFC-3 and captured separately in [RFC-3b](rfc-3b-platform.md).

In parallel, RFC-3 scales `/spec:plan` *down* into large monoliths by splitting the code-handling pipeline across two skills — a cheap whole-source analysis via `/spec:analyze` at plan-authoring time (emitting capability summaries whose `sources:` lists propose scope directly), and a deep per-slice extraction via `/spec:extract` at `/spec:define` time — and by adding an optional `scope` field to plan entries so each change's define runs against only the files that change owns. See §*Large-Monolith Decomposition*.

## Planning Model Overview

Specify planning model

`/spec:plan` runs a fixed internal flow. For single-repo initiatives the flow is *analyse inputs → generate plan*; for multi-repo initiatives (registry declares more than one project) a *sync peers* phase is inserted between them. The three diagram phases correspond to:

1. **Analyse inputs.** Read seed material — legacy code, documentation — and produce a **capability inventory**. Dispatches every input to `/spec:analyze`, which branches internally on the declared `kind` (`legacy-code` or `documentation`) and emits capability summaries — name, one-line summary, `sources:` file-hint list, `depends-on:` capability edges, and a `confidence` marker — into `discovery.md`. Both branches share one output shape; the propose brief downstream does not distinguish which branch produced a given capability.
2. **Sync peers.** *(Runs if* `registry.yaml` *declares more than one project.)* Clone every project declared in `registry.yaml` into `.specify/workspace/<project>/` (local, read-only cache), then inventory each repo's existing `.specify/` tree (baseline specs, in-flight plans, schema). Emits `workspace.md`.
3. **Generate plan.** Combine the input inventory (`discovery.md`) and — when present — the peer inventory (`workspace.md`) into the **Plan**: the ordered, dependency-aware list of changes RFC-2 drains with `specify initiative next`. Emits `plan.yaml`.

The `Plan` box in this diagram is the same `Plan` box on the left of RFC-2's execution diagram. Planning produces it; execution consumes it and amends it back.

The flow is fixed: phases run in order, and the *sync peers* phase is present if and only if `registry.yaml` declares more than one project. There is no operator-visible step ID surface, no configuration file, and no auto-insert policy to diagnose. The diagram labels (`discovery`, `workspace`, `propose`) and artifact filenames (`discovery.md`, `workspace.md`) retain those names for continuity; the RFC prose uses the activity-oriented descriptions above.

### Diagram labels → skills and CLI


| Diagram label                    | Phase / skill                            | CLI                                                                         |
| -------------------------------- | ---------------------------------------- | --------------------------------------------------------------------------- |
| `plan` (centre)                  | `/spec:plan` (registry- and brief-aware) | `specify initiative {init, create, amend, validate}` (unchanged from RFC-2) |
| `registry.yaml` (read)           | —                                        | `specify initiative registry {show, validate}`                              |
| `initiative.md` (read)           | —                                        | `specify initiative brief {init, show}`                                     |
| Step ① — analyse inputs          | Discovery → `/spec:analyze`              | — (phase reads `--from` / `--against` / `--source` + brief inputs)          |
| Step ② — sync peers              | Workspace (CLI-driven)                   | `specify initiative workspace {sync, status}`                               |
| Step ③ — generate plan           | Propose                                  | `specify initiative create` (per accepted slice; unchanged from RFC-2)      |
| `Inputs` box (legacy code, docs) | —                                        | — (filesystem paths under `--from` / `--source` or `initiative.md:inputs`)  |
| `Workspace` box (cloned repos)   | —                                        | `.specify/workspace/<project>/`                                             |
| `Plan` box (output)              | —                                        | `.specify/plan.yaml` (RFC-2 format, unchanged)                              |

CLI verb names listed above are normative for this RFC. They have not yet shipped in `specify-cli` but are the contract new code under RFC-3 should implement against; a future CLI RFC may rename them, with migration notes at that point.


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

RFC-3 is structured in two layers; each layer describes a feature added to the planning flow. **Layer 1** makes `/spec:plan` registry-aware and fixes the discovery-dispatch rule for code vs. documentation inputs. **Layer 2** adds the *sync peers* phase. A parallel §*Large-Monolith Decomposition* adds the two-skill analyze/extract split (plan-time vs define-time) and the `scope` field; it composes with both layers but sits outside their progression. Federation at execution time (`@peer:capability` references, contract reconciliation, peer status roll-up) is deferred to [RFC-3b](rfc-3b-platform.md).

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

A formal JSON schema for `registry.yaml` is deferred for v1 — the example above is normative, and `specify initiative registry validate` is expected to enforce that shape directly until a schema file lands alongside `plan.schema.json`. Same posture for `initiative.md` frontmatter (§*The Initiative Brief* below).

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


| `kind`          | Dispatch target | Purpose                                                                                                               |
| --------------- | --------------- | --------------------------------------------------------------------------------------------------------------------- |
| `legacy-code`   | `/spec:analyze` | Infer capability summaries from code structure (name, `sources:` file-hints, `depends-on:`, confidence).              |
| `documentation` | `/spec:analyze` | Extract capability summaries from prose, PDFs, runbooks, and API docs (alongside constraints and open questions).     |


The `kind` vocabulary is a **closed enum** for v1. An input with any other `kind` is a hard error at the *analyse inputs* phase; extending the vocabulary requires a new skill and an RFC update. This keeps discovery auditable ("which skill produced this line of `discovery.md`?") and forces any future ambiguity in source material to be resolved at the RFC level rather than by a generic fallback.

`/spec:analyze` is **new in RFC-3** and is the sole plan-time discovery skill. It accepts both code and documentation inputs, branches internally on `kind`, and contributes a merged **capability inventory** to `discovery.md`. Both branches emit the same shape — capability summaries carrying name, one-line description, `sources:` file-hint list (inferred from code clustering or extracted from prose references), `depends-on:` capability edges, optional structural hints (entry points, external dependencies), and a `confidence` marker — so the propose brief downstream does not distinguish which branch produced a given capability. For code inputs the clustering step (going from import graph + endpoint names + docstrings + READMEs to capability boundaries) is where output scaling lives: summaries grow with capability count (typically 10–40 for a large monolith), not LOC. For documentation inputs the extraction step identifies capabilities the docs describe, alongside the constraints and open questions the body carries. Both branches share one output artifact (`discovery.md`), one idempotency contract, and one fixture tree; the detailed per-kind clustering / extraction prompts are schema-owned and ship in the dispatching `schemas/<schema>/briefs/plan/discovery.md` brief. RFC-3 fixes only the skill boundary (one plan-time skill, one artifact), the output shape (capability summaries), and the closed `kind` vocabulary.

**On-disk shape of capability summaries.** Each capability is emitted as a markdown heading (`### <capability-name>`) followed by a fenced YAML block carrying the structured fields (`summary`, `sources`, `depends-on`, `hints`, `confidence`). The heading keeps `discovery.md` human-scannable with existing tools; the YAML block makes the fields mechanically parseable by the propose brief without markdown-specific heuristics. See §*Plan-time analysis, define-time extraction* for the full example.

`/spec:extract` (RFC-2 Layer 3, unchanged) moves to `/spec:define` time. It produces `specs/<change>/` + `design.md` for a single change, scoped by the change's `scope` field. See §*Large-Monolith Decomposition* for the plan-time / define-time split.

### `--source` flags and the brief

`/spec:plan`'s existing `--source <key>=<path-or-url>` / `--from` / `--against` flags continue to work unchanged. They're additive with `initiative.md:inputs` — the *analyse inputs* phase reads both and merges them into `discovery.md`. Inputs supplied via CLI flags may carry an explicit kind (e.g. `--source legacy=./old/:legacy-code`) so they route through the same closed-enum dispatch as brief inputs. **A `--source` with no `:<kind>` suffix defaults to `kind: legacy-code`**, preserving RFC-2 `--source` semantics (every existing `--source` call site today is legacy code). `--from` defaults to `kind: documentation`; `--against` defaults to `kind: legacy-code`.

### The flow

```
analyse inputs ──▶ generate plan ──▶ plan.yaml
```

Unchanged from today. The *sync peers* phase is absent because `registry.yaml` either is absent or declares a single project.

---

## Large-Monolith Decomposition

A deep extraction over a whole monolith via `/spec:extract` is intractable: for 100k+ LOC with dozens of modules, the output overflows context, the resulting specs are unwieldy, and the human has no chance to draw slice boundaries before the extractor commits to them. RFC-3 solves this by keeping plan-time discovery cheap (`/spec:analyze` emits capability summaries — name, source-file hints, dependencies, confidence — not full specs) and deferring deep extraction (`/spec:extract`) to `/spec:define` time, where it runs per-slice against the scope each change owns.

This section is orthogonal to the Layer 1 / 2 progression. It extends Layer 1's discovery dispatch and the Plan schema, and composes with Layer 2's *sync peers* phase (per-peer decomposition works identically). It introduces no new operator-facing skill: `/spec:plan` remains the sole human entry point.

### Plan-time analysis, define-time extraction

The monolith case is handled by splitting plan-time and define-time across two different skills:

1. **`/spec:analyze` (plan-time, whole monolith).** Reads the whole code tree and emits a **capability-level inventory** — one entry per inferred capability, carrying a name, one-line summary, `sources:` file-hint list (the files the capability appears to inhabit), `depends-on:` capability edges, optional structural hints (entry points, external dependencies), and a `confidence` marker. Not full specs. Output size grows with capability count (typically 10–40 for a large monolith), not LOC. The code-branch output contract matches the documentation-branch contract by construction — both emit capability summaries — so `discovery.md` has one shape regardless of which inputs produced a given section, and the propose brief's downstream logic is unified. Example entry:

    ```yaml
    capabilities:
      - name: user-registration
        summary: Create new user accounts with email verification.
        sources:
          - src/users/register.ts
          - src/users/validation.ts
          - src/auth/verify.ts
        depends-on: [email-verification, shared-validation]
        hints:
          entry_points: [POST /users]
          external_deps: [postgres, sendgrid]
        confidence: high
    ```

    The `sources:` list is the key design choice: it carries the capability-to-file mapping the skill inferred during clustering. Propose uses it directly as the proposed `scope.<src>.include` without re-running a clustering pass. The `confidence` marker (`high` / `medium` / `low`) surfaces inference uncertainty — a `low` capability has structural ambiguity the human should review (often a candidate for the manifest escape hatch, §*Manifest shape*, rather than a clean glob-based slice).

2. **`/spec:extract` (define-time, one slice).** Runs per-change under `/spec:execute`, scoped to the files the change owns via its `scope` field. Produces `specs/<change>/` + `design.md` for that slice only. No single invocation ever sees more than one slice's worth of source.

The split is on the skill boundary, not a mode flag: the discovery brief under `/spec:plan` calls `/spec:analyze` on every input; the define pipeline under `/spec:execute` calls `/spec:extract` on the current change's scope. Neither skill has a mode flag that changes its output shape — `/spec:analyze` always emits capability summaries into `discovery.md`, `/spec:extract` always emits `specs/` + `design.md`. The detailed clustering algorithm inside `/spec:analyze` — how import graphs, endpoint names, docstrings, test names, and README signals combine into capability boundaries with a confidence score — is **schema-owned**: the Omnia discovery brief carries Omnia's clustering rules; another schema ships its own. RFC-3 fixes only the skill boundary and the output shape (capability summaries with `sources:` + `depends-on:` + `confidence`).

**Why capabilities, not modules.** A module-level inventory (entry points, import-graph edges, candidate capability hints as annotations) is an alternative shape that satisfies the output-size constraint, but it forces the propose brief to cluster modules into capabilities as a separate downstream pass, with a different heuristic per schema, and leaves `discovery.md` carrying two different shapes for code vs documentation inputs (modules for code, capabilities for documentation). Emitting capabilities directly from the code branch unifies the two input kinds, moves the schema-owned clustering judgment into the one place it belongs (the brief that prompts `/spec:analyze`), and makes the capability → slice transform in propose mechanical (1:1, `sources:` → `scope.include`) rather than a second inference pass. The cost is that clustering can be wrong; the `confidence` marker + propose's accept / edit / reject / abort loop is the mitigation. See §*Alternatives Considered — Module-level analyze output*.

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

- `scope.<source>.{include, exclude}` are glob lists — gitignore-style matching so operators see the semantics they already know — resolved relative to the top-level `sources[<source>]` path.
- `scope.<source>.manifest` is an escape-hatch pointer to a per-slice manifest file (see §*Manifest shape*). Mutually exclusive with `include` / `exclude` for the same source key.
- A change with no `scope` entry for a given source hands the whole source to `/spec:extract` — behaves exactly as today. Backwards compatible and valid indefinitely: "no scope" means "the whole `sources[<key>]` tree is this slice". Making scope mandatory for legacy-code sources would break small-legacy changes, break pre-RFC-3 plans on amend, and make the Stage A ramp (humans author scope by hand) harder; the monolith-scale lint under §*Validation* captures the soundness concern without a hard rule.
- A change with a `scope` hands only the filtered subset to `/spec:extract`.

The field lands as a structural extension to `plan.schema.json` and to the RFC-1 Plan library types (an additional `BTreeMap<String, Scope>` on `PlanChange`, where `Scope` carries `include`, `exclude`, and `manifest`). Because `plan.schema.json` is strict (`additionalProperties: false`), the schema update MUST land in the same change as the library type addition — a plan written with `scope` will fail validation until the schema accepts the new field. No semantic changes to `sources`, `depends-on`, or `affects`; no change to the status state machine.

### How `scope` travels through the pipeline

`scope` passes from `plan.yaml` to `/spec:extract` along a three-hop path, with each layer holding responsibility for one thing: the CLI / driver owns parsing and flag plumbing, the schema's define brief owns multi-source merge strategy, and `/spec:extract` owns the filter natively. Phase skills never read `plan.yaml` directly.

**`/spec:execute` grows three repeatable flags**, symmetric with the existing `--source` and `--affects`, that it forwards unchanged to `/spec:define`:

```text
/spec:define <name>
  [--source <key>=<path-or-url> ...]
  [--affects <change-name> ...]
  [--scope-include <key>=<glob> ...]             # NEW
  [--scope-exclude <key>=<glob> ...]             # NEW
  [--scope-manifest <key>=<manifest-path> ...]   # NEW, mutually exclusive
                                                 # with include/exclude per key
```

Resolution per plan entry:

- For each `scope.<key>.include` glob, emit one `--scope-include <key>=<glob>`.
- For each `scope.<key>.exclude` glob, emit one `--scope-exclude <key>=<glob>`.
- For each key carrying `scope.<key>.manifest`, emit one `--scope-manifest <key>=<path>`. The path is passed through as-is; resolution is the brief's / extract's concern.
- Keys present in `scope` but missing from the entry's `sources` list are a Config-level halt (see §*Validation*).
- Keys in `sources` without any `scope` entry emit zero scope flags — the back-compat path.

The driver never reads scope semantics; this is pure string forwarding alongside the existing `--source` and `--affects` rows in `/spec:execute`'s *Argument resolution* table.

**The schema's define brief iterates per source** and reconciles outputs into one `specs/` tree. `/spec:extract` stays single-root — a multi-root extract would double the fixture / test surface, compound language-detection edge cases across heterogeneous source manifests, and the merge rule has to live somewhere regardless. Keeping it in the schema brief keeps extract's contract tight. The Omnia brief's loop:

```text
for each --source <key>=<path>:
    scope_flags = collect --scope-{include,exclude,manifest} <key>=...
    /spec:extract <path> <change-dir>/.extract/<key>/ {scope_flags}
merge <change-dir>/.extract/<key>/specs/   -> <change-dir>/specs/
merge <change-dir>/.extract/<key>/design.* -> <change-dir>/design.md
```

`.extract/<key>/` is a per-source scratch directory under the change dir; the brief decides whether to clean it up after merge or leave it for debugging. Merge strategy is schema-owned: name collisions across sources (e.g. two extracts both producing `specs/user-registration/spec.md`) are resolved by the schema's merge rules. Omnia's initial rule is that the propose brief should already have forced distinct names or consolidated duplicates under one source; otherwise collision is a brief-level error.

**`/spec:extract` grows three native filter flags** that the brief translates the `--scope-*` flags for the current source key into:

```text
/spec:extract <source-path> <change-dir>
  [--include <glob> ...]
  [--exclude <glob> ...]
  [--manifest <manifest-path>]   # mutually exclusive with --include/--exclude
```

Globs resolve relative to `<source-path>`. Empty filter ≡ today's behaviour, unchanged for small-legacy / greenfield callers. The filter lives inside extract — not as a symlink farm, not as a file-list-only input — because `/spec:extract`'s own Step 1 (*Identify Component Structure*) depends on the real root layout for source-language detection, manifest files (`package.json`, `go.mod`, `Cargo.toml`, …), and lock-file dependency-version pinning. A staged filtered tree breaks those heuristics; native filters keep the root intact and only restrict *which source files are read for business-logic extraction*.

**Sentinels always read.** Extract reads a fixed set of files regardless of the filter, for language / dependency detection:

- `package.json`, `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`
- `Cargo.toml`, `Cargo.lock`
- `go.mod`, `go.sum`
- `pyproject.toml`, `poetry.lock`, `requirements.txt`
- `pom.xml`, `build.gradle[.kts]`, `gradle.lockfile`
- `*.csproj`, `packages.lock.json`
- top-level `README*`

`include` cannot subtract sentinels; `exclude` cannot hide them. Scope filters *business-logic extraction*, not manifest discovery.

### Manifest shape

For tangled monoliths where globs cannot cleanly separate slices, `scope.<source>.manifest` points at a per-slice manifest under `.specify/plans/<initiative>/slices/<change>.yaml`:

```yaml
    scope:
      monolith:
        manifest: .specify/plans/<initiative>/slices/ingest-pipeline.yaml
```

The manifest itself is a YAML file — structured data, not prose. v1 ships `include` only:

```yaml
# .specify/plans/<initiative>/slices/ingest-pipeline.yaml
version: 1
include:
  - src/ingest/api.ts
  - src/ingest/pipeline.ts
  - src/common/shared-helpers.ts
```

- Paths resolve relative to the corresponding `sources[<key>]` and name files directly — no globbing inside a manifest. Globs live in `scope.include` / `scope.exclude`; the manifest is the explicit enumeration used when globs cannot express the slice.
- `--manifest` is mutually exclusive with `--include` / `--exclude` for the same source key, validated at `specify initiative validate` time.
- Line-range subsets per file (e.g. `lines: [[1, 50], [120, 180]]`) are the natural v2 extension; a v1 manifest entry is a whole file.

Path-based `include` / `exclude` is the 95% case; the manifest is load-bearing only for the ~5% tangled slice where no glob cleanly describes the set of files that belong together — the same population the monolith-scale lint (§*Validation*) pushes operators toward.

### `--affects` composition with scope

Scope and affects are orthogonal. **Scope narrows which *source* files extract reads; affects redirects which *baseline* specs the brief writes against.** They compose via an explicit brief step, not by extract awareness — extract's contract remains "read source, emit reconstruction-grade specs" and it never reads `.specify/specs/`. Delta-form rewriting is post-processing the brief already knows how to perform (today's Modified-Crates branch in the Omnia `specs.md`); pushing baseline awareness into extract would couple two unrelated concerns.

When `--affects <name>` flags are present alongside `--source`:

1. Run each source's scoped `/spec:extract` into `.extract/<key>/` as in the per-source loop above.
2. For each capability the merged extract output covers whose name matches an `--affects <name>`, emit a DELTA spec block (ADDED / MODIFIED / RENAMED / REMOVED against the baseline at `.specify/specs/<name>/spec.md`) under `<change-dir>/specs/<name>/spec.md`, following the delta rules in the define skill's "Delta-specific workflows".
3. Capabilities whose names do NOT match any `--affects` are written as fresh new-crate specs (today's default).
4. `--affects <name>` values with no matching extract capability are a brief-level warning — either the baseline is untouched by this slice (drop the flag via `specify initiative amend --affects-rm`), or the scope is too narrow to see the file that would change the baseline behaviour (widen scope).

Worked fixture — the canonical `extract-shared-validation` case from RFC-2:

```yaml
changes:
  - name: extract-shared-validation
    sources: [monolith]
    affects: [user-registration, email-verification]
    scope:
      monolith:
        include:
          - src/common/validation/**
```

Driver invocation:

```text
/spec:define extract-shared-validation \
    --source monolith=./legacy/monolith \
    --affects user-registration --affects email-verification \
    --scope-include monolith=src/common/validation/**
```

Expected emissions:

- `<change-dir>/specs/user-registration/spec.md` — DELTA against `.specify/specs/user-registration/spec.md` (shared validation rules moved out).
- `<change-dir>/specs/email-verification/spec.md` — DELTA against baseline (same).
- `<change-dir>/specs/shared-validation/spec.md` — new-crate spec for the extracted validation capability itself.

### Validation

`specify initiative validate` (RFC-1) gains five scope-aware checks — two errors and three warnings:

- **Scope key matches sources** (`scope-key-not-in-sources`) — every key under `scope:` on a change must appear in that change's `sources:` list. A scope entry for a key not declared as a source has no sensible interpretation and halts the driver before it runs. *Error.*
- **Path existence** — every glob root in `include` (and every file path in a referenced `manifest`) must resolve under `sources[<key>]`. *Error.*
- **Monolith-scale without scope** — non-blocking lint: if `/spec:analyze` has classified `sources[<key>]` as monolith-scale (structural size above a schema-owned threshold; Omnia default: 10k+ LOC or ≥ 20 top-level modules) and the change carries no `scope` entry for that key, warn that define-time `/spec:extract` may overflow context. Driven by the structural metadata `/spec:analyze` emits alongside the capability inventory under `.specify/plans/<name>/analyze/<key>/`, so it only fires after discovery has run — small-legacy and greenfield changes never see it. Example diagnostic:

  ```text
  warn: change `ingest-pipeline` has sources[`monolith`] classified as
        monolith-scale by /spec:analyze (42 modules, 87k LOC) and no
        scope entry — define-time /spec:extract may overflow context.
        Run `specify initiative amend ingest-pipeline --scope-include
        'src/ingest/**'` to narrow the slice.
  ```

  *Warning.*
- **Overlap warning** — a file claimed by more than one change's scope is flagged. Not an error; shared code is a real case (see *Alternatives Considered*). *Warning.*
- **Orphan warning** — a file under any `sources[<key>]` claimed by zero changes is flagged. Useful as a "did I cover everything?" audit before merging. *Warning.*

### Propose-brief capability → slice mapping

With capability-shaped `discovery.md`, propose's slicing job collapses to a **1:1 map**: one plan entry per discovered capability, carrying `name` from the capability's `name`, `sources: [<source-key>]` from the dispatch key the capability came from, `scope.<source-key>.include` pre-filled from the capability's `sources:` list, and `depends-on` carried forward from the capability's `depends-on:` edges. Capability boundaries are decided upstream in `/spec:analyze`; propose turns them into plan-shape without a second clustering pass.

The Omnia propose brief extends this 1:1 map with its existing "one WASM crate per change" convention — capability names flow directly into change names, and the WASM-crate mapping happens at `/spec:define` time as today. The brief presents candidate slices — each with include paths (from `sources:`), `depends-on` edges, and the capability's `summary:` as the human-facing caption — to the operator via the existing accept / edit / reject / abort loop. **Low-confidence capabilities** (from `/spec:analyze`'s `confidence` marker) are surfaced with a "review before accepting" flag rather than silently shaping slice boundaries. Edit may move paths between slices before the slice is committed via `specify initiative create --scope-include ... --scope-exclude ...`.

**Tangled cases land as manifests, not globs.** Where a capability's inferred `sources:` list overlaps with another capability's, or where the files don't line up with a clean glob-expressible boundary, the brief emits a **manifest-based** slice (§*Manifest shape*) — writing the explicit file list under `slices/<change>.yaml` and setting `scope.<src>.manifest` on the plan entry instead of `scope.<src>.include`. Glob-based slices remain the 95% case; the manifest path exists for the tangled 5% that `/spec:analyze`'s low-confidence marker would have flagged.

The clustering algorithm inside `/spec:analyze` that produces capability boundaries in the first place is schema-owned (see §*Plan-time analysis, define-time extraction*); other schemas ship their own clustering rules. RFC-3 fixes only the propose contract: take capability-shaped `discovery.md`, produce slices with `scope` pre-filled.

### Working-directory additions

Extends the Layer 2 layout:

```
.specify/plans/<initiative>/
├── discovery.md        # /spec:analyze output (cheap, whole-monolith)
├── workspace.md        # Layer 2 only
├── proposal.md         # slice decisions
└── slices/             # NEW — one YAML manifest per tangled slice (optional)
    └── ingest-pipeline.yaml
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

**Stage A — the `scope` field and the flag plumbing.** Add `scope.<source>.{include, exclude, manifest}` to `plan.yaml`. Grow `/spec:execute` with three new repeatable flags — `--scope-include`, `--scope-exclude`, `--scope-manifest`, each keyed by source — that it forwards unchanged to `/spec:define`. Grow `/spec:extract` with three new native filter flags (`--include`, `--exclude`, `--manifest`) and the "sentinels always read" rule. Rewrite the Omnia define brief (`schemas/omnia/briefs/specs.md`) around two branches — source-driven and manual — with the per-source iteration that merges `.extract/<key>/` outputs into one `specs/` tree, and add the "Affects composition" step. Collapse `schemas/omnia/briefs/proposal.md`'s source-branching accordingly. Extend `specify initiative validate` with the `scope-key-not-in-sources` and path-existence diagnostics. Humans author scopes by hand. If Stage A ships before Stage B, the discovery brief keeps calling `/spec:extract` at plan time *temporarily* (the pre-RFC-3 shape); scope-aware `/spec:extract` is still exercised at define time. The call-site move from `/spec:extract` to `/spec:analyze` lands with Stage B. Unblocks real monolith work without touching propose.

**Stage B — `/spec:analyze` code branch emitting capabilities.** Ship the code-reading branch of `/spec:analyze` emitting capability summaries (name, one-line summary, `sources:` file-hint list, `depends-on:`, optional structural hints, `confidence`) in the same shape as the documentation branch. Update `schemas/omnia/briefs/plan/discovery.md` to invoke `/spec:analyze --kind legacy-code` for every code input (and `/spec:analyze --kind documentation` for doc inputs), dropping the plan-time `/spec:extract` call. Update `schemas/omnia/briefs/plan/proposal.md` to the 1:1 capability → slice mapping — each discovered capability becomes one plan entry with `scope.<src>.include` pre-filled from the capability's `sources:` list and `depends-on` carried forward from capability edges; low-confidence capabilities flagged for human review. The monolith-scale scope lint (§*Validation*) becomes meaningful because it reads the `/spec:analyze` output under `.specify/plans/<name>/analyze/<key>/`. `discovery.md` now scales to monoliths **and** propose emits slices with scope pre-filled — the auto-slicing story lands with Stage B, not a separate follow-up.

**Stage C — tangled-case manifest emission.** For capabilities whose inferred `sources:` lists overlap with another capability's, or whose structure doesn't permit clean glob-based scoping (the `confidence: low` cases from Stage B), the propose brief emits **manifest-based** slices (§*Manifest shape*) — writing the explicit file list under `slices/<change>.yaml` and setting `scope.<src>.manifest` on the plan entry instead of `scope.<src>.include`. Stage B's glob-based output handles the 95% case; Stage C handles the tangled 5% without requiring hand-authored manifests.

Stages compose with RFC-3's other features: Layer 2's *sync peers* phase runs unchanged (peer inventory is orthogonal to within-peer decomposition). Federation at execution time (RFC-3b, cross-repo reference resolution) remains orthogonal to slice scope when it eventually lands.

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

`.specify/workspace/` is `.gitignore`d by default — `specify init` appends `.specify/workspace/` to the project's `.gitignore` when it scaffolds `.specify/`, and `specify initiative workspace sync` asserts the entry is present (appending it if missing) before any clone writes land. The directory is rebuilt by `specify initiative workspace sync`; nothing else writes to it.

### Plan output shape

The *generate plan* phase emits a **single cross-repo `plan.yaml*`* in the initiating repo. Entries whose work spans peer projects reference them by registry name in `sources` / `affects`; execution of those entries requires the federation layer deferred to [RFC-3b](rfc-3b-platform.md).

Per-repo `plan.yaml`s linked by a feature manifest — staged under `.specify/plans/<initiative-name>/<peer>/` and delivered out-of-band — is a plausible alternative output shape, but it is deferred (see *Alternatives Considered*). RFC-3 ships with exactly one shape.

### CLI surface additions


| Operation                        | CLI                                             |
| -------------------------------- | ----------------------------------------------- |
| Scaffold / show initiative brief | `specify initiative brief {init, show}`         |
| Read / verify registry           | `specify initiative registry {show, validate}`  |
| Clone / refresh workspace        | `specify initiative workspace sync`             |
| Inspect workspace state          | `specify initiative workspace status`           |


These are all machinery the *sync peers* / *generate plan* phases shell out to; none are operator-facing entry points. `/spec:plan` remains the only command humans invoke.

### `--dry-run` and `--extend` under Layer 2

- `**--dry-run`.** The *sync peers* phase's read side may run (inventory whatever is already cloned) but MUST NOT clone new repos, write to `.specify/workspace/`, or write `workspace.md`. Mirrors the *analyse inputs* dry-run rule.
- `**--extend`.** When `.specify/workspace/` is already present, the *sync peers* phase reuses the existing clones — no new clone or fetch is performed — and regenerates `workspace.md` from the existing cache. Operators refresh peer clones explicitly via `specify initiative workspace sync` between runs; `/spec:plan --extend` never implicitly pulls from remotes. This keeps amend-style re-runs fast and deterministic, and makes the "is the workspace current?" question an explicit CLI action rather than a hidden side effect of `--extend`.

The single-writer invariant is unaffected: *sync peers* writes `workspace.md` under `.specify/plans/<name>/` and clones into `.specify/workspace/`; neither path touches `.specify/plan.yaml`.

---

## Relation to RFC-2

- RFC-2's `/spec:plan` skill is unchanged in invocation, core loop, working directory, and single-writer invariant. RFC-3's contribution is registry and initiative-brief awareness, a fixed internal *sync peers* phase that runs automatically when `registry.yaml` declares more than one project, and a new `/spec:analyze` skill that owns plan-time discovery for both code and documentation inputs; the skill picks up all three automatically. `/spec:extract` is otherwise unchanged from RFC-2 but moves from plan-time to `/spec:define` time, where it runs per-change against the change's `scope` field.
- RFC-2 Layer 3 placed `pipeline.plan` inside `schema.yaml`. RFC-3 does not relocate that declaration to any new file — it removes the configuration surface. `schema.yaml` returns to its Layer 1 role (per-repo `define/build/merge`). Stack-specific planning helpers may still ship in schema directories and be consumed directly by the *generate plan* phase; that is a separate concern from a pipeline-shape declaration, which no longer exists.
- The Plan format gains one optional field: `scope` on change entries (`scope.<source>.{include, exclude}` globs or a `manifest` pointer, mutually exclusive per source key) for large-monolith slicing. The field is plumbed through `/spec:execute` with three new repeatable flags (`--scope-include`, `--scope-exclude`, `--scope-manifest`, symmetric with `--source` and `--affects`), which `/spec:define` translates per-source into `/spec:extract`'s native `--include` / `--exclude` / `--manifest` filter flags. Changes without `scope` are unaffected and behave exactly as in RFC-2. RFC-3's only other semantic extension on the Plan itself is that `sources` and `affects` may reference peer projects by registry name (resolved via `registry.yaml`). See §*Large-Monolith Decomposition*.
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

An earlier draft gave `/spec:extract` a `--mode=survey` flag that produced discovery-shaped output (summaries) instead of spec-shaped output. Rejected: it overloads one skill with two output contracts (specs + design in default mode, capability summaries in survey mode), and it leaves the documentation-input case with a different skill, so the intended "one skill covering both tiers" simplification does not actually materialise. RFC-3 places the plan-time / define-time split on the skill boundary — `/spec:analyze` at plan time, `/spec:extract` at define time — rather than on a mode flag. The two skills have non-overlapping output artifacts (`discovery.md` vs `specs/` + `design.md`), which keeps each skill's contract sharp and avoids the category error of an "extract" that does not extract.

### Module-level analyze output

An earlier shape had `/spec:analyze`'s code branch emit a **module-level inventory** — entry points, outbound dependencies, import-graph edges between modules, candidate capability *hints* derived from docstrings, endpoint names, and READMEs — rather than capability summaries. Output size would scale with module count (not LOC), which satisfies the context-budget constraint that motivates the plan-time / define-time split in the first place. Rejected for two reasons. First, it leaves `discovery.md` carrying two different shapes for code vs documentation inputs (modules for code, capabilities for documentation), which the propose brief has to reconcile — the artifact is "one file" but not "one shape", and propose ends up with a code-branch slicing pass that does not apply to documentation inputs. Second, it moves capability inference into the propose brief as a downstream clustering pass (entry-point clustering, directory cohesion, LOC budget, etc.), duplicating the judgment that is already present on the documentation branch and splitting it across two surfaces per schema. Emitting capabilities directly from the code branch unifies the output contract, puts the schema-owned clustering judgment in one place (the `/spec:analyze` brief), and turns propose into a mechanical 1:1 transform (`sources:` → `scope.include`). The cost is that clustering can be wrong; the `confidence` marker on each capability surfaces uncertainty to the human reviewer in propose, preserving the existing accept / edit / reject / abort correction loop as the mitigation, and the manifest escape hatch (§*Manifest shape*) absorbs the tangled-file-boundary cases that globs cannot cleanly describe.

### `/spec:decompose` as the plan-time code-reading skill

An alternative naming for the plan-time code skill — `/spec:decompose` — was considered on the grounds that monolith breakdown is the skill's purpose. Rejected: the skill *identifies* capabilities, it does not produce the plan. Plan entries — their names, their `depends-on` edges as encoded in `plan.yaml`, their status state machine — are propose's output, not analyze's. Naming the skill after the downstream outcome (the decomposed plan) rather than its own output (capability inference with source-file hints) creates a readership trap where operators expect plan-shape output in `discovery.md` and are surprised to find a capability-summary list. `/spec:analyze` names what the skill does; plan shape remains the propose brief's concern.

### Symbol-level scope

A slice manifest could enumerate individual functions or classes, letting a slice claim "just these symbols from `main.ts`". Rejected for v1: the parse-and-index infrastructure required to resolve symbols across languages is large, language-specific, and likely to drift from whatever the extractor already does internally. File-level scope covers the overwhelming majority of real monoliths; tangled cases use the manifest pointer as a file-level escape hatch. A later RFC may revisit symbol-level if real data shows file-level is too coarse.

### Scope overlap as the default for shared files

An alternative to the dedicated shared-infrastructure change is to permit scope overlap and re-extract shared files in every consuming slice. RFC-3 permits this (as a *warning*, not an error) but prefers the explicit shared-change pattern because it surfaces the dependency in the plan topology — every consumer listing the shared slice in `depends-on` makes the coupling visible to the human reading the plan. A future `read-only-in-scope` designation that lets multiple slices read a file without re-extracting it is deferred.

## References

- [RFC-1: `specify` CLI](rfc-1-cli.md) — CLI surface this RFC extends.
- [RFC-2: Execution](rfc-2-execution.md) — consumer of the Plan this RFC produces; introduces the `/spec:plan` skill RFC-3 extends.
- [RFC-3b: Federation at Execution Time (Layer 3)](rfc-3b-platform.md) — execution-time cross-repo references, contract reconciliation, and peer status roll-up; out of scope for this RFC.

