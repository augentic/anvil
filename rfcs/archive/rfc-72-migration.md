# Adapter Selection and Source Selection

> **Status: Superseded (archived).** Migration intake and topology ownership moved to [RFC-88 Detached Changes](../rfc-88-detached-changes.md). Do not implement this document; historical prior art only (filename keeps the old number; active [RFC-72](rfc-72-materialization.md) is managed materialization).
>
> Owns: durable source membership (`sources.yaml`), source materialization, repository profile schema + profiler, source-adapter selection policy, recommendation/approval, lowering approved sources into change plans.
>
> Supersedes: [rfc-21-catalogue.md](rfc-21-catalogue.md).

## Intent

Let an operator provide repositories and supporting inputs once, then reuse them across many Emery changes. Keep source inputs (`sources.yaml`) separate from target projects (`registry.yaml`) and per-change bindings (`plan.yaml`).

The operator hands Emery a list of repositories. Emery profiles each one deterministically, matches the profile against adapter descriptors, recommends an exact pinned source binding per input, and — once approved — lowers those bindings into `emery plan author`. The operator never has to know that `typescript` is the adapter that reads a Node monolith.

## Intake shape

`sources.yaml` at the platform-repo root, sibling to `registry.yaml`. Membership is durable platform state, not change state, so a missing file is inert rather than an error — the same posture as the registry.

Inputs are not all code. A design-document tree, a screenshot set, and a runtime capture tree are first-class intake entries alongside legacy repositories, because the same profile-then-recommend loop routes them to `documentation`, `screenshots`, and `captures` respectively.

## The profiler

Profiling must run *before* any adapter is chosen, which settles where it lives: the profiler is engine-side and deterministic. It cannot be an adapter, because selecting the adapter is the question it exists to answer.

It reads manifest sentinels and a file census — `package.json`, `go.mod`, `pom.xml`, `build.gradle`, `*.csproj`, `Cargo.toml`, `pyproject.toml`, `requirements.txt`, `Gemfile`, `composer.json` — and emits a repository profile: languages by weight, manifest evidence, framework hints, candidate workload kinds from the [RFC-71](rfc-71-discovery.md) vocabulary, and input kind (`code` / `documentation` / `images` / `captures`). No model call, no component dispatch, byte-stable for a given tree.

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | `sources.yaml` is the durable source membership; the CLI is its single writer. | Legacy repository lists stop being retyped per change. Sources stay separate from `registry.yaml` (targets) and `plan.yaml` (per-change bindings). |
| D2 | The repository profiler is engine-side, deterministic, and model-free. | It can run before adapter selection, and the same tree always profiles the same way, so a recommendation is reproducible and diffable. |
| D3 | Intake covers non-code inputs. Profiles carry an input kind, so docs, screenshots, and capture trees route through the same loop. | Design-document input participates in selection instead of being a separate hand-bound special case. |
| D4 | Selection is profile → descriptor filter ([RFC-71](rfc-71-discovery.md)) → recommendation → operator approval → exact pinned binding. | The operator approves adapter names and pins once per input, then stops thinking about them. |
| D5 | First cut recommends one source adapter per profiled input. | Multi-binding composition over one repository (say, `typescript` plus `captures` over the same tree) stays deferred; an operator can still declare it by hand. |
| D6 | Approved bindings install through the existing pinned pull-on-miss path ([RFC-71](../rfc-71-deployment.md)). | Intake adds no download mechanism, no second store, and no configurable registry. |
| D7 | Approved bindings lower into `plan.yaml.sources` through `emery plan author`, addressed by an `@key` selector form. | Gate 1 still reviews the authored plan. Intake feeds the existing plan surface rather than becoming a parallel one. |
| D8 | Source snapshots are immutable, live out of tree, and are never the target slot. | A repository that is both migration source and target keeps evidence integrity while its slot is being written ([RFC-72](rfc-72-materialization.md)). |
| D9 | Plan-time survey stays serial in the first cut. | Repeat-until-drained is repository-at-a-time ([RFC-88](../rfc-88-detached-changes.md)), so a `--jobs` fan-out is a throughput optimisation with no correctness role yet. It stays deferred, and the current first-failure-aborts behaviour remains the contract. |

## First delivery

Stages 1–3, serial:

1. CLI-owned `sources.yaml` membership
2. Git + docs snapshot materialization into an out-of-tree cache
3. Repository profile + deterministic profiler
4. Recommend → approve exact source bindings
5. Lower approved `@key` bindings into `emery plan author`

## Deferred

- Captures / screenshots intake as first-class membership kinds beyond path binds
- Multi-binding auto-composition
- Auto-approve policy
- `--jobs` parallelism, prune, portal import

## Non-goals

- Replacing `plan.yaml` source bindings for ordinary single-repo work
- Putting regenerable source snapshots under `.emery/cache/`
- Profiling as a model judgment — the profile is deterministic evidence; judgment happens later, over the recommendation
- Selecting or binding target adapters ([RFC-88](../rfc-88-detached-changes.md) owns that policy)
