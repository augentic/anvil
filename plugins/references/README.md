# Plugin References

This directory contains shared reference documentation used by multiple plugins.

## Structure

- **`specify.md`**: The core Specify artifact format specification (Proposal, Spec, Design, Tasks). Referenced by all plugins.
- **`agent-teams.md`**: Patterns for multi-agent collaboration (Lead/Specialist/Antagonist). Referenced by `code-reviewer` and other complex skills.

## Plugin-Specific References

Plugin-specific references are located within each plugin's directory:

- `plugins/omnia/references/`: Omnia SDK patterns, WASM constraints, and provider documentation.
- `plugins/spec/references/`: Artifact templates and instructions for propose/apply/archive.

Skills link to these references using relative paths and symlinks.
