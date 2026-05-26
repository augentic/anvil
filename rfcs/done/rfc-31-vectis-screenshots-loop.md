# RFC-31: Vectis / Screenshots Loop Hardening

> Status: Accepted - Depends: [RFC-25](../done/rfc-25-workflow.md), [RFC-5](../done/rfc-5-tooling.md), [RFC-16 (archived)](../done/rfc-16-wasi-vectis.md) - Compatible with [RFC-29](../next/rfc-29-fan-in-fan-out.md), [RFC-30](../next/rfc-30-init.md) - Enables: standalone screenshots inference review, cross-slice component reuse

## Abstract

RFC-25 wired the `screenshots` source adapter and the `vectis` target adapter into the Specify 2.0 workflow. The two adapters now produce and consume the spatial `region` / `container` / `leaf` Evidence claim kinds, and `composition.yaml` regenerates inside `vectis.build` rather than living as a Specify artifact. The contract is correct, but daily use of the screenshots → vectis pipeline has exposed three rough edges that none of RFC-25, RFC-29, or RFC-30 close:

1. **Schemas are duplicated across repos.** The Vectis runtime schemas (`tokens.schema.json`, `assets.schema.json`, `composition.schema.json`) live in *two* places — the plugin repo's [`adapters/targets/vectis/schemas/`](../../adapters/targets/vectis/schemas/) and the CLI repo's [`wasi-tools/vectis/embedded/`](https://github.com/augentic/specify-cli/tree/main/wasi-tools/vectis/embedded/). The vendor comment in `wasi-tools/vectis/src/validate/engine/shared.rs` openly says "the upstream is canonical and any edit there must be mirrored here byte-for-byte." Manual mirroring is a permanent error source.
2. **The screenshots adapter has no standalone review surface.** Before 2.0, an operator could run `specify tool run vectis -- validate layout` against a freshly-inferred `layout.yaml` and immediately see whether the inference was correct. 2.0 replaced `layout.yaml` with Evidence claims, which is correct as a workflow change, but it left no way to *iterate on the screenshots adapter itself* without a full plan / refine / build cycle. An operator who wants to refine the prompt, add a better source image, or sanity-check inference quality is forced to run the entire pipeline and read the synthesized `spec.md` to infer what was detected.
3. **The Vectis build has no way to know which UI structures should be shared components.** Each `screenshots.extract` invocation only sees the candidate it is asked to extract. Common navigation patterns, headers, and toolbars across slices are re-inferred from scratch, the Vectis build inlines visually-equivalent code per screen, and the UI drifts pixel-by-pixel from one slice to the next. The operator must hand-promote `component:` directives across slices with no assistance from the tooling.

This RFC adds three refinements that stay inside RFC-25's framework:

1. **Tool-owned schemas with a cross-repo reference verb.** The WASI tool that *runs* a schema is its sole owner. Plugin briefs reference schemas by `$id` and operators can extract any tool-owned schema with a new `specify tool schema <tool> <name>` verb. The plugin repo's schema copies retire.
2. **A `specify source preview` verb.** Operators can run a source adapter's `enumerate` + `extract` against a directory of inputs in complete isolation of the workflow — no plan, no slice, no `change.md`. The output is the Evidence files, dumped to a local directory.
3. **An operator-curated component catalog.** A new file at `.specify/design-system/components.yaml` lets operators declare shared components. The Vectis target reads the catalog at build time and factors shared components in code. The catalog follows the same operator-curated pattern as `tokens.yaml` and `assets.yaml`.

None of the three asks introduces a new lifecycle, a new slash command, or a new slice ceremony. They refine surfaces that already exist.

## Motivation

The findings this RFC closes, in order of magnitude:

| Magnitude | Finding | Current state | RFC-31 resolution |
| --- | --- | --- | --- |
| Small | Vectis runtime schemas duplicated across repos. | `adapters/targets/vectis/schemas/*.schema.json` and `wasi-tools/vectis/embedded/*.schema.json` are byte-identical copies kept in sync by hand. The CLI source comment names the discipline explicitly. The `$id` URLs even disagree on whether they live under `adapters/vectis/` or `targets/vectis/`. | Schemas are owned by the tool that runs them. The plugin repo holds no schema bodies; briefs cite the tool's canonical `$id`. A new `specify tool schema` verb extracts any tool-owned schema on demand. |
| Medium | Screenshots inference cannot be reviewed in isolation. | `screenshots.extract` runs only inside `/spec:refine`. Operators iterating on prompt quality or source-image quality must drive a full plan → refine cycle and read synthesized `spec.md` to guess what the adapter saw. | `specify source preview <adapter>` runs `enumerate` + `extract` against a directory with no plan or slice and dumps the resulting Evidence to a local directory. |
| Medium | Vectis build cannot factor shared components across slices. | Each slice re-infers common navigation, headers, and toolbars from scratch. The Vectis build inlines per-screen. Common UI structures drift visually across slices. | An operator-curated `.specify/design-system/components.yaml` declares shared components. The Vectis build reads it and factors shared component code, the same way it already reads `tokens.yaml` and `assets.yaml`. |

The schema-duplication finding is the simplest to land; the shared-components finding requires the operator to observe patterns across slices and declare them explicitly. Stacking them in one RFC is deliberate — all three live at the screenshots / vectis seam and benefit from a single migration window.

## Principles

1. **Refine, do not rebuild.** Every surface added here lives inside the RFC-25 vocabulary (source / target adapters, candidate / Evidence, slice lifecycle, plan). Nothing introduces a new slash command, a new lifecycle state, or a second writer for any existing artifact.
2. **One source of truth per schema.** The tool that consumes a schema is the only repo that contains its body. Everywhere else cites the canonical `$id`.
3. **Source adapters get a workbench.** Operators must be able to exercise a source adapter against real inputs without invoking the workflow. Adapter quality is then directly debuggable.
4. **Follow the tokens/assets pattern.** Component reuse is a *project-level* concern. The component catalog is operator-curated and follows the same pattern as the existing `tokens.yaml` and `assets.yaml` design-system files — the operator authors it, the build reads it, validation checks consistency.
5. **Backward compatibility within 2.0.** Projects without `.specify/design-system/components.yaml` work exactly as today; the catalog is opt-in.

## Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 Tool-owned schemas** | Every JSON Schema is owned by the repo of the WASI tool (or the CLI) that runs it. Plugin briefs MUST NOT contain schema bodies. | Delete `adapters/targets/vectis/schemas/*.schema.json`; replace with a `README.md` that links to canonical `$id` URLs and documents `specify tool schema`. Update every brief that cites a vendored schema path. |
| **D2 `specify tool schema` verb** | The CLI exposes `specify tool schema <tool> <name>` that prints any embedded schema to stdout as pretty-printed JSON. | Add `src/commands/tool/schema.rs`; route through `Tool::resolve` and the tool's declared schema registry; works for `vectis` and `contract` on day one. |
| **D3 Schema `$id` convention** | Tool-owned schemas use a stable `$id` of the form `https://schemas.specify.dev/<tool>/<name>.schema.json`. The `$id` is hardcoded in each schema file. | Fix the existing `$id` disagreement (`adapters/vectis/` vs `targets/vectis/`) by settling on one convention. No build-time rewriting needed. |
| **D4 `specify source preview`** | The CLI exposes `specify source preview <adapter> --source <path>` that runs `enumerate` + `extract` against a directory, with no plan and no slice required. | Add `src/commands/source/preview.rs`; routes through `SourceAdapter::resolve` and the RFC-29 source runner; writes Evidence to `--out` (default `./.specify-preview/`). |
| **D5 Project-level component catalog** | A project MAY carry `.specify/design-system/components.yaml`. The file is operator-curated — the operator authors and maintains it, the same way they author `tokens.yaml` and `assets.yaml`. The schema is CLI-owned. | Add `schemas/design-system/components.schema.json` to the CLI repo; validate via `specify slice validate`; resolve catalog paths from the active project root (workspace slot after sync/chdir, same routing as slice execution). |
| **D6 Vectis catalog consumer** | The Vectis target's `build` brief reads the component catalog and factors shared components in generated code and `composition.yaml`. | Update `adapters/targets/vectis/briefs/build.md` and `briefs/build/composition.md` to require catalog reads; emit one shared component file per confirmed catalog entry per in-scope shell tree. |

## Operator surface

The default operator rhythm does not change:

```bash
/spec:plan onboarding-screens source ui=screenshots:./screens
specify plan transition onboarding-screens reviewed
/spec:execute
/spec:finalize onboarding-screens
```

The new CLI surfaces are workshop tools, not workflow steps:

```bash
# Inspect tool-owned schemas without leaving the terminal.
specify tool schema vectis tokens
specify tool schema vectis composition

# Iterate on a source adapter against real inputs, with no plan or slice.
specify source preview screenshots --source ./design-explorations/onboarding
specify source preview screenshots --source ./design-explorations/onboarding --out ./preview
```

The component catalog is a hand-authored YAML file, curated exactly like `tokens.yaml` and `assets.yaml`. The Vectis build reads it; `specify slice validate` checks that Evidence `component:` directives resolve to confirmed catalog entries.

## Schema single-source (D1, D2, D3)

### Current state

Three Vectis schemas are duplicated:

| Schema | Plugin repo copy | CLI repo copy | Drift mechanism |
| --- | --- | --- | --- |
| `tokens.schema.json` | `adapters/targets/vectis/schemas/tokens.schema.json` | `wasi-tools/vectis/embedded/tokens.schema.json` | Hand-mirror, "byte-identity discipline" comment in `shared.rs`. |
| `assets.schema.json` | `adapters/targets/vectis/schemas/assets.schema.json` | `wasi-tools/vectis/embedded/assets.schema.json` | Same. |
| `composition.schema.json` | `adapters/targets/vectis/schemas/composition.schema.json` | `wasi-tools/vectis/embedded/composition.schema.json` | Same. |

The two copies disagree even on the `$id` URL (`adapters/vectis/...` vs `targets/vectis/...`). Anyone editing one and forgetting the other ships a real wire breakage on the first WASI run.

### Resolution

**The tool that runs a schema is its sole owner.** For Vectis runtime schemas, that means `specify-cli/wasi-tools/vectis/embedded/*.schema.json` is the only copy that exists; the plugin repo carries zero schema bodies.

Plugin briefs reference schemas exclusively by `$id`:

```markdown
Validates against [`vectis.tokens.schema.json`](https://schemas.specify.dev/vectis/tokens.schema.json).
```

Operators and agents pull the body on demand:

```bash
specify tool schema vectis tokens > /tmp/tokens.schema.json
```

### `specify tool schema` (D2)

```text
specify tool schema <tool> <name>
```

- `<tool>` resolves through the same path as `specify tool run` (declared `tools[]`).
- `<name>` is the kebab-case schema id; the tool advertises its registry through a new `Tool::schemas()` -> `&[(name, sha256, body)]` accessor.
- Output is pretty-printed JSON with stable key ordering.

Exits:

- `0` — schema emitted to stdout.
- `2` — unknown tool or unknown schema.

### `$id` convention (D3)

Each tool-owned schema uses a stable `$id` of the form:

```text
https://schemas.specify.dev/<tool>/<name>.schema.json
```

The `$id` is hardcoded in each schema file. The existing disagreement between `adapters/vectis/...` and `targets/vectis/...` is resolved by settling on the `<tool>/` convention. The URL is a logical identifier; it does not need to resolve to a hosted copy to be useful as a stable reference.

The invariant lands in the unified framework checker shipped by [RFC-5](../done/rfc-5-tooling.md) — `specdev check` in `specify-cli`'s `specify-authoring` crate, invoked locally and in CI via `make check`:

```text
links.brief-schema-link-resolve: every URL matching schemas.specify.dev/<tool>/...
                                 in any brief or reference must round-trip through
                                 `specrun tool schema <tool> <name>` byte-for-byte.
```

Implementation lives in `crates/authoring/src/check/schema_links.rs` as `check::schema_links`, registered alongside the other predicates in `check/mod.rs`. The check scans adapter briefs and `references/` trees for tool-owned schema URLs, invokes the operator CLI's schema registry, and emits a `links.brief-schema-link-resolve` finding when a cited URL does not resolve or its body disagrees with the tool-embedded copy. Fixture coverage follows the same pattern as `check::links` and `check::tools` under `crates/authoring/tests/`. Extend [`docs/contributing/checks.md`](../docs/contributing/checks.md) with check 14 when the predicate lands.

### Migration

| Step | Action |
| --- | --- |
| 1 | Land `specify tool schema` in the CLI; ship `Tool::schemas()` for `vectis` and `contract`. |
| 2 | Fix the `$id` in each tool's embedded schemas to use the stable convention. |
| 3 | Update every plugin-repo brief that cites a local schema path to cite the canonical `$id` URL instead. |
| 4 | Delete `adapters/targets/vectis/schemas/*.schema.json`; replace `adapters/targets/vectis/schemas/README.md` with the URL list and a `specify tool schema` quickstart. |
| 5 | Add `check::schema_links` to `specify-authoring` with rule id `links.brief-schema-link-resolve`; register it in `specdev check` so `make check` and CI enforce the invariant. |
| 6 | Remove the "byte-identity discipline" comment block from `wasi-tools/vectis/src/validate/engine/shared.rs`; the schemas are now first-class CLI assets. |

After migration, the plugin repo carries zero `.schema.json` files for tool-owned artifacts. Framework-level schemas (adapter, source, target, evidence, plan, slice/fusion) stay where they are — they are CLI-owned and already follow this pattern.

## Standalone source preview (D4)

### Current state

`screenshots.extract` runs only inside `/spec:refine`. An operator iterating on the adapter (refining the prompt, adding hints, trying better source images) must:

1. Author or amend a slice that binds the candidate.
2. Stamp Gate 1.
3. Run `/spec:execute` (or `/spec:refine` directly).
4. Read the resulting `spec.md` and `evidence/screens.yaml` to guess what the adapter actually inferred.
5. Discard the slice (`/spec:drop`) if the result is poor.
6. Repeat.

Every iteration costs a slice lifecycle round-trip and a synthesized `spec.md` they will throw away.

### Resolution

A new CLI verb runs the source adapter directly against a directory, with no plan or slice:

```text
specify source preview <adapter> --source <path>
                                 [--candidate <id> ...]
                                 [--out <path>]
```

- `<adapter>` resolves against the adapter loader. The CLI runs the adapter's `enumerate` and `extract` operations under the RFC-25 sandbox profile.
- `--source` is the bound path that becomes `$SOURCE_DIR`. Required.
- `--candidate` restricts extraction to a subset; defaults to "every candidate `enumerate` produced."
- `--out` is the output directory; defaults to `./.specify-preview/`.

The verb is workflow-free: nothing is written into `.specify/`, no lifecycle moves, no journal events fire. Output lives entirely under `--out`.

### Output

The preview writes two things:

1. **A textual summary to stdout** listing each candidate discovered by `enumerate`, the claim counts by kind (`region` / `container` / `leaf`), and any `notes.candidate_component` or `notes.todo` annotations.
2. **Evidence files** at `${--out}/evidence/<source-key>.yaml`, byte-identical to what `/spec:refine` would persist for the same input.

The Evidence files are the report — they are structured YAML that operators and agents can inspect directly. No additional report schema is needed; Evidence already has a well-defined schema.

### Acceptance for the preview verb

- `specify source preview screenshots --source <dir>` succeeds against a fresh checkout with no `.specify/` present.
- The command produces Evidence at `${--out}/evidence/screens.yaml` that is byte-equal to what `/spec:refine` would persist for the same input.
- The verb leaves no residue under `.specify/` and no journal event fires.

Depends on RFC-29 wave A (executable `specify source enumerate` / `specify source extract`). Without RFC-29, `specify source preview` either ships against the agent-run fallback or waits.

## Component catalog (D5, D6)

### Current state

The screenshots adapter's `extract/pipeline.md` stage 6 ("Detect candidate components conservatively") only fires within one `extract` call: a `component: <slug>` directive lands when ≥2 structurally identical groups appear *in the same run*. Across slices and across runs, the adapter has no memory:

- The first slice's `task-list` screen has a tab bar. Stage 6 sees one instance → emits `notes.candidate_component: tab-bar`. No promotion.
- The second slice's `settings` screen has the same tab bar. The new extract sees one instance in *its* run → emits the same note. Still no promotion.
- The Vectis build inlines the tab bar twice. Visually they drift. The operator never knew the catalog opportunity existed.

### Resolution

**An operator-curated component catalog tells the Vectis build which structures to factor into shared components.** The catalog follows the same pattern as `tokens.yaml` and `assets.yaml` — the operator authors it, the build reads it, validation checks consistency.

#### File

```text
.specify/design-system/components.yaml
```

In workspace mode the path is the same relative to the **active project root** — the materialised registry slot, not the coordinator root:

```text
<coordinator-root>/.specify/workspace/<project>/.specify/design-system/components.yaml
```

The file is opt-in: projects that never produce one keep working exactly as today.

#### Schema

The catalog schema is deliberately simple (`schemas/design-system/components.schema.json`):

```yaml
version: 1
components:
  tab-bar:
    status: confirmed            # confirmed | rejected
    description: "Bottom navigation across the primary app sections."
  card-row:
    status: confirmed
    description: "Horizontal card layout used in browse and search screens."
```

Each entry has:

- **`status`**: `confirmed` (the build should factor this as a shared component) or `rejected` (the operator has decided this is not a real shared component; suppresses `slice-catalog-drift` warnings for Evidence that carries the slug in `notes.candidate_component`).
- **`description`**: Human-readable note for operators and agents. Optional.

The schema is **CLI-owned** (alongside `evidence.schema.json`, `plan.schema.json`, and other framework-level schemas) and lives in the CLI repo at `schemas/design-system/components.schema.json`.

#### Operator workflow

The operator observes patterns across slices — either by noticing repeated `notes.candidate_component` annotations in Evidence, by visual inspection of generated screens, or by looking at the preview output — and adds entries to the catalog:

1. Open `.specify/design-system/components.yaml`.
2. Add an entry with `status: confirmed` and a description.
3. On the next `/spec:build`, the Vectis build reads the catalog and generates a shared component file per confirmed entry.

This mirrors how operators already work with `tokens.yaml` (observe design patterns → curate tokens) and `assets.yaml` (observe asset needs → curate asset entries). The screenshots adapter's stage-6 `notes.candidate_component` annotations serve as hints — they surface potential shared components in the Evidence, and the operator decides which ones to promote to the catalog.

#### Validation

`specify slice validate` adds one finding:

| Finding | Meaning |
| --- | --- |
| `slice-catalog-drift` | The Evidence persists a claim with `component: <slug>` where `<slug>` is not in `.specify/design-system/components.yaml`, or the catalog entry is `rejected`. |

#### Vectis target consumes the catalog (D6)

The Vectis target's `build` brief gains one new responsibility:

> Before regenerating `composition.yaml`, read `.specify/design-system/components.yaml`. For every claim in `spec.md` / `design.md` that resolves to a confirmed catalog entry, emit the `component: <slug>` directive on the corresponding group in `composition.yaml`. For every confirmed entry, generate one shared component file per in-scope shell tree (`shared/src/components/<slug>.rs` for core view-helpers; `iOS/.../Components/<slug>.swift` for the iOS shell; `android/.../components/<slug>.kt` for the Android shell). Per-screen rendering invokes the shared component instead of inlining.

The Vectis WASI tool's `validate composition` mode gains a check: every claim carrying `component: <slug>` MUST resolve to a confirmed catalog entry, and every confirmed catalog entry MUST have ≥1 generated shared-component file in each in-scope shell tree.

`tokens.yaml` and `assets.yaml` are unchanged — they remain operator-curated. The components catalog joins them as the third operator-curated design-system input.

### Worked example

Plan 1: operator imports five onboarding screens via `screenshots`.

```bash
/spec:plan seed-app source ui=screenshots:./screens/onboarding
```

- `enumerate` produces five candidates (`splash`, `signin`, `task-list`, `archive`, `settings`).
- `/spec:execute` runs refine; extract sees a 3-tab footer on `task-list` + `archive` + `settings` → stage 6 emits `component: tab-bar` on three claims (≥2 instances in the same run).
- Vectis build factors `tab-bar` as a shared component on all three screens.
- Plan merges; baseline screens reference `component: tab-bar`.
- The operator adds `tab-bar: status: confirmed` to `.specify/design-system/components.yaml` so future slices benefit.

Plan 2: operator imports two new screens via `screenshots`.

```bash
/spec:plan profile-screens source ui=screenshots:./screens/profile
```

- `enumerate` produces `profile` + `profile-edit`.
- Refine runs extract. Both new screens carry a structurally matching footer → the operator (or the agent reading the catalog) applies `component: tab-bar` to both claims.
- Vectis build emits `composition.yaml` for both new screens with `component: tab-bar` already wired. No shared-component file regeneration needed (it already exists from plan 1).

### Acceptance for the catalog

- A project without `.specify/design-system/components.yaml` works exactly as today — no behavior change.
- A project with a catalog file containing `tab-bar: status: confirmed` causes the Vectis build to generate a `tab-bar` shared component file per in-scope shell tree.
- `specify slice validate` passes when every `component:` directive resolves to a confirmed entry; fails with `slice-catalog-drift` when it does not.
- A catalog entry with `status: rejected` suppresses the `slice-catalog-drift` finding for `notes.candidate_component` annotations carrying that slug (the operator has intentionally decided not to promote it).

## Implementation plan

Two waves; ship in order.

### Wave A — Tool-owned schemas and standalone preview (D1–D4)

A.1 — Land `Tool::schemas()` accessor and `specify tool schema` verb on the CLI side.
A.2 — Fix the `$id` in each tool's embedded schemas to use the stable convention.
A.3 — Update every plugin-repo brief that cites a local schema path to cite the canonical `$id` URL instead.
A.4 — Delete `adapters/targets/vectis/schemas/*.schema.json`; rewrite the README.
A.5 — Add `check::schema_links` to `specify-authoring` (`links.brief-schema-link-resolve`); register it in `specdev check` alongside the RFC-5 predicate set.
A.6 — Land `specify source preview` with stdout summary and Evidence output.

Steps A.1–A.5 are independent of A.6. A.6 depends on RFC-29 wave A (executable `specify source enumerate` / `specify source extract`).

### Wave B — Component catalog (D5–D6)

B.1 — Land `schemas/design-system/components.schema.json` as a CLI-owned schema.
B.2 — Add `slice-catalog-drift` to `specify slice validate`.
B.3 — Update `adapters/targets/vectis/briefs/build.md` and `briefs/build/composition.md` to read the catalog.
B.4 — Update the Vectis WASI tool's `validate composition` mode for catalog cross-references.
B.5 — Write `docs/explanation/components.md`.

Independent of wave A in principle. No new journal events, no new CLI verbs for catalog management, no new adapter preopens.

## Migration

| Concern | Migration path |
| --- | --- |
| Existing plugin-repo schema copies | Mechanical delete after wave A; `specdev check` (`links.brief-schema-link-resolve`) catches any brief still citing the old path. |
| Existing slices without a catalog | Continue to work. The catalog is opt-in and operator-authored. |
| Workspace mode | No coordinator-root catalog. Each materialised slot owns `.specify/design-system/components.yaml` beside its other project-local design-system files. |
| Existing baseline screens with inlined equivalents | The catalog does not retroactively rewrite baseline `composition.yaml`. The operator schedules a refactor slice when ready. |

`migrate-to-2.0.sh` does not change; RFC-31 ships as a 2.x point release inside the 2.0 contract.

## Non-goals

- **No auto-refactor of baseline `composition.yaml`.** The catalog is declarative input to the build. A future RFC may wire an opt-in `specify component apply <slug>` that rewrites baseline screens, but it is out of scope here.
- **No automatic catalog population.** The catalog is operator-curated, not auto-populated by the CLI. Stage-6 `notes.candidate_component` annotations in Evidence serve as *hints* to the operator; the operator decides what to add. Automated catalog reconciliation (skeleton matching, candidate-to-confirmed promotion, cross-slice instance tracking) may be added in a future RFC if manual curation proves too burdensome, but the manual workflow should be validated first.
- **No catalog CLI verbs.** The catalog follows the `tokens.yaml` / `assets.yaml` pattern: the operator edits the file directly. CLI verbs for catalog management (`list`, `promote`, `reject`, `drop`) are not needed in v1 — the file is small and human-readable. Verbs can be added later if the catalog grows unwieldy.
- **No `$CATALOG_DIR` preopen for source adapters.** Source adapters do not read the catalog. The catalog is consumed at build time by the Vectis target, not at extract time by source adapters. If a future adapter needs catalog awareness, the preopen can be added then.
- **No refactor proposals.** The operator observes when baseline screens should adopt a shared component and schedules a follow-up slice manually. Automated refactor-proposal generation is deferred until the catalog system is validated in practice.
- **No `Operation::Preview` on the closed `SourceOperation` enum.** The preview verb runs existing `enumerate` + `extract` operations. A dedicated `preview` adapter operation with its own brief slot, preopens, and HTML rendering can be added when a second adapter needs visual rendering.
- **No preview golden / regression infrastructure.** `--check` mode, fixture directories, structural diffing, and `--regenerate` are deferred. Adapter authors can compare Evidence files manually or with external diffing tools. The golden infrastructure is worth adding when adapter development scales.
- **No inference report schema.** The preview verb emits Evidence files, which already have a well-defined schema. A separate `InferenceReport` schema with a discriminated-union `inputs` block designed for future adapters is premature with one adapter.
- **No catalog sharing across projects or workspace peers.** Each project's `.specify/design-system/components.yaml` is local to that project's root (including its workspace slot). Cross-project sharing remains out of scope.
- **No per-claim authority overrides at the catalog level.** Authority continues to apply at the Evidence / synthesis layer.
- **No new slice lifecycle.** The slice still progresses `refining → refined → built → merged` regardless.
- **No new plan-time gate.** The operator-stamped `reviewed` gate remains the only plan gate.
- **No replacement of `tokens.yaml` or `assets.yaml`.** Both remain operator-curated. The catalog joins them as a *third* design-system file; it does not subsume them.
- **No live IDE preview.** The preview verb writes files to a directory. Hot-reload integration with Cursor or any editor is out of scope.
- **No versioned schema URLs or `build.rs` rewriting.** The `$id` is a stable logical identifier hardcoded in each schema file. Version-specific `$id` URLs, `build.rs` compile-time rewriting, and GitHub Pages hosting are deferred until an external consumer needs to resolve schemas by URL against specific tool versions.

## Open questions

1. ~~**Catalog scope vs Vectis-target specificity.** The catalog as drafted is Vectis-shaped (regions / containers / leaves). Should it generalise?~~ Resolved: keep it simple. The catalog records component slugs, status, and descriptions. It does not embed structural information (skeletons, digests). The vocabulary is naturally generic enough to support other UI target frameworks when they arrive. Generalisation can follow when a second target needs it.
2. ~~**Catalog file under workspace mode.** Where does the catalog live in workspace mode?~~ Resolved: **per project**. The catalog is design-system state and follows the same project-root routing as slices, baseline specs, and sibling `tokens.yaml` / `assets.yaml`. In workspace mode it lives under the materialised registry slot (`.specify/workspace/<project>/.specify/design-system/components.yaml`), not at the coordinator root.

## Acceptance proof

RFC-31 is complete when:

1. The plugin-repo `adapters/targets/vectis/schemas/` directory contains only the README.
2. `make check` fails with `links.brief-schema-link-resolve` when a brief or reference cites a non-canonical or non-round-tripping `schemas.specify.dev/<tool>/...` URL.
3. `specify tool schema vectis tokens` round-trips byte-identical against the CLI-embedded copy.
4. `specify source preview screenshots --source <dir>` produces Evidence at `${--out}/evidence/screens.yaml` that is byte-equal to what `/spec:refine` would persist for the same input.
5. The preview verb leaves no residue under `.specify/` and no journal event fires.
6. A project with `.specify/design-system/components.yaml` containing `tab-bar: status: confirmed` causes the Vectis build to emit a `tab-bar` shared component file per in-scope shell tree.
7. `specify slice validate` catches every cross-reference between Evidence `component:` directives and the catalog.

## References

- [RFC-25: Workflow](../done/rfc-25-workflow.md) — source/target split; screenshots adapter; `screenshots` and `vectis` first-party plugins; spatial Evidence kinds.
- [RFC-16 (archived): WASI Vectis tool](../done/rfc-16-wasi-vectis.md) — `vectis` WASI tool; embedded schema discipline.
- [RFC-11 (archived): UI Specification Workflow](../done/rfc-11-ui-spec.md) — §A unwired-subset; §G component-directive emission and structural-identity; §H validate modes.
- [RFC-29: Fan-In/Fan-Out](../next/rfc-29-fan-in-fan-out.md) — executable `specify source enumerate` / `specify source extract`; required for the `specify source preview` plumbing.
- [RFC-30: Init bootstrap](../next/rfc-30-init.md) — `specify tool` family discipline; cross-repo schema migration precedent.
- [RFC-5: Framework Developer Tooling](../done/rfc-5-tooling.md) — unified `specdev check` predicate engine; home for `check::schema_links`.
- [Layout inferer contract](../../adapters/targets/vectis/references/layout-inferer-contract.md) — pre-2.0 producer contract; component directive emission policy reused here.
- [Screenshots `extract/pipeline.md`](../../adapters/sources/screenshots/briefs/extract/pipeline.md) — stage-6 component detection.
- [Vectis schemas README](../../adapters/targets/vectis/schemas/README.md) — directory targeted for retirement in wave A.
- [`wasi-tools/vectis/src/validate/engine/shared.rs`](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/src/validate/engine/shared.rs) — "byte-identity discipline" comment; deleted in wave A.
