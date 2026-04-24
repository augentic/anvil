# RFC-3b: Platform Changes

> Status: Draft · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-2](archive/rfc-2-execution.md), [RFC-3a](archive/rfc-3a-monoliths.md)

## Abstract

RFC-3a lands initiative *planning* across repos: a platform catalogue (`registry.yaml`), an operator-authored brief (`initiative.md`), a *sync peers* phase that materialises `.specify/workspace/<peer>/`, and a single cross-repo `plan.yaml`. That plan contains an ordered list of changes — but no indication of *which repo* each change belongs to. RFC-3b bridges the gap: it determines which registry project each change targets, records that assignment in the plan, and teaches `/spec:execute` to route each change's define-build-merge cycle to the correct repo with the correct schema.

The assignment problem has two shapes. For **brownfield** work — modernising or extending an existing multi-repo platform — the framework infers assignment from the domain descriptions operators write in `registry.yaml`, cross-referenced against each change's description and the peer baseline specs already materialised by RFC-3a's sync-peers phase. For **greenfield** work — standing up new repos for a system that does not yet exist — the operator predetermines the repo topology by authoring registry entries with descriptions that capture the intended responsibility boundaries (e.g. "frontend", "backend API", "shared types"). In both cases the propose brief presents the inferred assignment for human review; the operator can override.

RFC-3b is a follow-up to RFC-3a rather than a layer of it because the planning flow (Layers 1–2 + Large-Monolith Decomposition) is independently useful without routing, and routing depends on the workspace and registry being in place first.

## Motivation

RFC-3a's plan output is a single cross-repo `plan.yaml` in the initiating repo. Each change entry carries a `description` and optional `sources`, but nothing that says "run this change against the `traffic` repo using the `omnia@v1` schema." Without routing metadata:

- `/spec:execute` cannot determine *where* to run define-build-merge for a given change.
- The schema used for each change is ambiguous when the registry declares projects with different schemas (e.g. `omnia@v1` for the backend, `vectis@v1` for the mobile app).
- The operator must manually track which changes belong to which repos — the plan looks like a flat list rather than a coordinated cross-repo programme.

RFC-3b closes this gap with three additions: a `description` field on registry projects (the domain signal), a `project` field on plan changes (the routing decision), and an assignment algorithm in the propose brief (the inference).

## Registry extension: project `description`

RFC-3a's `RegistryProject` carries `name`, `url`, and `schema`. RFC-3b adds an optional `description` field — a short, domain-level characterisation of what the project owns:

```yaml
# .specify/registry.yaml
version: 1
projects:
  - name: traffic
    url: git@github.com:org/traffic.git
    schema: omnia@v1
    description: >
      Real-time traffic ingestion and route optimisation.
      Owns Kafka consumers, the routing engine, and the
      traffic-state read model.

  - name: command-centre
    url: git@github.com:org/command-centre.git
    schema: omnia@v1
    description: >
      Operator dashboard and alerting. Owns the web UI,
      notification dispatch, and escalation workflows.
```

### Semantics

- `description` is a free-form string. It describes the project's **business domain** — the capabilities the repo owns — not its tech stack (that is `schema.yaml`'s `domain` field).
- `description` is optional when the registry declares a single project (routing is trivially "everything goes here"). When `len(projects) > 1`, `description` is required on every project entry — `specify initiative registry validate` enforces this.
- The description is stable across initiatives. Like the rest of `registry.yaml`, it describes the platform, not a cycle.

### Brownfield inference signal

For brownfield platforms, the `description` is the primary signal the propose brief uses to match capabilities from `discovery.md` to projects. It is complemented by secondary signals available from the workspace:

1. **Baseline specs.** Each peer's `.specify/workspace/<peer>/specs/` (materialised by RFC-3a sync-peers) contains the capabilities already specified in that repo. A discovered capability whose name or domain overlaps with existing baseline specs has a strong affinity signal.
2. **Schema identity.** When projects use different schemas, the schema itself is a coarse routing signal (e.g. a UI capability is unlikely to route to an `omnia@v1` backend project if a `vectis@v1` frontend project exists).

The `description` is always operator-authored. RFC-3b does not attempt to infer project descriptions from baseline specs — the operator knows the intended responsibility boundaries; the framework matches capabilities against those boundaries.

### Greenfield guidance

For greenfield initiatives, no baseline specs exist. The operator authors registry entries with descriptions that capture the *intended* code organisation:

```yaml
projects:
  - name: api
    url: .
    schema: omnia@v1
    description: >
      REST API and business logic. Owns all server-side
      capabilities, data access, and external integrations.

  - name: mobile
    url: git@github.com:org/mobile.git
    schema: vectis@v1
    description: >
      iOS and Android mobile application. Owns all client-side
      UI, navigation, and offline-first behaviour.
```

The descriptions serve the same role as in brownfield — they are the signal the propose brief matches against — but since no baseline specs exist, the description carries the full weight of the routing decision. For greenfield projects, `url` should be the git remote URL (not a local path) so that `workspace sync` can bootstrap the clone and `workspace push` can push to the remote.

### URL classification

The `url` field on `RegistryProject` serves both sync and push. The framework classifies each URL to determine behaviour:

- **Remote URL** — starts with `git@`, `ssh://`, `https://`, or `http://`. Used by `workspace sync` for cloning and by `workspace push` for pushing. For greenfield projects whose remote does not yet exist, `workspace sync` falls back to local bootstrapping (see §*Greenfield bootstrapping*).
- **Local path** — everything else (`.`, `../foo`, `/absolute/path`). Used by `workspace sync` to resolve the project on the local filesystem (symlink or direct reference). For push, `workspace push` reads `git remote get-url origin` from the resolved repo to discover the push target. If no `origin` remote is configured, the project is classified as **local-only**: `workspace push` skips it with a `"local-only"` status and emits a diagnostic advising the operator to either configure a git remote in the repo or switch `url` to a git remote URL.

This keeps `registry.yaml` minimal — one `url` field per project — while supporting the full sync-and-push lifecycle without ambiguity.

## Plan extension: the `project` field

RFC-3b adds a `project` field to `planChange` in `plan.schema.json`:

```yaml
changes:
  - name: ingest-pipeline
    project: traffic
    sources: [monolith]
    description: "Extract the Kafka ingestion pipeline into a standalone capability."
    status: pending

  - name: operator-dashboard
    project: command-centre
    description: "Build the operator alerting dashboard."
    status: pending
    depends-on: [ingest-pipeline]
```

### Semantics

- `project` is a kebab-case string that must match a `projects[].name` in `registry.yaml`. Validated by `specify plan validate`.
- For single-project registries (or absent registry), `project` is optional. Absence means "the current repo" — the pre-RFC-3b default, fully backwards compatible.
- For multi-project registries (`len(projects) > 1`), `project` is required on every change entry. `specify plan validate` rejects entries without it.
- `project` determines which schema governs the change's define-build-merge cycle: `registry.yaml[project].schema` resolves to the schema whose briefs `/spec:execute` will invoke.

### `specify plan create` extension

`specify plan create` gains an optional `--project` flag:

```text
specify plan create <name> \
    [--project <registry-project-name>] \
    [--sources <key> ...] \
    [--depends-on <name> ...] \
    [--description "..."]
```

`--project` specifies which registry project a change targets — analogous to `--sources` specifying which sources it draws from. The propose brief passes `--project` for each accepted slice. For single-repo plans, omitting `--project` preserves today's behaviour.

`PlanChangePatch` gains a corresponding `project` field (`Option<Option<String>>` — same three-way semantics as `description`) so that `specify plan amend <name> --project <project>` is available for post-creation corrections.

## Assignment algorithm

The assignment algorithm runs inside the **propose brief** (schema-owned, not framework-level). RFC-3b defines the contract the propose brief must satisfy; the algorithm itself is schema-specific.

### Inputs

The propose brief receives:

1. `**discovery.md`** — the capability inventory from step 3(a). Each capability carries `name`, `summary`, `sources`, `depends-on`, and `confidence`.
2. `**workspace.md`** — the peer inventory from step 3(a½) (multi-repo only). Each peer's entry includes its baseline specs, schema, materialisation state, and the project's `description` from `registry.yaml`. The description bullet makes `workspace.md` a self-contained context document for assignment — the propose brief does not read `registry.yaml` directly.

### Contract

For each candidate slice the propose brief produces, it must also produce a `project` assignment. The assignment is presented to the operator alongside the slice in the accept / edit / reject / abort loop. The operator can override the inferred project during the edit action.

The propose brief must record the assignment rationale in `proposal.md` for auditability — a short phrase per slice (e.g. "matched: description overlap with `traffic` — ingestion, Kafka", or "matched: baseline spec `user-registration` exists in `command-centre`", or "operator override").

### Inference heuristics (normative contract, not algorithm)

The propose brief should use the following signal priority:

1. **Description match.** Compare the capability's `summary` and `depends-on` edges against each project's `description`. Domain-term overlap is the primary signal.
2. **Baseline spec affinity.** If a peer already has baseline specs whose names or domains overlap with the capability, that peer is a strong candidate. This signal is only available for brownfield (materialised workspace with existing specs).
3. **Schema compatibility.** If the capability's nature (e.g. UI vs backend logic) aligns with only one schema type in the registry, use that as a tiebreaker.
4. **Ambiguity → human.** When no signal clearly differentiates, or when confidence is low, surface the assignment as "unresolved" and require operator input. Never silently assign a low-confidence match.

The ranking and weighting of these signals is schema-owned. RFC-3b fixes the signal vocabulary and the "ambiguity → human" rule; schemas decide how to combine them.

### Proposal shape (multi-repo)

When the registry declares more than one project, the propose brief's `proposal.md` table gains two columns: `Project` and `Rationale`.

```markdown
# Proposal — <initiative-name>

## Slices

| # | Slice | Project | Source(s) | Depends on | Rationale | Decision | Plan entry |
|---|---|---|---|---|---|---|---|
| 1 | ingest-pipeline | traffic | monolith | — | description overlap: ingestion, Kafka | accept | ingest-pipeline |
| 2 | operator-dashboard | command-centre | — | ingest-pipeline | baseline spec: user-alerts exists | accept | operator-dashboard |
| 3 | shared-types | ? | — | — | ambiguous: matches both projects | edit → accept (operator: traffic) | shared-types |

## Notes

- shared-types: operator override — assigned to traffic for co-location with ingestion types.
```

The interactive loop's **edit** action gains `project` as an editable field alongside name, sources, depends-on, and description. The `project` prompt is a pick-from-list field, not free text — the legal values are the `projects[].name` entries from `registry.yaml`. The inferred value is the default: `Project [traffic]: `. Invalid input (a name not in the registry) re-prompts.

For **unresolved** assignments (ambiguity → human rule), the `Project` column shows `?` and the prompt requires the operator to assign a project before accept is available.

For each accepted slice, the propose brief shells out to:

```text
specify plan create <name> \
    --project <project> \
    --sources <key> \
    [--depends-on <dep> ...] \
    --description "<rich prose>"
```

### Single-repo plans

When the registry is absent or single-project, the propose brief skips assignment entirely. The `Project` and `Rationale` columns are omitted from `proposal.md`. No `--project` flag is passed to `specify plan create`. This is the backwards-compatible path — pre-RFC-3b plans are valid without modification.

## Execution routing

`/spec:execute` already reads `plan.yaml` entries and drives define-build-merge per change. RFC-3b extends the driver to route each change to the correct project using **CWD-based routing**: the driver changes working directory to the target project's root before invoking phase skills. Phase skills (define, build, merge) are unaware of multi-repo routing — they run unmodified in whatever working directory the driver places them in.

### Working directory

Each change's define-build-merge cycle runs against the **project's clone in the workspace** — not the initiating repo's root:

- For `url: .` or relative-path projects, the resolved filesystem path is the working directory.
- For remote projects, `.specify/workspace/<name>/` (the clone materialised by RFC-3a sync-peers) is the working directory.

The driver resolves the target project's filesystem root from `registry.yaml`, sets the working directory to that root, and invokes `/spec:define <name>` with no additional project flag. Define discovers `.specify/project.yaml` via its normal CWD walk and resolves the schema from that file. Build and merge follow the same pattern. Phase skills need no changes — the driver owns the routing decision entirely.

In `--loop` mode, the driver saves and restores CWD around each change's define-build-merge cycle so that the next iteration's `specify plan next` (which reads the initiating repo's `plan.yaml`) runs from the initiating repo root.

The workspace clone is **writable during execution**. Spec artifacts (design.md, specs/, tasks/) are written into the project clone's `.specify/changes/<name>/` subtree during define and build; on merge, the delta specs are folded into the clone's `.specify/specs/` baseline and committed locally. The commit remains in the workspace clone until the operator explicitly pushes via `specify initiative workspace push` (see §*Workspace push*).

### Schema resolution

Schema resolution happens via the target project clone's `.specify/project.yaml`, not via the driver looking up `registry.yaml[project].schema`. When the driver `chdir`s into a workspace clone:

1. Define reads `.specify/project.yaml` in CWD → `{ schema: "omnia@v1", ... }`.
2. `specify schema pipeline define` resolves the schema directory from that identifier.
3. The schema's `pipeline.define`, `pipeline.build`, and `pipeline.merge` briefs govern the change's lifecycle.

The registry's `schema` field is used at validation time (see §*Validation*, `schema-mismatch-workspace`) but is not the resolution path during execution.

For changes without a `project` field (single-repo plans), resolution is unchanged from RFC-2.

### Workspace freshness

Before executing a change targeting a peer project, `/spec:execute` checks the workspace slot's materialisation state via `specify initiative workspace status`. If the slot is `missing`, execution halts with a diagnostic pointing the operator at `specify initiative workspace sync`. Stale-but-present slots are accepted — the operator controls freshness via explicit `specify initiative workspace sync` calls between execution runs, matching the RFC-3a `--extend` contract.

### Source path resolution

Source paths in the plan's top-level `sources` map are interpreted relative to the **initiating repo root** (where `plan.yaml` lives). Before changing working directory to the target project's clone, the driver resolves each source value to an **absolute filesystem path**. Git URLs are passed through unchanged.

The resolved absolute path is what gets passed to `/spec:define --source <key>=<absolute-path>`. This ensures source paths remain valid regardless of which project clone the driver has `chdir`'d into.

All existing argument-resolution rows (`sources`, `depends-on`, `description`) are unchanged in shape. The `project` field is consumed by the driver for CWD routing and is **not** forwarded as a flag to any phase skill.

### Sources map scoping

The plan's top-level `sources` map is initiative-scoped and shared across all changes regardless of target project. Each change's `sources` list references only the keys relevant to its scope. Per-project source scoping is not needed — the map is a flat namespace keyed by operator-chosen identifiers.

### `specify plan next` extension

`specify plan next --format json` gains the full plan-entry fields in its response so the driver can route without a second round-trip:

```json
{
  "next": "ingest-pipeline",
  "reason": null,
  "project": "traffic",
  "description": "Extract the Kafka ingestion pipeline into a standalone capability.",
  "sources": ["monolith"]
}
```

- `project` is `null` when the entry has no `project` field (single-repo plans). The driver skips CWD routing and falls through to the pre-RFC-3b path.
- `description` and `sources` are included for parity — the driver already needs both for argument resolution. Surfacing them in `plan next` eliminates the TOCTOU window between "pick entry" and "read entry fields." The execute skill reads these three fields from the `plan next` response rather than performing a separate plan read — the `plan next` response is the single source of truth for the current entry's routing and argument-resolution data.
- When `reason` is non-null (`"all-done"`, `"stuck"`, `"in-progress"`), the entry fields are absent and `next` is `null` — unchanged from today.

### Merge commit contract

Merge commits inside workspace clones follow these rules:

- **Branch.** Commits land on the clone's current HEAD — whatever branch `workspace sync` checked out (typically `main` or `master` for brownfield; the default branch from `git init` for greenfield). `workspace push` later creates or force-updates a `specify/<initiative-name>` branch from HEAD before pushing. This separation keeps merge simple (it doesn't need the initiative name) and puts branch-naming policy entirely in `workspace push`.
- **Who commits.** `specify merge` (the CLI verb) auto-commits when it detects it is running inside a workspace clone. The heuristic is: CWD contains `.specify/project.yaml` **and** CWD is a subdirectory of a `.specify/workspace/` tree (i.e. an ancestor directory matches `*/.specify/workspace/*/`). The secondary check — CWD does **not** contain `.specify/plan.yaml` — is retained as a safety guard but is not sufficient on its own, because `plan.yaml` may be absent after `specify initiative archive`. The workspace-path ancestry test is the primary signal. The commit message follows the existing convention: `"specify: merge <change-name>"`.
- **Git staging scope.** The auto-commit stages only `.specify/` subtrees: `git add .specify/specs/` (merged baselines) and `git add .specify/archive/` (archived change directory). Non-Specify files (application code, configuration, etc.) are never staged by the auto-commit. If the index already has staged changes from non-Specify files, they ride along in the commit — the commit-failure path (below) handles the case where this is undesirable. The commit message format `"specify: merge <change-name>"` is a hard contract pinned by the `execute-loop-transcript.md` fixture.
- **Implementation location.** The auto-commit logic lives in `run_merge` in `src/main.rs` (post-`merge_change` call), not in `crates/merge/`. The merge engine stays git-unaware; the CLI handler owns the git integration.
- **Commit failure.** If the git commit fails (dirty index from non-Specify files, merge conflicts in tracked files, etc.), `specify merge` still succeeds at the spec-merge level — the commit failure is a **warning**, not an error. `workspace push` will detect uncommitted changes and surface them as a per-project diagnostic. The operator resolves manually before pushing.

### Workspace writability policy

RFC-3a describes workspace clones as read-only during planning. RFC-3b relaxes this to writable during execution. The policy is enforced by convention, not by filesystem permissions or a write-guard mechanism. The plan skill's sync-peers step reads but does not write into workspace clones; the execute skill writes via the normal phase skills running under CWD-based routing. No code change to `workspace.rs` is needed — the relaxation is a documented policy update to the execute skill's guardrails section.

### Execute skill amendments

The execute skill's per-change algorithm (SKILL.md §Per-change algorithm) gains two new steps and one modified step for CWD-based routing. Phase skills are unaffected. Step references below use semantic anchors rather than step numbers — match by the described operation, not the current numbering, since numbering shifts when the new steps are inserted.

**New step: CWD routing.** Inserted after the `specify plan transition <name> in-progress` step and before the `/spec:define` invocation step. All subsequent step numbers in the SKILL.md shift by one.

1. Read `project` from the `specify plan next` response (see §*`specify plan next` extension*).
2. If `project` is non-null, resolve the target directory from `registry.yaml`: relative-path `url` → resolved filesystem path; remote `url` → `.specify/workspace/<name>/`.
3. Check workspace freshness via `specify initiative workspace status` for that slot. If `missing`, halt with a diagnostic pointing the operator at `specify initiative workspace sync`. Release the lock and exit non-zero.
4. Save CWD (the initiating repo root).
5. Resolve every key in the entry's `sources` list to an absolute filesystem path anchored to the initiating repo root. Git URLs pass through unchanged.
6. `chdir` into the target project root.

If `project` is null, the CWD routing step is skipped entirely — the pre-RFC-3b single-repo path.

**Diagnostic output for the CWD routing step.** The driver emits a routing line before entering the target project:

```text
Routing: <name> → <project> (<resolved-path>)
```

Under `--dry-run`, the same line is emitted prefixed by the existing `[dry-run]` banner. The routing line is pinned by the `execute-loop-transcript.md` fixture.

**Modified step: source argument resolution.** The `/spec:define` invocation step (previously immediately after the `transition in-progress` step, now immediately after the CWD routing step) uses the absolute paths resolved in the CWD routing step instead of the raw plan values. The invocation shape is unchanged: `--source <key>=<absolute-path>`.

**New step: CWD restore.** Inserted after the phase-outcome classification step (the `specify change outcome` read and success/failure/deferred dispatch) and before the success/failure/deferred wrap-up steps (transition done / drop+transition failed / drop+transition blocked). All subsequent step numbers shift by one.

1. Restore CWD to the saved initiating repo root from the CWD routing step.

This ensures `specify plan transition` (which reads `plan.yaml` in the initiating repo) runs from the correct directory. In `--loop` mode, the CWD routing and CWD restore steps bracket every iteration so that `specify plan next` always runs from the initiating repo root.

### Self-heal under multi-repo

Self-heal (execute skill §Self-heal on startup) reads `.specify/changes/<name>/.metadata.yaml` to reconcile in-progress entries left by a prior crash. Under multi-repo, that path lives in the target project's workspace clone, not the initiating repo.

For each `in-progress` entry `E` in `plan.yaml`:

1. Read `E.project` from the plan entry. If non-null, resolve the target project directory from `registry.yaml` (same resolution as the CWD routing step of the per-change algorithm).
2. Check workspace freshness for that slot. If `missing`, halt — same semantics as the main loop.
3. Look for `.specify/changes/<E.name>/.metadata.yaml` under the resolved project root instead of the initiating repo root. The classification logic (step 2 of self-heal) and recovery journal append (step 4) are unchanged.
4. Restore CWD to the initiating repo root after each entry's reconciliation.

For entries without a `project` field, self-heal is unchanged from RFC-2.

## Validation

`specify plan validate` gains four new checks:

- **Project references registry** (`project-not-in-registry`). Every `project` value on a change must match a `projects[].name` in `registry.yaml`. *Error.*
- **Project required for multi-repo** (`project-missing-multi-repo`). When `len(projects) > 1`, every change must carry a `project` field. *Error.*
- **Description required for multi-repo** (`description-missing-multi-repo`). When `len(projects) > 1`, every registry project must carry a `description`. *Error.*
- **Schema mismatch between registry and workspace clone** (`schema-mismatch-workspace`). When a workspace clone exists and its `.specify/project.yaml` declares a `schema` that differs from the corresponding `registry.yaml` project entry's `schema`, emit a diagnostic. *Warning* (not error) — the clone's `project.yaml` is authoritative at execution time, but the mismatch likely indicates a stale clone or a registry typo.

The first two checks require cross-referencing the plan against the registry. The fourth check requires reading workspace clones on disk and is skipped when no workspace exists. `specify plan validate` (in `main.rs`) already loads `registry.yaml` for the existing `registry-shape` hook. The loaded `Registry` (or `None` when no registry exists) is passed to `Plan::validate` as a new parameter — see §*Migration* for the signature change. When `registry` is `None`, both project-related checks are skipped.

`specify plan create` and `specify plan amend` validate `--project` against the loaded registry at write time, not only at `specify plan validate` time. A `--project` value that doesn't match `registry.yaml` is rejected before the plan entry is written.

## Design decisions

### Workspace-centric execution

All multi-repo work is undertaken in a single workspace rooted in the registry repo. The operator creates an initiative from the registry repo, which triggers all registry projects to be cloned into `.specify/workspace/<project>/` via `specify initiative workspace sync`. Each change's define-build-merge cycle runs against the relevant project clone inside this workspace. On merge, the resulting specs and code artifacts are committed to the clone. Changes are pushed to remotes explicitly via `specify initiative workspace push` — the operator controls when pushes happen; `/spec:execute` never pushes automatically.

### Greenfield bootstrapping

For greenfield projects whose remote repos do not yet exist, `workspace sync` and `workspace push` collaborate to handle the full lifecycle:

**Precondition.** The initiating repo must have a populated `.specify/.cache/` with schema definitions for every `schema` identifier referenced in `registry.yaml`. `workspace sync` does not download or populate schema caches; it reuses the initiating repo's cache. If a schema identifier in a registry entry has no corresponding cache entry in the initiating repo, the per-project bootstrap fails with a diagnostic pointing the operator at `/spec:init` in the initiating repo.

**`workspace sync` (local bootstrapping).** For each registry project whose workspace slot is `missing`:

1. Attempt a shallow clone from the registry entry's `url`. If the clone succeeds (brownfield), the slot is materialised as today.
2. If the clone fails (404, repo not found) or the `url` is a relative path to a non-existent directory, treat the project as greenfield:
   - Create the workspace slot directory (`mkdir -p .specify/workspace/<name>/`).
   - `git init` inside the slot.
   - `git remote add origin <url>` (from `registry.yaml`). The `url` must be a remote URL for greenfield projects — local paths have no meaningful remote to add.
   - Resolve the schema source directory from the **initiating repo's** `.specify/.cache/` using the same `locate_schema_root` logic that `Schema::resolve` uses. For a bare schema identifier like `omnia@v1`, the resolved path is `<initiating-repo>/.specify/.cache/omnia@v1/`; for a URL-shaped identifier, the last non-empty path segment (before any `@ref`) names the subdirectory under `.cache/`. `workspace sync` calls `locate_schema_root(registry_entry.schema, initiating_repo_dir)` to obtain the path and passes it as `--schema-dir`. If the path does not exist, the per-project bootstrap fails with a diagnostic: `"schema '<identifier>' not cached in <initiating-repo>/.specify/.cache/; run /spec:init in the initiating repo first."` Then `chdir` into the workspace slot and run `specify init <registry-entry-schema> --schema-dir <resolved-cache-dir>` to scaffold `.specify/project.yaml`. No new CLI flags are needed — `workspace sync` uses the existing `specify init` positional + `--schema-dir` interface that the agent and skills already use. The `chdir` ensures `specify init` writes `.specify/project.yaml` into the slot, not the initiating repo.
   - `git add . && git commit -m "Initial Specify scaffold"`.

**Error handling.** Each project's bootstrap is independent. A failure in one project (e.g. `specify init` fails because the schema name in `registry.yaml` is invalid, or `git init` fails due to filesystem permissions) is reported as a per-project error; sync continues to the next project.

A partially bootstrapped slot (e.g. `git init` succeeded but `specify init` failed) is **left on disk** with a diagnostic. On re-run, `workspace sync` uses a two-tier check for existing directories:

1. If `dest/.git/` exists **and** `dest/.specify/project.yaml` exists → the slot is a healthy clone or a complete greenfield bootstrap. `workspace sync` runs the existing brownfield refresh path (`git fetch --depth 1`) if the slot is a git clone, or no-ops if it is a symlink.
2. If `dest/.git/` exists but `dest/.specify/project.yaml` is **absent** → partial greenfield bootstrap. `workspace sync` re-runs the `specify init` step (resolve schema cache, `chdir`, `specify init`, `git add . && git commit --amend -m "Initial Specify scaffold"`) without re-running `git init` or `git remote add`. The `--amend` ensures the scaffold commit is updated rather than creating a second commit. If the re-run also fails, the per-project error is reported and sync continues.
3. If `dest` exists but has no `.git/` → existing error path (non-git directory; `workspace sync` errors with "not a git clone").

This avoids the case where `materialise_git_remote` sees `.git/` on a partial greenfield slot and tries `git fetch --depth 1` against a remote that doesn't exist (the `git init`'d repo has no tracking branch). The operator can always delete the slot and re-sync as an escape hatch.

Non-zero exit if any project failed, with a per-project status summary matching the `workspace push` output shape.

After `workspace sync`, every successfully bootstrapped workspace slot is a valid Specify project with `.specify/project.yaml` — whether brownfield or greenfield. Phase skills discover the project via their normal CWD walk and need no special greenfield handling.

### Workspace push

```text
specify initiative workspace push [<project>...]
```

Pushes workspace clones that have local commits back to their remote repositories. Omitting the project argument pushes all dirty clones.

The initiative name used for branch naming (`specify/<initiative-name>`) is read from `.specify/plan.yaml`'s `name` field. `workspace push` loads the plan via `Plan::load` (the same path `specify plan status` uses) and reads `plan.name`. The plan is considered **active** when `.specify/plan.yaml` exists on disk — no further checks on entry statuses are required. If `.specify/plan.yaml` does not exist (never created, or already swept to `.specify/archive/plans/` by `specify initiative archive`), the verb exits non-zero with a diagnostic: `"No active plan found at .specify/plan.yaml. Run 'specify initiative init' to create one, or check whether the plan was already archived."`

**Per-project algorithm:**

1. **Remote resolution.** Classify the registry entry's `url` (see §*URL classification*). If the URL is a remote URL, use it as the push target. If it is a local path, read `git remote get-url origin` from the resolved repo or workspace clone; if an `origin` remote exists, use that as the push target. If no `origin` remote is configured, skip the project with `"local-only"` status and a diagnostic advising the operator to either configure a git remote in the repo or switch `url` to a git remote URL.
2. **Branch.** Create or update `specify/<initiative-name>` from the clone's current HEAD.
3. **Remote repo creation (greenfield).** Detect whether the remote repository exists via `gh repo view <org/name> --json name` (non-zero exit = does not exist). The `<org/name>` slug is extracted from the resolved remote URL by a `extract_github_slug` utility function with the following rules:

   | URL form | Extraction rule | Example → slug |
   |---|---|---|
   | `git@github.com:<org>/<repo>.git` | strip `git@github.com:` prefix + `.git` suffix | `git@github.com:org/mobile.git` → `org/mobile` |
   | `git@github.com:<org>/<repo>` | strip prefix only (no `.git`) | `git@github.com:org/mobile` → `org/mobile` |
   | `https://github.com/<org>/<repo>.git` | strip scheme + host prefix + `.git` suffix | `https://github.com/org/mobile.git` → `org/mobile` |
   | `https://github.com/<org>/<repo>` | strip scheme + host prefix | `https://github.com/org/mobile` → `org/mobile` |
   | `ssh://git@github.com/<org>/<repo>.git` | strip scheme + user + host + leading `/` + `.git` | `ssh://git@github.com/org/mobile.git` → `org/mobile` |
   | Any other form | returns `None` | `git@gitlab.com:org/repo.git` → `None` |

   When `extract_github_slug` returns `None`, skip repo creation and let the push step surface any authentication or not-found error. When it returns `Some(slug)`, check existence via `gh repo view <slug> --json name`. If the remote does not exist, create it via `gh repo create <slug> --private --source .`. This keeps GitHub API interaction at the write boundary — `workspace sync` is local-only. The extraction function has unit tests covering all six rows in the table above (see §*Test coverage*).
4. **Push.** `git push --force-with-lease -u origin specify/<initiative-name>`. The lease guard prevents clobbering concurrent pushes to the same branch from another workspace (e.g. a teammate's parallel initiative run). If the lease check fails, the per-project status is `"failed"` with a diagnostic advising `workspace sync` to refresh the clone before retrying.
5. **PR.** If no open PR exists for this branch, create one via `gh pr create --title "specify: <initiative-name>" --body "<auto-generated summary>"`. If a PR already exists, the branch push updates it; the PR body is left unchanged.
6. **Report.** Per-project status line.

**Pre-flight check.** The `gh auth status` pre-flight is required only when at least one project needs repo creation (greenfield) or PR creation. If all target projects have existing remotes and open PRs, `workspace push` falls through to plain `git push` without requiring `gh`. When `gh` is required, if it is not installed or not authenticated, the verb emits a single diagnostic listing which projects need `gh` and which do not, then exits non-zero before attempting any project. This avoids N identical `gh`-related errors.

**Error handling.** Per-project errors (push rejection, remote permission issues) are reported and the verb continues to the next project. Non-zero exit if any project failed.

**`--dry-run` mode.** `workspace push --dry-run` runs the remote-resolution and status-classification steps for each project but performs no writes. Concretely it MUST NOT: run `git push`, run `gh repo create`, run `gh pr create`, or modify any branch. It MUST: load the plan and registry, classify each project's push status (would push, up-to-date, local-only, would create repo), and emit the same output format as the real run with a `[dry-run]` banner on the first line. The `--dry-run` output uses the same status vocabulary (`pushed`, `created`, `up-to-date`, `local-only`, `failed`) but all action statuses are prefixed with "would-" in the human-readable output (e.g. `would-push`, `would-create`). The JSON output adds `"dry_run": true` at the top level. The `gh auth status` pre-flight check still runs under `--dry-run` so the operator learns about missing credentials before attempting a real push.

**Output format (human-readable, default):**

```text
specify: workspace push — <initiative-name>

  traffic        pushed       specify/platform-v2  PR #42
  command-centre up-to-date
  mobile         created      specify/platform-v2  PR #7

1 created, 1 pushed, 1 up-to-date. 0 failed.
```

The summary line uses fixed-order status buckets (`created`, `pushed`, `up-to-date`, `failed`) so downstream tooling can parse it. `created` means the remote repo was created (greenfield); `pushed` means an existing remote was updated; `up-to-date` means no local commits ahead of the remote; `failed` includes a per-project error line for any that errored.

**Output format (machine-readable, `--format json`):**

```json
{
  "projects": [
    { "name": "traffic", "status": "pushed", "branch": "specify/platform-v2", "pr": 42 },
    { "name": "command-centre", "status": "up-to-date" },
    { "name": "mobile", "status": "created", "branch": "specify/platform-v2", "pr": 7 },
    { "name": "local-lib", "status": "local-only" }
  ]
}
```

The `"created"` status indicates the remote repo was created as part of this push (greenfield). `"pushed"` indicates an existing remote was updated. `"up-to-date"` indicates the clone has no local commits ahead of the remote. `"local-only"` indicates the project's `url` is a local filesystem path and no `origin` git remote is configured in the repo — `workspace push` skips these projects with a diagnostic (see §*URL classification*).

PRs are merged manually by the operator. A future extension may add `specify initiative workspace merge` for automated PR merging, but this is out of scope for RFC-3b.

### Cross-repo dependency ordering

`/spec:plan` produces an ordered set of changes; `/spec:execute` processes them in that order, respecting `depends-on` edges as it does today. Cross-repo `depends-on` edges (a change in `traffic` depending on a change in `command-centre`) are legal and enforced by the same ordering logic — there is no special cross-project case. That said, `depends-on` edges across project boundaries are expected to be uncommon in practice. Schemas should prefer decomposing capabilities so that each project's changes are internally ordered, with cross-project coordination handled at the initiative level rather than through fine-grained inter-project dependencies.

### One change, one project

Each plan change targets exactly one project. Capabilities that span multiple repos (e.g. "add OAuth login" touching both a backend and a frontend) are decomposed into separate plan entries — one per project — linked by `depends-on` edges where ordering matters. Cross-cutting concerns (shared auth libraries, API contracts, protocol definitions) are a framework-level concern addressed by the platform's dependency management and build tooling, not by the Specify code-generation pipeline. The one-change-one-project model keeps the execution loop simple: each define-build-merge cycle has a single project root, a single schema, and a single set of baseline specs.

### Peer-to-peer spec references (deferred)

A spec in `traffic` referencing a capability in `command-centre` (`@peer:capability` syntax) is an execution-time federation concern that sits downstream of routing. It is deferred until real multi-repo initiatives surface a concrete need. The workspace already materialises peer baselines under `.specify/workspace/<peer>/specs/`, so the read path exists; the reference syntax and resolution rules can be designed against real examples when the need arises.

## Implementation scope

RFC-3b touches four layers of the stack. Each item below is in scope; items not listed are out of scope.

### CLI (`specify-cli`)

- `RegistryProject` gains `description` field + validation.
- `PlanChange` gains `project` field; `PlanChangePatch` gains `project` (three-way).
- `plan.schema.json` gains `"project"` property.
- `specify plan create` gains `--project` flag.
- `specify plan amend` gains `--project` flag.
- `specify plan next --format json` gains `project`, `description`, `sources` in response.
- `specify plan validate` gains four new checks (see §*Validation*).
- `Plan::validate` signature changes (see §*Migration*).
- `workspace sync` greenfield bootstrapping calls the existing `specify init <schema> --schema-dir <dir>` interface (positional schema identifier + pre-resolved schema source directory). The schema source directory is resolved from the initiating repo's `.specify/.cache/`. No `specify init` CLI changes are needed.
- `specify merge` gains workspace-clone auto-commit (see §*Merge commit contract*).
- `specify initiative workspace push` is a new verb (see §*Workspace push*). `WorkspaceAction` in `src/main.rs` gains a `Push` variant with an optional `projects: Vec<String>` argument; the match arm in `run_initiative` dispatches to a new `run_initiative_workspace_push` handler in `src/workspace.rs`.
- `specify initiative workspace sync` gains greenfield bootstrapping (see §*Greenfield bootstrapping*).

### Forward-reference updates

- Update `AGENTS.md` (line 21) and `plugins/spec/skills/plan/SKILL.md` (line 17) from `rfc-3b-layer-3.md` to `rfc-3b-platform.md`. The file was renamed from its working title; these references now point at a non-existent path.
- `rfcs/archive/rfc-3a-monoliths.md` also references `rfc-3b-layer-3.md` in several places (lines 17, 69, 508, 604). These are left as-is — archived RFCs are frozen. The stale link is cosmetic; the archived file's content is not consulted at implementation time.
- Update the existing `workspace.md` fixture at `plugins/spec/skills/plan/fixtures/plan-layer2/workspace.md` to include the `Description` and `Schema` bullets defined in §*Plan skill*. Update the plan SKILL.md §`workspace.md` shape pin (lines 184–201) to match the new shape. These updates land with the plan skill amendments.
- Update the plan SKILL.md's "state the skill mutates" section to include `.specify/plans/<initiative-name>/workspace.md` authored by step 3(a½) when the registry is multi-project. The new item reads: `4. .specify/plans/<initiative-name>/workspace.md written by step 3(a½) when the registry declares more than one project.`
- Replace the `§Peer registry sources (Layer 2)` paragraph in `schemas/omnia/briefs/plan/propose.md` (line 37: `"actually pointing a plan entry at a peer checkout path belongs to **RFC-3b** (federation)"`) with the assignment-contract wiring text defined in §*Schema propose briefs* → *Existing forward-reference removal*. This is a behavioural change, not just a filename fix — it replaces the deferral with the active assignment contract.

### Schema propose briefs

Both Omnia and Vectis propose briefs must implement the assignment contract defined in §*Assignment algorithm*. The amendments below apply identically to both `schemas/omnia/briefs/plan/propose.md` and `schemas/vectis/briefs/plan/propose.md`.

**Prerequisite for Vectis.** The Vectis propose brief currently has no `workspace.md` input and no multi-repo awareness. Before implementing the RFC-3b assignment contract, bring Vectis to parity with Omnia's current Layer 2 handling. The concrete diff to `schemas/vectis/briefs/plan/propose.md`:

1. Add to the `## Input` section (after the `discovery.md` bullet): `- **\`.specify/plans/<name>/workspace.md\`** when present (multi-repo / Layer 2). Authored by \`/spec:plan\` step 3(a½) after \`specify initiative workspace sync\`. Summarises each peer under \`.specify/workspace/<project>/\` so propose can attach capabilities that land in a peer repo. When absent, assume single-repo mode.`
2. Add a new `### Peer registry sources (Layer 2)` subsection after `### Resulting draft order`, with content identical to Omnia's current version (word-for-word copy of lines 35–37 of `schemas/omnia/briefs/plan/propose.md`).
3. Verify the brief reads each `## <project>` section of `workspace.md` when present.

This parity step can land as a standalone change (implementation step 7) before the assignment algorithm is added to either brief.

**Input section.** Add `workspace.md` as a conditionally-required input (present when multi-repo). The brief reads each `## <project>` section's `Description` and `Schema` bullets to build the assignment-signal table. The brief does not read `registry.yaml` directly — `workspace.md` is the sole source.

**New section: Assignment (multi-repo).** Insert after the existing `## Decomposition` section and before the interactive loop. When `workspace.md` is present and contains more than one `## <project>` section, the brief runs the assignment pass on each candidate slice:

1. For each slice, score every project using the signal priority from §*Inference heuristics*: description-match (domain-term overlap between the capability's `summary` and the project's `Description` bullet), baseline-spec affinity (capability name or domain overlaps with specs listed in the project's `Specify tree` bullet), and schema compatibility (capability nature vs project's `Schema` bullet).
2. If one project scores clearly above the rest, assign it. Record a rationale phrase (e.g. "description overlap: ingestion, Kafka").
3. If scores are tied or all low-confidence, mark the assignment as `?` (unresolved). Record "ambiguous: matches both/all projects" as the rationale.

The assignment is presented inline with the slice in the interactive loop (see below). When `workspace.md` is absent or contains a single project, the assignment pass is skipped entirely — the brief's existing single-repo flow runs unmodified.

**Interactive loop amendments.** The presentation block for each slice gains a `Project` line showing the inferred assignment (or `?` for unresolved). The **edit** action gains `project` as an editable field — prompted as a pick-from-list with the inferred value as default: `Project [traffic]: `. Invalid input (a name not in `workspace.md`'s project list) re-prompts. For unresolved assignments, the operator must assign a project before `accept` is available.

**`specify plan create` invocation amendment.** The shell-out gains `--project <project>`:

```text
specify plan create <name> \
    --project <project> \
    --sources <source-key> \
    [--depends-on <dep> ...] \
    --description "<rich prose>"
```

**Output section amendment.** The `proposal.md` table gains `Project` and `Rationale` columns when multi-repo (see §*Proposal shape (multi-repo)* for the exact table shape). Single-repo proposals are unchanged.

**Existing forward-reference removal.** Remove the `§Peer registry sources (Layer 2)` paragraph that currently defers to RFC-3b ("actually pointing a plan entry at a peer checkout path belongs to RFC-3b (federation)") and replace it with: "When `workspace.md` is present, use the `Description` and `Schema` bullets from each project section alongside the capability's `summary` to infer a `project` assignment per the contract in RFC-3b §*Assignment algorithm*."

Both briefs' single-repo paths are unchanged: when `workspace.md` is absent or single-project, assignment is skipped entirely and the existing flow runs unmodified.

### Plan skill (`/spec:plan`)

The `workspace.md` shape authored during the sync-peers phase (step 3(a½)) is extended to include each project's `description` from `registry.yaml`. The plan skill's `SKILL.md` workspace.md shape pin (§`workspace.md` shape) must be updated to the following:

```markdown
# Workspace — <initiative-name>

## <registry-project-name>

- **Slot:** `.specify/workspace/<registry-project-name>/`
- **Description:** <registry description text from registry.yaml>
- **Schema:** `<schema identifier from registry.yaml>`
- **Materialisation:** `symlink` | `git-clone` | `missing` (mirror
  `specify initiative workspace status`).
- **Head:** `<40-char sha or —>` when the slot is a git work tree.
- **Dirty:** `yes` | `no` | `—`
- **Specify tree:** one bullet each if present: `plan.yaml`, active
  changes under `changes/`, baseline specs under `specs/`, cached
  schema under `.specify/.cache/` — paths relative to the peer slot.

<!-- one `##` section per registry project, alphabetically by name -->
```

The `Description` bullet is placed immediately after `Slot` and before the materialisation-state bullets so that the propose brief encounters the assignment-relevant signals first. The `Schema` bullet is new — it surfaces the registry entry's `schema` field alongside the description for schema-compatibility inference. Step 3(a½) reads `registry.yaml` (already loaded by the plan skill at startup for the multi-repo guard) to populate both bullets; the workspace walk provides the remaining fields. This keeps the propose brief's input contract at two files (`discovery.md` and `workspace.md`) with no direct registry read, keeps the registry as the single source of truth for descriptions, and makes `workspace.md` a self-contained context document for the propose brief.

Under `--extend`, step 3(a½) still rewrites `workspace.md` from the current on-disk cache plus registry metadata, so the `Description` and `Schema` bullets reflect the latest `registry.yaml` content even when the workspace clones themselves are not re-synced. If the operator has added a new project to `registry.yaml` between runs, the new project will appear in `workspace.md` with `Materialisation: missing` — the propose brief can describe it but the execute driver will halt when it encounters a change targeting a missing workspace slot. The operator must run `specify initiative workspace sync` to materialise the new project before execution.

Step 3(c) of the plan skill's core loop gains `--project` wiring. When the propose brief produces a project assignment for a slice, the plan skill's `specify plan create` shell-out includes `--project <project>`. The single-repo path (absent or single-project registry) omits `--project`, preserving backwards compatibility. The plan skill itself does not decide the project — it forwards whatever the propose brief emitted, same as it forwards `--sources` and `--depends-on` today.

The plan skill's `--dry-run` output shape gains `Project` and `Rationale` columns in the proposal-preview table when the registry is multi-project. Single-project dry-runs are unchanged.

### Execute skill (`/spec:execute`)

The execute skill gains the step-level amendments described in §*Execute skill amendments* and §*Self-heal under multi-repo*. Phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`) are unaffected.

### Test coverage (`specify-cli`)

Each CLI change carries unit or integration tests following the existing patterns in the crate:

- **Validation checks** (`project-not-in-registry`, `project-missing-multi-repo`, `description-missing-multi-repo`, `schema-mismatch-workspace`): unit tests in `crates/change/src/plan.rs` (for plan checks) and `crates/schema/src/registry.rs` (for registry checks), following the existing `Plan::validate` and `Registry::validate_shape` test patterns.
- **`--project` flag on create/amend**: integration tests verifying that `--project` is written to and round-trips through `plan.yaml`, and that invalid `--project` values (not in registry) are rejected at write time.
- **`plan next` response extension**: test that `project`, `description`, and `sources` appear in the JSON response when the entry has them, and are absent when the entry doesn't or when `reason` is non-null.
- **`workspace push`**: integration tests mocking `git` and `gh` commands, verifying per-project status classification, branch naming, and error handling. The `gh` pre-flight check should have a unit test. `--dry-run` mode should have its own test verifying no side-effects.
- **`extract_github_slug`**: unit tests covering all six URL forms in the §*Workspace push* extraction table, plus the `None` case for non-GitHub hosts.
- **Greenfield bootstrap**: integration test that exercises the `mkdir` → `git init` → `specify init` sequence for a missing remote, verifying the resulting directory structure and `.specify/project.yaml` content.
- **Greenfield partial re-run**: integration test that exercises the partial-bootstrap re-run path: `git init` succeeded, `specify init` failed, second `workspace sync` re-runs `specify init` and produces a healthy slot.
- **`specify merge` auto-commit**: test that the workspace-clone heuristic correctly identifies workspace clones (CWD under `.specify/workspace/*/` with `project.yaml`) vs the initiating repo (including the edge case where `plan.yaml` has been archived), and that git staging is scoped to `.specify/` subtrees.

## Non-goals

- **Automated PR merging.** `workspace push` creates PRs; merging is a manual operator action. Automated merge is a future extension.
- **Cross-repo spec references.** `@peer:capability` syntax in spec bodies, contract reconciliation across the workspace, and peer status roll-up are execution-time federation concerns deferred until real multi-repo initiatives surface a concrete need.
- **Cross-cutting code generation.** Capabilities that span multiple repos are decomposed into per-project changes. Shared concerns (API contracts, auth libraries, protocol definitions) are handled by the platform's own dependency management, not by Specify's code-generation pipeline.
- **Multi-plan output.** RFC-3a's single `plan.yaml` in the initiating repo is preserved. RFC-3b adds routing metadata to change entries; it does not produce per-repo plans.
- **Inferring project descriptions.** The `description` on registry projects is always operator-authored. The framework does not attempt to generate descriptions from baseline specs or code analysis.
- **Re-authoring planning-time behaviour.** Discovery dispatch, the sync-peers phase, and the capability inventory are unchanged from RFC-3a. RFC-3b extends only the propose brief (assignment) and execution (routing).
- **Non-GitHub forges.** `workspace push` uses `gh` for remote repo creation and PR management. GitLab, Bitbucket, and self-hosted forges are not supported. The `gh`-dependent code paths (repo creation, PR creation) are isolated behind the pre-flight check so that plain `git push` works for any forge; only the repo-creation and PR-creation steps are GitHub-specific. Supporting additional forges is a future extension.

## Fixtures

Every behavioural pin for RFC-3b is listed here. Fixture files are authored during implementation; this table defines the required set. Fixture parent directories are skill-scoped:

- **Execute skill fixtures** live under `plugins/spec/skills/execute/fixtures/`.
- **Plan skill fixtures** live under `plugins/spec/skills/plan/fixtures/`.

| Fixture | Parent | Pins |
|---|---|---|
| `fixtures/multi-project/registry.yaml` | execute | Multi-project registry with `description` fields on every project. |
| `fixtures/multi-project/plan.yaml` | execute | Plan with `project` on every change entry, including cross-project `depends-on` edges. |
| `fixtures/multi-project/proposal.md` | plan | Proposal table with `Project` and `Rationale` columns, including one operator override and one unresolved → resolved case. |
| `fixtures/multi-project/execute-loop-transcript.md` | execute | `/spec:execute --loop` transcript showing CWD switches (with `Routing:` diagnostic lines) across two projects, with one success and one failure. |
| `fixtures/multi-project/workspace-push-output.json` | execute | Machine-readable output from `workspace push` showing branch + PR per project, including one greenfield `"created"` status. |
| `fixtures/multi-project/workspace-push-dry-run.json` | execute | Machine-readable output from `workspace push --dry-run` showing would-push / would-create statuses with `"dry_run": true`. |
| `fixtures/greenfield-bootstrap/` | execute | `workspace sync` output for a greenfield project: directory creation, git init, specify init sequence. |
| `fixtures/greenfield-bootstrap/partial-rerun/` | execute | `workspace sync` re-run after partial bootstrap (`.git/` present, `.specify/project.yaml` absent): re-runs `specify init`, verifies healthy slot. |

## Migration

Both new fields are additive — no schema version bump is required.

### Registry (`RegistryProject`)

- Add `description: Option<String>` with `#[serde(default)]` to the Rust struct. Existing single-project registries deserialise without change.
- `deny_unknown_fields` stays on the struct — adding the field to the struct definition is sufficient; serde will now accept it.
- Add a validation check in `Registry::validate_shape()`: when `projects.len() > 1`, every project must have `description.is_some()` with non-empty content. Error code: `description-missing-multi-repo`.

### Plan (`PlanChange`)

- Add `project: Option<String>` with `#[serde(default)]` to `PlanChange`. Existing plans deserialise without change.
- Update `plan.schema.json`: add `"project": { "type": "string" }` to `#/$defs/planChange/properties`.
- Add `project: Option<Option<String>>` to `PlanChangePatch` (same three-way semantics as `description`) so that `specify plan amend <name> --project <project>` is available.
- **Signature change:** Replace the unused `_project_dir` parameter on `Plan::validate()` with `registry: Option<&Registry>`. The caller (`specify plan validate` in `main.rs`) loads the registry and passes it. When `None` (no `registry.yaml`), the two project-related checks are skipped.
- Add two validation checks in `Plan::validate()`:
  - `project-not-in-registry`: every non-None `project` value must match a `projects[].name` in the registry. Requires `registry.is_some()`.
  - `project-missing-multi-repo`: when `registry` is present and `registry.projects.len() > 1`, every change must have `project.is_some()`.

### `specify plan next` (`PlanNextResponse`)

- Add `project: Option<String>`, `description: Option<String>`, and `sources: Option<Vec<String>>` to the JSON response from `specify plan next --format json`. Fields are present only when `next` is non-null (an eligible entry was found); absent when `reason` is non-null (`"all-done"`, `"stuck"`, `"in-progress"`).

### Ordering dependency

`RegistryProject` must gain the `description` field **before** any multi-project `registry.yaml` with `description` keys can parse, because `#[serde(deny_unknown_fields)]` on the struct rejects unknown keys. Implement the registry struct extension first, then the plan struct extension, then the CLI flag additions, then the validation cross-checks.

## Implementation order

The following sequence minimises integration risk. Each step is independently shippable and testable.

1. **CLI struct extensions** (specify-cli). `RegistryProject` gains `description`; `PlanChange` gains `project`; `PlanChangePatch` gains `project`; `plan.schema.json` updated. Validation checks added. All additive, backwards-compatible.
2. **CLI verb extensions** (specify-cli). `specify plan create` and `amend` gain `--project`. `specify plan next --format json` gains `project`, `description`, `sources`. Depends on step 1.
3. **`specify merge` auto-commit** (specify-cli). Workspace-clone detection heuristic + git staging + commit logic. Independent of steps 1–2; can ship in parallel.
4. **`workspace sync` greenfield bootstrap** (specify-cli). Extends `sync_registry_workspace` with the greenfield fallback. Depends on step 1 (needs `description` field to parse).
5. **`workspace push`** (specify-cli). New verb. Depends on steps 1 and 3 (merge commits exist to push).
6. **Plan skill + workspace.md shape** (specify repo). Update SKILL.md §workspace.md shape pin, update fixture, update state-mutation list. Depends on step 1 (descriptions exist in registry).
7. **Vectis propose brief parity** (specify repo). Add `workspace.md` input to Vectis brief. No dependency on RFC-3b CLI work.
8. **Propose brief assignment contract** (specify repo). Both Omnia and Vectis. Depends on steps 6 and 7.
9. **Execute skill amendments** (specify repo). CWD routing + CWD restore steps, self-heal multi-repo, `--dry-run` routing line. Depends on step 2 (`plan next` response has `project`).
10. **Forward-reference fixes + fixture authoring**. Can land at any point.

## Relation to RFC-3a

- The registry gains one optional field (`description` on `RegistryProject`). The field is optional for v1 single-repo registries; required for multi-project registries. See §*Migration* for serde and validation details.
- The plan schema gains one optional field (`project` on `planChange`). The field is optional for single-repo plans; required for multi-project plans. See §*Migration*.
- `specify plan create` gains `--project`. `PlanChangePatch` gains a corresponding `project` field for `amend`. Both validate `--project` against the loaded registry at write time. The single-writer invariant is preserved — the propose brief passes `--project` to `specify plan create`; no direct plan edits.
- `specify plan next --format json` gains `project`, `description`, and `sources` in its response so the execute driver can route without a second round-trip. See §*`specify plan next` extension*.
- The propose brief's contract expands: it must produce a `project` assignment per slice, record the rationale in `proposal.md`, and surface unresolved assignments for operator input. Both Omnia and Vectis propose briefs are in scope for update. See §*Assignment algorithm* and §*Implementation scope*.
- The plan skill's step 3(c) gains `--project` wiring on its `specify plan create` shell-out. See §*Implementation scope*.
- `/spec:execute` uses **CWD-based routing**: the driver `chdir`s into the target project's workspace clone before invoking phase skills (new CWD routing and CWD restore steps). Phase skills are unaware of multi-repo routing. Source paths are resolved to absolute paths before the CWD change. Self-heal gains per-entry CWD routing for multi-repo entries. See §*Execute skill amendments* and §*Self-heal under multi-repo*.
- The workspace clone's write policy relaxes from read-only (RFC-3a planning convention) to writable during execution. This is a documented policy change, not a code enforcement change. See §*Workspace writability policy*.
- On merge inside a workspace clone, `specify merge` auto-commits the folded baseline. The commit lands on the clone's current HEAD; branch management is deferred to `workspace push`. See §*Merge commit contract*.
- `workspace sync` gains greenfield bootstrapping with per-project error handling: when a clone fails (repo not found), it creates the local directory, runs `git init` + `specify init`, and sets the remote. Partial bootstrap failures are left on disk for operator triage. See §*Greenfield bootstrapping*.
- `workspace push` is a new CLI verb with a `gh auth status` pre-flight check. It creates a `specify/<initiative-name>` branch per dirty clone, pushes to the remote (creating the GitHub repo via `gh` if needed for greenfield), and opens a PR. See §*Workspace push*.
- `Plan::validate` gains an `Option<&Registry>` parameter (replacing the unused `_project_dir`), enabling plan-registry cross-validation. See §*Validation* and §*Migration*.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md)
- [RFC-2: Execution](archive/rfc-2-execution.md)
- [RFC-3a: Initiative Planning](archive/rfc-3a-monoliths.md)

