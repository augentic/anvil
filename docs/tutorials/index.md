# Tutorials

These tutorials walk you through Specify from simplest to most sophisticated. Each one builds on vocabulary introduced by the previous, and each can be followed end to end in a real project.

## Progression

The tutorials progress from single-slice basics through multi-repo platform changes. Each builds on vocabulary introduced by the ones before it.

| # | Tutorial | What you learn | Prerequisites |
|---|---------|---------------|---------------|
| 0 | [Quick Start (5 Minutes)](quick-start.md) | The shortest path through init, define, build, and merge | [Prerequisites](../orientation/prerequisites.md) installed |
| 1 | [Your First Slice](first-change.md) | The define-build-merge loop, artifacts, baseline | [Prerequisites](../orientation/prerequisites.md) installed |
| 2 | [Iterating on a Baseline](iterating-on-baseline.md) | Delta specs, baseline accumulation, merge keys | Tutorial 1 |
| 3 | [Brownfield Onboarding](brownfield-onboarding.md) | Extracting specs from existing code | [Prerequisites](../orientation/prerequisites.md) installed |
| 4 | [A Multi-Slice Change](single-repo-change.md) | Plans, execute, dependency tracking | Tutorials 1-2 |
| 5 | [Working across repos: planning](cross-repo-change.md) | Hub topology, registry, workspace sync, project assignment | Tutorial 4 |
| 5a | [Reviewing the plan](reviewing-a-plan.md) | The human seam between `/change:draft` and `/change:execute` — `specify plan status`, `specify plan amend`, abort | Tutorial 5 |
| 6 | [Working across repos: executing](cross-repo-execute.md) | Workspace inspection, `/change:execute loop` across projects, `specify workspace push` | Tutorial 5 |
| 7 | [Working across repos: landing](landing-a-change.md) | Operator PR merge, `/change:finalize`, the three change shapes | Tutorial 6 |
| 8 | [Legacy Migration at Scale](legacy-migration-at-scale.md) | Analyze/extract split, monolith decomposition | Tutorials 5-7 |
| 9 | [Monolith Decomposition](monolith-decomposition.md) | Surface scanning, candidate sizing, same-source clustering | Tutorial 8 |
| 10 | [Legacy Fleet Decomposition](legacy-fleet-decomposition.md) | Multi-source survey, combined inventory, operator review | Tutorials 8-9 |

## Where to start

- **New to Specify?** Run the [Quick Start](quick-start.md) if you want the fastest path, then read [Your First Slice](first-change.md).
- **Have an existing codebase?** Start with [Brownfield Onboarding](brownfield-onboarding.md), then return to [Your First Slice](first-change.md).
- **Planning a large change?** Read Tutorials 1-2 first, then skip to [A Multi-Slice Change](single-repo-change.md).
- **Working across multiple repos?** Walk through Tutorials 5-7 in order: [planning](cross-repo-change.md), [reviewing the plan](reviewing-a-plan.md), [executing](cross-repo-execute.md), and [landing](landing-a-change.md).
