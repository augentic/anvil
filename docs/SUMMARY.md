# Summary

[Introduction](index.md)

---

# Getting Started

- [What is Specify?](orientation/index.md)
- [Prerequisites](orientation/prerequisites.md)
- [Quick Start (5 Minutes)](tutorials/quick-start.md)

---

# Tutorials

- [Overview](tutorials/index.md)
- [Your First Slice](tutorials/first-change.md)
- [Iterating on a Baseline](tutorials/iterating-on-baseline.md)
- [Brownfield Onboarding](tutorials/brownfield-onboarding.md)
- [A Multi-Slice Change](tutorials/single-repo-change.md)
- [Cross-Repo Changes](tutorials/cross-repo-change.md)
- [Landing a Change](tutorials/landing-a-change.md)
- [Legacy Migration at Scale](tutorials/legacy-migration-at-scale.md)

---

# How-To Guides

- [Overview](how-to/index.md)
- [Recover from a Failed Change](how-to/recover-failed-change.md)
- [Add a Capability to an Existing Project](how-to/add-capability.md)
- [Bootstrap a Platform Hub](how-to/bootstrap-a-platform-hub.md)
- [Manage Registry Projects](how-to/manage-registry-projects.md)
- [Work with Contracts Across Repos](how-to/cross-repo-contracts.md)
- [Resolve Cross-Project Contract Warnings](how-to/resolve-cross-project-contract-warnings.md)
- [Land a Change](how-to/land-a-change.md)
- [Recover from registry-amendment-required](how-to/recover-from-registry-amendment.md)
- [Onboard a Team Member](how-to/onboard-team-member.md)
- [Drop Down a Layer](how-to/drop-down-a-layer.md)

---

# Reference

- [Overview](reference/index.md)
- [Quick Reference Card](reference/quick-reference.md)
- [Artifact Format](reference/artifact-format.md)
- [Lifecycle](reference/lifecycle.md)
- [Directory Layout](reference/directory-layout.md)
- [Slice Skills (Layer 2)](reference/slice-skills/index.md)
  - [/spec:init](reference/slice-skills/init.md)
  - [/spec:define](reference/slice-skills/define.md)
  - [/spec:build](reference/slice-skills/build.md)
  - [/spec:merge](reference/slice-skills/merge.md)
  - [/spec:drop](reference/slice-skills/drop.md)
  - [/spec:extract](reference/slice-skills/extract.md)
- [Change Skills (Layers 3 & 4)](reference/change-skills/index.md)
  - [/change:plan <name> orchestrate](reference/change-skills/change.md)
  - [/change:plan](reference/change-skills/plan.md)
  - [/change:execute](reference/change-skills/execute.md)
  - [/spec:analyze](reference/change-skills/analyze.md)
- [CLI Reference](reference/cli/index.md)
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
  - [Omnia Capability](reference/capabilities/omnia.md)
  - [Vectis Capability](reference/capabilities/vectis.md)
  - [Contracts Capability](reference/capabilities/contracts.md)
- [Registry](reference/registry.md)
- [Change Component](reference/change-component.md)
- [Configuration Files](reference/configuration.md)
- [Declared Tool Helper Inventory](reference/declared-tool-helper-inventory.md)

---

# Understanding Specify

- [The Layered Stack](explanation/three-layer-stack.md)
- [Artifacts in Depth](explanation/artifacts.md)
- [Capabilities and Plugins](explanation/capabilities-and-plugins.md)
- [Platform Repo Topologies](explanation/platform-repo.md)
- [Workspace Tiers](explanation/workspace-tiers.md)
- [Tool Declarations](explanation/tool-declarations.md)
- [Decision Log](explanation/decision-log.md)
- [Migrating CLI v1](explanation/migrating-cli-v1.md)
- [What's New Since v0.23](explanation/whats-new.md)

---

# Appendices

- [Glossary](appendices/glossary.md)
- [Troubleshooting](appendices/troubleshooting.md)

---

# Contributing

- [Overview](contributing/index.md)
- [Anatomy of a Skill](contributing/skill-anatomy.md)
- [Anatomy of a Capability](contributing/capability-anatomy.md)
- [Plugin Development](contributing/plugin-development.md)
- [CLI Architecture](contributing/cli-architecture.md)
- [Consistency Checks](contributing/checks.md)
