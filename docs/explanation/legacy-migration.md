# Legacy migration at scale

How existing codebases enter Emery through source adapters rather than hand-rewritten intent. This page explains the migration path conceptually; for the hands-on counterpart, see [Migrate a legacy service](../tutorials/migrate-a-legacy-service.md).

## How legacy code enters Emery

At plan time, bind a code source alongside or instead of documentation:

```text
/emery:plan legacy-migration source legacy=typescript:./vendor/monolith
```

The source adapter's `survey` operation scans the bound tree and emits slice-sized **[leads](../appendices/glossary.md#l)** into `discovery.md`. At slice time, `extract` produces **Evidence** YAML that core synthesis reconciles into `spec.md`. The `typescript` adapter covers TypeScript today; language siblings follow the same pattern.

Multi-slice migrations look like any other multi-slice plan: review the topology, drain refinement with `emery plan refine`, review the specifications, then `emery plan execute` drives each slice through build → merge. Scale changes the plan row count, not the machinery.

## Why authority matters in migrations

Legacy code is rarely the whole truth — it encodes what the system *does*, not always what it *should* do. Emery expresses that with the authority hierarchy: evidence extracted from code carries `authority: behaviour`, the lowest class, so operator intent and written documentation win any disagreement automatically. When legacy behaviour and design notes conflict at the same authority, the requirement is tagged `[conflict]` for the operator to reconcile — see [Resolve spec conflicts](../how-to/resolve-spec-conflicts.md).

## Runtime captures

Runtime capture trees are consumed by the [`captures` source adapter](https://github.com/augentic/emery-adapters/tree/main/sources/captures) (default `authority: behaviour`). Operators produce those trees outside Emery; bind them at plan time like any other source. See the [Adapter contract](../reference/adapter-contract.md) for the source/target contract.

## Recommended reading order

1. [Migrate a legacy service](../tutorials/migrate-a-legacy-service.md) — bind a TypeScript codebase and drive it to Omnia, hands-on
2. [Your first multi-slice change](../tutorials/first-change.md) — the multi-slice execute rhythm
3. [Bind multiple sources](../how-to/bind-multiple-sources.md) — combine legacy code with design notes at plan time
4. [Anatomy of an adapter](adapter-anatomy.md) — survey vs extract operations
5. [Omnia target](../reference/targets/omnia.md) — build and merge briefs for the generated code

## See also

- [Quick reference card](../reference/quick-reference.md) — source binding grammar
- [Resolve spec conflicts](../how-to/resolve-spec-conflicts.md) — when legacy and docs disagree
