# Source Adapters

> Source adapters declare the input side of the source/target split (see [Anatomy of an adapter](../../explanation/adapter-anatomy.md) for the full contract). The first-party sources (`intent`, `documentation`, `typescript`, `screenshots`, `captures`) are authored at [`sources/<name>/`](https://github.com/augentic/specify-adapters/tree/main/sources) in the adapters repo and published as `augentic:<name>@<semver>` components. The output-side counterparts are documented under [Target adapters](../targets/index.md).

## What is a source adapter?

A **source adapter** is the input role in the Specify plugin model. It reads external material — operator intent, written documentation, legacy code, screenshots, runtime captures — and turns it into structured `Evidence` that core synthesis can reconcile. Two operations:

- `survey` — plan-time. Reads the operator-bound source and emits one **lead** block per slice-sized unit of work under `## Lead inventory` in `discovery.md`. Runs inside `/spec:plan`.
- `extract` — slice-time. Reads one matched lead plus the bound source and returns an `Evidence` document the CLI persists to `.specify/slices/<slice>/evidence/<source>.yaml`. Runs inside `/spec:refine`.

Source adapters do not write `spec.md` — that is core synthesis's responsibility. A source supplies evidence; synthesis reconciles evidence from every bound source into one spec. See [From sources to slices](../../explanation/reconciliation.md) for the end-to-end flow.

## First-party source adapters

You bind sources per change at plan time (`/spec:plan <name> source docs=./design-notes source legacy=./repo`). Each source declares the **authority** its evidence carries, which decides who wins when two sources disagree (`intent` > `documentation` > `behaviour` — canonical: [Authority hierarchy](../../../plugins/spec/references/synthesis/authority.md)).

| Adapter | Reads | Evidence authority | Typical use |
| ------- | ----- | ------------------ | ----------- |
| `intent` | An operator-supplied free-form string | `intent` | The degenerate N=1 entry point; backs every plan, including pure greenfield work. |
| `documentation` | A read-only directory of written docs | `documentation` | Design notes, specs, and operator-authored intent. |
| `typescript` | A read-only TypeScript/JavaScript source tree | `behaviour` | Reconstructing behaviour from a legacy service. |
| `screenshots` | A directory of screen images | `documentation` | Vision-assisted layout inference for UI targets (Vectis). |
| `captures` | A runtime capture tree (from `/capture:wiretapper`) | `behaviour` | Behaviour observed at runtime, anchored by replay digests. |

Authority is set on the **Evidence document** during `extract`; the table above lists the default each adapter's extract prompt emits. Operators can override authority per slice at Gate 1 with `specify plan amend <entry> --authority-override`.

## Identity and metadata

There is no manifest file. Identity is the guest crate's `(name, version)` — the kebab-case package name (unique across both axes) and the exact-semver `Cargo.toml` version, published as `augentic:<name>@<semver>`. Metadata is the WIT `manifest` record returned by the component's deterministic `describe` export; for sources it carries only the optional `specify-floor` host-CLI compatibility floor.

The operation set is not declared anywhere on the wire — it derives from the closed WIT contract (`wit/specify.wit`: `survey`, `extract`), and the prompts are compiled into the adapter component.

## How a source adapter participates in the loop

```text
/spec:plan    →  runs source.survey    (emits leads into discovery.md)
/spec:refine  →  runs source.extract   (emits evidence/<source>.yaml)
```

Both operations run sandboxed under the WASI Preview 2 posture — directory preopens only, no inherited host environment, no network. The host gives `survey` and `extract` read-only access to the bound source path and a write-only scratch directory, and denies access to the project root. See [Sandboxing](../../explanation/adapter-anatomy.md#sandboxing) for the preopened roots and the guest orchestration that drives each operation.

## Validation

The metadata shape is the WIT `manifest` record on the `source` interface (`wit/specify.wit`) — typed at the component boundary, so there is no wire schema to validate against. `specify source resolve <name>` locates the component and dispatches `describe` on first use; `specify source survey` / `specify source extract` run the bound operation as one guest orchestration each.

## See also

- [Anatomy of an adapter](../../explanation/adapter-anatomy.md) — full source/target contract, claim kinds, and sandboxing.
- [From sources to slices](../../explanation/reconciliation.md) — how leads and evidence become slices and specs.
- [Target adapters](../targets/index.md) — the output-side counterpart.
- [Bind multiple sources](../../how-to/bind-multiple-sources.md) — reconcile legacy code and docs at plan time.
