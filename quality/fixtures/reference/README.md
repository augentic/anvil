# Eval fixture status

Fixture classifications describe whether a tree is part of the current eval execution contract:

- **executable** — a checked-in runner or framework test consumes the fixture.
- **reference** — a documentation pin or worked example; no runner grades it.
- **obsolete** — retained as historical evidence for a removed surface; do not use it as the current contract.

No fixture family in this directory is currently executable. The live operator-driven contract is the scenario catalog plus committed run records.

| Family | Classification | Current role |
| --- | --- | --- |
| [`sources/`](sources/) | reference | Static source inputs and expected survey/extract shapes. |
| [`skills/refine/`](skills/refine/) | reference | Worked synthesis inputs and expected artifacts. |
| [`skills/build/`](skills/build/) | reference | Historical skill-body inputs and visible-output pins; current skills are thin CLI wrappers. |
| [`skills/merge/`](skills/merge/) | reference | Historical skill-body inputs and visible-output pins; current skills are thin CLI wrappers. |
| [`skills/execute/`](skills/execute/) | obsolete | Retained `/spec:execute` fixtures; the current execution surface is `specify plan execute`. |
| [`targets/omnia/`](targets/omnia/) | reference | Static Omnia output-shape examples. |
| [`targets/vectis/`](targets/vectis/) | reference | Static Vectis output-shape examples. |

Executable operator helpers live under [`quality/profiles/workflow/`](../../profiles/workflow/README.md), outside the fixture tree.
