# Monolith propose fixture

Pins the 1:1 capability → slice mapping the propose brief applies to a unified-YAML `discovery.md`. Consumes the C22 fixture's discovery output verbatim and emits a three-entry `plan.yaml` with rich `description` fields carrying file-path hints from each capability's `sources:` list and delta-targeting intent.

This fixture is the acceptance target for [RFC-3a C24](../../../../../../../rfcs/archive/rfc-3a-monoliths.md) (the decomposition-heuristic rewrite in [`plugins/change/skills/plan/briefs/omnia/propose.md`](../../../briefs/omnia/propose.md)).

| Path | Role |
| --- | --- |
| [`inputs/discovery.md`](inputs/discovery.md) | Starting-state `discovery.md` — byte-identical to the C22 monolith discovery fixture's `expected/discovery.md`. Drives the expected plan (`expected/plan.yaml`). |
| [`inputs/discovery-manifest.md`](inputs/discovery-manifest.md) | Same three capabilities as `discovery.md`, but `user-registration` is `confidence: low`. |
| [`expected/plan.yaml`](expected/plan.yaml) | Byte-stable plan when every slice is accepted without edit — all path hints and delta-targeting intent carried in `description`. |
| [`expected/create-invocations.md`](expected/create-invocations.md) | The three `specify plan add` commands. |
| [`expected/create-invocations-manifest.md`](expected/create-invocations-manifest.md) | Alternative invocation sequence (description-driven). |
| [`notes.md`](notes.md) | Mapping rationale + cross-references to C22. |

Sibling of the flat-layout platform-v2 fixture ([`../discovery.md`](../discovery.md), [`../expected-plan.yaml`](../expected-plan.yaml), [`../transcript.md`](../transcript.md)), which pins the interactive accept/edit/reject/abort loop on a five-slice multi-source run under the pre-C19 discovery shape. This fixture pins the decomposition rule on the post-C23 unified-YAML shape.
