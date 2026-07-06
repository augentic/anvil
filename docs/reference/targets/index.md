# Target Adapters

> Target adapters declare the output side of the source/target split (see [Anatomy of an adapter](../../explanation/adapter-anatomy.md) for the full contract). The first-party targets (`omnia`, `vectis`, `contracts`) are authored at [`targets/<name>/`](https://github.com/augentic/specify-adapters/tree/main/targets) in the adapters repo and published as `augentic:<name>@<semver>` components. The source-side counterparts are documented under [Source adapters](../sources/index.md).

## What is a target adapter?

A **target adapter** is the output role in the Specify plugin model. It describes how the core `refine → build → merge` slice loop produces an outcome domain's artefacts. Three operations:

- `guidance` — idiom guidance consumed by core synthesis. Read into context when `/spec:refine` writes `spec.md` / `design.md`. Empty `guidance` is valid.
- `build` — implementation drive: consume **only** the build request's `inputs` manifest (rendered `proposal.md` / `spec.md` / `design.md` / `tasks.md` plus the adapter's declared `inputs[]`), write code (and any target-specific structured manifests like Vectis `composition.yaml`), run target-local validation, and write the build report to `build/report.yaml`. Driven by `specify slice build` — see [`specify slice build`](../cli/slice.md#specify-slice-build).
- `merge` — landing gate: requires lifecycle `built`, re-runs the target's validators, surfaces conflicts, and drives verification commands. v1 adds **no** merge report — `specify slice merge` is the writer and `slice.merge.*` events fire on its validator outcome.

Target adapters do not own `spec.md` or `design.md` synthesis — that is **core**'s responsibility. The plan-level `Slice.target` field selects the target; v1 supports one target per project.

## Identity and metadata

There is no manifest file. Identity is the guest crate's `(name, version)` — the kebab-case package name and the exact-semver `Cargo.toml` version, published as `augentic:<name>@<semver>`. Metadata is the WIT `manifest` record returned by the component's deterministic `describe` export:

| Field           | Required | Meaning |
| --------------- | -------- | ------- |
| `specify-floor` | no       | Exact-semver minimum host-CLI version; resolve aborts with `adapter-cli-too-old` (exit 3) when the running binary is older. |
| `inputs`        | no       | Flat list of `{ path, required }` declaring the target-specific build inputs `build` consumes (e.g. Vectis `tokens.yaml` / `assets.yaml` / `components.yaml` or the contracts `contracts/` subtree). Paths are relative to the build request's `inputs.root` (the slice tree); the CLI resolves them into `inputs.artifacts.additional[]`. A missing `required` path aborts `specify slice build` with `target-build-input-missing`. v1 keeps the declaration a flat path list — globs and conditional inputs are deferred. Defaults to empty. |
| `platforms`     | no       | `{ required, allowed, default }` platforms capability; see [Anatomy of an adapter](../../explanation/adapter-anatomy.md). |

Deterministic helper behaviour is in-guest library code compiled into the adapter's component; there is no separate extension declaration or host-dispatched helper. See [Tool declarations](../../explanation/tool-declarations.md).

## How a target adapter participates in the loop

```text
/spec:refine  →  reads target.guidance   (idiom guidance for synthesis)
/spec:build   →  drives target.build     (code generation)
/spec:merge   →  drives target.merge     (validates and lands the slice)
```

Core synthesis writes the canonical artifacts (`proposal.md` / `spec.md` / `design.md` / `tasks.md`) in a fixed substep order regardless of target. The `guidance` prompt is read into context as idiom guidance but never replaces synthesis output. The operation set is not declared anywhere on the wire — it derives from the closed WIT contract (`wit/specify.wit`).

## Dependency direction

The dependency graph is one-way; `specify-core` never depends on the plugin loader's axis routing:

```text
specify (binary)
   └─ specify-workflow
        ├─ specify-tool
        └─ plugin loader (adapters/sources/ + adapters/targets/)
              └─ specify-error
```

The invariant: **adapter resolution is a downstream concern**. Core owns the slice loop; adapters supply prompts.

## Distribution

A target adapter ships as one published component carrying the prompts that implement domain behaviour. Imperative behaviour (provider configuration, file generation, format validation, drift detection) lives in the prompts and in-guest library code. There is no second plugin runtime hidden behind the component.

Shared material used by multiple adapters lives outside the adapter roots under `codex/`:

- **`codex/rules/universal/`** — shared **engineering standards** (`UNI-*`) at [`codex/rules/universal/`](../../../codex/rules/universal/); per-target overlays stay at `targets/<name>/prose/rules/` in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters). See [Standards layer](../../explanation/standards-layer.md).
- **`codex/references/replay/`** — shared build-time replay hook contract at [`codex/references/replay/` in specify-adapters](https://github.com/augentic/specify-adapters/tree/main/codex/references/replay); per-target runners stay at `targets/<name>/prose/prompts/build/replay.md` when implemented.

## Validation

The metadata shape is the WIT `manifest` record on the `target` interface (`wit/specify.wit`) — typed at the component boundary, so there is no wire schema to validate against. `specify target resolve <value>` locates the component and dispatches `describe` on first use.

## See also

- [Anatomy of an adapter](../../explanation/adapter-anatomy.md) — full source/target contract.
- [Registry](../registry.md) — workspace topology that routes slices to target projects.
- Per-target reference: [Omnia](omnia.md), [Vectis](vectis.md), [Contracts](contracts.md).
