# Anatomy of a Skill

A skill is a markdown file (`SKILL.md`) that instructs a Cursor agent how to perform a specific task. Skills are the primary unit of behavior in Specify -- every `/spec:*`, `/change:*`, `/omnia:*`, `/vectis:*`, `/contract:*`, `/rt:*`, and `/client:*` command maps to one skill.

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

Every `SKILL.md` begins with YAML frontmatter validated against `.cursor/schemas/skill.schema.json`:

```yaml
---
name: specify-define
description: Defines a new Specify slice and generates every artifact (proposal, spec, design, tasks, optional contracts and composition) in one step. Use when an operator describes a slice in chat, when a plan entry transitions in-progress, or when the user explicitly asks for /spec:define.
argument-hint: "[description]"
---
```

### Field order

Frontmatter fields appear in this canonical order:

1. `name`
2. `description`
3. `argument-hint` (optional)
4. `allowed-tools` (optional; rare)

No other top-level keys are permitted. RFC-10 (§D) removed `license`, `compatibility`, `metadata`, `disable-model-invocation`, `when_to_use`, `user-invocable`, and `paths` from the accepted shape; host-specific fields such as `context` are likewise rejected by the closed schema. `make checks` rejects any extra top-level key.

### Frontmatter fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Globally unique, plugin-qualified, kebab-case identifier (`^[a-z][a-z0-9-]*$`, ≤64 chars). Must start with the containing plugin's directory name plus `-` (e.g. `omnia-crate-writer`, `vectis-core-writer`, `contract-openapi`). The `spec/` plugin uses the `specify-` prefix per RFC §A.1 (so `plugins/spec/skills/init/` carries `name: specify-init`). Reserved words `anthropic` and `claude` are not allowed. |
| `description` | yes | Description that includes both *what* the skill does and *when* to use it, in third person (10–512 characters). Avoid XML tags and avoid RFC / layer citations — those belong in the body. |
| `argument-hint` | no | Cursor placeholder text shown after the user types the slash command. Single short hint with `<>` for required and `[]` for optional positional arguments; no flag names; no trailing `?`; no `--` prefix; avoid alternation pipes outside bracketed enums. Secondary skill arguments belong in the body's "Invocation" section as positionals. |
| `allowed-tools` | no | Space-separated list of tools the skill may use. Recommended policy is to omit this field and inherit the caller's full toolbelt; see RFC §A.5 for the rationale. When set, values are validated against a known toolset plus `mcp__*` prefixed tools. |

### Argument-hint vs reference-doc synopsis

The `argument-hint` field is single-line Cursor placeholder text — the operator sees it after typing `/plugin:skill ` in chat. Use `<required>` for required positionals and `[optional]` for optional positionals; never list flags in the hint.

The reference docs under `docs/reference/slice-skills/` and `docs/reference/change-skills/` use a **narrative** synopsis convention that documents secondary positionals as well:

```text
/spec:define [description] [source <key>=<path-or-url>...]
```

This is documentation-only. Contributors copying a synopsis line from a reference doc into a SKILL.md frontmatter must reduce it to a single placeholder for the primary positional argument; secondary-positionals documentation goes into the body's "Invocation" section. Slash-skill examples must not use `--flag` notation; reserve that shape for underlying CLI commands.

### Cursor plugin posture

This repository is **Cursor-plugin-first**. Source `SKILL.md` files are authored for the Cursor plugin marketplace and the `/plugin:skill` slash-command surface, then kept close to Anthropic Agent Skills conventions where that does not weaken the Cursor workflow.

The following conventions are Cursor-specific and should not be treated as portable upstream Agent Skills syntax:

- Plugin manifests under `.cursor-plugin/` and per-plugin `plugin.json` files.
- Slash commands such as `/spec:define` and `/omnia:crate-writer`, which route by plugin directory plus skill directory.
- `argument-hint`, which is Cursor placeholder text for a slash command, not a general usage synopsis.
- `<!-- skill: plugin:skill-name -->` directives, which tell the Cursor agent to load another repository skill at that point in the procedure.
- Cursor tool names in `allowed-tools`, especially the Cursor-only tools listed below.

### Cursor-specific tool names

The `KNOWN_TOOLS` set enforced by [scripts/checks.ts](../../scripts/checks.ts) is the Cursor toolset:

```text
Read Write StrReplace Shell Grep Glob ReadLints WebFetch WebSearch
AskQuestion Task TodoWrite SemanticSearch EditNotebook GenerateImage
```

`mcp__*` prefixed tools are also accepted (MCP server tools).

Several of these are Cursor-only and do not exist in Claude Code: `StrReplace`, `ReadLints`, `SemanticSearch`, `AskQuestion`, `EditNotebook`, `GenerateImage`. Skills that reference them won't run cleanly on Claude Code or other Agent Skills consumers without substitutions.

### Claude Code / Agent Skills export

Do not add Claude Code-only frontmatter to source skills unless the repository policy changes. Claude Code and Agent Skills consumers may support optional fields such as `disable-model-invocation`, `user-invocable`, `context`, `paths`, or appended trigger metadata, but this repository keeps source frontmatter small and validates it with `additionalProperties: false`.

A future Claude Code or Agent Skills export should be a separate profile or generated artifact. That export would need to:

- Map or remove Cursor-only metadata such as `argument-hint`.
- Replace Cursor-only tools with host-supported equivalents.
- Translate `<!-- skill: plugin:skill-name -->` delegation into that host's composition mechanism, or inline explicit instructions for loading the referenced skill.
- Decide per exported skill whether Claude Code fields such as `disable-model-invocation`, `user-invocable`, or `context` are appropriate.

Side-effect workflow skills currently omit `disable-model-invocation` intentionally. In Cursor, `/spec:*` and `/change:*` skills are user-invoked or pipeline-invoked orchestrators whose bodies and CLI calls gate mutations at runtime. Hiding them from model invocation in the source profile would make direct operator requests and plan-driven delegation less reliable. If a Claude Code export needs stricter user-only activation for mutating workflows, add that field in the export profile rather than in the Cursor source skill.

### Body sections

The body after the frontmatter varies by skill type, but the following patterns are common across the codebase:

**Workflow skills** (e.g. `/spec:define`, `/spec:build`, `/spec:merge`) typically include:

- **Context** -- when the skill runs, what state it expects, how it fits into the workflow
- **Driver-supplied arguments** -- arguments passed by `/change:execute` in plan-driven mode
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
                                       specify slice create <name>
                                  ←    { "status": "created", ... }
Read brief, write artifact       →
                                       specify slice validate <name>
                                  ←    { "passed": true, ... }
                                       specify slice transition <name> defined
                                  ←    { "status": "defined", ... }
```

The litmus test: "Would this operation need to understand `.specify/` directory structure or spec format?" If yes, it belongs in the CLI. If no (like running `cargo test` or writing a Rust file), it stays with the agent.

## How skills delegate to other skills

Skills can invoke other skills using `<!-- skill: plugin:skill-name -->` directives in their body. For example, `/spec:build` delegates implementation to specialist skills declared by the active capability:

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

2. **Write the frontmatter.** The `name` field must be globally unique, plugin-qualified (start with the containing plugin's directory name + `-`, with `specify-` for the `spec/` plugin), and lowercase-kebab-case. Include a `description` (10–512 characters) that names both *what* the skill does and *when* to use it.

3. **Write the body.** Lead with a `## Critical Path` 5–7 entry block (numbered/bullet items, or `### N. Title` H3 step headings) when the body will exceed 150 lines. Keep total body length under 250 lines; factor longer content into sibling files (`rules.md`, `team-protocol.md`, `categories.md`, `template.md`, etc.) linked one level deep. Generator skills should define an authority hierarchy in a sibling; workflow skills should document the phase outcome contract via the shared reference at `plugins/spec/references/phase-outcome-contract.md`.

4. **Add references** if needed. Place supporting documents in a `references/` subdirectory or alongside SKILL.md as `<topic>.md`, and link to them from the skill body using relative paths like `./rules.md` or `references/guardrails.md`.

5. **Register the skill** in the plugin's `.cursor-plugin/plugin.json` if one exists. The marketplace manifest at `.cursor-plugin/marketplace.json` declares plugins by `source` directory; individual skills are discovered by directory walking.

6. **Run `make checks`** to verify:
   - Frontmatter validates against `.cursor/schemas/skill.schema.json`
   - `name` is globally unique, plugin-qualified, and matches `^[a-z][a-z0-9-]*$`
   - SKILL.md body (post-frontmatter) is ≤250 lines (per-file `bodyLineCount` baselines in `scripts/standards-allowlist.toml` grandfather oversized files)
   - SKILL.md bodies with ≥150 post-frontmatter lines include `## Critical Path` with 5-7 bullets, numbered items, or `### N. Title` H3 step headings
   - `description` is ≤512 characters
   - `argument-hint` does not contain `?`, `--`, or `|`
   - No retired or host-specific top-level keys (`license`, `compatibility`, `metadata`, `disable-model-invocation`, `when_to_use`, `user-invocable`, `context`, `paths`)
   - Any `allowed-tools` entries are recognized
   - All `references/` and `examples/` links resolve
   - All `$VARIABLE` definitions are used and all uses are defined

## Shared references

Two files in `plugins/references/` are shared across plugins:

- **`specify.md`** -- the master reference for artifact format, lifecycle states, naming conventions, delta-merge rules, and hard constraints that apply to every skill
- **`agent-teams.md`** -- the multi-agent review pattern used by reviewer skills (structural, logic, and quality specialists plus an antagonist)

Skills that need these references typically symlink them into their own `references/` directory so relative paths resolve. The `checks.ts` script validates that all symlinks under `plugins/` resolve to valid targets.
