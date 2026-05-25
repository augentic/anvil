# RFC-31: Vectis / Screenshots Loop Hardening

> Status: Draft - Depends: [RFC-25](../done/rfc-25-workflow.md), [RFC-16 (archived)](../done/rfc-16-wasi-vectis.md) - Compatible with [RFC-29](rfc-29-fan-in-fan-out.md), [RFC-30](rfc-30-init.md) - Enables: standalone screenshots inference review, cross-slice component reuse, pixel-perfect UI consistency

## Abstract

RFC-25 wired the `screenshots` source adapter and the `vectis` target adapter into the Specify 2.0 workflow. The two adapters now produce and consume the spatial `region` / `container` / `leaf` Evidence claim kinds, and `composition.yaml` regenerates inside `vectis.build` rather than living as a Specify artifact. The contract is correct, but daily use of the screenshots → vectis pipeline has exposed three rough edges that none of RFC-25, RFC-29, or RFC-30 close:

1. **Schemas are duplicated across repos.** The Vectis runtime schemas (`tokens.schema.json`, `assets.schema.json`, `composition.schema.json`) live in *two* places — the plugin repo's [`adapters/targets/vectis/schemas/`](../../adapters/targets/vectis/schemas/) and the CLI repo's [`wasi-tools/vectis/embedded/`](https://github.com/augentic/specify-cli/tree/main/wasi-tools/vectis/embedded/). The vendor comment in `wasi-tools/vectis/src/validate/engine/shared.rs` openly says "the upstream is canonical and any edit there must be mirrored here byte-for-byte." Manual mirroring is a permanent error source.
2. **The screenshots adapter has no standalone review surface.** Before 2.0, an operator could run `specify tool run vectis -- validate layout` against a freshly-inferred `layout.yaml` and immediately see whether the inference was correct. 2.0 replaced `layout.yaml` with Evidence claims, which is correct as a workflow change, but it left no way to *iterate on the screenshots adapter itself* without a full plan / refine / build cycle. An operator who wants to refine the prompt, add a better source image, or sanity-check inference quality is forced to run the entire pipeline and read the synthesized `spec.md` to infer what was detected.
3. **Source adapters carry no cross-slice state.** Each `screenshots.extract` invocation only sees the candidate it is asked to extract. There is no mechanism for the adapter to notice that "this screen's tab bar looks structurally identical to a tab bar already living in the baseline." The result: every new slice re-infers the same navigation pattern from scratch, the operator must hand-promote `component:` directives across slices, the Vectis build inlines visually-equivalent code per screen, and the UI drifts pixel-by-pixel from one slice to the next.

This RFC adds three coordinated refinements that stay inside RFC-25's framework:

1. **Tool-owned schemas with a cross-repo reference verb.** The WASI tool that *runs* a schema is its sole owner. Plugin briefs reference schemas by their canonical `$id` URL and operators can extract any tool-owned schema with a new `specify tool schema <tool> <name>` verb. The plugin repo's schema copies retire.
2. **A `specify source preview` verb.** Operators can run a source adapter against a directory of inputs in complete isolation of the workflow — no plan, no slice, no `change.md`. The output is a deterministic inference report (Evidence + per-claim provenance) plus, for source adapters that opt in, an HTML / annotated-image render. A `--check` mode compares the report against a golden snapshot for regression testing.
3. **A project-level component catalog.** A new operator-curated artifact at `.specify/design-system/components.yaml` records confirmed and candidate shared components across slices. The screenshots adapter reads the catalog through a new read-only `$CATALOG_DIR` preopen; it auto-confirms `component:` promotions when the catalog already contains a matching skeleton and emits a *refactor proposal* when a new skeleton matches multiple baseline screens. The Vectis target reads the catalog at build time and factors shared components in code.

None of the three asks introduces a new lifecycle, a new slash command, or a new slice ceremony. They refine surfaces that already exist.

## Motivation

The findings this RFC closes, in order of magnitude:

| Magnitude | Finding | Current state | RFC-31 resolution |
| --- | --- | --- | --- |
| Small | Vectis runtime schemas duplicated across repos. | `adapters/targets/vectis/schemas/*.schema.json` and `wasi-tools/vectis/embedded/*.schema.json` are byte-identical copies kept in sync by hand. The CLI source comment names the discipline explicitly. The `$id` URLs even disagree on whether they live under `adapters/vectis/` or `targets/vectis/`. | Schemas are owned by the tool that runs them. The plugin repo holds no schema bodies; briefs cite the tool's canonical `$id`. A new `specify tool schema` verb extracts any tool-owned schema on demand. Cross-repo CI verifies cited `$id`s resolve. |
| Medium | Screenshots inference cannot be reviewed in isolation. | `screenshots.extract` runs only inside `/spec:refine`. Operators iterating on prompt quality or source-image quality must drive a full plan → refine cycle and read synthesized `spec.md` to guess what the adapter saw. | `specify source preview <source-key>` runs `enumerate` + `extract` against a directory with no plan or slice and emits a structured inference report. Optional `briefs/preview.md` lets a source adapter ship a renderer; for `screenshots`, this produces annotated PNGs and an HTML index. `--check` compares against goldens for regression. |
| Large | Source adapters have no cross-slice state. | Each `screenshots.extract` invocation sees only one candidate. The stage-6 component-detection rule requires ≥2 matching skeletons inside the *same run*. Common navigation, headers, and toolbars across slices are re-discovered from scratch and never promoted to shared components. Vectis build inlines per-screen, UI drifts visually. | A project-level `.specify/design-system/components.yaml` catalog records confirmed and candidate shared components. The screenshots adapter sees it through a read-only `$CATALOG_DIR` preopen, auto-confirms catalog hits, and emits refactor proposals when a new skeleton matches existing baseline screens. The Vectis target reads the catalog at build time. |

The schema-duplication finding is the simplest to land; the cross-cutting components finding is the largest and shapes much of the downstream code-generation contract. Stacking them in one RFC is deliberate — all three live at the screenshots / vectis seam and benefit from a single migration window.

## Principles

1. **Refine, do not rebuild.** Every surface added here lives inside the RFC-25 vocabulary (source / target adapters, candidate / Evidence, slice lifecycle, plan). Nothing introduces a new slash command, a new lifecycle state, or a second writer for any existing artifact.
2. **One source of truth per schema.** The tool that consumes a schema is the only repo that contains its body. Everywhere else cites the canonical `$id`. CI enforces it.
3. **Source adapters get a workbench.** Operators must be able to exercise a source adapter against real inputs without invoking the workflow. Adapter quality is then directly debuggable.
4. **Cross-cutting state lives outside the slice.** Component reuse is a *project-level* concern. Slices read from and write into the catalog, but the catalog survives every slice merge and informs every future extract and every future build.
5. **Refactor is operator-triggered.** The catalog and the screenshots adapter together *propose* refactors; only an operator-scheduled slice executes one. RFC-31 v1 does not auto-rewrite baseline `composition.yaml` behind anyone's back.
6. **Backward compatibility within 2.0.** Projects without `.specify/design-system/components.yaml` work exactly as today; the catalog is opt-in and self-bootstraps on the first extract that emits a `notes.candidate_component` annotation — single-instance skeletons seed the catalog as `candidate` entries so that a later slice's second instance auto-promotes.

## Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 Tool-owned schemas** | Every JSON Schema is owned by the repo of the WASI tool (or the CLI) that runs it. Plugin briefs MUST NOT contain schema bodies. | Delete `adapters/targets/vectis/schemas/*.schema.json`; replace with a `README.md` that links to canonical `$id` URLs and documents `specify tool schema`. Update every brief that cites a vendored schema path. |
| **D2 `specify tool schema` verb** | The CLI exposes `specify tool schema <tool> <name> [--format json\|yaml]` that prints any embedded schema to stdout. | Add `src/commands/tool/schema.rs`; route through `Tool::resolve` and the tool's declared schema registry; works for `vectis` and `contract` on day one. |
| **D3 Schema `$id` normalisation** | Every tool-owned schema's `$id` URL points at the CLI repo's published release path for that tool version, e.g. `https://schemas.specify.dev/vectis/0.3.0/tokens.schema.json`. | Add a `versions.toml`-driven `$id` rewrite in each tool's `build.rs`; add a `make check` predicate in the plugin repo that every brief-cited schema URL resolves. |
| **D4 `specify source preview`** | The CLI exposes `specify source preview <adapter> --source <path>` that runs `enumerate` + `extract` against a directory, with no plan and no slice required. | Add `src/commands/source/preview.rs`; routes through `SourceAdapter::resolve` and the RFC-29 source runner; writes an inference report to `--out` (default `./.specify-preview/`). |
| **D5 Optional `preview` adapter operation** | Source adapters MAY declare an optional `briefs/preview.md` that the CLI invokes after extract to produce a human-reviewable render (HTML, annotated images, prose). | Extend `SourceOperation` with a `Preview` variant; treat absent `briefs.preview` as "fall back to a textual report renderer in the CLI"; ship a `preview.md` for `screenshots` that draws claim bboxes onto source images. |
| **D6 Preview goldens** | `specify source preview --check <fixture>` compares the live inference report against a stored golden and exits non-zero on drift. | Add `InferenceReport` schema; teach the preview command to read `expected/report.json` and diff structurally; reuse `tests/fixtures/sources/screenshots/task-list-two-screen/` as the first golden. |
| **D7 Project-level component catalog** | A project MAY carry `.specify/design-system/components.yaml`. Entries record canonical skeletons, known variants, asset references, and per-entry status (`candidate`, `confirmed`, `rejected`). The CLI owns both the schema and lifecycle writes; operators may hand-edit `usage_hint` and `description` fields. The schema is CLI-owned (not tool-owned) because the catalog is a project-level artifact mediated by CLI verbs, not a tool runtime input. | Add `schemas/design-system/components.schema.json` to the CLI repo (alongside `evidence.schema.json` and `plan.schema.json`); add `specify component {list, promote, reject, drop}` verbs; expose the catalog to source adapter operations as the read-only `$CATALOG_DIR` preopen. |
| **D8 Catalog-aware extract** | After every `screenshots.extract` invocation, the CLI runs a deterministic catalog reconciliation pass that (a) promotes catalog candidates to confirmed when the new Evidence supplies the missing ≥2-instance match, (b) writes catalog-confirmed `component:` directives back into the Evidence body before persistence, and (c) records single-instance `notes.candidate_component` skeletons as `candidate` catalog entries so that a future extract's second instance auto-promotes without operator intervention. | Add `crates/domain/src/catalog/reconcile.rs`; runs at the same point as `evidence.schema.json` validation; never mutates claims that already carry `component:` set by the adapter. |
| **D9 Refactor proposals** | When catalog reconciliation finds that a new confirmed component matches ≥1 baseline screen, the CLI writes `refactor-proposal.md` into the slice and emits a `slice.refactor.proposed` journal event. The operator schedules a follow-up slice; no auto-edit of baseline `composition.yaml` ships in v1. | Add `RefactorProposal` DTO and renderer; document the follow-up slice pattern in `docs/explanation/components.md`. |
| **D10 Vectis catalog consumer** | The Vectis target's `build` brief reads the component catalog and factors shared components in generated code and `composition.yaml`. The scaffold tool's template registry is data-driven by in-scope platform tokens so that new shells (e.g. `web` → React+TypeScript) require only a template addition, not orchestrator code changes. | Update `adapters/targets/vectis/briefs/build.md` and `briefs/build/composition.md` to require catalog reads; teach the scaffold tool to emit one component file per confirmed catalog entry per in-scope platform, dispatching to per-platform templates. |

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
specify tool schema vectis composition --format yaml

# Iterate on a source adapter against real inputs, with no plan or slice.
specify source preview screenshots --source ./design-explorations/onboarding
specify source preview screenshots --source ./design-explorations/onboarding --render html --out ./preview
specify source preview screenshots --check tests/fixtures/sources/screenshots/task-list-two-screen

# Inspect and curate the cross-slice component catalog.
specify component list
specify component promote tab-bar
specify component reject card-row --reason "false-positive on stage-6 detection"
```

The component-catalog write surface is intentionally minimal in v1: most catalog state flows through `specify source extract`'s reconciliation pass, and operators only reach for `specify component *` when they disagree with what the CLI proposed.

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
Validates against [`vectis.tokens.schema.json`](https://schemas.specify.dev/vectis/0.3.0/tokens.schema.json).
```

Operators and agents pull the body on demand:

```bash
specify tool schema vectis tokens > /tmp/tokens.schema.json
specify tool schema vectis composition --format yaml | less
```

### `specify tool schema` (D2)

```text
specify tool schema <tool> <name> [--format json|yaml] [--version <semver>]
```

- `<tool>` resolves through the same path as `specify tool run` (declared `tools[]`).
- `<name>` is the kebab-case schema id; the tool advertises its registry through a new `Tool::schemas()` -> `&[(name, sha256, body)]` accessor.
- `--format yaml` re-emits the JSON Schema as YAML for inline review; `--format json` (default) pretty-prints with stable key ordering.
- `--version` pins the tool version; defaults to the version declared in the project's `targets.yaml` (single-project mode) or in the requesting plugin's `adapter.yaml` `tools[]` entry.

Exits:

- `0` — schema emitted to stdout.
- `2` — unknown tool, unknown schema, or version mismatch.

### `$id` normalisation (D3)

Each WASI tool's `build.rs` rewrites the `$id` field of every embedded schema at compile time to the form:

```text
https://schemas.specify.dev/<tool>/<version>/<name>.schema.json
```

`<version>` is the tool version from `Cargo.toml` (already mirrored in `wasi-tools/vectis/embedded/versions.toml`). The published URL resolves to a GitHub Pages mirror of the CLI repo's release tag; operators offline can read identical content with `specify tool schema`.

Plugin-repo CI gains a `make check` predicate:

```text
brief-schema-link-resolves: every URL matching schemas.specify.dev/<tool>/<version>/...
                            in any brief or reference must round-trip through
                            `specify tool schema <tool> <name>` byte-for-byte.
```

### Migration

| Step | Action |
| --- | --- |
| 1 | Land `specify tool schema` in the CLI; ship `Tool::schemas()` for `vectis` and `contract`. |
| 2 | Land the `$id` rewrite in each tool's `build.rs`. |
| 3 | Update every plugin-repo brief that cites a local schema path to cite the canonical URL instead. |
| 4 | Delete `adapters/targets/vectis/schemas/*.schema.json`; replace `adapters/targets/vectis/schemas/README.md` with the URL list and a `specify tool schema` quickstart. |
| 5 | Add the `brief-schema-link-resolves` predicate to `scripts/check.ts` and gate `make check` on it. |
| 6 | Remove the "byte-identity discipline" comment block from `wasi-tools/vectis/src/validate/engine/shared.rs`; the schemas are now first-class CLI assets. |

After migration, the plugin repo carries zero `.schema.json` files for tool-owned artifacts. Framework-level schemas (adapter, source, target, evidence, plan, slice/fusion) stay where they are — they are CLI-owned and already follow this pattern.

## Screenshots adapter standalone validation (D4, D5, D6)

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
                                 [--assets <path>]
                                 [--tokens <path>]
                                 [--candidate <id> ...]
                                 [--render <text|json|html>]
                                 [--out <path>]
                                 [--check <fixture-path>]
                                 [--format json]
```

- `<adapter>` resolves against the adapter loader. The CLI runs the adapter's `enumerate` and `extract` operations under the RFC-25 sandbox profile (with the additional `$CATALOG_DIR` preopen described in §Cross-cutting components when a project exists).
- `--source` is the bound path that becomes `$SOURCE_DIR`. Required.
- `--assets` / `--tokens` are optional sibling preopens for source adapters that consult `design-system/assets.yaml` / `design-system/tokens.yaml`. The CLI exposes them at the same paths the brief expects.
- `--candidate` restricts extraction to a subset; defaults to "every candidate `enumerate` produced."
- `--render` selects the renderer (`text` default; `json` for machine consumers; `html` for adapters that ship `briefs/preview.md`).
- `--out` is the report destination directory; defaults to `./.specify-preview/`.
- `--check` is the regression mode (§Preview goldens).

The verb is workflow-free: nothing is written into `.specify/`, no lifecycle moves, no journal events fire. Output lives entirely under `--out`.

### Inference report (D4)

The report is a structured document the CLI always emits, regardless of whether the adapter ships `preview.md`:

```yaml
version: 1
adapter: screenshots
adapter-version: 1
source: ./design-explorations/onboarding
generated-at: 2026-05-25T12:00:00Z
candidates:
  - id: task-list
    summary: "Task list: today's open tasks for the signed-in user."
    inputs:
      type: screenshots
      images:
        - path: task-list-populated.png
        - path: task-list-empty.png
          state: empty
    evidence:
      claims-by-kind:
        region: 4
        container: 6
        leaf: 11
      promoted-components: []
      candidate-components:
        - slug: task-row
          instances: 1
          reason: stage-6 conservative emission
      gaps:
        - claim-id: task-list.body.tasks.task-row.title
          note: confirm text
warnings: []
errors: []
```

The `inputs` block is a discriminated union keyed by `type`. Each source adapter defines its own input descriptor shape; the schema uses `oneOf` against the `type` discriminator. Known types in v1:

| `type` | Adapter | Adapter-specific fields |
| --- | --- | --- |
| `screenshots` | `screenshots` | `images[]` with `path` and optional `state`. |

Future visual adapters extend `inputs` additively — e.g. a `figma-frames` type would carry `node_ids[]` and `page`; a `rendered-code` type would carry `source_files[]` and `viewport`. Adding a new `type` is a non-breaking schema extension (new `oneOf` variant); existing consumers that only inspect `evidence` are unaffected.

Schema: `schemas/source/inference-report.schema.json`. The full Evidence body is emitted alongside the report at `${--out}/evidence/<source-key>.yaml`, byte-identical to what `/spec:refine` would persist.

### Optional `preview.md` adapter operation (D5)

A source adapter MAY declare:

```yaml
briefs:
  enumerate: briefs/enumerate.md
  extract:   briefs/extract.md
  preview:   briefs/preview.md   # optional
```

`Operation::Preview` is added to the closed `SourceOperation` enum. When `briefs.preview` is present and `--render html` is requested, the CLI runs the brief after extract with the additional symbols:

| Symbol | Meaning |
| --- | --- |
| `$REPORT_PATH` | The CLI-written inference report (read-only). |
| `$EVIDENCE_DIR` | The CLI-written Evidence directory (read-only). |
| `$RENDER_DIR` | Per-candidate writable scratch where the brief deposits HTML and rendered images. |

For `screenshots`, `briefs/preview.md` draws every emitted `region` / `container` / `leaf` claim's `bbox` onto the source image, colour-coded by kind and labelled with the claim id; the brief writes one annotated PNG per source image plus an `index.html` that pairs the report's textual summary with the annotated images side by side.

This forces a constraint on `screenshots.extract`: claims whose `bbox` is unknown render as a "no-bbox" badge in the preview, which surfaces directly back to the brief author as "the inference did not measure this region." That is an *intended* feedback loop — the operator immediately sees where the brief is guessing geometry vs measuring it.

### Preview goldens (D6)

A fixture directory has a fixed shape:

```text
tests/fixtures/sources/<adapter>/<name>/
  source/                    # adapter-specific inputs (images for screenshots, mocked API trees for figma, source snapshots for code adapters)
  design-system/             # optional tokens.yaml / assets.yaml
  expected/
    report.json              # canonical InferenceReport
    evidence/screens.yaml    # canonical Evidence
```

The `source/` directory is the adapter-specific input set — the CLI passes it as `$SOURCE_DIR` and delegates interpretation entirely to the adapter's `enumerate` + `extract` operations. For `screenshots`, this is a directory of PNG/JPEG files; future adapters define their own input conventions (e.g. a Figma adapter may carry a mocked API response tree; a legacy-code adapter may carry a rendered-viewport snapshot set). The `--check` harness is source-format-agnostic — it only compares `expected/` outputs.

`specify source preview screenshots --check tests/fixtures/sources/screenshots/<name>` runs the live adapter against `source/`, produces a fresh report, and structurally diffs it against `expected/report.json` and `expected/evidence/screens.yaml`. Drift exits non-zero with a unified-diff-style summary. The CLI never overwrites goldens; refresh is `--check --regenerate` and is gated by a confirmation prompt.

This becomes the harness for both:

- **Adapter authors** — every change to `briefs/extract.md` or `briefs/extract/pipeline.md` must keep the goldens green or amend them deliberately.
- **Operators** — when the live adapter drifts on a specific input set, the operator captures the drift as a new fixture, opens an issue, and the adapter author has a reproducible test case.

### Acceptance for the screenshots workbench

- `specify source preview screenshots --source <dir>` succeeds against a fresh checkout with no `.specify/` present.
- The same command produces an inference report whose `evidence/screens.yaml` is byte-equal to what `/spec:refine` would persist for the same input.
- `--render html` produces an `index.html` per candidate with annotated source images and a textual summary.
- `--check <fixture>` exits 0 on the in-tree `task-list-two-screen` fixture and non-zero with a structured diff after a deliberate brief edit.
- The verb leaves no residue under `.specify/` and no journal event fires.

## Cross-cutting components and the component catalog (D7, D8, D9, D10)

### Current state

The screenshots adapter's `extract/pipeline.md` stage 6 ("Detect candidate components conservatively") only fires within one `extract` call: a `component: <slug>` directive lands when ≥2 structurally identical groups appear *in the same run*. Across slices and across runs, the adapter has no memory:

- The first slice's `task-list` screen has a tab bar. Stage 6 sees one instance → emits `notes.candidate_component: tab-bar`. No promotion.
- The second slice's `settings` screen has the same tab bar. The new extract sees one instance in *its* run → emits the same note. Still no promotion.
- The Vectis build inlines the tab bar twice. Visually they drift. The operator never knew the catalog opportunity existed.

The pipeline's stage 6 wording — "across screens of the *same run* (within `<candidate-id>` plus any prior candidates extracted for the same plan)" — already gestures at "same plan," but there is no on-disk state that survives the plan. Once the plan archives, the next plan starts blind.

### Resolution

**A project-level component catalog persists across plans.** It is not a Specify artifact (no slice owns it); it is a project-curated registry the CLI mediates writes to and that the screenshots adapter and the Vectis target both read.

#### File

```text
.specify/design-system/components.yaml
```

The file is opt-in: projects that never produce one keep working exactly as today. The CLI creates the file when the first `candidate` entry is recorded — a single-instance skeleton tagged `notes.candidate_component` by stage 6 is enough to seed the catalog; a subsequent slice whose extract produces a matching skeleton auto-promotes the entry to `confirmed`.

Schema sketch (`schemas/design-system/components.schema.json`):

```yaml
version: 1
components:
  tab-bar:
    status: confirmed              # candidate | confirmed | rejected
    description: "Bottom navigation across the primary app sections."
    first-seen-slice: onboarding-screens
    first-seen-at: 2026-05-25T12:00:00Z
    confirmed-at: 2026-05-30T09:00:00Z
    skeleton:                      # authoritative structural shape from claims
      kind: container
      container: group
      direction: row
      children:
        - kind: leaf
          leaf: icon-button
          role: tab
        - kind: leaf
          leaf: icon-button
          role: tab
        - kind: leaf
          leaf: icon-button
          role: tab
    skeleton-digest: sha256:a7f3…  # derived from normalised skeleton per structural-identity.md
    variants:                      # asset-family variants (RFC-31 + screenshots stage 5b)
      - id: home
        assets: [nav-home-default, nav-home-active]
      - id: search
        assets: [nav-search-default, nav-search-active]
    instances:                     # back-references for refactor proposals
      - slice: onboarding-screens
        screen: task-list
        claim-id: task-list.footer.nav
      - slice: onboarding-screens
        screen: archive
        claim-id: archive.footer.nav
```

The schema is **CLI-owned** (alongside `evidence.schema.json`, `plan.schema.json`, and other framework-level schemas) and lives in the CLI repo at `schemas/design-system/components.schema.json`. The `specify component` verbs validate against it; briefs cite it by its `$id` URL (`https://schemas.specify.dev/cli/<version>/design-system/components.schema.json`). Unlike the Vectis runtime schemas (which are tool-owned per D1), the component catalog is a project-level artifact whose lifecycle the CLI mediates directly — the same ownership pattern as `plan.yaml` and Evidence. This avoids coupling the catalog's `$id` to the Vectis tool version and ensures future non-Vectis targets that consume the catalog do not require a schema migration.

#### Catalog as adapter input ($CATALOG_DIR)

RFC-25 §Sandboxing enumerates the source-adapter filesystem grant. RFC-31 adds one optional read-only preopen:

| Root | Mode | Contents |
| --- | --- | --- |
| `$CATALOG_DIR` | read-only | `.specify/design-system/` — the operator-curated catalog plus sibling `tokens.yaml` / `assets.yaml`. Absent when the project has no catalog yet. |

The CLI exposes the preopen to *every* source adapter that opts in via `adapter.yaml`:

```yaml
needs:
  - catalog
```

Adapters that do not opt in see no `$CATALOG_DIR` and behave exactly as today. The `screenshots` adapter opts in; `intent`, `documentation`, and `code-typescript` do not in v1.

#### Catalog-aware extract (D8)

After the adapter returns Evidence to the CLI, the CLI runs a deterministic reconciliation pass *before* persisting Evidence:

1. For each catalog entry, walk the new Evidence container claims looking for skeleton matches using the structural-identity rules defined in `docs/reference/structural-identity.md` (ordered nested item kinds, `*-when` key presence, `platforms.*` exemption).
2. For each match:
   - If the catalog entry is `confirmed`: rewrite the matched claim's body to add `component: <slug>` (replacing any `notes.candidate_component` on the same claim).
   - If the catalog entry is `candidate` and this Evidence supplies the missing instance to cross the ≥2 threshold: promote the catalog entry to `confirmed`, write `confirmed-at`, rewrite all current and prior instance claims to `component: <slug>`, and emit a `slice.component.confirmed` journal event.
3. For each *new* skeleton in the Evidence that appears ≥2 times in this single run and has no catalog entry: write a new `confirmed` entry to the catalog (stage 6 already emitted `component: <slug>` on the claims — the catalog should reflect that). Record `first-seen-at`, `confirmed-at`, the skeleton, and all instances. Emit a `slice.component.confirmed` journal event.
4. For each *single-instance* new skeleton that carries `notes.candidate_component`: write a new `candidate` entry to the catalog with the skeleton, the source slice, screen, and `first-seen-at`. No `component:` directive is written on the claim — the claim keeps its `notes.candidate_component` annotation until a future extract supplies a second instance and triggers step 2's promotion path. Emit a `slice.component.candidate` journal event. This is cheap: `candidate` entries are inert metadata — they do not trigger `component:` rewrites, shared-component file generation, or refactor proposals. Only `confirmed` entries have downstream effects.

**Instance uniqueness.** Instances are counted by unique `(slice, screen)` pairs, not by source adapter. When a project binds multiple visual source adapters against the same screen set (e.g. `screenshots` + a future `figma` adapter for cross-validation), structurally-identical claims from different adapters targeting the same logical screen contribute *one* instance, not two. The deduplication key is the screen slug derived from the candidate id; the source key is metadata on the instance record but does not inflate the count. This prevents premature auto-promotion when operators bind redundant sources for validation.

The reconciliation pass is pure CLI code; the adapter brief does not change. The Evidence schema does not change (catalog rewrites only set fields the schema already accepts). The pass is idempotent: re-running extract against the same source inputs produces byte-identical Evidence.

`specify slice validate` adds one finding:

| Finding | Meaning |
| --- | --- |
| `slice-catalog-drift` | The Evidence persists a claim with `component: <slug>` where `<slug>` is not in `.specify/design-system/components.yaml`, or the catalog entry is `rejected`. |

#### Refactor proposals (D9)

When reconciliation promotes a catalog entry to `confirmed` *and* the catalog already records ≥1 instance from a previously-merged baseline screen, the CLI writes a refactor proposal into the slice:

```text
.specify/slices/<slice>/refactor-proposal.md
```

```markdown
# Refactor proposal: tab-bar component promotion

Catalog entry `tab-bar` was promoted to `confirmed` during refinement of
slice `onboarding-screens`. The catalog records baseline instances on:

- `baseline.task-list.footer.nav` (slice `seed-app`, merged 2026-05-20)
- `baseline.archive.footer.nav` (slice `seed-app`, merged 2026-05-20)

The current slice's `composition.yaml` will use `component: tab-bar` for
its new screens. The baseline screens above continue to inline an
equivalent group and will visually drift.

## Recommended follow-up

Open a refactor slice that rebinds the affected baseline screens to use
the shared component:

```bash
/spec:plan tab-bar-refactor source ui=intent
```

Bind `intent` to "rewrite baseline tab bars to use the shared
`tab-bar` component" and let the Vectis build regenerate
`composition.yaml` for the listed screens.

## Catalog instance list

(machine-readable, consumed by future tooling)

instances:
  - slice: seed-app
    screen: task-list
    claim-id: task-list.footer.nav
  - slice: seed-app
    screen: archive
    claim-id: archive.footer.nav
```

`/spec:refine` flags the file in its closing summary; the slice still progresses to `refined` regardless. The proposal is advisory in v1 — the operator decides whether and when to open the follow-up slice. No automatic rewrite of baseline `composition.yaml` ships in this RFC.

Journal events:

| Event | When |
| --- | --- |
| `slice.component.confirmed` | Catalog reconciliation promoted a candidate to confirmed. |
| `slice.component.candidate` | Catalog reconciliation recorded a new candidate. |
| `slice.refactor.proposed` | `refactor-proposal.md` was written into the slice. |

#### Operator catalog verbs

```text
specify component list                                  # text or json table of every entry
specify component show <slug>                           # full entry including instances + skeleton
specify component promote <slug> [--reason <text>]      # candidate -> confirmed
specify component reject <slug>  [--reason <text>]      # any -> rejected (suppresses future auto-confirm)
specify component drop    <slug> [--reason <text>]      # remove the entry entirely (rare)
```

Promotion / rejection writes are atomic (stage-then-rename) and fire the corresponding journal events. Single-instance candidates are auto-recorded by the reconciliation pass whenever stage 6 emits `notes.candidate_component` on a claim; the operator reaches for `specify component promote` only when they want to force-confirm a candidate before a second instance appears organically.

#### Vectis target consumes the catalog (D10)

The Vectis target's `build` brief gains one new responsibility:

> Before regenerating `composition.yaml`, read `.specify/design-system/components.yaml`. For every claim in `spec.md` / `design.md` that resolves to a confirmed catalog entry, emit the `component: <slug>` directive on the corresponding group in `composition.yaml`. For every confirmed entry, generate exactly one shared component file in the appropriate shell tree (`shared/src/components/<slug>.rs` for core view-helpers; `iOS/.../Components/<slug>.swift` for the iOS shell; `android/.../components/<slug>.kt` for the Android shell). Per-screen rendering invokes the shared component instead of inlining.

The Vectis WASI tool's `validate composition` mode gains a check: every claim carrying `component: <slug>` MUST resolve to a confirmed catalog entry, and every confirmed catalog entry MUST have ≥1 generated shared-component file in each in-scope shell tree.

The Vectis target's scaffold tool gains a `scaffold component <slug>` mode that emits the per-shell shared-component scaffold. The scaffold mode iterates in-scope platform tokens from `proposal.md` `## Platforms` and dispatches to per-platform templates — the template registry is data-driven so that adding a new platform (e.g. `web` → React+TypeScript `.tsx` components) is a template addition, not a code change to the scaffold orchestrator. The Vectis build orchestrator iterates the catalog and invokes the scaffold mode per confirmed entry on each build.

`tokens.yaml` and `assets.yaml` are unchanged — they remain operator-curated. The components catalog joins them as the third operator-curated design-system input.

### Worked example

Plan 1: operator imports five onboarding screens via `screenshots`.

```bash
/spec:plan seed-app source ui=screenshots:./screens/onboarding
```

- `enumerate` produces five candidates (`splash`, `signin`, `task-list`, `archive`, `settings`).
- `/spec:execute` runs refine; extract sees a 3-tab footer on `task-list` + `archive` + `settings` → stage 6 emits `component: tab-bar` on three claims and the CLI writes catalog entry `tab-bar: status: confirmed`.
- Vectis build factors `tab-bar` as a shared component on all three screens.
- Plan merges; baseline screens reference `component: tab-bar`.

Plan 2: operator imports two new screens via `screenshots`.

```bash
/spec:plan profile-screens source ui=screenshots:./screens/profile
```

- `enumerate` produces `profile` + `profile-edit`.
- Refine runs extract. The catalog contains `tab-bar: status: confirmed`. Both new screens carry a structurally matching footer group → reconciliation rewrites both claims to `component: tab-bar` *without* the adapter needing to see ≥2 instances in this single run.
- Vectis build emits `composition.yaml` for both new screens with `component: tab-bar` already wired. No shared-component file regeneration needed (it already exists from plan 1).

Plan 3: operator imports a third batch where the screens carry a different *4-tab* footer.

- Reconciliation finds the new skeleton has 4 children, not 3 → does not match `tab-bar`. Stage 6 sees ≥2 instances in this run → emits `component: tab-bar-v2` and writes a new candidate to the catalog.
- The CLI also notices the new skeleton matches no baseline screen. No refactor proposal fires.
- Operator runs `specify component show tab-bar-v2` to inspect, decides the 4-tab variant should replace the 3-tab one fleet-wide, and opens a refactor slice manually. (RFC-31 v1 does not auto-propose this.)

### Worked example: single-instance-per-slice promotion

A plan where each slice binds exactly one screen, and no single extract run ever sees ≥2 instances of the shared structure.

```bash
/spec:plan app-screens source ui=screenshots:./screens
```

`enumerate` produces three candidates as separate slices: `task-list`, `settings`, `profile`. All three screens carry a structurally identical 3-tab footer, but `/spec:execute` refines them one at a time.

- **Slice 1** (`task-list`): extract sees one screen with a 3-tab footer. Stage 6 sees one instance → emits `notes.candidate_component: tab-bar` (no `component:` directive). CLI reconciliation finds no catalog → records `tab-bar: status: candidate` with the skeleton and emits `slice.component.candidate`. The claim keeps `notes.candidate_component`.
- **Slice 2** (`settings`): extract sees one screen with the same footer. Stage 6 sees one instance → emits `notes.candidate_component: tab-bar`. CLI reconciliation finds `tab-bar: status: candidate` in the catalog, structurally matches the skeleton, and this is the second unique `(slice, screen)` instance → promotes to `tab-bar: status: confirmed`, writes `confirmed-at`, rewrites this claim to `component: tab-bar`, and emits `slice.component.confirmed`. The Vectis build generates a shared `tab-bar` component file.
- **Slice 3** (`profile`): extract sees one screen with the same footer. Stage 6 sees one instance → emits `notes.candidate_component: tab-bar`. CLI reconciliation finds `tab-bar: status: confirmed` → rewrites the claim to `component: tab-bar`. The Vectis build reuses the existing shared component.

Without proactive candidate recording, all three slices would finish with only `notes.candidate_component` and the catalog would never exist.

### Acceptance for the catalog

- A fresh project produces `.specify/design-system/components.yaml` with `candidate` entries after a refine pass on a one-screen slice whose stage 6 emits `notes.candidate_component` annotations. A one-screen slice with no candidate-component annotations produces no catalog file.
- A two-screen slice whose screens share a 3-tab footer auto-creates the catalog with a `confirmed` `tab-bar` entry.
- A follow-up single-screen slice with a matching footer auto-promotes a `candidate` entry to `confirmed` (if not already confirmed) and auto-applies `component: tab-bar` to the new screen's claims.
- Two sequential single-screen slices that each carry a structurally identical footer — but never share a run — produce a `confirmed` `tab-bar` entry after the second slice's extract: the first slice seeds the `candidate`, the second slice's reconciliation pass crosses the ≥2 threshold and promotes.
- A follow-up single-screen slice with a *non-matching* footer leaves existing catalog entries unchanged (though it may add new `candidate` entries for its own candidate-component annotations).
- `specify component reject tab-bar` flips the entry to `rejected`; subsequent extracts no longer rewrite claims to `component: tab-bar` and `slice-catalog-drift` fires if a stale Evidence file still carries the directive.
- `specify slice validate` passes when every `component:` directive resolves to a confirmed entry; fails with `slice-catalog-drift` when it does not.
- Re-running extract against unchanged source images produces byte-identical Evidence (catalog reconciliation is deterministic).

## Implementation plan

Three independent waves; ship in numbered order but a downstream consumer needing a later wave does not block on earlier ones beyond the noted dependencies.

### Wave A — Tool-owned schemas (D1-D3)

A.1 — Land `Tool::schemas()` accessor and `specify tool schema` verb on the CLI side.
A.2 — Land the `$id` rewrite in each WASI tool's `build.rs`.
A.3 — Update every plugin-repo brief to cite canonical schema URLs.
A.4 — Delete `adapters/targets/vectis/schemas/*.schema.json`; rewrite the README.
A.5 — Add `brief-schema-link-resolves` to `scripts/check.ts`.

This wave is independent of waves B and C and is the cheapest win.

### Wave B — Standalone preview surface (D4-D6)

B.1 — Add `Operation::Preview` to the `SourceOperation` enum (additive).
B.2 — Add `schemas/source/inference-report.schema.json`.
B.3 — Land `specify source preview` with the `text` / `json` renderers in-CLI.
B.4 — Add the `screenshots` adapter's `briefs/preview.md` for the `html` renderer.
B.5 — Land `--check <fixture>` against the existing `task-list-two-screen` golden.

Depends on RFC-29 wave A (executable `specify source enumerate` / `specify source extract`). Without RFC-29, `specify source preview` either ships against the agent-run fallback or waits.

### Wave C — Component catalog and refactor proposals (D7-D10)

C.1 — Land `schemas/design-system/components.schema.json` as a CLI-owned schema (alongside `evidence.schema.json` and `plan.schema.json`; the catalog is a project-level artifact, not a tool runtime input).
C.2 — Land `crates/domain/src/catalog/{reconcile,store}.rs` with the deterministic reconciliation pass.
C.3 — Add `$CATALOG_DIR` preopen to the source-adapter sandbox; surface it only for adapters that declare `needs: [catalog]`.
C.4 — Update `adapters/sources/screenshots/adapter.yaml` to declare `needs: [catalog]`.
C.5 — Land `specify component {list, show, promote, reject, drop}` verbs.
C.6 — Add `slice-catalog-drift` to `specify slice validate`.
C.7 — Add `slice.component.{confirmed,candidate}` and `slice.refactor.proposed` to the closed `EventKind` enum.
C.8 — Update `adapters/targets/vectis/briefs/build.md` and `briefs/build/composition.md` to read the catalog.
C.9 — Update the Vectis WASI tool's `validate composition` mode for catalog cross-references.
C.10 — Add `vectis scaffold component <slug>` and wire it into the Vectis build orchestrator.
C.11 — Write `docs/reference/structural-identity.md` — the normative definition of skeleton structural identity (ordered nested item kinds, `*-when` key presence, `platforms.*` exemption). Both `catalog/reconcile.rs` and every source adapter's stage-6 brief cite this document as authoritative. Extracting the definition ensures future visual adapters (Figma, legacy-code) have a single reference for the contract their claims must satisfy to participate in catalog reconciliation.
C.12 — Write `docs/explanation/components.md`.

Depends on wave A (the components schema follows the same single-source rule). Independent of wave B in principle, but the screenshots `preview.md` becomes substantially more useful with catalog-aware extract — recommended to ship B before C.

## Migration

| Concern | Migration path |
| --- | --- |
| Existing plugin-repo schema copies | Mechanical delete after wave A; CI predicate catches any brief still citing the old path. |
| Existing slices without a catalog | Continue to work. The catalog auto-bootstraps when the first ≥2-instance skeleton appears in an extract run. |
| Existing baseline screens with inlined equivalents | Reconciliation does not retroactively rewrite baseline `composition.yaml`. The operator schedules a refactor slice when ready (RFC-31 v1 is opt-in by design). |
| Existing `screenshots` golden fixtures | The single-fixture `task-list-two-screen` directory grows an `expected/` subdirectory; existing tests continue to read the same `evidence/screens.yaml` they always did. |
| Tool versions | The `$id` rewrite lands in the next minor version of `vectis` and `contract` tools. Older tool versions continue to publish their original `$id`s; the `specify tool schema` verb resolves against whichever version is bound. |

`migrate-to-2.0.sh` does not change; RFC-31 ships as a 2.x point release inside the 2.0 contract.

## Non-goals

- **No auto-refactor of baseline `composition.yaml`.** Refactor proposals are advisory in v1. A future RFC may wire an opt-in `specify component apply <slug>` that rewrites baseline screens, but it is out of scope here.
- **No catalog sharing across projects.** Each project's `.specify/design-system/components.yaml` is local. A future "design-system source adapter" could bind external catalogs, but RFC-31 keeps the file project-scoped.
- **No per-claim authority overrides at the catalog level.** The catalog records *what* the structure looks like, not *who* asserted it. Authority continues to apply at the Evidence / synthesis layer.
- **No new slice lifecycle.** Refactor proposals do not park the slice or introduce a new state. The slice still progresses `refining → refined → built → merged` regardless of whether the operator acts on the proposal.
- **No new plan-time gate.** The component-detection feedback all happens at slice time. The operator-stamped `reviewed` gate remains the only plan gate.
- **No replacement of `tokens.yaml` or `assets.yaml`.** Both remain operator-curated. The catalog joins them as a *third* design-system file; it does not subsume them.
- **No automatic component naming.** The screenshots adapter still derives slugs from visible content (stage 6 unchanged). The catalog stores whatever slug the adapter emitted; rename is operator work via `specify component drop <slug>` plus a hand-edit of Evidence.
- **No live IDE preview.** `specify source preview --render html` produces a static HTML file. Hot-reload integration with Cursor or any editor is out of scope.

## Forward compatibility

This RFC was reviewed for forward compatibility with three planned extensions that are not yet in scope:

| Future | How RFC-31 accommodates it |
| --- | --- |
| **Figma API source adapter** (`figma`) | `specify source preview` is adapter-generic; the inference report's discriminated `inputs` union accepts new types without a breaking schema change; the `$CATALOG_DIR` preopen is opt-in via `needs: [catalog]`; `docs/reference/structural-identity.md` defines the claim contract any visual adapter must satisfy for catalog participation. No RFC-31 surface is screenshots-specific in a way that would block a Figma adapter. |
| **Legacy-code visual inference source adapter** (`rendered-code`) | Same as Figma above. The adapter produces `region`/`container`/`leaf` claims (per the existing layout-inferer-contract), opts in to catalog reads, and benefits from the preview harness and goldens infrastructure with its own fixture shape under `source/`. |
| **React+TypeScript web shell** (Vectis `web` platform) | The build brief already carries `web` as a deferred platform token. The scaffold tool's template registry is data-driven by in-scope platforms, so adding a `web` template (`web/.../components/<slug>.tsx`) is an additive change. The catalog validation rule scopes "each in-scope shell tree" dynamically via platform detection. No RFC-31 decision hard-codes the iOS/Android pair. |

Design choices that explicitly support these futures:

- **Inference report `inputs` is a discriminated union** (D4) — new adapter input types extend `oneOf` without breaking existing consumers.
- **Component schema is CLI-owned** (D7) — its `$id` is decoupled from any single target tool version; non-Vectis targets can consume the catalog without a migration.
- **Structural-identity rules live in a standalone reference** (C.11) — any adapter can verify its claims will participate in reconciliation without reading screenshots-specific prose.
- **Instance uniqueness is by `(slice, screen)`** (D8) — multi-source projects that bind both `screenshots` and `figma` against the same screen set do not over-count component instances.
- **Scaffold template registry is data-driven** (D10) — new platform templates are a data addition, not orchestrator code changes.

## Open questions

1. ~~**Catalog skeleton serialisation.** The catalog needs a serialised skeleton that the CLI can structurally diff against new claims. Two options: (a) embed the claim subtree verbatim, (b) compute a content-addressable digest of the normalised skeleton.~~ Resolved: both. The embedded `skeleton:` subtree is **authoritative** and human-reviewable; a sibling `skeleton-digest:` field (sha256 of the skeleton normalised per `docs/reference/structural-identity.md`) is a CLI-maintained derived cache for O(1) reconciliation lookup. The CLI recomputes the digest on every catalog write. If an operator hand-edits the skeleton tree, `specify slice validate` catches digest drift and the CLI rewrites the digest on the next reconciliation pass. No additional normalisation spec is required — the structural-identity rules (C.11) already define the canonical form that feeds the hash.
2. ~~**Catalog scope vs Vectis-target specificity.** The catalog as drafted is Vectis-shaped (regions / containers / leaves). Should it generalise (catalog `<kind>` becomes adapter-extensible) or stay Vectis-only with a hard-coded schema?~~ Resolved: keep it Vectis-shaped (regions / containers / leaves) without generalising. The entire process targets Vectis for now, and the vocabulary — regions, containers, leaves — describes user-interface construction at a level that is naturally generic enough to support other UI target frameworks when they arrive. Adding adapter-extensible `<kind>` indirection now would be premature abstraction with no second consumer to validate the design. The structural-identity rules extracted into `docs/reference/structural-identity.md` (C.11) already give future visual adapters (Figma, legacy-code) a clear contract for the claim shapes catalog reconciliation accepts; generalisation can follow when a second target actually needs it.
3. ~~**Single-instance candidates.** Today the adapter emits `notes.candidate_component: <slug>` on single-instance skeletons. Should the catalog record those as `candidate` entries proactively (so a later slice's second instance auto-promotes), or wait until ≥2 instances exist within one run?~~ Resolved: record proactively. The noise concern does not hold — `candidate` entries are inert metadata with no downstream effects (no `component:` rewrites, no shared-component generation, no refactor proposals). Only `confirmed` entries trigger downstream work. The cost of a false positive (`specify component reject`) is far lower than the cost of a false negative (operator manually discovering cross-slice duplication or never discovering it). Without proactive recording, N sequential single-screen slices that each carry one instance of a shared structure never bootstrap the catalog at all — the exact scenario the catalog was designed to solve.
4. **Preview verb scope beyond screenshots.** `specify source preview` is useful for any source adapter, not just `screenshots`. Should `documentation` and `code-typescript` ship `briefs/preview.md` too in this RFC? Current preference: defer — the verb is generic, but only `screenshots` needs the visual render path in v1.
5. **`specify tool schema` discovery.** Should the verb list available schemas when called as `specify tool schema vectis` (no `<name>`)? Current preference: yes, print the kebab-case schema names and their canonical URLs.
6. **`brief-schema-link-resolves` predicate cost.** Live HTTP resolution makes `make check` network-dependent. Alternative: ship a snapshot of every published `$id` body in `scripts/check.fixtures/` and diff against it. Current preference: snapshot — keeps `make check` hermetic.
7. **Catalog file under workspace mode.** Where does the catalog live in workspace mode? Per-project (`.specify/workspace/<project>/.specify/design-system/components.yaml`) or shared at the workspace root? Current preference: per-project; the catalog is design-system state and design systems usually align with the project, not the workspace.
8. ~~**Component schema lives under the Vectis tool, but the catalog file lives in the operator's project.**~~ Resolved: the schema is CLI-owned from day one (see D7). The `$id` cites the CLI version, not the Vectis tool version; no migration is needed when a non-Vectis target consumes the catalog.

## Acceptance proof

RFC-31 is complete when:

1. The plugin-repo `adapters/targets/vectis/schemas/` directory contains only the README.
2. `make check` fails when a brief cites a non-canonical schema URL.
3. `specify tool schema vectis tokens` round-trips byte-identical against the CLI-embedded copy.
4. `specify source preview screenshots --source <fixture>` produces an inference report that matches `expected/report.json` for the in-tree golden.
5. `specify source preview screenshots --check <fixture>` exits 0 on the unchanged golden and non-zero after a brief edit.
6. The `task-list-two-screen` fixture grows an `expected/` directory and runs in CI.
7. A two-slice fixture where slice 1 produces three matching footers and slice 2 produces one matching footer ends with `.specify/design-system/components.yaml` showing `tab-bar: status: confirmed` and slice 2's Evidence carrying `component: tab-bar`.
8. A three-slice fixture where each slice produces exactly one screen with a structurally identical footer ends with `tab-bar: status: confirmed` after slice 2's extract (slice 1 seeds `candidate`, slice 2 crosses the ≥2 threshold) and slice 3's Evidence carrying `component: tab-bar`.
9. The same fixture as (7), with `specify component reject tab-bar` interleaved, ends with slice 2's `slice validate` reporting `slice-catalog-drift` on the stale `component:` directive.
10. The Vectis build emits a `tab-bar` shared component file per in-scope shell tree exactly once across both slices.
11. `specify slice validate` catches every cross-reference between Evidence `component:` directives and the catalog.

## References

- [RFC-25: Workflow](../done/rfc-25-workflow.md) — source/target split; screenshots adapter; `screenshots` and `vectis` first-party plugins; spatial Evidence kinds.
- [RFC-16 (archived): WASI Vectis tool](../done/rfc-16-wasi-vectis.md) — `vectis` WASI tool; embedded schema discipline.
- [RFC-11 (archived): UI Specification Workflow](../done/rfc-11-ui-spec.md) — §A unwired-subset; §G component-directive emission and structural-identity; §H validate modes.
- [RFC-29: Fan-In/Fan-Out](rfc-29-fan-in-fan-out.md) — executable `specify source enumerate` / `specify source extract`; required for the `specify source preview` plumbing.
- [RFC-30: Init bootstrap](rfc-30-init.md) — `specify tool` family discipline; cross-repo schema migration precedent.
- [Layout inferer contract](../../adapters/targets/vectis/references/layout-inferer-contract.md) — pre-2.0 producer contract; component directive emission policy reused here.
- [Screenshots `extract/pipeline.md`](../../adapters/sources/screenshots/briefs/extract/pipeline.md) — stage-6 component detection; reused by catalog reconciliation.
- [Vectis schemas README](../../adapters/targets/vectis/schemas/README.md) — directory targeted for retirement in wave A.
- [`wasi-tools/vectis/src/validate/engine/shared.rs`](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/src/validate/engine/shared.rs) — "byte-identity discipline" comment; deleted in wave A.
