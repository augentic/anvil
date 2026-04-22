# Monolith propose fixture

Pins the **post-C24** 1:1 capability → slice mapping the propose
brief applies to a unified-YAML `discovery.md`. Consumes the C22
fixture's discovery output verbatim and emits a three-entry
`plan.yaml` with `scope.monolith.include` pre-filled from each
capability's `sources:` list.

**C27 (Stage C)** adds a second pinned shape beside the original
glob plan: when `user-registration` is `confidence: low` and still
shares `src/auth/verify.ts` with `email-verification`, the brief
emits `expected/plan-manifest.yaml` plus
`expected/slices/user-registration.yaml` (`scope.monolith.manifest`
→ `.specify/plans/traffic/slices/user-registration.yaml`) while the
two leaf slices remain glob-based in the same authoring run.

This fixture is the acceptance target for
[RFC-3a C24](../../../../../../../rfcs/rfc-3a-plan.md) (the
decomposition-heuristic rewrite in
[`schemas/omnia/briefs/plan/propose.md`](../../../../../../../schemas/omnia/briefs/plan/propose.md))
and [RFC-3a C27](../../../../../../../rfcs/rfc-3a-plan.md) (manifest
escape hatch for low-confidence tangled scopes).

| Path | Role |
| --- | --- |
| [`inputs/discovery.md`](inputs/discovery.md) | Starting-state `discovery.md` — byte-identical to the C22 monolith discovery fixture's `expected/discovery.md`. Drives the **glob** expected plan (`expected/plan.yaml`). |
| [`inputs/discovery-manifest.md`](inputs/discovery-manifest.md) | Same three capabilities as `discovery.md`, but `user-registration` is `confidence: low`. Drives the **manifest** expected plan (`expected/plan-manifest.yaml`) per Stage C in the propose brief. |
| [`expected/plan.yaml`](expected/plan.yaml) | Byte-stable plan when every slice is accepted without edit — all `scope.monolith.include` lists (overlap on `verify.ts` remains a `scope-overlap` warning at validate time). |
| [`expected/plan-manifest.yaml`](expected/plan-manifest.yaml) | Alternate pinned plan: `user-registration` uses `scope.monolith.manifest` pointing at `.specify/plans/traffic/slices/user-registration.yaml`. |
| [`expected/slices/user-registration.yaml`](expected/slices/user-registration.yaml) | v1 slice manifest (`version: 1` + `include:`) pinned for that entry; paths relative to `sources.monolith`. |
| [`expected/create-invocations.md`](expected/create-invocations.md) | The three `specify initiative create` commands for the glob plan. |
| [`expected/create-invocations-manifest.md`](expected/create-invocations-manifest.md) | The mixed invocation sequence (`--scope-include` on leaves, `--scope-manifest` on `user-registration`). |
| [`notes.md`](notes.md) | Mapping rationale + cross-references to C22 and C27. |

Sibling of the flat-layout platform-v2 fixture
([`../discovery.md`](../discovery.md),
[`../expected-plan.yaml`](../expected-plan.yaml),
[`../transcript.md`](../transcript.md)), which pins the interactive
accept/edit/reject/abort loop on a five-slice multi-source run
under the pre-C19 discovery shape. This fixture pins the
decomposition rule on the post-C23 unified-YAML shape.
