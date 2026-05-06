---
id: merge
description: Merge the change into the repository
needs: [build]
---

Before merging, confirm all task checkboxes in `tasks.md` are complete and the change status is `complete`. The merge skill delegates delta-spec merging and baseline coherence validation to the `specify` CLI (`specify change merge preview`, `specify change merge conflict-check`, `specify change merge run`, `specify change validate`).

The `specify change merge run` command merges both spec deltas (markdown) and composition deltas (YAML) in a single operation. The merge surface is broader than spec / design / task deltas: per RFC-11 §I "Merge handoff", `composition.yaml`, `tokens.yaml`, `assets.yaml`, and any referenced asset files under `design-system/assets/**` (or change-local `assets/`) are reviewable lifecycle artifacts when they appear in a change. `composition.yaml` continues to merge into the Specify baseline; token and asset updates merge into `design-system/tokens.yaml`, `design-system/assets.yaml`, and `design-system/assets/**` respectively. Review every UI input delta alongside the spec / design / task changes in the `specify change merge preview` output before confirming the merge so reviewers can understand which downstream shell generations will be affected.

After `specify change merge run` succeeds, re-run the deterministic UI input validator against the merged baseline:

```bash
specify vectis validate composition
```

The CLI honours the `artifacts:` block in `schemas/vectis/schema.yaml` to discover the now-merged `composition.yaml` (baseline path) and to auto-invoke `tokens` / `assets` modes against any sibling `tokens.yaml` / `assets.yaml`. Run this even when the current change did not generate any platform code, because later shell work may consume the merged baseline input set (RFC-11 §I "Merge handoff"). The same exit semantics apply: errors block merge finalisation, warnings flow into the operator-facing summary, clean runs are silent. When `composition.yaml` is absent from the merged baseline (no UI input set in the project), the validator exits cleanly without performing wired-mode checks.
