---
id: merge
description: Merge the change into the repository
needs: [build]
---

Before merging, confirm all task checkboxes in `tasks.md` are complete and the change status is `complete`. The merge skill delegates delta-spec merging and baseline coherence validation to the `specify` CLI (`specify slice merge preview`, `specify slice merge conflict-check`, `specify slice merge run`, `specify slice validate`).

The `specify slice merge run` command merges both spec deltas (markdown) and composition deltas (YAML) in a single operation. Review the composition delta alongside spec changes in the `specify slice merge preview` output before confirming the merge.
