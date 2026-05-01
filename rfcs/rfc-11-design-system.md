# RFC-11: Design System Workflow

> Status: Draft · Depends: [RFC-7](archive/rfc-7-ui.md)

## Abstract

Rethink the **design-system workflow** that Vectis ships today. RFC-7 introduced `composition.yaml` as a multi-source layout artifact and assumed `design-system/tokens.yaml` as its companion source-of-truth for typography, colour, spacing, and corner-radius tokens. The token side of that pair has stayed thin: a single hand-authored YAML file, a deterministic `vectis:design-system-writer` that maps it to one Swift Package and one Compose library, and downstream shell writers that quote the generated symbols by name. This RFC broadens the design-system surface to match the maturity the composition surface reached in RFC-7 — a typed artifact with a published schema, multiple authoring sources (manual, Figma Variables, design-token tools, legacy app extraction), explicit verification against composition, and a richer vocabulary for the things shell writers and reviewers already need (motion, elevation, iconography, component primitives, multi-brand theming).

The previous RFC-11 (`screenshots → composition`) is dropped from this slot. Source-of-truth import for the *layout* artifact may return as a follow-up, but it is a separate concern from the *tokens / design-system* artifact this RFC scopes.

## Motivation

### What Ships Today

The vectis design-system workflow is currently:

1. **One artifact.** `design-system/tokens.yaml`, hand-authored, no JSON Schema, value shapes inferred from the first entry per category (colour with `light`/`dark`, font with `size`/`weight`, scalar number).
2. **One generator skill.** [`vectis:design-system-writer`](../plugins/vectis/skills/design-system-writer/SKILL.md) regenerates an iOS Swift Package (`design-system/ios/`, `VectisDesign`) and an Android Compose library (`design-system/android/`, `vectis-design`). Mapping is mechanical; new top-level YAML keys require coordinated edits to `references/swift-token-templates.md` and `references/kotlin-token-templates.md`.
3. **Tokens-by-name in composition.** [`composition.yaml`](../schemas/vectis/briefs/composition.md) references token symbols as bare strings (`gap: md`, `color: primary`, `style: body`). The composition schema marks them only as `"Reference to a design token name from tokens.yaml."` — no enforced resolution.
4. **Tokens-by-symbol in shells.** The iOS and Android writers consume `VectisColors`, `VectisTypography`, `VectisSpacing`, `VectisCornerRadius` directly; their reviewers flag any hardcoded hex/RGB, inline `.system(size:)`, or magic-number padding as a violation.
5. **One slice in the plan.** The vectis [propose brief](../schemas/vectis/briefs/plan/propose.md) treats `design-tokens` as a single mid-tier entry between shared-core and per-shell entries; splits are allowed but undirected.
6. **Implicit fallback.** When `tokens.yaml` is absent the shell writers fall back to platform defaults (Material You dynamic colour on Android, system styles on iOS). That policy is documented inside each shell writer's references rather than owned by the design-system surface.

### What Is Missing

The thin slice above works for a one-brand, one-platform-pair, four-category app. It bottlenecks for everything else. The gaps that motivate this RFC:

- **No published token schema.** Adding a category (motion, elevation, gradient, iconography) is a manual coordination across writer + per-platform templates with no validator to keep the three in step.
- **No authoring or import skill.** Operators write `tokens.yaml` from scratch even when a Figma Variables file, a Style Dictionary export, or a legacy app's stylesheet already encodes the same data.
- **No verification.** `composition.yaml` token references are not checked against `tokens.yaml`; nothing fails when a `gap: huge` token name does not exist.
- **No component-primitive layer.** RFC-7 sketched a future `design-system/components.yaml` for reusable item compositions but it never landed; today every screen reassembles common patterns (cards, list rows, dialog footers) from primitive items.
- **Single theme, single brand.** No support for brand variants, white-label themes, or anything beyond light/dark.
- **No canonical export format.** The writer emits Swift and Kotlin; web (RFC-7's open follow-up) and any future platform require their own bespoke mapping rather than a shared intermediate (e.g. W3C Design Tokens spec).
- **Loose plan integration.** A non-trivial design-system change (new motion vocabulary, new brand) cannot be sliced into reviewable units within the current `design-tokens` mid-tier.

### Non-goals

- **Not a replacement for `composition.yaml`.** Layout stays in composition; this RFC only touches token, primitive, and theming surfaces.
- **Not a design tool.** Specify does not become a Figma. Tokens authoring may import from external tools; visual editing is out of scope.
- **Not a runtime theming engine.** Output is generated source the shells compile against, same as today; no dynamic theme-swap protocol.
- **Not a hosted token service.** All artifacts stay local and reviewable, per the [roadmap directional principles](roadmap.md#directional-principles).
- **Not the layout-import problem.** Source-of-truth import for `composition.yaml` (Figma frames, screenshots, legacy app DOMs) is a separate concern; if it returns it does so as its own RFC.

## Detailed Design

> This section is intentionally a scaffold. Subsections marked **TBD** are the questions that need to be resolved over follow-up sessions; the bullets are the open questions, not the answers.

### A. Tokens artifact and schema

**TBD.** Open questions:

- Is `design-system/tokens.yaml` still one file, or does it split (e.g. `tokens/colour.yaml`, `tokens/typography.yaml`, `tokens/motion.yaml`) once the vocabulary grows?
- What does the published JSON Schema for tokens look like? Today value shapes (colour / font / scalar) are inferred; should they be declared per-category in the schema instead?
- Does the artifact carry a `provenance` block analogous to `composition.yaml` (kinds: `manual`, `figma-variables`, `style-dictionary`, `legacy`)?
- Does the artifact carry a `version` and per-category `meta` (description, deprecation, replacement) to support rename / deprecation flows?
- Do tokens get baseline + per-change deltas the way specs and composition do, or are they always overwrite-style?

### B. Vocabulary scope

**TBD.** What categories belong in v1 vs. follow-ups:

- **Colour** (current). Light/dark; semantic vs. palette layering.
- **Typography** (current). Beyond `size`/`weight`: line-height, letter-spacing, font-family, font-stack fallback, italic.
- **Spacing** (current). Scale and named tokens.
- **Corner radius** (current). Scalar.
- **Elevation / shadow.** Composition's `group.elevation` already references token names with no backing category.
- **Border.** Composition's `group.border` ditto.
- **Motion.** Durations, easings, transitions; consumed by future animation primitives.
- **Iconography.** Composition uses `icon: { name: trash }` with no central name table; should icons be a token category mapping logical names to per-platform symbol sets (SF Symbols, Material Symbols)?
- **Opacity.** Including the disabled-state convention (38%) currently hardcoded in shell writer references.
- **Gradient.** If the design language requires it.

Open question: which of these are first-class in v1, which are deferred, and what is the extension contract for adding new categories without coordinated template edits?

### C. Component primitive layer

**TBD.** RFC-7's deferred `design-system/components.yaml` idea — named compositions of primitive items (cards, list rows, dialog footers, error states) that screens can reference by name in `composition.yaml`.

Open questions:

- Does this land here, or stay deferred?
- If it lands: what is the artifact, what is the schema, and how does composition reference a primitive (`- card: { ... }` becoming a recognised group shape)?
- Does it live alongside tokens (`design-system/components.yaml`) or under composition?
- How are primitives generated on each platform — composed from token-aware view code, or expressed as Swift / Kotlin templates the design-system-writer renders?
- What is the relationship between primitives and the existing item vocabulary? Are primitives macros that expand at brief time, or first-class entities the shell writers know about?

### D. Authoring and import surface

**TBD.** Today the only path is hand-edit `tokens.yaml`. The interfaces plugin pattern (author / import / verify intents per format, see [`plugins/interfaces/`](../plugins/interfaces/)) is the most natural model.

Candidate intents:

- **author.** Generate or extend `tokens.yaml` from a brief (e.g. brand prompt, palette description).
- **import.** Pull from an external source — Figma Variables export, Tokens Studio JSON, Style Dictionary input, a legacy app stylesheet (CSS variables, `Colors.xml`, `UIColor` extensions).
- **verify.** Internal consistency (no orphan tokens, no duplicate names, light/dark coverage) and **cross-artifact** consistency (every token referenced from `composition.yaml` resolves; every token defined in `tokens.yaml` is referenced or explicitly marked unused).

Open questions:

- One specialist skill (`vectis-design-system`) with internal intents, mirroring the interfaces plugin? Or one skill per intent (`design-system-author`, `design-system-import`, `design-system-verify`)?
- For import: which formats are first-class in v1 (Figma Variables, Tokens Studio, Style Dictionary, W3C Design Tokens Community Group spec)?
- Is `vectis:design-system-writer` reframed as the **emit** intent of this skill, or kept as a sibling generator?
- How does `/spec:extract` interact — does extracting from a legacy app produce a `tokens.yaml` skeleton alongside the spec/design artifacts?

### E. Verification and the composition contract

**TBD.** Today there is no checked contract between `composition.yaml` and `tokens.yaml`.

Open questions:

- Where does the cross-artifact validator live — in the design-system surface (verify intent) or in the existing `specify validate` cross-artifact pass?
- What is the failure mode when a composition references an unknown token: hard fail, warning, or auto-coerced fallback to a default token?
- Do composition tokens resolve through a *semantic* layer (e.g. `color: primary` resolves to whatever `colorRoles.primary` points at, which may then resolve to a palette swatch) or stay direct?
- Do shell reviewers (ios-reviewer / android-reviewer) gain a token-resolution check, or does the writer already guarantee it by construction?
- How are renames and deprecations surfaced — both at validate time and to operators planning the change?

### F. Multi-platform export

**TBD.** Today: Swift Package + Compose library. RFC-7's open follow-up is web. Any new platform repeats the writer-template pattern.

Open questions:

- Does the workflow gain a canonical intermediate format (W3C Design Tokens Community Group spec, or our own JSON) that platform emitters consume, decoupling token vocabulary from platform mappings?
- Or does the writer keep direct YAML→platform emission, with the platform list growing per-target?
- Where does **web** land — same `vectis:design-system-writer` with a third output, or a sibling skill (`vectis-design-system-web`) that shares a token reader but owns its emission?
- How are platform-specific overrides expressed (e.g. a colour that must differ between iOS and Android because of system contrast rules)?
- Does the M3 / HIG fallback (currently spread across the shell writers' "no tokens.yaml" branches) move into an explicit "stock theme" the design-system surface owns?

### G. Multi-brand and theming

**TBD.** No current support beyond light/dark.

Open questions:

- Does the artifact support multiple themes (e.g. `themes: { default: …, holiday: …, partner-x: … }`) or one theme per file with a separate manifest?
- How are themes selected on each platform — generated as separate `VectisColorScheme` factories the shell consumes, or a runtime theme-id lookup?
- Does this interact with the proposal's `Platforms` list (a platform listing `design-system` may now imply multiple per-platform outputs per theme)?
- Is there a "brand baseline + theme delta" model analogous to the Specify spec baseline + change-delta pattern?

### H. Plan, brief, and pipeline integration

**TBD.** Where the new surface plugs into the workflow contract.

Open questions:

- Does the `pipeline.plan` propose brief gain finer-grained design-system tiers (`design-tokens`, `design-primitives`, `design-themes`) or stay as one `design-tokens` slice with sub-task organisation?
- Does the vectis define pipeline grow new briefs (e.g. `tokens.md`, `components.md`) or fold into the existing composition brief?
- Does `tokens.yaml` (and any future `components.yaml`, `themes.yaml`) participate in the spec baseline + delta lifecycle the way `composition.yaml` does?
- Does `specify change validate` learn the new cross-artifact rules, and does `specify validate` (project-wide) gain a token-graph health check?
- How does the build brief's shell-writer handoff change — do iOS / Android writers continue to receive `tokens.yaml` directly, or do they receive a derived theme manifest the design-system surface produced?

### I. Skill shape and ownership

**TBD.** Naming and decomposition for the new surface, following the [skill-authoring conventions](../docs/explanation/skill-authoring.md).

Open questions:

- Single specialist skill `vectis-design-system` with author / import / verify / emit intents, mirroring `interfaces-openapi`?
- Or split: `vectis-design-system-author`, `vectis-design-system-import`, `vectis-design-system-verify`, with `vectis-design-system-writer` (current skill) as the emit half?
- Does the existing `vectis:design-system-writer` get renamed (e.g. `vectis-design-system-emitter`) or kept for backward-compatible discoverability?
- What is the slash-command surface — `/vectis:design-system <intent>`, or per-intent commands?
- Where does the platform-fallback ("stock theme" when no tokens) policy live — owned by this skill, or by the shell writers as today?

### J. Migration

**TBD.** How a project that already has `design-system/tokens.yaml` upgrades to the new surface.

Open questions:

- Is the existing `tokens.yaml` shape forward-compatible (new categories additive, value-shape inference preserved) or does it need a one-shot migration?
- Does the existing `design-system/ios/` and `design-system/android/` output stay byte-identical for unchanged input?
- Does the existing `design-system-writer` SKILL stay until its replacement reaches parity, then deprecate? On what timeline?
- Do downstream consumer repositories need any change beyond regenerating?

## Open Questions

Consolidated for the next iteration session(s):

1. Tokens artifact shape: one file vs. split, schema-first vs. inferred, baseline+delta vs. overwrite. (§A)
2. Which categories ship in v1 vs. follow-ups, especially elevation / motion / iconography that composition already references. (§B)
3. Does the component-primitive layer (RFC-7 deferred `components.yaml`) land in this RFC or wait for its own. (§C)
4. Author / import / verify decomposition vs. one combined skill; which import formats are first-class. (§D)
5. Where the composition↔tokens cross-artifact contract is enforced and what its failure modes are. (§E)
6. Canonical intermediate token format vs. direct platform emission; web target ownership. (§F)
7. Multi-brand / multi-theme support — artifact shape and platform projection. (§G)
8. Plan-tier granularity, brief growth, and validate-time integration. (§H)
9. Skill shape (single vs. split), naming, and the fate of `vectis:design-system-writer`. (§I)
10. Migration story for existing `tokens.yaml` projects. (§J)

## Alternatives Considered

**TBD.** Candidates to evaluate when the design firms up:

- **Status quo.** Leave the design-system surface thin; absorb new categories as one-off writer extensions. Cheap to maintain; punts every gap above to ad-hoc edits.
- **Adopt an external token spec wholesale.** Replace `tokens.yaml` with the W3C Design Tokens Community Group format and outsource authoring to existing tools. Reduces what Specify owns; couples vectis to an external spec's evolution.
- **Fold design-system into composition.** Make `composition.yaml` the only source of truth for both layout and design-system data. Conceptually simple; loses the ability to evolve tokens independently of layout and conflates two concerns RFC-7 deliberately separated.
- **Split design-system into a sibling plugin.** A standalone `design-system` plugin (peer of `vectis`) that any UI plugin can consume. Forward-looking if web / desktop / TV shells arrive; over-engineered if vectis remains the only consumer.

## References

- [RFC-7: View Layout Artifact for UI Generation](archive/rfc-7-ui.md) — the layout artifact this surface complements; introduced the assumption of a `tokens.yaml` companion
- [`plugins/vectis/skills/design-system-writer/SKILL.md`](../plugins/vectis/skills/design-system-writer/SKILL.md) — the current emit-only skill this RFC reframes
- [`schemas/vectis/composition.schema.json`](../schemas/vectis/composition.schema.json) — the composition schema whose token references this RFC will make resolvable
- [`schemas/vectis/briefs/composition.md`](../schemas/vectis/briefs/composition.md) — the composition brief that consumes the design-system surface
- [`schemas/vectis/briefs/plan/propose.md`](../schemas/vectis/briefs/plan/propose.md) — the plan brief whose `design-tokens` tier this RFC may refine
- [`plugins/interfaces/`](../plugins/interfaces/) — author / import / verify intent pattern this RFC's skill shape may follow
- [`plugins/vectis/skills/ios-writer/references/design-system-integration.md`](../plugins/vectis/skills/ios-writer/references/design-system-integration.md) — current iOS consumer-side rules
- [`plugins/vectis/skills/android-writer/references/design-system-integration.md`](../plugins/vectis/skills/android-writer/references/design-system-integration.md) — current Android consumer-side rules and Material 3 fallback policy
- [Roadmap](roadmap.md) — directional principles (CLI-authoritative, local-and-reviewable, separation of workflow / standards / artifacts) this RFC must respect
