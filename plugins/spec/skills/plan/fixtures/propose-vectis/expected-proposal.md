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

- Heuristics applied (Vectis, from `schemas/vectis/briefs/plan/propose.md`):
  shared-core first (slices 1–2), design-system next (slice 4),
  per-shell last (slices 5–6). Each shell slice depends on the
  corresponding `-core` entry plus `design-tokens`; iOS and
  Android shells never depend on each other. Cross-cutting
  refactors (slice 3) are presented *before* the shell slices
  that would seed their edges so a reject only trims upcoming
  drafts (never already-written entries).
- Slice 3 (`extract-shared-viewmodel-adapter`) was proposed by
  the Vectis cross-cutting heuristic after observing that both
  legacy shells wrote near-identical ViewModel→SwiftUI /
  ViewModel→Compose mapping glue. Rejected for this initiative —
  the operator preferred to defer the refactor until a second
  feature lands and the full ViewModel mapping surface is
  visible. The brief had seeded `depends-on:
  [extract-shared-viewmodel-adapter]` on slices 5 and 6 (the two
  shell views); that edge was dropped from both draft slices
  before they were presented.
- Slice 6's `description` was edited during review to name
  Material 3 and the `vectis-design` Compose library explicitly
  (the draft had read "Compose view"); the slice name and
  dependency edges were unchanged.
- Open questions from discovery answered inline:
  - *Should `theme-core` own the light/dark toggle state?* —
    yes, retained on `theme-core` (slice 2); the shells read it
    via the `ViewModel`.
  - *Do we ship the Android `vectis-design` library as a
    sibling module or a published artifact?* — sibling module
    in the counter app's Gradle build for this initiative; a
    standalone publication is deferred.
- `specify initiative validate` — no errors.
