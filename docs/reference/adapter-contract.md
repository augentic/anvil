# Adapter contract

The precise authoring contract for source and target adapters: identity and metadata, the per-axis operation signatures, the sandbox posture, resolver mechanics, and the author checklist. For the conceptual picture — the two roles, the shared component shape, authority resolution — see [Anatomy of an adapter](../explanation/adapter-anatomy.md).

## Identity and metadata

There is no manifest file. Identity and metadata split:

- **Identity** — the `(name, version)` pair. The kebab-case `name` is unique per axis; the exact-semver `version` is the guest crate's `Cargo.toml` version, published as `emery:<name>@<semver>` (`wkg publish`). Resolution keys on the identity, and synthesized refs render `name@<semver>`.
- **Metadata** — the WIT `metadata` record returned by the component's deterministic `metadata` export: an optional `emery` host-CLI compatibility floor (an exact-semver minimum platform version, enforced at resolve time and aborting with `adapter-cli-too-old` on exit 3 when the running binary is older; absent means no floor) plus, for targets, the optional `inputs[]` and `platforms` capability. The host dispatches `metadata` at resolve time and caches the answer against the component's digest.

The operation set is **not** declared on the wire — it derives from the closed WIT contract (`wit/emery.wit`) per axis: sources expose `survey` / `extract`, targets expose `guidance` / `build` / `merge`. Each operation's prompt body and any deterministic helper behaviour ship compiled into the component; the prompt markdown stays authored under `prose/prompts/` in the adapter's guest crate. Path-based `detect[]` auto-detection is deferred — operators bind sources explicitly (`source legacy=typescript:./repo`).

## Source adapter contract

A source adapter participates in two places in the lifecycle.

**`survey(Source) → Lead[]`** runs inside the guest-routed `emery plan author` (and standalone `emery source survey`). It reads the operator-bound source path or value and emits one block per slice-sized **raw lead** under `## Lead inventory` in `discovery.md`. Each block carries a stable `lead` and the scalar `source` that surfaced it; identity is the `(source, lead)` pair. Re-surveying the same source replaces that source's blocks by `(source, lead)` and never merges across sources — cross-source unification is the reconcile leg inside `emery plan author`. The lead grammar:

```markdown
### legacy-monolith:user-registration

- lead: user-registration
- source: legacy-monolith
- synopsis: Registration endpoint accepting email + password with email-format validation.
```

**`extract(Lead, Source) → Evidence`** runs inside the guest-routed `emery slice refine` (and standalone `emery source extract`). It returns a structured document the CLI persists to `.emery/slices/<slice>/evidence/<source>.yaml`:

```yaml
authority: behaviour
lead: user-registration
claims:
  - kind: excerpt
    id: users.register.email-validation
    path: src/users/register.ts#L12-L87
```

Claims have a closed `kind` enum (`intent`, `requirement`, `criterion`, `decision`, `section`, `diagram`, `contract`, `excerpt`, `type`, `call`, `region`, `container`, `leaf`); new kinds require an RFC update. Top-level `authority:` is required per `Evidence`. The document's `(slice, source)` identity is path-borne (slice directory + `<source>.yaml` filename) and its adapter resolves from `plan.yaml.sources.<source>.adapter`, so neither is written in-document. `id` is required on `requirement` and `criterion` for deterministic reconciliation. Claim `path:` carries an optional GitHub-style anchor (`<path>`, `<path>#L<n>`, or `<path>#L<start>-L<end>`).

## Sandboxing

Source adapter operations run under the WASI Preview 2 posture: Wasm modules with directory preopens, no inherited host environment, no runtime network access, fixed working directory. The host pre-opens four runtime roots per call:

| Root              | Mode       | Contents                                                                                                                                                                                                                                                                                                          |
| ----------------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `$SOURCE_DIR`     | read-only  | The operator-bound source path; absent for `value:`-style bindings.                                                                                                                                                                                                                                               |
| `$CAPABILITY_DIR` | read-only  | The adapter's capability shelf (out-of-tree, when present) — reference material distributed beside the component.                                                                                                                                                                                                 |
| `$SCRATCH_DIR`    | write-only | Per-operation scratch lane under the transient working-state root, structurally outside the cache tree: `extract` → `.emery/scratch/<adapter>/<slice>/`; `survey` (plan-time, no slice) → `.emery/scratch/<adapter>/survey/`. Recreated empty at `prepare` time — only what this run writes can be finalized. |
| `$PROJECT_DIR`    | none       | Source adapters do not get the project root; lifecycle state stays off-limits.                                                                                                                                                                                                                                    |

Access outside these roots is denied. Symlinks are resolved during canonicalization; a symlink inside `$SOURCE_DIR` pointing outside it is denied even if its textual path looks contained. A denied access surfaces as structured error `source-extract-path-denied` (or `source-survey-path-denied`) and the slice stays `refining`. Resolution paths: rebind the source via `emery plan amend` to include the needed root, or drop the source.

Each source operation runs as one guest orchestration: it builds the sandbox above, scaffolds the output target, emits `source.execution.agent`, drives the adapter guest's compiled-in prompt against the prepared directory, then validates the output before it becomes visible (lead set / Evidence schema), merges it into `discovery.md` (`survey`) or persists `evidence/<source>.yaml` (`extract`), and journals the completion event. Results are never cached — each run re-executes the prompt.

## Target adapter contract

Target adapters do not own `spec.md` or `design.md` synthesis. They contribute three operations:

- **`guidance`** — idiom guidance consumed by core synthesis. The prompt shapes how `proposal.md` / `spec.md` / `design.md` / `tasks.md` are written for slices that target this adapter. Empty `guidance` is valid; the prompt body is read into synthesis context, not executed.
- **`build`** — implementation drive: consume **only** the build request's `inputs` manifest (the rendered `proposal.md` / `spec.md` / `design.md` / `tasks.md` plus the adapter's declared `inputs[]`), write code (and any target-specific structured manifests like Vectis `composition.yaml`), run target-local validation, and write the build report to `build/report.yaml`. The operation receives a `Workspace` handle — a private, disposable materialization of the frozen base snapshot (RFC-87): product code is written under `workspace.root`, slice artifacts resolve read-only under `workspace.artifacts`, and the engine captures the result as a code patch after the report validates. `emery slice build` owns base freezing, workspace prepare/capture/discard, request assembly, report validation, and the `built` transition gate; the target's build prompts own only code generation.
- **`merge`** — phased landing gate: requires lifecycle `built` and is dispatched twice per merge by the closed WIT `merge-phase` enum, each phase over a read-only prepared view of the captured result snapshot. **Preflight** runs the target's staged checks before the engine's deterministic commit (a blocking finding aborts with the slice still `built`); **postflight** re-runs the target's validators over the merged baseline (a blocking finding is a terminal diagnostic — the merge stands). Each gate returns a report in the build-report shape, schema-gated and persisted by `emery slice merge`; the gate never performs lifecycle transitions, baseline spec merging, or archive moves.

A target adapter MAY declare an optional `inputs[]` in its `metadata` answer — a flat list of `{ path, required }` entries naming the target-specific build inputs `build` consumes (e.g. Vectis `tokens.yaml` / `assets.yaml` / `components.yaml` or the contracts `contracts/` subtree). Paths are relative to the build request's `inputs.root` (the slice tree); the CLI resolves them into the request's `inputs.artifacts.additional[]`, and a missing `required` path aborts the build with `target-build-input-missing`. v1 keeps the declaration a flat path list — globs and conditional inputs are deferred. See the [target adapter reference](targets/index.md#identity-and-metadata) and [`emery slice build`](cli/slice.md#emery-slice-build).

Target-specific structured outputs are produced by `build` alongside the code they accompany; they are not Emery artifacts and do not need a fourth capability. Each slice binds a `project` in `plan.yaml`; the target adapter is resolved on demand from that project (it is not stored per slice). v1 supports one target per project.

## Resolver and cache

<div class="pipeline">

![Source and target adapter axes](../assets/diagrams/adapter-anatomy/adapter-axes.svg)

<p class="pipeline-caption">Sources survey/extract into evidence; core synthesis reads target guidance; target build/merge lands code.</p>
</div>

The adapter resolver (`crates/project/src/adapter/`) routes by binding axis. There is no `if name == "intent"` branch in core — the first-party adapters are published components (`emery:intent`, `emery:documentation`, `emery:typescript`, `emery:screenshots`, `emery:captures`, `emery:omnia`, `emery:vectis`, `emery:contracts`) that resolve through the same code path as a third-party adapter: a pinned identity resolves the global single-file store entry (`<store-root>/<name>@<version>.wasm`, verify-on-read), a bare name resolves local-first — the seeded project component cache (`<project-cache>/components/<name>.wasm`, populated by `emery adapter add` or a local component at init), else the newest installed store version, else pull-latest provisioning; there is no sibling-checkout or build-tree probe.

CLI entry points: `emery source resolve <name>` and `emery target resolve <value>` locate the component and report its resolved path, location, and version. `emery plan add`, `emery plan amend <entry> --add-source / --remove-source`, and the reconcile leg inside `emery plan author` write slice bindings into `plan.yaml`.

## Authoring checklist

1. **Pick the axis.** Source if your adapter reads external material and writes `Evidence`; target if your adapter consumes `spec.md` + `design.md` and writes code.
2. **Create the guest crate.** `sources/<name>/` or `targets/<name>/` in the adapters repo: a `Cargo.toml` (its `version` is the adapter identity) and a `prose/prompts/` subdirectory (plus `prose/references/` and, where needed, `prose/rules/`).
3. **Implement the operations.** The operation set is closed per axis by the WIT contract — sources implement `survey` / `extract`, targets implement `guidance` / `build` / `merge`.
4. **Write the prompts.** Each operation prompt is a markdown file compiled into the adapter guest. Source `survey` writes `discovery.md` blocks; source `extract` returns `Evidence` content; target `guidance` is idiom guidance read into synthesis context; target `build` and `merge` drive code generation and landing.
5. **Ship helper behaviour in the guest.** Deterministic helper behaviour is in-guest library code compiled into the adapter's component; there is no separate extension declaration.
6. **Validate.** Build with `cargo make release` in the adapters repo, then `emery source resolve <name>` / `emery target resolve <name>` exercises resolution and the metadata dispatch.

## See also

- [Anatomy of an adapter](../explanation/adapter-anatomy.md) — the conceptual half: roles, shared shape, authority resolution
- [Source adapters](sources/index.md) / [Target adapters](targets/index.md) — first-party catalogs and metadata tables
- [emery adapter](cli/adapter.md) — `add`, `upgrade`, and the resolve envelope
