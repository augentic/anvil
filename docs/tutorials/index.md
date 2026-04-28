# Tutorials

These tutorials walk you through Specify from simplest to most sophisticated. Each one builds on vocabulary introduced by the previous, and each can be followed end to end in a real project.

## Progression

The tutorials progress from single-change basics through multi-repo platform initiatives. Each builds on vocabulary introduced by the ones before it.

| # | Tutorial | What you learn | Prerequisites |
|---|---------|---------------|---------------|
| 1 | [Your First Change](first-change.md) | The define-build-merge loop, artifacts, baseline | [Prerequisites](../orientation/prerequisites.md) installed |
| 2 | [Iterating on a Baseline](iterating-on-baseline.md) | Delta specs, baseline accumulation, drift detection | Tutorial 1 |
| 3 | [Thinking Before Defining](thinking-before-defining.md) | Explore mode as a first-class activity | Tutorial 1 |
| 4 | [Brownfield Onboarding](brownfield-onboarding.md) | Extracting specs from existing code | [Prerequisites](../orientation/prerequisites.md) installed |
| 5 | [A Multi-Change Initiative](single-repo-initiative.md) | Plans, execute, dependency tracking | Tutorials 1-2 |
| 6A | [Cross-Repo Initiatives](cross-repo-initiative.md) | Hub topology, registry, workspace sync, project assignment, push to PRs | Tutorial 5 |
| 6B | [Landing an Initiative](landing-an-initiative.md) | `workspace merge`, `initiative finalize`, the three umbrella shapes | Tutorial 6A |
| 7 | [Legacy Migration at Scale](legacy-migration-at-scale.md) | Analyze/extract split, monolith decomposition | Tutorials 5-6 |

## Where to start

- **New to Specify?** Start with [Tutorial 1](first-change.md).
- **Have an existing codebase?** Start with [Tutorial 4](brownfield-onboarding.md), then return to Tutorial 1.
- **Planning a large initiative?** Read Tutorials 1-2 first, then skip to [Tutorial 5](single-repo-initiative.md).
- **Working across multiple repos?** Read through to [Tutorial 6A](cross-repo-initiative.md), then continue to [Tutorial 6B](landing-an-initiative.md) for the landing half of the loop.
