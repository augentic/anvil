# RFC-8: API Contracts — Implementation Plan (Layers 1 & 2)

## Notation

- **Repo S** = `specify` (plugins, schemas, briefs, docs)
- **Repo C** = `specify-cli` (Rust CLI crate)
- Chunks in the same **parallel group** can be implemented concurrently.
- Arrows show hard dependencies.

---

## Layer 1

### Chunk 1: Contracts plugin scaffold + writer skill references

**Parallel group: A** | Repo S

Create `plugins/contracts/` (mirroring `plugins/omnia/`):
- `plugins/contracts/.cursor-plugin/plugin.json`
- `plugins/contracts/README.md`
- `plugins/contracts/references/json-schema-conventions.md` — `$id` URI format, `title`/`description` rules, `$ref` conventions, type-mapping guidance
- `plugins/contracts/references/openapi-conventions.md` — OpenAPI 3.1 structure, `$ref → ../schemas/`, path/method/response conventions
- `plugins/contracts/references/asyncapi-conventions.md` — AsyncAPI 3.0 structure, channel/operation conventions
- `plugins/contracts/references/artifact-structure.md` — `.specify/contracts/` directory layout, naming conventions, change-level delta rules

**Files:** ~6 new. **Context:** RFC-8 §*Format choice*, §*Artifact structure*, §*Naming conventions*. Existing `plugins/omnia/.cursor-plugin/plugin.json` as format example.

---

### Chunk 2: `/contracts:writer` SKILL.md

**Parallel group: A** | Repo S

Author `plugins/contracts/skills/writer/SKILL.md`:
1. Frontmatter (name, description, argument-hint, allowed-tools)
2. Authority hierarchy
3. Hard rules (valid JSON Schema, valid OpenAPI 3.1 / AsyncAPI 3.0, `$ref` resolution, `$id` stability)
4. The 6-step algorithm: read baseline → read specs → validate alignment & determine delta → generate JSON Schema → generate/update OpenAPI → generate/update AsyncAPI
5. Alignment report output format
6. References links to `references/*.md`

**Files:** 1 new. **Context:** RFC-8 §*`/contracts:writer`*. Existing `plugins/omnia/skills/crate-writer/SKILL.md` as format example.

---

### Chunk 3: `/contracts:validator` SKILL.md

**Parallel group: A** | Repo S

Author `plugins/contracts/skills/validator/SKILL.md`:
1. Frontmatter
2. Three check categories: `$ref` resolution, schema metadata, binding completeness
3. Output format (file path + issue description per failure)
4. Scope rules: validator does not generate or modify files

**Files:** 1 new. **Context:** RFC-8 §*`/contracts:validator`*.

---

### Chunk 4: `contracts.md` brief (shared)

**Parallel group: B** (depends on Chunks 2 & 3) | Repo S

Author the brief that will be used by the standalone `contracts` schema and by Omnia/Vectis schemas:
1. Frontmatter: `id: contracts`, `generates: contracts/**/*.yaml`, `needs: [specs]`
2. Brief body: thin orchestrator delegating to `/contracts:writer` then `/contracts:validator`
3. Verify-repair loop (max 2 iterations)

This brief is placed initially at one canonical location and referenced by all three schemas (e.g. `schemas/contracts/briefs/contracts.md`, symlinked or copied to `schemas/omnia/briefs/` and `schemas/vectis/briefs/`).

**Files:** 1 new (+ 2 copies/symlinks in Chunk 7 and Chunk 8). **Context:** RFC-8 §*Brief frontmatter*, §*Brief body*, §*Verify-repair loop*. Existing `schemas/omnia/briefs/build.md` as format example.

---

### Chunk 5: Update `specs.md` briefs

**Parallel group: B** (independent) | Repo S

Update `schemas/omnia/briefs/specs.md` and `schemas/vectis/briefs/specs.md`: add a brief-body instruction to read `.specify/contracts/` as read-only context when the directory exists, writing scenarios consistent with existing endpoint paths, payload schemas, and error responses. No frontmatter changes.

**Files:** 2 modified. **Context:** RFC-8 §*Baseline contract visibility in the specs brief*.

---

### Chunk 6: Update `design.md` briefs

**Parallel group: B** (independent) | Repo S

- Omnia: change `needs: [proposal]` → `needs: [proposal, contracts]`
- Vectis: change `needs: [proposal, specs]` → `needs: [proposal, specs, contracts]`
- Reword `## API Contracts` and `## Publication & Timing Patterns` sections to reference `.specify/contracts/http/` and `.specify/contracts/messages/` respectively

**Files:** 2 modified. **Context:** RFC-8 §*Relationship to `design.md`*.

---

### Chunk 7: Standalone `contracts` schema + briefs

**Parallel group: C** (depends on Chunks 2, 3, 4) | Repo S

Author `schemas/contracts/` — a purpose-built schema for contract-only changes:

- `schemas/contracts/schema.yaml` — define pipeline (proposal, specs, contracts, tasks), build pipeline (build), merge pipeline (merge). **No `design` stage.**
- `schemas/contracts/README.md`
- `schemas/contracts/briefs/proposal.md` — contract-change-specific proposal template (interface scope, not implementation scope)
- `schemas/contracts/briefs/specs.md` — interface-level behavioral spec template (endpoint-level `SHALL` statements, not internal logic)
- `schemas/contracts/briefs/contracts.md` — the shared brief from Chunk 4 (copy or symlink)
- `schemas/contracts/briefs/tasks.md` — validation-focused task template (validate `$ref` resolution, verify schema metadata, etc.)
- `schemas/contracts/briefs/build.md` — delegates to `/contracts:validator` only, no code generation
- `schemas/contracts/briefs/merge.md` — standard merge brief

**Files:** ~8 new. **Context:** RFC-8 §*Schema integration* — the `contracts` schema subsection. Existing `schemas/omnia/` as structural reference.

---

### Chunk 8: Omnia and Vectis schema updates

**Parallel group: C** (depends on Chunk 4) | Repo S

- `schemas/omnia/schema.yaml`: insert `- id: contracts` / `brief: briefs/contracts.md` between `specs` and `design`
- `schemas/vectis/schema.yaml`: insert `- id: contracts` / `brief: briefs/contracts.md` between `specs` and `composition`
- Place `briefs/contracts.md` in both schema directories (copy/symlink from Chunk 4)

**Files:** 2 modified + 2 new brief files. **Context:** RFC-8 §*The `contracts` brief in Omnia and Vectis*.

---

### Chunk 9: Plan entry `schema` field

**Parallel group: C** (independent of other C-group chunks) | **Repo C** (CLI change)

Extend the plan entry with an optional `schema` field:
- `crates/change/src/plan.rs`: add `schema: Option<String>` to `PlanEntry`. Update serde.
- `src/main.rs`: `specify plan create` and `specify plan amend` accept `--schema <identifier>` as an alternative to `--project`
- Validation: each plan entry must have at least one of `project` or `schema`. Both may be present (an implementation change in a known project using a specific schema override).
- Update plan entry JSON schema if one exists under `schemas/plan/`
- Tests

**Files:** `crates/change/src/plan.rs`, `src/main.rs` (Repo C). Possibly `schemas/plan/plan.schema.json` (Repo S). Tests. **Context:** RFC-8 §*Schema resolution for contract changes*, item 7.

---

### Chunk 10: Generation fixture

**Parallel group: D** (depends on Chunks 2, 3, 7) | Repo S

Author worked example under `schemas/contracts/fixtures/generation/`:
- Empty baseline (spec-first pattern)
- A representative spec file describing a user registration API
- Expected writer output: JSON Schema files + OpenAPI binding
- Expected validator output: clean pass
- `README.md`

**Files:** ~5-6 new. **Context:** RFC-8 item 9.

---

### Chunk 11: Conformance fixture

**Parallel group: D** (depends on Chunks 2, 3) | Repo S

Author worked example under `schemas/contracts/fixtures/conformance/`:
- Pre-existing baseline contracts
- A change whose specs describe behavior against that baseline
- Expected writer alignment output (mostly-covered, small/empty delta)
- Expected validator results
- `README.md`

**Files:** ~5-6 new. **Context:** RFC-8 item 10.

---

### Chunk 12: `checks.ts`, docs, and project rules updates

**Parallel group: D** (depends on Chunks 1, 2, 3, 7) | Repo S

- `scripts/checks.ts`: add the contracts plugin and schema to consistency checks
- `docs/architecture.md`: document the contracts plugin
- `.cursor/rules/project.mdc`: add `/contracts:writer`, `/contracts:validator` to specialist skills list
- `AGENTS.md`: add contract skills to workflow overview

**Files:** 3-4 modified. **Context:** Existing checks and docs structure.

---

### Layer 1 dependency graph

```
Group A (parallel):   [1]  Plugin scaffold + refs
                      [2]  Writer skill
                      [3]  Validator skill

Group B (parallel):   [4]  Contracts brief        ← depends on 2, 3
                      [5]  Specs brief update       (independent)
                      [6]  Design brief update      (independent)

Group C (parallel):   [7]  Contracts schema       ← depends on 2, 3, 4
                      [8]  Omnia/Vectis updates   ← depends on 4
                      [9]  Plan entry schema (C)    (independent — Repo C)

Group D (parallel):   [10] Gen fixture            ← depends on 2, 3, 7
                      [11] Conformance fixture    ← depends on 2, 3
                      [12] Checks/docs            ← depends on 1, 2, 3, 7
```

---

## Layer 2

### Chunk 13: `specify merge` — contract file copying

**Parallel group: E** | Repo C

Extend `merge_change` in `crates/merge/src/change.rs`:
- Discover files under `<change_dir>/contracts/` recursively
- After writing baselines, copy contract files to `.specify/contracts/` preserving subdirectory structure (opaque replacement)
- Update merge summary to include contract files copied
- Unit tests with tempdir fixtures

**Files:** `crates/merge/src/change.rs` (modify), possibly new `crates/merge/src/contracts.rs`. Tests.

---

### Chunk 14: `specify spec preview` — include contract changes

**Parallel group: E** | Repo C

Extend `preview_change`:
- Discover `<change_dir>/contracts/**` files
- Classify as "added" or "replaced" (baseline exists vs not)
- Include in return value (new `ContractEntry` struct or similar)
- Update CLI output formatting in `src/main.rs`
- Tests

**Files:** `crates/merge/src/change.rs`, `crates/merge/src/lib.rs`, `src/main.rs`. Tests.

---

### Chunk 15: `specify spec conflict-check` — contract drift

**Parallel group: E** | Repo C

Extend `conflict_check`:
- After existing spec/composition checks, scan `<change_dir>/contracts/` for files
- Check if corresponding `.specify/contracts/<path>` modified after `defined_at`
- Include contract conflicts in `BaselineConflict` return
- Tests

**Files:** `crates/merge/src/change.rs`. Tests.

---

### Chunk 16: `specify validate` — contract validation rules

**Parallel group: E** | Repo C

Extend `crates/validate/src/registry.rs`:
- `contracts.schemas-dir-has-files` — when pipeline declares `contracts` brief, `.specify/contracts/schemas/` has at least one `.yaml`
- `contracts.refs-resolve` — `$ref` pointers in OpenAPI/AsyncAPI resolve
- `contracts.schema-metadata` — JSON Schema files have `$id`, `title`, `description`
- Register rules for brief_id `"contracts"` in `rules_for()`
- Tests

**Files:** `crates/validate/src/registry.rs`, `crates/validate/src/primitives.rs`. Tests.

---

### Chunk 17: `workspace sync` — materialise contracts

**Parallel group: F** (soft dep on Chunk 13) | Repo C

Extend `sync_registry_workspace` in `src/workspace.rs`:
- After materialising slots, copy `.specify/contracts/` from initiating repo into each non-symlink workspace slot's `.specify/contracts/`
- For symlink slots, no action needed
- Tests

**Files:** `src/workspace.rs`. Tests.

---

### Chunk 18: `/contracts:importer` skill

**Parallel group: F** (independent) | Repo S

Author `plugins/contracts/skills/importer/SKILL.md`:
- Format detection (Swagger 2.0, OpenAPI 3.0, 3.1, AsyncAPI 2.x, 3.0, standalone JSON Schema)
- Version upgrade rules
- Inline schema decomposition
- Specify metadata injection
- `plugins/contracts/skills/importer/references/format-detection.md`
- `plugins/contracts/skills/importer/references/upgrade-rules.md`

**Files:** ~3 new. **Context:** RFC-8 §*`/contracts:importer`*, item 16.

---

### Chunk 19: Registry contract roles

**Parallel group: F** (soft dep on Chunk 16) | Repo C + Repo S

Repo C:
- `crates/schema/src/registry.rs`: add `contracts: Option<ContractRoles>` to `RegistryProject` with `produces`, `consumes`, `imports` fields
- Extend `validate_shape` for 4 invariants (single producer, produce/import mutual exclusion, path validity, self-consistency)
- Update `specify initiative registry validate` output
- Tests

Repo S:
- Update `/spec:plan` skill (`plugins/spec/skills/plan/SKILL.md`) to document populating contract roles

**Files:** `crates/schema/src/registry.rs` (Repo C), `plugins/spec/skills/plan/SKILL.md` (Repo S). Tests.

---

### Chunk 20: `context` plan entry field

**Parallel group: F** (independent) | Repo C + Repo S

Repo C:
- `crates/change/src/plan.rs`: add `context: Option<Vec<String>>` to `PlanEntry`
- `src/main.rs`: `specify plan create --context <path>...` and `specify plan amend --context <path>...`
- Validation: context paths must be relative (no `..`, no absolute)
- Tests

Repo S:
- Update `/spec:plan` documentation for auto-population of `context`

**Files:** `crates/change/src/plan.rs`, `src/main.rs` (Repo C), `plugins/spec/skills/plan/SKILL.md` (Repo S). Tests.

---

### Layer 2 dependency graph

```
Group E (parallel):   [13] Merge contracts
                      [14] Preview contracts
                      [15] Conflict-check contracts
                      [16] Validate contracts

Group F (parallel):   [17] Workspace sync     → soft dep on 13
                      [18] Importer skill      (independent — Repo S)
                      [19] Registry roles      → soft dep on 16
                      [20] Context field        (independent)
```

---

## Full dependency graph

```
           Layer 1                                  Layer 2
           ═══════                                  ═══════

Group A ─┬─ [1]  Plugin scaffold + refs
         ├─ [2]  Writer skill             ─┐
         └─ [3]  Validator skill           ─┤
                                            │
Group B ─┬─ [4]  Contracts brief      ←────┤
         ├─ [5]  Specs brief update        │
         └─ [6]  Design brief update       │
                                            │
Group C ─┬─ [7]  Contracts schema    ←── [4]
         ├─ [8]  Omnia/Vectis updates ←─ [4]    Group E ─┬─ [13] Merge contracts
         └─ [9]  Plan entry schema (C)           (parallel)├─ [14] Preview contracts
                                                          ├─ [15] Conflict-check
Group D ─┬─ [10] Gen fixture        ←── [7]              └─ [16] Validate contracts
         ├─ [11] Conformance fixture ←── [2,3]
         └─ [12] Checks/docs        ←── [1,7]   Group F ─┬─ [17] Workspace sync
                                                 (parallel)├─ [18] Importer skill (S)
                                                          ├─ [19] Registry roles
                                                          └─ [20] Context field
```

**Minimum critical path:** A → B → C → D (Layer 1) → E → F (Layer 2)

**Maximum parallelism per group:**

| Group | Chunks | Notes |
|-------|--------|-------|
| A | 3 | All independent |
| B | 3 | Once A done |
| C | 3 | Once B done; Chunk 9 can start with A (independent Repo C work) |
| D | 3 | Once C done for fixtures needing contracts schema; Chunk 11 only needs A |
| E | 4 | Once Layer 1 artifact structure settled |
| F | 4 | Once E done |

**Cross-repo note:** Chunk 9 (plan entry `schema` field) is the only Layer 1 chunk touching Repo C. It can start as early as Group A since it has no dependency on the Repo S skills/briefs — only on the RFC's design for the `schema` field. This makes it a good candidate to run in parallel with Groups A–B.
