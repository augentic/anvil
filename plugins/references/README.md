# Plugin References

This directory contains shared reference documentation used by multiple plugins.

## Structure

- **`specify.md`**: The core Specify artifact format specification (Proposal, Spec, Design, Tasks). Referenced by all plugins.
- **`agent-teams.md`**: Patterns for multi-agent collaboration (Lead/Specialist/Antagonist). Referenced by `code-reviewer`, `core-reviewer`, `ios-reviewer`, and `android-reviewer`.
- **`cli-output-shapes.md`**: Canonical JSON envelope shapes for `specify *` commands skills shell out to.
- **`guardrails.md`**: Cross-cutting "do not / never / always" rules repeated across 3+ skills (single-writer for lifecycle state, contract baseline immutability). Skills link here instead of restating these rules verbatim.

## Plugin-Specific References

Plugin-specific references are located within each plugin's directory:

- `adapters/targets/omnia/references/`: Omnia SDK patterns, WASM constraints, and provider documentation.
- `adapters/targets/vectis/references/`: Crux core idioms, iOS / Android shell patterns, design-system docs, and the layout-inferer contract.
- `plugins/spec/references/`: Artifact templates and instructions for define/build/merge.

Skills link to these references using relative paths and symlinks.
