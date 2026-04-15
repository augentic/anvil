---
id: merge
description: Merge the change into the repository
needs: [build]
---

Before merging, confirm all task checkboxes in `tasks.md` are complete and the
change status is `complete`. Consider running `/spec:verify` to check that
the implemented code matches the specs. The merge skill handles delta-spec
merging via `merge-specs.py` and runs a baseline coherence check afterward.
