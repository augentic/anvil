# Adapter contract

The precise authoring contract for source and target adapters: identity and metadata, the per-axis operation signatures, the sandbox posture, resolver mechanics, and the author checklist. For the conceptual picture — the two roles, the shared component shape, authority resolution — see [Anatomy of an adapter](../explanation/adapter-anatomy.md).

## Identity and metadata

There is no manifest file. Identity and metadata split:

- **Identity** — the `(name, version)` pair. The kebab-case `name` is unique per axis; the exact-semver `version` is the guest crate's `Cargo.toml` version, published as `emery:<name>@<semver>` (`wkg publish`). Resolution keys on the identity, and synthesized refs render `name@<semver>`.
- **Metadata** — the WIT `metadata` record returned by the component's deterministic `metadata` export: an optional `emery` host-CLI compatibility floor (an exact-semver minimum platform version, enforced at resolve time and aborting with `adapter-cli-too-old` on exit 3 when the running binary is older; absent means no floor) plus, for targets, the optional `inputs[]`, the `writable-artifacts[]` grants, and the `platforms` capability. The host dispatches `metadata` at resolve time and caches the answer against the component's digest.

The operation set is **not** declared on the wire — it derives from the closed WIT contract (`wit/emery.wit`) per axis: sources expose `survey` / `extract`, targets expose `guidance` / `build` / `verify` / `repair` / `review` / `merge`. Each operation's prompt body and any deterministic helper behaviour ship compiled into the component; the prompt markdown stays authored under `prose/prompts/` in the adapter's guest crate. Path-based `detect[]` auto-detection is deferred — operators bind sources explicitly (`source legacy=typescript:./repo`).

## Source adapter contract

A source adapter participates in two places in the lifecycle.

**`survey(Source) → Lead[]`** runs inside the guest-routed `emery plan author` (and standalone `emery source survey`). It reads the operator-bound source path or value and emits one block per slice-sized **raw lead** under `## Lead inventory` in `discovery.md`. Each block carries a stable `lead` and the scalar `source` that surfaced it; identity is the `(source, lead)` pair. Re-surveying the same source replaces that source's blocks by `(source, lead)` and never merges across sources — cross-source unification is the reconcile leg inside `emery plan author`. The lead grammar:

```markdown
### legacy-monolith:user-registration

- lead: user-registration
- source: legacy-monolith
- synopsis: Registration endpoint accepting email + password with email-format validation.
```

**`extract(Lead, Source) → Evidence`** runs inside the `emery plan refine` drain (and standalone `emery source extract`). It returns a structured document the CLI persists to `.emery/slices/<slice>/evidence/<source>.yaml`:

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

Target adapters do not own `spec.md` or `design.md` synthesis. They contribute six operations — `guidance` and `merge` around the four build-loop operations (`build`, `verify`, `repair`, `review`) that the engine's build phase machine dispatches:

- **`guidance`** — idiom guidance consumed by core synthesis. The prompt shapes how `proposal.md` / `spec.md` / `design.md` / `tasks.md` are written for slices that target this adapter. Empty `guidance` is valid; the prompt body is read into synthesis context, not executed.
- **`build`** — generation only: consume **only** the build request's `inputs` manifest (the rendered `proposal.md` / `spec.md` / `design.md` / `tasks.md` plus the adapter's declared `inputs[]`), then run the target-specific preparation, scaffolding, writer ordering, and capture replay needed to produce the candidate — and nothing more. It must not verify, repair, or run standards remediation, and generation may not run a verification or repair loop. `build` alone owns output declaration and UI-surface classification: its `outputs` and `ui-surface` become the candidate values for the final report.
- **`verify`** — one model-assisted check pass on the lent workspace, returning findings. An adapter may also include deterministic in-guest validators and report the phase as `hybrid`, or `deterministic` when no model leg ran. `verify` receives only the candidate workspace — no slice identity — and cannot mutate the continuation.
- **`repair`** — one findings-directed repair pass. It receives the engine-projected repair brief (the blocking findings in canonical order, first 16) plus its `repair-origin` (`verification | review`) naming which engine gate supplied them. A returned repair report never selects the next operation: after any completed or non-applicable repair, the engine dispatches `verify`.
- **`review`** — one engineering-standards review pass, reporting its findings.
- **`merge`** — phased landing gate: requires lifecycle `built` and is dispatched twice per merge by the closed WIT `merge-phase` enum, each phase over a read-only prepared view of the captured result snapshot. **Preflight** runs the target's staged checks before the engine's deterministic commit (a blocking finding aborts with the slice still `built`); **postflight** re-runs the target's validators over the merged baseline (a blocking finding is a terminal diagnostic — the merge stands). Each gate returns a report in the build-report shape, schema-gated and persisted by the merge phase; the gate never performs lifecycle transitions, baseline spec merging, or archive moves.

**One pass per dispatch, engine-owned loop.** Operation order, repair routing, budgets, terminal success, and terminal failure are deterministic engine policy: the engine drives `build → verify ⇄ repair → review ⇄ repair` with at most three verification repairs and one review remediation (engine constants — never adapter or model fields). A target adapter cannot select its next operation, reset a budget, silently retry, or claim completion while a blocking phase report remains; prompts may describe how to perform their single operation, never contain retry loops. An inapplicable operation returns a typed `not-applicable` report (no blocking findings, no writes) rather than inventing adapter-specific operation names.

**Typed phase reports.** Each build-loop dispatch returns one `phase-report { outcome, source, findings, outputs, ui-surface, written, next-continuation }` — the engine persists it, decides the next operation, and assembles the final build report itself (the adapter never writes `build/report.yaml`; the terminal report is the engine's deterministic projection of the build report plus the latest verification and review reports). `outcome` is `completed | not-applicable` — there is no adapter-selected `success | failure`; blocking findings and dispatch errors determine failure. The required `source` (`deterministic | model-assisted | hybrid`; `tool` is reserved on the wire but rejected until a trusted host-tool execution seam exists) is an assurance claim, not an execution selector, and must cover every finding source — a deterministic report cannot carry model-assisted findings, and a report mixing both must be `hybrid`. `repair`, `verify`, and `review` must return empty `outputs` and no `ui-surface`. These coherence rules are engine gates, not prompt conventions.

**Continuation.** `next-continuation` is an adapter-opaque byte payload (it may represent several writer or reviewer sessions) that the engine persists and echoes only to the same resolved target identity, attempt, and build workspace: `none` preserves the current value, an empty value clears it, and a non-empty value replaces it. The engine rejects a continuation larger than 1 MiB before persistence. `verify` cannot mutate it; `build`, `repair`, and `review` may return a replacement. It never crosses attempts, survives no workspace loss, and is never interpreted or treated as lifecycle authority.

**Workspace and artifact stage.** Every build-loop operation receives the same `Workspace` handle for the whole loop — one private, disposable materialization of the frozen base snapshot (RFC-87) plus one attempt-local **artifact stage** (`workspace.artifact-stage`), a writable mirror rooted at the candidate slice tree beside the read-only project-wide `artifacts` root. Product code goes under `workspace.root`; target-owned slice artifacts go under `workspace.artifact-stage.root`; the authoritative slice tree remains read-only to every target operation. Writes to the stage are admitted only under the target's declared `writable-artifacts[]` metadata grants (`{ path, kind: file | tree }` — a `file` grant names exactly one slice-relative file, a `tree` grant that directory and its descendants; `/`-separated relative paths, no glob or `..` grammar; e.g. Omnia declares `tasks.md`, Vectis `tasks.md` / `composition.yaml` / its build bookkeeping subtree, Contracts `tasks.md` / `contracts/`). The engine seeds the stage before `build`, derives the actual diff after every mutating phase, rejects changes outside the grants even when omitted from `written`, and promotes the staged diff all-or-none only after the terminal gates pass; every failure path discards both writable trees without touching authoritative state.

**Assurance boundary.** Verification remains model-assisted: the agent runs its declared commands inside the lent workspace, possibly including tests authored by the same build, so a green result means the candidate passes its own reported checks — self-consistency evidence, not an independent or protected oracle. Operator output names the terminal verification's `source` even on a clean pass. Deterministic native verification is a [deferred follow-on](../../rfcs/rfc-95-native-verification.md), not part of this contract.

A target adapter MAY declare an optional `inputs[]` in its `metadata` answer — a flat list of `{ path, required }` entries naming the target-specific build inputs `build` consumes (e.g. Vectis `tokens.yaml` / `assets.yaml` / `components.yaml` or the contracts `contracts/` subtree). Paths are relative to the build request's `inputs.root` (the slice tree); the CLI resolves them into the request's `inputs.artifacts.additional[]`, and a missing `required` path aborts the build with `target-build-input-missing`. v1 keeps the declaration a flat path list — globs and conditional inputs are deferred. See the [target adapter reference](targets/index.md#identity-and-metadata) and [`emery plan execute`](cli/plan.md#emery-plan-execute).

Target-specific structured outputs are produced by `build` alongside the code they accompany; they are not Emery artifacts and do not need a separate capability. Each slice binds a `project` in `plan.yaml`; the target adapter is resolved on demand from that project (it is not stored per slice). v1 supports one target per project.

## Resolver and cache

<div class="pipeline">

![Source and target adapter axes](../assets/diagrams/adapter-anatomy/adapter-axes.svg)

<p class="pipeline-caption">Sources survey/extract into evidence; core synthesis reads target guidance; the target build loop and merge land code.</p>
</div>

The adapter resolver (`crates/project/src/adapter/`) routes by binding axis. There is no `if name == "intent"` branch in core — the first-party adapters are published components (`emery:intent`, `emery:documentation`, `emery:typescript`, `emery:screenshots`, `emery:captures`, `emery:omnia`, `emery:vectis`, `emery:contracts`) that resolve through the same code path as a third-party adapter: a pinned identity resolves the global single-file store entry (`<store-root>/<name>@<version>.wasm`, verify-on-read), a bare name resolves local-first — the seeded project component cache (`<project-cache>/components/<name>.wasm`, populated by `emery adapter add` or a local component at init), else the newest installed store version, else pull-latest provisioning; there is no sibling-checkout or build-tree probe.

CLI entry points: `emery source resolve <name>` and `emery target resolve <value>` locate the component and report its resolved path, location, and version. `emery plan add`, `emery plan amend <entry> --add-source / --remove-source`, and the reconcile leg inside `emery plan author` write slice bindings into `plan.yaml`.

## Authoring checklist

1. **Pick the axis.** Source if your adapter reads external material and writes `Evidence`; target if your adapter consumes `spec.md` + `design.md` and writes code.
2. **Create the guest crate.** `sources/<name>/` or `targets/<name>/` in the adapters repo: a `Cargo.toml` (its `version` is the adapter identity) and a `prose/prompts/` subdirectory (plus `prose/references/` and, where needed, `prose/rules/`).
3. **Implement the operations.** The operation set is closed per axis by the WIT contract — sources implement `survey` / `extract`, targets implement `guidance` / `build` / `verify` / `repair` / `review` / `merge`.
4. **Write the prompts.** Each operation prompt is a markdown file compiled into the adapter guest, and performs exactly one pass. Source `survey` writes `discovery.md` blocks; source `extract` returns `Evidence` content; target `guidance` is idiom guidance read into synthesis context; target `build` / `verify` / `repair` / `review` drive one generation, check, repair, or standards-review pass each (no retry loops — the engine owns the loop); target `merge` gates the landing.
5. **Ship helper behaviour in the guest.** Deterministic helper behaviour is in-guest library code compiled into the adapter's component; there is no separate extension declaration.
6. **Validate.** Build with `cargo make release` in the adapters repo, then `emery source resolve <name>` / `emery target resolve <name>` exercises resolution and the metadata dispatch.

## See also

- [Anatomy of an adapter](../explanation/adapter-anatomy.md) — the conceptual half: roles, shared shape, authority resolution
- [Source adapters](sources/index.md) / [Target adapters](targets/index.md) — first-party catalogs and metadata tables
- [emery adapter](cli/adapter.md) — `add`, `upgrade`, and the resolve envelope
