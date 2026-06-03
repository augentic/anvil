# Baseline vs Delta

Cross-format rules for computing the minimal delta between a contract baseline and the slice under construction. This reference complements [`artifact-structure`](artifact-structure.md), which describes the directory layout. Here we document **how** the author paths reason about what to write.

The rules apply uniformly to OpenAPI HTTP bindings, AsyncAPI message bindings, and standalone JSON Schema documents — each format skill's `author.md` references this file from its delta-classification step.

## Locations

| Concept | Path | Lifetime |
|---|---|---|
| **Baseline** | `contracts/{schemas,http,messages}/` | Persists across changes; merged contracts only. |
| **Change-local delta** | `.specify/slices/<slice-name>/contracts/{schemas,http,messages}/` | Exists during the slice lifecycle; merged into the baseline at `specify slice merge run` or discarded at `specify slice drop`. |

The baseline is the source of truth for the platform's current contract surface. The slice-local delta is a **proposed modification**, pending review and merge. The delta directory contains **only the files this slice adds or replaces** — never a full copy of the baseline.

## The three authorship patterns

Every author run falls into one of three patterns. The classification depends on the baseline state and the brief's plan structure, not on the algorithm — the same delta-computation steps run in all three.

| Pattern | Baseline state | Most spec interactions are… |
|---|---|---|
| **Contract-first** | Rich. A preceding contract change populated root `contracts/`. | Already covered by the baseline. The delta is small or empty. |
| **Spec-first** | Empty. Single-repo, no external consumers. | New. The delta is the full contract set, derived from the slice's specs. |
| **Contract-given** | Imported. The baseline came from an external system via the matching `contracts.build` format importer. | Covered. The delta is non-empty only for extensions the slice introduces. |

The author algorithms produce the same artefact regardless of pattern. The pattern label is for the alignment report and the brief's transcript, not for branching logic.

## Computing the minimal delta

Each format skill's author follows the same three-step classification. The terminology is intentionally format-neutral — the format-skill author files supply the format-specific predicates (e.g. "matching `(path, method)`" for OpenAPI; "matching `$id`" for JSON Schema).

### 1. Already covered

The baseline already defines the interaction the spec describes. Verify alignment between the spec's requirements and the baseline shape:

- **Identity match.** The baseline element's identity (path+method, channel+operation, `$id`) matches the spec's reference.
- **Shape alignment.** Property names, types, status codes, message payloads — every claim the spec makes is present in the baseline.

If alignment fails, **record a warning** in the alignment report. The author **never overwrites the baseline silently** — surface the discrepancy and let a human resolve it. The author also produces **no output file** for covered interactions; they remain in the baseline unchanged.

### 2. New or modified

The spec describes interactions absent from the baseline, or asserts new claims about a baseline element. Add to the delta:

- **Wholly new elements.** New endpoints, channels, or types — write a new file under the appropriate subdirectory of `$SLICE_DIR/contracts/`.
- **Extensions to baseline elements.** New status codes on a baseline operation, new optional fields on a baseline schema. The delta file must contain **both the existing baseline content and the new additions** because merge is opaque file replacement (see §Opaque file replacement below).
- **Backwards-incompatible changes.** Newly-required fields, removed fields, narrowed types. Surface as warnings in the alignment report; the slice is permitted but the brief must flag it for human review.

### 3. Normalisation

The baseline file lacks Specify-required metadata (e.g. `$id` on a schema, `info.description` on an OpenAPI document). Propose a normalisation delta that adds the metadata **without changing the interface shape**. Surface the normalisation entries as a separate section of the alignment report; they are not behavioural changes.

## The "do not modify the baseline directly" rule

All output goes into `$SLICE_DIR/contracts/`. Never edit a file under root `contracts/` from inside a slice — even when the slice is producing a normalisation delta, the new file lives in the slice directory until merge.

Two reasons:

1. **Reviewability.** A reviewer needs to see exactly what a slice contributes to the contract surface. Mixing edits across the baseline and the slice directory makes the diff ambiguous.
2. **Mergeability.** `specify slice merge conflict-check` compares the slice's `defined-at` timestamp against the baseline files it intends to replace. Edits to the baseline outside this audit trail will be flagged as conflicts at merge time and may be silently lost.

The verifier flags every modification to a baseline file made by an author / importer run as a hard failure.

## Opaque file replacement

Contract files use **whole-file replacement semantics** at merge. Specify does not parse contract YAML to compute property-level deltas the way it does for spec files (which use the ADDED/MODIFIED/REMOVED format). Instead:

- The slice's `contracts/<subdir>/<file>.yaml` replaces the baseline's `contracts/<subdir>/<file>.yaml` byte-for-byte at merge time.
- Files **absent** from the slice's `contracts/` are left untouched in the baseline.
- **New files** (paths that do not exist in the baseline) are added.
- **Deletion is not expressible** through the slice-level directory. Removing a contract file from the baseline requires a manual baseline edit, which is rare and out of scope for the format author skills.

Two consequences for authors:

1. **When extending an existing file** (adding a path to `user-api.yaml`, adding a field to `user.yaml`, adding a channel to `order-events.yaml`), the delta file **must include every existing element** alongside the new ones. Omitting an existing element silently deletes it at merge.
2. **When modifying an existing file**, change only the keys the spec asserts. Do not reorder unrelated keys or re-format the file — opaque replacement means a re-ordered file looks like a wholesale rewrite to reviewers.

## Conflict detection

Two concurrent changes that both modify the same contract file conflict. `specify slice merge conflict-check` detects this by comparing the slice's `defined-at` timestamp against the baseline file's last-merged timestamp:

- **No conflict.** Baseline file unchanged since the slice was defined → merge proceeds.
- **Conflict.** Baseline file modified after the slice's `defined-at` → merge is blocked. Resolution: re-run the slice's define phase against the updated baseline (typically via `/spec:refine` resume), recompute the delta, and re-merge.

Conflicts are detected at file granularity, not at the property / path / channel level. Two changes that add disjoint paths to the same `user-api.yaml` will still conflict — Specify defers to the operator to merge them manually (the format authors run again with the second change rebased onto the post-first-merge baseline).

## See also

- [`artifact-structure`](artifact-structure.md) — directory layout, naming conventions, three-subdir rule.
- [`import-upgrade-policy`](import-upgrade-policy.md) — companion reference for the importer side; importer paths produce delta files via the same baseline-immutability rules.
- [`report-shape`](report-shape.md) — verifier output that surfaces baseline-vs-delta findings.
- [`cross-project-compatibility`](cross-project-compatibility.md) — cross-format vocabulary used when the delta touches a contract a downstream project consumes.
