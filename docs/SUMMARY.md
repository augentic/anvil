# Summary

[Introduction](index.md)

---

# Getting Started

- [What is Specify?](orientation/index.md)
- [Prerequisites](orientation/prerequisites.md)

---

# Tutorials

- [Overview](tutorials/index.md)
- [Quick start](tutorials/quick-start.md)
- [Your first multi-slice change](tutorials/first-change.md)
- [Cross-repo changes](tutorials/cross-repo-change.md)
- [Legacy migration at scale](tutorials/legacy-migration-at-scale.md)

---

# How-to Guides

- [Overview](how-to/index.md)
- [Drop down a layer](how-to/drop-down-a-layer.md)
- [Drive a slice manually](how-to/drive-slice-manually.md)
- [Amend a plan at Gate 1](how-to/amend-plan-at-gate-1.md)
- [Resolve spec conflicts](how-to/resolve-spec-conflicts.md)
- [Bind multiple sources](how-to/bind-multiple-sources.md)

---

# Understanding Specify

- [Core concepts](explanation/concepts.md)
- [Workflow, standards, and artifacts](explanation/standards-layer.md)
- [The layered stack](explanation/layered-stack.md)
- [Artifacts in depth](explanation/artifacts.md)
- [From sources to slices](explanation/reconciliation.md)
- [Anatomy of an adapter](explanation/adapter-anatomy.md)
- [Tool declarations](explanation/tool-declarations.md)
- [Component factoring (Vectis)](explanation/components.md)

---

# Reference

- [Overview](reference/index.md)
- [Quick reference card](reference/quick-reference.md)
- [Artifact format](reference/artifact-format.md)
- [Lifecycle](reference/lifecycle.md)
- [Directory layout](reference/directory-layout.md)
- [Change skills](reference/change-skills/index.md)
  - [/spec:plan](reference/change-skills/plan.md)
  - [/spec:execute](reference/change-skills/execute.md)
  - [/spec:finalize](reference/change-skills/finalize.md)
- [Slice skills](reference/slice-skills/index.md)
  - [/spec:init](reference/slice-skills/init.md)
  - [/spec:refine](reference/slice-skills/refine.md)
  - [/spec:build](reference/slice-skills/build.md)
  - [/spec:merge](reference/slice-skills/merge.md)
  - [/spec:drop](reference/slice-skills/drop.md)
- [CLI reference](reference/cli/index.md)
  - [specrun init](reference/cli/init.md)
  - [specrun slice](reference/cli/slice.md)
  - [specrun plan](reference/cli/plan.md)
  - [specrun source and target resolve](reference/cli/adapter.md)
  - [specrun registry](reference/cli/registry.md)
  - [specrun workspace](reference/cli/workspace.md)
  - [specrun tool](reference/cli/tool.md)
  - [Contract validator (WASI tool)](reference/cli/contract.md)
  - [Vectis WASI tools](reference/cli/vectis.md)
  - [CLI output shapes](reference/cli-output-shapes.md)
- [Plugins](reference/plugins/index.md)
  - [Client](reference/plugins/client.md)
- [Source adapters](reference/sources/index.md)
- [Target adapters](reference/targets/index.md)
  - [Omnia](reference/targets/omnia.md)
  - [Vectis](reference/targets/vectis.md)
  - [Contracts](reference/targets/contracts.md)
- [Registry](reference/registry.md)
- [Configuration files](reference/configuration.md)
- [Ignore directives](reference/ignore-directives.md)
- [Declared tool helper inventory](reference/declared-tool-helper-inventory.md)
- [Review team protocol](reference/review-team-protocol.md)

---

# Appendices

- [Glossary](appendices/glossary.md)
- [Decision log](explanation/decision-log.md)
- [Release notes](explanation/release-notes.md)

---

# Contributing

- [Overview](contributing/index.md)
- [Augentic specialist usage](explanation/augentic-specify-usage.md)
- [Documentation authoring standards](standards/doc-authoring.md)
- [Skill authoring standards](standards/skill-authoring.md)
- [Skill guardrails](standards/skill-guardrails.md)
- [Plugin development](contributing/plugin-development.md)
- [CLI architecture](contributing/cli-architecture.md)
- [Consistency checks](contributing/checks.md)
- [Skills test coverage](contributing/skills-test-coverage.md)
- [Acceptance tests](contributing/acceptance.md)
