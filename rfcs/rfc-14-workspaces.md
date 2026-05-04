# RFC-14: Workspaces — multiple domain schemas per repository

> Status: Draft · Depends: [RFC-13](rfc-13-extensibility.md), [RFC-1](archive/rfc-1-cli.md), [RFC-9](archive/rfc-9-platform.md)

## Abstract

[RFC-13](rfc-13-extensibility.md) reframes the runtime so that **schemas are the only extension point**, and lands one domain schema (`omnia` / `contracts` / `vectis` / …) plus three first-party platform schemas (`plan@v1`, `registry@v1`, `initiative@v1`) per project. RFC-14 is the follow-up that lets a single repository carry **more than one domain schema** — for example, a service repo that maintains both Rust+WASM code (`omnia@v1`) and the API contracts that surround it (`contracts@v1`), or a platform repo that hosts code, infrastructure, and design tokens side by side.

The shape borrowed from Cargo is that of `Cargo.toml`: one file format that collapses to a single package, a workspace of members, or both at once. RFC-14 maps the same model onto `.specify/project.yaml` — `package:` declares an optional root domain schema, `workspace:` declares an optional list of nested **member scopes**, and at least one of the two must be present. Every artifact, change, brief substitution, and capability skill action gains a **scope coordinate**; everything else from RFC-13's manifest protocol is preserved unchanged.

This is the test of whether RFC-13 got the abstraction right: workspaces should fit on top of the capability protocol without re-opening it. RFC-14 confirms that read by changing **path resolution** and **change scoping**, and nothing inside the protocol itself.

## Motivation

### The unstated singleton

RFC-13 is explicit that multiple schemas coexist (§"Cross-schema coexistence"), but the coexistence it describes is one **domain** schema plus the three **platform** schemas. The shape of `project.yaml` reflects that: `schema:` is a singular field, and §"What this enables" lists candidate schemas (`infra@v1`, `client-sdk@v1`, `standards@v1`, `design-tokens@v1`, `fixtures@v1`) that each replace the existing domain schema rather than coexisting with it.

The two open questions RFC-13 leaves on this point —

- §Open Questions #2 ("Multiple schemas per project") — provisional: "out of scope here; track as a candidate RFC once a real multi-concern project lands."
- §Open Questions #13 ("Single `schema:` vs `schemas:` in `project.yaml`") — provisional: "single `schema:` with auto-activation."

— land in the same place: defer until a real multi-domain repo shows up. Two such repos already exist or are about to:

1. **Code + contracts in the same repo.** The `contracts@v1` schema already supports dedicated contract changes that evolve interface shapes independently of implementation. A service repo today must choose between activating `omnia@v1` (and producing contracts as a side effect of code changes) or activating `contracts@v1` (and losing `/spec:build` for the Rust crates). Neither is right: contracts and code have different lifecycles and different reviewers.
2. **Platform repos with multiple concerns.** A platform repo that owns shared infrastructure (`infra@v1`), design tokens (`design-tokens@v1`), and a generated client SDK (`client-sdk@v1`) has three orthogonal artefact families, each governed by its own schema. Today every additional concern requires a new repo and a new registry entry; the operational overhead is real.

### What today's surface forces

- A repo with both code and contracts has to fold one into the other. If `omnia@v1` is active, contract changes ride on the back of implementation changes; if `contracts@v1` is active, the Rust crates have no `/spec:build`. Either way, half the lifecycle goes unmanaged.
- Splitting concerns across repos works, but pays the cost in registry edits, workspace clones, plan entries, and a planning-time correlation step that exists only because the file system can't carry more than one domain schema in one tree.
- The `--hub` mode covers the "no domain schema, only platform" case (registry-only platform hub), but there is no symmetric story for "one domain at the root plus more domains in subdirectories." The asymmetry hints at a missing layer.

### What this enables

With multi-domain workspaces:

- A **service repo** activates `omnia@v1` at the root and `contracts@v1` under `contracts/`. Contract changes use the contracts pipeline; implementation changes use the omnia pipeline; the two coexist without forcing one to be a side effect of the other.
- A **platform repo** activates no root schema and lists `{ infra@v1, design-tokens@v1, client-sdk@v1, codex@v1 }` as members under `infra/`, `tokens/`, `clients/`, `codex/`. Each member's lifecycle is its own.
- A **monorepo** activates `omnia@v1` per service under `services/<name>/` (one schema, multiple scopes) — the same pattern Cargo uses to manage many crates in a workspace.
- Cross-repo plans (RFC-3a, RFC-3b, RFC-9) gain finer-grained routing: a plan entry can target `<repo>/<scope>` rather than just `<repo>`, so a single plan can drive a contracts change and an omnia change in dependency order without reaching across repo boundaries.

The repo bar drops: every new concern is a member directory and one schema URL, not a new repository.

## Design

### Principle

**A repository is a workspace; a workspace contains zero or more scopes; each scope activates exactly one domain schema.** The workspace is the unit that owns platform schemas (`plan`, `initiative`, `registry`), the lock file, the change archive, and the cross-scope coordination surface. A scope is the unit that owns a domain schema, its artefacts, and its briefs. Path resolution and change creation become scope-aware; nothing else changes.

The Cargo analogy is load-bearing because Cargo solved the same problem: a single configuration file that scales smoothly from "one package" to "one workspace" to "one workspace with a root package," and a per-member configuration file for what's local to each member. The asymmetry RFC-14 borrows is that **workspace-level concerns live at the workspace root** and **scope-level concerns live in the scope's subtree** — the same split Cargo applies to `[workspace.dependencies]` (root) versus `[dependencies]` (per-member).

### `project.yaml` shape

The new shape collapses three configurations into one file format. At least one of `package:` or `workspace:` MUST be present:

```yaml
# .specify/project.yaml
name: my-app
specify_version: 1.5

# Optional root scope — the "[package]" of Cargo
package:
  schema: https://github.com/augentic/specify/schemas/omnia@v1
  domain:
    description: Customer orders service
  rules:
    proposal: rules/proposal.md

# Optional members — the "[workspace]" of Cargo
workspace:
  members:
    - path: contracts/
      schema: https://github.com/augentic/specify/schemas/contracts@v1
    - path: infra/
      schema: https://github.com/augentic/specify/schemas/infra@v1
  # OR via glob:
  # members: ["scopes/*"]

# Workspace-wide platform-schema config — always at the root
extensions:
  plan:     { … }
  registry: { … }
  initiative: { … }
```

Three legal modes follow:

| Mode                  | `package:` | `workspace:` | Today's equivalent                       |
| --------------------- | ---------- | ------------ | ---------------------------------------- |
| Package only          | yes        | absent       | `schema: <url>` (the common case today)  |
| Workspace only        | absent     | yes          | `--hub` (registry-only platform hub)     |
| Package + workspace   | yes        | yes          | **New** — root domain plus member scopes |

Mode A is the dominant case today and is preserved verbatim under the new shape — operators are not forced to opt in to anything. Mode B subsumes today's `hub: true` flag (see §Migration). Mode C is the new capability.

### `scope.yaml` for member-local config

A workspace member MAY carry its own `<member-path>/.specify/scope.yaml` for member-local rules, brief overrides, and `extensions.<schema>` settings:

```yaml
# contracts/.specify/scope.yaml
rules:
  specs: rules/contracts-specs.md
extensions:
  contracts:
    format-policy: strict-semver
```

`scope.yaml` is optional and member-local; when absent, the member inherits the workspace-root `extensions.<schema>` block. When present, the member values override the workspace values for that scope only — `allOf: [workspace-block, scope-block]` semantics, so members can tighten but not silently broaden config.

The member's `schema:` URL is **always declared in the workspace root** `project.yaml`, never in `scope.yaml`. Pinning the schema URL at the workspace root keeps the workspace's active-schema set determinable from a single file (matches Cargo's principle that `[workspace.dependencies]` lives at the workspace root, not the member).

### Scope as a first-class lifecycle dimension

Every artefact, change, brief substitution, and capability skill action is associated with exactly one **scope**. The scope is one of:

- **Root scope** (`""`) — owns artefacts whose paths sit at the project root. Active when `package:` is declared.
- **Member scope** (`<path>`) — owns artefacts under `<workspace-root>/<path>/`. One per `workspace.members[]` entry.

The workspace root itself is **not a scope** in the path-resolution sense — it is the container that holds platform schemas, the lock file, the change archive, and the registry. Platform schemas (`plan@v1`, `registry@v1`, `initiative@v1`) activate **once** at the workspace root and are visible from every scope.

### Path resolution becomes scope-relative

RFC-13 §Artifacts (declarative lifecycle) introduces `$ARTIFACT_DELTA[<id>]` and `$ARTIFACT_BASELINE[<id>]` substitutions, deliberately abstract over literal paths. RFC-14 reuses that abstraction and changes only the resolver:

| Substitution                  | Pre-RFC-14 (RFC-13)                            | Post-RFC-14                                                   |
| ----------------------------- | ---------------------------------------------- | ------------------------------------------------------------- |
| `$ARTIFACT_DELTA[<id>]`       | `$PROJECT_DIR/.specify/changes/<name>/<delta>` | `$PROJECT_DIR/<scope>/.specify/changes/<name>/<delta>`        |
| `$ARTIFACT_BASELINE[<id>]`    | `$PROJECT_DIR/<baseline>`                      | `$PROJECT_DIR/<scope>/<baseline>`                             |
| `$PROJECT_DIR`                | repository root                                | repository root (unchanged — workspace root)                  |
| `$SCOPE_DIR` *(new)*          | (n/a)                                          | `$PROJECT_DIR/<scope>` (`""` for root scope)                  |

For the root scope, `<scope>` is the empty path, so resolution collapses to today's behaviour — Mode A repos see no change. Briefs that already use `$ARTIFACT_*` substitutions need no edits to work in a workspace. Briefs that need to reach **outside** their own scope (e.g., a `client-sdk@v1` change that consumes the `contracts@v1` baseline as read-only context) use `$ARTIFACT_BASELINE[<id>]@<consumed-scope>` — see §Cross-scope reads below.

This change is also the canonical extension of RFC-13's Phase 1 work: `ProjectConfig::baseline_path(&schema, artifact_id)` becomes `baseline_path(&scope, &schema, artifact_id)`. The `Scope` newtype is the only structural type the change loop gains.

### Cross-schema coexistence rules become scope-aware

RFC-13 §Cross-schema coexistence states two project-wide invariants:

> - **Artefact id uniqueness.** No two active schemas may declare the same `artifact.id`.
> - **Baseline-path uniqueness.** No two active schemas may claim the same baseline path or project-path.

Under RFC-14 these become:

| Invariant                            | Scope                                                                                              |
| ------------------------------------ | -------------------------------------------------------------------------------------------------- |
| Artefact id uniqueness               | **Within a scope** (so two scopes can each declare a `specs` artefact without collision).         |
| Baseline-path / project-path uniqueness | **Workspace-global** (resolved absolute paths must not overlap; the scope prefix usually guarantees this). |
| Member-path overlap                  | **New** — `workspace.members[*].path` entries must not be ancestors or descendants of each other. |
| Platform-schema artefact ids         | **Workspace-global** (platform schemas live at the workspace root, so their ids are unique by construction). |

The first two are familiar in spirit; the new lint is member-path overlap. A workspace declaring members at `infra/` and `infra/aws/` is rejected at `specify check` time — overlapping members would fight over which scope owns a given path.

### Cross-scope reads via `consumes:`

RFC-13 §Cross-schema coexistence introduces a `consumes:` array for read-only schema-to-schema dependencies (e.g., `client-sdk@v1` reads the `contracts@v1` baseline). In a workspace, `consumes:` resolves through the workspace's active scope set:

```yaml
# schemas/client-sdk/schema.yaml (illustrative)
consumes:
  - contracts
```

When `client-sdk@v1` is active in scope `clients/` and `contracts@v1` is active in scope `contracts/`, the consumer gets read-only access to `<workspace-root>/contracts/contracts/` (the contracts baseline) via `$ARTIFACT_BASELINE[contracts]@contracts/`. The `@<scope>` qualifier is required when more than one scope provides the consumed schema; it is optional and inferred when there is exactly one provider. This keeps the common case (one provider) ergonomic and makes the disambiguating case explicit.

Workspace-wide platform schemas (`plan`, `registry`, `initiative`) are **always reachable** from any scope without a `@<scope>` qualifier — they live at the workspace root, not in any scope.

### Changes get a scope coordinate

A change is created within exactly one scope. Three CLI shapes are equivalent:

```bash
# Implicit from cwd (Cargo-flavored)
cd contracts/
specify change create add-orders-api

# Explicit flag from the workspace root
specify change create add-orders-api --scope contracts/

# Workspace-root scope (omitted scope flag, no cwd inference needed)
specify change create my-feature                 # scope defaults to root if package: is declared
```

The cwd-inference rule is exactly Cargo's: walk up from the current directory until a `.specify/project.yaml` (or `.specify/scope.yaml`) is found, and bind the change to that scope. From the workspace root in a workspace-only repo (no `package:`), `--scope <path>` is mandatory because there is no root scope to default to.

`.metadata.yaml` records `scope: <path>` (`""` for root scope). The change's `.specify/changes/<name>/` directory lives at `<workspace-root>/<scope>/.specify/changes/<name>/`, so a change is physically rooted in the scope it belongs to. This matches the Cargo intuition that a member is self-contained: deleting `contracts/` deletes its change history with it, and the workspace root never accumulates per-scope artefacts.

#### Cross-scope changes are forbidden

A change writes to exactly one scope's baselines. Coordinated multi-scope work routes through `plan@v1` in the standard way: a plan entry is created per scope, the entries are ordered with the usual `needs:` dependency edges, and `/spec:execute` walks the plan one entry at a time.

The alternative — letting a single change carry deltas for multiple scopes' baselines — was considered and rejected for three reasons:

1. **It breaks the "core never switches on a schema name" principle.** A multi-scope change has multiple pipelines to run, each from a different schema, with no single owning schema. The change loop becomes schema-aware again, undoing RFC-13's reframe.
2. **It re-introduces the very coupling RFC-14 is trying to break.** The whole point of multi-domain repos is independent lifecycles — making a change span both lifecycles puts the coupling back at change granularity.
3. **Plans already solve this.** RFC-2's plan loop is the existing mechanism for ordering multiple changes with dependencies. A multi-scope plan is one plan with two entries; the only additional cost is one extra journal entry per change, which is the right cost for the audit story to remain coherent.

Repos that need an atomic update across two scopes (e.g., contract + implementation in lock-step) accept the two-change overhead. The `plan@v1` schema is gaining a `needs:` field for this purpose anyway under RFC-2; the same field handles cross-scope dependencies inside a workspace and cross-repo dependencies across workspaces.

### Capability skills get scope context

Capability skill behavior is scoped by the change or workflow entry that invokes it. In a workspace where the same capability is active in two scopes (e.g., `infra@v1` in both `infra-aws/` and `infra-gcp/`), the resolved scope path must be part of the skill context so diagnostics and written paths can be attributed unambiguously.

When a capability is active in exactly one scope, the same cwd-inference rule used by `specify change create` resolves the scope automatically. Mode A repos see no change at the call site.

### Workspace-level concerns

Three artefacts that today live at the project root remain there in workspace mode, owned by the workspace rather than any scope:

| Artefact                  | Owner       | Path                                                  | Why workspace-level                                    |
| ------------------------- | ----------- | ----------------------------------------------------- | ------------------------------------------------------ |
| Plan (`plan.yaml`)        | `plan@v1`   | `<workspace-root>/.specify/plan.yaml`                 | A single plan can have entries spanning scopes.        |
| Registry (`registry.yaml`) | `registry@v1` | `<workspace-root>/.specify/registry.yaml`           | Cross-repo registry is per-repo, not per-scope.        |
| Initiative (`initiative.md`) | `initiative@v1` | `<workspace-root>/.specify/initiative.md`        | Initiatives orchestrate work across the whole repo.    |
| Plan lock                 | core        | `<workspace-root>/.specify/plan.lock`                 | One executor per workspace prevents racing scopes.     |
| Change archive            | core        | `<workspace-root>/.specify/archive/`                  | Flat archive with `scope:` metadata per change.        |
| `.specify/.cache/`        | core        | `<workspace-root>/.specify/.cache/`                   | Schema cache shared across scopes when URLs match.     |

Every per-scope artefact (`changes/`, scope-local rules) lives under `<scope>/.specify/`. The split is mechanical: anything coordinating across scopes lives at the workspace root; anything specific to a scope's domain schema lives in the scope.

### What stays unchanged from RFC-13

Re-stating for clarity, because the surface area RFC-14 leaves alone is large:

- The manifest protocol (`artifacts:`, `consumes:`).
- The lifecycle taxonomy (`managed`, `external`, `read-only`, `audited`).
- First-party schemas embedded in the CLI binary and exposed through the same resolver as URL-resolved schemas.
- The `consumes:` field for read-only inter-schema dependencies (RFC-14 only adds the `@<scope>` qualifier when disambiguation is needed).

The reframe principle — "schemas are the only extension point, without exception" — is preserved. Workspaces are not a new extension point; they are a structural composition of existing schema activations across a path-prefix tree. The core never learns the word "scope" except in path-resolution and change-selection layers; everything inside a schema's pipeline is unchanged.

## Alternatives Considered

**One repo per domain schema.** The status quo. Already used and works, but pays cost in registry edits, workspace sync, and cross-repo plan correlation for what is logically one team's repository. Rejected as the only path because the per-repo overhead is real and growing.

**Multi-domain through a fused capability.** Considered: a `super-schema@v1` that manually combines both `omnia@v1` and `contracts@v1`. Rejected: forcing two unrelated artefact families through one fused pipeline produces a pipeline neither schema's authors signed up for, and artefact ids would still collide without scopes.

**A flat `schemas: [<url>, …]` list at the project root.** Considered as the minimum-pain shape — no scopes, just multiple domain schemas, each with project-root path resolution. Rejected because it reintroduces RFC-13's "baseline-path uniqueness across active schemas" rule with no escape valve: two schemas that both want a `specs` baseline at `.specify/specs/` would collide. The scope prefix is what makes the multi-domain case actually work.

**Per-member `project.yaml` with no workspace root.** Considered: each member directory carries its own `.specify/project.yaml`, no workspace shape at all. Rejected: platform schemas (plan, registry, initiative) are inherently workspace-wide. Multiple `project.yaml` files force a separate "where does the registry live?" decision and re-introduce hub-vs-leaf discriminators we're trying to retire.

**Cargo-precise TOML sections.** Considered keeping the literal `[package]` / `[workspace]` section names from `Cargo.toml`. Rejected because YAML doesn't have TOML's section syntax — using `package:` and `workspace:` as top-level keys is the closest natural translation, and reads more naturally than nested mapping keys in YAML.

**Cross-scope changes as an opt-in.** Considered: a single change that writes deltas to two scopes' baselines, gated by a `cross-scope: true` flag. Rejected for the three reasons in §Cross-scope changes are forbidden — it breaks the core principle, re-couples the lifecycles, and `plan@v1` already solves the dependency-ordering problem.

**Schema-local executable extensions inside each member directory.** Considered: in a workspace, every member ships separate executable helpers under `<member-path>/`. Rejected: imperative behavior belongs to capability skills and their existing tool/script mechanisms; member-local executable discovery would create resolution rules that change between the workspace root and a member subdirectory.

## Non-Goals

- **Replacing single-schema (Mode A) projects.** Mode A is the common case and stays the default. Operators never see workspace concepts unless they opt in.
- **Multiple **domain** schemas in one scope.** A scope activates exactly one domain schema. Two domain schemas in one path tree means two scopes — splitting the tree, not folding the schemas.
- **Cross-scope changes within one transaction.** Coordinated work uses the plan loop. (See §Cross-scope changes are forbidden.)
- **Scope-level plan / initiative / registry.** Platform schemas remain workspace-wide. A scope cannot have its own `plan.yaml` or `registry.yaml` separate from the workspace's.
- **Heterogeneous schema versions in the same scope.** A scope pins exactly one version of its domain schema (the URL in `workspace.members[].schema`). Sibling scopes may pin different versions.
- **Workspace dependencies between scope schemas.** Cargo's `[workspace.dependencies]` for shared crate versions has no Specify analogue — schemas don't have a transitive dependency graph in that sense. The `consumes:` field handles read-only baseline reads, which is the only inter-schema coupling Specify uses today.
- **Default-members and selective execution.** Cargo's `default-members` is a future concern. RFC-14 always operates on every active scope unless the operator narrows with cwd inference or `--scope`.

## Implementation Scope

A staged landing on top of RFC-13 phases 1–4, each independently testable and shippable. Every stage preserves working `/spec:draft → /spec:build → /spec:adopt` for Mode A (single domain schema, no workspace).

### Phase 1 — `project.yaml` shape and parsing

1. Widen `schemas/project.schema.json` (or wherever the `project.yaml` schema lives) to admit the new shape: optional `package:`, optional `workspace:`, with the constraint "at least one MUST be present." Today's flat `schema:` field becomes `package.schema:` under a back-compat shim.
2. Parse the new shape in `crates/specify-platform/` (or its successor) into `ProjectConfig { package: Option<PackageConfig>, workspace: Option<WorkspaceConfig> }`.
3. Add the `Scope` newtype (`""` for root, member paths otherwise) and thread it through `ProjectConfig::active_schemas() -> Vec<(Scope, SchemaActivation)>`.
4. `specify check` lints: at least one of `package:` / `workspace:` present, member-path overlap rejected, member paths exist on disk, glob patterns match at least one directory.

Mode A repos parse identically under the new shape via the back-compat shim — see §Migration.

### Phase 2 — Scope-aware path resolution and brief substitutions

1. `ProjectConfig::baseline_path(&scope, &schema, artifact_id)` and `delta_path(&scope, &schema, artifact_id)` replace the RFC-13 phase-1 signatures.
2. Brief renderer learns `$SCOPE_DIR` and resolves `$ARTIFACT_*` substitutions through the scope. For Mode A repos with `Scope::root`, the resolved paths are byte-identical to today.
3. `consumes:` resolution gains the `@<scope>` qualifier (optional when one provider, mandatory when multiple).
4. `specify check` lints: every brief's `$ARTIFACT_*` substitution refers to an artefact id active in the same scope (or a `@<scope>`-qualified consumed schema), no literal paths.

### Phase 3 — Change scoping

1. `specify change create <name>` accepts `--scope <path>` and infers from cwd otherwise.
2. `.metadata.yaml` records `scope:` and `specify change list` / `status` / `validate` / `adopt` / `drop` filter or fan out by scope.
3. The change directory physically lives at `<workspace-root>/<scope>/.specify/changes/<name>/`.
4. Cross-scope changes are rejected: writing a delta outside the change's declared scope errors with `cross-scope-write-forbidden` pointing at this RFC.

### Phase 4 — `specify migrate v1-to-workspaces`

1. One-shot migration that turns a `schema:` at the top level of `project.yaml` into `package.schema:`, preserves `domain:` and `rules:` under `package:`, and writes an empty `workspace.members: []` block iff the operator passed `--add-workspace`.
2. The `hub: true` flag is rewritten to "no `package:`, `workspace:` only." A back-compat shim continues to read `hub: true` for two minor releases, with a deprecation warning pointing at the new shape.
3. `specify init --workspace` scaffolds a workspace shell from scratch; `specify init --add-scope <path> --schema <url>` adds a member to an existing workspace.

### This repo (`augentic/specify`)

1. Update `schemas/README.md` to document the workspace shape, scope concept, and per-member `scope.yaml`.
2. Add the new fields to `schemas/project.schema.json` (or wherever `project.yaml` is validated) — `package`, `workspace`, `members`, `scope` admission rules, member-path-overlap rule.
3. Update fixture `project.yaml` files under `plugins/spec/skills/plan/fixtures/` to demonstrate the new shape — including a `package + workspace` mode fixture.
4. Update `plugins/spec/skills/init/SKILL.md` and `plan/SKILL.md` to describe the workspace surface, the `--add-scope` flow, and how plans target scopes.
5. Document scope-aware `consumes:` in `docs/reference/schema-extensions.md` (the RFC-13 docs landing).
6. Add a glossary entry for "workspace," "scope," "root scope," "member scope," and "workspace-root concern."

Estimated total: ~1200–1700 lines of Rust + schema updates + fixture refresh + plugin doc updates. Substantially smaller than RFC-13 because the capability protocol carries the heavy lifting; RFC-14 is mostly a `Scope` parameter threaded through existing call sites and three new lints in `specify check`.

## Migration

The cut-over is **strictly additive** for single-domain repos: today's `schema: <url>` at the top level of `project.yaml` continues to work as an alias for `package.schema: <url>` with no `workspace:` block. No existing project changes shape unless its operator opts in to a workspace.

Two invariants guard the landing:

1. **Mode A keeps working byte-for-byte.** Every phase's acceptance criterion runs `/spec:draft → /spec:build → /spec:adopt` on the canonical omnia change with the **flat** `schema: <url>` shape preserved. No path on disk changes for Mode A repos.
2. **Multi-mode parity.** A workspace with one member at `./` (a redundant but legal Mode C shape) produces byte-identical paths to a Mode A project — i.e., the root scope is the empty path, not a path component named `root`.

The `hub: true` flag is **deprecated, not removed** in the same release that lands RFC-14. A back-compat shim translates `hub: true` to "no `package:`, `workspace.members: []`" and emits a one-time deprecation warning pointing at the new shape. The shim is removed two minor releases later, matching RFC-13's hard cut-over posture.

`specify migrate v1-to-workspaces` is **opt-in** rather than required. A repo only needs to migrate if its operator wants to add a member scope; until then, the flat shape continues to parse.

`specify check` (RFC-5) gains the workspace lints described in §Implementation Scope — they fail CI on any new ill-formed `project.yaml`, but cannot fail on existing repos because Mode A is unaffected.

## Open Questions

1. **Per-member `scope.yaml` mandatory or optional?** Provisional: optional, with workspace-root values as the default. A scope only needs its own file if it wants to override config or rules. A future RFC may require it for very large workspaces (consistency over flexibility).
2. **Cwd inference depth.** `specify change create` without `--scope` walks up looking for the nearest scope marker. How far should it walk? Provisional: until it hits a `.specify/project.yaml` (workspace root) or a `.specify/scope.yaml` (member). If neither is found, error with `not-in-a-specify-project`.
3. **`@<scope>` qualifier syntax.** Considered: `$ARTIFACT_BASELINE[contracts]@contracts/`, `$ARTIFACT_BASELINE[contracts:contracts/]`, `$ARTIFACT_BASELINE[contracts/contracts]`. Provisional: `@<path>` because the `@` separator visually distinguishes a path qualifier from an artefact id, and nothing else in the substitution vocabulary uses `@` today.
4. **Scope name vs scope path.** Today scopes are identified by their relative path. Should scopes have a separate logical `name:` field for diagnostics and references? Provisional: no — the path is the canonical identifier, exactly as Cargo uses workspace member paths. A scope renamed by moving its directory is a `git mv`, not a config change. Revisit if `@<long/path/with/slashes>` becomes painful in briefs.
5. **Glob discovery semantics.** `members: ["scopes/*"]` discovers every direct subdirectory of `scopes/`. What about nested globs (`scopes/**/*.specify-marker`)? Provisional: only single-level globs in the first landing; nested globs require a scope marker file (e.g., `.specify/scope.yaml`) and are deferred until a real workspace needs them.
6. **Per-scope plan locks.** Today `.specify/plan.lock` is workspace-wide and serialises `/spec:execute --loop`. Should two scopes be allowed to execute in parallel? Provisional: no — the loop's invariants (one journal stream, one phase outcome write at a time) are workspace-wide. A future RFC may relax this if real workspaces need concurrent scope execution.
7. **Default scope when both `package:` and `workspace:` are present.** A change-create call from the workspace root: does it default to the root scope (because `package:` is present) or refuse and require an explicit `--scope`? Provisional: default to root scope when `package:` is declared; refuse when only `workspace:` is declared. The cwd-inference rule covers the in-member case.
8. **Cross-repo plan entries that target a `<repo>/<scope>` pair.** `plan@v1`'s entry shape today carries `project: <repo-name>`. With workspaces, an entry may target a specific scope. Provisional: extend the entry shape with optional `scope: <path>`; absent scope means the root scope or the only scope; required when the target repo activates more than one scope and the entry doesn't otherwise disambiguate. RFC-14 leaves the exact shape to a small follow-up patch on the `plan@v1` schema.
9. **Should `specify init` enforce a Mode A default?** When an operator runs `specify init` without flags, do they get Mode A (a flat `schema:`) or the new `package: schema:` shape? Provisional: Mode A — the migration shim keeps both shapes equivalent, and the flat shape stays the default for the common case until a real opt-in (`--workspace`, `--add-scope`) happens. A future minor release may flip the default once workspaces are common.
10. **Workspace-level rules (`rules:` block).** Today `rules:` lives next to `schema:` at the project root. Under `package:` / `workspace:` split, where does a workspace-wide `rules:` block go? Provisional: `rules:` is per-scope (under `package.rules:` or `scope.yaml:rules:`), with no workspace-wide default. A workspace-wide rules-override mechanism is a future RFC if the duplication becomes painful.
11. **First-party schema versioning across scopes.** RFC-13 §First-party schemas pins platform-schema versions to the CLI version. In a workspace, the CLI version is one — so the platform schemas are uniformly versioned across scopes. Confirm this stays true even in `workspace.members[]` entries that pin older domain-schema versions. Provisional: yes — domain schemas are independently pinnable; platform schemas track the CLI.
12. **Backward-compat shim retirement.** The `hub: true` and flat `schema: <url>` shims continue to work for two minor releases. Confirm this is the right window. Provisional: two minors, matching RFC-13's hard cut-over posture. Adjust if real adoption is slower than expected.

## References

- [RFC-13: Immutable core + schema extensions](rfc-13-extensibility.md) — owns the capability protocol that RFC-14 layers scope-resolution on top of. The §Cross-schema coexistence rules are the direct ancestors of RFC-14's scope-aware uniqueness rules; §Open Questions #2 and #13 are the points this RFC resolves.
- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) — owns `crates/specify-platform/`, where `ProjectConfig` parses today's `project.yaml`. Phase 1 of this RFC widens that parser.
- [RFC-2: Execution](archive/rfc-2-execution.md) — owns the `plan@v1` schema and `/spec:execute --loop`. RFC-14's "cross-scope changes route through the plan loop" stance leans on RFC-2's existing dependency-ordering primitives.
- [RFC-3a: Monoliths](archive/rfc-3a-monoliths.md) — plan authoring; the per-entry `scope:` field is a minor extension to RFC-3a's plan-entry shape.
- [RFC-3b: Platform](archive/rfc-3b-platform.md) — registry routing across repos. Cross-repo plans gain the `<repo>/<scope>` target shape from this RFC's §Open Questions #8.
- [RFC-9: Platform](archive/rfc-9-platform.md) — moved registry, plan, initiative, and contracts to the repo root. The `--hub` flag this RFC's `workspace:`-only mode subsumes was introduced here.
- [RFC-12: Refine RFC-8](archive/rfc-12-refine-rfc-8.md) — cross-repo contract id uniqueness. Continues to apply in workspaces (a contract id is still globally unique across the registry, regardless of which scope or repo holds it).
- [RFC-5: Framework Linter](rfc-5-lint.md) — home of the new lints: at-least-one-of-package-or-workspace, member-path-overlap, glob-must-match, scope-aware artefact-id uniqueness, scope-relative `consumes:` resolution.
- [Roadmap](roadmap.md) — §3 (standards/codex), §4 (CI-native review), and §6 (observability) all benefit from per-scope routing once workspaces land.
- Cargo manifest reference, [Workspaces section](https://doc.rust-lang.org/cargo/reference/workspaces.html) — the model RFC-14 borrows: `[package]` / `[workspace]` shape, member globs, workspace-root coordination, default-members deferral.
