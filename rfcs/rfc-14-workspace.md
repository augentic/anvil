# RFC-14: Registry workspace

> Status: Draft · Depends: [RFC-13](archive/rfc-13-extensibility.md), [RFC-1](archive/rfc-1-cli.md), [RFC-9](archive/rfc-9-platform.md)

## Abstract

RFC-14 lets one repository activate more than one domain capability. A repo can, for example, use `omnia@v1` at the root for Rust+WASM implementation work and `contracts@v1` under `contracts/` for independent API contract work.

RFC-14 also makes that shape work for registry-managed remote repositories. `registry.yaml` continues to declare which repos participate in a change; `specify workspace sync` materialises those repos under `.specify/workspace/<project>/`; RFC-14 then resolves a scope inside the selected materialised project. The slice loop modifies the materialised checkout, and the existing `specify workspace push` / `specify workspace merge` flow pushes those changes back to the remote origin.

The implementation adds **scopes** on top of RFC-13's capability protocol:

- `.specify/project.yaml` gains optional `package:` and `workspace:` blocks.
- `package:` declares the optional root scope.
- `workspace.members[]` declares member scopes by relative path and capability URL.
- Platform components (`registry`, `change`) stay coordinator-root concerns.
- Artifacts, slices, brief substitutions, and capability skill invocations resolve through a scope coordinate.

RFC-14 changes path resolution and change scoping only. Registry topology, remote materialisation, push/PR mechanics, capability manifests, artifact lifecycles, and the RFC-13 extension protocol are unchanged.

## Motivation

RFC-13 allows one domain capability plus first-party platform components per project. That singleton does not fit repositories that own multiple independent artifact families:

- A service repo may need `omnia@v1` for code and `contracts@v1` for API contracts. Today it must pick one primary lifecycle and treat the other as a side effect.
- A platform repo may own infrastructure, design tokens, and generated clients. Today each concern usually becomes a separate repo and registry entry.
- A monorepo may need the same capability in many subdirectories, such as `omnia@v1` per service.

RFC-14 makes the filesystem carry these boundaries. Each scope activates one domain capability; the coordinator root remains the workspace that coordinates registry, change plans, locks, and archives.

## Design

### Core Model

A project workspace contains zero or more scopes. Each scope activates exactly one domain capability.

- **Coordinator root**: owns platform components: `registry.yaml`, `change.md`, `plan.yaml`, `.specify/plan.lock`, `.specify/archive/` for change/plan records, and `.specify/workspace/`.
- **Project workspace root**: owns `.specify/project.yaml` for the repo currently being operated on. In single-repo mode this is the coordinator root. In registry mode this may be a materialised peer at `.specify/workspace/<project>/`.
- **Root scope** (`""`): active when `package:` is declared. Its domain artifacts resolve from the project workspace root.
- **Member scope** (`<path>`): active for each `workspace.members[]` entry. Its domain artifacts resolve under `<project-workspace-root>/<path>/`.

The project workspace root is not itself a scope unless `package:` is declared.

### `project.yaml`

At least one of `package:` or `workspace:` MUST be present.

```yaml
# .specify/project.yaml
name: my-app
specify_version: 1.5

package:
  capability: https://github.com/augentic/specify/capabilities/omnia@v1
  domain:
    description: Customer orders service
  rules:
    proposal: rules/proposal.md

workspace:
  members:
    - path: contracts/
      capability: https://github.com/augentic/specify/capabilities/contracts@v1
    - path: infra/
      capability: https://github.com/augentic/specify/capabilities/infra@v1
  # Optional shorthand:
  # members: ["services/*"]

extensions:
  registry: { ... }
  change: { ... }
```

Legal modes:


| Mode                | Shape        | Notes                                                                                         |
| ------------------- | ------------ | --------------------------------------------------------------------------------------------- |
| Package only        | `package:`   | Current single-capability project, also read from legacy flat `capability:` during migration. |
| Workspace only      | `workspace:` | Registry-only/platform hub or multi-member repo with no root domain capability.               |
| Package + workspace | both         | Root domain plus member scopes.                                                               |


### `scope.yaml`

A member MAY define `<member-path>/.specify/scope.yaml` for member-local rules and capability extension config.

```yaml
# contracts/.specify/scope.yaml
rules:
  specs: rules/contracts-specs.md
extensions:
  contracts:
    format-policy: strict-semver
```

`scope.yaml` never declares the capability URL. Capability activation is always listed in the project workspace root's `project.yaml` so the active scope set is discoverable from one file. Member config merges as `workspace extensions.<capability>` plus `scope.yaml`, with scope-local values overriding for that scope only.

### Registry Materialisation

RFC-14 composes with the registry workspace from RFC-9/RFC-13 instead of replacing it.

Remote repositories are still declared only in `registry.yaml`:

```yaml
version: 1
projects:
  - name: orders
    url: git@github.com:org/orders.git
    schema: omnia@v1
    description: Customer orders service.
```

`workspace.members[]` never carries a `url:` and never identifies a remote repository. It identifies scopes inside a project workspace root after that project has been resolved.

When a change plan entry targets a registry project, `/change:execute` resolves in two steps:

1. **Project resolution.** The registry materialiser resolves `project: <name>` to a local project workspace root. Remote URLs are shallow-cloned or fetched under `<coordinator-root>/.specify/workspace/<name>/`; local paths are symlinked. These materialised slots are derived state and may be refreshed or removed between changes.
2. **Scope resolution.** RFC-14 resolves the plan entry's `scope: <path>` inside that materialised project workspace root. Capability skills then run with `$PROJECT_DIR` bound to the project workspace root and `$SCOPE_DIR` bound to the selected scope.

The slice loop may modify files inside the materialised project checkout, including scoped baselines, implementation code, and slice metadata. After execution, `specify workspace push` remains responsible for creating or updating the `specify/<change-name>` branch, pushing it to the project's remote origin, and opening or updating the PR. `specify workspace merge` remains responsible for merging those PRs once CI passes.

This means a remote repository is temporarily cloned into the coordinator's derived workspace, mutated locally by the ordinary slice loop, then pushed back through the existing registry workspace verbs. RFC-14 only adds the extra coordinate needed to choose which capability scope inside that repo receives the work.

### Path Resolution

RFC-13 brief substitutions stay the public interface. RFC-14 changes their resolver:


| Substitution               | Resolution                                                     |
| -------------------------- | -------------------------------------------------------------- |
| `$PROJECT_DIR`             | Project workspace root.                                        |
| `$SCOPE_DIR`               | `$PROJECT_DIR/<scope>`; root scope resolves to `$PROJECT_DIR`. |
| `$ARTIFACT_DELTA[<id>]`    | `$SCOPE_DIR/.specify/slices/<name>/<delta-path>`               |
| `$ARTIFACT_BASELINE[<id>]` | `$SCOPE_DIR/<baseline-path>`                                   |


For the root scope in single-repo mode, resolved paths are byte-identical to today's single-capability behavior. For a registry project, the same substitutions resolve inside the materialised project checkout. Existing briefs that use `$ARTIFACT_*` substitutions continue to work.

Implementation note: `ProjectConfig::baseline_path(&capability, artifact_id)` becomes `baseline_path(&scope, &capability, artifact_id)`. Delta resolution follows the same signature pattern. `Scope` is a newtype around the empty root path or a normalized member path.

### Validation

`specify check` MUST enforce:

- At least one of `package:` or `workspace:` exists.
- Every member path exists and is a directory.
- Member glob patterns match at least one direct child directory.
- Member paths do not overlap; no member path may be an ancestor or descendant of another.
- Artifact ids are unique within a scope.
- Baseline/project paths are unique workspace-wide after absolute resolution.
- Platform-component artifact ids are unique workspace-wide.
- Brief substitutions resolve to artifacts active in the same scope unless they use a valid consumed-scope qualifier.

### Cross-Scope Reads

RFC-13 `consumes:` remains the only read-only dependency mechanism between capabilities.

When a consuming capability needs an artifact from another scope, it uses:

```text
$ARTIFACT_BASELINE[<artifact-id>]@<scope>
```

The `@<scope>` qualifier is optional when exactly one active scope provides the consumed capability and required when multiple scopes provide it. Platform components are always reachable without a scope qualifier because they live at the coordinator root.

### Slices

A slice belongs to exactly one scope.

```bash
# Inferred from cwd
cd contracts/
specify slice create add-orders-api

# Explicit from project workspace root
specify slice create add-orders-api --scope contracts/

# Root scope, when package: is declared
specify slice create my-feature
```

Scope inference walks upward from cwd until it finds a project workspace root `.specify/project.yaml` or a member `.specify/scope.yaml`. From a workspace-only project root, `--scope <path>` is required because there is no root scope.

Slice metadata records:

```yaml
scope: contracts/
```

The slice directory lives at:

```text
<project-workspace-root>/<scope>/.specify/slices/<name>/
```

Archived or dropped slices move to the matching scoped archive:

```text
<project-workspace-root>/<scope>/.specify/archive/<archive-name>/
```

Cross-scope slices are forbidden. A slice may write only to baselines in its declared scope; attempted writes outside that scope fail with `cross-scope-write-forbidden`. Coordinated multi-scope work uses a change plan with one entry per scope and ordinary `needs:` dependencies.

### Change Plans

Plan and change artifacts stay coordinator-level:


| Artifact            | Path                                      |
| ------------------- | ----------------------------------------- |
| Change brief        | `<coordinator-root>/change.md`            |
| Plan                | `<coordinator-root>/plan.yaml`            |
| Plan lock           | `<coordinator-root>/.specify/plan.lock`   |
| Registry            | `<coordinator-root>/registry.yaml`        |
| Change/plan archive | `<coordinator-root>/.specify/archive/`    |


Plan entries gain an optional `scope: <path>` field. It is required when the target repository has more than one possible scope and the entry is otherwise ambiguous. Absent scope means the root scope or the only active scope.

`/change:execute` continues to process one plan entry at a time. The selected entry supplies the scope for `/spec:define`, `/spec:build`, `/spec:merge`, and any capability skill they invoke.

### Capability Skills

Capability skill context MUST include the resolved scope path when invoked from a scoped slice or plan entry. This is required when the same capability is active in multiple scopes so diagnostics and writes can be attributed unambiguously. Single-scope projects keep the current call surface.

## Non-Goals

- Multiple domain capabilities in one scope.
- A scope-local registry, `change.md`, or `plan.yaml`.
- A single slice that writes to multiple scopes.
- A new capability dependency system beyond RFC-13 `consumes:`.
- Parallel `/change:execute` per scope.
- Literal path access in briefs as a substitute for `$ARTIFACT_*` resolution.

## Implementation Scope

### Phase 1 - Config Parsing

1. Update the `project.yaml` schema for `package:`, `workspace:`, `workspace.members[]`, and the at-least-one constraint.
2. Preserve legacy `capability:` as an alias for `package.capability:` during the migration window.
3. Parse `ProjectConfig { package: Option<PackageConfig>, workspace: Option<WorkspaceConfig> }`.
4. Add `Scope` and expose `ProjectConfig::active_capabilities() -> Vec<(Scope, CapabilityActivation)>`.
5. Add workspace validation lints for member paths, globs, overlap, and uniqueness.

### Phase 2 - Resolution

1. Thread `Scope` through baseline and delta path resolution.
2. Add `$SCOPE_DIR`.
3. Resolve `$ARTIFACT_*` substitutions relative to the selected scope.
4. Add consumed-scope parsing for `$ARTIFACT_BASELINE[<id>]@<scope>`.
5. Reject unresolved or literal artifact paths in brief validation.

### Phase 3 - Slice Scoping

1. Add `--scope <path>` to `specify slice create`.
2. Infer scope from cwd when possible.
3. Record `scope:` in `.metadata.yaml`.
4. Store slice directories under `<project-workspace-root>/<scope>/.specify/slices/<name>/` and slice archives under `<project-workspace-root>/<scope>/.specify/archive/`.
5. Make `slice list`, `status`, `validate`, `adopt`, and `drop` scope-aware.
6. Reject cross-scope writes with `cross-scope-write-forbidden`.

### Phase 4 - Migration and Init

1. Add `specify migrate v1-to-workspaces`.
2. Rewrite flat `capability:` to `package.capability:`.
3. Move root `domain:` and `rules:` under `package:`.
4. Rewrite `hub: true` to workspace-only shape.
5. Keep `hub: true` readable for two minor releases with a deprecation warning.
6. Add `specify init --workspace`.
7. Add `specify init --add-scope <path> --capability <url>`.

### Repository Updates

1. Update `capabilities/README.md` with workspace shape, scopes, and `scope.yaml`.
2. Update the `project.yaml` JSON Schema.
3. Refresh `plugins/spec/skills/plan/fixtures/` with package-only, workspace-only, and package-plus-workspace examples.
4. Update `plugins/spec/skills/init/SKILL.md` and `plugins/spec/skills/plan/SKILL.md` for scope-aware flows.
5. Document scope-aware `consumes:` in `docs/reference/capability-extensions.md`.
6. Add glossary entries for coordinator root, project workspace root, scope, root scope, member scope, and coordinator-root concern.

## Migration

The change is additive for existing single-capability repositories:

- Existing flat `capability: <url>` remains valid as an alias for `package.capability: <url>` during the migration window.
- Mode A paths remain byte-identical.
- No project must migrate unless it wants member scopes.
- `hub: true` is deprecated, translated to workspace-only shape, and removed after two minor releases.

Acceptance criteria for every phase:

- A legacy flat `capability:` project can still run `/spec:define -> /spec:build -> /spec:merge`.
- A root-scope package project resolves paths identically to the legacy shape.
- `specify check` rejects malformed workspace configs without failing valid legacy configs.

## Open Questions

1. **Scope qualifier syntax.** Provisional: `$ARTIFACT_BASELINE[contracts]@contracts/`.
2. **Logical scope names.** Provisional: no separate `name:`; the normalized member path is the scope identifier.
3. **Glob semantics.** Provisional: only single-level member globs in the first landing.
4. **Default scope at project workspace root.** Provisional: default to root when `package:` exists; require `--scope` in workspace-only roots.
5. **Workspace-wide rules.** Provisional: rules are per-scope only (`package.rules:` or `scope.yaml:rules:`).
6. **Shim retirement.** Provisional: flat `capability:` and `hub: true` compatibility lasts two minor releases.

## References

- [RFC-13: Immutable core + capability extensions](archive/rfc-13-extensibility.md) - capability protocol, artifact lifecycle, and coexistence invariants.
- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) - `ProjectConfig` parser location.
- [RFC-2: Execution](archive/rfc-2-execution.md) - plan loop and dependency ordering.
- [RFC-3a: Monoliths](archive/rfc-3a-monoliths.md) - plan entry shape.
- [RFC-3b: Platform](archive/rfc-3b-platform.md) - cross-repo routing.
- [RFC-9: Platform](archive/rfc-9-platform.md) - registry/change root placement and `--hub`.
- [RFC-5: Framework Linter](rfc-5-lint.md) - validation/lint home.
- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html) - package/workspace/member model RFC-14 mirrors.

