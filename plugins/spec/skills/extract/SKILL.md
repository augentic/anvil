---
name: specify-extract
description: Extract Specify artifacts (specs + design.md) from existing source as language-agnostic captures of domain logic. Use when bootstrapping artifacts from a codebase with no `.specify/`; not for fresh slices or plan-time adapter inference.
argument-hint: <source-path> <slice-dir> [include]... [exclude]... [manifest]
---

## Critical Path

1. **Identify component structure** — detect source language, entry points, module organization, async patterns, and guest/entry-point layer. Pin dependency versions from the lock file, not the manifest.
2. **Extract business logic** — apply scope filters (`include`/`exclude`/`manifest`), then analyze depth-first by domain. Tag every statement `[domain]`, `[infrastructure]`, `[mechanical]`, or `[unknown]`. Copy type definitions verbatim; capture all serialization attributes, wire-format names, and field optionality. See [business-logic.md](business-logic.md).
3. **Document external API surfaces** — trace actual deserialization code (not type declarations) for every HTTP/API call. Record exact URLs, headers, request/response shapes, auth sources, retries, and timeouts. See [external-api.md](external-api.md).
4. **Capture external service dependencies** — classify each service by type (`database`, `managed table store`, `message broker`, `cache`, `identity provider`, `API`, `WebSocket`). See [dependencies.md](dependencies.md).
5. **Capture publication & timing patterns** — document exact publication counts, delay placement, payload identity, partition keys, and message metadata. See [dependencies.md](dependencies.md).
6. **Capture metrics and observability** — record metric names, types, emission points, and labels. See [observability.md](observability.md).
7. **Write artifacts** — create `<slice-dir>/specs/<crate-name>/spec.md` (flat `### Requirement:` blocks with `ID: REQ-XXX`) and `<slice-dir>/design.md` (all 14 sections from Context through Notes). See [design-template.md](design-template.md). Validate against [verification.md](verification.md) before completing.

# Extract

> See also `/change:analyze` for plan-time adapter inference — the sibling skill that emits adapter summaries into `discovery.md`, not full `specs/` + `design.md`.

## Overview

Analyze a source codebase to produce reconstruction-grade, **language-agnostic** Specify artifacts (specs + design.md) capturing domain-level business logic. The artifacts split behavioral requirements (specs) from technical details (design), enabling cleaner separation of "what" from "how", in a format suitable for migration to any target language or runtime.

**Key principle**: The artifacts are an intermediary format with **no bias toward any target language**. They describe what the code does, not how it should be implemented in a specific language.

## Scope filters at a glance

Scope filters restrict **which source files are read for business-logic extraction** (Step 2 onward). They never touch Step 1 — language detection and dependency version pinning always run against the full set of sentinel files. Empty filter set ≡ today's behaviour: extract reads the full source tree.

The full filter rules, the sentinel file list, and the v1 manifest schema live in [scope-filters.md](scope-filters.md).

## Principles (non-negotiable)

Nine principles govern every extract: domain-only focus, descriptive not interpretive prose, zero inference, explicit constants, traceability to source, mandatory tagging of every business-logic line, conservatism (prefer `unknown` over guessing), language-agnostic descriptions, and depth-first analysis when domain boundaries are clear. The full text — each principle is non-negotiable — lives in [extract-principles.md](../../references/extract-principles.md).

## Tags and Unknown Tokens

See complete definitions in [Specify Artifact Format Specification — Tags Reference](references/specify.md#tags-reference).

## Process

### Step 1: Identify Component Structure

Detect the source language, entry points, module organization, async patterns, type definitions, and guest/entry-point layer at `$SOURCE_PATH`, then pin dependency versions from the lock file rather than the manifest. Scope filters never hide manifest files from this step — see [scope-filters.md](scope-filters.md) §"Sentinels always read".

The full THINK / ANALYZE / VERIFY procedure and the lock-file mapping table live in [component-structure.md](component-structure.md). Run that procedure before moving to Step 2.

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

Synthesize findings, create the slice directory and `specs/`, write `<slice-dir>/design.md` with all 14 sections (Context through Notes) and `<slice-dir>/specs/<crate-name>/spec.md` with flat `### Requirement:` blocks tagged `ID: REQ-XXX`.

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

Lifecycle state (`.metadata.yaml` transitions, baseline merge into `.specify/specs/`) is owned by `/spec:define` and `/spec:merge` via the CLI verbs in [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state); extract writes only inside the supplied `<slice-dir>` and leaves the slice in `defining`.

### Skill scope

- Write only `specs/` and `design.md` under the supplied `<slice-dir>`; the source tree is read-only.
- Never author `tasks.md` or implement code — task authoring lives in `/spec:define`; implementation lives in `/spec:build`.
- Never run plan-time adapter inference — that delegates to `/change:analyze`, which emits adapter summaries into `discovery.md`.
- Never clone git URLs from `<source-path>` — the caller (or the invoking define brief) materialises the source tree before extract runs.
- Never extend the closed kind enum — `legacy-code` / `documentation` are frozen at `/change:analyze`; extract operates on a materialised path regardless of kind.
