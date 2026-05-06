---
id: merge
description: Merge the change into the repository
needs: [build]
---

Before merging, confirm all task checkboxes in `tasks.md` are complete and the change status is `complete`. The merge skill delegates delta-spec merging and baseline coherence validation to the `specify` CLI (`specify slice merge preview`, `specify slice merge conflict-check`, `specify slice merge run`, `specify slice validate`).
