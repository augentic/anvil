# Standards layer (runtime excerpt)

Specify separates workflow, artifacts, and engineering standards. Workflow **mutates** `.specify/` state through CLI verbs. Artifacts **record** product intent. Engineering standards **constrain** generated and hand-written code via rules under `codex/rules/` and per-adapter `prose/rules/` overlays, embedded in each target adapter's component and applied by its build review prompts.

Standards enforcement is **not** a workflow phase: findings may block a pipeline but never call `specify slice transition` or write lifecycle fields. Plan **Gate 1** (`specify plan transition <name> approved`) is operator approval of a plan, not engineering-standards enforcement.

Full triad and enforcement tables: [Workflow, standards, and artifacts](https://specify.augentic.io/explanation/standards-layer.html).
