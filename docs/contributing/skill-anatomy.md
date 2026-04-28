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
argument-hint: "description? artifact-id?"
allowed-tools: Read Write StrReplace Shell Grep Glob ReadLints WebFetch
---
```

### Field order

Frontmatter fields appear in this canonical order:

1. `name`
2. `description`
3. `license`
4. `argument-hint`
5. `allowed-tools`
6. Optional extension fields, alphabetically (e.g. `compatibility`, `disable-model-invocation`, `metadata`, `paths`, `user-invocable`, `when_to_use`)

### Frontmatter fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Skill identifier in kebab-case; lowercase letters, numbers, and hyphens; max 64 characters; must match the parent directory name. Avoid the reserved words `anthropic` and `claude` (Anthropic platform policy). |
| `description` | yes | One-line description (minimum 10, maximum 1024 characters per Anthropic spec) including when to use the skill. Avoid XML tags. |
| `license` | no | License identifier: SPDX ID (e.g. `MIT`, `Apache-2.0`, `BSD-3-Clause`), a custom name, or a path to a bundled `LICENSE` file. |
| `argument-hint` | no | Argument pattern shown to the user using bare names for required and a `?` suffix for optional (e.g. `"crate-name?"`, `"source-path output-dir kind value source-key k?"`). Use literal pipes for choices (`a|b|c`) and `...` for repeatables. No `--` flag prefix, no angle or square brackets. This is a deliberate Specify house style; upstream tools treat the value as opaque text, but the divergence reduces cosmetic portability when the hint is surfaced inside Claude Code or copies of the Cursor docs verbatim. |
| `allowed-tools` | no | Space-separated list of tools the skill may use (matches Cursor and Claude Code spec); validated against a known set plus `mcp__*` prefixed tools. |
| `compatibility` | no | Environment requirements such as system packages or network access (Cursor). Accepts a string or object. |
| `metadata` | no | Arbitrary key-value mapping for additional metadata (Cursor). |
| `disable-model-invocation` | no | When `true`, the skill is only included when explicitly invoked via `/skill-name`; the agent will not auto-trigger it (Cursor + Claude Code). |
| `when_to_use` | no | Additional trigger phrases or example requests appended to `description` in the skill listing (Claude Code). Max 1024 characters. |
| `user-invocable` | no | When `false`, hide the skill from the `/` menu so only the agent can invoke it (Claude Code). |
| `paths` | no | Glob patterns that limit when this skill is auto-activated (Claude Code). Accepts a comma-separated string or YAML list. |

### Cursor-specific tool names

The `KNOWN_TOOLS` set enforced by [scripts/checks.ts](../../scripts/checks.ts) is the Cursor toolset:

```text
Read Write StrReplace Shell Grep Glob ReadLints WebFetch WebSearch
AskQuestion Task TodoWrite SemanticSearch EditNotebook GenerateImage
```

`mcp__*` prefixed tools are also accepted (MCP server tools).

Several of these are Cursor-only and do not exist in Claude Code: `StrReplace`, `ReadLints`, `SemanticSearch`, `AskQuestion`, `EditNotebook`, `GenerateImage`. Skills that reference them won't run cleanly on Claude Code or other Agent Skills consumers without substitutions.

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
                                       specify change validate <name>
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
