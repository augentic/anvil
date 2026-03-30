# Type safety in skill expression
_Exported on 18/03/2026 at 5:58:40 GMT+13 from Cursor (2.6.20)_

---

**User**

The issue with skills is that they are expressed in natural language. This means they are inherrently not type-safe like, for example, Rust. I want a way to build skills that gives me better "compile-time" feedback when I break a link, do something unreasonable, or otherwise break something.

Do you have any suggestions based on what others are doing. For example, building skills dynalically from a strongly typed base such as Rust? 

---

**Cursor**

Great question. Let me first look at your current skill structure to ground the discussion, then I'll address the broader landscape.

Good question. You've already built more validation than most -- your `schema.schema.json`, `checks.ts` (link resolution, cycle detection, blueprint integrity, symlink validation), and the schema.yaml structure give you a partial "type system" for the blueprint layer. But the SKILL.md files themselves are essentially untyped: YAML frontmatter with no schema enforcement, and free-form markdown bodies with conventions but no structural validation.

Here's what I see in the ecosystem and what I'd consider for your situation:

---

## 1. BAML (Boundary) -- Typed language for AI functions

The closest thing to "compiled prompts" in production today. BAML gives you a dedicated language with:
- Typed function signatures (inputs/outputs with structured types)
- A compiler that validates prompt templates reference valid fields
- IDE support with syntax highlighting and type errors
- Generates client code in Python/TypeScript/Ruby

**Relevance to you**: BAML is oriented toward structured LLM function calls (extract X from Y), not complex multi-step agent skills. Your skills are more like "behavioral contracts + execution playbooks" than typed function signatures. It's worth studying their type-checking approach, but it doesn't map directly to your skill shape.

## 2. Rust DSL that compiles to SKILL.md

This is probably the most natural fit for a Rust shop. The idea:

```rust
skill! {
    name: "crate-writer",
    description: "Generate Rust WASM crates from Specify artifacts",
    
    requires: [
        ref!(omnia::guardrails),
        ref!(omnia::capabilities),
        ref!(omnia::guest_wiring),
        artifact!(design),  // compile error if blueprint doesn't exist
        artifact!(spec),
    ],
    
    tools: [Read, Write, StrReplace, Shell, Grep],
    
    arguments: [
        arg!(crate_path: String, required: true),
        arg!(mode: CrateWriterMode, required: false, default: Auto),
    ],
    
    authority: [SkillMd, Artifacts, References, Source, Inference],
    
    phases: [
        phase!(detect_mode, "Determine create vs update"),
        phase!(read_artifacts, "Load spec and design", requires: [detect_mode]),
        phase!(generate, "Write or update crate", requires: [read_artifacts]),
        phase!(validate, "cargo check + clippy", requires: [generate]),
    ],
}
```

The Rust compiler gives you:
- **Broken references**: `ref!(omnia::nonexistent)` fails to compile because the target doesn't exist as a const/type
- **Blueprint alignment**: `artifact!(design)` checks against a generated enum from `schema.yaml`
- **Phase DAG validation**: dependency cycles or missing phases caught at compile time
- **Tool allow-lists**: enum-based, so typos are impossible

The proc macro or build script emits the markdown SKILL.md files. You'd keep a `cargo build` step in your `make checks` pipeline.

**Trade-off**: You now maintain a Rust build alongside the markdown. But since your downstream consumers are Rust shops anyway, this isn't foreign tooling.

## 3. Priompt / JSX-style prompt construction

Primer's open-source [Priompt](https://github.com/anysphere/priompt) (used in Cursor itself) models prompts as a JSX component tree with priority-based truncation. Each node is typed, composable, and can be tested programmatically.

**Relevance**: This is more about runtime prompt assembly and token-budget management than compile-time skill correctness. But the composability model is interesting -- skills could be components that compose, with typed props.

## 4. Graph-based validation (extend what you have)

Your `checks.ts` already validates blueprint DAGs and symlinks. You could push this further without changing the authoring format:

- **JSON Schema for SKILL.md frontmatter**: Validate `name`, `description`, `argument-hint`, `allowed-tools` against a schema (like you do for `schema.yaml`)
- **Structural linting for skill bodies**: Require specific H2 sections (`## Authority Hierarchy`, `## Hard Rules`, etc.), validate that `$CHANGE_DIR` variables are defined, check that tool names in prose match the frontmatter allow-list
- **Cross-skill reference graph**: Build a dependency graph from symlinks + markdown links, validate completeness (every skill that references `guardrails.md` actually has the symlink)
- **Artifact-blueprint alignment**: Validate that skills referencing `design.md` belong to a schema whose blueprints include `design`

This is the lowest-friction path -- you keep markdown authoring but add progressively stricter validation.

## 5. Effect-system / trait-bound modeling

This is more speculative but very Rust-idiomatic. Model skills the way you model Omnia handlers:

```rust
trait Skill {
    type Input;
    type Output;
    type Requires: SkillDependencies;  // compile-time capability requirements
    
    fn validate(&self) -> Result<(), SkillError>;
    fn render(&self) -> SkillMarkdown;
}

// A crate-writer skill requires these capabilities
impl Skill for CrateWriter {
    type Input = CrateWriterArgs;
    type Output = CrateArtifact;
    type Requires = (DesignBlueprint, SpecBlueprint, OmniaGuardrails);
    // ...
}
```

If `DesignBlueprint` isn't provided by the schema, the trait bound fails. This mirrors how your Omnia handlers use `P: Config + HttpRequest + Identity` -- the provider trait bounds are your "capability type system."

---

## My recommendation for your situation

Given where you are today, I'd suggest a **two-phase approach**:

**Phase 1 (immediate, low friction)**: Extend `checks.ts` with structural validation of SKILL.md files -- frontmatter schema, required sections, reference graph completeness, variable consistency. This gets you 80% of the value with minimal disruption. You already have the infrastructure.

**Phase 2 (medium-term)**: Build a small Rust crate in this repo that defines skills as typed structs and emits SKILL.md via a build step. Start with the metadata layer (references, arguments, tool lists, phase ordering) and leave the prose body as embedded markdown strings. This gives you compile-time checking on the structural skeleton while preserving natural language for the behavioral instructions that genuinely need to be free-form.

The key insight is that your skills have two distinct layers: **structural metadata** (what a skill depends on, what tools it uses, what arguments it takes, what phases it runs) and **behavioral instructions** (how the agent should think and act). The first layer benefits enormously from typing. The second layer is inherently natural language and should stay that way -- but even there, you can validate that referenced paths, variable names, and tool names are consistent with the typed metadata.

Would you like me to dig deeper into any of these approaches -- for example, what the Rust DSL or the extended `checks.ts` validation would look like concretely?

---

**User**

Yes, please dig deeper.

---

**Cursor**

Let me read the actual files to ground the concrete designs.

Now let me look at the reference graph to understand what cross-cutting links exist.

Now let me look at a few more cross-cutting pieces to understand the full reference web.

Good. Now I have the full picture. Here's the deep dive on both approaches.

---

## Phase 1: Extended `checks.ts` — Structural Validation of SKILL.md

This extends your existing infrastructure with minimal friction. You already validate `schema.yaml` against a JSON Schema and check symlinks/links. The gap is that SKILL.md files have zero validation today.

### 1a. SKILL.md Frontmatter Schema

You'd add a `skill.schema.json` alongside `schema.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://github.com/augentic/specify/schemas/skill.schema.json",
  "title": "SKILL.md frontmatter",
  "type": "object",
  "required": ["name", "description"],
  "additionalProperties": false,
  "properties": {
    "name": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*$",
      "description": "Skill identifier, must match directory name"
    },
    "description": {
      "type": "string",
      "minLength": 10,
      "description": "One-line description including 'Use when...' trigger"
    },
    "license": {
      "type": "string",
      "enum": ["MIT", "Apache-2.0", "proprietary"]
    },
    "argument-hint": {
      "type": "string",
      "pattern": "^\\[.*\\]$",
      "description": "Argument pattern using [required] [optional?] syntax"
    },
    "allowed-tools": {
      "type": "string",
      "description": "Comma-separated tool names"
    }
  }
}
```

### 1b. New Check Functions for `checks.ts`

Here's what you'd add to the existing check runner:

**Check 6 — SKILL.md frontmatter validation:**

```typescript
async function validateSkillFrontmatter(): Promise<void> {
  const skillSchema = JSON.parse(
    await Deno.readTextFile(join(SCHEMA_DIR, "skill.schema.json")),
  );
  const ajv = new Ajv2020({ allErrors: true });
  const validate = ajv.compile(skillSchema);

  const KNOWN_TOOLS = new Set([
    "Read", "Write", "StrReplace", "Shell", "Grep", "Glob",
    "ReadLints", "WebFetch", "WebSearch", "AskQuestion", "Task",
    "EditNotebook", "GenerateImage", "TodoWrite", "SemanticSearch",
  ]);

  for await (const entry of walk(join(REPO_ROOT, "plugins"), {
    match: [/SKILL\.md$/],
    includeDirs: false,
  })) {
    if (await isUnderSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);

    // Extract YAML frontmatter
    const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
    if (!fmMatch) {
      fail(`Missing frontmatter: ${rel}`);
      continue;
    }
    const fm = parseYaml(fmMatch[1]) as Record<string, unknown>;

    // Validate against schema
    if (!validate(fm)) {
      for (const err of validate.errors ?? []) {
        fail(`Skill frontmatter: ${rel} — ${err.instancePath} ${err.message}`);
      }
    }

    // Skill name must match directory name
    const dirName = entry.path.split("/").at(-2);
    if (fm.name && fm.name !== dirName) {
      fail(`Skill name mismatch: ${rel} — name '${fm.name}' != dir '${dirName}'`);
    }

    // Validate tool names
    if (typeof fm["allowed-tools"] === "string") {
      for (const tool of (fm["allowed-tools"] as string).split(",").map(t => t.trim())) {
        if (!KNOWN_TOOLS.has(tool) && !tool.startsWith("mcp__")) {
          fail(`Unknown tool in allowed-tools: ${rel} — '${tool}'`);
        }
      }
    }
  }
}
```

**Check 7 — Skill reference link resolution (within skill directories):**

Your existing link checker validates all markdown links repo-wide, but it doesn't understand the *semantic* relationship between a SKILL.md and its `references/` directory. This check would catch the case where a skill references `[guardrails.md](../plugins/omnia/references/guardrails.md)` but the symlink `references/guardrails.md` hasn't been created yet — even though the target file exists elsewhere.

```typescript
async function checkSkillReferences(): Promise<void> {
  const REF_LINK_RE = /\[([^\]]*)\]\((references\/[^)]+|examples\/[^)]+)\)/g;

  for await (const entry of walk(join(REPO_ROOT, "plugins"), {
    match: [/SKILL\.md$/],
    includeDirs: false,
  })) {
    if (await isUnderSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const skillDir = dirname(entry.path);
    const content = await Deno.readTextFile(entry.path);

    // Strip code blocks to avoid false positives
    const stripped = content.replace(/```[\s\S]*?```/g, "");

    for (const m of stripped.matchAll(REF_LINK_RE)) {
      const refPath = m[2].split("#")[0]; // strip anchors
      const resolved = resolve(skillDir, refPath);
      try {
        await Deno.stat(resolved);
      } catch {
        fail(`Skill reference missing: ${rel} links to '${refPath}' but it doesn't exist`);
      }
    }
  }
}
```

**Check 8 — Skill variable consistency:**

This checks that variables defined in the `## Arguments` section are actually used in the body, and that variables used in the body are defined.

```typescript
async function checkSkillVariables(): Promise<void> {
  const DEF_RE = /^\$([A-Z_]+)\s*=\s*/gm;
  const USE_RE = /\$([A-Z_]+)/g;
  const BUILTIN = new Set(["ARGUMENTS"]);

  for await (const entry of walk(join(REPO_ROOT, "plugins"), {
    match: [/SKILL\.md$/],
    includeDirs: false,
  })) {
    if (await isUnderSymlink(entry.path)) continue;
    const rel = relative(REPO_ROOT, entry.path);
    const content = await Deno.readTextFile(entry.path);
    const stripped = content.replace(/```[\s\S]*?```/g, "");

    // Find the Arguments section
    const argsMatch = content.match(/## Arguments\n\n```text\n([\s\S]*?)```/);
    if (!argsMatch) continue;

    const defined = new Set<string>();
    for (const m of argsMatch[1].matchAll(DEF_RE)) {
      defined.add(m[1]);
    }

    // Find all uses outside the arguments block
    const bodyAfterArgs = stripped.slice(stripped.indexOf("## Arguments") + argsMatch[0].length);
    const used = new Set<string>();
    for (const m of bodyAfterArgs.matchAll(USE_RE)) {
      if (!BUILTIN.has(m[1])) used.add(m[1]);
    }

    for (const v of defined) {
      if (!used.has(v)) {
        fail(`Unused variable: ${rel} — $${v} defined but never referenced in body`);
      }
    }
    for (const v of used) {
      if (!defined.has(v) && !BUILTIN.has(v)) {
        // Only warn for UPPER_CASE vars that look like skill variables
        if (/^[A-Z][A-Z_]+$/.test(v)) {
          fail(`Undefined variable: ${rel} — $${v} used but not defined in Arguments`);
        }
      }
    }
  }
}
```

**Check 9 — Skill directive validation:**

When `tasks.md` instructions reference `<!-- skill: omnia:crate-writer -->`, verify that `omnia` is a real plugin and `crate-writer` is a real skill within it.

```typescript
async function checkSkillDirectives(): Promise<void> {
  const DIRECTIVE_RE = /<!-- skill: ([a-z-]+):([a-z-]+) -->/g;
  const pluginsDir = join(REPO_ROOT, "plugins");

  // Build skill registry
  const registry = new Map<string, Set<string>>();
  for await (const entry of walk(pluginsDir, {
    match: [/SKILL\.md$/],
    includeDirs: false,
  })) {
    const parts = relative(pluginsDir, entry.path).split("/");
    // parts: ["omnia", "skills", "crate-writer", "SKILL.md"]
    if (parts.length >= 4 && parts[1] === "skills") {
      const plugin = parts[0];
      const skill = parts[2];
      if (!registry.has(plugin)) registry.set(plugin, new Set());
      registry.get(plugin)!.add(skill);
    }
  }

  // Scan all markdown files for directives
  for await (const entry of walk(REPO_ROOT, {
    exts: [".md"],
    includeDirs: false,
  })) {
    if (/node_modules|\.git/.test(entry.path)) continue;
    const content = await Deno.readTextFile(entry.path);
    const rel = relative(REPO_ROOT, entry.path);

    for (const m of content.matchAll(DIRECTIVE_RE)) {
      const [, plugin, skill] = m;
      if (!registry.has(plugin)) {
        fail(`Invalid skill directive: ${rel} — plugin '${plugin}' not found`);
      } else if (!registry.get(plugin)!.has(skill)) {
        fail(`Invalid skill directive: ${rel} — skill '${plugin}:${skill}' not found`);
      }
    }
  }
}
```

**Check 10 — Cross-plugin consistency with `marketplace.json`:**

```typescript
async function checkPluginConsistency(): Promise<void> {
  const manifest = JSON.parse(
    await Deno.readTextFile(join(REPO_ROOT, ".cursor-plugin", "marketplace.json")),
  );

  const declaredPlugins = new Set(manifest.plugins.map((p: any) => p.name));

  // Every directory under plugins/ with a .cursor-plugin/plugin.json should be in marketplace
  for await (const entry of walk(join(REPO_ROOT, "plugins"), {
    maxDepth: 2,
    match: [/plugin\.json$/],
    includeDirs: false,
  })) {
    const pluginName = relative(join(REPO_ROOT, "plugins"), entry.path).split("/")[0];
    if (!declaredPlugins.has(pluginName)) {
      fail(`Plugin '${pluginName}' has plugin.json but is not in marketplace.json`);
    }
  }

  // Every marketplace plugin should have at least one skill
  for (const p of manifest.plugins) {
    const skillsDir = join(REPO_ROOT, "plugins", p.source, "skills");
    try {
      const stat = await Deno.stat(skillsDir);
      if (!stat.isDirectory) {
        fail(`Plugin '${p.name}' has no skills/ directory`);
      }
    } catch {
      fail(`Plugin '${p.name}' declared in marketplace.json but skills/ not found`);
    }
  }
}
```

### What Phase 1 catches

| Failure mode | Check |
|---|---|
| Typo in `allowed-tools` (e.g. `Readlints` vs `ReadLints`) | Frontmatter validation |
| Skill name doesn't match directory name | Frontmatter validation |
| SKILL.md references `references/sdk-api.md` but symlink not created | Skill reference resolution |
| Variable `$CRATE_PATH` used but never defined in Arguments block | Variable consistency |
| `<!-- skill: omnia:crate-writter -->` (typo) in tasks template | Skill directive validation |
| Plugin added to filesystem but not to `marketplace.json` | Plugin consistency |
| Frontmatter missing required `description` field | Frontmatter schema |

This is all additive to your existing `checks.ts`. You wire them into the runner:

```typescript
await Promise.all([
  checkMarkdownLinks(),
  checkStaleClaims(),
  checkSymlinks(),
  validateSkillFrontmatter(),
  checkSkillReferences(),
  checkSkillVariables(),
  checkSkillDirectives(),
  checkPluginConsistency(),
]);
```

---

## Phase 2: Rust DSL That Compiles to SKILL.md

This is the more ambitious approach. The core idea: skills have a **typed skeleton** (metadata, references, arguments, phases, rules) and **prose blocks** (behavioral instructions). Model the skeleton in Rust, embed the prose as content literals, and generate SKILL.md at build time.

### Architecture

```text
specify/
├── skill-dsl/                    # New Rust crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                # Core types: Skill, Ref, Arg, Phase, Rule, etc.
│   │   ├── registry.rs           # Plugin/skill registry for cross-references
│   │   ├── render.rs             # SKILL.md markdown renderer
│   │   └── validate.rs           # Compile-time and runtime validations
│   ├── skills/                   # One module per skill
│   │   ├── mod.rs
│   │   ├── omnia_crate_writer.rs
│   │   ├── omnia_test_writer.rs
│   │   ├── spec_build.rs
│   │   ├── spec_define.rs
│   │   └── ...
│   └── tests/
│       └── render_test.rs        # Snapshot tests for rendered markdown
├── plugins/                      # Generated SKILL.md output (or checked in)
```

### Core Types

```rust
use std::path::PathBuf;

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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SkillId(&'static str);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PluginId(&'static str);

pub enum License { Mit, Apache2, Proprietary }

pub enum Tool {
    Read, Write, StrReplace, Shell, Grep, Glob,
    ReadLints, WebFetch, WebSearch, AskQuestion,
    Task, TodoWrite, SemanticSearch,
    Mcp(&'static str),  // e.g. Mcp("atlassian__*")
}

pub struct Reference {
    pub id: &'static str,
    pub path: &'static str,          // relative to skill dir
    pub description: &'static str,
    pub mode: RefMode,               // Both, CreateOnly, UpdateOnly
}

pub enum RefMode { Both, CreateOnly, UpdateOnly }

pub struct Arguments {
    pub positional: &'static [Arg],
    pub derived: &'static [DerivedVar],
}

pub struct Arg {
    pub name: &'static str,         // e.g. "crate-name"
    pub var: &'static str,          // e.g. "CRATE_NAME"
    pub position: usize,            // index into $ARGUMENTS
    pub required: bool,
}

pub struct DerivedVar {
    pub name: &'static str,         // e.g. "CHANGE_DIR"
    pub expr: &'static str,         // e.g. ".specify/changes/$CRATE_NAME"
    pub depends_on: &'static [&'static str],  // var names this references
}

pub enum AuthorityLevel {
    SkillMd,
    Artifacts,
    References,
    Examples,
    ExistingCode,
    OriginalSource,
    LlmInference,
}

pub struct HardRule {
    pub number: u32,
    pub text: &'static str,
    pub mode: RuleMode,             // Core, UpdateOnly, CreateOnly
}

pub enum RuleMode { Core, UpdateOnly, CreateOnly }

pub struct Phase {
    pub id: &'static str,
    pub title: &'static str,
    pub requires: &'static [&'static str],  // phase IDs
    pub content: &'static str,              // prose block
}

pub struct Mode {
    pub id: &'static str,
    pub condition: &'static str,    // e.g. "$CRATE_PATH/Cargo.toml exists"
    pub content: &'static str,      // full prose for this mode
}

pub struct Section {
    pub heading: &'static str,
    pub level: u8,                  // 2 = ##, 3 = ###
    pub content: &'static str,
}
```

### What the Compiler Catches

Here's the critical part — how Rust's type system enforces correctness:

**1. Reference paths must resolve:**

```rust
impl SkillDef {
    pub fn validate_references(&self, skill_dir: &Path) -> Vec<String> {
        let mut errors = vec![];
        for r in self.references {
            let resolved = skill_dir.join(r.path);
            if !resolved.exists() {
                errors.push(format!(
                    "Reference '{}' at path '{}' does not exist",
                    r.id, r.path
                ));
            }
        }
        errors
    }
}
```

But better — you can go further with const-time validation using a build script:

```rust
// build.rs
fn main() {
    println!("cargo:rerun-if-changed=plugins/");
    
    // Scan all skill definitions and validate references exist
    for skill in inventory::iter::<SkillDef> {
        let skill_dir = PathBuf::from("plugins")
            .join(skill.plugin.0)
            .join("skills")
            .join(skill.name.0);
        
        for r in skill.references {
            let path = skill_dir.join(r.path);
            if !path.exists() {
                panic!(
                    "SKILL VALIDATION FAILED: {}/{} references '{}' but {} does not exist",
                    skill.plugin.0, skill.name.0, r.id, path.display()
                );
            }
        }
    }
}
```

This means `cargo build` fails if a reference path is broken. Actual compile error.

**2. Cross-skill references are typed:**

```rust
pub struct SkillDirective {
    pub plugin: PluginId,
    pub skill: SkillId,
}

impl SkillDirective {
    pub const fn new(plugin: PluginId, skill: SkillId) -> Self {
        Self { plugin, skill }
    }
}

// In the skill definition:
const CRATE_WRITER: SkillDirective = SkillDirective::new(
    PluginId("omnia"),
    SkillId("crate-writer"),
);

const TEST_WRITER: SkillDirective = SkillDirective::new(
    PluginId("omnia"),
    SkillId("test-writer"),
);
```

The build script validates that every `SkillDirective` references a real plugin and skill. A typo like `SkillId("crate-writter")` would fail the build.

**3. Derived variables form a DAG:**

```rust
pub fn validate_variable_dag(args: &Arguments) -> Vec<String> {
    let defined: HashSet<&str> = args.positional.iter().map(|a| a.var)
        .chain(args.derived.iter().map(|d| d.name))
        .collect();
    
    let mut errors = vec![];
    for d in args.derived {
        for dep in d.depends_on {
            if !defined.contains(dep) {
                errors.push(format!(
                    "Derived variable '{}' depends on '{}' which is not defined",
                    d.name, dep
                ));
            }
        }
    }
    // Also: topological sort to detect cycles
    errors
}
```

**4. Tool allow-lists are exhaustive enums:**

```rust
// This is a compile error — no such variant:
allowed_tools: &[Tool::Read, Tool::Writ, Tool::Shell],
//                            ^^^^^^^^^ error[E0599]: no variant `Tool::Writ`
```

**5. Phase dependencies form a valid DAG:**

```rust
pub fn validate_phase_dag(phases: &[Phase]) -> Vec<String> {
    let ids: HashSet<&str> = phases.iter().map(|p| p.id).collect();
    let mut errors = vec![];
    
    for phase in phases {
        for req in phase.requires {
            if !ids.contains(req) {
                errors.push(format!(
                    "Phase '{}' requires '{}' which doesn't exist",
                    phase.id, req
                ));
            }
        }
    }
    // Cycle detection via topological sort...
    errors
}
```

### What a Skill Definition Looks Like

Here's `crate-writer` partially expressed in the DSL:

```rust
pub fn crate_writer() -> SkillDef {
    SkillDef {
        name: SkillId("crate-writer"),
        plugin: PluginId("omnia"),
        description: "Write Rust WASM crates from Specify artifacts -- greenfield \
            creation or incremental updates -- following Omnia SDK patterns with \
            provider-based dependency injection.",
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
                DerivedVar {
                    name: "SPECS_DIR",
                    expr: "$CHANGE_DIR/specs",
                    depends_on: &["CHANGE_DIR"],
                },
                DerivedVar {
                    name: "DESIGN_PATH",
                    expr: "$CHANGE_DIR/design.md",
                    depends_on: &["CHANGE_DIR"],
                },
                DerivedVar {
                    name: "CRATE_PATH",
                    expr: "crates/$CRATE_NAME",
                    depends_on: &["CRATE_NAME"],
                },
            ],
        },
        references: &[
            Reference {
                id: "sdk-api",
                path: "references/sdk-api.md",
                description: "Handler<P>, Context, Reply, IntoBody, Client, Error types",
                mode: RefMode::Both,
            },
            Reference {
                id: "capabilities",
                path: "references/capabilities.md",
                description: "all 9 provider traits with exact signatures and artifact triggers",
                mode: RefMode::Both,
            },
            Reference {
                id: "update-patterns",
                path: "references/update-patterns.md",
                description: "update strategy patterns by category",
                mode: RefMode::UpdateOnly,
            },
            // ... etc
        ],
        authority: &[
            AuthorityLevel::SkillMd,
            AuthorityLevel::Artifacts,
            AuthorityLevel::References,
            AuthorityLevel::Examples,
            AuthorityLevel::ExistingCode,
            AuthorityLevel::OriginalSource,
            AuthorityLevel::LlmInference,
        ],
        rules: &[
            HardRule { number: 1, mode: RuleMode::Core,
                text: "**Omnia SDK only** -- all errors return `omnia_sdk::Error`; \
                    no custom error types in public API" },
            HardRule { number: 10, mode: RuleMode::UpdateOnly,
                text: "**No regressions** -- the build orchestration layer captures \
                    a test baseline before changes..." },
            // ...
        ],
        modes: vec![
            Mode {
                id: "create",
                condition: "$CRATE_PATH/Cargo.toml does NOT exist",
                content: include_str!("../prose/crate-writer/create-mode.md"),
            },
            Mode {
                id: "update",
                condition: "$CRATE_PATH/Cargo.toml exists",
                content: include_str!("../prose/crate-writer/update-mode.md"),
            },
        ],
        phases: vec![],  // Modes define the phase sequence internally
        body_sections: vec![
            Section {
                heading: "Artifact Dispatch",
                level: 2,
                content: include_str!("../prose/crate-writer/artifact-dispatch.md"),
            },
            Section {
                heading: "Crate Structure",
                level: 2,
                content: include_str!("../prose/crate-writer/crate-structure.md"),
            },
            // ...
        ],
    }
}
```

### The Renderer

The renderer takes a `SkillDef` and produces the markdown SKILL.md:

```rust
pub fn render(skill: &SkillDef) -> String {
    let mut out = String::new();

    // Frontmatter
    out.push_str("---\n");
    writeln!(out, "name: {}", skill.name.0).unwrap();
    writeln!(out, "description: \"{}\"", skill.description).unwrap();
    // ... argument-hint, allowed-tools, license
    out.push_str("---\n\n");

    // Authority hierarchy
    if !skill.authority.is_empty() {
        out.push_str("## Authority Hierarchy\n\n");
        for (i, level) in skill.authority.iter().enumerate() {
            writeln!(out, "{}. **{}**", i + 1, level.display()).unwrap();
        }
        out.push('\n');
    }

    // Hard rules
    if !skill.rules.is_empty() {
        out.push_str("## Hard Rules\n\n");
        // Group by mode...
        for rule in skill.rules {
            writeln!(out, "{}. {}", rule.number, rule.text).unwrap();
        }
        out.push('\n');
    }

    // Arguments
    if !skill.arguments.positional.is_empty() {
        out.push_str("## Arguments\n\n```text\n");
        for arg in skill.arguments.positional {
            writeln!(out, "${:<15} = $ARGUMENTS[{}]", arg.var, arg.position).unwrap();
        }
        out.push('\n');
        for d in skill.arguments.derived {
            writeln!(out, "${:<15} = {}", d.name, d.expr).unwrap();
        }
        out.push_str("```\n\n");
    }

    // Required references
    if !skill.references.is_empty() {
        out.push_str("## Required References\n\n");
        for (i, r) in skill.references.iter().enumerate() {
            writeln!(out, "{}. [{}]({}) -- {}", i + 1, r.id, r.path, r.description).unwrap();
        }
        out.push('\n');
    }

    // Body sections (prose)
    for section in &skill.body_sections {
        let prefix = "#".repeat(section.level as usize);
        writeln!(out, "{prefix} {}\n", section.heading).unwrap();
        out.push_str(section.content);
        out.push_str("\n\n");
    }

    // Modes
    for mode in &skill.modes {
        writeln!(out, "## Mode: {}\n", mode.id).unwrap();
        writeln!(out, "**When**: {}\n", mode.condition).unwrap();
        out.push_str(mode.content);
        out.push_str("\n\n");
    }

    out
}
```

### Build Script + CI Integration

```rust
// build.rs or a standalone binary: skill-dsl/src/bin/generate.rs
fn main() {
    let skills = vec![
        omnia_crate_writer::crate_writer(),
        omnia_test_writer::test_writer(),
        spec_build::build(),
        spec_define::define(),
        // ...
    ];

    let mut errors = vec![];
    for skill in &skills {
        let skill_dir = PathBuf::from("plugins")
            .join(skill.plugin.0)
            .join("skills")
            .join(skill.name.0);

        // Validate references resolve
        errors.extend(skill.validate_references(&skill_dir));
        // Validate variable DAG
        errors.extend(validate_variable_dag(&skill.arguments));
        // Validate phase DAG
        errors.extend(validate_phase_dag(&skill.phases));
    }

    // Validate cross-skill directives
    let registry: HashMap<(&str, &str), &SkillDef> = skills.iter()
        .map(|s| ((s.plugin.0, s.name.0), s))
        .collect();
    // ... scan prose for <!-- skill: X:Y --> and verify against registry

    if !errors.is_empty() {
        for e in &errors {
            eprintln!("ERROR: {e}");
        }
        std::process::exit(1);
    }

    // Render SKILL.md files
    for skill in &skills {
        let output_path = PathBuf::from("plugins")
            .join(skill.plugin.0)
            .join("skills")
            .join(skill.name.0)
            .join("SKILL.md");
        std::fs::write(&output_path, render(skill)).unwrap();
    }
}
```

Your Makefile becomes:

```makefile
.PHONY: generate
generate:
	cargo run --manifest-path skill-dsl/Cargo.toml --bin generate

.PHONY: checks
checks: generate
	@$(DENO) run --allow-read scripts/checks.ts
```

---

## Comparing the Two Approaches

| Dimension | Phase 1 (Extended checks.ts) | Phase 2 (Rust DSL) |
|---|---|---|
| **Authoring format** | Keep writing markdown directly | Write Rust structs, prose in `include_str!` files |
| **Feedback loop** | CI-time (run `make checks`) | Compile-time (`cargo build` fails) |
| **Broken reference detection** | Runtime file-exists check | Build script panic or compile error |
| **Typo in tool name** | String comparison against known set | Enum variant — won't compile |
| **Invalid skill directive** | Regex scan + registry lookup | Typed const + registry validation |
| **Variable consistency** | Regex-based heuristic | Typed `depends_on` with DAG validation |
| **Effort to adopt** | ~200 lines of TypeScript | New crate + migration of 18 SKILL.md files |
| **Ongoing authoring friction** | None — still markdown | Moderate — Rust for skeleton, markdown for prose |
| **Composability** | Limited — string matching | High — skills are data, can derive/compose programmatically |

### The Hybrid: Use Both

Phase 1 is worth doing regardless. Even if you build the Rust DSL, you still want `checks.ts` validating the rendered output — defense in depth. The DSL gives you early feedback during authoring; `checks.ts` catches anything that slips through (manual edits to generated files, prose-embedded references, etc.).

### The Pragmatic Middle Ground

If the full Rust DSL feels like too much ceremony for 18 skills, there's a middle path: **YAML skill manifests validated by JSON Schema, with prose staying in markdown.**

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
    - name: SPECS_DIR
      expr: "$CHANGE_DIR/specs"
      depends_on: [CHANGE_DIR]

references:
  - id: sdk-api
    path: references/sdk-api.md
    mode: both
  - id: capabilities
    path: references/capabilities.md
    mode: both
  - id: update-patterns
    path: references/update-patterns.md
    mode: update-only

authority:
  - skill-md
  - artifacts
  - references
  - examples
  - existing-code
  - original-source
  - llm-inference

skill-directives:
  - omnia:test-writer
  - omnia:guest-writer
```

Then you write a `manifest.schema.json`, validate it in `checks.ts` alongside the SKILL.md, and cross-check that the manifest and the SKILL.md frontmatter agree. This gives you structured metadata without forcing every skill author into Rust. The SKILL.md prose stays hand-authored. If you later want to generate SKILL.md from the manifest + prose files, you can — but it's not required.

This YAML manifest approach has roughly the same validation power as the Rust DSL for the structural layer, with much less adoption friction. The Rust DSL mainly wins on ergonomics (IDE autocomplete, type inference) and on the ability to programmatically compose or derive skills — which matters more as the skill count grows.

