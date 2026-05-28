# Review Notes — RFC Reference Retirement

This repository no longer treats implemented RFC files as maintained reference material. Current review and CI guidance should cite maintained docs, schemas, adapter references, command names, or code surfaces.

Open review focus for this cleanup:

1. Maintained prose must cite current docs, schemas, adapter references, command names, or code surfaces as normative authority.
2. Engineering standards references should point to `docs/explanation/standards-layer.md`, rule schemas, `specrun rules export`, `specrun lint`, and the `LintFinding` wire shape.
3. Workflow and synthesis references should point to `docs/reference/lifecycle.md`, `plugins/spec/references/synthesis/`, `docs/contributing/acceptance.md`, and CLI schema or decision surfaces.

Run `make check` after edits to catch unresolved links, then run the three retirement searches requested by the cleanup plan.
