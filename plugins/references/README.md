# Plugin References

This directory contains shared reference documentation used by multiple plugins.

## Structure

- **`specify.md`**: The core Specify artifact format specification (Proposal, Spec, Design, Tasks). Referenced by all plugins.
- **`agent-teams.md`**: Patterns for multi-agent collaboration (Lead/Specialist/Antagonist). Referenced by `code-reviewer`, `core-reviewer`, `ios-reviewer`, and `android-reviewer`.
- **`review-checks.md`**: Language and domain-agnostic review checklist (UNI-001 through UNI-021). Referenced by all reviewer skills (`code-reviewer`, `core-reviewer`, `ios-reviewer`, `android-reviewer`).

## Plugin-Specific References

Plugin-specific references are located within each plugin's directory:

- `plugins/omnia/references/`: Omnia SDK patterns, WASM constraints, and provider documentation.
- `plugins/spec/references/`: Artifact templates and instructions for define/build/merge.

Skills link to these references using relative paths and symlinks.
