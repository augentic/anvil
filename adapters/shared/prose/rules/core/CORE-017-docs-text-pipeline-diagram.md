---
id: CORE-017
title: Docs Text Pipeline Diagram
severity: important
trigger: Documentation uses a text pipeline diagram where an asset is required.
rule_hints:
  - kind: path-pattern
    value: "docs/explanation/**/*.md"
    description: Narrow to explanation docs before the text-diagram check fires.
  - kind: path-pattern
    value: "docs/orientation/**/*.md"
    description: Narrow to orientation docs before the text-diagram check fires.
  - kind: path-pattern
    value: "docs/tutorials/**/*.md"
    description: Narrow to tutorial docs before the text-diagram check fires.
  - kind: path-pattern
    value: "docs/how-to/**/*.md"
    description: Narrow to how-to docs before the text-diagram check fires.
  - kind: fenced-block
    value: fenced-body-contains
    description: Flag every `text` fenced block in the candidate set whose body contains a flow-diagram arrow. One finding per offending fence.
    config:
      langs:
        - text
      substrings:
        - "->"
        - "→"
---

## Rule

Prose documentation under `docs/explanation/`, `docs/orientation/`, `docs/tutorials/`, and `docs/how-to/` does not draw pipeline or flow diagrams inside a ` ```text ` fence. A fenced text block whose body contains a flow arrow (`->` or `→`) is a diagram in disguise; replace it with an SVG under `docs/assets/diagrams/` and embed the asset (see `docs/assets/diagrams/_STYLE.md`).

The deterministic-hint interpreter consumes the `FencedBlock` facts the framework indexer already produced, restricted to the `text` info string in the candidate set. The language allow-list and the banned arrow glyphs are policy carried in the rule's `config:`, not the engine.

## Look For

- A ` ```text ` block under `docs/explanation/` that lays out a stage-to-stage pipeline with `->` arrows.
- A how-to or tutorial that sketched a flow with `→` arrows inside a fenced text block instead of an embedded diagram asset.

## Fix

Author the diagram as an SVG under `docs/assets/diagrams/` (following `docs/assets/diagrams/_STYLE.md`) and embed it with an image link, removing the ` ```text ` flow block.
