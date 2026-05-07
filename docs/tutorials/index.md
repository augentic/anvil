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
| 5A | [Cross-Repo Changes](cross-repo-change.md) | Hub topology, registry, workspace sync, project assignment, push to PRs | Tutorial 4 |
| 5B | [Landing a Change](landing-a-change.md) | operator PR merge, `specify change finalize`, the three umbrella shapes | Tutorial 5A |
| 6 | [Legacy Migration at Scale](legacy-migration-at-scale.md) | Analyze/extract split, monolith decomposition | Tutorials 4-5 |

## Where to start

- **New to Specify?** Run the [Quick Start](quick-start.md) if you want the fastest path, then read [Tutorial 1](first-change.md).
- **Have an existing codebase?** Start with [Tutorial 3](brownfield-onboarding.md), then return to Tutorial 1.
- **Planning a large change?** Read Tutorials 1-2 first, then skip to [Tutorial 4](single-repo-change.md).
- **Working across multiple repos?** Read through to [Tutorial 5A](cross-repo-change.md), then continue to [Tutorial 5B](landing-a-change.md) for the landing half of the loop.
