# Anatomy of a Skill

A skill is a markdown file (`SKILL.md`) that instructs a Cursor agent how to perform a specific task. Skills are the primary unit of behavior in Specify -- every `/spec:*`, `/omnia:*`, `/vectis:*`, `/contracts:*`, `/rt:*`, and `/plan:*` command maps to one skill.

## Directory structure

Skills live under `plugins/<plugin>/skills/<skill-name>/`:

```text
plugins/
└── spec/
    └── skills/
        └── define/
            ├── SKILL.md            # The skill definition
            └── references/         # Optional supporting documents
                ├── spec-format.md
                └── ...
```

A skill directory must contain a `SKILL.md` at the top level. It may optionally include a `references/` subdirectory with supporting markdown files that the skill body links to. Some skills also have an `examples/` subdirectory with worked examples.

## SKILL.md structure

Every `SKILL.md` begins with YAML frontmatter validated against `schemas/skill.schema.json`:

```yaml
---
name: define
description: Define a new change with all artifacts generated in one step. Use when the user wants to quickly describe what they want to build.
license: MIT
argument-hint: "[description] [artifact-id?]"
allowed-tools: Read, Write, StrReplace, Shell, Grep, Glob, ReadLints, WebFetch
---
```

### Frontmatter fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Kebab-case identifier, must match the parent directory name |
| `description` | yes | One-line description (minimum 10 characters) including when to use the skill |
| `license` | no | `MIT`, `Apache-2.0`, or `proprietary` |
| `argument-hint` | no | Argument pattern shown to the user (e.g. `"[description]"`, `"[crate-name]"`) |
| `allowed-tools` | no | Comma-separated list of Cursor tools the skill may use; validated against a known set plus `mcp__*` prefixed tools |

### Body sections

The body after the frontmatter varies by skill type, but the following patterns are common across the codebase:

**Workflow skills** (e.g. `/spec:define`, `/spec:build`, `/spec:merge`) typically include:

- **Context** -- when the skill runs, what state it expects, how it fits into the workflow
- **Driver-supplied arguments** -- arguments passed by `/spec:execute` in plan-driven mode
- **Phase outcome contract** -- how the skill reports success or failure back to the execute driver
- **Steps** -- numbered procedure the agent follows, interleaving CLI commands with agent reasoning
- **Guardrails** -- hard constraints on what the skill may and may not do

**Generator skills** (e.g. `/omnia:crate-writer`, `/vectis:core-writer`) typically include:

- **Authority hierarchy** -- precedence order for resolving conflicts (SKILL.md > artifacts > references > source > inference)
- **Hard rules** -- invariants that must never be violated (e.g. "no forbidden crates", "provider-only I/O")
- **Arguments** -- variable definitions derived from artifact inputs
- **Derived arguments** -- computed values the skill resolves at runtime
- **Process** -- generation or update procedure, often split by mode (create vs update)
- **Required references** -- files the skill must read before generating code

**Reviewer skills** (e.g. `/omnia:code-reviewer`, `/vectis:core-reviewer`) follow the [agent-teams](../../plugins/references/agent-teams.md) pattern with structural, logic, and quality specialist roles plus an antagonist.

## How skills invoke the CLI

A core design principle: **the CLI owns correctness, the agent owns judgment.**

Skills delegate every deterministic operation to the `specify` CLI and consume its structured JSON output. They never hand-edit `.metadata.yaml`, never manipulate the `.specify/` directory directly, and never duplicate validation logic.

```text
Agent (skill)                          CLI (specify binary)
─────────────                          ────────────────────
Elicit intent from user          →
                                       specify change create <name>
                                  ←    { "status": "created", ... }
Read brief, write artifact       →
                                       specify validate .specify/changes/<name>/
                                  ←    { "passed": true, ... }
                                       specify change transition <name> defined
                                  ←    { "status": "defined", ... }
```

The litmus test: "Would this operation need to understand `.specify/` directory structure or spec format?" If yes, it belongs in the CLI. If no (like running `cargo test` or writing a Rust file), it stays with the agent.

## How skills delegate to other skills

Skills can invoke other skills using `<!-- skill: plugin:skill-name -->` directives in their body. For example, `/spec:build` delegates implementation to specialist skills declared by the active schema:

```markdown
<!-- skill: omnia:crate-writer -->
<!-- skill: omnia:test-writer -->
```

The Cursor agent reads these directives and loads the referenced skill when it reaches that point in the procedure. The `checks.ts` script validates that all `<!-- skill: ... -->` directives reference existing skills.

## Adding a new skill

1. **Create the directory** under the appropriate plugin:

   ```text
   plugins/<plugin>/skills/<skill-name>/SKILL.md
   ```

2. **Write the frontmatter.** The `name` field must match the directory name exactly. Include a `description` of at least 10 characters.

3. **Write the body.** Follow the conventions of existing skills in the same plugin. Generator skills should define an authority hierarchy; workflow skills should document the phase outcome contract.

4. **Add references** if needed. Place supporting documents in a `references/` subdirectory and link to them from the skill body using relative paths like `references/guardrails.md`.

5. **Register the skill** in the plugin's `.cursor-plugin/plugin.json` if one exists. The marketplace manifest at `.cursor-plugin/marketplace.json` declares plugins by `source` directory; individual skills are discovered by directory walking.

6. **Run `make checks`** to verify:
   - Frontmatter validates against `schemas/skill.schema.json`
   - The `name` matches the directory name
   - All `allowed-tools` entries are recognized
   - All `references/` and `examples/` links resolve
   - All `$VARIABLE` definitions are used and all uses are defined

## Shared references

Two files in `plugins/references/` are shared across plugins:

- **`specify.md`** -- the master reference for artifact format, lifecycle states, naming conventions, delta-merge rules, and hard constraints that apply to every skill
- **`agent-teams.md`** -- the multi-agent review pattern used by reviewer skills (structural, logic, and quality specialists plus an antagonist)

Skills that need these references typically symlink them into their own `references/` directory so relative paths resolve. The `checks.ts` script validates that all symlinks under `plugins/` resolve to valid targets.
