# RFC-25: Source Profiles

> Status: Draft - Depends: [RFC-13](archive/rfc-13-extensibility.md), [RFC-20](rfc-20-survey.md), [RFC-21](rfc-21-catalogue.md), [RFC-23](archive/rfc-23-change-lifecycle.md)

## Abstract

Introduce **source profiles** as Layer 2's source-side customisation axis, parallel to (and orthogonal to) Layer 1's target-capability axis. A source profile (`typescript-node`, `csharp-dotnet`, `cobol-mvs`, …) bundles the brief content, detector pack, and per-language conventions needed to analyse and survey a legacy source tree. A profile binds at the source level (via `sources.yaml`'s declaration or auto-detection), not at the change level.

This RFC adds:

1. **`source-profiles/<name>/profile.yaml`** — a profile manifest, parallel to `capabilities/<name>/capability.yaml`, that declares the briefs a profile owns (`analyze.md`, optional `survey.md`, optional `extract.md`).
2. **`specify source-profile {resolve, list, validate}`** — a new CLI verb family, parallel to `specify capability {resolve, pipeline}`.
3. **A `profile` field on `sources.yaml:sources[]`** — explicit binding of a source key to a profile name; defaults are inferred by auto-detection when absent.
4. **A profile-axis split for the planning briefs** — `plugins/change/skills/draft/briefs/<capability>/analyze.md` loses its source-side prose; equivalent prompts move to `source-profiles/<profile>/briefs/analyze.md`. The target-capability brief retains target-shape constraints (`one crate per capability`, `vectis composition`, …) only.
5. **A profile-keyed detector registry for RFC-20 `specify change survey`** — the v1 single global registry is generalised to a `profile -> detector-pack` map. v1 first-party profiles ship the detectors RFC-20 originally listed (Express, NestJS, BullMQ) under `source-profiles/typescript-node/`.

These additions are **strictly additive** at the operator level — a project that does not declare profiles continues to work via auto-detection against a `default` profile that carries today's TS-flavoured content. They are **schema-breaking** for capability authors who currently ship a `briefs/<cap>/analyze.md` file: the per-kind prose moves out of the capability brief into the matching profile.

## Motivation

The framework already has a clean axis for **target customisation** at Layer 1: `capabilities/{omnia,vectis,contracts}/` bind the `define -> build -> merge` slice loop to a target-framework-aware set of briefs. The pattern is well-trodden: a kebab-case manifest, a `briefs/` directory, a CLI resolver, and a `specify capability resolve` lookup the skills consult before they load a brief.

Layer 2 (`/change:draft`, `/change:analyze`, RFC-20's `/change:survey`) has no equivalent axis for **source customisation**, despite reading and clustering source code being its central job. Today's structure conflates the two axes:

- `plugins/change/skills/draft/briefs/<capability>/analyze.md` is keyed by the **target** capability (`omnia`, `vectis`), but its contents embed **source-side** heuristics — TS-style clustering signals (`Modules that cluster in a tight import SCC`), TS-style language detection (`detected primary source language, kebab-case: typescript, javascript, rust, …`), TS-style LOC conventions (`Exclude vendored dependency directories (node_modules, vendor, target, .venv)`). A non-TS source (COBOL copybooks, JCL, C#'s csproj graph, Java's Maven multi-module) needs different prompts, not different target conventions.
- RFC-20 explicitly defers the per-axis detector pack: *"v1 is a single global registry. Per-capability detector packs at `plugins/change/skills/survey/briefs/<cap>/detectors/` are explicitly deferred"* — but even the deferred path keys by `<cap>` (target), not by source language. RFC-20 cannot proceed with sorting this without changing its v1 scope.
- `/spec:extract` (Layer 1) consumes the same legacy source trees at slice-define time. Its language-aware steps (parse imports, locate handlers, follow public API edges) are hand-written for TS/Python/Go shapes today, with no axis at all. A future COBOL or C# extract pass needs the same source-profile resolver Layer 2 will grow.

Without a source axis, every new source-language adoption costs:

- A merge into every capability's `analyze.md` (currently 2 files; will be N for N first-party capabilities).
- A new branch in every RFC-20 detector dispatch site, hard-coded against language signals.
- A scattered set of edits in `/spec:extract`'s reference files (`business-logic.md`, `component-structure.md`, `dependencies.md`, `external-api.md`, `observability.md`).

Adding the source axis once means new-language adoption becomes a single new directory under `source-profiles/<name>/`, with explicit manifest and brief contracts.

## Core Idea

**Source profile is to Layer 2 what capability is to Layer 1.**

| Axis             | Layer 1 (`/spec`)                                       | Layer 2 (`/change`)                                                          |
| ---------------- | ------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Customises       | The **target** framework the slice lands in.            | The **source** code/documentation shape that feeds planning and extraction.  |
| Concept          | Capability.                                             | Source profile.                                                              |
| Directory        | `capabilities/<name>/`.                                 | `source-profiles/<name>/`.                                                   |
| Manifest         | `capability.yaml`.                                      | `profile.yaml`.                                                              |
| Briefs           | `briefs/{proposal,specs,design,tasks,build,merge}.md`.  | `briefs/{analyze,survey,extract}.md` (analyze required; others optional).    |
| CLI resolver     | `specify capability {resolve, pipeline}`.               | `specify source-profile {resolve, list, validate}`.                          |
| Bound at         | Project level (`.specify/project.yaml:capability`).     | Source level (`sources.yaml:sources[].profile`).                             |
| Per-instance     | One capability per project.                             | One profile per source key (a change may mix many profiles).                 |
| Auto-detection   | None — `capability` is operator-declared.               | Yes — profile defaults to the result of an auto-detect scan of the source.   |
| First-party set  | `omnia`, `vectis`, `contracts`, `default`.              | `typescript-node`, `default` in v1; `csharp-dotnet`, `cobol-mvs`, etc. later. |

The two axes **compose orthogonally** at every analyse/survey/extract dispatch site:

1. The skill resolves the **source profile** (per input or per source key) → loads the source-side brief from `source-profiles/<profile>/briefs/`.
2. The skill resolves the **target capability** (per project / per change) → loads any target-shape brief from `plugins/change/skills/draft/briefs/<capability>/` (propose, sync-workspace, assignment) or `capabilities/<capability>/briefs/` (define / build / merge).
3. The two briefs collaborate via a small set of **handoff contracts** that already exist (`discovery.md` `## Candidate inventory` heading, `surfaces.json`, the `analyze/<source-key>/metadata.json` sidecar). Neither brief reads the other's prose.

The dispatch table for a multi-source change is therefore a Cartesian fan-out **on the source axis only**: `/change:analyze` runs once per source, each resolving to one profile; `/change:survey` (RFC-20) runs the same way. Propose still runs once per change against the single target capability.

## Design

### `profile.yaml` — the source profile manifest

Schema: `specify-cli/schemas/source-profile/profile.schema.json` (parallel to `capability.schema.json`).

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/augentic/specify-cli/main/schemas/source-profile/profile.schema.json
name: typescript-node
version: 1
description: Node-flavoured TypeScript / JavaScript sources (Express, NestJS, BullMQ, plain Node, Deno).

detect:
  - { kind: file-glob,       pattern: "package.json" }
  - { kind: file-glob,       pattern: "tsconfig*.json" }
  - { kind: file-extension,  ext: ".ts" }
  - { kind: file-extension,  ext: ".tsx" }

briefs:
  analyze:
    - id: analyze
      brief: briefs/analyze.md
  survey:
    - id: survey-cluster
      brief: briefs/survey.md
  extract:
    - id: extract-business-logic
      brief: briefs/extract-business-logic.md
    - id: extract-external-api
      brief: briefs/extract-external-api.md

detectors:
  - id: express
  - id: nestjs
  - id: bullmq
```

| Field          | Required | Notes |
|----------------|----------|-------|
| `name`         | yes      | Kebab-case (`^[a-z][a-z0-9-]*$`). Must match the directory name under `source-profiles/`. |
| `version`      | yes      | Integer, `1` only at first land. |
| `description`  | yes      | Single-sentence summary. |
| `detect`       | no       | Ordered list of auto-detection heuristics; first match wins. Empty / absent means the profile is never auto-selected (only operator-bound via `sources.yaml`). |
| `briefs`       | yes      | Map keyed by source-side phase (`analyze`, `survey`, `extract`). Each value is an ordered list of pipeline entries with the same shape as `capability.yaml`'s `pipeline` entries. `analyze` is required; `survey` and `extract` are optional (absent means "this profile relies on the framework defaults for that phase"). |
| `detectors`    | no       | Optional list of RFC-20 detector ids the profile activates. Empty / absent means the profile contributes no surface detectors and survey falls back to the global registry. |

`additionalProperties: false` everywhere; `serde(deny_unknown_fields)` in the Rust parser. The schema deliberately rejects a `pipeline` field — the slice loop (`define -> build -> merge`) belongs to the capability axis, not the source-profile axis.

### Directory layout

```text
source-profiles/
  default/
    profile.yaml
    briefs/
      analyze.md
  typescript-node/
    profile.yaml
    briefs/
      analyze.md
      survey.md
      extract-business-logic.md
      extract-external-api.md
  README.md
```

First-party profiles ship inside the framework repo, mirroring `capabilities/`. Third-party profiles install into `.specify/.cache/source-profiles/<name>/`, mirroring the existing capability cache convention from [`plugins/spec/references/capability-resolution.md`](../plugins/spec/references/capability-resolution.md).

### Resolution algorithm

`specify source-profile resolve <key-or-name> --format json` returns the resolved directory path plus a `source` flag (`local` | `cached` | `default`). The CLI accepts three input shapes:

| Input                                          | Resolution                                                                                                       |
|------------------------------------------------|------------------------------------------------------------------------------------------------------------------|
| `--source-key <key>` against the current `sources.yaml` | Read `sources[]`, find the matching entry, return its `profile`. If unset, fall through to auto-detection on the resolved local path. |
| `--name <profile-name>`                        | Bare-name lookup: cache first (`.specify/.cache/source-profiles/<name>/`), then in-repo (`source-profiles/<name>/`). Same fall-through rules as capability resolution. |
| `--path <local-path>`                          | Auto-detection: run every registered profile's `detect:` rules against the path; pick the first match by `name` order. Fall back to `default` on no match. |

`specify source-profile list --format json` enumerates every available profile (cache + in-repo + first-party). `specify source-profile validate` validates a `profile.yaml` against the schema, asserts every declared brief file resolves, and (for first-party profiles) asserts detector ids referenced under `detectors:` exist in the binary's detector registry.

The exit-code surface mirrors `specify capability`: `0` success, `1` generic, `2` validation. New error discriminants (kebab-case): `source-profile-name-unknown`, `source-profile-detect-ambiguous` (auto-detect matched multiple non-default profiles; operator must pick), `source-profile-brief-missing`, `source-profile-detector-unknown`.

### `sources.yaml`: a new `profile` field

RFC-21's `sources.yaml` gains one optional field:

```yaml
version: 1
sources:
  - key: legacy-billing
    url: git@github.com:org/legacy-billing.git
    language: typescript
    profile: typescript-node          # NEW — optional
    description: 2018 billing monolith; subscription, invoicing, dunning.
  - key: legacy-mainframe
    url: file:///srv/cobol/lib
    language: cobol
    profile: cobol-mvs                # NEW
    description: 1996 batch settlement system.
```

| Field                | Required | Notes |
|----------------------|----------|-------|
| `sources[].profile`  | no       | Kebab-case profile name; must resolve via `specify source-profile resolve --name <profile>`. When absent, the resolver auto-detects against the materialised source path (when present in the tier-1 cache from RFC-21) or against the inline `--source <key>=<path>` invocation. |

`language` remains advisory (per RFC-21); `profile` is the binding key. A `language` value that disagrees with the resolved profile's `name` produces a `Warning` finding from `specify sources validate` (`profile-language-mismatch`), never an error — a language string is descriptive, a profile is a binding.

The existing `--source <key>=<path-or-url>` inline form (RFC-21) gains an optional `:profile=<name>` suffix:

| Form                                                              | Meaning |
|-------------------------------------------------------------------|---------|
| `--source legacy-billing=./inputs/billing`                        | Auto-detect profile (today's behaviour).                                                                          |
| `--source legacy-billing=./inputs/billing:profile=typescript-node` | Pin profile explicitly. Skips auto-detection.                                                                     |
| `--source @legacy-billing` (RFC-21)                               | Resolve via `sources.yaml`. Profile comes from the catalogue entry's `profile` field, falling back to auto-detect. |

### Brief slots a profile owns

A profile claims source-side brief slots only. Target-side brief slots stay where they are.

| Phase                      | Today (target-keyed)                                                                  | Post-RFC-25                                                                                                 |
|----------------------------|----------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------|
| `/change:analyze` per-kind | `plugins/change/skills/draft/briefs/<capability>/analyze.md` (both branches embedded). | `source-profiles/<profile>/briefs/analyze.md` (per-source dispatch).                                        |
| `/change:survey` cluster   | (RFC-20 v1) global single-pass clustering owned by the skill.                          | `source-profiles/<profile>/briefs/survey.md` (per-source dispatch). Skill falls back to RFC-20 v1 prose if absent. |
| `/change:survey` detectors | (RFC-20 v1) single global registry baked into the binary.                              | Per-profile detector set (still in-binary); registry filtered by profile at dispatch time.                  |
| `/spec:extract`            | `plugins/spec/skills/extract/{business-logic,external-api,...}.md`.                    | Per-profile overrides at `source-profiles/<profile>/briefs/extract-*.md`; framework defaults remain as the fallback. |
| `/change:draft` discovery  | `plugins/change/skills/draft/briefs/<capability>/discovery.md`.                        | Unchanged — discovery is target-axis (it knows about `## Proposed registry topology` for the target).        |
| `/change:draft` propose    | `plugins/change/skills/draft/briefs/<capability>/propose.md`.                          | Unchanged — propose imposes target-shape constraints on the candidate inventory.                            |
| `/change:draft` assignment | `plugins/change/skills/draft/{assignment,sync-workspace}.md`.                          | Unchanged — assignment is target-axis.                                                                      |
| `define -> build -> merge` | `capabilities/<capability>/briefs/*.md`.                                                | Unchanged — slice loop is target-axis throughout.                                                            |

The discriminator inside the existing per-capability `analyze.md` (the `Documentation branch` / `Legacy-code branch` split keyed on `kind`) survives the move. Documentation inputs continue to dispatch to a per-profile `analyze.md` too — the framework ships a `documentation` profile (in addition to `default` and `typescript-node`) that owns today's docs-side prose. `kind: documentation` resolves directly to that profile, bypassing auto-detection.

### Profile + capability composition

Two dispatch sites need a small contract. Both already have the artifact handoff RFC-20 designed; this RFC names the prose pairing.

**Site 1: `/change:analyze`** (Layer 2, per source input):

1. Resolve **source profile** for `$INPUT_PATH` / `$SOURCE_KEY` via `specify source-profile resolve`.
2. Load `source-profiles/<profile>/briefs/analyze.md`. Execute the `legacy-code` or `documentation` branch.
3. Emit capability summaries under the discovery brief's `## Candidate inventory` heading and (for the code branch) the per-source `analyze/<source-key>/metadata.json` sidecar.

The target capability is **not consulted** during analyze. Propose (target-axis) is the first place the capability shape enters the planning pipeline.

**Site 2: `/change:survey`** (Layer 2, per source input, RFC-20):

1. Resolve **source profile** for the source key.
2. Look up the profile's `detectors:` list; run only those detectors against the source root. Falls back to "run every registered detector" when the profile declares none.
3. Load `source-profiles/<profile>/briefs/survey.md` for the clustering pass. Falls back to RFC-20 v1 prose when absent.
4. Emit `surfaces.json`, `metadata.json`, and the candidate-block appendix under the discovery-owned `## Candidate inventory` heading.

This is the v1.1 generalisation of RFC-20's "Capability scoping" deferral: instead of `plugins/change/skills/survey/briefs/<cap>/detectors/`, the detector pack lives under `source-profiles/<profile>/` and is filtered at runtime. The directory shape RFC-20 reserved (`plugins/change/skills/survey/briefs/<cap>/detectors/`) is **explicitly retired**; it never shipped.

**Site 3: `/spec:extract`** (Layer 1, per source input):

1. Resolve **target capability** as today (from `.specify/project.yaml`).
2. Resolve **source profile** for each source the slice declares (from `sources.yaml` or auto-detect).
3. For every extract reference file (`business-logic.md`, `external-api.md`, …), prefer the per-profile override at `source-profiles/<profile>/briefs/extract-<topic>.md` when present; otherwise use the framework default at `plugins/spec/skills/extract/<topic>.md`.

This is the smallest defensible touch on `/spec:extract`: the resolver lands, the override slot is honoured, but no first-party profile ships extract overrides in this RFC. New overrides land as follow-on work when a non-TS legacy migration actually requires them.

### Migration of existing brief content

Today's `plugins/change/skills/draft/briefs/omnia/analyze.md` has two branches: documentation and legacy-code. The post-RFC-25 layout splits them by source profile:

| Today                                                                                                                   | Post-RFC-25                                                                                                                                                                                              |
|--------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `plugins/change/skills/draft/briefs/omnia/analyze.md` `## Documentation branch`                                          | `source-profiles/documentation/briefs/analyze.md` (full content) + target hooks lifted into `plugins/change/skills/draft/briefs/omnia/propose.md` (the "Apply Omnia's conventions" wording).               |
| `plugins/change/skills/draft/briefs/omnia/analyze.md` `## Legacy-code branch`                                            | `source-profiles/typescript-node/briefs/analyze.md` (full content) + target hooks lifted into the same propose brief.                                                                                    |
| `plugins/change/skills/draft/briefs/vectis/analyze.md` (if/when it lands)                                                | Per-source-profile content shared; Vectis-specific composition guidance lifts into `plugins/change/skills/draft/briefs/vectis/propose.md`.                                                                |

The migration is **structural only** in v1 — every word of the existing Omnia analyze brief moves to the matching profile brief verbatim, except the explicit target-shape phrases ("Prefer capabilities that align with the 'one crate per capability' rule downstream", "Shared utilities that serve multiple capabilities become their own capability") which migrate to the Omnia propose brief instead. A side-by-side diff of the rendered planning output on the existing RFC-20 / RFC-21 fixtures MUST be byte-identical before and after the migration. (See the *Implementation Plan* below for the acceptance fixture.)

A `default` profile ships alongside the move so a project with no auto-detect hits keeps working. `default/briefs/analyze.md` is a thin, language-agnostic wrapper that emits a `confidence: low` capability block per detected `top_level_module` and surfaces an open question pointing the operator at `specify source-profile list`.

### First-party profile set (v1)

| Profile          | Ships briefs                                                                                                | Ships detectors           | Auto-detects via                                                            |
|------------------|------------------------------------------------------------------------------------------------------------|---------------------------|-----------------------------------------------------------------------------|
| `default`        | `analyze.md` (language-agnostic fallback).                                                                  | None.                     | Never. Used as the resolver's last-resort fallback.                          |
| `documentation`  | `analyze.md` (today's `## Documentation branch` from `omnia/analyze.md`, deduped).                          | None.                     | `kind: documentation` only — never inferred from filesystem.                |
| `typescript-node`| `analyze.md` (today's `## Legacy-code branch` from `omnia/analyze.md`), `survey.md` (RFC-20 v1 cluster prose, lifted). | RFC-20's Express, NestJS, BullMQ. | `package.json` OR `tsconfig*.json` OR `*.ts` / `*.tsx` files.                |

No COBOL, C#, Java, or Python profile ships in v1. The point of this RFC is to make those additions *cheap*, not to ship them. The first follow-on RFC that targets a new legacy stack writes only the new profile directory.

## Migration

This RFC is **strictly additive for operators** and **schema-stable for plans**:

- Existing `plan.yaml`, `registry.yaml`, `sources.yaml`, `change.md`, and archives validate without change.
- Existing `/change:draft` invocations continue to work — no new flag is required; auto-detection picks `typescript-node` against any pre-existing TS-flavoured source tree and `default` otherwise.
- The `sources.yaml:sources[].profile` field is optional. Operators with one TS legacy source need not declare anything.
- No verb is renamed, retired, or repurposed.

It is **schema-breaking for capability authors who ship a `briefs/<cap>/analyze.md` file**. The relevant files in the first-party tree today:

- `plugins/change/skills/draft/briefs/omnia/analyze.md` — content moves under `source-profiles/{documentation,typescript-node}/`. The target hooks move to `briefs/omnia/propose.md`. The file itself is deleted.
- `plugins/change/skills/draft/briefs/vectis/analyze.md` — does not exist today. The directory layout reserved by [`draft/SKILL.md`](../plugins/change/skills/draft/SKILL.md) §*Critical Path* step 4(a) ("missing `briefs/<capability>/{discovery,propose}.md` for the active capability is a hard failure") is unchanged.
- Third-party capability authors who shipped a `briefs/<cap>/analyze.md` migrate the same way: per-kind prose moves to a source profile they own (or a profile they share); target-shape constraints move to their `briefs/<cap>/propose.md`.

The `/change:draft` skill's hard-failure rule generalises: a missing `source-profiles/<profile>/briefs/analyze.md` for the resolved profile is a hard failure with the same exit shape as the existing capability-side variant.

For skill authors consuming planning artifacts: **no contract change**. `discovery.md`, the `## Candidate inventory` heading, `surfaces.json`, `metadata.json` shapes, and the propose brief's input contract are all unchanged. Skills do not see the profile axis; only the resolver does.

## Non-Goals

- A COBOL, C#, Java, Python, or Go first-party profile in v1.
- Per-profile `define -> build -> merge` briefs (slice loop is target-axis; pulling source-side knowledge into the slice loop is out of scope until a concrete need lands).
- A profile-level `cargo.toml` / `package.json` / `pom.xml` parser embedded in the CLI (auto-detection is filesystem-pattern-only in v1).
- LLM-fallback analyze for sources with no matching profile (the `default` profile's language-agnostic prose is the v1 fallback; LLM fallback is a separate RFC).
- A profile *plugin marketplace* (third-party profiles install via the existing `.specify/.cache/source-profiles/<name>/` convention; discovery, distribution, signing, and versioning beyond what `capabilities/` already does are out of scope).
- Cross-profile inheritance (`extends:`) — profiles are flat in v1, mirroring the closed `extends`-free shape `capability.yaml` settled on per RFC-13.
- A `profile` field on `change.md:inputs[]` — the field lives on `sources.yaml` (RFC-21's catalogue) and on the inline `--source ...:profile=<name>` suffix. Adding it to `change.md` is deferred until the catalogue indirection proves insufficient.

## Out Of Scope

| Item                                                                                                  | Re-open when                                                                                                                              |
|-------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------|
| Per-profile slice-loop briefs (a profile that overrides `proposal.md` / `specs.md` / `design.md`)     | A target-agnostic source profile demonstrably needs to shape the slice loop without the target capability being a usable substitute.       |
| `extends:` cross-profile inheritance                                                                  | More than three first-party profiles share >50% of a brief and the duplication becomes a maintenance burden.                              |
| Auto-detection beyond filesystem glob / extension matching (e.g. `package.json:dependencies` content) | A real ambiguity reaches the operator (e.g. a Node project that's actually a TS-to-Wasm toolchain consuming the same `package.json`).      |
| LLM-fallback `analyze.md` for sources with no profile match                                           | A real legacy stack reaches the planning pipeline that genuinely cannot be served by the `default` profile's language-agnostic content.    |
| `profile` field on `change.md:inputs[]`                                                               | An operator workflow repeatedly needs per-change profile overrides that the `sources.yaml` indirection cannot express.                    |
| Per-profile `extract/*.md` overrides shipped first-party                                              | A non-TS migration lands and the framework defaults at `plugins/spec/skills/extract/*.md` produce demonstrably bad extracts.              |
| A `profile` projection in `specify plan validate` (warn when slice `sources:` mixes incompatible profiles) | Operators repeatedly assemble slices that mix profiles in ways the propose brief cannot reasonably reshape.                                |
| Per-profile codex rules (a profile that wants to ship source-side review patterns)                    | A profile materially reuses codex shape — propose-time review of source-stack-specific anti-patterns becomes a pattern.                    |
| Versioning the profile manifest (`version: 2`, `extends:`, migration tools)                           | The `version: 1` manifest accumulates compatibility constraints that block a wanted change.                                                |
| Profile-level fixtures separate from `plugins/change/skills/draft/fixtures/`                          | Profiles ship divergent enough analyze prose that brief-level regression coverage outgrows the per-skill fixture tree.                     |

## Implementation Plan

1. **Schema and resolver.** Land `specify-cli/schemas/source-profile/profile.schema.json`. Add `SourceProfile`, `SourceProfileBriefs`, `DetectRule` types in `specify-domain` (`crates/domain/src/source_profile/`). Mirror the `Capability` posture: `serde(deny_unknown_fields)`, `load()` / `validate_shape()` helpers. `specify-error` gains `source-profile-*` discriminants.
2. **`specify source-profile` verb family.** Add `src/commands/source_profile/{cli,resolve,list,validate}.rs`. JSON envelope mirrors `specify capability {resolve, pipeline}`. Integration tests under `tests/source_profile.rs`.
3. **Auto-detection.** Implement `DetectRule` evaluation against a local path (`file-glob`, `file-extension`, future-reserved `file-content` rejected with `source-profile-detect-rule-unknown`). Land `specify source-profile resolve --path <p>` with deterministic ordering (first match wins; ties on `name` ordering).
4. **`source-profiles/` directory + first-party profiles.** Add `source-profiles/{default,documentation,typescript-node}/profile.yaml` and the migrated brief content. Land `make checks` predicates that mirror today's `capabilities/` validators (manifest validates, every declared brief resolves, brief frontmatter `id` matches the manifest).
5. **Content migration.** Move `plugins/change/skills/draft/briefs/omnia/analyze.md`'s two branches under `source-profiles/{documentation,typescript-node}/briefs/analyze.md`. Move the target-shape phrases into `plugins/change/skills/draft/briefs/omnia/propose.md`. Delete `briefs/omnia/analyze.md`. Update `plugins/change/skills/analyze/SKILL.md` so its §*Per-kind prompts* points at the profile resolver, not the capability resolver.
6. **RFC-20 detector registry generalisation.** Refactor `DetectorRegistry` from a single global registry to a `BTreeMap<ProfileName, Vec<Box<dyn Detector>>>`. `specify change survey` consults the resolved profile's `detectors:` list at dispatch time. Empty list = run every detector (preserves RFC-20 v1 behaviour for projects without a profile binding). New error discriminant: `surface-scan-profile-unknown`.
7. **`sources.yaml:sources[].profile` field.** Additive schema bump on RFC-21's `sources.schema.json`. `specify sources validate` cross-references each declared profile against `specify source-profile list`. `profile-language-mismatch` finding lands at `Warning` severity.
8. **`--source <key>=<path>:profile=<name>` inline form.** Update RFC-21's `--source @<key>` resolver and RFC-21 §*--source @<key> selector* documentation.
9. **`/change:analyze` skill update.** Per *Profile + capability composition Site 1*: resolve profile first, load `source-profiles/<profile>/briefs/analyze.md`. The skill's existing `kind` dispatch becomes a profile-selection input (`documentation` -> `documentation` profile; `legacy-code` -> auto-detect).
10. **`/change:survey` skill update (when RFC-20 lands or in parallel).** Per *Profile + capability composition Site 2*: same resolver call before detector dispatch and before the clustering pass.
11. **`/spec:extract` skill update.** Per *Profile + capability composition Site 3*: resolve per-source profile, prefer per-profile `extract-<topic>.md` when present. No first-party overrides ship in v1; the resolver lands and the override slot is honoured, that is all.
12. **Acceptance fixtures.**
    - Single-source TS monolith: rendered `discovery.md` is byte-identical to today's `plugins/change/skills/draft/fixtures/discovery/monolith/expected/discovery.md`. **This is the migration's correctness gate.**
    - Mixed-input fixture: rendered output byte-identical to `plugins/change/skills/draft/fixtures/discovery/mixed-inputs/expected/discovery.md`.
    - `default`-profile fallback: a source tree with no auto-detect hit produces a `discovery.md` with low-confidence per-`top_level_module` capability blocks and a `## Open questions` block pointing at `specify source-profile list`.
    - Explicit override: `--source legacy=./inputs:profile=typescript-node` against a `package.json`-less directory produces the TS-profile output.
    - Two-profile change: one TS source + one `documentation` source produces one merged `discovery.md` with both profiles' contributions byte-stable under the existing `## Candidate inventory` ordering.
13. **Tutorials and references.** Update [`docs/explanation/capabilities-and-plugins.md`](../docs/explanation/capabilities-and-plugins.md) to describe the source-profile axis alongside the capability axis. New reference at `docs/reference/source-profiles/index.md` mirroring `docs/reference/capabilities/index.md`. Update `plugins/change/skills/analyze/SKILL.md` and `plugins/spec/references/capability-resolution.md` (rename the latter or split it).
14. **RFC-20 follow-up.** Open a small RFC-20 amendment that retires the deferred `plugins/change/skills/survey/briefs/<cap>/detectors/` directory (per *Profile + capability composition Site 2*) and points at this RFC.

## Worked Example

A multi-source change against one TS monolith plus one COBOL settlement system:

```bash
specify sources add legacy-billing --url git@github.com:org/legacy-billing.git --profile typescript-node
specify sources add legacy-mainframe --url file:///srv/cobol/lib --profile cobol-mvs
/change:draft migrate-settlement --source @legacy-billing --source @legacy-mainframe
```

Inside `/change:draft`'s discovery brief:

1. `specify source-profile resolve --source-key legacy-billing` -> `typescript-node`.
2. `specify source-profile resolve --source-key legacy-mainframe` -> `cobol-mvs`.
3. Two `/change:analyze` invocations dispatch in parallel (RFC-21's `--analyze-concurrency`):
   - Source `legacy-billing` loads `source-profiles/typescript-node/briefs/analyze.md`, runs its TS-shaped clustering.
   - Source `legacy-mainframe` loads `source-profiles/cobol-mvs/briefs/analyze.md`, runs its COBOL-shaped clustering (this profile is hypothetical for the example; it ships in a follow-on RFC).
4. The discovery brief merges both invocations' output under one `## Candidate inventory` heading, alphabetically by capability name across sources.
5. Propose runs once. The target capability (`omnia`, say) loads `plugins/change/skills/draft/briefs/omnia/propose.md` and reshapes the merged inventory into the Omnia "one crate per capability" plan entries. The COBOL side and the TS side both reach propose in the same shape; the target brief does not know they came from different profiles.

At no point does propose's brief read profile-specific content. At no point does either profile's `analyze.md` read target-capability-specific content. The handoff is the existing `discovery.md` `## Candidate inventory` shape.

## Alternatives Considered

**Add a `kind` value to `/change:analyze` per supported source platform (`cobol-legacy-code`, `csharp-legacy-code`, …).** Rejected. The closed enum on `kind` is already a closed enum on *input modality* (code vs documentation vs RFC-20's future `domain-model`). Overloading it with a language axis muddles the dispatch and forces every consumer of `kind` to grow with each new language.

**Fan out by `language` inside each capability's analyze brief.** Rejected. Concentrates source-side knowledge inside target-axis files. Every new target capability would need to grow every new language branch. The axes do not commute that way.

**Ship per-language analyze briefs at `plugins/change/skills/analyze/lang/<lang>.md` without a manifest, validator, or resolver.** Rejected. Reuses none of the existing capability-resolver infrastructure, has no cache convention for third-party profiles, has no equivalent of `specify capability resolve` for skills to consult, and provides no contract for the RFC-20 detector pack split. The resolver is the load-bearing piece.

**Make profile a `.specify/project.yaml` field instead of a per-source field.** Rejected. A multi-repo migration may mix TS, COBOL, and documentation in one change; a per-project profile cannot describe that. The source level is the right granularity. Project-level capability is the right granularity for the target axis because every slice lands in one project. Sources fan out per change.

**Defer source profiles entirely until a non-TS migration actually lands.** Rejected for two reasons. First, RFC-20 already cannot proceed with its capability-keyed detector deferral — adopting the wrong axis is worse than leaving it open. Second, the migration cost compounds with every new capability that ships an `analyze.md`; doing the split once now is cheaper than splitting twice later.

## Open Questions

- Should `default` and `documentation` ship as profiles or as well-known names inside the resolver (no manifest)? The RFC currently assumes profiles to keep the resolver uniform; an inline-resolver alternative would save two `profile.yaml` files at the cost of two resolver special cases.
- Where does `language` end up after this RFC settles — does it stay as RFC-21's advisory field, or does the warning-only `profile-language-mismatch` finding push us toward dropping it? Current proposal: keep it; the warning is the contract.
- Should `specify source-profile resolve --source-key <key>` be a separate verb, or should `specify sources show <key>` grow a `resolved_profile` field? Current proposal: separate verb, mirroring `specify capability resolve`.
- Does RFC-22's migration ledger want to record the profile a source migrated under, or is the `sources[].profile` field sufficient (the ledger reads `sources.yaml` per change anyway)? Punt to RFC-22 unless a concrete consumer surfaces.
- Should profile detection ever read git remote URLs or repo metadata (e.g. a `.github/`-shaped directory hints `typescript-node` more strongly than `package.json` alone)? Out of scope here; the `Out Of Scope` table tracks the re-open trigger.
- Does the `/spec:extract` integration need a Layer-1 follow-on RFC? Probably yes once the first non-TS profile ships extract overrides; this RFC lands the resolver only.

## References

- [RFC-13: Extensibility](archive/rfc-13-extensibility.md) — capability manifest protocol; the source-profile manifest is its parallel.
- [RFC-20: Survey to Plan](rfc-20-survey.md) — detector contract and the deferred capability-keyed detector pack this RFC retires.
- [RFC-21: Source Catalogue and Tier-1 Cache](rfc-21-catalogue.md) — `sources.yaml` host of the new `profile` field.
- [RFC-22: Migration Ledger](rfc-22-ledger.md) — downstream consumer of `sources[].profile`.
- [RFC-23: Change Lifecycle](archive/rfc-23-change-lifecycle.md) — three-skill planning lifecycle this RFC fits inside.
- [`/change:analyze` SKILL.md](../plugins/change/skills/analyze/SKILL.md) — primary dispatch site.
- [`/change:draft` SKILL.md](../plugins/change/skills/draft/SKILL.md) — brief pipeline owner.
- [`/spec:extract` SKILL.md](../plugins/spec/skills/extract/SKILL.md) — Layer 1 dispatch site.
- [`capabilities/capability.schema.json`](../capabilities/capability.schema.json) — manifest shape mirrored by `profile.schema.json`.
- [`plugins/spec/references/capability-resolution.md`](../plugins/spec/references/capability-resolution.md) — resolver convention mirrored by `specify source-profile resolve`.
