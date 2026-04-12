# Type Safety in Skill Expression

Recommendations for adding compile-time feedback to skill definitions,
catching broken links, invalid references, and structural errors before
they reach an LLM.

## The Problem

Skills are expressed in natural language markdown. This means they are
inherently not type-safe: a broken reference path, a misspelled tool name,
or an undefined variable are all invisible until the agent encounters them
at runtime. The repository already has partial validation via `checks.ts`
(link resolution, cycle detection, blueprint integrity, symlink validation,
frontmatter schema), but SKILL.md bodies remain structurally unvalidated.

## Landscape: What Others Are Doing

### BAML (Boundary) -- Typed Language for AI Functions

The closest thing to "compiled prompts" in production. BAML provides typed
function signatures, a compiler that validates prompt templates reference
valid fields, and IDE support with syntax highlighting and type errors.
Generates client code in Python/TypeScript/Ruby.

**Relevance**: BAML is oriented toward structured LLM function calls
(extract X from Y), not complex multi-step agent skills. Specify skills are
more like "behavioral contracts + execution playbooks" than typed function
signatures. The type-checking approach is worth studying, but it does not
map directly.

### Priompt / JSX-Style Prompt Construction

Primer's open-source [Priompt](https://github.com/anysphere/priompt) (used
in Cursor) models prompts as a JSX component tree with priority-based
truncation. Each node is typed, composable, and testable programmatically.

**Relevance**: More about runtime prompt assembly and token-budget
management than compile-time skill correctness. The composability model is
interesting -- skills could be components that compose with typed props --
but this is not the primary gap today.

## Proposed Approach: Two Phases

### Phase 1 -- Extend `checks.ts` (Immediate, Low Friction)

Push the existing validation infrastructure further without changing the
authoring format. This gets 80% of the value with minimal disruption.

> **Status**: Most of Phase 1 is now implemented. The current `checks.ts`
> includes frontmatter schema validation, skill reference resolution,
> variable consistency checking, skill directive validation,
> cross-plugin marketplace consistency, and `docs/plugins.md` inventory
> alignment.

#### What Phase 1 Catches

| Failure Mode | Check |
|---|---|
| Typo in `allowed-tools` (e.g., `Readlints` vs `ReadLints`) | Frontmatter schema validation |
| Skill name does not match directory name | Frontmatter validation |
| SKILL.md references `references/sdk-api.md` but symlink not created | Skill reference resolution |
| Variable `$CRATE_PATH` used but never defined in Arguments block | Variable consistency |
| `<!-- skill: omnia:crate-writter -->` (typo) in tasks template | Skill directive validation |
| Plugin added to filesystem but not to `marketplace.json` | Plugin consistency |
| Frontmatter missing required `description` field | Frontmatter schema |
| Skill listed in docs but not on disk (or vice versa) | `docs/plugins.md` inventory |

### Phase 2 -- Rust DSL That Compiles to SKILL.md (Medium-Term)

Skills have two distinct layers: **structural metadata** (dependencies,
tools, arguments, phase ordering) and **behavioral instructions** (how the
agent should think and act). The structural layer benefits enormously from
typing. The behavioral layer is inherently natural language.

The idea: define skills as Rust structs, embed prose blocks via
`include_str!`, and generate SKILL.md at build time. The Rust compiler and
a build script then enforce correctness.

#### What the Compiler Would Catch

1. **Broken references**: `ref!(omnia::nonexistent)` fails to compile
   because the target does not exist as a const/type.
2. **Blueprint alignment**: `artifact!(design)` checks against an enum
   generated from `schema.yaml`.
3. **Phase DAG validation**: dependency cycles or missing phases caught at
   compile time.
4. **Tool allow-lists**: enum-based, so typos are impossible.
5. **Cross-skill directives**: typed consts validated against a registry.
6. **Variable DAGs**: `depends_on` fields are checked for completeness and
   acyclicity.

#### Sketch: Core Types

```rust
pub struct SkillDef {
    pub name: SkillId,
    pub plugin: PluginId,
    pub description: &'static str,
    pub license: License,
    pub arguments: Arguments,
    pub allowed_tools: &'static [Tool],
    pub references: &'static [Reference],
    pub authority: &'static [AuthorityLevel],
    pub rules: &'static [HardRule],
    pub phases: Vec<Phase>,
    pub modes: Vec<Mode>,
    pub body_sections: Vec<Section>,
}

pub enum Tool {
    Read, Write, StrReplace, Shell, Grep, Glob,
    ReadLints, WebFetch, WebSearch, AskQuestion,
    Task, TodoWrite, SemanticSearch,
    Mcp(&'static str),
}

pub struct Reference {
    pub id: &'static str,
    pub path: &'static str,
    pub description: &'static str,
    pub mode: RefMode,
}

pub struct Arguments {
    pub positional: &'static [Arg],
    pub derived: &'static [DerivedVar],
}
```

#### Sketch: Skill Definition

```rust
pub fn crate_writer() -> SkillDef {
    SkillDef {
        name: SkillId("crate-writer"),
        plugin: PluginId("omnia"),
        description: "Write Rust WASM crates from Specify artifacts...",
        license: License::Mit,
        allowed_tools: &[
            Tool::Read, Tool::Write, Tool::StrReplace,
            Tool::Shell, Tool::Grep, Tool::ReadLints,
        ],
        arguments: Arguments {
            positional: &[
                Arg { name: "crate-name", var: "CRATE_NAME", position: 0, required: true },
            ],
            derived: &[
                DerivedVar {
                    name: "CHANGE_DIR",
                    expr: ".specify/changes/$CRATE_NAME",
                    depends_on: &["CRATE_NAME"],
                },
            ],
        },
        references: &[
            Reference {
                id: "sdk-api",
                path: "references/sdk-api.md",
                description: "Handler<P>, Context, Reply types",
                mode: RefMode::Both,
            },
        ],
        // ...
    }
}
```

#### Build Integration

The Makefile would chain generation before validation:

```makefile
.PHONY: generate
generate:
	cargo run --manifest-path skill-dsl/Cargo.toml --bin generate

.PHONY: checks
checks: generate
	@$(DENO) run --allow-read scripts/checks.ts
```

## Pragmatic Middle Ground: YAML Manifests

If the full Rust DSL is too much ceremony for ~25 skills, a middle path
exists: **YAML skill manifests validated by JSON Schema, with prose staying
in markdown.**

```yaml
# plugins/omnia/skills/crate-writer/manifest.yaml
name: crate-writer
plugin: omnia
description: "Write Rust WASM crates from Specify artifacts..."
license: MIT
argument-hint: "[crate-name]"

allowed-tools:
  - Read
  - Write
  - StrReplace
  - Shell
  - Grep
  - ReadLints

arguments:
  positional:
    - name: crate-name
      var: CRATE_NAME
      position: 0
      required: true
  derived:
    - name: CHANGE_DIR
      expr: ".specify/changes/$CRATE_NAME"
      depends_on: [CRATE_NAME]

references:
  - id: sdk-api
    path: references/sdk-api.md
    mode: both

skill-directives:
  - omnia:test-writer
  - omnia:guest-writer
```

The manifest is validated by JSON Schema in `checks.ts` and cross-checked
against the SKILL.md frontmatter. This gives structured metadata without
requiring Rust. The Rust DSL mainly wins on ergonomics (IDE autocomplete,
type inference) and on the ability to programmatically compose or derive
skills -- which matters more as the skill count grows.

## Comparison

| Dimension | Phase 1 (Extended checks.ts) | Phase 2 (Rust DSL) | YAML Manifests |
|---|---|---|---|
| **Authoring format** | Markdown directly | Rust structs + `include_str!` | YAML + markdown |
| **Feedback loop** | CI-time (`make checks`) | Compile-time (`cargo build`) | CI-time (`make checks`) |
| **Broken references** | Runtime file-exists check | Build script panic | Runtime file-exists check |
| **Tool name typo** | String comparison | Enum variant -- won't compile | String against schema enum |
| **Variable consistency** | Regex-based heuristic | Typed `depends_on` DAG | Schema-validated DAG |
| **Adoption effort** | ~200 lines TypeScript | New crate + migration | JSON Schema + cross-check |
| **Authoring friction** | None -- still markdown | Moderate -- Rust for skeleton | Low -- YAML is familiar |
| **Composability** | Limited | High -- skills are data | Medium |

## Recommendation

Phase 1 is worth doing regardless -- even if the Rust DSL is built later,
`checks.ts` should continue validating rendered output as defense in depth.
Phase 2 is the long-term target for a Rust shop, but only justified once
skill count and complexity make the authoring ceremony worthwhile. The YAML
manifest path is available as a pragmatic intermediate step.
