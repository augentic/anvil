# Consistency Checks

The `specify` repo includes an automated consistency checker at `scripts/checks.ts` that validates documentation, skills, capability manifests, and the marketplace manifest. Run it before every pull request.

## Running checks

```bash
make checks
```

This runs `scripts/checks.ts` via [Deno](https://deno.land):

```bash
deno run --allow-read --allow-env scripts/checks.ts
```

Exit code `0` means all checks pass. Any failure prints `FAIL: <description>` and exits non-zero with a count of failures.

## What the checks enforce

### 1. Markdown link resolution

Every relative link in every `.md` file must resolve to an existing file. External links (`http://`, `mailto:`, `#` anchors) and `src/` paths are skipped. Fenced code blocks and HTML comments are stripped before scanning.

**Common fix:** update the link target or remove a stale link.

### 2. Stale claims

No markdown file may reference a stale checklist count from an earlier version of the documentation. The specific patterns are defined in `scripts/checks.ts`.

### 3. Capability manifest YAML validation

Every `capabilities/<name>/capability.yaml` file (at most two directory levels deep) must validate against `capabilities/capability.schema.json` using JSON Schema 2020-12.

**Common fix:** check that all required fields (`name`, `version`, `description`, `pipeline`) are present and correctly typed.

### 4. Capability referential integrity

For each `capability.yaml`, the check validates:

- **Brief files exist** -- every pipeline entry's `brief` path resolves to a file
- **Frontmatter present** -- each brief file has valid YAML frontmatter between `---` markers
- **ID match** -- the brief frontmatter `id` matches the pipeline entry `id`
- **Needs resolution** -- every `needs` reference in a brief points to a declared pipeline `id`
- **No cycles** -- the `needs` dependency graph is acyclic (verified by Kahn's topological sort)

**Common fix:** ensure the brief's frontmatter `id` matches the pipeline entry, and that `needs` references use exact `id` values from the same capability.

### 5. Symlink integrity

Every symlink under `plugins/` must resolve to a valid target. Skills often symlink shared references (e.g. `plugins/references/specify.md`) into their `references/` directory.

**Common fix:** recreate the symlink if the target was moved or renamed.

### 6. SKILL.md frontmatter validation

Every `SKILL.md` under `plugins/` is validated against `.cursor/schemas/skill.schema.json`:

- **Required fields** -- `name` (kebab-case) and `description` (minimum 10 characters)
- **Name match** -- the `name` field must match the parent directory name
- **Known tools** -- every entry in `allowed-tools` must be a recognized Cursor tool name or match the `mcp__*` prefix

The recognized tool set includes: `Read`, `Write`, `StrReplace`, `Shell`, `Grep`, `Glob`, `ReadLints`, `WebFetch`, `WebSearch`, `AskQuestion`, `Task`, `TodoWrite`, `SemanticSearch`, `EditNotebook`, `GenerateImage`.

Long `SKILL.md` bodies are also checked for structure: bodies over 500 post-frontmatter lines fail, and bodies with at least 150 post-frontmatter lines must include a `## Critical Path (Quick Reference)` section with 5-7 bullets or numbered items.

### 7. Skill reference link resolution

Links in `SKILL.md` bodies that point to `references/...` or `examples/...` paths are resolved relative to the skill directory. Every such link must resolve to an existing file.

### 8. Skill variable consistency

For skills that declare an `## Arguments` or `## Derived Arguments` section with `$VARIABLE = ...` definitions in ` ```text` blocks:

- Every defined variable must be referenced somewhere in the skill body
- Every `$VARIABLE` reference in the body (outside fenced blocks) must have a definition in the arguments section

Built-in variables (`$ARGUMENTS`, `$HOME`) are excluded from the check.

### 9. Skill directive validation

`<!-- skill: plugin:skill-name -->` directives in markdown files must reference a real skill. The check walks `plugins/` to build a registry of `plugin → skill` mappings and validates every directive against it. Files under `rfcs/` are excluded.

### 10. Marketplace manifest consistency

Cross-checks `plugins/` against `.cursor-plugin/marketplace.json`:

- Every plugin with a `.cursor-plugin/plugin.json` file must be listed in the manifest
- Every plugin listed in the manifest must have a `skills/` directory

### 12. Instruction file preambles

Files matching `capabilities/**/instructions/<name>.md` must contain an output location preamble:

```markdown
> **Output location**: `.specify/slices/...`
```

This prevents cross-plugin path contamination by making every instruction file declare where its output goes.

## Extending the checks

To add a new check:

1. Write an `async function` in `scripts/checks.ts` following the existing pattern.
2. Call `fail(msg)` for each violation -- this increments the error counter and prints the failure.
3. Add the function to one of the `Promise.all` groups at the bottom of the file. Independent checks can run in the same group; checks that depend on earlier results go in a later group.
4. Run `make checks` to verify the new check works.

The checks are numbered but the numbers are not contiguous (check 11 does not exist). New checks should use the next available number.

## CLI checks

The specify-cli repo has its own check suite via `cargo-make`:

```bash
cargo make ci     # lint, test, test-docs, vet, outdated, deny, fmt
cargo make check  # audit, fmt, lint, outdated, deps
```

These are Rust-specific checks (clippy, formatting, dependency auditing, test suite) and are separate from the documentation checks in the specify repo.
