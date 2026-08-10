# Target Adapters

> Target adapters declare the output side of the source/target split (see the [Adapter contract](../adapter-contract.md) for the full authoring contract). The first-party targets (`omnia`, `vectis`, `contracts`) are authored at [`targets/<name>/`](https://github.com/augentic/emery-adapters/tree/main/targets) in the adapters repo and published as `emery:<name>@<semver>` components. The source-side counterparts are documented under [Source adapters](../sources/index.md).

## Operations

For what a target adapter *is* and how it fits a change, see [Understanding Emery](../../explanation/concepts.md) and [Anatomy of an adapter](../../explanation/adapter-anatomy.md). The contract facts — six operations:

- `guidance` — idiom guidance consumed by core synthesis. Read into context when the refine phase writes `spec.md` / `design.md`. Empty `guidance` is valid.
- `build` — generation only: consume **only** the build request's `inputs` manifest (rendered `proposal.md` / `spec.md` / `design.md` / `tasks.md` plus the adapter's declared `inputs[]`) and write code (and any target-specific structured manifests like Vectis `composition.yaml`) into the lent workspace, returning a typed phase report. It must not verify or repair — the engine assembles the terminal `build/report.yaml` itself. Driven by the build phase of `emery plan execute` — see [`emery plan execute`](../cli/plan.md#emery-plan-execute).
- `verify`, `repair`, `review` — the rest of the build loop: one model-assisted check pass, one findings-directed repair pass, one engineering-standards review pass. The engine's phase machine dispatches them one pass at a time (`build → verify ⇄ repair → review ⇄ repair`) under engine-owned budgets; each returns a typed phase report. See the [Adapter contract](../adapter-contract.md#target-adapter-contract).
- `merge` — landing gate: requires lifecycle `built`, re-runs the target's validators, surfaces conflicts, and drives verification commands. Dispatched twice per merge (preflight / postflight) — the merge phase is the writer and `slice.merge.*` events fire on its validator outcome.

Target adapters do not own `spec.md` or `design.md` synthesis — that is **core**'s responsibility. The plan-level `Slice.target` field selects the target; v1 supports one target per project.

## Identity and metadata

There is no manifest file. Identity is the guest crate's `(name, version)` — the kebab-case package name and the exact-semver `Cargo.toml` version, published as `emery:<name>@<semver>`. Metadata is the WIT `metadata` record returned by the component's deterministic `metadata` export:

| Field           | Required | Meaning |
| --------------- | -------- | ------- |
| `emery-floor` | no       | Exact-semver minimum host-CLI version; resolve aborts with `adapter-cli-too-old` (exit 3) when the running binary is older. |
| `inputs`        | no       | Flat list of `{ path, required }` declaring the target-specific build inputs `build` consumes (e.g. Vectis `tokens.yaml` / `assets.yaml` / `components.yaml` or the contracts `contracts/` subtree). Paths are relative to the build request's `inputs.root` (the slice tree); the CLI resolves them into `inputs.artifacts.additional[]`. A missing `required` path aborts the build phase with `target-build-input-missing`. v1 keeps the declaration a flat path list — globs and conditional inputs are deferred. Defaults to empty. |
| `writable-artifacts` | no | Typed `{ path, kind: file \| tree }` grants naming the only slice artifacts the build-loop operations may write through the attempt-local artifact stage (e.g. Omnia `tasks.md`; Vectis `tasks.md`, `composition.yaml`, and its build bookkeeping subtree; Contracts `tasks.md` and `contracts/`). Paths are slice-relative, `/`-separated, no glob or `..` grammar. The engine rejects staged changes outside the grants. Defaults to empty. |
| `platforms`     | no       | `{ required, allowed, default }` platforms capability; see the [Adapter contract](../adapter-contract.md#identity-and-metadata). |

Deterministic helper behaviour is in-guest library code compiled into the adapter's component; there is no separate extension declaration or host-dispatched helper.

## How a target adapter participates in the loop

```text
refine phase   →  reads target.guidance   (idiom guidance for synthesis)
build phase    →  drives target.build → verify ⇄ repair → review ⇄ repair
                  (the engine-owned build loop; one pass per dispatch)
merge phase    →  drives target.merge     (validates and lands the slice)
```

Core synthesis writes the canonical artifacts (`proposal.md` / `spec.md` / `design.md` / `tasks.md`) in a fixed substep order regardless of target. The `guidance` prompt is read into context as idiom guidance but never replaces synthesis output. The operation set is not declared anywhere on the wire — it derives from the closed WIT contract (`wit/emery.wit`).

## Dependency direction

The dependency graph is one-way; `emery-core` never depends on the plugin loader's axis routing:

```text
emery (binary)
   └─ workflow
        ├─ emery-tool
        └─ plugin loader (source + target adapter components)
              └─ error
```

The invariant: **adapter resolution is a downstream concern**. Core owns the slice loop; adapters supply prompts.

## Distribution

A target adapter ships as one published component carrying the prompts that implement domain behaviour. Imperative behaviour (provider configuration, file generation, format validation, drift detection) lives in the prompts and in-guest library code. There is no second plugin runtime hidden behind the component.

Shared material used by multiple adapters lives outside the adapter roots under `codex/`:

- **`codex/rules/universal/`** — shared **engineering standards** (`UNI-*`) at [`codex/rules/universal/`](https://github.com/augentic/emery-adapters/tree/main/codex/rules/universal) in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters); per-target overlays stay at `targets/<name>/prose/rules/` in the same repo. See [Standards layer](../../explanation/standards-layer.md).
- **`codex/references/replay/`** — shared build-time replay hook contract at [`codex/references/replay/` in emery-adapters](https://github.com/augentic/emery-adapters/tree/main/codex/references/replay); per-target runners stay at `targets/<name>/prose/prompts/build/replay.md` when implemented.

## Validation

The metadata shape is the WIT `metadata` record on the `target` interface (`wit/emery.wit`) — typed at the component boundary, so there is no wire schema to validate against. `emery target resolve <value>` locates the component and dispatches `metadata` on first use.

## See also

- [Adapter contract](../adapter-contract.md) — full source/target contract.
- Per-target reference: [Omnia](omnia.md), [Vectis](vectis.md), [Contracts](contracts.md).
