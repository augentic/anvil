# RFC-13 Implementation Plan

> Status: Draft · Source: [rfc-13-extensibility.md](rfc-13-extensibility.md) · Spans repos: `augentic/specify-cli`, `augentic/specify`

## How to read this plan

Each entry below is a **change** — a unit of work small enough to fit one
subagent without smashing context. Changes are grouped into the four phases
the RFC itself prescribes; phases are sequential, but inside a phase
changes are tagged with a **wave** number so a coordinator can dispatch a
wave's worth of subagents in parallel.

Conventions:

- **Repo** — `cli` is `augentic/specify-cli`; `repo` is `augentic/specify`.
- **Wave** — within a phase, all changes in the same wave are independent
  and can run as concurrent subagents. Waves are strictly ordered: wave N
  must complete before wave N+1 starts.
- **Depends** — explicit upstream changes that must merge first (across
  phases or waves).
- **Size** — rough Rust LOC budget; doc-only changes are a few hundred
  markdown lines.
- **Scope** — bounded set of files / crates the subagent should touch.
  Extra files signal scope creep.
- **Acceptance** — the concrete check the subagent must satisfy before
  handing off.

The four invariants from RFC-13 §Migration apply to every change:

1. Omnia keeps working: `/spec:define → /spec:build → /spec:merge` runs
   end-to-end on a canonical omnia slice after every change.
2. The core never learns a capability name.
3. Concern-specific behaviour leaves core.
4. Platform components stay outside the active capability set
   (dependency direction: `specify-change → specify-registry → core`).

Phase boundaries are hard cut-overs (§Migration): no compatibility
aliases survive past their phase.

---

## Phase 0 — Pre-flight (single subagent, no waves)

Cheap, low-risk groundwork that lets later subagents start with a clear
target. Land before Phase 1 wave 1.

### 0.1 — Capability-manifest schema design (`repo`)

- Repo: `repo`
- Size: ~120 lines (JSON + prose)
- Scope: `capabilities/capability.schema.json` (new), short README,
  `docs/reference/capabilities.md` skeleton.
- Deliverables:
  - Draft `capabilities/capability.schema.json` with the post-RFC field
    set: `name`, `version`, `description`, `pipeline { define, build,
    merge }`. No `domain`, no `extends`, no `plan` phase.
  - Stub `docs/reference/capabilities.md` documenting the manifest
    protocol and the dependency direction core ← change ← registry.
- Acceptance: `make checks` passes; the JSON Schema validates the example
  manifest in RFC-13 §Capability manifest and protocol.
- Notes: Pure design artefact. Nothing in `cli` consumes it yet — that
  arrives in Phase 1.

---

## Phase 1 — Capability vocabulary cut-over (~400-700 LOC)

Renames the extension primitive without changing artefact mechanics.
`pipeline:` behaviour stays byte-for-byte identical.

### Wave 1.A — Core type rename

#### 1.1 — Rename `specify-schema` crate to `specify-capability` (`cli`)

- Repo: `cli`
- Size: ~250 LOC + Cargo wiring
- Scope: `crates/schema/` → `crates/capability/`; `Cargo.toml` workspace
  members + `[workspace.dependencies]` table; `Schema → Capability`,
  `ResolvedSchema → ResolvedCapability`, `SchemaSource →
  CapabilitySource` across `crates/capability/src/`. Drop `domain` and
  `extends` from the parsed type.
- Deliverables: rebuilt workspace; downstream consumers of
  `specify-schema` (`specify` lib, `crates/merge`, `crates/change`,
  `crates/validate`) updated to the new crate name. Public re-exports
  in `src/lib.rs` follow.
- Acceptance: `cargo build && cargo test -p specify-capability` green;
  `cargo build -p specify` green.
- Notes: Mechanical rename plus type renames. The CLI dispatch and
  user-facing `specify schema *` verbs stay unchanged in this change so
  the diff stays bisectable.

### Wave 1.B — Surface rename (parallel × 4)

All four changes start once 1.1 lands.

#### 1.2 — Rename `Commands::Schema` to `Commands::Capability` (`cli`)

- Repo: `cli`
- Depends: 1.1
- Size: ~150 LOC
- Scope: `src/cli.rs` (`SchemaAction` → `CapabilityAction`),
  `src/commands/mod.rs`, `src/commands/schema.rs` →
  `src/commands/capability.rs`. Update help text. Pre-cut-over
  manifest detection: `specify capability check` still recognises a
  `schema.yaml` on disk and emits `Error::SchemaBecameCapability` with
  a clear message linking to RFC-13 §Migration.
- Acceptance: `cargo run -- capability {resolve,check,pipeline}` works
  on the omnia capability fixture; `specify schema *` is gone from
  `--help`.

#### 1.3 — `specify init <capability>` positional (`cli`)

- Repo: `cli`
- Depends: 1.1
- Size: ~200 LOC (init logic + tests)
- Scope: `src/cli.rs` (Init args), `src/commands/init.rs`,
  `src/init.rs`, `src/config.rs` (`project.yaml:schema` →
  `project.yaml:capability`). Drop `--schema-uri`. Mutual exclusion
  with `--hub`. New diagnostic
  `init-requires-capability-or-hub`. Hub mode writes only `hub: true`
  (drop the `schema: hub` sentinel).
- Acceptance: integration tests in `tests/cli.rs` cover (a)
  `specify init <url>`, (b) `specify init --hub`, (c) `specify init`
  with neither (errors), (d) `specify init <url> --hub` (errors).
  `project.yaml` shape after init shows `capability:` for regular
  projects and just `hub: true` for hubs.

#### 1.4 — Rename `schema.schema.json` → `capability.schema.json` in CLI (`cli`)

- Repo: `cli`
- Depends: 1.1
- Size: ~80 LOC
- Scope: `schemas/schema.schema.json` → `schemas/capability.schema.json`;
  drop `domain` and `extends` properties; require `pipeline { define,
  build, merge }`. Drop the `plan` phase from the schema (planning
  briefs move to the change surface in Phase 3 — schema cut-over here
  reflects the RFC's "manifest pipeline contains only slice phases").
- Acceptance: schema-validation tests pass against the rewritten
  capability fixtures.

#### 1.5 — Move `schemas/<name>/` → `capabilities/<name>/` in repo (`repo`)

- Repo: `repo`
- Depends: 0.1 (shape) — independent of `cli` waves
- Size: file moves + ~100 lines of frontmatter edits
- Scope: `schemas/{omnia,contracts,vectis}/schema.yaml` →
  `capabilities/{omnia,contracts,vectis}/capability.yaml`. Strip
  `domain` and `extends`; keep `pipeline:` as-is byte-for-byte (the
  Phase 1 invariant). The omnia manifest's `pipeline.plan` block stays
  here for now — it migrates in Phase 3 together with the rest of the
  planning surface.
- Acceptance: `make checks` passes against the moved manifests; each
  manifest validates against `capabilities/capability.schema.json`.

### Wave 1.C — Skill, doc, and acceptance fan-out (parallel × 3)

#### 1.6 — Update `/spec:init` skill and platform-init docs (`repo`)

- Repo: `repo`
- Depends: 1.3, 1.5
- Size: ~150 lines of markdown
- Scope: `plugins/spec/skills/init/SKILL.md`,
  `docs/explanation/platform-repo.md`,
  `docs/reference/directory-layout.md`, top-level `AGENTS.md` and
  `.cursor/rules/project.mdc` references. Replace
  `specify init --schema-uri <uri>` with `specify init <capability>`
  everywhere; replace "schema" with "capability" wherever it refers to
  the extension primitive (preserve "schema" for JSON Schema and for
  the `schema:` field on plan entries).
- Acceptance: `make checks` (which already enforces vocabulary
  consistency) passes.

#### 1.7 — Vocabulary sweep across remaining skills, references, RFCs (`repo`)

- Repo: `repo`
- Depends: 1.5
- Size: ~200 lines of markdown across many files
- Scope: every SKILL.md and reference under `plugins/`, `docs/`, plus
  RFC-14 and `rfcs/roadmap.md`. Use the schema → capability rename
  table from RFC-13 §Migration TL;DR. **Do not** rename change → slice
  / initiative → change yet — that lands in Phase 3.
- Acceptance: `make checks` passes; `rg -i 'specify schema |schema:' `
  in non-JSON-Schema contexts returns only intentional residue (the
  `schema:` field on plan entries, which is renamed in Phase 3).

#### 1.8 — Phase 1 acceptance smoke (`cli` + `repo`)

- Repo: both
- Depends: 1.2, 1.3, 1.4, 1.6, 1.7
- Size: ~150 LOC of integration tests + fixture refresh
- Scope: refresh `tests/cli.rs` and `tests/schema.rs` in `cli`;
  refresh canonical fixtures for omnia in `cli/schemas/omnia/`. Run
  `/spec:define → /spec:build → /spec:merge` against the canonical
  omnia fixture and confirm a clean pass.
- Acceptance: invariant #1 holds; pre-cut-over `schema.yaml` files
  fail loudly with `Error::SchemaBecameCapability`.

---

## Phase 2 — Component extraction (~700-1000 LOC)

Carves `specify registry` and the umbrella orchestration crate out of
core, deletes the concern-specific top-level command surfaces, and
strips `contracts`/`specs` literals from generic core helpers. Per the
RFC, this phase still uses the **pre-rename** noun set: per-loop unit is
`change`, umbrella orchestration is `initiative`, brief substitution is
`$CHANGE_DIR`. The lifecycle rename is Phase 3.

### Wave 2.A — Registry extraction (sequential within wave)

#### 2.1 — `crates/registry`: lift parsing & helpers (`cli`)

- Repo: `cli`
- Depends: Phase 1 complete
- Size: ~400 LOC
- Scope: create `crates/registry/`; move `crates/capability/src/registry.rs`
  + the registry-shape validators currently in
  `crates/validate/src/registry.rs` into the new crate; expose
  `Registry`, `RegistryProject`, parse/validate, add/remove helpers.
  Update `specify` lib re-exports to point at the new crate.
- Acceptance: `cargo build && cargo test -p specify-registry` green;
  `cargo test -p specify-validate` green (validate still runs the
  registry rules but now via the registry crate).

#### 2.2 — `crates/registry`: lift workspace materialisation (`cli`)

- Repo: `cli`
- Depends: 2.1
- Size: ~500 LOC
- Scope: move `src/workspace.rs` and `src/workspace_merge.rs` into
  `crates/registry/src/{workspace.rs, merge.rs}`. The
  `.specify/workspace/` directory remains derived registry state. Keep
  the public function names so `src/commands/workspace.rs` only changes
  its imports.
- Acceptance: `cargo test -p specify-registry --test workspace` green.

#### 2.3 — Re-wire `Commands::{Registry, Workspace}` to the registry crate (`cli`)

- Repo: `cli`
- Depends: 2.1, 2.2
- Size: ~150 LOC
- Scope: `src/commands/registry.rs` and `src/commands/workspace.rs`
  become thin dispatchers over `specify-registry` API. Drop registry
  knowledge from `crates/capability` and `specify` lib re-exports
  (registry types are re-exported from `specify-registry` only).
- Acceptance: `specify registry *` and `specify workspace *` behave
  identically to pre-extraction; `cargo test -p specify --test cli`
  green.

### Wave 2.B — Initiative crate extraction (sequential, can overlap with 2.A from 2.1 onward)

#### 2.4 — `crates/initiative`: lift plan + initiative orchestration (`cli`)

- Repo: `cli`
- Depends: 2.1 (`registry` crate must exist to depend on it)
- Size: ~700 LOC moved (no behaviour change)
- Scope: create `crates/initiative/` (placeholder name; renamed to
  `crates/change` in Phase 3). Move `src/initiative_finalize.rs`,
  `src/commands/plan/*`, plan logic from `crates/change/src/{plan.rs,
  plan_doctor.rs, lock.rs, journal.rs at the initiative slice}`,
  `src/commands/initiative.rs`. The lifted module names track their
  pre-rename surface: `initiative`, `plan`, `lock`. The crate depends
  on `specify-registry`, not on `specify-change` (the slice loop crate).
- Acceptance: `cargo build` clean; the dependency edge
  `specify-initiative → specify-registry → specify-capability` matches
  RFC-13 invariant #4.

#### 2.5 — Rewire `Commands::{Plan, Initiative}` to the new crate (`cli`)

- Repo: `cli`
- Depends: 2.4
- Size: ~150 LOC
- Scope: `src/commands/plan/mod.rs`, `src/commands/initiative.rs`,
  `src/commands/mod.rs` dispatch table. Drop initiative-related re-exports
  from `specify` lib unless externally consumed.
- Acceptance: `specify plan *` and `specify initiative *` keep working
  byte-for-byte.

### Wave 2.C — Concern-specific core surface removal (parallel × 2)

#### 2.6 — Drop `Commands::Vectis` from binary CLI (`cli`)

- Repo: `cli`
- Depends: 2.3, 2.5 (so platform components are clean before this)
- Size: ~200 LOC removed
- Scope: `src/cli.rs` (drop `Vectis` variant + `VectisAction`),
  `src/commands/vectis.rs` deleted, `src/commands/mod.rs` dispatch
  cleared. Keep `crates/vectis` reachable as a library so capability
  skills can call into it (it migrates to a standalone binary in Phase
  4.3a). Tests in `tests/vectis.rs` removed or moved to library-level.
- Acceptance: `specify --help` no longer lists `vectis`; the omnia
  define→build→merge loop is unaffected.

#### 2.7 — Drop `Commands::Contract` and contract validators from public API (`cli`)

- Repo: `cli`
- Depends: 2.3, 2.5
- Size: ~250 LOC removed/moved
- Scope: `src/cli.rs` (drop `Contract` variant + `ContractAction`),
  `src/commands/contract.rs` deleted, `src/commands/mod.rs` dispatch
  cleared. Stop re-exporting `validate_baseline_contracts`,
  `ContractFinding`, `CrossRule`, etc. from `specify` lib — keep the
  functions in `crates/validate` so Phase 4.2a can package them as a
  standalone binary.
- Acceptance: `specify --help` no longer lists `contract`; the
  contracts capability skill (`/contract:openapi verify` etc.) still
  runs through its own helpers.

### Wave 2.D — De-concern the generic helpers (sequential)

#### 2.8 — Strip `contracts`/`specs` literals from `specify-merge` and `ProjectConfig` (`cli`)

- Repo: `cli`
- Depends: 2.6, 2.7
- Size: ~300 LOC
- Scope: `crates/merge/src/change.rs` — replace `contracts_dir`
  parameter and `ContractPreviewEntry` type with a generic
  `ArtifactClass { staged_dir, baseline_dir, strategy }` slice supplied
  by the merge brief. `src/config.rs` — drop `ProjectConfig::specs_dir`
  and `ProjectConfig::contracts_dir` helpers; replace with a generic
  artefact-classes accessor that defers to the active capability
  manifest. Update callers in `commands/change.rs` and
  `crates/merge/tests/`.
- Acceptance: `cargo test -p specify-merge` green; `rg
  '"contracts"|"specs"' crates/merge crates/capability src/config.rs`
  returns nothing (outside tests).

### Wave 2.E — Init wires components, not capabilities (sequential)

#### 2.9 — Move platform-component scaffolding out of init (`cli`)

- Repo: `cli`
- Depends: 2.3, 2.5, 2.8
- Size: ~150 LOC
- Scope: `src/init.rs` and `src/commands/init.rs`. `specify init
  <capability>` records `capability:` in `project.yaml`. Scaffolding
  for `registry.yaml` is done by `specify registry add` (already
  dynamic). Scaffolding for `initiative.md` and `plan.yaml` happens
  via `specify initiative create` / `specify plan create` (already
  dynamic). Hub mode still writes `registry.yaml` because that's
  intrinsic to a hub's purpose.
- Acceptance: `specify init <capability>` writes only `project.yaml`
  (and `.specify/` skeleton); `specify init --hub` writes
  `project.yaml { hub: true }` plus an empty `registry.yaml`.

### Wave 2.F — Documentation (parallel × 1)

#### 2.10 — Component reference docs (`repo`)

- Repo: `repo`
- Depends: 2.3, 2.5 (so docs reflect landed surfaces)
- Size: ~300 lines markdown
- Scope: `docs/reference/registry.md` (registry topology + workspace
  materialisation), `docs/reference/change-component.md` (will be
  re-titled in Phase 3 — author it under the post-Phase-3 name to
  avoid a second rewrite). Cross-link from
  `docs/reference/capabilities.md`. Document the dependency direction
  and the platform-components-not-capabilities invariant.
- Acceptance: `make checks` passes; cross-links resolve.

### Phase 2 acceptance

- `specify-core` no longer depends on `specify-registry` or
  `specify-initiative` (invariant #4).
- `Commands::{Vectis, Contract}` are gone (invariant #3 enforced for
  these two surfaces).
- `validate_baseline_contracts` and `ContractPreviewEntry` are not in
  `specify` lib's public API.
- `/spec:define → /spec:build → /spec:merge` still passes on the
  canonical omnia slice (invariant #1) using the **pre-rename** noun
  set: `specify change *`, `.specify/changes/`, `$CHANGE_DIR`.

---

## Phase 3 — Lifecycle vocabulary cut-over (~400-700 LOC)

Renames the per-loop unit (`change → slice`) and the umbrella
orchestration noun (`initiative → change`) and ships the on-disk
migrations operator projects need.

### Wave 3.A — Slice rename (sequential within wave)

#### 3.1 — Rename `specify-change` crate to `specify-slice` (`cli`)

- Repo: `cli`
- Depends: Phase 2 complete
- Size: ~250 LOC + Cargo wiring
- Scope: `crates/change/` → `crates/slice/`; package name
  `specify-change` → `specify-slice`; re-exports through `specify`
  lib follow. Update `crates/initiative/Cargo.toml` and `cli`'s
  `Cargo.toml` workspace deps.
- Acceptance: `cargo build && cargo test -p specify-slice` green.
- Notes: This change frees the `specify-change` package name so
  Phase 3.4 can take it.

#### 3.2 — `Commands::Change → Commands::Slice` (`cli`)

- Repo: `cli`
- Depends: 3.1
- Size: ~250 LOC
- Scope: `src/cli.rs` (`ChangeAction` → `SliceAction`),
  `src/commands/change.rs` → `src/commands/slice.rs`,
  `src/commands/mod.rs`. Update outcome and journal verbs:
  `specify change outcome show` → `specify slice outcome show`,
  `specify change journal append` → `specify slice journal append`.
  Update help text.
- Acceptance: `specify slice *` works; `specify change *` is gone
  from the binary's `--help`.

#### 3.3 — Brief-substitution & internal-symbol sweep for slice (`cli` + `repo`)

- Repo: both
- Depends: 3.1, 3.2
- Size: ~200 LOC (Rust) + ~150 lines (markdown)
- Scope: replace `$CHANGE_DIR` → `$SLICE_DIR` in any in-binary string
  constants (`src/init.rs` template strings, brief loaders,
  diagnostics) and in skill markdown brief substitutions across
  `plugins/spec/`, `plugins/contract/`, `plugins/vectis/`,
  `plugins/omnia/`. Internal Rust symbols carrying "change" for the
  per-loop unit are renamed (`ChangeMetadata` → `SliceMetadata`,
  `change_actions` → `slice_actions`, etc.).
- Acceptance: `rg '\$CHANGE_DIR' plugins/ schemas/ ` returns nothing;
  `rg 'change' crates/slice/src/` only matches comments referencing
  the historical noun.

### Wave 3.B — Umbrella rename (sequential, after wave 3.A)

#### 3.4 — Rename `specify-initiative` crate to `specify-change` (`cli`)

- Repo: `cli`
- Depends: 3.1
- Size: ~200 LOC + Cargo wiring
- Scope: `crates/initiative/` → `crates/change/`; package name
  `specify-initiative` → `specify-change` (now free thanks to 3.1).
  Internal Rust symbols renamed (`InitiativeBrief` → `ChangeBrief`,
  `InitiativeFrontmatter` → `ChangeFrontmatter`, etc.).
- Acceptance: `cargo build && cargo test -p specify-change` green.

#### 3.5 — `Commands::Initiative → Commands::Change`; fold `Plan` under `Change` (`cli`)

- Repo: `cli`
- Depends: 3.2, 3.4
- Size: ~350 LOC
- Scope: `src/cli.rs` — drop `Plan` variant; `Initiative` becomes
  `Change` with a `ChangeAction` enum that nests
  `Create | Plan { action } | Execute | Finalize | Archive`. The
  inner `Plan` action keeps its existing verbs (`add`, `amend`,
  `next`, `status`, `doctor`, `lock`, `transition`, `archive`).
  `src/commands/initiative.rs` → `src/commands/change.rs`;
  `src/commands/plan/` becomes `src/commands/change/plan/`. Update
  `commands/mod.rs` dispatch.
- Acceptance: `specify change {create, plan {add,amend,next,status,
  doctor,lock,transition,archive}, execute, finalize, archive}`
  matches the durable post-RFC surface in §What becomes a capability.
  `specify plan *` is gone; `specify initiative *` is gone.

### Wave 3.C — On-disk migrations (parallel × 2)

#### 3.6 — `specify migrate slice-layout` (`cli`)

- Repo: `cli`
- Depends: 3.3
- Size: ~250 LOC + tests
- Scope: extend `src/commands/migrate.rs` with a `SliceLayout`
  variant. Renames `.specify/changes/` → `.specify/slices/` on disk,
  rewrites any in-tree `$CHANGE_DIR` substitutions in skill markdown
  to `$SLICE_DIR` (for projects that vendor skills locally).
  Idempotent. Refuses to run when an in-progress per-loop-unit
  carries an unfinished phase — operator must finish or drop the
  in-progress unit first. Diagnostic
  `slice-migration-blocked-by-in-progress`.
- Acceptance: tests in `tests/cli.rs` cover (a) clean run, (b)
  re-run no-op, (c) blocked by in-progress slice.

#### 3.7 — `specify migrate change-noun` (`cli`)

- Repo: `cli`
- Depends: 3.5
- Size: ~150 LOC + tests
- Scope: same migrate module. Renames `initiative.md` → `change.md`
  at the repo root. Idempotent. No on-disk changes to other platform
  artefacts (`registry.yaml`, `plan.yaml`, `contracts/` stay put per
  RFC-9 §1B).
- Acceptance: tests cover clean run and re-run no-op.

### Wave 3.D — Fixture & test fan-out (parallel × 3)

#### 3.8 — Refresh `cli` fixtures and integration tests (`cli`)

- Repo: `cli`
- Depends: 3.5, 3.6, 3.7
- Size: ~400 LOC
- Scope: `tests/change.rs` (now slice-loop tests), `tests/initiative.rs`
  (now change-orchestration tests, may rename file to
  `tests/change_umbrella.rs`), `tests/cli.rs`, `tests/e2e.rs`,
  `tests/fixtures/`. Replace every `change`/`initiative` test asset
  with the new vocabulary. Add migration round-trip tests.
- Acceptance: `cargo test --workspace` green.

#### 3.9 — Move `/spec:plan` and `/spec:execute` to the change surface (`repo`)

- Repo: `repo`
- Depends: 3.5
- Size: ~250 lines markdown
- Scope: rename `plugins/spec/skills/plan/` → `plugins/change/skills/plan/`
  (or wherever the change surface lands — coordinate with §5 of the
  Implementation Scope: "Move `plugins/spec/skills/plan/` and
  `plugins/spec/skills/execute/` to the change surface; keep any
  `/spec:plan` or `/spec:execute` material as a compatibility shim
  only"). Author the new commands; leave a one-line shim in the old
  locations that delegates and warns. The shim is removed before
  release per §Migration.
- Acceptance: invoking `/change:plan` and `/change:execute`
  authoritatively scopes / drives a plan; `/spec:plan` and
  `/spec:execute` warn and delegate.

#### 3.10 — Vocabulary sweep (slice / change) across `repo` (`repo`)

- Repo: `repo`
- Depends: 3.5
- Size: ~400 lines markdown
- Scope: every SKILL.md, reference, RFC, AGENTS.md, project.mdc that
  uses `change`/`initiative` in the pre-RFC sense. Apply the change →
  slice / initiative → change rename table from §Migration TL;DR.
  Special care: do **not** rewrite RFC-13 itself (it has the
  authoritative pre-rename history); do rewrite RFC-14 and the
  roadmap.
- Acceptance: `make checks` passes (it already enforces vocabulary
  consistency); the phrase "the change loop" no longer appears.

#### 3.11 — Drop `pipeline.plan` from the omnia capability manifest (`repo`)

- Repo: `repo`
- Depends: 3.9, 1.4
- Size: ~30 lines YAML + a few briefs moved
- Scope: `capabilities/omnia/capability.yaml` — drop the `plan:` block;
  move the discovery + propose briefs to the new change-planning skill
  added in 3.9. Tighten `capabilities/capability.schema.json` to forbid
  `pipeline.plan` (already loose-forbidden in 1.4 but enforce error
  with a clear message now that the change surface owns planning).
- Acceptance: `specify capability check` rejects manifests with
  `pipeline.plan`.

### Phase 3 acceptance

- `specify slice *` (per-loop unit) and `specify change {create, plan,
  execute, finalize, archive}` (umbrella) are the durable surfaces.
- `specify migrate slice-layout` followed by `specify migrate
  change-noun` upgrades a v1 omnia project to a working post-RFC
  layout that completes a canonical slice end-to-end.
- Invariant #1 still holds.

---

## Phase 4 — First-party capability migration (~500-900 LOC)

Moves domain mechanics out of the binary's command modules into
first-party capability skills + helper binaries. Capability skills
publish their full surface (`pipeline:` declared, briefs own validation
and adoption).

### Wave 4.A — Omnia full surface (parallel × 1)

#### 4.1 — Omnia capability skills publish full surface (`repo`)

- Repo: `repo`
- Depends: Phase 3 complete
- Size: ~250 lines markdown + a few helper scripts
- Scope: `capabilities/omnia/capability.yaml` (already minimal post-3.11);
  audit `plugins/omnia/skills/` so each pipeline brief id maps to a
  skill or reference. Document the merge-and-adoption contract usage:
  the merge brief must call `specify slice outcome set --phase merge
  --outcome {success,failed,blocked}` and may add journal entries via
  `specify slice journal append`.
- Acceptance: omnia define→build→merge fixture run produces the same
  artefacts as it did pre-RFC.

### Wave 4.B — Contracts and Vectis tool repackaging (parallel × 2)

#### 4.2a — Repackage contract validators as a standalone binary (`cli`)

- Repo: `cli`
- Depends: 2.7 (`Commands::Contract` already removed)
- Size: ~300 LOC + Cargo wiring
- Scope: create `crates/contract-validate/` with a `main.rs` that wraps
  `crates/validate/src/{contracts.rs, registry.rs}`. Build target
  `specify-contract-validate` (binary). Move RFC-12 SemVer +
  `info.x-specify-id` + cross-project uniqueness checks behind this
  binary. Drop the helpers from the `specify` binary's lib re-exports
  (already done in 2.7) and from `crates/validate`'s public API where
  no longer used.
- Acceptance: `specify-contract-validate <baseline-dir>` returns the
  same findings the old `specify contract validate` returned.

#### 4.3a — Repackage Vectis tooling as a standalone binary (`cli`)

- Repo: `cli`
- Depends: 2.6 (`Commands::Vectis` already removed)
- Size: ~250 LOC + Cargo wiring
- Scope: add a `[[bin]]` entry to `crates/vectis/Cargo.toml` so the
  five verbs (`init`, `verify`, `add-shell`, `update-versions`,
  `versions`) ship as `specify-vectis` (separate binary). Keep the
  library API for capability skills that prefer to call in-process.
- Acceptance: `specify-vectis verify` matches the legacy `specify
  vectis verify` JSON output byte-for-byte.

### Wave 4.C — Capability skills consume the new tools (parallel × 2)

#### 4.2b — Contracts capability skills call `specify-contract-validate` (`repo`)

- Repo: `repo`
- Depends: 4.2a, 4.1
- Size: ~200 lines markdown + a small helper script
- Scope: update `plugins/contract/skills/{openapi,asyncapi,json-schema}/verifier.md`
  to invoke `specify-contract-validate` for the post-merge cross-project
  consumer check. Update the contracts capability merge brief to record
  outcomes via `specify slice outcome set` and journal failures via
  `specify slice journal append`.
- Acceptance: a contracts-only slice runs the validator from the merge
  brief, blocks on failure, and surfaces the journal entries to the
  operator.

#### 4.3b — Vectis capability skills call `specify-vectis` (`repo`)

- Repo: `repo`
- Depends: 4.3a, 4.1
- Size: ~250 lines markdown
- Scope: update `plugins/vectis/skills/{core-writer, core-reviewer,
  ios-writer, ios-reviewer, android-writer, android-reviewer,
  design-system-writer, template-updater, test-writer}/SKILL.md` so
  validation, generation, and review behaviour route through the
  standalone `specify-vectis` binary or the library API. Update the
  Vectis capability merge brief to use the §Merge and adoption contract
  protocol.
- Acceptance: a Vectis end-to-end fixture run completes via the
  `vectis@v2` capability without any binary-side `specify vectis`
  surface.

### Wave 4.D — Optional follow-up (deferred to RFC-5)

#### 4.4 — RFC-13 invariant lints in `specify-check` (`cli`, deferred)

- Repo: `cli`
- Depends: 4.1, 4.2b, 4.3b
- Size: ~400 LOC
- Scope: implement the lints listed in RFC-13 §Migration:
  hard-coded-name lint (no first-party capability name literals in
  core crate sources outside tests), platform-components-outside-core
  lint (specify-core depends on neither specify-registry nor
  specify-change), first-party capability parity lint (bundled
  manifests pass every URL-resolved capability rule). Per RFC-13,
  this is RFC-5 design work and may land in a separate RFC.
- Acceptance: lints fire on intentional violation fixtures and stay
  quiet on the post-RFC tree. Because this is RFC-5 work, treat as
  out-of-scope for the initial RFC-13 landing and track separately.

### Phase 4 acceptance

- Bundled `omnia`, `contracts`, `vectis` capabilities each declare
  `pipeline:` and own their validation, generation, adoption, and
  cleanup behaviour through skills.
- `specify-cli` no longer carries first-party domain command modules
  for vectis or contracts.
- Platform components publish their own file formats and command
  contracts (`specify registry *`, `specify change *`) separately from
  the capability surface.
- All four invariants hold.

---

## Cross-cutting wave summary

```
Phase 0 — single subagent (0.1)
   |
   v
Phase 1
   wave 1.A: { 1.1 }                           (sequential, 1 subagent)
   wave 1.B: { 1.2, 1.3, 1.4, 1.5 }            (4 subagents in parallel)
   wave 1.C: { 1.6, 1.7, 1.8 }                 (3 subagents in parallel)
   |
   v
Phase 2
   wave 2.A: { 2.1 -> 2.2 -> 2.3 }             (sequential)
   wave 2.B: { 2.4 -> 2.5 } || from 2.A.2.1     (overlaps with 2.A from 2.1 onward)
   wave 2.C: { 2.6, 2.7 }                      (2 subagents in parallel after 2.A+2.B)
   wave 2.D: { 2.8 }                           (sequential after 2.C)
   wave 2.E: { 2.9 }                           (sequential after 2.D)
   wave 2.F: { 2.10 }                          (1 subagent, can run after 2.A+2.B)
   |
   v
Phase 3
   wave 3.A: { 3.1 -> 3.2 -> 3.3 }             (sequential)
   wave 3.B: { 3.4 -> 3.5 }                    (sequential, after 3.A)
   wave 3.C: { 3.6, 3.7 }                      (2 subagents in parallel after 3.B)
   wave 3.D: { 3.8, 3.9, 3.10, 3.11 }          (4 subagents in parallel after 3.C)
   |
   v
Phase 4
   wave 4.A: { 4.1 }                           (1 subagent)
   wave 4.B: { 4.2a, 4.3a }                    (2 subagents in parallel)
   wave 4.C: { 4.2b, 4.3b }                    (2 subagents in parallel after 4.B)
   wave 4.D: { 4.4 }                           (deferred to RFC-5)
```

## Notes for the coordinator

- **Always re-run the omnia smoke test** between waves. Invariant #1
  (omnia keeps working) is the single non-negotiable acceptance gate.
- **Each wave's subagents share a common base commit.** The
  coordinator merges the wave's branches once they all pass before
  starting the next wave; otherwise rebase storms are guaranteed.
- **Cross-repo waves require a deterministic order**: when a wave has
  both `cli` and `repo` changes, land `cli` first, regenerate
  fixtures consumed by skills, then land `repo`. The plan reflects
  this in the `Depends:` lines.
- **Fixture refreshes** are first-class change deliverables, not
  janitorial work. Schedule them inside the same wave as the
  surface they describe.
- **Migration commands** (3.6, 3.7) ship behind the existing `specify
  migrate` umbrella. Each takes its own kebab-case migration name and
  refuses to run when prerequisites aren't met.
- **Open Questions in RFC-13** stay open across this plan:
  - OQ-1 (structured merge diagnostics) — deliberately out of scope.
  - OQ-2 (in-progress slice migration) — handled by the "refuses to
    run" guard in 3.6.
  - OQ-3 (hub discriminator) — defer to RFC-14; 1.3 just lands the
    `hub: true` sentinel.
  - OQ-4 (validator distribution shape) — answered for contracts
    (standalone binary, 4.2a) and Vectis (standalone binary, 4.3a).
    Third-party capabilities are free to choose differently.
