# Standards layer (runtime excerpt)

Specify separates workflow, artifacts, and engineering standards. Workflow **mutates** `.specify/` state through CLI verbs. Artifacts **record** product intent. Engineering standards **constrain** generated and hand-written code via rules under `adapters/**/rules/`, resolved by `specify rules export` and enforced by `specify lint project`.

`specify lint project` is **not** a workflow phase. It is CI-native **standards enforcement**: findings may block a pipeline (exit code `2`) but never call `specify slice transition` or write lifecycle fields. Plan **Gate 1** (`specify plan transition <name> approved`) is operator approval of a plan, not engineering-standards enforcement.

Full triad and enforcement tables: [Workflow, standards, and artifacts](https://specify.augentic.io/explanation/standards-layer.html).
