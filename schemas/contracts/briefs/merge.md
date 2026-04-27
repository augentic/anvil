---
id: merge
description: Merge the contract change into the baseline
needs: [build]
---

Before merging, confirm all task checkboxes in `tasks.md` are complete and the change status is `complete`. The merge skill delegates delta-spec merging and baseline coherence validation to the `specify` CLI (`specify spec preview`, `specify spec conflict-check`, `specify merge`, `specify validate`).
