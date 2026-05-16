---
id: cluster
description: Per-source clustering prompt for /change:survey (Vectis).
generates: .specify/plans/<name>/survey.md
---

# Vectis legacy-code clustering for `/change:survey`

This brief carries Vectis's source-local clustering refinements for the `/change:survey` skill. The skill resolves this file and applies the refinements below after the global candidate algorithm in [`/change:survey` SKILL.md §Candidate algorithm](../../SKILL.md).

---

## Clustering refinements

v1 has no Vectis-specific clustering refinements beyond the global algorithm. The Crux stack's three-tier decomposition (shared core, iOS shell, Android shell) is a propose-time concern handled by the Vectis propose brief — survey operates at the source-local surface level and does not classify candidates into Crux tiers.

When Vectis-specific clustering signals emerge (e.g. detecting `App` trait boundaries from Rust Crux sources, or pairing SwiftUI/Compose views with their shared-core counterpart), they will follow the same structure as the Omnia clustering brief at [`plugins/change/skills/survey/briefs/omnia/cluster.md`](../omnia/cluster.md).

## Interaction with Vectis tiers

Survey emits flat `kind: candidate` blocks. The Vectis propose brief ([`plugins/change/skills/draft/briefs/vectis/propose.md`](../../../draft/briefs/vectis/propose.md)) is responsible for classifying accepted candidates into the shared-core / iOS-shell / Android-shell tiers and the cross-cutting UI-inputs section. Survey does not pre-classify.
