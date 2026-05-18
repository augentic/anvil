---
id: cluster
description: Per-source clustering prompt for /change:survey (Omnia).
generates: .specify/plans/<name>/survey.md
---

# Omnia legacy-code clustering for `/change:survey`

This brief carries Omnia's source-local clustering prompts for the `/change:survey` skill. The skill resolves this file and executes the clustering algorithm below against each `legacy-code` source independently.

---

## Legacy-code clustering

This branch infers adapter boundaries from source-tree structure and emits one adapter summary per inferred adapter. Output lives at `$DISCOVERY` (adapter summaries) and at `<plan-dir>/analyze/<$SOURCE_KEY>/metadata.json` (per-source structural facts).

### 1. Inputs

Expect `$INPUT_PATH` to be a local directory containing a source tree (cloned or symlinked by the discovery brief). Skim the tree to build the clustering signals below; do not require network access, a build step, or a language runtime.

### 2. Clustering signals

Combine the following signals to draw adapter boundaries. No single signal is load-bearing; adapters emerge from concordance.

| Signal         | Weight | Details                                                                                                             |
| -------------- | ------ | ------------------------------------------------------------------------------------------------------------------- |
| Import graph   | Strong | Modules that cluster in a tight import SCC are likely one adapter.                                               |
| Endpoint names | Strong | HTTP / CLI / message-handler names that share a prefix (e.g. `POST /users`, `POST /users/verify`) are one adapter. |
| Docstrings     | Medium | First-paragraph summaries on modules, classes, or handlers that name a behaviour.                                   |
| Test names     | Medium | `describe('user registration', …)`, `test_user_registration_*` patterns bound behaviour.                            |
| READMEs        | Medium | Per-module or per-directory READMEs often list adapters explicitly.                                             |

Apply Omnia's conventions:

- Prefer adapters that align with the "one crate per adapter" rule downstream: a cohesive HTTP surface + its domain model + its data access.
- Shared utilities that serve multiple adapters become their own adapter (kebab-case, e.g. `shared-validation`).
- Deprecated or vendored code goes out of scope (the propose brief later calls this out via `scope.<k>.exclude`).

Adapter names are kebab-case, 2–4 tokens, noun-phrases describing the behaviour.

### 3. Per-adapter output

Emit one adapter summary per inferred adapter, in the on-disk shape pinned by [`analyze/SKILL.md` §Output contract](../../../analyze/SKILL.md) — `### <name>` heading followed by a fenced YAML block.

Fields, in strict order:

- **`summary`** — single-sentence imperative ("Create new user accounts with email verification."). Derived from the adapter's dominant docstring, README entry, or endpoint-cluster description.
- **`sources`** — **file-hint list**: source files this adapter substantively inhabits, relative to `$INPUT_PATH`, alphabetically sorted. This list becomes `scope.<key>.include` downstream (see [`rfc-3a-monoliths.md` §*Scoped extraction on monoliths*](../../../../../../rfcs/archive/rfc-3a-monoliths.md)). Include every file that substantively implements the adapter; exclude pure test files and imports-only glue.
- **`depends-on`** — names of OTHER adapters this one imports or calls. Alphabetically sorted. Empty list allowed.
- **`hints.entry_points`** — HTTP routes, CLI commands, message-broker topics, scheduled triggers. Alphabetically sorted. Either omit the key entirely or emit a non-empty list.
- **`hints.external_deps`** — external systems (databases, queues, identity providers, third-party APIs) inferred from imports and configuration. Kebab-case names (`postgres`, `sendgrid`, `azure-ad`). Alphabetically sorted. Either omit the key entirely or emit a non-empty list.
- **`confidence`** — one of `high | medium | low`:

  | value    | when                                                                                                                                            |
  | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
  | `high`   | Clean file boundary (all sources under one subdirectory or a small explicit fan-out), a documented entry point, and a clear import-graph cluster. |
  | `medium` | Boundary is plausible but one signal is fuzzy — scattered files, ambiguous name, or weak imports. A human should skim before accepting.         |
  | `low`    | Adapter is implied — files span multiple directories, entry points unclear, or the clustering required a judgement call. Propose flags it for review. |

Do NOT emit `## Constraints` or `## Open questions` appendix blocks from this branch — those are documentation-only. The code branch has no equivalent source of prose rules or deferred decisions.

### 4. Structural metadata (`metadata.json`)

In addition to adapter summaries, write `<plan-dir>/analyze/<$SOURCE_KEY>/metadata.json` with the shape pinned in [`analyze/SKILL.md` §Structural metadata](../../../analyze/SKILL.md). Omnia conventions for the numeric fields:

- **`language`** — detected primary source language, kebab-case (`typescript`, `javascript`, `rust`, `go`, `python`, `java`, `kotlin`, `csharp`). Prefer the language with the largest share of non-generated LOC.
- **`loc`** — total non-blank non-comment source lines. Exclude test files, vendored dependency directories (`node_modules`, `vendor`, `target`, `.venv`), and generated code (`*.gen.ts`, `*_pb.go`, etc.).
- **`module_count`** — total module count. Omnia convention:
  - TypeScript / JavaScript: source files under the primary source tree (exclude tests, `*.d.ts`, build output).
  - Rust: crates; for a single-crate repo, count top-level `mod` declarations.
  - Go: packages.
  - Python: modules (`.py` files and `__init__.py`-bearing directories).
  - Java / Kotlin / C#: top-level types (classes + interfaces) under the primary source tree.
- **`top_level_modules`** — immediate children of the source root that are directories, alphabetically sorted, relative paths (`src/auth`, `src/ingest`). Flat-layout projects with code at the root produce an empty array.

The documentation branch MUST NOT write this file — see [`analyze/SKILL.md` §*Error handling*](../../../analyze/SKILL.md).

### 5. Idempotency

Same rules as the documentation branch (§*Documentation branch* Step 5) plus:

- `top_level_modules` in `metadata.json` is alphabetically sorted.
- Numeric fields (`loc`, `module_count`) are deterministic for a given input tree — re-running on unchanged sources produces byte-identical counts.
- No timestamps, environment variables, absolute paths, or run IDs in either output.

Reruns on unchanged inputs produce byte-identical `discovery.md` and `metadata.json`.

### 6. Error and empty-input handling

- **`$INPUT_PATH` absent or unreadable** — hard exit before writing either output; never ship a partial `$DISCOVERY` or `metadata.json`.
- **Empty adapter inventory** (tree parseable but no cohesive adapters inferable) — still emit `$DISCOVERY` with no `###` blocks, AND emit `metadata.json` with accurate structural counts. An empty project is legal.
- **Unknown `language`** (detected value not in the Omnia convention list above) — use the detected value verbatim in `metadata.json` and let the propose brief's review list surface it; do NOT block the run.

### Worked example

Fixture tree: [`plugins/change/skills/draft/briefs/omnia/fixtures/analyze/legacy-code/`](../../../draft/briefs/omnia/fixtures/analyze/legacy-code/).

Invocation (run from the fixture directory):

```
/change:analyze legacy-code monolith ./inputs/monolith/ ./expected/plans/legacy-code/
```

Input: a small TypeScript monolith under [`inputs/monolith/`](../../../draft/briefs/omnia/fixtures/analyze/legacy-code/inputs/monolith/) with four inferable adapters spanning `src/users`, `src/auth`, `src/common`, and `src/billing`.

Expected output: four adapter summaries (alphabetical: `billing-subscription`, `email-verification`, `shared-validation`, `user-registration`) plus a structural-metadata sidecar. The `user-registration` block reproduces the canonical sample from [`rfc-3a-monoliths.md` §*Plan-time analysis, define-time extraction*](../../../../../../rfcs/archive/rfc-3a-monoliths.md) in the on-disk shape. See [`expected/discovery.md`](../../../draft/briefs/omnia/fixtures/analyze/legacy-code/expected/discovery.md) and [`expected/plans/legacy-code/analyze/monolith/metadata.json`](../../../draft/briefs/omnia/fixtures/analyze/legacy-code/expected/plans/legacy-code/analyze/monolith/metadata.json) for the byte-stable targets.
