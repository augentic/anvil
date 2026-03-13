# Plugin References

This directory contains shared reference documentation used by multiple plugins.

## Structure

- **`specify.md`**: The core Specify artifact format specification (Proposal, Spec, Design, Tasks). Referenced by all plugins.
- **`agent-teams.md`**: Patterns for multi-agent collaboration (Lead/Specialist/Antagonist). Referenced by `code-reviewer` and other complex skills.

## Plugin-Specific References

Plugin-specific references are located within each plugin's directory:

- `plugins/omnia/references/`: Omnia SDK patterns, WASM constraints, and provider documentation.
- `plugins/rt/references/`: Migration patterns and analysis guides.
- `plugins/plan/references/`: JIRA analysis and SoW generation guides.

Skills link to these references using relative paths.
