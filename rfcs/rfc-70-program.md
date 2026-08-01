# Migration Walking Skeleton

> Status: Draft — nothing landed
>
> Owns: one implementation cut — adapter descriptors, durable source intake, and a serial migration program coordinator.
>
> Absorbs: archived [descriptor](archive/rfc-71-discovery.md) and [intake](archive/rfc-72-migration.md) drafts. Supersedes: [archive/rfc-21-catalogue.md](archive/rfc-21-catalogue.md), [archive/rfc-22-ledger.md](archive/rfc-22-ledger.md).
>
> Depends on: [RFC-71](rfc-71-deployment.md) install (already landed). Defers: [RFC-72](future/rfc-72-materialization.md) (managed slots — operator-prepared slots suffice first).

## Intent

An operator hands Emery a repository list once. Emery profiles each input, recommends source and target adapter pins from a typed descriptor index, and — after one Gate M1 approval — works repository at a time through the existing change → refine → build → merge loop.

Three former RFCs are one cut because none delivers the story alone: descriptors without intake cannot recommend, intake without a program gate cannot schedule, and a coordinator without both has nothing to approve.

Installation is already solved ([RFC-71](rfc-71-deployment.md) / [RFC-76](archive/rfc-76-adapter-install.md)). What is missing is the fact that decides *which* pin to install, the durable source list, and the serial coordinator.

## Operator loop

Target surface, not implemented:

```text
emery source add … / import          # repository list, once
emery source profile                 # deterministic repo profiles
emery program recommend              # source + target candidates
emery program approve                # Gate M1 — topology + adapters
emery program next                   # claim the next repository
  emery init <target> --platforms …  #   apply approved topology
  emery plan author --source @key …  #   lowered bindings, exits pending
  emery plan approve                 #   Gate 1, operator-only
  emery plan next → refine → build → merge
emery program status                 # durable progress and re-entry
```

---

## Part A — Adapter descriptors

Typed `AdapterDescriptor` substrate so engine core does not hard-code adapter names. Source selection ("what can inspect this input?") and target selection ("what should this project become?") share the substrate but keep different policy owners (Parts B and C).

### Two descriptor faces

- **source — `inspects`**: languages, manifest sentinels, framework idioms, recognised workload kinds.
- **target — `produces`**: workload kinds the adapter can build.

Illustrative shape, authored beside each adapter in `augentic/emery-adapters`:

```yaml
# sources/typescript/descriptor.yaml
axis: source
name: typescript
emery-floor: 0.28.0
inspects:
  languages: [typescript, javascript]
  manifests: [package.json, tsconfig.json]
  frameworks: [express, fastify, nestjs, next, bullmq, node-cron]
  workloads: [service, web-frontend, batch, cli]
```

```yaml
# targets/vectis/descriptor.yaml
axis: target
name: vectis
emery-floor: 0.28.0
produces:
  workloads: [mobile-app]
```

Platforms stay off the descriptor. A target's `PlatformsCapability` is decided after approval, when the component is installed and its `metadata` export is authoritative (`project-platforms-required` already exists at init).

### Workload kinds

Closed vocabulary — the join key between a repository profile and a target's `produces`:

| Kind | Meaning |
| --------------- | -------------------------------------------------------------------------- |
| `service` | Request/response or message-handling backend |
| `web-frontend` | Browser-delivered UI |
| `mobile-app` | iOS / Android application |
| `library` | Reusable code unit with no independent deployment surface |
| `batch` | Scheduled or queue-driven work with no synchronous caller |
| `cli` | Operator-invoked command surface |

Workload-neutral adapters (e.g. `contracts`) declare no `produces.workloads` filter and are always candidates.

### Decisions (A)

| # | Decision | Consequence |
| - | -------- | ----------- |
| A1 | Descriptor authored beside its adapter; projected into a static index shipped with the host. | Discovery never downloads a candidate to ask what it is for. |
| A2 | Keep the `describe` WIT operation deferred. | `metadata` stays post-install authority; the index is the pre-install projection. |
| A3 | Restated fields (`emery-floor`, …) are advisory in the index and authoritative in `metadata`; the publish gate asserts agreement. | Drift is caught at publish ([RFC-77](rfc-77-release-process.md)), not at resolve. |
| A4 | Two axis-specific predicates (`inspects` / `produces`) over one shared vocabulary. | Source and target selection reuse the filter kernel without sharing a policy owner. |
| A5 | Filtering is deterministic and total: every catalogued adapter is a candidate or excluded with a recorded reason. | Operators can tell "no adapter reads Java" from "floor above host". |
| A6 | `emery-floor` participates in filtering. | Un-dispatchable candidates are excluded up front. |
| A7 | Discovery emits an immutable recommendation report and stops — never installs, binds, or writes config. | Approval is Gate M1 (this RFC) / Gate 1 for ordinary plans. |
| A8 | The report records profile digest, index revision, and host version; invalidate when any change. | Stale recommendations cannot be approved after the profile moves. |
| A9 | First index covers the `emery:` namespace only. | Third-party descriptors and namespace trust stay deferred, gated on RM-21. |

---

## Part B — Intake and source selection

Durable source membership separate from target projects (`registry.yaml`) and per-change bindings (`plan.yaml`).

### Intake shape

`sources.yaml` at the platform-repo root, sibling to `registry.yaml`. Missing file is inert (same posture as the registry). Inputs are not all code: design-document trees, screenshot sets, and capture trees are first-class beside legacy repositories — the same profile-then-recommend loop routes them to `documentation`, `screenshots`, and `captures`.

### The profiler

Engine-side, deterministic, model-free — it must run *before* any adapter is chosen. Reads manifest sentinels and a file census (`package.json`, `go.mod`, `pom.xml`, `build.gradle`, `*.csproj`, `Cargo.toml`, `pyproject.toml`, `requirements.txt`, `Gemfile`, `composer.json`) and emits a repository profile: languages by weight, manifest evidence, framework hints, candidate workload kinds from Part A, and input kind (`code` / `documentation` / `images` / `captures`). Byte-stable for a given tree.

### Decisions (B)

| # | Decision | Consequence |
| - | -------- | ----------- |
| B1 | `sources.yaml` is durable source membership; the CLI is its single writer. | Lists stop being retyped per change. |
| B2 | Profiler is engine-side, deterministic, and model-free. | Recommendations are reproducible and diffable. |
| B3 | Intake covers non-code inputs via input kind on the profile. | Docs / screenshots / captures participate in selection. |
| B4 | Selection is profile → descriptor filter → recommendation → operator approval → exact pinned binding. | Operator approves names and pins once per input. |
| B5 | First cut recommends one source adapter per profiled input. | Multi-binding auto-composition stays deferred (hand-declare still works). |
| B6 | Approved bindings install through existing pull-on-miss. | Intake adds no download path or registry. |
| B7 | Approved bindings lower into `plan.yaml.sources` via `emery plan author` with an `@key` selector. | Gate 1 still reviews the authored plan. |
| B8 | Source snapshots are immutable, out of tree, and never the target slot. | Evidence integrity when a repo is both source and target ([RFC-72](future/rfc-72-materialization.md)). |
| B9 | Plan-time survey stays serial in this cut. | Matches Part C's repository-at-a-time coordinator; `--jobs` deferred. |

---

## Part C — Program coordinator

Migration-sized umbrella above changes. Each work item still uses the existing change → slice loop; the program schedules batches and records progress.

### Target selection

Source selection asks "what can read this repository?" (Part B). Target selection asks "what should this repository become?" — the two must not collapse. The program derives candidate workload kinds from the profile, filters targets by `produces` (Part A), and presents a recommendation. A profile that looks like an Express monolith makes `service` the default workload kind; it does **not** decide the migrated result stays a service. Operator intent at Gate M1 wins.

### One target adapter per target repository

`project.yaml.adapter` stays singular. Two workloads in one tree is a **topology decision**: at Gate M1 the operator picks one workload or splits into two registry projects. The program never binds two targets to one project and does not schedule until that decision is recorded.

### Applying approved topology

A newly scheduled target may lack `.emery/project.yaml`. The program proposes name, exact target pin, and platforms; the operator approves at M1; application runs through `emery init` — the program writes nothing itself. An existing `project.yaml` is authoritative and is never rewritten.

### Decisions (C)

| # | Decision | Consequence |
| - | -------- | ----------- |
| C1 | One target adapter per target repository. | No multi-target project shape. |
| C2 | Two candidate workload kinds block at Gate M1 until split or pick-one. | Ambiguity surfaces once, at approval. |
| C3 | Target selection consumes profile workload kinds plus operator intent; intent wins. | Source shape is not desired target architecture. |
| C4 | Approved topology applied through `emery init`, never by program file writes. | One writer for `project.yaml`. |
| C5 | Gate M1 approves topology and adapter decisions only. | Gate 1 and slice lifecycle keep their authority. |
| C6 | Coordinator is serial and repository-at-a-time. | Unambiguous failure attribution; parallelism deferred. |
| C7 | Coordinator sits above `plan execute`, driving `plan status` / `plan next` plus project-bound refine/build/merge. | No new lifecycle writer; workspace refusals stay. |
| C8 | A target with `platforms.required` makes platforms part of the M1 decision. | `emery init --platforms` can run unattended. |
| C9 | Progress is journal-derived in this cut. | Rich `progress.yaml` deferred. |

---

## Adapter inventory prerequisite

The program can only route to adapters that exist. Today's first-party set is narrower than a multi-language, multi-target migration needs:

- **Source**: `typescript` is the only code adapter (TS/JS; survey grammar excludes tRPC, GraphQL, gRPC, Lambda, Cloudflare Workers). Java, Python, Go, C#, Ruby each need their own adapter.
- **Target**: `omnia` covers `service` / `library`, `vectis` covers `mobile-app`, `contracts` is workload-neutral. No `web-frontend` target yet.

This is content work in `augentic/emery-adapters` under RM-21, not engine work. Descriptors (Part A) make the shortfall legible — "no adapter inspects Java" rather than a silent empty recommendation.

## One implementation cut

Ship Parts A–C together, serial:

1. Descriptor shape + static first-party index + closed workload vocabulary + deterministic filter + recommendation-report currency
2. CLI-owned `sources.yaml`, out-of-tree source snapshots, profiler, recommend → approve source bindings, `@key` lowering into `plan author`
3. Gate M1 + serial `program next` / `program status` over approved topology (operator-prepared workspace slots)

Do **not** land Part A alone "for later" or Part C against hand-typed bindings — the cut's exit is the operator loop above on a small in-house list.

## Deferred (after the skeleton is in daily use)

- [RFC-72](future/rfc-72-materialization.md) — managed clone / lease / sync
- Rich `progress.yaml`
- Parallelism across repositories
- Forge / hosted runner integration
- Captures / screenshots as first-class membership beyond path binds
- Multi-binding auto-composition; auto-approve policy
- Model adjudication / ranking over candidates; `describe` WIT operation
- Third-party registry / publisher / namespace trust (RM-21)
- Teaching `plan execute` workspace routing

## Non-goals

- Replacing Gate 1 / slice lifecycle with a second lifecycle authority
- Moving publication / PR merge into Emery — unchanged by [RFC-82](rfc-82-cross-repo-changesets.md), whose changeset surface *tracks and verifies* publication but never performs it
- Multi-target projects, or inferring a repository split without operator approval
- Auto-executing an arbitrary package from the network
- Replacing `plan.yaml` source bindings for ordinary single-repo work
- Replacing the compiled first-party GHCR mapping with descriptor-supplied locations
- Putting regenerable source snapshots under `.emery/cache/`
