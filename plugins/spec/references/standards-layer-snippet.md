# Standards layer (runtime excerpt)

Specify separates workflow, artifacts, and engineering standards. Workflow **mutates** `.specify/` state through CLI verbs. Artifacts **record** product intent. Engineering standards **constrain** generated and hand-written code via rules under `adapters/**/rules/`, resolved by `specrun rules export` and enforced by `specrun lint`.

`specrun lint` is **not** a workflow phase. It is CI-native **standards enforcement**: findings may block a pipeline (exit code `2`) but never call `specrun slice transition` or write lifecycle fields. Plan **Gate 1** (`specrun plan transition <name> approved`) is operator approval of a plan, not engineering-standards enforcement.

Full triad and enforcement tables: [Workflow, standards, and artifacts](https://specify.augentic.io/explanation/standards-layer.html).
