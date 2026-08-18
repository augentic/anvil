# Emery Developer Guide

Emery is being rebuilt as a **spec generator** under the remediation programme ([ADR-0008](https://github.com/augentic/emery/blob/main/rfcs/decisions/0008-spec-generator-programme.md)). The v1 delivery engine — the `plan → refine → execute → finalize` workflow, the target-adapter build loop, and the definition loop — is frozen and archived at git tag `v1`:

```bash
git worktree add ../emery-v1 v1
```

This guide documents what ships **today**: the `emery` CLI's reduced surface (`init` plus the reserved `specify` stub), the source-adapter seam, and the contributor standards for the Rust workspace. Feature work is frozen until the spec walking skeleton is green; the plan of record is [`rfcs/remediation-plan.md`](https://github.com/augentic/emery/blob/main/rfcs/remediation-plan.md).

## Guide structure

- **[Reference](reference/index.md)** — the shipped CLI verbs and output shapes.
- **[Contributing](contributing/index.md)** — the Rust workspace, quality gates, and Cursor operator plugins.
- **Standards** — the durable engineering policy: [CLI contract](standards/cli-contract.md), [testing](standards/testing.md), [architecture](standards/architecture.md), [coding standards](standards/coding-standards.md), [Rust style](standards/style.md), [handler shape](standards/handler-shape.md), and [documentation authoring](standards/doc-authoring.md).
