# Legacy migration at scale

Bring existing codebases into Specify through source adapters rather than rewriting intent by hand. This page orients you to the migration path; it is not a full fixture walk-through.

## Prerequisites

- Completed [Quick start](quick-start.md)
- A legacy codebase suitable for the `code-typescript` source adapter (TypeScript today; language siblings follow the same pattern)

## How legacy code enters Specify

At plan time, bind a code source alongside or instead of documentation:

```text
/spec:plan legacy-migration source legacy=code-typescript:./vendor/monolith
```

The source adapter's `survey` operation scans the bound tree and emits slice-sized **leads** into `discovery.md`. At slice time, `extract` produces **Evidence** YAML that core synthesis reconciles into `spec.md`.

Multi-slice migrations look like any other multi-slice plan: one operator review step (Gate 1), then `/spec:execute` drives each slice through refine → build → merge.

## Capture and replay

For regression-ready migrations, use the Capture plugin to wiretap a legacy service and record fixtures:

- **`wiretapper`** — add capture code to a legacy TypeScript repo
- **`replay-writer`** — fold captured fixtures into tests on the generated Omnia crate

See the RT plugin skills and [Anatomy of an adapter](../explanation/adapter-anatomy.md) for the source/target contract.

## Recommended reading order

1. [Bind multiple sources](../how-to/bind-multiple-sources.md) — combine legacy code with design notes at plan time
2. [Your first multi-slice change](first-change.md) — multi-slice execute rhythm
3. [Anatomy of an adapter](../explanation/adapter-anatomy.md) — survey vs extract operations
4. [Target adapters](../reference/targets/omnia.md) — Omnia build and merge briefs

## Acceptance scenarios

The `lifecycle` acceptance pack includes a code-multi-slice scenario under `acceptance/lifecycle/04-code-multi-slice/` for operators validating releases. Contributors running acceptance tests should see [Acceptance tests](../contributing/acceptance.md).

## Next steps

- [Quick reference card](../reference/quick-reference.md) — source binding grammar
- [Resolve spec conflicts](../how-to/resolve-spec-conflicts.md) — when legacy and docs disagree
