# Type-Safe Skill Expression

## The Two-Layer Problem

Skills have two distinct layers with fundamentally different validation needs:

1. **Structural metadata** — what a skill depends on, what tools it uses, what arguments it takes, what phases it runs, what artifacts it references. This layer is a graph of typed relationships that can be validated mechanically.

2. **Behavioral instructions** — how the agent should think and act. This layer is inherently natural language and should stay that way.

Today both layers live in untyped markdown. YAML frontmatter has no schema enforcement, and the prose body uses conventions (variable names, reference links, section headings) with no structural validation. Breaking a reference or misspelling a tool name produces no feedback until runtime.

## Relationship to the CLI and `checks.ts`

The [CLI (Horizon 1)](cli.md) addresses the most acute validation gaps — artifact structure, spec format, task tracking — through `specify validate`. That work is prerequisite. This horizon extends the same principle to skill authoring itself: deterministic checks for the structural layer, agent judgment for the behavioral layer.

Importantly, `checks.ts` (the existing Deno validation script) already implements the core of Option 1 below: frontmatter schema enforcement, reference resolution, variable consistency, skill directive validation, marketplace consistency, and docs inventory checks. The primary gap for this horizon is not designing these checks but porting them from TypeScript into the `specify-check` crate — a migration that happens naturally as part of Horizon 1's `specify check` subcommand. Once that port is complete, the incremental work for this horizon is small.

## Options

Three approaches, ranked by investment:

### 1. CLI-integrated skill validation (low friction)

Add skill-aware checks to `specify validate`:

- **Frontmatter schema** — validate `name`, `description`, `argument-hint`, `allowed-tools` against a JSON Schema; verify skill name matches directory name
- **Reference resolution** — every `references/` link in a SKILL.md must resolve to an existing file
- **Variable consistency** — variables defined in the `## Arguments` block must be used in the body, and vice versa
- **Skill directive validation** — `<!-- skill: plugin:name -->` directives in task templates must reference real skills
- **Plugin consistency** — every plugin directory must appear in `marketplace.json` and have at least one skill

This catches typos, broken links, and structural drift without changing the authoring format.

### 2. YAML skill manifests (moderate friction)

Extract the structural metadata from SKILL.md into a companion `manifest.yaml` per skill, validated by JSON Schema. The manifest declares arguments, references, tool allow-lists, authority levels, and cross-skill directives as structured data. The SKILL.md prose stays hand-authored. The CLI cross-checks that the manifest and the SKILL.md frontmatter agree.

This separates the two layers explicitly. Authoring friction is low — YAML is familiar and IDE-supported. Validation power is comparable to a full DSL for the structural layer.

### 3. Rust DSL that compiles to SKILL.md (high investment)

Model the structural skeleton in Rust — typed structs for skills, enums for tools and authority levels, `include_str!` for prose blocks. A build script validates reference paths, variable DAGs, phase dependencies, and cross-skill directives at compile time. `cargo build` fails if a reference is broken or a tool name is misspelled.

The Rust compiler gives you broken-reference detection, exhaustive enum matching for tool names, and typed cross-skill references. The trade-off is maintaining a Rust build alongside the markdown and migrating all existing skills into the DSL. This pays off as the skill count grows and composability becomes important — skills become data that can be derived, composed, and tested programmatically.

## Recommendation

Option 1 is already mostly implemented in `checks.ts` and will be ported to Rust as part of Horizon 1's `specify-check` crate. Once that port is complete, this horizon's Option 1 is done — no additional design or implementation beyond what the CLI migration delivers.

Option 2 is the right next step if skill count grows beyond ~20 and structural drift becomes a recurring problem. Option 3 is justified only when skills need to compose programmatically (e.g., generating variant skills from a base definition) or when the skill count makes manual consistency impractical.

Revisit options 2 and 3 when the ported validation catches real bugs and the failure modes point toward stronger typing.
