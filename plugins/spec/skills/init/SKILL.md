---
name: init
description: Initialize Specify in a project. Creates the .specify/ directory structure and config.yaml. Use when setting up a new project for spec-driven development.
license: MIT
---

Initialize Specify in this project.

I'll create the `.specify/` directory structure and a starter `config.yaml` for you to customize.

---

**Input**: None required. Optionally the user may describe their tech stack or project context.

**Steps**

1. **Check if already initialized**

   Check whether `.specify/config.yaml` exists.

   - If it exists, inform the user: "Specify is already initialized in this project. Your config is at `.specify/config.yaml`."
   - Use **AskQuestion tool** to confirm whether they want to reinitialize (which overwrites config).
   - If they decline, stop.

2. **Select Schema**

   Use the **AskQuestion tool** to let the user select the project schema:
   - **omnia**: New development (JIRA -> Rust WASM)
   - **realtime**: Migration (TypeScript -> Rust WASM)

   Store the selection as `$SCHEMA`.

3. **Create directory structure**

   ```bash
   mkdir -p .specify/changes .specify/specs
   ```

4. **Write config.yaml**

   Write `.specify/config.yaml` with the following template, using the selected `$SCHEMA`. If the user provided project context, fill in the `context` field. Otherwise use the default below.

   ```yaml
   schema: $SCHEMA

   context: |
     <project context -- tech stack, architecture, testing approach>

   rules:
     proposal:
       - "Identify the source: a git repository URL (for code analysis), a
         JIRA/ADO/Linear epic key (for requirements analysis), or Manual (feature
         work). This determines the workflow for downstream artifacts."
       - Capability names are crate names. For modified capabilities, use the
         existing spec folder name from .specify/specs/. For new capabilities,
         choose a name that will be the crate name
     specs:
       - Use WHEN/THEN format for scenarios
       - Reference existing patterns before inventing new ones
     design:
       - Document domain model with entity relationships
       - Document business logic per handler as tagged pseudocode
     tasks:
       - Structure tasks around the skill chain for new crates
       - Structure tasks around behavioral changes for existing crate updates
   ```

5. **Prompt for customization**

   Tell the user:
   - "Specify initialized. Config written to `.specify/config.yaml`."
   - "Edit the `context` field to describe your project's tech stack, architecture, and testing approach."
   - "Edit `rules` to add project-specific constraints for each artifact type."
   - "When ready, run `/spec:propose` to start your first change."

**Output**

```
## Specify Initialized

**Config**: .specify/config.yaml
**Changes**: .specify/changes/
**Baseline specs**: .specify/specs/

Next steps:
1. Edit `.specify/config.yaml` to describe your project
2. Run `/spec:propose` to create your first change
```

**Guardrails**
- Do not overwrite an existing config without user confirmation
- Keep the default config template minimal -- users should customize it
- Do not create `.specify/schemas/` -- skills embed schema knowledge directly
