# Troubleshooting

This section catalogs common failure modes and how to recover from them.

## Quick lookup

| Symptom | Page |
|---------|------|
| Slice not found | [Slice lifecycle issues](slice-lifecycle.md) |
| Slice not in expected state | [Slice lifecycle issues](slice-lifecycle.md) |
| Artifacts incomplete after define | [Slice lifecycle issues](slice-lifecycle.md) |
| Baseline conflict on merge | [Merge issues](merge.md) |
| Coherence failure after merge | [Merge issues](merge.md) |
| Plan execution lock held | [Plan and execution issues](plan-and-execution.md) |
| Self-heal on startup | [Plan and execution issues](plan-and-execution.md) |
| Workspace slot missing | [Plan and execution issues](plan-and-execution.md) |
| `origin-head-unresolved` | [Plan and execution issues](plan-and-execution.md) |
| Dirty workspace slot before execution | [Plan and execution issues](plan-and-execution.md) |
| Execution stuck | [Plan and execution issues](plan-and-execution.md) |
| Registry amendment required | [Plan and execution issues](plan-and-execution.md) |
| Phase failure during execution | [Plan and execution issues](plan-and-execution.md) |
| `cycle-in-depends-on` | [Plan and execution issues](plan-and-execution.md) |
| `orphan-source-key` | [Plan and execution issues](plan-and-execution.md) |
| `stale-workspace-clone` | [Plan and execution issues](plan-and-execution.md) |
| `unreachable-entry` | [Plan and execution issues](plan-and-execution.md) |
| `$ref` resolution failures | [Contract issues](contracts.md) |
| Schema metadata incomplete | [Contract issues](contracts.md) |
| Binding completeness failures | [Contract issues](contracts.md) |
| Alignment warnings | [Contract issues](contracts.md) |
| Adapter resolution failure | [Init and adapter issues](init-and-adapters.md) |
| Cache stale after adapter update | [Init and adapter issues](init-and-adapters.md) |
| `hub-cannot-be-project` | [Hub and registry issues](hub-and-registry.md) |
| `description-missing-multi-repo` | [Hub and registry issues](hub-and-registry.md) |
| `no-branch` from `workspace push` | [Change landing issues](change-landing.md) |
| Dirty slot from `workspace push` or `change finalize` | [Change landing issues](change-landing.md) |
| `unmerged` from `change finalize` | [Change landing issues](change-landing.md) |
| `branch-pattern-mismatch` | [Change landing issues](change-landing.md) |
| `plan-not-found` from `change finalize` | [Change landing issues](change-landing.md) |
| Breaking findings from `specify compatibility check` | [Change landing issues](change-landing.md) |

## When in doubt

If your symptom is not listed, run `specify status` to confirm the current state of your project, then check the [Glossary](../../appendices/glossary.md) for any term you don't recognise.
