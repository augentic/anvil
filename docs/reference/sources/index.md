# Source Adapters

> Source adapters declare the input side of the source/target split (see [Anatomy of an adapter](../../explanation/adapter-anatomy.md) for the full contract). The first-party sources (`intent`, `documentation`, `typescript`, `screenshots`, `captures`) live at [`adapters/sources/<name>/adapter.yaml`](https://github.com/augentic/specify/tree/main/adapters/sources). The output-side counterparts are documented under [Target adapters](../targets/index.md).

## What is a source adapter?

A **source adapter** is the input role in the Specify plugin model. It reads external material — operator intent, written documentation, legacy code, screenshots, runtime captures — and turns it into structured `Evidence` that core synthesis can reconcile. Two operations:

- `survey` — plan-time. Reads the operator-bound source and emits one **lead** block per slice-sized unit of work under `## Lead inventory` in `discovery.md`. Runs inside `/spec:plan`.
- `extract` — slice-time. Reads one matched lead plus the bound source and returns an `Evidence` document the CLI persists to `.specify/slices/<slice>/evidence/<source>.yaml`. Runs inside `/spec:refine`.

Source adapters do not write `spec.md` — that is core synthesis's responsibility. A source supplies evidence; synthesis reconciles evidence from every bound source into one spec. See [From sources to slices](../../explanation/reconciliation.md) for the end-to-end flow.

## First-party source adapters

You bind sources per change at plan time (`/spec:plan <name> source docs=./design-notes source legacy=./repo`). Each source declares the **authority** its evidence carries, which decides who wins when two sources disagree (`intent` > `documentation` > `behaviour`).

| Adapter | Reads | Evidence authority | Typical use |
| ------- | ----- | ------------------ | ----------- |
| `intent` | An operator-supplied free-form string | `intent` | The degenerate N=1 entry point; backs every plan, including pure greenfield work. |
| `documentation` | A read-only directory of written docs | `documentation` | Design notes, specs, and operator-authored intent. |
| `typescript` | A read-only TypeScript/JavaScript source tree | `behaviour` | Reconstructing behaviour from a legacy service. |
| `screenshots` | A directory of screen images | `documentation` | Vision-assisted layout inference for UI targets (Vectis). |
| `captures` | A runtime capture tree (from `/capture:wiretapper`) | `behaviour` | Behaviour observed at runtime, anchored by replay digests. |

Authority is set on the **Evidence document** during `extract`, not in `adapter.yaml`; the table above lists the default each adapter's extract brief emits. Operators can override authority per slice at Gate 1 with `specify plan amend <entry> --authority-override`.

## Manifest shape

Every source adapter ships a single `adapter.yaml` at `adapters/sources/<name>/`:

```yaml
# yaml-language-server: $schema=https://github.com/augentic/specify-cli/raw/main/schemas/source.schema.json
name: typescript
version: 1
axis: source
execution: agent
description: TypeScript / JavaScript legacy-code source adapter.
briefs:
  survey: briefs/survey.md
  extract: briefs/extract.md
# optional: WASI helper tools the host caches alongside the manifest
tools:
  - name: replay-index
    version: 0.1.0
```

| Field | Required | Meaning |
| ----- | -------- | ------- |
| `name` | yes | Kebab-case source identifier. Must match the directory name under `adapters/sources/` and be unique across both axes. |
| `version` | yes | Integer ≥ 1. Increments when the adapter ships breaking changes. |
| `axis` | yes | Must be `source`. |
| `execution` | yes | Closed mode (`agent` \| `tool`). `agent` forces `cache: opt-out` and runs the brief via an agent; all first-party sources declare `agent`. |
| `description` | yes | Single-sentence summary of what the source reads and emits. |
| `briefs` | yes | Map of operation → brief markdown path relative to the manifest. The keys are the operation set, closed to `survey` and `extract` by `source.schema.json`. |
| `tools` | no | WASI helpers the host caches under the per-axis manifest cache at `.specify/.cache/manifests/sources/<name>/`. See [Tool declarations](../../explanation/tool-declarations.md). |

## How a source adapter participates in the loop

```text
/spec:plan    →  runs source.survey    (emits leads into discovery.md)
/spec:refine  →  runs source.extract   (emits evidence/<source>.yaml)
```

Both operations run sandboxed under the WASI Preview 2 posture — directory preopens only, no inherited host environment, no network. The host gives `survey` and `extract` read-only access to the bound source path and a write-only scratch directory, and denies access to the project root. See [Sandboxing](../../explanation/adapter-anatomy.md#sandboxing) for the preopened roots and the `prepare` / `finalize` two-phase agent dispatch.

## Validation

The wire-level schema is `schemas/source.schema.json` (distributed with the binary). It enforces the field set and the closed `[survey, extract]` operation list. `specify source resolve <name>` loads and validates the manifest on first use; `specify source survey` / `specify source extract` run the bound operation under the declared `execution` mode.

## See also

- [Anatomy of an adapter](../../explanation/adapter-anatomy.md) — full source/target contract, claim kinds, and sandboxing.
- [From sources to slices](../../explanation/reconciliation.md) — how leads and evidence become slices and specs.
- [Target adapters](../targets/index.md) — the output-side counterpart.
- [Bind multiple sources](../../how-to/bind-multiple-sources.md) — reconcile legacy code and docs at plan time.
