---
id: merge
description: Merge the change into the repository
needs: [build]
---

Before merging, confirm all task checkboxes in `tasks.md` are complete and the change status is `complete`. Consider running `/spec:verify` to check that the implemented code matches the specs. The merge skill delegates delta-spec merging and baseline coherence validation to the `specify` CLI (`specify spec preview`, `specify spec conflict-check`, `specify merge`, `specify validate`).

The `specify merge` command merges both spec deltas (markdown) and composition deltas (YAML) in a single operation. Review the composition delta alongside spec changes in the `specify spec preview` output before confirming the merge.
