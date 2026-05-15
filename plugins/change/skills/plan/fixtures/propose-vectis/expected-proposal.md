# Proposal — counter-migration

## Slices

| # | Slice                          | Source(s)                    | Depends on                    | Decision      | Plan entry            |
|---|--------------------------------|------------------------------|-------------------------------|---------------|-----------------------|
| 1 | counter-core                   | legacy-ios, legacy-android   | —                             | accept        | counter-core          |
| 2 | theme-core                     | legacy-ios, legacy-android   | —                             | accept        | theme-core            |
| 3 | extract-shared-viewmodel-adapter | —                          | —                             | reject        | —                     |
| 4 | design-tokens                  | legacy-tokens                | theme-core                    | accept        | design-tokens         |
| 5 | counter-ios-view               | legacy-ios                   | counter-core, design-tokens   | accept        | counter-ios-view      |
| 6 | counter-android-view           | legacy-android               | counter-core, design-tokens   | edit → accept | counter-android-view  |

## Notes

- Heuristics applied (Vectis, from `plugins/change/skills/plan/briefs/vectis/propose.md`): shared-core first (slices 1–2). The `design-tokens` UI-input slice was promoted under heuristic 2 — the legacy token palette is large enough to warrant an independent review pass, so it was sliced rather than folded into the shell entries — and slotted between core and shells (slice 4). Per-shell last (slices 5–6); each shell slice depends on the corresponding `-core` entry plus the promoted `design-tokens` input. Cross-cutting refactors (slice 3) are presented *before* the UI-input and shell slices that would seed their edges so a reject only trims upcoming drafts (never already-written entries). iOS and Android shells never depend on each other.
- Slice 3 (`extract-shared-viewmodel-adapter`) was proposed by the Vectis cross-cutting heuristic after observing that both legacy shells wrote near-identical ViewModel→SwiftUI / ViewModel→Compose mapping glue. Rejected for this change — the operator preferred to defer the refactor until a second feature lands and the full ViewModel mapping surface is visible. The brief had seeded `depends-on: [extract-shared-viewmodel-adapter]` on slices 5 and 6 (the two shell views); that edge was dropped from both draft slices before they were presented.
- Slice 6's `description` was edited during review to name Material 3 explicitly (the draft had read "Compose view"); the slice name and dependency edges were unchanged.
- Open questions from discovery answered inline:
  - *Should `theme-core` own the light/dark toggle state?* — yes, retained on `theme-core` (slice 2); the shells read it via the `ViewModel`.
  - *The legacy Android codebase ships custom motion / elevation tokens that have no iOS counterpart — surface them as Android-only theme entries or omit from `tokens.yaml`?* — surface as Android-only for now; revisit when iOS adds motion support.
- `specify plan validate` — no errors.
