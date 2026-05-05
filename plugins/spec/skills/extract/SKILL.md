---
name: specify-extract
description: Extract Specify artifacts (specs + design.md) from existing source code. Produces reconstruction-grade, language-agnostic artifacts capturing domain-level business logic. Supports optional `--include` / `--exclude` / `--manifest` filters that scope which source files are read for business-logic extraction without changing the artifact output shape. Use when reconstructing Specify artifacts from a legacy code tree, or when the user mentions `extract`.
argument-hint: "<source-path> <change-dir>"
---

## Critical Path (Quick Reference)

1. **Identify component structure** — detect source language, entry points, module organization, async patterns, and guest/entry-point layer. Pin dependency versions from the lock file, not the manifest.
2. **Extract business logic** — apply scope filters (`--include`/`--exclude`/`--manifest`), then analyze depth-first by domain. Tag every statement `[domain]`, `[infrastructure]`, `[mechanical]`, or `[unknown]`. Copy type definitions verbatim; capture all serialization attributes, wire-format names, and field optionality. See [business-logic.md](business-logic.md).
3. **Document external API surfaces** — trace actual deserialization code (not type declarations) for every HTTP/API call. Record exact URLs, headers, request/response shapes, auth sources, retries, and timeouts. See [external-api.md](external-api.md).
4. **Capture external service dependencies** — classify each service by type (`database`, `managed table store`, `message broker`, `cache`, `identity provider`, `API`, `WebSocket`). See [dependencies.md](dependencies.md).
5. **Capture publication & timing patterns** — document exact publication counts, delay placement, payload identity, partition keys, and message metadata. See [dependencies.md](dependencies.md).
6. **Capture metrics and observability** — record metric names, types, emission points, and labels. See [observability.md](observability.md).
7. **Write artifacts** — create `$SPECS_DIR/$CRATE_NAME/spec.md` (flat `### Requirement:` blocks with `ID: REQ-XXX`) and `$DESIGN_PATH` (all 14 sections from Context through Notes). See [design-template.md](design-template.md). Validate against [verification.md](verification.md) before completing.

# Extract

> See also [`../analyze/SKILL.md`](../analyze/SKILL.md) for plan-time capability inference — the sibling skill that emits capability summaries into `discovery.md`, not full `specs/` + `design.md`.

## Overview

Analyze a source codebase to produce reconstruction-grade, **language-agnostic** Specify artifacts (specs + design.md) capturing domain-level business logic. The artifacts split behavioral requirements (specs) from technical details (design), enabling cleaner separation of "what" from "how", in a format suitable for migration to any target language or runtime.

**Key principle**: The artifacts are an intermediary format with **no bias toward any target language**. They describe what the code does, not how it should be implemented in a specific language.

## Derived Arguments

1. **Source Path** (`$SOURCE_PATH`): Path to the source codebase
2. **Change Directory** (`$SLICE_DIR`): Specify change directory (e.g., `./.specify/changes/component/`)
3. **Include globs** (`$INCLUDE`): Zero or more `--include <glob>` values that narrow the read set for business-logic extraction. Empty ≡ today's behaviour.
4. **Exclude globs** (`$EXCLUDE`): Zero or more `--exclude <glob>` values that remove paths from the read set for business-logic extraction. Empty ≡ today's behaviour.
5. **Manifest path** (`$MANIFEST`): Optional single `--manifest <path>` pointing at a slice manifest. Mutually exclusive with `$INCLUDE` / `$EXCLUDE`. See [scope-filters.md](scope-filters.md) §Manifest shape.

```text
$SOURCE_PATH = $ARGUMENTS[0]
$SLICE_DIR  = $ARGUMENTS[1]
$SPECS_DIR   = $SLICE_DIR/specs
$DESIGN_PATH = $SLICE_DIR/design.md
$INCLUDE     = [--include <glob> ...]       # repeatable; possibly empty
$EXCLUDE     = [--exclude <glob> ...]       # repeatable; possibly empty
$MANIFEST    = --manifest <path>            # single; mutually exclusive with $INCLUDE/$EXCLUDE
```

`$MANIFEST` is mutually exclusive with `$INCLUDE` / `$EXCLUDE`. Invoking extract with a `$MANIFEST` alongside any `$INCLUDE` or `$EXCLUDE` flag is a hard error — the driver (`/spec:execute`) and the capability's define brief should have caught it upstream at `specify plan validate` time. Extract fails fast with a clear message rather than trying to reconcile the two modes.

## Scope filters at a glance

Scope filters restrict **which source files are read for business-logic extraction** (Step 2 onward). They never touch Step 1 — language detection and dependency version pinning always run against the full set of sentinel files. Empty filter set ≡ today's behaviour: extract reads the full source tree.

The full filter rules, the sentinel file list, and the v1 manifest schema live in [scope-filters.md](scope-filters.md).

## Principles (non-negotiable)

1. **Focus**: Extract only domain/business logic and its inputs/outputs. Exclude infrastructure unless part of a domain rule.
2. **Descriptive, not interpretive**: Produce algorithmic descriptions of what the code does. Do not infer "why" unless present in source.
3. **Zero inference**: Do not invent behavior or semantics. Use explicit `unknown` tokens.
4. **Explicit constants**: List every constant by identifier and semantic availability.
5. **Traceability**: Each statement must be traceable to code. Do not attribute intent not in comments.
6. **Tagging**: Each Business Logic line must include one tag: `[domain]`, `[infrastructure]`, `[mechanical]`, or `[unknown]`.
7. **Conservatism**: Prefer `unknown` over guessing.
8. **Language-agnostic**: Do not introduce target-language concepts. Describe behavior, not implementation.
9. **Depth-first when possible**: When the source has clear functional domain boundaries, analyze depth-first by domain (all types + handlers + utilities for one domain before moving to the next). Fall back to step-by-step for simpler or single-domain components.

## Tags and Unknown Tokens

See complete definitions in [Specify Artifact Format Specification — Tags Reference](references/specify.md#tags-reference).

## Process

### Step 1: Identify Component Structure

**THINK**: Before analyzing code, reason through these questions:

1. What source language is this? (Check file extensions: .ts, .js, .go, .py, .rs, .java, .cs)
2. What is the entry point? (Look for: main.\*, index.\*, handler exports, main functions)
3. How is the code organized? (Monolithic file? Multiple modules? Layered architecture?)
4. What external libraries are used? (Check manifest: package.json, go.mod, requirements.txt, Cargo.toml)
5. What async patterns are present? (async/await, Promises, goroutines, callbacks, futures)
6. What types are defined? (interfaces, classes, structs, enums)
7. Is there a guest/entry-point layer? (Middleware, CORS, error mapping, body injection, parameter sourcing)

**ANALYZE**: Read the source at `$SOURCE_PATH` and identify:

1. **Source language**: Detect from file extensions
2. Entry points (e.g., `main.*`, `index.*`, handler exports, `func main()`, `if __name__ == "__main__"`, etc.)
3. Module organization and file structure
4. External dependencies from manifest files (`package.json`, `go.mod`, `requirements.txt`, `Cargo.toml`, `pom.xml`, etc.)
5. Async boundaries (async/await, Promises, goroutines, threads, futures, etc.)
6. Type definitions (interfaces, types, classes, structs, enums)
7. **Guest/entry-point layer**: Middleware (CORS, auth), error code → HTTP status mapping, body injection/transformation, parameter sourcing, and any validation performed before the domain handler.

Scope filters never hide manifest files from this step — see [scope-filters.md](scope-filters.md) §"Sentinels always read". Language detection and dependency extraction always run against the full set of sentinel files regardless of `$INCLUDE` / `$EXCLUDE` / `$MANIFEST`.

**Dependency version pinning**:

Dependency version drift is a leading cause of build failures when regenerating from a specification. Capture dependency versions from the source project's **lock file**, not just the manifest.

| Stack | Manifest | Lock File | Version Source |
|-------|----------|-----------|----------------|
| Rust | `Cargo.toml` | `Cargo.lock` | Lock file |
| Node/TypeScript | `package.json` | `package-lock.json` / `yarn.lock` / `pnpm-lock.yaml` | Lock file |
| Python | `pyproject.toml` / `setup.cfg` | `poetry.lock` / `requirements.txt` (pinned) | Lock file or pinned requirements |
| C# | `.csproj` | `packages.lock.json` | Lock file |
| Go | `go.mod` | `go.sum` | `go.mod` (already pinned) |
| Java/Kotlin | `pom.xml` / `build.gradle` | Dependency tree output | Resolved dependency tree |

For each dependency, record: package name, **exact version** from lock file (e.g., `1.4.0`, not `^1.4`), whether it is direct or transitive, and any feature flags / optional features enabled.

In the design.md Dependencies section, list the **manifest version specifier** (e.g., `"1.0.100"` from Cargo.toml, `"^2.3.0"` from package.json) as the primary version — this is what goes into the generated project's dependency declaration. Also note the lock file resolved version for API compatibility reference.

**When the lock file is absent**: Use the manifest version constraints and flag this in Risks / Open Questions.

**VERIFY**:

- [ ] I've identified the primary source language correctly
- [ ] I've found all entry points (there may be multiple)
- [ ] I've understood the module structure (not just listed files)
- [ ] I've checked the manifest file for dependencies
- [ ] I've noted async vs sync execution patterns
- [ ] I've checked for a guest/entry-point layer (middleware, error mapping, body injection)
- [ ] I've read the lock file for dependency versions (or flagged its absence)

### Step 2: Extract Business Logic

Restrict the read set per the scope filters from Step 1. Apply optional semantic-discovery hints, then analyze each function depth-first by domain. Tag every statement `[domain]` / `[infrastructure]` / `[mechanical]` / `[unknown]`. Copy type definitions verbatim from the source — never hand-write from memory. Capture every serialization attribute (renames, aliases, conditional and unconditional skips, custom converters), every input field's optionality, and every output type's full schema (including fields not populated by this component).

The full per-function THINK / ANALYZE / VERIFY checklist, the type extraction rules, and the orchestration / shared-handler rules live in [business-logic.md](business-logic.md).

### Step 3: Document External API Surfaces

For every HTTP/API call, trace the **actual deserialization code**, not the type declaration. The runtime response shape is determined by how the code uses the response (e.g., `const allocated: string[] = await response.json()` is `string[]`, not the wider declared interface). Document the exact URL, HTTP method, headers, request body, response shape (with a concrete JSON example), authentication source (config-driven vs hardcoded), error response codes, retries, and timeouts.

The full per-call rubric and authentication-source patterns live in [external-api.md](external-api.md).

### Step 4: Capture External Service Dependencies

Classify every external service by type — `database`, `managed table store`, `message broker`, `cache`, `identity provider`, `API`, `WebSocket` — and record the technology, connection details, operations, data formats, and authentication. Cloud-managed table/document stores (Azure Table Storage, Cosmos DB, DynamoDB) are `managed table store`, never `API`.

The full type taxonomy lives in [dependencies.md](dependencies.md) §Step 4.

### Step 5: Capture Publication & Timing Patterns

Document exact publication counts, delay placement (BEFORE or AFTER each round), payload identity (identical or modified between rounds), retry patterns, batch vs individual, concurrency, and message metadata (partition keys, custom headers, topic construction).

The full per-pattern guidance lives in [dependencies.md](dependencies.md) §Step 5.

### Step 6: Capture Metrics and Observability Patterns

Record metric names, types (counter / gauge / histogram), emission points, dimensions / labels, and purpose. See [observability.md](observability.md).

### Step 7: Write Specify Artifacts

Synthesize findings, create `$SLICE_DIR/` and `$SPECS_DIR/`, write `$DESIGN_PATH` with all 14 sections (Context through Notes) and `$SPECS_DIR/$CRATE_NAME/spec.md` with flat `### Requirement:` blocks tagged `ID: REQ-XXX`.

The pre-write synthesis checklist, the directory layout, the 14-section design.md template, the managed-data-store classification rules, and the spec file format live in [design-template.md](design-template.md). The post-write verification checklist lives in [verification.md](verification.md).

## Reference Documentation

Detailed guidance and specifications are available in `references/`:

- **[Specify Artifact Format Specification](references/specify.md)** — Complete artifact structure with spec and design.md templates
- **[Language Mapping Guide](references/language-mapping.md)** — How to map common language constructs to artifact format (with examples from TypeScript, Go, Python, etc.)
- **[Context Gaps Reference](references/context-gaps.md)** — Commonly missed details and how to capture them, including data access phrasing (§13) and ensuring every endpoint has a business logic block (§14)
- **[Semantic Search Reference](references/semantic-search.md)** — Optional semantic search integration for improved analysis coverage
- **[Lessons Learned](references/lessons-learned.md)** — Anti-patterns from real extraction attempts and how to avoid them
- **[Examples](references/examples/)** — Complete analysis examples for different scenarios

## Examples

Detailed examples are available in the `references/examples/` directory:

1. [outbound-http.md](references/examples/outbound-http.md) — Analyze a TypeScript HTTP handler and produce Specify artifacts
2. [branching-caching.md](references/examples/branching-caching.md) — Capture complex conditional logic with hierarchical numbering
3. [parallel-execution.md](references/examples/parallel-execution.md) — Document async/parallel execution patterns in artifacts

## Verification Checklist

Before completing, verify all items from the [Specify Artifact Validation Checklist](references/specify.md#validation-checklists) are satisfied, plus the skill-specific items in [verification.md](verification.md). Common error modes and recovery steps also live in that file.

## Guardrails

### NEVER

- Assume a field type — verify against source
- Rename config keys — capture verbatim
- Invent wire names — extract from serialization attributes/decorators/annotations
- Skip fields — document every field (use `[unknown]` if unclear)
- Skip field-level attributes — keyword-collision renames, aliases, and unconditional skips are wire-format-critical
- Hand-write types from memory — copy from source
- Assume two handlers share construction details because they target the same API — verify each independently
- State patterns as universal rules — always check for exceptions (e.g., "all collection fields have default-when-absent" is rarely true for all)
- Skip the guest/entry-point layer — middleware, error mapping, body injection, and parameter sourcing are load-bearing behaviors
- Say one function "behaves like" another — verify each function's code paths independently
- Generate test fixtures without verifying against source response shapes
- Record dependency names without versions — always capture exact versions from manifest AND lock file
- Assume "latest" version compatibility — API surfaces change between versions
- Merge cross-struct column headers — use separate columns for each struct type

### ALWAYS

- Compare every type/class/interface field against source definition, including field-level renames, aliases, and serialization skips
- Include source traceability for every requirement
- Use `[unknown]` rather than guessing
- Capture dependency versions from both manifest and lock file; use manifest specifiers in design.md
- Check serialization wire names by applying naming convention rules — flag divergent naming
- Document each utility function's behavior independently, including error messages and status code handling
- Include guest/entry-point behaviors in the analysis (CORS, error mapping, body injection, owner parameter sourcing)
- Document response type serialization ownership — which module contains the canonical impl, which modules reuse it
- Document every outbound API call body completely, including vendor-specific field names for audit/secondary calls
- For orchestration handlers, document exact format strings, conditional null fields, and wrapper structures independently

## Important Notes

- **Language-agnostic**: Do not introduce target language concepts (e.g., Rust traits, Python decorators). Describe behavior only.
- **Preserve structure**: Maintain exact field names, nesting, and type shapes from source
- **No inference**: Use `unknown` tokens rather than guessing behavior or values
- **Traceability**: Every statement must be traceable to source code or comments
- **Reconstruction-grade**: The artifacts must contain sufficient detail for accurate code generation in any language
