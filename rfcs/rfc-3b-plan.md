# RFC-3b Implementation Plan

> 10 changes across two repositories (specify-cli and specify), organized into 5 layers by dependency. Changes within the same layer can be executed in parallel.

## Layer 0 — No dependencies (parallelizable)

### Change 1: CLI struct extensions (specify-cli)

**Depends on:** nothing

**Scope:** Pure additive struct changes, backwards-compatible.

| File | What |
|------|------|
| `crates/schema/src/registry.rs` | Add `description: Option<String>` with `#[serde(default)]` to `RegistryProject` (currently lines 39–57). Add validation in `Registry::validate_shape()`: when `projects.len() > 1`, every project must have a non-empty `description`. Error code: `description-missing-multi-repo`. |
| `crates/change/src/plan.rs` | Add `project: Option<String>` with `#[serde(default)]` to `PlanChange` (lines 138–162). Add `project: Option<Option<String>>` to `PlanChangePatch` (lines 164–181) with the same three-way semantics as `description`. |
| `schemas/plan/plan.schema.json` | Add `"project": { "type": "string" }` to `#/$defs/planChange/properties`. |
| Tests | Unit tests for registry validation (`description-missing-multi-repo`), round-trip serde tests for both structs. |

### Change 2: Workspace namespace promotion (specify-cli)

**Depends on:** nothing

**Scope:** Move `specify initiative workspace {sync,status}` to `specify workspace {sync,status}`. Structural refactor only — no new behavior.

| File | What |
|------|------|
| `src/main.rs` | Add a top-level `Commands::Workspace` variant (with `WorkspaceAction` enum carrying `Sync` and `Status`). Wire dispatch to the existing `sync_registry_workspace` and `workspace_status` functions in `src/workspace.rs`. Remove the workspace sub-variants from `InitiativeAction`. |
| `src/workspace.rs` | No logic changes, just ensure public API works from the new dispatch path. |

### Change 3: Merge auto-commit (specify-cli)

**Depends on:** nothing

**Scope:** When `specify merge` runs inside a workspace clone, auto-commit the merged specs.

| File | What |
|------|------|
| `src/main.rs` (`run_merge`) | After the `merge_change` call (line 744), detect whether CWD is a workspace clone: CWD has `.specify/project.yaml` AND an ancestor matches `*/.specify/workspace/*/`. If so: `git add .specify/specs/ .specify/archive/`, `git commit -m "specify: merge <change-name>"`. Commit failure is a warning, not an error. |
| Tests | Test workspace-clone detection heuristic (positive and negative cases, including the `plan.yaml`-archived edge case). Test that staging is scoped to `.specify/` subtrees. |

### Change 4: Forward-reference fixes (specify)

**Depends on:** nothing

**Scope:** Fix stale references throughout the specify repo. No behavioral change.

| File | What |
|------|------|
| `AGENTS.md` (line 21) | `rfc-3b-layer-3.md` → `rfc-3b-platform.md` |
| `plugins/spec/skills/plan/SKILL.md` (line 17) | Same rename |
| `schemas/omnia/briefs/plan/propose.md` (~line 37) | Replace the `§Peer registry sources (Layer 2)` deferral paragraph with: "Project assignment is handled by the plan skill's assignment step (RFC-3b §Assignment algorithm), not by the propose brief. The propose brief creates entries without `--project`." |
| Throughout plan/execute SKILL.md | Rename `specify initiative workspace {sync,push,status}` → `specify workspace {sync,push,status}` |

## Layer 1 — Depends on Layer 0

### Change 5: Plan validation cross-checks (specify-cli)

**Depends on:** Change 1 (struct extensions)

**Scope:** New validation checks that cross-reference plan against registry.

| File | What |
|------|------|
| `crates/change/src/plan.rs` | Replace the unused `_project_dir: Option<&Path>` parameter on `Plan::validate` (line 282) with `registry: Option<&Registry>`. Add checks: `project-not-in-registry` (every non-None `project` must match a registry project name), `project-missing-multi-repo` (when registry has >1 project, every change must have `project`). |
| `src/main.rs` | Update all `plan.validate(...)` call sites to pass the loaded `Registry` (or `None`). Add `schema-mismatch-workspace` warning check (compare registry `schema` vs workspace clone's `project.yaml`). |
| Tests | Unit tests for all four validation checks following existing `Plan::validate` test patterns. |

### Change 6: Greenfield bootstrap (specify-cli)

**Depends on:** Changes 1 (description field for parsing), 2 (workspace namespace)

**Scope:** Extend `workspace sync` with greenfield fallback for repos that don't exist yet.

| File | What |
|------|------|
| `src/workspace.rs` | In `sync_registry_workspace` / `materialise_git_remote`: when clone fails (404 / repo not found) or URL points to non-existent local path, treat as greenfield. Sequence: `mkdir -p`, `git init`, `git remote add origin <url>`, resolve schema cache from initiating repo's `.specify/.cache/`, `chdir` + `specify init <schema> --schema-dir <dir>`, `git add . && git commit`. Implement two-tier re-run check (`.git/` present but `.specify/project.yaml` absent → re-run `specify init`). Per-project error handling. |
| Tests | Integration test for full greenfield bootstrap sequence. Integration test for partial re-run path. |

## Layer 2

### Change 7: CLI verb extensions (specify-cli)

**Depends on:** Changes 1 (structs), 5 (validation)

**Scope:** Extend CLI verbs to use the new `project` field.

| File | What |
|------|------|
| `src/main.rs` | Add `--project` flag to `specify initiative create` (maps to `specify plan create`). Add `--project` flag to `specify initiative amend`. Both validate `--project` against the loaded registry at write time (reject before writing). |
| `src/main.rs` (`run_initiative_next`) | Extend JSON response (lines 2621–2626) with `project`, `description`, and `sources` fields from the plan entry when `next` is non-null. Fields absent when `reason` is non-null. |
| Tests | Round-trip test for `--project` on create/amend. Test that invalid `--project` is rejected. Test `plan next` JSON response includes new fields. |

## Layer 3 — Depends on Layers 1–2

### Change 8: Workspace push (specify-cli)

**Depends on:** Changes 2 (namespace), 3 (merge auto-commit), 6 (greenfield bootstrap)

**Scope:** New `specify workspace push` verb.

| File | What |
|------|------|
| `src/main.rs` | Add `Push` variant to `WorkspaceAction` with optional `projects: Vec<String>`, `--dry-run`, `--format`. Dispatch to `run_workspace_push` in `src/workspace.rs`. |
| `src/workspace.rs` | Implement `run_workspace_push`: load plan (read initiative name from `plan.name`), iterate projects. Per-project: URL classification + remote resolution, `extract_github_slug` utility, `gh auth status` pre-flight, branch creation (`specify/<initiative-name>`), `git push --force-with-lease`, `gh repo create` for greenfield, `gh pr create` if no open PR. Human-readable and JSON output. `--dry-run` mode. |
| Tests | Unit tests for `extract_github_slug` (all 6 URL forms + `None`). Integration tests for per-project status classification. `--dry-run` test. |

### Change 9: Plan skill amendments (specify)

**Depends on:** Changes 1 (description in registry), 7 (plan amend --project)

**Scope:** Update plan skill for multi-repo assignment.

| File | What |
|------|------|
| `plugins/spec/skills/plan/SKILL.md` | Renumber steps: 3(a)/3(a½)/3(b) → 3(a)/3(b)/3(c). Update workspace.md shape pin to include `Description` and `Schema` bullets. Add step 3(d): assignment pass (inference heuristics, batch review, `specify plan amend --project`, append rationale to `proposal.md`). Update "state the skill mutates" section. Update `--dry-run` output shape. |
| `plugins/spec/skills/plan/fixtures/plan-layer2/workspace.md` | Update fixture to match new workspace.md shape (add Description + Schema bullets). |
| `plugins/spec/skills/plan/fixtures/multi-project/proposal.md` | New fixture: proposal table with Project and Rationale columns, including one operator override and one unresolved-then-resolved case. |

### Change 10: Execute skill amendments (specify)

**Depends on:** Change 7 (`plan next` response has `project`)

**Scope:** CWD-based routing in the execute skill.

| File | What |
|------|------|
| `plugins/spec/skills/execute/SKILL.md` | Insert new CWD routing step after `transition in-progress` and before `/spec:define`: read `project` from `plan next`, resolve target directory, check workspace freshness, save CWD, resolve source paths to absolute, `chdir`. Insert CWD restore step after phase-outcome classification. Update self-heal for multi-repo (per-entry CWD routing). Add `Routing: <name> → <project> (<path>)` diagnostic line. |
| Fixtures | `fixtures/multi-project/registry.yaml`, `fixtures/multi-project/plan.yaml`, `fixtures/multi-project/execute-loop-transcript.md`, `fixtures/multi-project/workspace-push-output.json`, `fixtures/multi-project/workspace-push-dry-run.json`, `fixtures/greenfield-bootstrap/`, `fixtures/greenfield-bootstrap/partial-rerun/`. |

## Dependency graph

```
Layer 0 (parallel):   [1: Structs]    [2: Namespace]    [3: Auto-commit]    [4: Fwd-refs]
                          │  │              │                   │
Layer 1 (parallel):   [5: Validation]  [6: Greenfield]        │
                          │                 │                   │
Layer 2:              [7: Verb exts]       │                   │
                          │  │              │                   │
Layer 3 (parallel):   [9: Plan skill]  [8: Workspace push] [10: Execute skill]
```

Concrete dependency edges:

- 5 ← 1
- 6 ← 1, 2
- 7 ← 1, 5
- 8 ← 2, 3, 6
- 9 ← 1, 7
- 10 ← 7

## Estimated effort distribution

| Change | Repo | Relative size |
|--------|------|---------------|
| 1. Struct extensions | specify-cli | Small |
| 2. Namespace promotion | specify-cli | Small |
| 3. Merge auto-commit | specify-cli | Medium |
| 4. Forward-ref fixes | specify | Small |
| 5. Validation cross-checks | specify-cli | Medium |
| 6. Greenfield bootstrap | specify-cli | Large |
| 7. Verb extensions | specify-cli | Medium |
| 8. Workspace push | specify-cli | Large |
| 9. Plan skill amendments | specify | Large |
| 10. Execute skill amendments | specify | Medium |

## Notes for subagent scoping

- Each change touches at most one repository, keeping context small.
- Changes 1–3 and 5–8 target specify-cli; changes 4, 9, 10 target the specify plugins repo.
- The largest changes (6, 8, 9) are at the edges of the graph so they don't block other work.
- For the specify repo changes (4, 9, 10), the subagent only needs to edit markdown skill files and fixtures — no Rust compilation required.
- The plan skill (Change 9) is the most context-heavy: it needs the RFC's assignment algorithm, inference heuristics, and the workspace.md shape. That context should be included in the subagent prompt.
