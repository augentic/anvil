# Migration Programs and Durable Progress

> Status: Draft — nothing landed
>
> Owns: a migration-sized umbrella above changes, repository-by-repository scheduling, target selection policy, approved adapter/topology decisions, durable progress, re-entry, migration audit projections.
>
> Depends on: [RFC-71](rfc-71-discovery.md) Stage 1, [RFC-72](rfc-72-migration.md). [RFC-73](rfc-73-materialization.md) optional for the walking skeleton.
>
> Supersedes: [archive/rfc-22-ledger.md](archive/rfc-22-ledger.md).

## Intent

Coordinate work that spans many repositories and many Specify changes. Each work item still uses the existing change → slice refine → build → merge loop; the program schedules batches and records progress.

The operator supplies a repository list once and approves one set of topology decisions. The program then works repository at a time: pick the next input, apply its approved target binding, author and execute its change, record the outcome, move on.

## Target selection

Source selection asks "what can read this repository?" ([RFC-72](rfc-72-migration.md)). Target selection asks the different question "what should this repository become?", and the two must not collapse into one inference.

The program derives candidate workload kinds from the repository profile, filters targets by their descriptor `produces` ([RFC-71](rfc-71-discovery.md)), and presents a recommendation. A profile that looks like an Express monolith makes `service` the default workload kind — it does not decide that the migrated result stays a service. Operator intent at Gate M1 is authoritative over the profile.

## One target adapter per target repository

`project.yaml.adapter` is singular and stays singular. The program never binds two target adapters to one registry project, and there is no multi-target project shape in this cut — a single repository holding both a browser frontend and its backing service is out of scope.

A repository whose profile yields two independent workload kinds is therefore a **topology decision, not an adapter decision**. The program surfaces it at Gate M1 as a blocking split: the operator either declares which single workload the repository migrates as, or splits it into two registry projects with one target each. The program does not schedule the repository until that decision is recorded.

## Applying approved topology

A newly scheduled target may have no `.specify/project.yaml`. The program proposes its contents — name, exact target adapter pin, platforms — and the operator approves them at M1, but the program writes nothing itself: application runs through `specify init`. An existing `project.yaml` is authoritative and is never rewritten.

`project.yaml` therefore stays required. What changes is who types it: the program proposes, the operator approves, `specify init` writes. That preserves the roadmap's one-authored-home-per-fact principle instead of scattering target intent across program state.

## Operator loop

Target surface, not implemented:

```text
specify source add … / import          # the repository list, once      (RFC-72)
specify source profile                 # deterministic repo profiles    (RFC-72)
specify program recommend              # source + target candidates     (RFC-71/72/74)
specify program approve                # Gate M1 — topology + adapters  (RFC-74)
specify program next                   # claim the next repository      (RFC-74)
  specify init <target> --platforms …  #   apply approved topology
  specify plan author --source @key …  #   lowered bindings, exits pending
  specify plan approve                 #   Gate 1, operator-only
  specify plan next → refine → build → merge
specify program status                 # durable progress and re-entry  (RFC-74)
```

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | One target adapter per target repository; `project.yaml.adapter` stays singular. | No multi-target project shape, no frontend-plus-backend single-repo targets. Two workloads in one tree means two registry projects. |
| D2 | A repository with two candidate workload kinds blocks at Gate M1 until the operator splits it or picks one. | The ambiguity surfaces once, at approval time, instead of at build time in a half-migrated repository. |
| D3 | Target selection consumes the profile's workload kinds plus operator intent; intent wins. | Honours [RFC-71](rfc-71-discovery.md)'s non-goal that source implementation shape is not desired target architecture. |
| D4 | Approved topology is applied through `specify init`, never by program file writes. | One writer for `project.yaml`; platform validation and the adapter floor check run exactly where they already run. |
| D5 | Gate M1 approves topology and adapter decisions only. | Gate 1 and the slice lifecycle keep their authority. M1 is a prerequisite, not a second lifecycle. |
| D6 | The coordinator is serial and repository-at-a-time. | Matches [RFC-73](rfc-73-materialization.md)'s lease-per-slot model and keeps failure attribution unambiguous. Parallelism stays deferred. |
| D7 | The coordinator sits above `plan execute`, driving `plan status` / `plan next` plus the project-bound refine, build, and merge verbs. | It does not lift the guest `plan-author-workspace-unsupported` / `plan-execute-workspace-unsupported` refusals, and it adds no lifecycle writer. |
| D8 | A target declaring `platforms.required` makes its platform set part of the M1 decision. | `specify init --platforms` can run unattended when topology is applied, instead of failing `project-platforms-required` mid-schedule. |
| D9 | Progress is journal-derived in the first cut. | Re-entry works off durable events already emitted; a rich `progress.yaml` stays Stage 3. |

## Adapter inventory prerequisite

The program can only route to adapters that exist, and today's first-party set is narrower than a multi-language, multi-target migration needs:

- **Source**: `typescript` is the only code adapter (TypeScript and JavaScript; its survey grammar excludes tRPC, GraphQL, gRPC, Lambda, and Cloudflare Workers). Java, Python, Go, C#, and Ruby each need their own adapter.
- **Target**: `omnia` covers `service` and `library`, `vectis` covers `mobile-app`, `contracts` is workload-neutral. There is no `web-frontend` target — Vectis accepts `web` and `desktop` platform tokens but has no build prompts or shell interpretation behind them.

This is content work in `augentic/specify-adapters` under roadmap RM-21, not engine work, and it gates the end-to-end scenario more tightly than any RFC in this program. Descriptors ([RFC-71](rfc-71-discovery.md)) make the shortfall legible — an unmatched profile reports "no adapter inspects Java" rather than silently recommending nothing.

## First delivery

Stages 1–2, serial:

1. Serial coordinator over an approved program plan
2. Program Gate M1 (operator approval of topology / adapter decisions)
3. Substrate table pointing at RFCs 70–73 for deployment, discovery, intake, and optional materialization

## Deferred

- Rich `progress.yaml` (Stage 3)
- Parallelism across repositories
- Forge / hosted runner integration
- Requiring [RFC-73](rfc-73-materialization.md) before clone friction demands it — operator-prepared slots suffice first

## Non-goals

- Replacing Gate 1 / slice lifecycle with a second lifecycle authority
- Moving publication/merge of PRs into Specify
- Multi-target projects, or inferring a repository split without operator approval
- Teaching `plan execute` workspace routing (that remains its own change)
