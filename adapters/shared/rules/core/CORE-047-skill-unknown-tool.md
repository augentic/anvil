---
id: CORE-047
title: Skill Unknown Tool
severity: important
trigger: SKILL.md allowed-tools lists a tool name that is not recognized by the framework tool registry.
rule_hints:
  - kind: path-pattern
    value: "plugins/**/SKILL.md"
    description: Narrow the candidate set to plugin skill manifests before the allowed-tools check fires.
  - kind: set-coverage
    value: skill-allowed-tools
    description: For each skill, every `allowed-tools` entry must be covered by the recognised Cursor tool set in `config.allowed` (or match an `allowed-prefixes` exemption). One finding per uncovered tool.
    config:
      allowed:
        - Read
        - Write
        - StrReplace
        - Shell
        - Grep
        - Glob
        - ReadLints
        - WebFetch
        - WebSearch
        - AskQuestion
        - Task
        - TodoWrite
        - SemanticSearch
        - EditNotebook
        - GenerateImage
      allowed-prefixes:
        - "mcp__"
---

## Rule

Every entry in a skill's `allowed-tools` frontmatter must name a tool the framework recognises. The recognised set is the closed list of built-in Cursor tools, plus dynamically-named MCP tools whose names carry the `mcp__` prefix. A typo or an invented tool name silently disables a tool the skill expected, so unknown entries are flagged.

The deterministic-hint interpreter consumes the skill frontmatter the framework indexer already produced, restricted to the `plugins/**/SKILL.md` candidate set. The recognised tool set and the `mcp__` prefix exemption are policy carried in the rule's `config:`, not the engine. Skills that omit `allowed-tools` declare no tools and are never flagged.

## Look For

- An `allowed-tools` entry misspelling a built-in tool (`Reed` for `Read`).
- An `allowed-tools` entry naming a tool that does not exist in the framework registry.

## Fix

Correct the tool name to a recognised built-in, drop the entry if the tool is not needed, or use the `mcp__<server>__<tool>` form for an MCP tool.
