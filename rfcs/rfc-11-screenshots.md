# RFC-11: Composition From Screenshots

> Status: Draft · Depends: [RFC-7](archive/rfc-7-ui.md), [RFC-10](archive/rfc-10-skills.md)

## Abstract

Add one or more Vectis specialist skills that produce a `composition.yaml` artifact by having an agent analyse screenshot images of an existing application. The new skill(s) target the case where a team wants to rebuild — or visually mirror — an existing UI as a Crux app and the only authoritative source of the current layout is the running product itself. The artifact produced conforms to the existing composition schema ([`schemas/vectis/composition.schema.json`](../schemas/vectis/composition.schema.json)) and slots into the same `define` pipeline that today consumes spec-derived or Figma-derived skeletons.

## Motivation

[RFC-7](archive/rfc-7-ui.md) established `composition.yaml` as a multi-source artifact: it can be authored by the define agent from specs, imported from Figma Auto Layout, reverse-engineered from a legacy app, or hand-edited. The composition brief at [`schemas/vectis/briefs/composition.md`](../schemas/vectis/briefs/composition.md) already supports a "skeleton" mode where layout structure is preserved and only behavioural keys (`bind`, `event`, `maps_to`, `*-when`) are added at define time.

Today only two of those source paths are operational:

- The define agent can **infer** a composition from specs alone (low fidelity for non-trivial UIs — see RFC-7 §"The Inference Gap").
- A human can **hand-author** a skeleton or paste in a Figma export.

The "reverse-engineer the existing application" path that RFC-7 calls out is unbuilt. In practice this is one of the most common entry points for a Vectis project: a team has a working iOS, Android, or web app, no Figma file, no behavioural specs, and they want a Crux rewrite that visually matches the original screen-for-screen. Forcing the operator to translate screenshots into the composition vocabulary by hand defeats the point of having specialist skills.

The hypothesis behind this RFC is that the composition vocabulary — regions, groups with flex-like properties, a small leaf-item set — is constrained enough that a vision-capable agent, given screenshots and the schema, can produce a usable skeleton. The downstream composition brief then enriches it with bindings as it does for any other skeleton source.

### Non-goals

- **No spec generation from screenshots.** Behavioural specs are out of scope; this RFC produces composition only. Specs continue to come from `/spec:define`, `/spec:extract`, or hand-authoring.
- **No design-token extraction.** Token inference (colour palettes, type scales, spacing scales from pixel measurements) is its own problem with its own quality bar. The skill MAY emit raw values where tokens would normally appear and surface a gap report; it does not attempt to reverse a `tokens.yaml`.
- **No replacement of the composition brief.** The new skill produces a skeleton-grade artifact; the existing `composition` brief in the Vectis define pipeline remains responsible for binding the skeleton to ViewModel fields and Event variants once specs exist.
- **No live screen capture.** Screenshots are an input. How they are captured (manual, scripted, instrumentation) is outside this RFC.

## Detailed Design

> This section is intentionally a scaffold. Subsections marked **TBD** will be filled in over follow-up sessions; the bullets under each are the questions that need to be resolved, not the answers.

### A. Skill shape

**TBD.** Open questions:

- One skill or several? A single `vectis-composition-from-screenshots` covers the simple case; splitting into per-stage skills (e.g. *triage screenshots* → *infer regions* → *assemble composition*) better matches the progressive-disclosure pattern RFC-10 codified for larger skills.
- Where does it live? `plugins/vectis/skills/composition-from-screenshots/` is the natural home alongside the other Vectis writers; an alternative is a sibling top-level "extractor" plugin if the same vision-driven approach later applies to other artifacts.
- Does it integrate with `/spec:extract`, or stay independent? `/spec:extract` is source-code-driven today; making it screenshot-aware would blur its responsibility. A standalone Vectis skill called by the operator (or by `/spec:define` when a `screenshots/` input is present) is the safer default.

### B. Inputs

**TBD.** The skill needs to accept:

- One or more screenshot files per screen (PNG/JPEG; possibly HEIC). Resolution and aspect ratio are the operator's responsibility.
- Optional grouping signal — which screenshots represent the same screen in different states (loading/empty/populated/error) versus distinct screens.
- Optional platform hint (`ios` / `android` / `web`) so the agent can ignore platform chrome (status bar, navigation bar, system gestures) and not encode it as composition.
- Optional baseline `composition.yaml` to update rather than recreate.

Open questions: file naming convention vs. an explicit manifest; how to denote multi-state screens; whether per-screen prose hints are part of the input.

### C. Generation pipeline

**TBD.** The pipeline a single invocation runs through. Candidate stages:

1. **Triage.** Cluster the input screenshots into screens and states. Produce a screen inventory.
2. **Region pass.** For each screen, identify the four canonical regions (`header`, `body`, `footer`, optional `fab`). Reject screens that do not fit the region model and surface as gaps.
3. **Container pass.** Within each region, infer the group tree — `direction`, `gap`, `padding`, `align`, `justify`, sizing modes, surface decoration — using the vocabulary in [`schemas/vectis/composition.schema.json`](../schemas/vectis/composition.schema.json).
4. **Leaf-item pass.** Map visual elements to the item vocabulary (text/title/badge/icon/button/field/list/grid/segments/…).
5. **Verification.** Validate the emitted YAML against the composition schema; round-trip through `specify schema check` if available; emit a gap report for anything the agent could not classify.

Open questions: per-stage agent vs. one-shot prompt; whether a vision model + a text model are split or combined; how to keep the agent grounded in the vocabulary (schema injection vs. few-shot examples vs. both).

### D. Output

**TBD.** The artifact is a `composition.yaml` conformant with the existing schema, with:

- `provenance.sources` set to `kind: legacy` (or a new `kind: screenshots` if the schema needs to distinguish; an extension is likely cheaper than overloading `legacy`).
- Skeleton-grade content: regions, groups with layout properties, leaf items with token references where confidently inferable.
- **No** `maps_to`, `bind`, `event`, or `*-when` keys. Those are the composition brief's job once specs exist; emitting placeholders here would create ambiguity about what was authoritative.
- Gap reports as YAML comments per the existing convention (`# GAP: …`).

Open question: schema-level provenance change vs. reuse of `legacy`.

### E. Integration with the define pipeline

**TBD.** Two viable shapes:

1. **Pre-define generation.** The operator runs `/vectis:composition-from-screenshots` to produce `composition.yaml` in the change directory; `/spec:define` then runs the existing composition brief in skeleton mode and enriches it.
2. **Brief-orchestrated.** The composition brief in the Vectis define pipeline detects a `screenshots/` directory in the change and dispatches to the new skill internally.

The first is simpler and matches how Figma exports are expected to work; the second is more ergonomic but couples the brief to a specific extractor. Resolve in a follow-up session.

### F. Quality bar and verification

**TBD.** What "good enough" looks like for the skeleton:

- Schema-valid YAML, no exceptions.
- Every visible region in the screenshot accounted for, even if as a gap comment.
- Group structure that is *defensible* on inspection — the operator can tell the agent's intent — even if not pixel-perfect.
- A clear, reviewable gap report rather than silent guesses.

Open questions: do we ship reference fixtures (screenshot → expected composition pairs) as part of the skill for regression testing? Do we need a dedicated reviewer skill (mirroring `vectis-core-reviewer`) or is review folded into the writer's verification step?

### G. SKILL.md authoring

**TBD.** The skill(s) MUST follow [RFC-10](archive/rfc-10-skills.md) conventions verbatim:

- `name` is plugin-qualified (`vectis-composition-from-screenshots` or per-stage equivalents).
- `description` includes both *what* and *when to use* in third person.
- Body stays under the 500-line ceiling using progressive disclosure into siblings.
- A `## Critical Path (Quick Reference)` block leads the body.

## Open Questions

Consolidated for the next session(s):

1. Single skill or multi-stage skill set? (§A)
2. Input shape — convention vs. manifest? (§B)
3. Pipeline shape — one agent loop or staged passes? (§C)
4. Provenance kind — extend the schema or reuse `legacy`? (§D)
5. Pre-define skill vs. brief-orchestrated dispatch? (§E)
6. Reference-fixture testing strategy and need for a paired reviewer? (§F)
7. Vision model choice and whether the skill assumes a specific capability tier on the host runtime.
8. How (if at all) screenshots themselves are persisted in the change directory for review.

## Alternatives Considered

**TBD.** Candidates to evaluate when the design firms up:

- **Hand-authored skeletons only.** Status quo. Cheap to maintain; high operator friction for any non-trivial existing app.
- **Figma-import-only.** Forces operators to recreate the existing app in Figma first. Defeats the point when the goal is to mirror what already ships.
- **Outsource to a third-party design-to-code tool.** Some commercial tools (e.g. screenshot → Tailwind) exist; they target HTML/CSS, not the platform-neutral composition vocabulary, and would require a translation layer that is itself most of the work.
- **Source-tree extraction.** For platforms with declarative UI (SwiftUI, Compose) the source could in principle be parsed. Out of scope here — different inputs, different skill, possible follow-up RFC.

## References

- [RFC-7: View Layout Artifact for UI Generation](archive/rfc-7-ui.md) — the composition artifact this skill produces
- [RFC-10: Skill Improvements](archive/rfc-10-skills.md) — frontmatter, naming, body-size, and progressive-disclosure conventions
- [`schemas/vectis/composition.schema.json`](../schemas/vectis/composition.schema.json) — the composition schema the artifact must validate against
- [`schemas/vectis/briefs/composition.md`](../schemas/vectis/briefs/composition.md) — the existing composition brief that consumes a skeleton in skeleton mode
- [`plugins/vectis/skills/`](../plugins/vectis/skills/) — the existing Vectis skill set this RFC extends
