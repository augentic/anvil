# Acceptance Suites

> Status: Contract docs only. No suites are scaffolded here yet; this directory anchors the layout that follow-up changes in the [implementation plan](../../rfcs/rm-01-acceptance-framework-implementation-plan.md) will fill in.

This directory is the home for **shared outside-in acceptance suites** — Layer 4 in the framework's [Testing Layers](../README.md#testing-layers). A suite belongs here when it crosses skills, capabilities, repositories, or external boundaries in ways that no single capability or skill owns.

Narrow capability scenarios continue to live with the capability that owns them. See [Owner-local vs shared](../README.md#owner-local-scenarios-vs-shared-acceptance) for the boundary rule.

## Directory Convention

Each suite lives in its own subdirectory. The expected layout is:

```text
acceptance/suites/<suite-name>/
  README.md       # one-line goal, what the suite proves, what it does not cover
  scenario.md     # the canonical scenario pack (see Scenario Pack Shape in ../README.md)
  inputs/         # fixture briefs, source trees, registry/project YAML the suite seeds
  expected/       # expected structural shapes — JSON/YAML field assertions, file lists
  fixtures/       # optional: long-lived fixture data such as a recorded transcript
```

The runner discovers `acceptance/suites/<suite>/scenario.md` automatically; see [Scenario Discovery](../README.md#scenario-discovery) for the discovery rules and id conventions.

`<suite-name>` is kebab-case and stable. The first suite is `rm01-cross-repo`.

## Planned Suites

Suites are landed by follow-up changes in the [implementation plan](../../rfcs/rm-01-acceptance-framework-implementation-plan.md), not by this directory contract. The shape below is what the plan currently expects; this section will become a list of real entries as those changes land.

- **`rm01-cross-repo/`** — the first outside-in suite. A multi-repo journey starting from a concise feature brief (for example OAuth login or dark mode) under a registry-only platform hub plus `shop-backend` and `shop-mobile` fixture projects. Proves real `/change:plan` produces the expected role structure (one contract slice, one backend implementation slice, one mobile implementation slice, contract-first dependencies, correct project routing) and — through staged follow-up changes — drives execute, push, external-merge simulation, and finalize on a happy path. Recovery cases (RM-14) are explicitly out of scope for the first suite.

Until those changes land, this directory holds documentation only.

## Authoring Guidance

When a suite is added here:

- **Assert structurally.** See [`../assertions/README.md`](../assertions/README.md) for what to assert and what to avoid. Prefer role-based matching over exact slice names for live-agent runs.
- **Drive lifecycle through the CLI.** No suite hand-edits `.specify/` state; see the [CLI-Authoritative Invariant](../README.md#cli-authoritative-invariant). Stubbed phases may write fixture artifacts into a slice's working tree, but the surrounding lifecycle transitions still go through `specify` verbs.
- **Keep evidence temporary.** Run output goes to a temp directory under the runner's control, never into the repo tree. See the [Run Evidence Policy](../README.md#run-evidence-policy).
- **Use the post-RFC-13 vocabulary.** *Slice* and *change* have specific meanings (see [AGENTS.md](../../AGENTS.md)); there is no "change loop". Suite prose and assertion ids should match.
- **Stage scope.** Outside-in suites grow expensive quickly. Land plan-only assertions before execute, execute-with-stubs before real define/merge, real define/merge before real build, and one capability's real build at a time. The plan documents this staging.

## Out Of Scope For This Directory

- This directory does not host owner-local capability scenarios. Those stay in `capabilities/<capability>/tests/` (see [Contract Test Scenarios](../../capabilities/contracts/tests/README.md) for the seed pattern).
- This directory does not host the runner or assertion helpers. See [`../runner/README.md`](../runner/README.md) and [`../assertions/README.md`](../assertions/README.md).
- This directory does not host RM-14 recovery suites. Recovery cases will reuse the same framework but live alongside RM-01 in their own suite directories once they are scoped.
