# RFC-11: UI Specification Workflow

> Status: Draft · Depends: [RFC-7](archive/rfc-7-ui.md)

## Abstract

Define the **UI specification workflow** that produces every input the vectis shell writers need to render a Crux application: the layout intent (`layout.yaml`), the design tokens (`tokens.yaml`), the asset inventory (`assets.yaml` + image files), and the per-screen requirements that flow through `/spec:define` into `/spec:build`. RFC-7 introduced `composition.yaml` as a multi-source artifact and described its skeleton/wired duality; this RFC makes that boundary explicit by using `layout.yaml` for pre-define input and reserving `composition.yaml` for the wired Specify artifact. This first pass scopes:

- the **overarching UI specification workflow** and the shared contract future layout producers must target;
- the **initial `image-layout-inferer` skill**, which fronts the workflow by converting screenshots/images into reviewable `layout.yaml`;
- the **intended future roles** for Figma and source-code layout inferers, captured as direction-setting examples rather than accepted implementation detail;
- the **`tokens.yaml` and `assets.yaml`** artifacts that travel alongside the layout, with the operator as the source of truth and any future helper inferers/importers subject to separate review;
- the **Vectis schema artifact contract** to add to `schemas/vectis/schema.yaml`: it names `layout.yaml`, `tokens.yaml`, `assets.yaml`, and asset files as UI input artifacts, names `composition.yaml` / `design.md` / specs / tasks as define outputs, and tells build + merge which artifacts they consume or carry forward;
- the **`/spec:define`** contract that turns layout intent + requirements into a wired `composition.yaml` plus the existing vectis define briefs, with `design.md` made composition-aware rather than directly layout-driven;
- the **`/spec:build`** consumption surface where the shell writers see the wired composition, tokens, assets, and image files as a single coherent input set;
- the **dissolution of `design-system` as a peer "platform"** in proposals and the build phase. The "design system" name is reserved for the *input* artifacts the operator maintains (`layout.yaml`, `tokens.yaml`, `assets.yaml`, and any future component vocabulary). The lower-level reusable components — today the `VectisDesign` Swift Package and `vectis-design` Compose library emitted by `vectis:design-system-writer` — fold into each shell writer. iOS and Android stay the only runtime platforms; nothing parallel to them is generated.

## Motivation

### What `assets/ui-spec.png` captures

![Proposed UI specification workflow](assets/ui-spec.png)

The diagram embedded above from [`rfcs/assets/ui-spec.png`](assets/ui-spec.png) describes the target pipeline at a glance. In this RFC, the pipeline is established and exercised through the screenshot/image path first; the Figma and source-code paths are shown as intended future producers that should target the same workflow once their own details are reviewed.

1. **Several sources** can eventually drive the layout — a Figma file, a set of screenshots/images, or an existing codebase. The first implemented producer is the screenshot/image *layout-inferer* skill (green), which produces `layout.yaml`; the other producers remain future RFC work.
2. **The operator** is a peer source — they can hand-author `layout.yaml` directly, and they always own the `requirements`, `tokens.yaml`, and the raw image files. `assets.yaml` is derived from those image files, with the operator confirming names and per-platform choices.
3. **`/spec:define`** consumes `layout.yaml` plus requirements and emits the wired `composition.yaml` plus the rest of the vectis define briefs. `design.md` reads the wired composition for screen, ViewModel, binding, token, and asset implications; it does not read unwired layout as a separate source once the composition brief has run.
4. **`/spec:build`** consumes the wired composition along with the asset inventory, the design tokens, and the raw image files. `/spec:merge` then carries any change-local composition, token, asset-manifest, or asset-file deltas into the baseline input set.

Everything to the left of `/spec:define` is *UI input material*. Everything from `/spec:define` onward is the existing Specify lifecycle. This RFC scopes the left half and the contract at the seam.

### What ships today

- **Composition.** [RFC-7](archive/rfc-7-ui.md) defined the artifact and the skeleton/wired modes. This RFC renames the pre-define skeleton surface to `layout.yaml`, while keeping the post-define/baseline artifact as `composition.yaml`. Two source paths are operational today: agent-inferred-from-specs (low fidelity) and hand-authored. The Figma adapter is described but not implemented; the legacy-app and screenshot paths are open.
- **Tokens.** [`vectis:design-system-writer`](../plugins/vectis/skills/design-system-writer/SKILL.md) regenerates iOS Swift Package + Android Compose library from a hand-authored `design-system/tokens.yaml`. There is no JSON Schema for `tokens.yaml`; value shapes (colour / font / scalar) are inferred from the first entry per category.
- **Assets.** No artifact and no skill. Image files end up in shell-specific asset catalogues (`Assets.xcassets`, `res/drawable*/`) by hand or by ad-hoc shell-writer copy steps. The layout/composition vocabulary references icons and images by bare name with no central index.
- **Requirements.** Travel today as the user prompt + the existing vectis specs/proposal briefs. The diagram treats `requirements` as a first-class input to `/spec:define`, peer to `layout.yaml` — making it explicit that layout is *presentation intent* and requirements are *behavioural intent* and they meet at define time.
- **Define / build.** The vectis define pipeline already includes a [composition brief](../schemas/vectis/briefs/composition.md) that reads existing layout input when present and otherwise infers one. The build brief hands `composition.yaml`, `app.rs`, `tokens.yaml`, and the spec shell sections to each shell writer. There is no asset hand-off yet.

### What is missing

- **No source-of-truth path for layout intent beyond Figma-on-paper and hand-authoring.** Real teams arrive with screenshots, a deployed app, or both, and currently have to hand-translate them into layout vocabulary.
- **No shared producer contract.** Each prospective source (image first, then Figma and code later) faces the same problems — schema grounding, ambiguity reporting, idempotent re-runs, multi-source merging — with no shared scaffolding.
- **No assets pipeline.** Image files have no inventory artifact, no assets manifest schema, no per-platform mapping (`@2x`/`@3x` vs density buckets), no token-style naming, and no resolution check at validate time.
- **No published tokens schema.** Adding a category (motion, elevation, iconography) requires coordinated edits across writer + per-platform templates with no validator.
- **No formal define contract.** What `/spec:define` requires from `layout.yaml` vs. produces in wired `composition.yaml` is RFC-7 prose; it has never been pinned as an interface that the image inferer can target now and future inferers can target later.
- **No build hand-off for assets / tokens beyond ad-hoc.** The shell writers need to know which images and tokens this build expects, but they receive that knowledge by inference rather than as a manifest.
- **`design-system` is wired as a peer "platform" of `ios` and `android`.** The proposal brief lists it alongside the runtime platforms ([`schemas/vectis/briefs/proposal.md`](../schemas/vectis/briefs/proposal.md)), the build brief runs it as the first phase ([`schemas/vectis/briefs/build.md`](../schemas/vectis/briefs/build.md)), the plan-time discovery and propose briefs treat it as a tier ([`schemas/vectis/briefs/plan/discovery.md`](../schemas/vectis/briefs/plan/discovery.md), [`propose.md`](../schemas/vectis/briefs/plan/propose.md)), and `vectis:design-system-writer` ships a Swift Package and a Compose library into `design-system/ios/` and `design-system/android/` that the iOS and Android shells consume as external dependencies. This is an anomaly: nothing is *deployed* to "design-system" — the artifact is a build-time prerequisite for the real runtime targets. Conflating the input surface (the operator-maintained `layout.yaml`, `tokens.yaml`, `assets.yaml`) with the per-platform emit (Swift / Kotlin token files) creates redundant lifecycle scaffolding (its own platforms entry, its own build phase, its own writer skill, its own task ordering) and an unnatural shared-library boundary that each shell would otherwise own internally.

### Non-goals

- **Not a redesign of the composition schema.** RFC-7 already settled the artifact shape; this RFC separates its pre-define input filename (`layout.yaml`) from its wired lifecycle filename (`composition.yaml`) and defines the producers and consumers around that boundary.
- **Not a redesign of the Specify lifecycle.** `/spec:define` and `/spec:build` are existing skills; this RFC scopes their *contract* with the new producers, not their internal flow.
- **Not a design tool.** Specify does not replace Figma or any image editor. The inferers ingest existing artefacts; visual editing stays upstream.
- **Not a runtime theming engine.** Output is generated source the shells compile against, same as today; no dynamic theme-swap protocol.
- **Not a hosted service.** All inputs and outputs stay local and reviewable, per the [roadmap directional principles](roadmap.md#directional-principles).
- **No spec generation from screenshots, Figma, or code by these skills.** The inferers produce `layout.yaml` only. Behavioural specs continue to come from `/spec:define`, `/spec:extract`, or hand-authoring.

## Detailed Design

This section records the first-pass decisions for the UI input workflow. The normative implementation surface is the workflow contract plus `image-layout-inferer`; Figma, code, token, and asset helper inferers are described only as future direction unless a later RFC accepts their detailed behavior.

### A. Layout inferers — shared contract

The first implementation establishes a shared contract documented in `plugins/vectis/references/layout-inferer-contract.md`, then proves it with `vectis-image-layout-inferer`. Future Figma and code inferers should reuse the same contract unless testing shows the contract needs revision. The common job is to produce or refine `layout.yaml`: a schema-valid, unwired layout input that `/spec:define` can later wire to specs and Crux types as `composition.yaml`.

Common arguments for the image-fronted first pass are intentionally minimal, and are expected to become the starting point for future inferers rather than a final commitment for them:

| Argument | How it is used | Meaningful default | Precedence / override | Why it is shared |
| --- | --- | --- | --- | --- |
| `--output <path>` | Names the exact file the inferer should write. | Active change directory's `layout.yaml`, then `design-system/layout.yaml` for pre-define authoring outside a change. | Explicit `--output` wins over all project defaults. | Supports reviewable local authoring outside the normal lifecycle and lets tests / fixtures write to temporary paths instead of a project tree. |
| `--baseline <path>` | Provides an existing `layout.yaml` or wired `composition.yaml` that the inferer should refine. | Existing output-path content, then `design-system/layout.yaml`, then `.specify/specs/composition.yaml`. | Explicit `--baseline` wins over discovered local or baseline files. | Gives the image inferer, and later sibling inferers, the same idempotence hook: preserve operator edits, append new evidence, and refine existing layout instead of regenerating from scratch. |
| `--screen <slug>=<hint>` | Supplies repeatable screen-boundary hints. Hints can name source frame IDs, screenshot groups, or source-code view entrypoints. | No explicit hints; inferers derive screen candidates from their source material. | Supplied hints constrain or name inferred candidates, but do not force invalid schema output. | Screen identity is the first ambiguity every source type hits. Shared hints stabilize screen names and boundaries before `/spec:define` wires them to specs and routes. |

Arguments deliberately left out of the first-pass common surface:

- `--change-dir <path>` is redundant with default active-change discovery plus `--output` for explicit routing. If active-change detection is ambiguous, the operator can pass `--output .specify/changes/<name>/layout.yaml`.
- `--tokens <path>` and `--assets <path>` are not common first-pass arguments. The image inferer should auto-discover `design-system/tokens.yaml` and `design-system/assets.yaml` when those files exist, then use them for reference checks. Non-standard token or asset locations can wait until there is demonstrated demand, or live in a future source-specific skill if one import path truly needs it.

Operator ergonomics and scoping:

- The first pass optimizes for reviewable, bounded inference runs. Operators SHOULD run the image inferer for one screen or one small coherent flow at a time, especially when refining an existing `layout.yaml`.
- The image inferer MAY accept multiple image inputs in one run when those inputs clearly describe the same screen set, such as several screenshot states.
- To accumulate layout information in a single change, run the image inferer against the same `layout.yaml`; future inferers should follow the same grow-or-refine rule if accepted.
- Mixed-source reconciliation is not a first-pass mode. Future Figma/code runs should be reviewed one at a time against the same `layout.yaml`, but the exact reconciliation details remain subject to those future RFCs.

Output rules:

- Layout inferers MUST emit `layout.yaml` documents using the composition schema's unwired subset. Allowed structure is a full `screens` document with screen names, regions, groups, item vocabulary, token references, asset references, states, overlays without triggers, and platform overrides. A layout document MUST NOT use the change-local `delta` shape.
- The unwired subset forbids define-owned wiring: `maps_to`, `bind`, `event`, `error`, overlay `trigger`, navigation targets encoded in events, and conditional visual keys such as `strikethrough-when`. These keys are reserved for the wired `composition.yaml` emitted by `/spec:define`.
- Layout inferers MAY use token references when the source supplies a named token, variable, or style that can be confidently mapped to `tokens.yaml`. Otherwise they should prefer raw layout values only when the composition schema permits them, and add `# TODO` comments where tokenisation is expected later.
- Layout inferers MAY reference asset IDs only when they resolve through `assets.yaml` or are emitted with a matching `# TODO` gap asking the operator to add the asset inventory entry.
- Layout inferers MUST append to `provenance.sources[]` rather than replacing it. The composition schema should add provenance kinds `screenshots` and `code` alongside existing `figma`, `legacy`, and `manual`; `legacy` remains valid for broad source-code migration runs.
- Multi-source output is a single `layout.yaml`. Per-screen provenance is represented through comments adjacent to screen entries in v1, not a schema change. A future schema can promote that into structured per-screen metadata if needed.

Idempotence rules:

- Re-runs are additive and conservative. The image inferer may add new screens, add missing regions, fill empty hints, or refine content it previously emitted when the same source still supports the refinement; future inferers should preserve the same rule unless their RFC changes it.
- Layout inferers MUST NOT silently delete screens, groups, layout properties, token references, or comments that may have been operator-edited. When source material no longer contains a previously inferred element, the inferer reports a stale-source warning instead of removing the YAML.
- The first pass does not use "owned by inferer" markers. The merge rule is easier to review: preserve existing structure, append new evidence, and surface conflicts as comments / terminal warnings.

Verification:

- The image inferer invokes the CLI's deterministic layout validator before writing or reporting success. The validator checks YAML syntax, `schemas/vectis/composition.schema.json`, and the additional unwired-subset rules above. Future inferers should do the same unless their RFC changes the contract.
- The image inferer invokes the CLI's cross-artifact reference checks from §E and §F when `assets.yaml` or `tokens.yaml` exists.
- The terminal summary includes: screens added, screens refined, warnings, unresolved gaps, source provenance appended, and the exact output path.

Skill shape is decided in §J: `image-layout-inferer` is implemented first, with future sibling inferers preferred over one flag-dispatched skill if later RFCs accept the Figma and source-code paths.

### B. Skill 1 — `figma-layout-inferer`

`vectis-figma-layout-inferer` is future RFC work. The goal is reasonable: let a Figma file or export produce the same `layout.yaml` contract that the image inferer establishes here. The implementation details below are illustrative only, included to preserve intent and vocabulary for later review.

Input modes:

- `--figma-json <path>`: read an already-exported Figma file / node payload. This is the default recommended path.
- `--figma-url <url>`: fetch from the Figma REST API using `FIGMA_TOKEN` or an explicitly configured local credential. The skill does not require a hosted service or a Figma MCP server.
- `--node <id>`: optional repeatable node selection for narrowing the import to specific frames.

Mapping:

- Frames and sections become candidate screens when named like app screens or selected through `--screen` / `--node`.
- Auto Layout maps directly to layout groups: `layoutMode` -> `direction`, `itemSpacing` -> `gap`, padding fields -> `padding`, alignment -> `align` / `justify`, and resizing constraints -> `size`.
- Text, vector, instance, image fill, and interactive component-like nodes map to the closest layout item vocabulary. Unknown node kinds become comments rather than custom schema extensions.
- For instance, Figma component instances could be flattened into normal layout groups/items initially. The skill could record candidate component names as comments for the future component-primitives RFC (§G), but it should not emit `components.yaml` unless that future artifact exists.

Variables and styles:

- Figma Variables and styles are not written directly by the layout inferer. When variable/style metadata is present, the inferer reuses token names that already exist in `tokens.yaml` and reports unmapped variables as a gap.
- Token import should remain outside the layout inferer. A later token-import RFC may define a helper flow, but the artifacts should remain separate so layout inference cannot accidentally rewrite the token source of truth.

### C. Skill 2 — `image-layout-inferer`

`vectis-image-layout-inferer` converts screenshots or other UI images into `layout.yaml`. It is explicitly a layout recovery tool, not a visual design extraction system.

Inputs:

- One or more PNG, JPEG, or HEIC files.
- Optional `--platform ios|android|web` so the skill can ignore system chrome and recognise platform conventions.
- Optional `--group <screen-slug>:<path>,<path>` to identify screenshots that represent states of the same screen.
- Optional `--state <screen-slug>:<state-name>=<path>` for explicit loading / empty / populated / error state mapping.

Pipeline:

1. **Triage.** Group images into screens and states, using explicit hints first and visual similarity second.
2. **Crop platform chrome.** Remove status bars, navigation bars, browser chrome, and emulator frames when a platform hint is present.
3. **Infer regions.** Identify header, body, footer, fab, overlays, and repeated content zones.
4. **Infer containers.** Recover rows, columns, cards, lists, grids, padding, gap, alignment, fill/hug sizing, and surface decoration.
5. **Infer leaves.** Map visible text, controls, images, icons, progress indicators, fields, and segmented controls to layout items.
6. **Emit gaps.** Add comments for ambiguous grouping, unreadable text, uncertain icon identity, or layout choices requiring operator confirmation.

Vision assumptions:

- The skill assumes the agent runtime can inspect attached images. If no vision-capable runtime is available, it stops with a clear prerequisite message instead of pretending to infer layout from filenames.
- The skill ships regression fixtures in its own skill directory: screenshot inputs paired with expected `layout.yaml` fragments. These fixtures are not exhaustive visual tests; they guard the pipeline's contract and common layout patterns.

Token and asset extraction:

- The image inferer does not reverse-engineer `tokens.yaml` from pixels. It may emit token-like TODO comments such as `# TODO: replace measured gap 16 with spacing token` but does not invent token names from colours or font sizes.
- The image inferer may reference asset placeholders when screenshots clearly contain illustrations or icons, but it does not crop production assets out of screenshots. Asset inventory remains operator-supplied unless a future asset helper is accepted.

### D. Skill 3 — `code-layout-inferer`

`vectis-code-layout-inferer` is future RFC work. The goal is reasonable: recover layout structure from existing UI source code into the same `layout.yaml` contract. The implementation details below are illustrative only and should be re-evaluated after the image inferer has exercised the workflow in tests and real changes.

For instance, a future implementation might focus on declarative UI frameworks where hierarchy is explicit in source: SwiftUI, Jetpack Compose, and React/JSX. Vue, Flutter, HTML/CSS, UIKit, AppKit, Android Views/XML, and other imperative or split-template frameworks could remain deferred until the first declarative paths are reliable.

Inputs:

- `--source <path>`: existing application source tree.
- `--include <glob>` / `--exclude <glob>`: optional scope filters mirroring `/spec:extract`.
- `--entry <symbol-or-path>`: optional repeatable view entrypoint hint.
- The common inferer arguments from §A.

Strategy:

- Use a hybrid approach. Prefer syntax-aware parsing for obvious hierarchy (`VStack`, `HStack`, `LazyColumn`, `Row`, `Column`, JSX elements), then use agent reading to resolve local helper views, modifiers, style constants, and conditional branches.
- Recover container hierarchy and layout intent, not business behavior. Navigation calls, event handlers, and state bindings are useful hints for item names, but wired `event` / `bind` keys remain `/spec:define` responsibility.
- Treating reusable source-code components as inline structure would be a plausible starting point. Candidate components could be emitted as comments that may later inform `components.yaml`.

Relationship to `/spec:extract`:

- The skill should be an independent sibling, not a hidden phase inside `/spec:extract`. `/spec:extract` continues to reconstruct behavioral specs and design from source. Operators who want both behavior and UI layout would run both skills, usually as part of a plan-time migration flow.
- A future `/spec:plan` flow may invoke the code layout inferer during discovery when the initiative explicitly asks for UI reconstruction, but the default source-code extraction path should remain behavior-first.

Asset capture:

- A future code inferer may discover asset references in source (`Image("hero")`, `painterResource`, `src="/logo.svg"`). It should report these references, but it should not copy files or author `assets.yaml` itself unless a future asset workflow says so.

### E. Assets pipeline — `assets.yaml` + image files

`assets.yaml` is the inventory for image and icon files used by `layout.yaml` before define and by wired `composition.yaml` after define. It lives at `design-system/assets.yaml` by default and points at files under `design-system/assets/`. The operator owns the final naming and role metadata; tools may draft or update the file, but the manifest is intended to be reviewed like any other source artifact.

V1 schema sketch:

```yaml
version: 1

provenance:
  sources:
    - kind: manual

assets:
  onboarding-hero:
    kind: raster
    role: illustration
    alt: "People organizing tasks"
    sources:
      ios:
        1x: assets/onboarding-hero.png
        2x: assets/onboarding-hero@2x.png
        3x: assets/onboarding-hero@3x.png
      android:
        mdpi: assets/android/onboarding-hero-mdpi.png
        xhdpi: assets/android/onboarding-hero-xhdpi.png
        xxhdpi: assets/android/onboarding-hero-xxhdpi.png

  close:
    kind: symbol
    role: icon
    symbols:
      ios: xmark
      android: close
    tint: onSurface
```

Rules:

- **Asset IDs** are kebab-case and are the only names `layout.yaml` / `composition.yaml` may reference. Shell-specific filenames remain implementation detail.
- **Kinds** are `raster`, `vector`, and `symbol`. `raster` points at density-specific image files. `vector` points at source vector files and optional platform exports. `symbol` maps a semantic icon ID to curated platform symbol sets: SF Symbols for iOS and Material Symbols / Icons for Android.
- **Roles** are `decorative`, `icon`, `illustration`, and `photo`. `decorative` assets do not require `alt`; all other image-like assets SHOULD carry `alt` text for shell accessibility labels. Pure symbols used inside labelled controls may omit `alt` when the surrounding control supplies the accessible label.
- **Vector support is in v1.** SVG/PDF/vector-drawable conversion is not a hosted service; the manifest records either already-exported platform files or a source vector plus `# TODO` comments naming missing exports. Shell writers copy only files that exist.
- **Tint** is an optional token reference. It is valid for `symbol` and single-colour vector assets. Raster tinting is opt-in and shell-specific; writers may ignore it unless the platform supports safe template rendering.
- **Resolution checks live in the input validation gate.** During define, validate `layout.yaml` references when `assets.yaml` exists. Before shell generation, validate that every `image`, `icon`, and asset-like background reference in `composition.yaml` resolves to an `assets.yaml` entry or to an allowed platform symbol mapping in that entry. Missing files are errors; missing optional densities are warnings unless the target platform has no usable source.
- **Build hand-off is copy-on-generate.** iOS and Android writers copy referenced assets into their own asset catalogs (`Assets.xcassets`, `res/drawable*`, or equivalent) during generation. They do not symlink or reference `design-system/assets/` in place, because generated shell projects should remain buildable from their own platform directory after generation.

An `assets-inferer` helper is not in the first implementation pass. A future RFC may accept a helper that walks `design-system/assets/` or an imported legacy asset directory, groups density variants by filename convention, drafts `assets.yaml`, and reports ambiguous names for operator review. Any such helper should avoid visual-semantic decisions beyond conservative defaults (`role: illustration` for large rasters, `role: icon` for small square glyphs, `kind: symbol` only when a mapping is explicitly provided).

### F. Tokens artifact — input only

`tokens.yaml` stays an input artifact the operator maintains alongside `layout.yaml` and `assets.yaml`. It lives at `design-system/tokens.yaml` by default. The emit half folds into the shell writers per §L; there is no standalone generated design-system package.

V1 publishes a JSON Schema for one file, not a split directory. One file keeps the shell-writer handoff simple and preserves the existing `design-system/tokens.yaml` convention. Splitting into `tokens/colors.yaml`, `tokens/typography.yaml`, and similar can be introduced later as an import/export convenience without changing the canonical contract.

V1 schema sketch:

```yaml
version: 1

provenance:
  sources:
    - kind: manual

colors:
  primary:
    light: "#007AFF"
    dark: "#0A84FF"

typography:
  title:
    size: 28
    weight: bold
    lineHeight: 34

spacing:
  md: 16

cornerRadius:
  md: 8

elevation:
  card: 2

border:
  subtle:
    width: 1
    color: outline

opacity:
  disabled: 0.38
```

Rules:

- **Declared categories replace value-shape inference.** The schema defines value shapes per category. `colors` use light/dark `#RRGGBB`; `typography` uses numeric `size`, optional `lineHeight`, optional `letterSpacing`, and known weights; `spacing`, `cornerRadius`, `elevation`, and `opacity` are scalar categories with explicit units documented by the schema; `border` is a composite category with `width`, `color`, and optional `radius`.
- **Initial vocabulary is intentionally bounded.** V1 includes `colors`, `typography`, `spacing`, `cornerRadius`, `elevation`, `border`, and `opacity`. `motion`, `gradient`, icon families, and full component primitives are deferred. Composition already references elevation and border-like concepts, so those categories close the immediate schema gap.
- **Provenance mirrors composition.** `provenance.sources[]` supports `manual`, `figma-variables`, `style-dictionary`, `tokens-studio`, `dtcg`, and `legacy`. Importers append sources rather than replacing existing provenance.
- **Import is future helper work.** A later `vectis-tokens-inferer` could import from Figma Variables JSON, Style Dictionary, Tokens Studio JSON, or W3C DTCG into the canonical YAML. The canonical artifact remains `tokens.yaml`; W3C DTCG is an import/export format, not the internal intermediate each shell writer consumes.
- **No multi-brand in v1.** The only built-in theme axis is light/dark. Multi-brand support is deferred until there is a concrete downstream need. When it lands, it should add an explicit `themes:` map rather than relying on multiple token files with implicit naming conventions.
- **Verification is cross-artifact.** Before shell generation, validate that every token reference in `composition.yaml` and `assets.yaml` resolves to `tokens.yaml`. Undefined references are errors. Defined-but-unused tokens are warnings unless marked with `unused: allowed` or a similar schema-approved marker.
- **Fallback policy belongs to shell writers.** When `tokens.yaml` is absent, ios-writer uses platform-native HIG defaults and android-writer uses Material 3 defaults. When `tokens.yaml` is present but incomplete, shell writers may use platform defaults for categories that are absent, but MUST NOT silently substitute defaults for a token name that is referenced and missing.

### G. Component primitives (deferred decision)

Reusable component primitives are deferred to a later RFC. RFC-11 reserves the input slot but does not define `components.yaml`.

The first pass treats repeated screenshot structures as provenance signals only. The image inferer may report "candidate component" gaps, but it still flattens the structure into `layout.yaml` groups and items. Future Figma/code inferers should start from the same approach unless the component-primitives RFC lands first. Shell writers may create platform-local helper views/composables as an implementation detail, but those helpers are not a cross-platform artifact and are not referenced from the composition schema.

When a future component artifact lands, it should be an input sibling of `layout.yaml`, `tokens.yaml`, and `assets.yaml`, not a generated shared library. Each shell writer would read it directly and bake the platform implementation into its own tree, following the same rule as §L.

### H. `/spec:define` contract

`/spec:define` is the handoff from UI input material to the normal Specify lifecycle. It consumes layout intent from `layout.yaml` and behavioral intent from the operator's request / define briefs, then emits the wired artifacts that `/spec:build` can implement.

Planned schema-level artifact contract:

- `schemas/vectis/schema.yaml` MUST grow a first-class `artifacts` contract in addition to the ordered brief pipeline. The contract lists the UI input set (`layout.yaml`, `tokens.yaml`, `assets.yaml`, and `design-system/assets/**`), the define-phase outputs (`composition.yaml`, `design.md`, specs, and `tasks.md`), the build-phase consumption set, and the merge-managed UI input deltas.
- `layout.yaml` is an input-only artifact. It may appear change-local as `.specify/changes/<name>/layout.yaml` or as the project input `design-system/layout.yaml`. The composition brief consumes it and writes change-local `composition.yaml`; build and merge do not consume `layout.yaml` directly.
- `tokens.yaml` and `assets.yaml` are durable input artifacts. They may appear change-local during a Specify change, but their baseline home remains `design-system/tokens.yaml` and `design-system/assets.yaml`. Define validates references and may carry change-local updates forward; build reads the resolved artifacts directly; merge moves accepted deltas into the baseline input directory.
- Asset files are part of the artifact contract, not opaque side effects. `design-system/assets/**` is validated for referenced-file existence before shell generation and is merged with the same review surface as `assets.yaml`.
- The schema contract is descriptive metadata that briefs and skills will use for orchestration. Deterministic correctness still lives in the CLI validation modes, so agents do not infer artifact rules solely from prose.

Inputs:

- **Requirements.** No new `requirements.md` artifact in v1. The diagram's `requirements` input means the operator prompt plus any plan entry context and existing Specify define briefs. If a team wants a durable pre-define requirements file, they pass it through the existing prompt / source context mechanism rather than adding another canonical artifact.
- **Layout.** Optional `layout.yaml` in the active change directory, `design-system/layout.yaml`, or baseline `.specify/specs/composition.yaml` used as a starting point for iterative changes, resolved in that order. `layout.yaml` is validated in CLI `layout` mode before the composition brief uses it; baseline `composition.yaml` is validated in CLI `composition` mode because it may already contain wiring.
- **Tokens.** Optional `design-system/tokens.yaml` or change-local `tokens.yaml` when supplied. Define reads token names for reference consistency; it does not emit platform code.
- **Assets.** Optional `design-system/assets.yaml` plus files under `design-system/assets/`. Define validates references and may add TODO comments for missing asset IDs, but it does not copy files into shells.

Outputs:

- The existing vectis define artifacts: `proposal.md`, `specs/**/*.md`, `design.md`, `tasks.md`, `contracts.md` when the schema pipeline includes contracts, and a wired `composition.yaml`.
- `composition.yaml` is the concrete output of consuming `layout.yaml`; there is no generated `composition.md` artifact. The `composition.md` file in the repository is the define brief that performs this transformation.
- `design.md` is influenced by layout through `composition.yaml`: screen names, ViewModel variants, per-page view structs, Route needs, `bind` field completeness, token usage, asset usage, and platform-specific shell notes. `design.md` should not duplicate the raw layout tree or token/asset manifests.
- Change-local `tokens.yaml`, `assets.yaml`, and asset files remain inputs rather than generated code, but they are still part of the define output set for lifecycle purposes when a change updates them: tasks and build must see them, and merge must carry accepted deltas into `design-system/`.
- No `theme.md` or token summary artifact in v1. Token and asset usage is visible through `composition.yaml`, `tokens.yaml`, and `assets.yaml`; adding a generated summary would create another source of drift.

Wiring responsibilities:

- Preserve layout-owned structure: regions, group hierarchy, direction, gap, padding, align, justify, size, background, corner radius, elevation, token references, asset references, comments, and platform overrides.
- Add define-owned wiring: `maps_to`, `bind`, `event`, `error`, overlay `trigger`, navigation targets, and conditional visual keys such as `strikethrough-when`.
- Add missing screens only when specs describe a screen that has no layout entry. These additions are marked with provenance / comments so the operator can distinguish inferred-from-requirements layout from externally supplied layout.
- Do not rewrite token names or asset IDs unless they are invalid and the replacement is confirmed by an existing `tokens.yaml` / `assets.yaml` entry. Otherwise emit a gap comment and validation warning.

Multi-source handling:

- The image inferer writes into one `layout.yaml` before define runs. `/spec:define` does not perform a separate pre-define merge ceremony; future inferers should follow the same single-artifact handoff unless later RFCs add a richer merge workflow.
- `provenance.sources[]` preserves the source list. Per-screen source hints may remain comments in v1.
- Conflicts are resolved by preservation: existing layout structure wins; define adds wiring around it and reports conflicts rather than choosing between competing layouts.

Idempotence:

- Re-running define may update the wiring keys it owns when specs or `design.md` change.
- Re-running define MUST NOT rearrange groups, remove operator comments, rename layout-only items, or delete screens solely because the current requirements are silent.
- If a previously wired `bind` / `event` no longer resolves after a spec or design change, define reports the stale wiring and either updates it when there is a single obvious replacement or leaves a `# GAP` comment for operator review.

CLI validation modes:

- Deterministic validation belongs in the `specify` CLI, not in prompt prose. Skills call the CLI and then use the report to repair artifacts or explain blockers.
- `layout` mode validates `layout.yaml` as the unwired subset of `schemas/vectis/composition.schema.json`: YAML syntax, schema shape, `screens` only, no `delta`, and no define-owned wiring keys.
- `composition` mode validates `composition.yaml` as the lifecycle artifact: YAML syntax, schema shape, `screens` or `delta` as appropriate for baseline vs. change-local use, plus cross-artifact checks for `maps_to`, `bind`, `event`, overlay triggers, navigation targets, tokens, and assets.
- `tokens` mode validates `tokens.yaml` against the published token schema and reports category/value-shape errors before shell writers consume it.
- `assets` mode validates `assets.yaml` against the published asset schema, verifies referenced files under `design-system/assets/**`, and reports missing required platform sources.
- The define phase runs `layout` or `composition` validation before writing `composition.yaml`, then runs cross-artifact reference validation for token and asset names. The build phase repeats `composition` + token/asset validation on the resolved artifact set because change-local inputs may differ from the baseline.

### I. `/spec:build` contract

`/spec:build` consumes the wired UI input set and delegates implementation to core and shell writers. It does not invoke a design-system writer.

Inputs:

- Wired `composition.yaml` from the active change or baseline.
- `design.md`, `specs/**/*.md`, and generated / existing `app.rs`.
- Optional change-local `tokens.yaml`, falling back to `design-system/tokens.yaml`.
- Optional change-local `assets.yaml`, falling back to `design-system/assets.yaml`, plus referenced files under `design-system/assets/` or the change-local asset directory.
- Proposal `Platforms`, limited to `core`, `ios`, `android`, and future `web`.

Validation gate:

- Invoke the CLI's `composition` validation mode for `composition.yaml`, including schema validation and existing RFC-7 coverage rules: field coverage, event coverage, ViewModel mapping, overlay trigger consistency, and navigation consistency.
- When `tokens.yaml` exists, the CLI validates token schema and every token reference from `composition.yaml` / `assets.yaml`.
- When `assets.yaml` exists, the CLI validates asset schema, file existence, platform density/source coverage, and every asset reference from `composition.yaml`.
- Errors halt shell generation for affected screens or platforms. Warnings are reported but do not block generation.

Build phase ordering:

1. **Core.** Generate / update the Crux shared crate and tests.
2. **Shells.** Generate / update iOS and Android shells after core verification. Shell generation can run in parallel because both depend on the verified core and read-only input artifacts.
3. **Shell verification and review.** Verification remains serial where build tools contend for shared Rust artifacts; reviewer sub-agents can run in parallel when they touch disjoint shell trees.

Shell handoff:

- ios-writer receives `composition.yaml`, `tokens.yaml`, `assets.yaml`, image files, `app.rs`, `design.md`, and the iOS shell requirements. It emits SwiftUI layout, theme/token code, and asset catalog entries inside `iOS/`.
- android-writer receives the same artifact set plus Android shell requirements. It emits Compose layout, Material 3 theme/token code, and drawable/resource entries inside `Android/`.
- The shell writers own copy/reference logic for assets. There is no shared `vectis-assets-writer` in v1.
- The shell writers own token emit templates. The Swift templates that currently live under `design-system-writer` migrate to ios-writer; the Kotlin templates migrate to android-writer.

Reviewer surface:

- ios-reviewer and android-reviewer check unresolved asset references, missing copied resources, stale external design-system dependencies, and hardcoded visual literals that should come from tokens.
- Reviewers do not re-run the full input validation contract. They check generated platform code against the already-validated input set and flag drift introduced during generation.

Merge handoff:

- `/spec:merge` treats `composition.yaml`, `tokens.yaml`, `assets.yaml`, and referenced asset files as reviewable lifecycle artifacts when they appear in a change. `composition.yaml` continues to merge into the Specify baseline; token and asset updates merge into `design-system/tokens.yaml`, `design-system/assets.yaml`, and `design-system/assets/**`.
- Merge preview must show UI input deltas alongside spec/design/task changes so reviewers can understand which downstream shell generations will be affected.
- Merge validation re-runs the same token/asset reference checks used by build, even when a platform was not generated in the current change, because later shell work may consume the merged baseline input set.

### J. Skill shape, naming, and plugin layout

First-pass skills live under `plugins/vectis/skills/`. A sibling `ui` plugin remains an alternative for the day another runtime stack consumes the same artifacts, but it is unnecessary while Vectis is the only concrete shell target.

First-pass skill surface:

- `vectis-image-layout-inferer` (`/vectis:image-layout-inferer`)

Future candidate skill surface, subject to later RFC review:

- `vectis-figma-layout-inferer` (`/vectis:figma-layout-inferer`)
- `vectis-code-layout-inferer` (`/vectis:code-layout-inferer`)
- `vectis-tokens-inferer` (`/vectis:tokens-inferer`)
- `vectis-assets-inferer` (`/vectis:assets-inferer`)

The first-pass layout contract lives at `plugins/vectis/references/layout-inferer-contract.md` and is exercised by `vectis-image-layout-inferer`. If Figma and source-code inference are accepted later, they should be separate sibling skills rather than one `--source` dispatcher, because each source has different prerequisites, fixtures, troubleshooting, and examples. Keeping them separate also matches the diagram's producer boxes and improves skill discovery.

Future `vectis-tokens-inferer` and `vectis-assets-inferer` helpers would author input artifacts only. They should not run automatically during `/spec:define` or `/spec:build`; operators would invoke them when importing from external material.

`vectis:design-system-writer` is removed as an implementation skill and kept for one release as a deprecated no-op alias. Its body should explain the new path and exit without generating files. New briefs, tasks, and plans MUST NOT mention it.

Reference migration:

- `plugins/vectis/skills/design-system-writer/references/swift-token-templates.md` moves to `plugins/vectis/skills/ios-writer/references/`.
- `plugins/vectis/skills/design-system-writer/references/kotlin-token-templates.md` moves to `plugins/vectis/skills/android-writer/references/`.
- iOS and Android `design-system-integration.md` files are rewritten to describe shell-local token/theme code instead of external package/module consumption.

### K. Migration

Migration is a one-release transition that preserves existing inputs while removing generated shared libraries.

Existing `tokens.yaml`:

- Existing `design-system/tokens.yaml` files remain valid if they use the current categories (`colors`, `typography`, `spacing`, `cornerRadius`) and value shapes.
- The new schema is additive for `elevation`, `border`, and `opacity`. Existing projects do not need a one-shot token rewrite unless they relied on undocumented category shapes.
- The first validation pass should report schema warnings with clear category/value paths so operators can fix ambiguous tokens in place.

Existing layout / composition files:

- Existing input-only `design-system/composition.yaml` files should be renamed to `design-system/layout.yaml`. Existing wired `.specify/specs/composition.yaml` baselines and active change `composition.yaml` outputs remain valid.
- The only schema adjustment is provenance vocabulary expansion for `screenshots` and `code`; existing `figma`, `legacy`, and `manual` values remain valid.
- The CLI should expose separate validation entry points or flags for `layout.yaml` and `composition.yaml` even though both are grounded in `schemas/vectis/composition.schema.json`. The distinction is mode semantics: `layout.yaml` is unwired input; `composition.yaml` is the wired lifecycle artifact.

Existing generated design-system libraries:

- Projects that have `design-system/ios/` and `design-system/android/` should migrate in a dedicated change before unrelated feature work.
- The migration deletes generated library outputs, regenerates iOS and Android shells so theme/token code lives inside each shell tree, removes XcodeGen / Swift Package references to `VectisDesign`, and removes `:vectis-design` Gradle includes and dependencies.
- Do not add a one-shot `vectis-design-system-dissolve` skill in v1. The migration is structural but straightforward, and the normal shell writers should own the final generated shape. Document the steps and let reviewers catch stale dependencies.

Existing proposals, specs, tasks, and plans:

- New `proposal.md` files cannot list `design-system` in `Platforms`.
- Existing proposal deltas that still list `design-system` are treated as legacy input. `/spec:define` should rewrite them on next regeneration by removing `design-system` and attaching token / asset requirements to the affected shell platforms.
- Existing `## Design System Requirements` sections are migrated into platform-neutral requirements only when they describe observable behavior, or into iOS / Android shell requirements when they describe platform rendering obligations.
- Existing plan/discovery/propose language that refers to a design-system tier is replaced with cross-cutting UI input language. Token / asset / composition-only plan entries are allowed when they are independently reviewable, but they do not produce code except through later shell entries.

Existing assets:

- Projects without `assets.yaml` can continue if composition does not reference images/icons that need an inventory.
- When assets are referenced, hand-author `design-system/assets.yaml` before shell generation until a future asset helper is accepted.

Downstream consumer repos:

- The first regeneration after dissolution SHOULD ship as its own change so reviewers can separate structural movement from feature behavior.
- After migration, downstream repos should build without manual edits beyond removing stale external design-system dependencies and accepting regenerated shell-local theme/resource files.

### L. Dissolving the `design-system` peer platform

`design-system` is no longer a peer platform in proposals, tasks, plans, or build orchestration. The name remains useful only for the operator-maintained input directory: `design-system/layout.yaml`, `design-system/tokens.yaml`, `design-system/assets.yaml`, and `design-system/assets/<images>`. Nothing under that directory is generated as a runtime dependency.

The principle:

- **The "design system" name belongs to inputs, not emit.** `layout.yaml`, `tokens.yaml`, `assets.yaml`, and any future component vocabulary (§G) are the design system an operator maintains over time. They are the artifacts the inferers produce / refine and that `/spec:define` consumes.
- **There is no generated platform between inputs and runtime shells.** iOS and Android (and future web) are the only runtime platforms. Each shell writer reads the input artifacts directly and emits everything it needs inside its own tree: theme code, colour scheme, typography, spacing, corner radius, asset catalog wiring, and any reusable shell-local components.

What this means concretely:

- **Schema definition.** `schemas/vectis/schema.yaml` grows an `artifacts` contract that names the UI input set and phase hand-offs explicitly. Define consumes `layout.yaml`, `tokens.yaml`, `assets.yaml`, and asset files; define generates wired `composition.yaml` plus `design.md`, specs, and tasks; build consumes the wired composition and resolved design inputs; merge carries composition, token, asset-manifest, and asset-file deltas into the baseline. This makes `layout.yaml` / `tokens.yaml` / `assets.yaml` visible to the Specify lifecycle without pretending they are runtime platforms.
- **Proposal brief.** The `Platforms` enum (`schemas/vectis/briefs/proposal.md`) drops `design-system`. The remaining values are `core`, `ios`, `android`, and future `web`. `Platforms` continues to determine build scope; token or asset work is represented as input context for the shell platforms that consume it.
- **Specs brief.** `schemas/vectis/briefs/specs.md` retires `## Design System Requirements`. Requirements about tokens, assets, or component usage are written in the platform-neutral body only when they affect observable product behavior, or in `## iOS Shell Requirements` / `## Android Shell Requirements` when they are platform-specific rendering obligations.
- **Build brief.** `schemas/vectis/briefs/build.md` runs **core -> shells**. There is no design-system phase, no shared design-system verification step, no `VectisDesign` Swift Package, and no `:vectis-design` Gradle module. iOS and Android shell writers consume `tokens.yaml` and `assets.yaml` directly.
- **Plan briefs.** `schemas/vectis/briefs/plan/discovery.md` drops the design-system tier. Discovery reports layout, tokens, assets, and future components as cross-cutting UI inputs, with ordering hints naming the shell capabilities that consume them. `schemas/vectis/briefs/plan/propose.md` no longer creates a "design-tokens" rung between core and shells by default; token or asset changes become plan entries only when they are independently reviewable input-artifact work, and shell entries depend on them when needed.
- **Tasks brief.** `schemas/vectis/briefs/tasks.md` orders build work as core first, shells second. The skill table drops `vectis:design-system-writer`; shell tasks mention the relevant input artifacts when `layout.yaml`, `tokens.yaml`, or `assets.yaml` are in scope, and the generated `composition.yaml` when shell implementation consumes wired composition.
- **Composition brief.** The token-availability check in `schemas/vectis/briefs/composition.md` continues to reference `tokens.yaml`; its trigger simplifies to "`design-system/tokens.yaml` exists" or an explicit `tokens.yaml` path is supplied by the change. It no longer keys off `design-system` appearing in `Platforms`.
- **Vectis plugin.** `plugins/vectis/skills/design-system-writer/` is removed as an implementation skill. Its platform-specific references migrate into the shell writers: Swift token templates and HIG fallback policy into `plugins/vectis/skills/ios-writer/references/`, Kotlin token templates and Material 3 fallback policy into `plugins/vectis/skills/android-writer/references/`.
- **Generated layout.** The generated Theme and token code lives inside the shell trees. iOS emits its theme files under the app target, for example `iOS/<App>/Theme/`. Android emits its theme files under the app source tree, for example `Android/app/src/main/kotlin/.../ui/theme/`. The exact file split is a shell-writer concern, but generated apps MUST NOT depend on `import VectisDesign` or `implementation(project(":vectis-design"))`.

Compatibility policy:

- Keep `/vectis:design-system-writer` for one release as a deprecated alias that performs no generation and reports the replacement path: run `/vectis:ios-writer` and `/vectis:android-writer` for the shell platforms that consume `tokens.yaml` / `assets.yaml`. New plans and tasks MUST NOT emit the alias.
- Reviewers continue checking that generated app code uses token-backed colour, typography, spacing, and radius APIs rather than hardcoded literals. They also flag stale external design-system dependencies (`import VectisDesign`, `:vectis-design`, `design-system/ios`, `design-system/android`) as migration issues.
- Per-shell token parsing duplication is accepted for v1. The token schema is small, and shell-local parsing keeps each platform's fallback behavior explicit. A shared parser library is deferred until at least three shell targets need the same implementation.
- Future web follows the same rule: it consumes wired `composition.yaml`, `tokens.yaml`, and `assets.yaml` directly and emits theme / asset code inside the web shell. This RFC does not reintroduce a shared web design-system package.

## Open Questions

Resolved in this RFC:

1. The UI specification workflow is established around `layout.yaml`, `tokens.yaml`, `assets.yaml`, `/spec:define`, `/spec:build`, and `/spec:merge` (§A, §H, §I, §L).
2. The first-pass layout producer is `vectis-image-layout-inferer`, backed by the shared contract at `plugins/vectis/references/layout-inferer-contract.md` and deterministic CLI validation (§A, §C, §J).
3. Image inference uses a staged vision-assisted pipeline and ships fixtures; it does not infer tokens from pixels or crop production assets from screenshots (§C).
4. Figma and source-code layout inferers are future intent only. The goals are captured here, but their implementation details are illustrative and must be reviewed in future RFCs (§B, §D).
5. `assets.yaml` is a v1 artifact with raster, vector, and symbol entries; shell writers copy assets into their own platform catalogs (§E, §I).
6. `tokens.yaml` gets a one-file schema; YAML remains canonical, W3C DTCG is import/export only, and multi-brand is deferred (§F).
7. Component primitives are deferred; repeated structures are flattened into `layout.yaml` in v1 (§G).
8. `/spec:define` consumes requirements, optional `layout.yaml`, tokens, and assets; it emits the existing define artifacts plus wired `composition.yaml`, with `design.md` influenced through composition and no new `requirements.md` or `theme.md` (§H).
9. The Vectis schema declares the UI input/output contract so build consumes and merge carries forward composition, token, asset-manifest, and asset-file deltas (§H, §I, §L).
10. `/spec:build` runs core -> shells; shell writers consume tokens/assets directly; no shared assets-writer or design-system phase exists in v1 (§I, §L).
11. First-pass skill surface stays under Vectis; `vectis:design-system-writer` becomes a one-release deprecated no-op alias (§J).
12. Migration is a documented one-change consolidation, not a special migration skill (§K).
13. `design-system/` remains the input directory name; generated theme/resource code lives inside each shell tree; web follows the same pattern when it lands (§L).

Deferred beyond v1:

- `figma-layout-inferer` implementation details.
- `code-layout-inferer` implementation details, including source framework priorities.
- `tokens-inferer` and `assets-inferer` helper skills.
- `components.yaml` and any cross-platform component primitive vocabulary.
- Multi-brand / multi-theme token structures beyond light/dark.
- A shared parser library for tokens/assets across three or more shell targets.
- A sibling `ui` plugin for source-agnostic UI artifacts if non-Vectis consumers appear.

## Alternatives Considered

- **Status quo.** Keep the surface narrow: hand-author `layout.yaml` and `tokens.yaml`, no assets artifact. Cheap to maintain; defeats the point for any team starting from a deployed app or a Figma file.
- **One mega-inferer.** A single `vectis-ui-inferer` that takes any of (Figma, screenshots, code) and dispatches internally. Fewer skills to maintain; rejected because each source has different prerequisites, fixtures, failure modes, and examples, and separate skills are easier for agents to select.
- **Adopt an external token spec wholesale.** Replace `tokens.yaml` with W3C DTCG and outsource authoring to existing tools. Reduces what Specify owns; rejected for v1 because shell writers need a small stable contract and DTCG can remain an import/export format.
- **Fold everything into composition.** Make `composition.yaml` the only source of truth for layout, tokens, and assets. Conceptually simple; loses the ability to evolve tokens / assets independently and conflates concerns RFC-7 deliberately separated.
- **Spin a sibling `ui` plugin.** Move composition / inferers / tokens / assets into a UI plugin that vectis (and future shell plugins) consume. Forward-looking if web / desktop / TV shells arrive; deferred until there is a second concrete consumer.
- **Keep `design-system` as a peer platform** (status-quo for §L). Preserve `vectis:design-system-writer` and the shared `VectisDesign` / `vectis-design` libraries between tokens and shells. Pros: emit logic lives once, the shell writers stay smaller, downstream apps can import the libraries directly. Cons: introduces a build artifact that is never deployed on its own, complicates the proposal `Platforms` vocabulary, forces an external-library boundary onto theming code that each shell would otherwise own internally, and leaks platform packaging concerns (Swift Package, Gradle module, version pinning) into a surface whose only legitimate role is *input*. Rejected because the cost of the peer-platform shape exceeds the benefit of single-emit; the duplication §L introduces (each shell writer parses tokens / assets directly) is small and self-contained.

## References

- [RFC-7: View Layout Artifact for UI Generation](archive/rfc-7-ui.md) — the composition artifact and skeleton/wired duality this RFC operationalises
- [`schemas/vectis/schema.yaml`](../schemas/vectis/schema.yaml) — the Vectis schema definition that this RFC says should gain an artifact contract for layout, tokens, assets, define outputs, build inputs, and merge-managed UI input deltas
- [`.cursor/schemas/specify-schema.schema.json`](../.cursor/schemas/specify-schema.schema.json) — the schema validator that will need to permit schema-level artifact contracts when this RFC is implemented
- [`schemas/vectis/composition.schema.json`](../schemas/vectis/composition.schema.json) — the schema that both `layout.yaml` input and wired `composition.yaml` output validate against
- [`schemas/vectis/briefs/composition.md`](../schemas/vectis/briefs/composition.md) — the existing composition brief that consumes `layout.yaml` at define time and emits wired `composition.yaml`
- [`schemas/vectis/briefs/build.md`](../schemas/vectis/briefs/build.md) — the build brief whose hand-off list grows with `assets.yaml` + image files, and whose phase ordering simplifies once design-system retires (§I, §L)
- [`schemas/vectis/briefs/proposal.md`](../schemas/vectis/briefs/proposal.md) — the proposal brief whose `Platforms` vocabulary loses `design-system` (§L)
- [`schemas/vectis/briefs/specs.md`](../schemas/vectis/briefs/specs.md) — the specs brief whose `## Design System Requirements` section reframes once the peer platform retires (§L)
- [`schemas/vectis/briefs/tasks.md`](../schemas/vectis/briefs/tasks.md) — the tasks brief whose phase ordering and skill table shed the design-system tier (§L)
- [`schemas/vectis/briefs/plan/discovery.md`](../schemas/vectis/briefs/plan/discovery.md) and [`plan/propose.md`](../schemas/vectis/briefs/plan/propose.md) — the plan briefs whose tier model loses the design-system rung (§L)
- [`plugins/vectis/skills/design-system-writer/SKILL.md`](../plugins/vectis/skills/design-system-writer/SKILL.md) — the current emit-only design-system skill that this RFC dissolves (§F, §L); its references migrate into the shell writers
- [`plugins/vectis/skills/ios-writer/references/design-system-integration.md`](../plugins/vectis/skills/ios-writer/references/design-system-integration.md) — current iOS consumer-side rules (token usage, fallback policy)
- [`plugins/vectis/skills/android-writer/references/design-system-integration.md`](../plugins/vectis/skills/android-writer/references/design-system-integration.md) — current Android consumer-side rules and Material 3 fallback policy
- [`plugins/spec/skills/extract/SKILL.md`](../plugins/spec/skills/extract/SKILL.md) — the existing source-code extraction skill that the `code-layout-inferer` may sit alongside (§D)
- [Roadmap](roadmap.md) — directional principles (CLI-authoritative, local-and-reviewable, separation of workflow / standards / artifacts) this RFC must respect
- [`rfcs/assets/ui-spec.png`](assets/ui-spec.png) — the high-level diagram this RFC scopes
