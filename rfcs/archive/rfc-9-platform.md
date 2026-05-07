# RFC-9: Platform-First Operator Experience

> Status: Implemented · Depends: [RFC-1](rfc-1-cli.md), [RFC-2](rfc-2-execution.md), [RFC-3a](rfc-3a-monoliths.md), [RFC-3b](rfc-3b-platform.md), [RFC-8](rfc-8-api-contracts.md)
>
> This RFC is written against the v1 CLI surface ([migration map](../../docs/explanation/migrating-cli-v1.md)). Every verb shape referenced below — `specify change {validate, outcome set, journal append}`, top-level `specify {registry, initiative}`, and so on — assumes the post-v1 noun groupings. Pre-v1 shapes (`specify change phase-outcome`, `specify initiative brief …`, `specify initiative registry …`) no longer exist.
>
> **v2 layout note**: this RFC predates the v2 layout move (specify-cli `0.2.0`). Every reference below to `.specify/registry.yaml`, `.specify/plan.yaml`, `.specify/initiative.md`, and `.specify/contracts/` should be read as `registry.yaml`, `plan.yaml`, `initiative.md`, and `contracts/` at the repo root in current code. The hub topology, the `specify init --hub` flag, the registry add/remove verbs, and the `specify initiative finalize` archive sweep all behave identically — only the file locations changed. See [docs/explanation/whats-new.md](../../docs/explanation/whats-new.md) for the migration story.
>
> **RFC-14 supersession note**: this archive preserves the pre-RFC-14 design discussion for `workspace merge`, `--auto-merge`, and push-time branch creation. Current behavior is `workspace push` as transport-only PR publication, operator-owned PR merge, and `specify change finalize` as the read-only closure check; `specify workspace merge` is only a one-release non-zero shim if present.

## Abstract

Specify's ideal developer workflow is a **single-repo operator experience**: an operator works exclusively in a platform repo and Specify handles cloning, scaffolding, planning, executing, and pushing across all repos in the initiative. The mechanical infrastructure for this vision — workspace sync, greenfield bootstrapping, CWD-based execution routing, workspace push — is largely implemented across RFCs 1–3b and 8. However, several gaps prevent the experience from being end-to-end:

**Operator-experience gaps**

1. **No initiative umbrella.** The platform-first flow is five Layer 1/2/3 commands the operator must drive in sequence (`initiative init` → registry edit → `/spec:plan` → `/spec:execute --loop` → `workspace push` → manual PR merge). There is no `/spec:initiative create` Layer 4 verb that strings them together.
2. **Registry topology is manual.** The framework works with whatever the operator puts in `registry.yaml` but cannot propose, create, or modify registry entries as part of its analysis.
3. **The platform-repo hub pattern is hinted, not codified.** The data model already supports a registry-only platform repo, but no convention, scaffold, or tutorial pins whether the platform repo is itself a project (`url: .`) or a registry-only hub.
4. **Initiative landing has no closure verb.** `workspace push` ships the work; nothing observes the whole initiative as landed (all PRs merged, baselines committed, workspace clones pruned).

**Cross-repo coherence gaps**

1. **Contract federation is copy-based.** Central contracts are distributed by file copy during `workspace sync`; there is no version negotiation, breaking-change detection, or reconciliation across projects.

**Verb consistency gaps**

1. `**init` survives in two `specify` verbs.** `specify initiative init` and `specify plan init` are the only `init` verbs in the v1 surface; every other noun-create verb is `create`. Operators have to keep two near-identical verbs (`init` and `create`) in their head, and the 2C umbrella skill would have to remember which composite verb to call when shelling out.

**Housekeeping gaps**

1. **Stale fixtures reference removed schema fields.** Multiple execute fixtures still reference the `affects` field, which has been removed from the plan schema.
2. **The `PlatformConfig` trait is a stub.** The programmatic peer-resolution abstraction in `crates/platform/` returns `vec![]` unconditionally.
3. **No end-to-end multi-repo validation.** The full plan → execute → push path across multiple workspace clones has not been validated against a real platform initiative.
4. **Workspace-tier semantics are blurred.** Operator docs do not distinguish legacy-source clones (read-only, ephemeral, under `.specify/plans/<name>/analyze/<key>/`) from registered project clones (read-write, durable, under `.specify/workspace/<name>/`).

This RFC proposes a phased plan to close each gap, ordered by impact on the operator experience. Phase 4 also opportunistically picks up two RFC-2 §Future items (4B plan doctor, 4D fixture-backed verification) that fit naturally alongside the closure verbs.

## Motivation

The three-layer stack (CLI primitives → change lifecycle → initiative orchestration) composes cleanly in design. But when an operator sits down in a platform repo and says "migrate this legacy service" or "build a new feature that spans backend and mobile," several manual steps break the flow.

### The three initiative shapes

The platform-first workflow must support three initiative shapes through a single uniform loop:

1. **Migrate legacy.** Sources arrive via `--source <key>=<git-url-or-path>` and are cloned by `/spec:analyze` into `.specify/plans/<name>/analyze/<key>/` for shallow inventory; deep `/spec:extract` runs at define time. Targets are existing or newly-minted registered projects.
2. **New feature.** Sources arrive via `--from <docs>` only (or `initiative.md:inputs`); targets are existing registered projects, possibly with new ones spawned at assignment time.
3. **Update existing feature.** Sources are unused; targets are existing registered projects; baseline accumulation in `.specify/workspace/<peer>/specs/` is the dominant signal.

Each shape uses the same `/spec:plan → /spec:execute → workspace push → workspace merge` loop. The gaps in this RFC are gaps in the loop, not in the shape-handling.

### What the gaps look like in practice

- The operator must **manually author** `registry.yaml` entries for new projects before Specify can route work to them. For greenfield initiatives where the repo topology is itself a design decision, this front-loads a decision the framework should help make.
- The platform-first vision implies **one command** to start an initiative. Today it is five commands (`initiative init` → registry edit → `/spec:plan` → `/spec:execute --loop` → `workspace push`) plus N manual PR merges. Each step is correct in isolation; together they leak the operator out of the platform repo.
- The **platform repo's identity** is ambiguous. Tutorials and skills are agnostic about whether the platform repo is itself a code project (`url: .`) or a registry-only hub. The `/spec:init` flow does not distinguish the two; the choice is made implicitly by the first registry edit.
- Contracts are **copied** into workspace clones by `workspace sync`, but there is no mechanism to detect when a change in one project breaks a contract consumed by another. RFC-8 lands the contract format and role declarations; what's missing is the cross-project validation loop. (Cross-repo *spec* references — `@peer:capability` — are explicitly *not* a gap: contracts are the cross-repo boundary, and behavioural cross-references would re-couple consumer specs to producer internals. See *Non-goals*.)
- After `workspace push`, no Specify verb confirms **initiative landing**. The operator must check N PR pages, merge them manually, and then remember to `specify plan archive`.
- Fixture drift within the Specify repo itself — stale `affects` references in execute fixtures — creates confusion for contributors and risks agent behaviour divergence from the documented schema.

Closing these gaps in priority order transforms Specify from "infrastructure that supports the platform-first vision" to "a workflow that delivers it."

## Plan

### Phase 1: Housekeeping and confidence (low risk, high signal)

#### 1A. Fixture cleanup — remove stale `affects` references

**Problem.** The `affects` field was removed from the plan schema (supersession note in RFC-3a), but multiple execute *and* plan fixtures still contain it. The list below is **non-exhaustive** — the audit below sweeps both fixture trees in full. Spot-check hits, with paths relative to `plugins/spec/skills/`:

- `execute/fixtures/e2e-platform-v2/plan.yaml.before` and `.after`
- `execute/fixtures/e2e-platform-v2-with-crash/plan.yaml.before`, `.after`, `.after-crash`
- `execute/fixtures/loop/stuck-on-blocked/plan.yaml.before` and `.after`
- `execute/fixtures/dry-run/expected-output.md`
- `execute/fixtures/loop/stuck-on-blocked/transcript.md`
- `execute/fixtures/single-change/README.md`
- `execute/fixtures/e2e-platform-v2/README.md`
- `plan/fixtures/propose/transcript.md`
- `plan/fixtures/propose-vectis/transcript.md`

**Action.** Audit every fixture under `plugins/spec/skills/execute/fixtures/` and `plugins/spec/skills/plan/fixtures/`. Remove `affects:` entries from plan YAML fixtures. Update transcript and README references to use the current description-driven model. Extend `make checks` to flag any fixture YAML containing `affects:` as a schema-violation warning.

**Scope.** Specify repo only. No CLI changes.

#### 1B. Retire the `PlatformConfig` stub

**Problem.** `crates/platform/src/lib.rs` declares a `PlatformConfig` trait and `parse_platform_config` function that return `vec![]` unconditionally. The `PeerRepo` struct is unused. The real peer-resolution path goes through `Registry::load` directly.

**Action.** Evaluate whether the `PlatformConfig` abstraction is still needed. Two options:

- **(a) Remove it.** Delete the `specify-platform` crate, remove the `PlatformConfig` trait impl from `config.rs`, and update `lib.rs` re-exports. The `Registry` is the peer catalogue; no second abstraction is needed.
- **(b) Wire it.** Make `ProjectConfig` implement a meaningful `PlatformConfig` that delegates to `Registry::load`. This adds a layer of indirection without clear benefit — the registry is already the single source of truth.

**Recommendation.** Option (a). The stub was scaffolded before RFC-3a landed the registry. The registry subsumes its intended role. Removing it simplifies the crate graph.

**Scope.** specify-cli only.

#### 1C. End-to-end multi-repo validation

**Problem.** The plan → execute → push path across multiple workspace clones exists in skill definitions and fixtures but has not been validated against a real multi-project initiative.

**Action.** Author a **worked example** in `docs/tutorials/` that exercises the full path:

1. A platform repo with a two-project `registry.yaml` (one Omnia backend, one Vectis mobile app).
2. An `initiative.md` brief describing a feature that spans both.
3. `/spec:plan` producing a plan with entries assigned to both projects.
4. `/spec:execute --loop` driving define → build → merge across workspace clones.
5. `specify workspace push` creating branches and PRs.

The tutorial doubles as an integration test: if any step fails, the gap is in the implementation, not the design. Document discovered issues as items for subsequent phases. The tutorial must adopt the canonical platform-repo topology decided by 1D — using the "registry-only hub" pattern unless 1D rules otherwise — so the worked example is the reference, not a one-off.

**Scope.** Specify repo (tutorial + any discovered fixes). May surface CLI bugs.

#### 1D. Codify the platform-repo hub pattern

**Problem.** The `registry.yaml` data model is platform-scoped — it describes the platform, not the initiating repo — and supports a "registry-only hub" topology where the platform repo holds `registry.yaml`, `contracts/`, `initiative.md`, `plan.yaml`, and `workspace/` but is never itself a code project. RFC-3a's *Alternatives Considered* foreshadowed this. Today the convention is implicit: the operator may write `url: .` for the platform repo or omit it entirely, and tutorials make different choices. Without a canonical decision, the platform-first vision lacks an unambiguous starting shape.

**Action.** Decide the canonical topology and pin it across docs, scaffolding, and validation:

1. **Decision.** Adopt the **registry-only hub** as canonical. The platform repo holds platform state and never appears in its own `registry.yaml`. Code projects always live in their own repos, materialised under `.specify/workspace/<name>/`.
2. **Documentation.** Author `docs/explanation/platform-repo.md` describing the hub pattern, contrasting it with the "platform-as-project" pattern (still permitted for single-repo and small-team cases), and showing the on-disk shape of a hub.
3. **Scaffolding.** Add `specify init --hub` (or equivalent flag on `/spec:init`) that scaffolds a hub: writes `.specify/registry.yaml` with `version: 1` and `projects: []`, scaffolds `.specify/initiative.md` from the canonical template, and skips the per-project `project.yaml` rules block. A hub still has a `project.yaml` (so existing path helpers work) but with `schema: hub` reserved as a sentinel that disables phase pipelines on the hub itself.
4. **Validation.** Extend `Registry::validate_shape` with an opt-in `hub-only` mode: when enabled (e.g. via `project.yaml:hub: true`), reject `url: .` entries with a `hub-cannot-be-project` diagnostic. Non-hub projects keep the existing permissive validation.
5. **Tutorial alignment.** Update the cross-repo tutorial (`docs/tutorials/cross-repo-initiative.md`) and the `1C` worked example to use the hub pattern.

**Scope.** specify-cli (`init` flag, `Registry::validate_shape` extension, `ProjectConfig` `hub` field) + Specify repo (docs, tutorial alignment).

#### 1E. Document the two-tier workspace model

**Problem.** Operator docs blur two different "clones in the workspace" with different lifecycles and write semantics:


| Tier                     | Location                               | Lifecycle                                  | Writability                                             |
| ------------------------ | -------------------------------------- | ------------------------------------------ | ------------------------------------------------------- |
| Legacy-source clone      | `.specify/plans/<name>/analyze/<key>/` | Ephemeral; swept by `specify plan archive` | Read-only (analyze-only)                                |
| Registered project clone | `.specify/workspace/<name>/`           | Durable; persists across initiatives       | Read-write during execution; pushed by `workspace push` |


The two tiers serve different roles — analyze-only reading vs full define-build-merge — but the operator-facing language ("Specify clones the appropriate repo into its workspace") elides the distinction. This causes confusion when an operator expects `--source` clones to be writable, or expects registered projects to disappear after an initiative ends.

**Action.** Author `docs/explanation/workspace-tiers.md` covering:

1. The two-tier model (table above plus prose).
2. When each tier is materialised (`/spec:analyze` for tier 1; `specify workspace sync` for tier 2).
3. The lifecycle commands that affect each tier (`specify plan archive` for tier 1; `specify workspace sync`, `workspace push`, `workspace status` for tier 2).
4. Why the two tiers are not interchangeable — read-write writes from tier 1 would be lost; tier 2 is the only place generated code lives.

Cross-link from `docs/explanation/three-layer-stack.md`, the `/spec:plan` SKILL.md, and the `/spec:execute` SKILL.md so the model is discoverable from each entry point.

**Scope.** Specify repo only. Documentation; no CLI changes.

#### 1F. Rename `specify initiative init` to `specify initiative create`

**Problem.** The project's noun-create verbs are inconsistent. `specify change create` and `specify plan create` use `create`; `specify initiative init` uses `init`. This is incidentally consistent with `specify plan init`, but `plan` has *both* verbs (`plan init` scaffolds the file; `plan create` adds an entry) — `initiative` only has the file, so the `init` choice is gratuitous. The asymmetry leaks into the 2C umbrella skill: the skill verb is `/spec:initiative create <name>`, but it would shell out to `specify initiative init <name>`, forcing operators to remember two verbs for one act.

**Action.** Rename the CLI verb:

1. `InitiativeAction::Init { name }` → `InitiativeAction::Create { name }` in `src/cli.rs`.
2. The handler in `src/commands/initiative.rs` keeps its current behaviour (refuse-if-exists, kebab-case validation, template write).
3. Update the v1 migration map (`docs/explanation/migrating-cli-v1.md`) with a v1.x rename row: `specify initiative init <name>` → `specify initiative create <name>`. Tag it as a v1.x evolution rather than a v1 cleanup rename so operators reading the doc can tell the two waves apart.
4. Update every reference in skills, docs, tutorials, and fixtures (`docs/reference/cli/initiative.md`, plan/execute SKILL.md, AGENTS.md, README.md, `.cursor/rules/project.mdc`).

The verb rename is mechanical and behaviour-preserving. No flag changes, no JSON shape change, no `.specify/initiative.md` template change.

**What about `specify plan init`?** Handled in §1G. Renaming `plan init` alone collides with the existing `plan create`, so §1G renames both verbs in the same change (`init` → `create` for the file scaffold, `create` → `add` for the entry append). 1F and 1G ship together to avoid an interim state where `initiative` is consistent but `plan` still uses `init`.

**Scope.** specify-cli (`InitiativeAction` rename + handler signature) + Specify repo (skill, doc, tutorial, fixture updates + migration-map entry).

#### 1G. Rename `specify plan init` to `create` and `specify plan create` to `add`

**Problem.** With 1F landing `specify initiative create`, `specify plan init` is the only remaining `init` verb in the v1 surface. 1F declined the rename because `plan create` already exists (it appends a change entry to the plan), but the collision is solvable: rename both verbs in the same change. The file-creating verb takes the canonical `create`, and the entry-appending verb adopts `add` — matching `specify registry add` (2A) and the convention that child-add verbs use `add`. After 1G:

- `specify plan create <name>` scaffolds `.specify/plan.yaml` (was `plan init`).
- `specify plan add <entry>` appends a change entry to the plan (was `plan create`).

**Action.** Apply both renames in one change:

1. `PlanAction::Init { name, sources }` → `PlanAction::Create { name, sources }` in `src/cli.rs`.
2. `PlanAction::Create { name, project, description, depends_on, sources, affects }` (entry-append variant) → `PlanAction::Add { ... }`. Flag shapes and JSON output do not change.
3. Rename the matching dispatch arms in `src/commands/plan/mod.rs` and the underlying lifecycle helpers (`run_plan_init` → `run_plan_create`; the entry-append helper → `run_plan_add`). Handlers keep their current behaviour — refuse-if-exists for `create`, append-with-validation for `add`.
4. Add two v1.x rows to the v1 migration map (`docs/explanation/migrating-cli-v1.md`): `specify plan init <name>` → `specify plan create <name>`, and `specify plan create <name>` → `specify plan add <name>`. Tag both as v1.x evolution alongside 1F's row.
5. Audit and update every reference in skills, docs, tutorials, fixtures, and project rules. Non-exhaustive list: `docs/reference/cli/plan.md`, `docs/reference/initiative-skills/plan.md`, `docs/reference/quick-reference.md`, `docs/reference/configuration.md`, `docs/appendices/glossary.md`, `plugins/spec/skills/{plan,execute,merge,define,build}/SKILL.md`, `schemas/{omnia,vectis}/briefs/plan/propose.md`, plan-skill propose fixtures (`plan/fixtures/propose/`, `plan/fixtures/propose/monolith/`, `plan/fixtures/propose-vectis/`), execute fixture READMEs (`execute/fixtures/e2e-platform-v2/README.md`), `AGENTS.md`, `README.md`, `.cursor/rules/project.mdc`. Every occurrence of `plan init` and `plan create` in markdown and yaml gets reviewed.
6. **Within-RFC references — already updated.** This RFC ships with the post-rename surface in 2A, 2B, and 2C: 2A's validation-ordering invariant uses `specify plan add --project`, 2B's greenfield prompt uses `specify plan create`, and 2C's Layer 3 → Layer 1 pattern uses `specify plan {create, add}`. Implementers do not need to repeat the within-RFC update — only the repo-wide audit in step 5 remains.

**Why both at once.** Renaming `plan init` → `plan create` alone is impossible (the name is taken). Renaming `plan create` → `plan add` alone leaves the file-creating verb still called `init`, defeating the consistency win. The two renames must land in the same change so operators learn the new surface once.

**Scope.** specify-cli (`PlanAction::{Init, Create}` rename + handler signatures + tests + clap derive output) + Specify repo (skill, doc, tutorial, fixture, project-rule updates + two migration-map rows + the within-RFC references called out in step 6). Ships together with 1F.

---

### Phase 2: Dynamic registry management (high impact)

#### 2A. `specify registry add`

**Problem.** The operator must manually edit `registry.yaml` to add new projects. There is no CLI verb for creating or modifying registry entries.

**Action.** Add a `specify registry add` verb:

```text
specify registry add <name> \
    --url <url> \
    --schema <schema> \
    [--description "..."]
```

Semantics:

- Validates `name` as kebab-case, `url` via existing `validate_project_url`, `schema` as non-empty.
- Appends to `registry.yaml`; creates the file (with `version: 1`) if absent.
- Enforces the `description-missing-multi-repo` invariant: if the addition creates a multi-project registry and any existing project lacks a `description`, the verb fails with a diagnostic telling the operator to add descriptions to existing entries first.
- Runs `validate_shape` after the write.

Complementary verb: `specify registry remove <name>` — removes an entry, validates shape, warns if plan entries reference the removed project.

**Validation ordering invariant.** `specify plan add --project <name>` and `specify plan amend --project <name>` continue to reject unknown projects (RFC-3b §Validation; verb names per §1G). Any consumer that wants to assign work to a new project must therefore call `specify registry add` and `specify workspace sync` *before* the corresponding plan write. The 2B registry-proposal sub-step and the 2C umbrella skill must respect this ordering.

**Scope.** specify-cli: new `RegistryAction` variants (`Add`, `Remove`) added to `src/commands/registry.rs`, new tests covering kebab-case validation, URL classification, multi-project description enforcement, and round-tripping through `Registry::load`.

#### 2B. Plan skill proposes new registry entries

**Problem.** When `/spec:plan`'s discovery or propose phase identifies a capability that does not fit any existing registry project (or when no registry exists), it has no mechanism to suggest creating a new project.

**Action.** Extend the plan skill's assignment step (3d) with a **registry proposal** sub-step:

1. After inference, if any entry is tagged `unresolved` and the operator's override creates a project name that does not exist in the registry, prompt: "Project `<name>` does not exist in registry.yaml. Create it?"
2. If accepted, gather `url` (default: `git@github.com:<org>/<name>.git` inferred from existing registry entries' URL patterns) and `schema` (default: the schema used by the majority of existing entries, or prompted if ambiguous).
3. Shell out to `specify registry add <name> --url <url> --schema <schema> --description "<inferred>"`.
4. Run `specify workspace sync` to bootstrap the new slot.
5. Continue assignment with the new project available.

For **greenfield** initiatives where no registry exists at all, the discovery brief should propose an initial registry based on the capability decomposition: "These capabilities cluster into N groups; I recommend N projects with these boundaries." The operator reviews and approves before `specify plan create` runs (verb name per §1G; was `plan init`).

**Execute-time amendment path.** Registry amendments are not always foreseeable at plan time — a build brief may discover that a capability is misrouted, or `/spec:extract` may surface tangled code that should split into a new repo. To make this recoverable without leaving the platform-first flow:

1. Phase skills can emit a `registry-amendment-required` phase outcome with a structured payload `{ proposed-name, proposed-url, proposed-schema, proposed-description, rationale }`. `specify change outcome set` is extended to accept this outcome shape.
2. The execute driver classifies `registry-amendment-required` as `blocked` (existing classification), records the payload in the plan journal via `specify change journal append`, and surfaces the proposal to the operator at the end of the change.
3. The operator reviews the proposal and runs `specify registry add` (or accepts via 2C's umbrella skill), then `specify workspace sync`, then `specify plan amend <change> --project <new>`, then `specify plan transition <change> pending` to re-queue.
4. The recovery sequence is documented in the execute skill guardrails as the canonical `registry-amendment-required` recovery path.

This keeps phase skills unaware of registry mechanics (they just emit the outcome) and keeps the registry under operator control (the driver never auto-adds projects).

**Scope.** Plan skill SKILL.md amendments + Omnia/Vectis propose brief updates + execute SKILL.md guardrails section + `specify change outcome set` accepting the new outcome shape. Adding a fourth `Outcome` variant is a wire-format change: today `crates/change/src/lib.rs::Outcome` is closed (`Success` / `Failure` / `Deferred`), and `.metadata.yaml:outcome` round-trips it via serde. The new variant therefore bumps the change-metadata schema version and needs a back-compat read path for archived metadata. CLI verb from 2A is a prerequisite.

#### 2C. `/spec:initiative` umbrella skill

**Problem.** The platform-first vision implies "create an initiative" as a single operator action. Today it is five distinct commands plus N manual PR merges (see *Motivation*), each correct in isolation but together leaking the operator out of the platform repo. There is no Layer 3 skill *above* `/spec:plan` and `/spec:execute` that strings them into one experience.

**Action.** Add a new Layer 3 skill, `/spec:initiative`, that orchestrates the full platform-first loop:

```text
/spec:initiative create <name> \
    [--shape migrate-legacy | new-feature | update-existing] \
    [--from <path>...] \
    [--against <path>] \
    [--source <key>=<path-or-url>...] \
    [--auto-merge] \
    [--dry-run]
```

The skill verb (`create`) matches the renamed CLI verb (`specify initiative create`, see §1F). "Create at the orchestration layer calls create at the primitive layer" mirrors the existing Layer 3 → Layer 1 pattern (`/spec:plan` → `specify plan {create, add}`, verb names per §1G).

Internally, the skill drives the canonical loop:

1. **Brief.** If `.specify/initiative.md` is absent, run `specify initiative create` (1F) and prompt the operator to fill it (or accept defaults inferred from `--shape` and CLI flags).
2. **Registry.** Run `specify registry validate`. If the registry is multi-project, ensure every entry has a `description` (2A invariant). If `--shape` is `new-feature` or `migrate-legacy` and the registry is empty, prompt for an initial topology (2B greenfield path).
3. **Plan.** Invoke `/spec:plan <name>` with the forwarded `--from` / `--against` / `--source` flags. If `--dry-run` was passed, stop after the plan-skill's own dry-run preview.
4. **Execute.** Invoke `/spec:execute --loop`. Halts at the same points the underlying skill halts (self-heal, stuck, `registry-amendment-required` from 2B).
5. **Push.** Run `specify workspace push`.
6. **Land.** When `--auto-merge` is supplied, run `specify workspace merge` (4A) to merge PRs whose CI is green. Without `--auto-merge`, surface the open PR list and stop.
7. **Finalize.** When all PRs are merged, run `specify initiative finalize` (4C) to archive the plan and (optionally) prune workspace clones.

**Composition discipline.** The umbrella skill *only* invokes other Layer 1/2/3 skills and CLI verbs — no new logic. Every step has a manual-fallback equivalent (the existing skill or CLI command) so the operator can always drop down a layer. This is the same composition principle RFC-2 applied to `/spec:execute`'s relationship with `/spec:define` / `/spec:build` / `/spec:merge`.

**Verb-naming discipline.** Compose using v1 verb names verbatim — `specify change {validate, outcome set, journal append}`, top-level `specify {registry, initiative}`, and so on. The pre-v1 shapes (`specify change phase-outcome`, `specify change journal-append`, `specify initiative brief …`, `specify initiative registry …`) no longer exist. When this skill lands, double-check every shell-out against the [v1 migration map](../../docs/explanation/migrating-cli-v1.md); muscle memory for `phase-outcome` in particular dies hard.

**Three-shape acceptance criteria.** A successful 2C lands when the skill cleanly handles all three initiative shapes (Motivation §*The three initiative shapes*). The 1C tutorial is extended with a transcript per shape: a migrate-legacy transcript with `--source monolith=<git-url>`, a new-feature transcript with `--from ./docs/`, and an update-existing transcript with neither.

**Status of `/spec:plan` and `/spec:execute`.** Both remain operator-facing. `/spec:initiative` is the recommended entry point but the lower skills are still callable directly for power users, partial reruns, and CI pipelines.

**Scope.** Specify repo: new skill `plugins/spec/skills/initiative/SKILL.md`, new fixtures under `plugins/spec/skills/initiative/fixtures/`, three-layer-stack documentation update to introduce a "Layer 4" (initiative orchestration) above existing Layer 3 (or rename the layers — see §*Open question* below). CLI: no new verbs (the `init` → `create` renames are in 1F and 1G). Depends on 1C (worked-example tutorial — three-shape acceptance criteria extend it), 1F (`initiative create` rename), 1G (`plan {create, add}` rename), 2A (registry mutation), 2B (registry proposal), 4A (`workspace merge`, optional via `--auto-merge`), 4C (`initiative finalize`).

> **Open question.** The current three-layer stack labels the plan/execute skills as "Layer 3 — Initiative Orchestration." If `/spec:initiative` sits above them, either (a) rename Layer 3 to "Plan & Drive" and introduce Layer 4 "Initiative Orchestration," or (b) absorb `/spec:initiative` into Layer 3 alongside `/spec:plan` and `/spec:execute`, treating it as an aggregator within the same layer. Decide as part of 2C implementation; the docs change is small either way.

---

### Phase 3: Cross-repo coherence (medium impact)

#### 3B. Cross-project contract validation

**Problem.** RFC-8 lands contract roles (`produces`/`consumes`/`imports`) in the registry and a `contracts` brief in the define pipeline. But validation is single-project: the `contracts:validator` skill checks internal consistency within one project's view. There is no mechanism to detect when a change in the producer project breaks a contract consumed by another project.

**Action.** Extend the contracts validation pipeline with a **cross-project compatibility check**:

1. After merge in a producer project, the execute driver reads the project's `produces` list from `registry.yaml`.
2. For each produced contract, identify consumer projects (those listing the same path in `consumes`).
3. Run `contracts:validator` against each consumer's workspace clone with the updated contract, checking that the consumer's specs remain compatible.
4. Surface incompatibilities as warnings in the merge transcript. The execute driver does not halt on cross-project contract warnings — the operator triages them — but the warnings are written to the plan journal for auditability.

**Scope.** Contracts plugin: new cross-project validation mode. Execute skill: post-merge validation step. CLI: no changes (validation runs through existing skill infrastructure).

---

### Phase 4: Autonomy and polish

#### 4A. `specify workspace merge` — automated PR merging

**Problem.** RFC-3b §Non-goals explicitly defers automated PR merging. `workspace push` creates PRs; merging is manual. For fully autonomous execution, the driver should be able to merge PRs after CI passes.

**Action.** Add `specify workspace merge [<project>...]`:

1. For each project with an open PR on the `specify/<initiative-name>` branch, check CI status via `gh pr checks`.
2. If all checks pass, merge via `gh pr merge --squash`.
3. Report per-project merge status.
4. Respect `--dry-run`.

Guard: only merge PRs that were created by `workspace push` (match branch name pattern `specify/<initiative-name>`). Never force-merge or merge PRs with failing checks.

**Scope.** specify-cli: new `WorkspaceAction::Merge` variant. Requires `gh` (same constraint as `workspace push`).

#### 4B. Plan doctor — extended diagnostics

**Problem.** RFC-2 §Future defers `specify plan doctor` — extended plan health checks beyond what `validate` covers (e.g., circular dependencies, orphan sources, stale workspace clones, unreachable entries).

**Action.** Implement `specify plan doctor` as a superset of `specify plan validate` that adds:

- Cycle detection in `depends-on` graph (currently, `next_eligible` silently skips cycles).
- Orphan source keys (defined in top-level `sources` but unreferenced by any entry).
- Stale workspace clones (registry entry changed since last sync).
- Unreachable entries (entries whose dependencies can never be satisfied due to `failed`/`skipped` predecessors).

**Scope.** specify-cli: new command. Specify repo: document in execute skill guardrails.

#### 4C. `specify initiative finalize` — initiative landing closure

**Problem.** `workspace push` ships local commits; `workspace merge` (4A) lands the PRs; `specify plan archive` sweeps local plan state. But no single verb confirms an initiative as **fully landed** — all per-project PRs merged on remote, all baselines committed to mainline, workspace clones optionally pruned. The operator must check each forge's PR page, then remember to archive the plan.

**Action.** Add `specify initiative finalize`:

```text
specify initiative finalize \
    [--clean]      # remove .specify/workspace/<peer>/ clones after archive
    [--dry-run]
```

Per-initiative algorithm:

1. **Plan presence.** Load `.specify/plan.yaml`. Refuse if absent (initiative already finalized) or any entry is not in a terminal state (`done` / `failed` / `dropped`). Diagnostic points the operator at `specify plan status`.
2. **Per-project landing check.** For each registry project that has a `specify/<initiative-name>` branch on its remote, query `gh pr view --json state,merged` (or equivalent) to confirm the PR is `MERGED`. Open or unmerged PRs surface as a per-project blocker; finalize exits non-zero with the list.
3. **Workspace cleanliness.** For each workspace clone, refuse if `git status --porcelain` is non-empty (uncommitted work would be lost on `--clean`). Diagnostic suggests `workspace push` or manual triage.
4. **Archive.** Run `specify plan archive` to sweep `plan.yaml`, `.specify/initiative.md`, and `.specify/plans/<name>/` into `.specify/archive/plans/<YYYYMMDD>-<name>/`.
5. **Optional clean.** When `--clean` is supplied and step 3 passed, remove `.specify/workspace/<peer>/` clones. Without `--clean`, clones stay on disk for the operator's reference (and for the next initiative — they're cheap to refresh via `workspace sync`).

Output format mirrors `workspace push`: per-project status (`merged`, `unmerged`, `no-branch`, `failed`) plus a summary line. JSON output adds `"initiative": "<name>"` and `"finalized": true|false` at the top level.

**Recovery from a partially-landed state.** If finalize fails because some PR is unmerged, the operator merges it manually (or via `workspace merge`) and re-runs `finalize`. Partial archives are not created — finalize is all-or-nothing on the archive write.

**Scope.** specify-cli: new `InitiativeAction::Finalize` variant, new handler. Depends on 4A (`workspace merge`) only logically — finalize can run after manual merges. The 2C umbrella skill calls finalize as its terminal step.

#### 4D. Fixture-backed verification mode

**Problem.** RFC-2 §Future defers "fixture-backed verification" — a mode where `/spec:verify` compares live behavior against captured fixtures to detect drift post-migration. Without it, the replay-writer's fixtures are only useful as one-shot integration tests, not as ongoing regression guards.

**Action.** Design a verification mode that:

1. Accepts a fixture directory (the replay-writer's output format).
2. Replays each fixture against the migrated service's API.
3. Compares responses against expected outputs, allowing configurable tolerance (e.g., ignore timestamps, allow additional fields).
4. Reports drift as a verify-level diagnostic.

**Scope.** New skill definition. May require CLI extensions for the verify pipeline.

## Implementation order


| Phase | Item                                    | Depends on                 | Effort | Impact                     |
| ----- | --------------------------------------- | -------------------------- | ------ | -------------------------- |
| 1     | 1A. Fixture cleanup                     | —                          | S      | Contributor confidence     |
| 1     | 1B. Retire `PlatformConfig` stub        | —                          | S      | Crate graph simplification |
| 1     | 1D. Codify platform-repo hub pattern    | —                          | M      | Topology clarity           |
| 1     | 1E. Document two-tier workspace model   | —                          | S      | Operator clarity           |
| 1     | 1F. Rename `initiative init` → `create` | —                          | S      | Verb consistency           |
| 1     | 1G. Rename `plan init`/`plan create`    | 1F (ship together)         | S      | Verb consistency           |
| 1     | 1C. E2E multi-repo tutorial             | 1D                         | M      | Validation + documentation |
| 2     | 2A. `registry add/remove` CLI           | —                          | M      | Operator UX                |
| 2     | 2B. Plan skill registry proposals       | 2A                         | M      | Autonomous topology        |
| 2     | 2C. `/spec:initiative` umbrella skill   | 1C, 1F, 1G, 2A, 2B, 4A, 4C | M      | Single-command initiative  |
| 3     | 3B. Cross-project contract validation   | RFC-8                      | L      | Contract safety            |
| 4     | 4A. `workspace merge`                   | —                          | M      | Full autonomy              |
| 4     | 4B. Plan doctor                         | —                          | M      | Diagnostic depth           |
| 4     | 4C. `initiative finalize`               | —                          | M      | Initiative closure         |
| 4     | 4D. Fixture-backed verification         | —                          | M      | Ongoing regression         |


Effort: S = 1–2 days, M = 3–5 days, L = 1–2 weeks.

**Critical path for the platform-first vision.** The shortest path from today's state to "operator never leaves the platform repo" is:

1. **1D** (decide hub pattern) → unblocks 1C and gives 2C a canonical scaffold target.
2. **1C** (E2E multi-repo tutorial) → exercises the loop end-to-end and produces the worked example that 2C's three-shape acceptance criteria extend.
3. **1F** (`initiative create` rename) → trivial CLI rename so 2C can shell out symmetrically (`/spec:initiative create` → `specify initiative create`).
4. **1G** (`plan {create, add}` rename) → eliminates the last `init` verb so 2C composes against a single consistent surface; ships with 1F.
5. **2A** (`registry add/remove`) → unblocks 2B and 2C.
6. **2B** (plan-skill registry proposals) → makes the assignment step able to mint projects.
7. **4A** (`workspace merge`) → closes the upstream-landing half of the loop.
8. **4C** (`initiative finalize`) → closes the local-archive half of the loop.
9. **2C** (`/spec:initiative` umbrella) → composes 1C, 1D, 1F, 1G, 2A, 2B, 4A, 4C into a single operator-facing verb.

The other items (1A/1B/1E housekeeping, 3B coherence, 4B plan doctor, 4D fixture-backed verification) improve the experience but do not block the headline vision. Phase 1 housekeeping is safe to start immediately. The critical-path items can be driven as Specify initiatives themselves — authored via `/spec:plan` and executed via `/spec:execute --loop` — which would simultaneously validate the platform-first workflow and close the gaps in it.

## Non-goals

- **Non-GitHub forge support.** `workspace push`, `workspace merge`, and `initiative finalize` use `gh`. GitLab/Bitbucket/self-hosted support is a separate concern.
- **Multi-plan output.** RFC-3a's single `plan.yaml` in the initiating repo is preserved. Per-repo plans are not proposed.
- **Inferring project descriptions.** Registry project descriptions remain operator-authored. The plan skill may *propose* descriptions for new entries, but the operator always reviews.
- **Auto-creating registry entries.** Even with 2B's registry-proposal sub-step and 2B's execute-time amendment path, registry mutations always pass through operator confirmation. The framework never silently adds, removes, or modifies `registry.yaml` entries.
- **Mandatory hub pattern.** 1D codifies the registry-only hub as canonical, but the platform-as-project shape (`url: .` on the initiating repo) remains valid for single-repo and small-team use. The hub pattern is a recommendation, not an enforcement.
- **Full behavioural diff.** RFC-2 §Future's "behavioural diff" between pre- and post-migration services is undesigned and out of scope.
- **Cross-repo spec-body references.** `@peer:capability` syntax (and any other spec-body reference that names a peer project's internal capabilities) is out of scope. Contracts (RFC-8) are the cross-repo boundary: a consumer declares the contracts it depends on via `consumes`/`imports` in `registry.yaml`, and §3B closes the wire-level compatibility loop. Cross-project sequencing belongs to `depends-on` edges in `plan.yaml` (RFC-2/RFC-3a). A `@peer:capability` syntax in spec bodies would re-couple consumer specs to producer internals — exactly the coupling contracts were introduced to remove. RFC-8's "Why not `@peer:capability`?" carved out a residual "planning and ordering" use; that residual is already covered by `depends-on`, so this RFC closes the door on the syntax entirely.

## References

- [RFC-1: `specify` CLI](rfc-1-cli.md)
- [RFC-2: Execution](rfc-2-execution.md)
- [RFC-3a: Initiative Planning](rfc-3a-monoliths.md)
- [RFC-3b: Platform Changes](rfc-3b-platform.md)
- [RFC-8: API Contracts](rfc-8-api-contracts.md)
- [Three-Layer Stack](../../docs/explanation/three-layer-stack.md)
- [Migrating to CLI v1](../../docs/explanation/migrating-cli-v1.md)

