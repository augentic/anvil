# Summary

[Introduction](index.md)

---

# Getting Started

- [What is Emery?](orientation/index.md)
- [Prerequisites](orientation/prerequisites.md)

---

# Tutorials

- [Overview](tutorials/index.md)
- [Quick start](tutorials/quick-start.md)
- [Your first multi-slice change](tutorials/first-change.md)
- [Drive a slice by hand](tutorials/drive-a-slice-by-hand.md)
- [Cross-repo changes](tutorials/cross-repo-change.md)

---

# How-to Guides

- [Overview](how-to/index.md)
- [Drop down a layer](how-to/drop-down-a-layer.md)
- [Drive a slice manually](how-to/drive-slice-manually.md)
- [Amend a plan at Gate 1](how-to/amend-plan-at-gate-1.md)
- [Undo a plan entry](how-to/undo-a-plan-entry.md)
- [Drop a slice](how-to/drop-a-slice.md)
- [Resolve spec conflicts](how-to/resolve-spec-conflicts.md)
- [Interpret validate findings](how-to/interpret-validate-findings.md)
- [Bind multiple sources](how-to/bind-multiple-sources.md)
- [Recover from a stale guest lock](how-to/recover-from-a-stale-guest-lock.md)
- [Upgrade adapters](how-to/upgrade-adapters.md)

---

# Understanding Emery

- [Core concepts](explanation/concepts.md)
- [Workflow, standards, and artifacts](explanation/standards-layer.md)
- [The layered stack](explanation/layered-stack.md)
- [Artifacts in depth](explanation/artifacts.md)
- [From sources to slices](explanation/reconciliation.md)
- [Legacy migration at scale](explanation/legacy-migration.md)
- [Anatomy of an adapter](explanation/adapter-anatomy.md)
- [Component factoring (Vectis)](explanation/components.md)

---

# Reference

- [Overview](reference/index.md)
- [Quick reference card](reference/quick-reference.md)
- [Artifact format](reference/artifact-format.md)
- [Provenance projection](reference/provenance.md)
- [Lifecycle](reference/lifecycle.md)
- [Diagnostics index](reference/diagnostics.md)
- [Directory layout](reference/directory-layout.md)
- [Change skills](reference/change-skills/index.md)
- [Slice skills](reference/slice-skills/index.md)
- [CLI reference](reference/cli/index.md)
  - [emery init](reference/cli/init.md)
  - [emery slice](reference/cli/slice.md)
  - [emery plan](reference/cli/plan.md)
  - [emery adapter, source and target resolve](reference/cli/adapter.md)
  - [emery registry](reference/cli/registry.md)
  - [emery workspace](reference/cli/workspace.md)
  - [Contract validator (WASI tool)](reference/cli/contract.md)
  - [Vectis WASI tools](reference/cli/vectis.md)
  - [CLI output shapes](reference/cli-output-shapes.md)
- [Plugins](reference/plugins/index.md)
- [Adapter contract](reference/adapter-contract.md)
- [Source adapters](reference/sources/index.md)
- [Target adapters](reference/targets/index.md)
  - [Omnia](reference/targets/omnia.md)
  - [Vectis](reference/targets/vectis.md)
  - [Contracts](reference/targets/contracts.md)
- [Registry](reference/registry.md)
- [Configuration files](reference/configuration.md)
- [Review team protocol](reference/review-team-protocol.md)

---

# Appendices

- [Glossary](appendices/glossary.md)
---

# Contributing

- [Overview](contributing/index.md)
- [Augentic specialist usage](explanation/augentic-emery-usage.md)
- [Documentation authoring standards](standards/doc-authoring.md)
- [Authoring snippets](authoring-snippets/README.md)
  - [Hero](authoring-snippets/hero.md)
- [CLI contract](standards/cli-contract.md)
- [Workflow contract](standards/workflow.md)
- [Testing standards](standards/testing.md)
- [Architecture standards](standards/architecture.md)
- [Coding standards](standards/coding-standards.md)
- [Rust style](standards/style.md)
- [Handler shape](standards/handler-shape.md)
- [The developer loop](contributing/dev-loop.md)
- [Quality gates](contributing/quality-gates.md)
- [Cursor operator plugins](contributing/operator-plugins.md)
- [CLI architecture](contributing/cli-architecture.md)
