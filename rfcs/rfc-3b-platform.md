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
    url: ../mobile
    schema: vectis@v1
    description: >
      iOS and Android mobile application. Owns all client-side
      UI, navigation, and offline-first behaviour.
```

The descriptions serve the same role as in brownfield — they are the signal the propose brief matches against — but since no baseline specs exist, the description carries the full weight of the routing decision.

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

- `project` is a kebab-case string that must match a `projects[].name` in `registry.yaml`. Validated by `specify initiative validate`.
- For single-project registries (or absent registry), `project` is optional. Absence means "the current repo" — the pre-RFC-3b default, fully backwards compatible.
- For multi-project registries (`len(projects) > 1`), `project` is required on every change entry. `specify initiative validate` rejects entries without it.
- `project` determines which schema governs the change's define-build-merge cycle: `registry.yaml[project].schema` resolves to the schema whose briefs `/spec:execute` will invoke.

### `specify initiative create` extension

`specify initiative create` gains an optional `--project` flag:

```text
specify initiative create <name> \
    [--project <registry-project-name>] \
    [--sources <key> ...] \
    [--depends-on <name> ...] \
    [--description "..."]
```

`--project` specifies which registry project a change targets — analogous to `--sources` specifying which sources it draws from. The propose brief passes `--project` for each accepted slice. For single-repo plans, omitting `--project` preserves today's behaviour.

`PlanChangePatch` gains a corresponding `project` field (`Option<Option<String>>` — same three-way semantics as `description`) so that `specify initiative amend <name> --project <project>` is available for post-creation corrections.

## Assignment algorithm

The assignment algorithm runs inside the **propose brief** (schema-owned, not framework-level). RFC-3b defines the contract the propose brief must satisfy; the algorithm itself is schema-specific.

### Inputs

The propose brief receives:

1. `**discovery.md`** — the capability inventory from step 3(a). Each capability carries `name`, `summary`, `sources`, `depends-on`, and `confidence`.
2. `**workspace.md`** — the peer inventory from step 3(a½) (multi-repo only). Each peer's entry includes its baseline specs, schema, and materialisation state.
3. `**registry.yaml**` — the full registry, including each project's `description`.

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

The interactive loop's **edit** action gains `project` as an editable field alongside name, sources, depends-on, and description.

For **unresolved** assignments (ambiguity → human rule), the `Project` column shows `?` and the prompt requires the operator to assign a project before accept is available.

For each accepted slice, the propose brief shells out to:

```text
specify initiative create <name> \
    --project <project> \
    --sources <key> \
    [--depends-on <dep> ...] \
    --description "<rich prose>"
```

### Single-repo plans

When the registry is absent or single-project, the propose brief skips assignment entirely. The `Project` and `Rationale` columns are omitted from `proposal.md`. No `--project` flag is passed to `specify initiative create`. This is the backwards-compatible path — pre-RFC-3b plans are valid without modification.

## Execution routing

`/spec:execute` already reads `plan.yaml` entries and drives define-build-merge per change. RFC-3b extends the driver to route each change to the correct project using **CWD-based routing**: the driver changes working directory to the target project's root before invoking phase skills. Phase skills (define, build, merge) are unaware of multi-repo routing — they run unmodified in whatever working directory the driver places them in.

### Working directory

Each change's define-build-merge cycle runs against the **project's clone in the workspace** — not the initiating repo's root:

- For `url: .` or relative-path projects, the resolved filesystem path is the working directory.
- For remote projects, `.specify/workspace/<name>/` (the clone materialised by RFC-3a sync-peers) is the working directory.

The driver resolves the target project's filesystem root from `registry.yaml`, sets the working directory to that root, and invokes `/spec:define <name>` with no additional project flag. Define discovers `.specify/project.yaml` via its normal CWD walk and resolves the schema from that file. Build and merge follow the same pattern. Phase skills need no changes — the driver owns the routing decision entirely.

In `--loop` mode, the driver saves and restores CWD around each change's define-build-merge cycle so that the next iteration's `specify initiative next` (which reads the initiating repo's `plan.yaml`) runs from the initiating repo root.

The workspace clone is **writable during execution**. Spec artifacts (design.md, specs/, tasks/) are written into the project clone's `.specify/changes/<name>/` subtree during define and build; on merge, the delta specs are folded into the clone's `.specify/specs/` baseline and committed locally. The commit remains in the workspace clone until the operator explicitly pushes via `specify initiative workspace push` (see §*Workspace push*).

### Schema resolution

Schema resolution happens via the target project clone's `.specify/project.yaml`, not via the driver looking up `registry.yaml[project].schema`. When the driver `chdir`s into a workspace clone:

1. Define reads `.specify/project.yaml` in CWD → `{ schema: "omnia@v1", ... }`.
2. `specify schema pipeline define` resolves the schema directory from that identifier.
3. The schema's `pipeline.define`, `pipeline.build`, and `pipeline.merge` briefs govern the change's lifecycle.

The registry's `schema` field is used at validation time (`specify initiative validate` can warn if a clone's `project.yaml` schema disagrees with `registry.yaml`) but is not the resolution path during execution.

For changes without a `project` field (single-repo plans), resolution is unchanged from RFC-2.

### Workspace freshness

Before executing a change targeting a peer project, `/spec:execute` checks the workspace slot's materialisation state via `specify initiative workspace status`. If the slot is `missing`, execution halts with a diagnostic pointing the operator at `specify initiative workspace sync`. Stale-but-present slots are accepted — the operator controls freshness via explicit `specify initiative workspace sync` calls between execution runs, matching the RFC-3a `--extend` contract.

### Source path resolution

Source paths in the plan's top-level `sources` map are interpreted relative to the **initiating repo root** (where `plan.yaml` lives). Before changing working directory to the target project's clone, the driver resolves each source value to an **absolute filesystem path**. Git URLs are passed through unchanged.

The resolved absolute path is what gets passed to `/spec:define --source <key>=<absolute-path>`. This ensures source paths remain valid regardless of which project clone the driver has `chdir`'d into.

All existing argument-resolution rows (`sources`, `depends-on`, `description`) are unchanged in shape. The `project` field is consumed by the driver for CWD routing and is **not** forwarded as a flag to any phase skill.

### Sources map scoping

The plan's top-level `sources` map is initiative-scoped and shared across all changes regardless of target project. Each change's `sources` list references only the keys relevant to its scope. Per-project source scoping is not needed — the map is a flat namespace keyed by operator-chosen identifiers.

## Validation

`specify initiative validate` gains three new checks:

- **Project references registry** (`project-not-in-registry`). Every `project` value on a change must match a `projects[].name` in `registry.yaml`. *Error.*
- **Project required for multi-repo** (`project-missing-multi-repo`). When `len(projects) > 1`, every change must carry a `project` field. *Error.*
- **Description required for multi-repo** (`description-missing-multi-repo`). When `len(projects) > 1`, every registry project must carry a `description`. *Error.*

## Design decisions

### Workspace-centric execution

All multi-repo work is undertaken in a single workspace rooted in the registry repo. The operator creates an initiative from the registry repo, which triggers all registry projects to be cloned into `.specify/workspace/<project>/` via `specify initiative workspace sync`. Each change's define-build-merge cycle runs against the relevant project clone inside this workspace. On merge, the resulting specs and code artifacts are committed to the clone. Changes are pushed to remotes explicitly via `specify initiative workspace push` — the operator controls when pushes happen; `/spec:execute` never pushes automatically.

### Greenfield bootstrapping

For greenfield projects whose remote repos do not yet exist, `workspace sync` and `workspace push` collaborate to handle the full lifecycle:

**`workspace sync` (local bootstrapping).** For each registry project whose workspace slot is `missing`:

1. Attempt a shallow clone from the registry entry's `url`. If the clone succeeds (brownfield), the slot is materialised as today.
2. If the clone fails (404, repo not found) or the `url` is a relative path to a non-existent directory, treat the project as greenfield:
   - Create the workspace slot directory (`mkdir -p .specify/workspace/<name>/`).
   - `git init` inside the slot.
   - `git remote add origin <url>` (from `registry.yaml`).
   - Run `specify init` with the registry entry's schema to scaffold `.specify/project.yaml` and the schema cache.
   - `git add . && git commit -m "Initial Specify scaffold"`.

After `workspace sync`, every workspace slot is a valid Specify project with `.specify/project.yaml` — whether brownfield or greenfield. Phase skills discover the project via their normal CWD walk and need no special greenfield handling.

### Workspace push

```text
specify initiative workspace push [<project>...]
```

Pushes workspace clones that have local commits back to their remote repositories. Omitting the project argument pushes all dirty clones.

**Per-project algorithm:**

1. **Branch.** Create or update `specify/<initiative-name>` from the clone's current HEAD. If the branch already exists on the remote (from a prior push), force-update it.
2. **Remote repo creation (greenfield).** If the remote repository does not exist, create it via `gh repo create <org/name> --private --source .`. This keeps GitHub API interaction at the write boundary — `workspace sync` is local-only.
3. **Push.** `git push -u origin specify/<initiative-name>`.
4. **PR.** If no open PR exists for this branch, create one via `gh pr create --title "specify: <initiative-name>" --body "<auto-generated summary>"`. If a PR already exists, the branch push updates it; the PR body is left unchanged.
5. **Report.** Per-project status line.

**Error handling.** Per-project errors (auth failure, push rejection, `gh` not installed) are reported and the verb continues to the next project. Non-zero exit if any project failed.

**Output format (machine-readable, `--format json`):**

```json
{
  "projects": [
    { "name": "traffic", "status": "pushed", "branch": "specify/platform-v2", "pr": 42 },
    { "name": "command-centre", "status": "nothing-to-push" },
    { "name": "mobile", "status": "created", "branch": "specify/platform-v2", "pr": 7 }
  ]
}
```

The `"created"` status indicates the remote repo was created as part of this push (greenfield). `"pushed"` indicates an existing remote was updated. `"nothing-to-push"` indicates the clone has no local commits ahead of the remote.

PRs are merged manually by the operator. A future extension may add `specify initiative workspace merge` for automated PR merging, but this is out of scope for RFC-3b.

### Cross-repo dependency ordering

`/spec:plan` produces an ordered set of changes; `/spec:execute` processes them in that order, respecting `depends-on` edges as it does today. Cross-repo `depends-on` edges (a change in `traffic` depending on a change in `command-centre`) are legal and enforced by the same ordering logic — there is no special cross-project case. That said, `depends-on` edges across project boundaries are expected to be uncommon in practice. Schemas should prefer decomposing capabilities so that each project's changes are internally ordered, with cross-project coordination handled at the initiative level rather than through fine-grained inter-project dependencies.

### One change, one project

Each plan change targets exactly one project. Capabilities that span multiple repos (e.g. "add OAuth login" touching both a backend and a frontend) are decomposed into separate plan entries — one per project — linked by `depends-on` edges where ordering matters. Cross-cutting concerns (shared auth libraries, API contracts, protocol definitions) are a framework-level concern addressed by the platform's dependency management and build tooling, not by the Specify code-generation pipeline. The one-change-one-project model keeps the execution loop simple: each define-build-merge cycle has a single project root, a single schema, and a single set of baseline specs.

### Peer-to-peer spec references (deferred)

A spec in `traffic` referencing a capability in `command-centre` (`@peer:capability` syntax) is an execution-time federation concern that sits downstream of routing. It is deferred until real multi-repo initiatives surface a concrete need. The workspace already materialises peer baselines under `.specify/workspace/<peer>/specs/`, so the read path exists; the reference syntax and resolution rules can be designed against real examples when the need arises.

## Non-goals

- **Automated PR merging.** `workspace push` creates PRs; merging is a manual operator action. Automated merge is a future extension.
- **Cross-repo spec references.** `@peer:capability` syntax in spec bodies, contract reconciliation across the workspace, and peer status roll-up are execution-time federation concerns deferred until real multi-repo initiatives surface a concrete need.
- **Cross-cutting code generation.** Capabilities that span multiple repos are decomposed into per-project changes. Shared concerns (API contracts, auth libraries, protocol definitions) are handled by the platform's own dependency management, not by Specify's code-generation pipeline.
- **Multi-plan output.** RFC-3a's single `plan.yaml` in the initiating repo is preserved. RFC-3b adds routing metadata to change entries; it does not produce per-repo plans.
- **Inferring project descriptions.** The `description` on registry projects is always operator-authored. The framework does not attempt to generate descriptions from baseline specs or code analysis.
- **Re-authoring planning-time behaviour.** Discovery dispatch, the sync-peers phase, and the capability inventory are unchanged from RFC-3a. RFC-3b extends only the propose brief (assignment) and execution (routing).

## Fixtures

Every behavioural pin for RFC-3b is listed here. Fixture files are authored during implementation; this table defines the required set.

| Fixture | Pins |
|---|---|
| `fixtures/multi-project/registry.yaml` | Multi-project registry with `description` fields on every project. |
| `fixtures/multi-project/plan.yaml` | Plan with `project` on every change entry, including cross-project `depends-on` edges. |
| `fixtures/multi-project/proposal.md` | Proposal table with `Project` and `Rationale` columns, including one operator override and one unresolved → resolved case. |
| `fixtures/multi-project/execute-loop-transcript.md` | `/spec:execute --loop` transcript showing CWD switches across two projects, with one success and one failure. |
| `fixtures/multi-project/workspace-push-output.json` | Machine-readable output from `workspace push` showing branch + PR per project, including one greenfield `"created"` status. |
| `fixtures/greenfield-bootstrap/` | `workspace sync` output for a greenfield project: directory creation, git init, specify init sequence. |

## Migration

Both new fields are additive — no schema version bump is required.

### Registry (`RegistryProject`)

- Add `description: Option<String>` with `#[serde(default)]` to the Rust struct. Existing single-project registries deserialise without change.
- `deny_unknown_fields` stays on the struct — adding the field to the struct definition is sufficient; serde will now accept it.
- Add a validation check in `Registry::validate_shape()`: when `projects.len() > 1`, every project must have `description.is_some()` with non-empty content. Error code: `description-missing-multi-repo`.

### Plan (`PlanChange`)

- Add `project: Option<String>` with `#[serde(default)]` to `PlanChange`. Existing plans deserialise without change.
- Update `plan.schema.json`: add `"project": { "type": "string" }` to `#/$defs/planChange/properties`.
- Add `project: Option<Option<String>>` to `PlanChangePatch` (same three-way semantics as `description`) so that `specify initiative amend <name> --project <project>` is available.
- Add two validation checks in `Plan::validate()`:
  - `project-not-in-registry`: every non-None `project` value must match a `projects[].name` in the registry. `Plan::validate` gains a `&Registry` parameter (the existing `_project_dir` parameter is replaced or supplemented).
  - `project-missing-multi-repo`: when `registry.projects.len() > 1`, every change must have `project.is_some()`.

## Relation to RFC-3a

- The registry gains one optional field (`description` on `RegistryProject`). The field is optional for v1 single-repo registries; required for multi-project registries. See §*Migration* for serde and validation details.
- The plan schema gains one optional field (`project` on `planChange`). The field is optional for single-repo plans; required for multi-project plans. See §*Migration*.
- `specify initiative create` gains `--project`. `PlanChangePatch` gains a corresponding `project` field for `amend`. The single-writer invariant is preserved — the propose brief passes `--project` to `specify initiative create`; no direct plan edits.
- The propose brief's contract expands: it must produce a `project` assignment per slice, record the rationale in `proposal.md`, and surface unresolved assignments for operator input. See §*Proposal shape (multi-repo)*.
- `/spec:execute` uses **CWD-based routing**: the driver `chdir`s into the target project's workspace clone before invoking phase skills. Phase skills are unaware of multi-repo routing. Source paths are resolved to absolute paths before the CWD change. See §*Execution routing*.
- The workspace clone's write policy relaxes from fully read-only (RFC-3a planning) to writable during execution. Define and build write into `.specify/changes/<name>/`; merge commits the folded baseline into the clone.
- `workspace sync` gains greenfield bootstrapping: when a clone fails (repo not found), it creates the local directory, runs `git init` + `specify init`, and sets the remote. See §*Greenfield bootstrapping*.
- `workspace push` is a new CLI verb. It creates a `specify/<initiative-name>` branch per dirty clone, pushes to the remote (creating the GitHub repo via `gh` if needed for greenfield), and opens a PR. See §*Workspace push*.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md)
- [RFC-2: Execution](archive/rfc-2-execution.md)
- [RFC-3a: Initiative Planning](archive/rfc-3a-monoliths.md)

