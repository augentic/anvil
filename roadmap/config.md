# Configuration Architecture — Findings and Recommendations

*Exported on 15/04/2026 from Cursor*

---

## Context

Horizon 2 calls for a configuration system that is simple, flexible, deterministic, extensible, and forward-compatible with horizon 3's multi-repo support. This document proposes a three-layer configuration architecture designed from the ground up, informed by the current system's strengths and tensions.

---

## What Exists Today

The current system has a **two-layer** configuration model:

1. **`schema.yaml`** (per tech stack, lives in this repo) — owns blueprint definitions, per-artifact validation rules, build instructions, defaults for context and rules, and terminology. Currently carries both structural metadata (blueprint DAGs, validation flags) and behavioural guidance (inline prose validation strings, inline prose rules).

2. **`.specify/config.yaml`** (per consumer project, lives in the consumer) — a thin overlay with `schema`, `context` (freeform string), and `overrides` (per-blueprint prose rules).

There is no Layer 3 today. Multi-repo coordination doesn't exist yet.

---

## Where the Current Model Strains

### A. Schema does too many jobs

`schema.yaml` conflates pipeline routing (DAG, generates) with per-artifact acceptance criteria (validate arrays) and behavioural defaults (context, rules). The `validate` strings and `defaults.rules` prose are agent-readable content sitting in a machine-readable configuration file. The CLI has to parse YAML, extract these strings, and either ignore them (rules) or pass them through (validation) — in neither case does the CLI benefit from them being in the schema. This conflates three concerns: pipeline structure, artifact quality criteria, and generation guidance.

### B. Instructions are schema-owned but skill-consumed

The `instructions/` directory is co-located with the schema and referenced by blueprint `instructions` fields. But the build instruction (`instructions/build.md`) references specific skills like `omnia:crate-writer` and `omnia:guest-writer`. This creates an implicit coupling: the schema must know which plugins exist. If you add a new tech stack (say, a Python WASM target), you need a new schema *and* new instructions that reference new skills — even though the define/build/merge workflow is identical.

### C. Config resolution is over-specified for what it carries

The 205-line `schema-resolution.md` describes an elaborate resolution algorithm (local → cache → remote, `@ref` pinning, `extends` composition) for a config file with three meaningful fields. The resolution complexity exists because resolution logic lives in prose that the agent must interpret, rather than in a tool.

### D. No separation between "what artifacts to produce" and "how to produce them"

The blueprint `instructions` field points to a markdown file that contains both the artifact *format* (structural template) and the *process* (which skills to invoke, in what order). For multi-repo, different repos might share the same artifact format but use different skill chains.

### E. The pipeline is hardcoded as define → build → merge

This is fine for now, but there's no way to express alternative workflows (e.g., extract → merge, or define → review → build → merge) without creating a whole new schema.

---

## Design Principles

1. **Structure vs. guidance separation.** Machine-readable structure (pipeline DAGs, file paths, boolean flags) goes in YAML that the CLI validates. Agent-readable guidance (prose instructions, rules, context) and per-artifact acceptance criteria go in markdown files that the CLI resolves and extracts frontmatter from but never interprets as prose. The schema is pure routing; blueprints are self-contained.

2. **Each layer owns one concern.** Repo config says "what I am." Stack config says "what artifacts and pipeline." Platform config says "what repos compose the system."

3. **Deterministic resolution.** The CLI resolves all configuration. Skills receive a fully-resolved configuration object — no resolution logic in prose.

4. **Composition over inheritance.** Instead of `extends` with complex merge semantics, use explicit layering where each layer can only override specific fields.

5. **Forward-compatible for the CLI.** Every field should be either CLI-parseable (structural) or a file path to agent-readable content. No inline prose in YAML.

---

## Layer 1: Repo Config (`.specify/config.yaml`)

This is the per-repo file that every consumer project has. It answers: "What is this repo, what stack does it use, and what project-specific overrides does it need?"

```yaml
# .specify/config.yaml
version: 2

schema: omnia@v1

project:
  name: traffic-roadworks
  domain: |
    Traffic-related services including roadworks management,
    lane closures, and traffic flow analysis.

rules:
  proposal: rules/proposal.md
  specs: rules/specs.md
  design: rules/design.md
  tasks: rules/tasks.md
```

### Design decisions

- **`project.name`** — required, kebab-case. The CLI validates it. Used as an identifier in multi-repo coordination.
- **`project.domain`** — freeform prose. Replaces the old `context` field. This is the *only* inline prose allowed in config. It's short (2-3 sentences) and exists because asking users to create a separate file for a project description is overengineering.
- **`rules`** — per-blueprint overrides, now **file paths** instead of inline prose. Each points to a markdown file (relative to `.specify/`) containing project-specific guidance. If omitted, the stack's defaults apply. If the file doesn't exist, the CLI errors — no silent fallbacks.
- **No `overrides` key.** The `rules` block *is* the override mechanism. The word "overrides" was confusing because it suggested inheritance semantics. `rules` is clearer: "these are the rules for this project."
- **No `context`, `language`, `framework` fields.** These are aspects of the *domain* description or the *stack* choice. If the agent needs to know the language, it reads the stack config. If it needs project context, it reads `project.domain`.

### Where the rule files live

```text
.specify/
├── config.yaml
├── rules/              # project-specific rule overrides (optional)
│   ├── proposal.md
│   ├── specs.md
│   ├── design.md
│   └── tasks.md
├── changes/
├── specs/
└── .cache/
```

If a project doesn't need overrides, the `rules` block is simply omitted and the stack defaults apply.

---

## Layer 2: Stack Config (`schema.yaml`)

This is the per-technology-stack definition. It lives in this repo under `schemas/<name>/`. It answers: "What artifacts does this stack produce, in what order, using what pipeline?"

The schema is **pure declarative routing**. It defines what artifacts exist, their dependency order, and which blueprint files describe them. It does not contain validation rules, prose guidance, or agent-readable content. Everything the schema references is a file path or a structural flag — nothing the CLI has to parse and then ignore.

```yaml
# schemas/omnia/schema.yaml
name: omnia
version: 1

terminology:
  deliverable: crate

pipeline:
  define:
    - id: proposal
      generates: proposal.md
      blueprint: blueprints/proposal.md

    - id: specs
      generates: specs/**/*.md
      blueprint: blueprints/specs.md
      requires: [proposal]

    - id: design
      generates: design.md
      blueprint: blueprints/design.md
      requires: [proposal]

    - id: tasks
      generates: tasks.md
      blueprint: blueprints/tasks.md
      requires: [specs, design]

  build:
    blueprint: blueprints/build.md
    tracks: tasks.md

  merge:
    blueprint: blueprints/merge.md

consistency:
  proposal-crates-have-specs: true
  design-references-valid: true
  spec-format-valid: true

defaults:
  domain: |
    Tech stack: Rust, WASM (wasm32-wasip2), Omnia SDK
    Architecture: Handler<P> pattern with provider trait bounds
    Testing: Rust integration tests, cargo test
  rules:
    proposal: defaults/proposal.md
    specs: defaults/specs.md
    design: defaults/design.md
    tasks: defaults/tasks.md
```

### Where validation lives: in the blueprints

Validation rules are **acceptance criteria for an artifact**. They answer: "how do I know this artifact was generated correctly?" That's tightly coupled to the *blueprint* for that artifact, not to the pipeline routing in the schema.

Putting `validate` arrays in `schema.yaml` violates the "no prose in YAML" principle — strings like `"Has a Why section with at least one sentence"` are agent-readable prose sitting in a machine-readable configuration file. The current `schema.yaml` has the same problem with `defaults.rules`. Carrying validation forward into the new schema would repeat the mistake.

Instead, each blueprint file carries its own validation criteria in YAML frontmatter:

```markdown
---
validate:
  - "Has a Why section with at least one sentence"
  - "Has a Crates section listing at least one new or modified crate"
  - "Crate names are kebab-case"
---

Create the proposal document that establishes WHY this change is needed.

Sections:
- **Why**: 1-2 sentences on the problem or opportunity...
```

The CLI parses the frontmatter to extract validation rules (using the `Pass`/`Fail`/`Deferred` classification from horizon 1). The agent reads the prose body for generation guidance. Same file, clean separation of structural metadata from behavioural guidance.

This makes blueprints **self-contained**: a single file carries the generation guidance, the validation criteria, and (via frontmatter) any structural metadata the CLI needs. The schema doesn't need to know what "valid" means for each artifact — that's the blueprint's job.

### Cross-artifact consistency

The `consistency` block stays in the schema because it's a different category: pipeline-level invariants about how artifacts relate to each other. These are boolean flags with well-known names that the CLI checks — not prose strings the agent interprets. The schema toggles them on or off; the CLI owns the check logic.

### Design decisions

- **`pipeline` replaces `blueprints` + `build`.** The pipeline is now an explicit three-phase structure: `define` (artifact generation), `build` (implementation), `merge` (finalization). Each phase is a first-class concept rather than being implicit in the blueprint dependency graph. This makes the workflow visible and extensible — you could add a `review` phase between `build` and `merge` without restructuring.
- **`blueprint` replaces `instructions`.** The word "instructions" was overloaded (are they instructions to the agent? to the CLI? templates?). `blueprint` is clearer: it's the template/guide for generating that artifact. The file is still markdown consumed by the agent, with YAML frontmatter consumed by the CLI.
- **Validation moves to blueprint frontmatter.** Per-artifact acceptance criteria belong with the blueprint that describes how to create the artifact, not in the schema's pipeline routing. The schema stays pure structure; the blueprint is self-contained.
- **`consistency` stays in the schema.** Cross-artifact invariants are boolean flags about the pipeline, not prose — they belong in the schema as structural configuration.
- **`defaults.domain`** replaces `defaults.context`. Aligns with the repo config's `project.domain` field — the stack provides a domain default, the project can override it.
- **`defaults.rules`** are now file paths, not inline prose. Each points to a markdown file under the schema directory (e.g., `schemas/omnia/defaults/proposal.md`). The CLI resolves these paths; the agent reads the files.
- **No `extends`.** Schema composition through `extends` adds significant complexity (merge rules for each field, circular chain detection, multi-level resolution). For two schemas (omnia, vectis), this isn't worth it. If you later need composition, add it as a CLI concern that produces a *resolved* schema, rather than asking skills to interpret merge semantics.

### Schema directory layout

```text
schemas/omnia/
├── schema.yaml
├── blueprints/
│   ├── proposal.md          # frontmatter: validate rules; body: generation guidance
│   ├── specs.md
│   ├── design.md
│   ├── tasks.md
│   ├── build.md
│   └── merge.md
└── defaults/
    ├── proposal.md
    ├── specs.md
    ├── design.md
    └── tasks.md
```

The rename from `instructions/` to `blueprints/` is deliberate: these files are *blueprints* for creating artifacts, not instructions to the CLI. Each blueprint is self-contained — generation guidance in the body, validation criteria in the frontmatter. The `defaults/` directory is new — it pulls the inline prose rules out of `schema.yaml` and into standalone files.

---

## Layer 3: Platform Config (`.specify/platform.yaml`)

This is the multi-repo coordination layer. It answers: "What repos compose this platform, and how do they relate?"

```yaml
# .specify/platform.yaml (lives in any repo, or a dedicated platform repo)
name: traffic-platform
version: 1

repos:
  - name: traffic-roadworks
    url: git@github.com:org/traffic-roadworks.git
    schema: omnia
    path: .

  - name: traffic-dashboard
    url: git@github.com:org/traffic-dashboard.git
    schema: vectis

  - name: traffic-shared-types
    url: git@github.com:org/traffic-shared-types.git
    schema: omnia

contracts:
  - type: api
    provider: traffic-roadworks
    consumer: traffic-dashboard
    spec: "@traffic-roadworks:roadworks-api/spec.md#REQ-002"

  - type: types
    provider: traffic-shared-types
    consumers: [traffic-roadworks, traffic-dashboard]
    spec: "@traffic-shared-types:shared-models/spec.md"
```

### Design decisions

- **Separate file, not in `config.yaml`.** Platform config is a different concern from repo config. A repo can exist without being part of a platform. A platform config might live in a dedicated "platform" repo or in any participating repo.
- **`repos` is a flat list.** No nesting, no hierarchy. Each entry has a name (must match the repo's `project.name`), URL, and schema. The CLI validates that names are unique and that the schemas exist.
- **`contracts` are explicit.** Instead of hoping cross-repo spec references are discovered, contracts declare the interface boundaries. This is what `specify platform validate` checks. The `@repo:path` syntax is only used in contract declarations, not scattered through prose.
- **No `federation` key.** The horizon 3 design used `federation` as a nested block in `config.yaml`. Making it a standalone `platform.yaml` is cleaner — it's a different layer of configuration with different ownership (platform team vs. repo team).

---

## How the Layers Compose

The CLI resolves configuration in a deterministic order:

```text
platform.yaml (optional)
    └── locates repos
         └── each repo's config.yaml
              └── references a schema
                   └── schema.yaml + blueprints/ + defaults/
```

### Resolution rules

1. Read `config.yaml` → get `schema` reference.
2. Resolve schema (local, cache, or remote — this is a CLI concern, not a skill concern).
3. For each blueprint field that accepts a file path (`blueprint`, `defaults.rules.*`):
   - If the repo's `config.yaml` specifies a `rules.<id>` path, use that file.
   - Otherwise, use the schema's `defaults.rules.<id>` path.
4. Return a **resolved config object** (JSON) that the skill consumes. The skill never does resolution — it receives the final answer.

### Resolved output

The `specify config resolve` command produces:

```json
{
  "project": {
    "name": "traffic-roadworks",
    "domain": "Traffic-related services..."
  },
  "schema": {
    "name": "omnia",
    "version": 1,
    "terminology": { "deliverable": "crate" }
  },
  "pipeline": {
    "define": [
      {
        "id": "proposal",
        "generates": "proposal.md",
        "blueprint": "/resolved/path/to/blueprints/proposal.md",
        "rules": "/resolved/path/to/rules/proposal.md",
        "validate": [
          "Has a Why section with at least one sentence",
          "Has a Crates section listing at least one new or modified crate",
          "Crate names are kebab-case"
        ]
      }
    ],
    "build": {
      "blueprint": "/resolved/path/to/blueprints/build.md",
      "tracks": "tasks.md"
    },
    "merge": {
      "blueprint": "/resolved/path/to/blueprints/merge.md"
    }
  },
  "consistency": {
    "proposal-crates-have-specs": true
  }
}
```

The `validate` arrays are extracted from each blueprint's YAML frontmatter during resolution — they don't exist in the schema. Skills receive a single resolved object with everything assembled. The 205-line `schema-resolution.md` collapses to: "run `specify config resolve` and use the output."

---

## Impact on Skills

### Skill prose simplification

The current `define` skill's step 3 is 12 lines explaining how to resolve the schema, merge defaults, check placeholders. With the CLI handling resolution:

```markdown
3. **Read resolved configuration**

   Run `specify config resolve` to get the resolved configuration.
   Use `pipeline.define` for the blueprint list and dependency order.
   Use each blueprint's `rules` path for generation constraints.
```

The `build` skill's validation section (currently ~40 lines of prose) becomes:

```markdown
6. **Validate artifacts**

   Run `specify validate <change-dir>` which returns a JSON report.
   If any checks fail, show the report and halt.
```

### Skill-by-skill impact

| Skill   | Current config logic                                    | After                                 |
|---------|---------------------------------------------------------|---------------------------------------|
| `init`  | Schema resolution, cache population, config scaffolding | `specify init` handles all of this    |
| `define`| Schema resolution, default merging, placeholder checks  | `specify config resolve` → use output |
| `build` | Schema resolution, default merging, artifact validation | `specify config resolve` + `specify validate` |
| `merge` | Schema resolution, metadata reads                       | `specify config resolve` → use output |
| `status`| Schema resolution, metadata parsing                     | `specify status` (CLI command)        |

---

## Migration Path

1. **Phase 1**: Implement the new schema format (`pipeline`, `blueprints/`, `defaults/`) in this repo. Both omnia and vectis schemas get updated. The `schema.schema.json` gets a v2 definition. `checks.ts` validates both.

2. **Phase 2**: The `specify` CLI gains a `config resolve` command that reads `config.yaml`, resolves the schema, and outputs the resolved JSON. Skills are updated to consume the resolved output.

3. **Phase 3**: Consumer projects running `/spec:init` get v2 `config.yaml`. A `specify config migrate` command upgrades v1 configs.

4. **Phase 4**: `platform.yaml` and `specify platform sync` / `specify platform validate` for multi-repo.

---

## Naming Changes Summary

| Current | Proposed | Why |
|---------|----------|-----|
| `blueprints` (array in schema) | `pipeline.define` (array) | Makes the workflow phase explicit |
| `instructions` (field + dir) | `blueprint` (field), `blueprints/` (dir) | Clearer — it's the template for the artifact |
| `build` (in schema) | `pipeline.build` | Groups all phases under `pipeline` |
| — | `pipeline.merge` | Makes merge a first-class phase |
| `defaults.context` | `defaults.domain` | Aligns with repo config's `project.domain` |
| `defaults.rules` (inline prose) | `defaults.rules` (file paths) | No prose in YAML |
| `overrides` (in config) | `rules` (in config) | Clearer intent — these are rules, not overrides |
| `context` (in config) | `project.domain` | Structured, scoped |
| `validate` (in schema per blueprint) | Blueprint frontmatter `validate` | Acceptance criteria belong with the blueprint, not the pipeline routing |
| `validation` (in schema) | `consistency` | Distinguishes cross-artifact invariants from per-artifact acceptance criteria |
| `federation` (in config) | `platform.yaml` (separate file) | Separate concern, separate file |
| — | `pipeline` (top-level) | Workflow is now an explicit, extensible concept |

---

## What This Unlocks for 100+ Repo Platforms

The three-layer separation is designed for scale:

- **Adding a new repo** to a platform = add one entry to `platform.yaml` + run `/spec:init` in the repo. The repo inherits its schema's defaults. Override only what's unique.
- **Changing the stack** (e.g., from omnia to a Python variant) = point the repo's `schema` to a different stack. The pipeline, blueprints, and defaults all come from the new schema. No per-repo changes needed.
- **Enforcing platform-wide rules** = the `contracts` section in `platform.yaml` makes cross-repo interfaces explicit. CI runs `specify platform validate` to catch mismatches.
- **Schema evolution** = `@ref` pinning lets repos upgrade schemas independently. One repo can be on `omnia@v2` while another stays on `omnia@v1`. The CLI resolves each repo's schema independently.
