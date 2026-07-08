# Standards layer (runtime excerpt)

Specify separates workflow, artifacts, and engineering standards. Workflow **mutates** `.specify/` state through CLI verbs. Artifacts **record** product intent. Engineering standards **constrain** generated and hand-written code via rules under `codex/rules/` and per-adapter `prose/rules/` overlays, resolved by `specify rules export`.

`specify rules export` is **not** a workflow phase. It is a read-only projection: engineering standards reach consumer projects as exported rules enforced by each project's own CI; the export never calls `specify slice transition` or writes lifecycle fields. Plan **Gate 1** (`specify plan transition <name> approved`) is operator approval of a plan, not engineering-standards enforcement.

Full triad and enforcement tables: [Workflow, standards, and artifacts](https://specify.augentic.io/explanation/standards-layer.html).
