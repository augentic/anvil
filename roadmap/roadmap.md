# Specify Roadmap — Findings and Recommendations

_Exported on 14/04/2026 from Cursor_

---

## Context

Most organisations building AI-assisted development frameworks use deterministic programs to orchestrate the workflow (e.g. OpenSpec, SpecKit's spec-driven development, Geoff Huntley's Loop). Specify inverts control: an agent orchestrates skills to achieve the same outcome — automated code generation — with deterministic tools used only where precision is required.

This document captures a critique of the current framework and a roadmap structured as three horizons.

---

## What's Working Well

The inversion-of-control thesis is sound and architecturally distinct from the deterministic-program crowd. The key insight — that an LLM agent is better at orchestrating flexible, judgment-heavy workflows while deterministic tools should own precision-critical operations — maps well to the actual failure modes you'd see in practice.

### The skill-as-contract model is strong

Skills like `define`, `build`, and `merge` are behavioural contracts that the agent interprets, not rigid scripts. This gives resilience to ambiguity that a deterministic pipeline can't handle (e.g. the `build` skill's "pause if task is unclear" or "suggest artifact updates if design issues emerge").

### The schema system is well-factored

The separation of `schema.yaml` (structural metadata), `instructions/*.md` (behavioural guidance), and `config.yaml` (thin project overlay) creates clean layering. The `extends` mechanism for schema composition is forward-looking.

### The delta-merge spec format is clever

Using stable `REQ-XXX` IDs as merge keys with ADDED/MODIFIED/REMOVED/RENAMED operations gives a specification system that can evolve without losing traceability.

---

## Where the Tension Shows

### 1. The "deterministic islands" are ad-hoc and fragile

There is exactly one deterministic tool today: `merge-specs.py`. Everything else that needs precision — validation, task parsing, artifact structure checking — is done by the LLM interpreting prose rules. This creates a reliability gradient: the merge step is solid (deterministic Python with exit codes), but validation is probabilistic (the agent reading `validate` strings from `schema.yaml` and making judgment calls).

The build instructions in `schemas/omnia/instructions/build.md` illustrate this well — they contain shell commands like `cargo test 2>&1 | tee /tmp/...` embedded in prose. The agent must parse the prose, decide when to run the command, interpret the output, and classify failures. That's a lot of cognitive load on the LLM for what is fundamentally a structured decision tree.

### 2. The config/schema system is overloaded for what it does

`config.yaml` has three fields (`schema`, `context`, `overrides`), but the resolution semantics are complex: bare names vs URLs, `@ref` suffixes, cache invalidation, fallback chains for defaults vs overrides, non-empty checks for placeholder detection. The `schemas/README.md` is 217 lines explaining what is essentially a three-field config file. This complexity exists because resolution logic is encoded in prose that the agent must interpret, rather than in a tool that handles it deterministically.

### 3. Validation is specified declaratively but executed probabilistically

The `validate` arrays in `schema.yaml` are human-readable strings like "Has a Why section with at least one sentence" and "Uses SHALL/MUST language for normative requirements." These are great for communicating intent, but the agent's interpretation of "at least one sentence" or "WHEN/THEN format" will vary across invocations. The `checks.ts` script validates the *framework itself* rigorously, but the *artifacts it produces* get weaker validation.

### 4. Multi-repo is structurally hard

The `.specify/` directory is project-local. The `touched_specs` conflict detection in `.metadata.yaml` only works within a single workspace. There's no concept of a "spec reference" that spans repositories, and the schema resolution assumes a single project root.

---

## Roadmap — Three Horizons

### Horizon 1: `specify` CLI (Rust) — Deterministic Foundation

Extract every precision-critical operation from prose skills into a single Rust CLI binary (`specify`) that lives in this repo and ships alongside the plugins. Skills invoke it via shell commands. The agent handles judgment; the CLI handles correctness.

See [cli.md](cli.md) for the full crate structure sketch.

**Subcommands:**

| Command | Replaces |
|---------|----------|
| `specify init <schema>` | Agent's mkdir/copy/write logic in init skill |
| `specify validate <change-dir>` | 40 lines of prose validation in build skill |
| `specify merge <change-dir>` | `merge-specs.py` |
| `specify status [change-name]` | Agent parsing .metadata.yaml + task checkboxes |
| `specify schema resolve <value>` | Agent interpreting schema-resolution.md |
| `specify schema check <schema-dir>` | Parts of checks.ts |
| `specify task next <change-dir>` | Agent parsing tasks.md for next incomplete task |
| `specify task mark <change-dir> <n>` | Agent editing checkbox in tasks.md |
| `specify lint <change-dir>` | Cross-artifact consistency checks |

**The key benefit:** `specify validate` replaces the entire "Per-blueprint validation" and "Cross-blueprint consistency checks" sections in `build/SKILL.md` (currently ~40 lines of prose the agent must interpret) with a single shell command that returns structured JSON or a pass/fail exit code. The skill prose shrinks to "run the validation and act on the result."

**What the agent keeps:** The agent still decides *when* to validate, *how to respond* to failures, *whether to pause* for user input, and *what to suggest* for fixes. The CLI is a precision instrument; the agent is the practitioner.

### Horizon 2: Config Simplification

With a CLI handling resolution, the config can become genuinely thin.

**Current pain points:**
- `schema` field has complex resolution semantics (bare name vs URL vs `@ref`)
- `context` and `overrides` are free-form strings with placeholder detection
- The schema's `defaults` section duplicates what could be inline defaults
- `config.yaml`, `schema.yaml`, `.cache-meta.yaml`, and `.metadata.yaml` are four separate files with overlapping concerns

**Proposed `config.yaml` v2:**

```yaml
version: 2
schema: omnia@v1

project:
  name: my-service
  language: rust
  description: |
    A user authentication service built on Omnia SDK.

overrides:
  specs:
    format: given-when-then
  tasks:
    greenfield-chain: [guest-writer, crate-writer, test-writer, code-reviewer]
```

Key changes:
- **`project` block** replaces the freeform `context` string with structured fields the CLI can use for validation and the agent can use for context
- **`overrides` uses structured keys** where possible, with prose-only overrides staying as freeform strings under an `instructions` key
- **Schema resolution is a CLI concern**, not a skill concern
- **`.metadata.yaml` stays per-change** but gets validated by the CLI

### Horizon 3: Multi-Repo Coordination

Extend the existing `config.yaml` to declare peer repositories and use the CLI to coordinate.

See [horizons.md](horizons.md) for the full design.

---

## Shell Commands in Skills: Design Principles

| Use CLI (`specify ...`) when: | Use agent judgment when: |
|-------------------------------|--------------------------|
| The operation must be idempotent | The response depends on context |
| The output is structured (JSON, exit codes) | The output is natural language |
| Correctness is verifiable (schema validation) | Correctness requires semantic understanding |
| The operation is repeated across many skills | The operation is unique to one skill |
| Failure modes are enumerable | Failure modes are open-ended |

The `specify` CLI gives a clean abstraction boundary. Instead of skills containing scattered shell commands, they can use `specify` subcommands that return structured output. The principle: **the CLI owns Specify operations; external tool invocation stays with the agent.**

A good litmus test: "Would this command need to understand `.specify/` directory structure or spec format?" If yes, it belongs in the CLI. If no (like running `cargo test`), it stays as a direct shell command in the skill.

---

## Suggested Priority Order

1. **Specify CLI scaffolding** — `specify init`, `specify validate`, `specify merge` (replaces `merge-specs.py`)
2. **Migrate `init` and `merge` skills** to use CLI commands
3. **Migrate `build` validation** to use `specify validate`
4. **Config v2** with structured `project` block and CLI-backed resolution
5. **`specify task`** subcommands for deterministic task tracking
6. **Federation config** and `specify federation sync` for multi-repo
7. **Cross-repo spec references** and `specify federation validate`

The first three items would take a single `/spec:define` + `/spec:build` cycle to implement and would immediately simplify the three most complex skills.

---

## Impact on Existing Skills

| Skill | Current agent-interpreted logic | Moves to CLI |
|-------|---------------------------------|--------------|
| `init` | mkdir, file creation, schema resolution, cache population | `specify init` |
| `define` | Schema resolution, metadata writes, overlap detection | `specify schema resolve`, `specify status` |
| `build` | Artifact validation, task progress tracking | `specify validate`, `specify task next/mark` |
| `merge` | merge-specs.py invocation, coherence check, archive move | `specify merge` |
| `verify` | Spec parsing, requirement extraction | `specify diff` |
| `status` | Metadata + task parsing | `specify status` |

---

## The `checks.ts` to `specify check` Migration

The `roadmap/dsl.md` conversation explored extending `checks.ts` vs building a Rust DSL. With the `specify` CLI as Horizon 1, this becomes natural:

- **Phase 1 checks (already done):** `checks.ts` validates the framework repo (symlinks, schema integrity, skill frontmatter, etc.)
- **Phase 2:** `specify check` validates consumer project artifacts at runtime. `checks.ts` remains for framework CI; `specify check` is for runtime project validation.
- **Phase 3 (optional):** If the skill count grows significantly, the Rust DSL from `roadmap/dsl.md` can generate SKILL.md files from typed definitions, with the `specify` crate providing the type system. But this is premature for 18 skills.
