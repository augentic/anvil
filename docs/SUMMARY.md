# Summary

[Introduction](index.md)

---

# Getting Started

- [What is Specify?](orientation/index.md)
- [Core concepts](explanation/concepts.md)
- [Prerequisites](orientation/prerequisites.md)
- [Quick Start (5 minutes)](tutorials/quick-start.md)

---

# Tutorials

- [Overview](tutorials/index.md)
- [Your first slice](tutorials/first-change.md)
- [Iterating on a baseline](tutorials/iterating-on-baseline.md)
- [Brownfield onboarding](tutorials/brownfield-onboarding.md)
- [A multi-slice change](tutorials/single-repo-change.md)
- [Working across repos: planning](tutorials/cross-repo-change.md)
- [Working across repos: executing](tutorials/cross-repo-execute.md)
- [Working across repos: landing](tutorials/landing-a-change.md)
- [Legacy migration at scale](tutorials/legacy-migration-at-scale.md)

---

# How-To Guides

- [Overview](how-to/index.md)
- [Recover from a failed change](how-to/recover-failed-change.md)
- [Recover from `registry-amendment-required`](how-to/recover-from-registry-amendment.md)
- [Resolve cross-project contract warnings](how-to/resolve-cross-project-contract-warnings.md)
- [Add a capability to an existing project](how-to/add-capability.md)
- [Bootstrap a platform hub](how-to/bootstrap-a-platform-hub.md)
- [Onboard a team member](how-to/onboard-team-member.md)
- [Manage registry projects](how-to/manage-registry-projects.md)
- [Work with contracts across repos](how-to/cross-repo-contracts.md)
- [Land a change](how-to/land-a-change.md)
- [Drop a slice](how-to/drop-a-slice.md)
- [Drop down a layer](how-to/drop-down-a-layer.md)
- [Troubleshooting overview](how-to/troubleshooting/index.md)
  - [Slice lifecycle](how-to/troubleshooting/slice-lifecycle.md)
  - [Merge problems](how-to/troubleshooting/merge.md)
  - [Plan and execution](how-to/troubleshooting/plan-and-execution.md)
  - [Init and capabilities](how-to/troubleshooting/init-and-capabilities.md)
  - [Hub and registry](how-to/troubleshooting/hub-and-registry.md)
  - [Contracts](how-to/troubleshooting/contracts.md)
  - [Change landing](how-to/troubleshooting/change-landing.md)

---

# Reference

- [Overview](reference/index.md)
- [Quick reference card](reference/quick-reference.md)
- [Artifact format](reference/artifact-format.md)
- [Lifecycle](reference/lifecycle.md)
- [Directory layout](reference/directory-layout.md)
- [Slice skills](reference/slice-skills/index.md)
  - [/spec:init](reference/slice-skills/init.md)
  - [/spec:define](reference/slice-skills/define.md)
  - [/spec:build](reference/slice-skills/build.md)
  - [/spec:merge](reference/slice-skills/merge.md)
  - [/spec:drop](reference/slice-skills/drop.md)
  - [/spec:extract](reference/slice-skills/extract.md)
- [Change skills](reference/change-skills/index.md)
  - [/change:plan <name> orchestrate](reference/change-skills/change.md)
  - [/change:plan](reference/change-skills/plan.md)
  - [/change:execute](reference/change-skills/execute.md)
  - [/spec:analyze](reference/change-skills/analyze.md)
- [CLI reference](reference/cli/index.md)
  - [specify status](reference/cli/status.md)
  - [specify slice](reference/cli/slice.md)
  - [specify change plan](reference/cli/plan.md)
  - [specify change](reference/cli/change.md)
  - [specify registry](reference/cli/registry.md)
  - [specify capability](reference/cli/capability.md)
  - [specify codex](reference/cli/codex.md)
  - [specify context](reference/cli/context.md)
  - [specify tool](reference/cli/tool.md)
  - [specify workspace](reference/cli/workspace.md)
  - [Contract validator (WASI tool)](reference/cli/contract.md)
  - [specify init](reference/cli/init.md)
  - [Vectis WASI tools](reference/cli/vectis.md)
- [Plugins](reference/plugins/index.md)
  - [Omnia](reference/plugins/omnia.md)
  - [Vectis](reference/plugins/vectis.md)
  - [Contract](reference/plugins/contract.md)
  - [Change](reference/plugins/change.md)
  - [RT](reference/plugins/rt.md)
  - [Client](reference/plugins/client.md)
- [Capabilities](reference/capabilities/index.md)
  - [Omnia capability](reference/capabilities/omnia.md)
  - [Vectis capability](reference/capabilities/vectis.md)
  - [Contracts capability](reference/capabilities/contracts.md)
- [Registry](reference/registry.md)
- [Change component](reference/change-component.md)
- [Configuration files](reference/configuration.md)
- [Declared tool helper inventory](reference/declared-tool-helper-inventory.md)

---

# Understanding Specify

- [The layered stack](explanation/three-layer-stack.md)
- [Artifacts in depth](explanation/artifacts.md)
- [Capabilities and plugins](explanation/capabilities-and-plugins.md)
- [Platform repo topologies](explanation/platform-repo.md)
- [Workspace tiers](explanation/workspace-tiers.md)
- [Tool declarations](explanation/tool-declarations.md)
- [Decision log](explanation/decision-log.md)
- [Release notes](explanation/release-notes.md)

---

# Appendices

- [Glossary](appendices/glossary.md)

---

# Contributing

- [Overview](contributing/index.md)
- [Skill authoring standards](standards/skill-authoring.md)
- [Anatomy of a capability](contributing/capability-anatomy.md)
- [Plugin development](contributing/plugin-development.md)
- [CLI architecture](contributing/cli-architecture.md)
- [Consistency checks](contributing/checks.md)
- [Skills test coverage](contributing/skills-test-coverage.md)
- [Acceptance tests](contributing/acceptance.md)
