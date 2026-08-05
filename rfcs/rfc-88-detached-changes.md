# RFC-88: Detached Changes

> Status: Draft — step 3 of the platform-migration series ([platform.md](platform.md))
>
> Owns: the detached change home; discovery and immutable pinning of targets, sources, and adapters; and per-target execution over accepted CIDs.
>
> Builds on [RFC-86](rfc-86-change-facts.md)'s facts and digest-bound `plan.execute.started` and [RFC-87](rfc-87-working-trees.md)'s content-addressed trees and private workspaces. [RFC-89](rfc-89-publication-sets.md) publishes the results; [RFC-92](rfc-92-node-sync.md) transports them between nodes.
>
> Amends RFC-86 D1 (the in-place change home is `.emery/change/`, not all of `.emery/`) and RFC-87: location-backed sources use D2's read-only views, D4 and acceptance criterion 4 include the target repository's durable state in its tree, the tree identity is named a **CID** in plan and discovery artifacts (RFC-87's `SnapshotId` is that CID), and the interim `apply` is deleted.

## Intent

Decouple change coordination from product checkouts and permanent workspace infrastructure.

A change owns portable facts and artifacts, never product code or durable product state. Participating targets and independent source inputs are discovered and pinned as immutable values, then processed in disposable private workspaces. No permanent platform repository, committed registry, or operator-tended `workspace/<project>/` slots remain.

This gives migrate and ongoing change one multi-target pipeline that produces a final accepted CID for each touched target, carrying that repository's code and merged baseline together. New repositories are created on the forge by the operator (with `product:` so change-mode discovery can find them); Emery does not provision repositories in this cut. Publication remains [RFC-89](rfc-89-publication-sets.md)'s responsibility.

## Flow and terms

1. The operator runs detached `emery plan author` with an authoring scope (CLI args or `--scope`; see [Scope](#scope)). Change uses bounded discovery selector rows; migrate uses explicit sources and may also bound target candidates through discovery. Intent is optional inline source value.
2. Discovery pins exact repositories, CIDs, and adapter versions into `discovery.yaml`.
3. `plan author` copies that topology into `plan.yaml`, surveys sources, and authors slices. Source→target mapping rationale lives in design prose (`change.md` / `design.md`), not authoring scope.
4. `plan execute` appends `plan.execute.started` and runs slices in disposable private workspaces.
5. Each touched target ends with one accepted CID for RFC-89 to publish.

Nouns: **change home** (RFC-86 fact tree; no product code or durable product state); **target** (participating repo + initial CID + optional target-axis adapter); **source** (immutable CID or inline value); **CID** (`sha256:` + hex of RFC-87's tree manifest; wire field `cid`); **accepted target CID** (latest successful merge result for one target).

`sources` and `targets` share one row shape (`adapter`, `locator` or `value`, `cid`). Durable product state stays `.emery/project.yaml` inside each repository.

## Scope

The authoring scope is an invocation envelope, not a change artifact. Supply it as flags or with `--scope`; prefer the file when lists grow. After authoring, `discovery.yaml` and `plan.yaml` are authoritative. Source keys and adapter pins are discovery outputs (D5, D6).

When present, `discover` is a non-empty list of selector rows. Each row has exactly one required `target` bound — an exact repository URL, or a repository-host namespace (an org, group, or subgroup) ending in `/*`; nothing else globs — and optional `products` and `topics` filters. Rows union; filters within a row intersect. A namespace row is a bounded candidate search; an exact row skips the search and asserts one candidate directly, subject to the same verification. The CLI composes rows: `--target` appends a selector row (exact URL or namespace `/*`); repeated `--product` / `--topic` flags fill the most recent row's filters; `--source` appends to `sources`; `--intent` sets `intent`. Exact clap names are illustrative.

Scope cannot create targets or prescribe source→target mappings. Create a forge repository with `product:` so discovery can find it; record mapping rationale in `change.md` or `design.md`, then bind slices after discovery. There is no `--create` or `create:` surface.

```bash
# Change — bounded target discovery
emery plan author checkout-v2 \
  --target 'https://github.com/acme/*' \
  --product checkout \
  --intent "Raise checkout to the shared payment API"

# Same change scope from a file
emery plan author checkout-v2 --scope scope.yaml

# Migrate — explicit sources (small enough for args)
emery plan author orders-modernization \
  --source https://github.com/acme/legacy-orders@main \
  --target https://github.com/acme/orders-sdk
  --intent "Extract the orders bounded context into its own service"
```

```yaml
# scope.yaml — operator-owned; not written into the change home
intent: Raise checkout to the shared payment API
discover:
  - target: https://github.com/acme/*
    products: [checkout]
  - target: https://github.com/acme/payments-sdk
```

## Decisions

### D1 — The change home is the detached fact tree, separate from durable product state

Detached artifacts live at the change root, with no synthetic project configuration:

```text
<change>/
  change.md
  plan.yaml
  discovery.yaml              # pinned topology from discovery
  leads.md                    # source-adapter lead inventory
  events/<actor>.jsonl        # facts, including plan.execute.started
  slices/<slice>/...
```

`discovery.yaml` records the pinned targets and sources: CIDs and exact adapter package pins. Candidates that do not match are omitted; match failures surface as ephemeral diagnostics, not change artifacts.

`plan.yaml` copies that topology and adds slices.

`leads.md` contains source-adapter leads.

`change.md` and the documents under `slices/<slice>/` keep their existing formats. `leads.md` is the current `discovery.md` format under a clearer name; only `discovery.yaml` and the RFC-88 additions to `plan.yaml` are new document shapes.

In-place mode writes the same artifacts to `.emery/change/`. Durable product state (`project.yaml`, `specs/`, and `decisions/`) remains in `.emery/`, merges forward, and ships with the repository. The change home is temporary and is archived or deleted.

Operations therefore receive separate target (product) and change roots. A target tree excludes `.git` and any nested change home, but includes the rest of the repository. A detached change home needs neither `.emery/project.yaml` nor Git metadata. Versioning, backup, and review of it are operator concerns.

### D2 — Sources are generic immutable location bindings pinned as CIDs

Each source has a generated key, an exact adapter package pin, and exactly one of:

- `locator`: a Git reference (`url@revision`), change-relative path, external local path, or bounded HTTPS URL.
- `value`: inline content stored in `plan.yaml`.

Emery resolves each locator once, applies its optional `path` (default `.`), stages it temporarily, and stores the resulting file or tree under its CID. A file is represented as a one-file tree, so every location-backed source has the same read-only root shape. Inline values are already protected by the plan digest and are passed directly.

Git, local, and HTTPS locators all follow that path; none creates a persistent copy in the change home. Mutable Git refs become exact revisions, but every origin is provenance after the CID is recorded. Later runs use the recorded CID rather than rereading the origin. CIDs in `plan.yaml` remain store GC roots until the change ends. If one repository is both target and source, both roles reuse the same CID.

Source operations receive the source key, its read-only workspace or inline value, and read-only change artifacts. They never parse `plan.yaml`, assume that a source is a target, or capture a source workspace.

### D3 — Discovery runs before slice authoring

Detached `emery plan author` initializes the change home when needed, then runs two internal phases:

1. **Discover topology** — query bounded repository-host candidates, resolve sources, select adapters, run topology judgment, and write its validated result to `discovery.yaml`.
2. **Author slices** — copy the discovered targets and sources into `plan.yaml`, survey those sources into `leads.md`, and reconcile the leads into slices.

This decision owns phase order and the `discovery.yaml` / `plan.yaml` contract, not dispatch concurrency. Independent Discover-topology reads may proceed concurrently under D9's discovery limits; results still merge into one validated `discovery.yaml`. Parallel survey during Author slices is [RFC-91](rfc-91-concurrent-execution.md) D9 and does not change this ordering.

The discovery digest covers schema-validated content and is independent of YAML formatting. Both authoring and execution verify that `plan.yaml` matches `discovery.yaml` and that recorded source, target, CID, and adapter pins remain valid. `--force` runs discovery again. Raw judgment requests, responses, repair attempts, and source no-match or ambiguity diagnostics are ephemeral, not change artifacts.

One target and one source produce:

```yaml
# discovery.yaml
version: 1
targets:
  orders:
    adapter: emery:omnia@1.4.0
    locator: https://github.com/acme/orders@0123456789abcdef0123456789abcdef01234567
    cid: sha256:…
sources:
  orders-code:
    adapter: emery:typescript@1.2.0
    locator: https://github.com/acme/orders@0123456789abcdef0123456789abcdef01234567
    cid: sha256:…
```

`plan.yaml` copies those maps and binds slices to them:

```yaml
name: orders-modernization
discovery-digest: sha256:…
targets:
  orders:
    adapter: emery:omnia@1.4.0
    locator: https://github.com/acme/orders@0123456789abcdef0123456789abcdef01234567
    cid: sha256:…
sources:
  orders-code:
    adapter: emery:typescript@1.2.0
    locator: https://github.com/acme/orders@0123456789abcdef0123456789abcdef01234567
    cid: sha256:…
slices:
  - name: orders-api
    target: orders
    sources:
      - source: orders-code
        lead: orders-api
```

### D4 — Registry-backed workspace coordination is removed

Detached plans create no `registry.yaml`, topology lock, or workspace slots. `plan.yaml.targets` is the sole stored topology; registry-shaped and target-head views are computed when needed. `emery init --workspace`, workspace routing, slot synchronization, and committed-registry handlers are removed. Regular in-place product repositories remain.

### D5 — Targets record both an exact Git base and an initial CID

A target records both:

- `locator`: the exact Git commit as `url@revision`, discovered at the repository host.
- `cid`: the content identifier of that commit's tree, excluding `.git` and any nested change home.

The identified tree includes durable Emery state such as `project.yaml`, `specs/`, and `decisions/`. Git revisions identify publication bases; CIDs identify trees used by Emery. A moved branch is only a freshness warning, but an unavailable recorded commit is an error.

Every target is an existing forge repository at discovery time — including an empty or freshly initialized one the operator created by hand so change-mode filters can see its `product:`. Authoring scope has no `--create` or `create:` field, discovery has no `action: create`, and there is no execute-time repository provisioning in this cut. How a surveyed source maps onto a target is design prose (`change.md`, per-slice `design.md`) plus the authored `slices[].target` binding, not a scope create list.

In change mode, each `discover` row's `target` bound limits repository-host discovery and a server-side marker narrows namespace rows, but the pinned `.emery/project.yaml` decides membership through its non-empty `product:` list. A row's product filters intersect that list; an exact row nominates its repository without filters but passes the same membership check; `platforms:` remains the build set. In migrate mode, source locators and optional `discover` rows provide candidates; topology judgment may propose target bindings and fill adapter gaps only against discovered repositories. Existing `project.yaml` configuration wins. A slice may bind only a target that carries a target-axis adapter.

Target discovery and source selection are independent. One target may supply several sources, a source may have no target, and a source no-match never removes its target row.

### D6 — The host supplies the adapter catalog and exact versions are recorded

The host supplies a bounded, versioned catalog of source adapters and their recognition profiles, plus target adapters and their platform constraints. The engine performs deterministic matching and topology validation without owning the adapter inventory.

When a source omits `adapter`, Emery fingerprints its immutable value. One matching profile selects the adapter and pins it. No matches or several matches omit that source and surface `source-adapter-no-match` or `source-adapter-ambiguous` as ephemeral diagnostics until the operator names an adapter. There is no ranking or model fallback.

Discovery records every selected adapter as an exact package pin (`emery:<name>@<semver>`). Unversioned local components cannot enter detached topology. The initial catalog recognizes `typescript`, `documentation`, `screenshots`, and `captures`; `intent` is explicit; target-axis adapters are `omnia`, `vectis`, and `contracts`. Explicit third-party adapters are allowed when the resolver can produce the same exact identity.

Source keys are independent of targets. A locator uses its normalized basename; an inline value uses its adapter; `intent` is reserved. Unchanged bindings retain their keys, collisions receive stable digest suffixes, and duplicate bindings are rejected. The persisted key is authoritative downstream.

### D7 — Execution maintains one accepted CID per target

`plan.yaml` stores only each target's initial CID. A build fact names its base and result CIDs; a successful merge makes that result the target's next accepted CID. Emery computes the current CID from the merge facts and rejects any broken base/result chain. Later slices therefore start from earlier accepted work. `depends-on` controls scheduling, not this merge order.

Each code operation receives a fresh writable workspace prepared from the selected target's current accepted CID. Change artifacts are mounted separately and read-only. The product baseline already lives inside the identified tree, so there is no second baseline tree. Drift from a slice's pinned baseline is reported rather than hidden.

`emery slice merge` prepares the build result, runs target-adapter preflight, merges the delta spec into its baseline, captures the new accepted CID, then runs postflight against that merged state. Merges remain serial per target. The change home is never prepared or captured as product code, and RFC-87's interim write-back operation, `apply`, is removed.

### D8 — Plan execution is the only privileged-start surface

`emery plan execute` verifies that the plan matches `discovery.yaml`, then appends RFC-86's `plan.execute.started` fact. Detached start requires `discovery-digest` alongside the plan and any existing spec digests. Only then may Emery execute slices. Any covered change (amended plan, discovery, or in-scope specs) requires the operator to run execute again. Repository-host writes (create, push, branch, PR, merge) remain out of scope — create on the forge if you need a new target; RFC-89 owns publication.

### D9 — Reads are bounded and the change home is disposable

Repository-host access is infrastructure, not an adapter axis. Each `discover` row's `target` bound selects the provider and bounds its namespace; for example, `https://github.com/acme/*` selects GitHub repositories owned by `acme`, while a GitLab group or subgroup `/*` bound covers that provider accordingly. The provider narrows each namespace row with a server-side marker and the row's product and topic filters, then verifies selected repositories at exact revisions; an exact row costs one verification read, not a search. The budgets below cover the union of rows; oversized searches fail with `discovery-too-broad` and suggest narrowing rows or filters.

A versioned policy limits candidates, API requests, concurrency, time, inspected bytes, imported trees, redirects, and HTTPS bodies. The concurrency bound covers Discover-topology reads (candidate queries, source and target resolve and CID capture, fingerprint reads), not source-adapter survey — that fan-out is RFC-91. Remote URLs require HTTPS, contain no credentials, and cannot target private networks. Tree reads run no hooks, submodules, LFS filters, or escaping symlinks. GitHub document pages resolve to raw content.

After RFC-89 publishes and archives the change, no coordination state is required. Product configuration, baselines, repositories, repository-host history, and caches may remain. Deleting an unreplicated change home loses its facts; retaining it preserves more audit history. Before RFC-92, copying the change home does not transport source or result tree objects.

## Implementation requirements

- The public workflow remains `emery plan author → emery plan execute → emery plan archive`; RFC-89 owns the seal and successful archive gate after execute.
- `Plan` gains `targets`, `discovery-digest`, exact source pins with `cid`, and singular `slices[].target`; `ProjectConfig` gains a unique kebab-case `product` list. The current `discovery.md` lead inventory becomes `leads.md`. Validation checks the resulting graph as one unit.
- Operations take explicit target (product) and change roots. Detached roots are unrelated; in-place changes use `<product>/.emery/change/`. Detached homes have no synthetic `project.yaml`.
- Target trees ignore only `.git` and a nested change home. `.emery/` is otherwise included.
- Repository ingestion accepts an exact revision; workspace preparation accepts an explicit target and CID. Builds no longer `freeze` ambient roots, and merges update the baseline inside the workspace. `Workspaces::apply` and its write-back machinery are removed.
- The source WIT receives either a read-only workspace or inline value. Target-axis adapters continue to receive a prepared workspace and read-only change artifacts.
- Plan CIDs remain GC roots for the change lifetime. Resolution exposes exact adapter package pins; the host supplies bounded discovery access, the adapter catalog, and repository-host access.
- Discovery DTOs reject unknown fields and use typed canonical digests. Integration tests use local repository-host, HTTP, content-addressed store, and component fixtures.
- Artifact field `cid` is the RFC-87 tree identity; keep `SnapshotId` as the Rust type alias or rename in a follow-on cut — wire documents say `cid`.

## Acceptance criteria

1. An empty non-Git directory can author a detached change without `.emery/project.yaml`; in-place mode writes the same artifacts under `.emery/change/`. Durable product state remains outside the change home and inside target trees.
2. Every location-backed source resolves once to a CID; inline values remain under the plan digest. Source operations receive only the pinned read-only root or inline value. A repository used as both target and source reuses one CID.
3. Source inference selects exactly one adapter or omits the source with an ephemeral no-match or ambiguity diagnostic without removing its target. Generated keys are deterministic, stable across later collisions, and reject duplicate bindings.
4. Before survey, `discovery.yaml` records the pinned targets and sources with CIDs and adapter package pins. Edits, unknown fields, stale revisions, or changed adapter pins block execution.
5. Two slices on one target form a valid initial → result₁ → result₂ CID chain. Each result tree contains code and merged baseline; the second workspace sees the first result. The change home and ambient product trees are never write targets.
6. `plan execute` is the only privileged-start action (`plan.execute.started`). It does not create forge repositories.
7. Repository-host and source reads obey recorded limits and fail closed. Host markers narrow discovery but never override pinned `project.yaml.product`, and execution never repeats membership discovery.
8. Execution ends with one accepted CID per touched target and no commit or branch; RFC-89 publishes it. Copying the change home before RFC-92 does not transport source or result tree objects.
9. Removed concepts stay removed: workspace registries and slots, ambient product roots, authored source keys, engine-owned adapter inventories, separate discovery or approve commands, plan-approval vocabulary (`plan.approved`, projected `approved`), authoring-scope `--create` / discovery `action: create` / execute-time repository provisioning, a second source-digest scheme, discovery `mode` / catalog-digest / policy-digest / disposition fields, persisted exclusion rows, nested `adapter.package` / `adapter.component-digest`, the artifact field name `snapshot` for tree identity, baseline writes outside workspaces, `apply`, and repository-host publication writes.
10. `cargo make ci` passes with crate-level integration coverage for discovery, pinning, execution, and `plan.execute.started` coverage.

## Rejected alternatives

- **Permanent platform repository, registry, or durable out-of-tree change store** — makes change-scoped coordination a platform to tend.
- **Asymmetric `projects:` topology with nested `repository:` / `target:` adapter fields** — invents a third noun beside the source/target axes; the isomorphic `targets:` / `sources:` maps keep one row shape and one adapter pin per binding.
- **A single `discover.root` with intersecting filters, or arbitrary repository globs** — one root cannot express the union "every namespace repository with this product, plus these exact repositories" in one change, and mid-path patterns (`acme/orders-`*) unbound the namespace guarantee name filters were never meant to carry. Selector rows union, filters within a row intersect, and the only glob is a trailing namespace `/*`.
- **Origin-specific source schemas or target-bound sources as the only source form** — cannot represent local documents, HTTPS material, inline intent, or several inputs from one repository uniformly.
- **Live source reads or ambient checkouts during operations** — make judgments and builds depend on mutable location rather than pinned values.
- **Git revision as a CID or a mutable target head in** `plan.yaml` — conflates publication identity, tree identity, and state already computed from facts.
- **Naming the tree-identity field `snapshot`** — the value is a content identifier; `cid` matches that role. RFC-87's prepare/capture contract is unchanged.
- **Adopting IPFS multicodec CIDs** — Emery's wire form stays `sha256:<64 lowercase hex>` over the RFC-87 manifest; no multibase or codec prefix.
- **A source digest scheme separate from tree CIDs** — a second content identity for the same kind of value, when the tree manifest already distinguishes a file from a tree and one path from another.
- **Keeping the baseline outside the target tree** — splits one result across two authorities, hides the baseline from target workspaces, and requires separate composition.
- **Granting a second read-only baseline root** — works around that split rather than fixing it; baseline drift should remain a diagnostic.
- **Separate open, discover, or topology-approve commands** — expose internal authoring phases without adding an operator decision boundary; `plan execute` already records `plan.execute.started`.
- **Owning survey/extract fan-out concurrency here** — Discover-topology reads may use D9's concurrency budget; Author-slices survey parallelism is RFC-91. Mixing them would gate the product path on the scale track.
- **Persisting discovery `mode`, catalog-digest, policy-digest, disposition, or exclusion rows** — process envelope, not topology. Pinned locators, CIDs, and adapter package pins are the authority; rejects and change-vs-migrate are invocation or diagnostic concerns, and catalog/policy stay host runtime.
- **Recording `adapter.component-digest` beside the package pin** — enterprise supply-chain hardening. MVP pins `adapter: emery:<name>@<semver>`; store verify-on-read stays host-side.
- **Engine-owned adapter inventories, model-ranked selection, or resolving bare names on every machine** — violate adapter neutrality or make recorded topology non-reproducible.
- **Atomic/idempotent cross-system repository creation** — not needed while Emery does not provision repositories; if a create surface returns later, GitHub still offers no such guarantee and intent/receipt would be required.
- **Authoring-scope `--create` / discovery `action: create` / execute-time provisioning** — invents a privileged topology path before survey and design can say where work belongs. Operator-created forge repos with `product:` plus design prose for source→target mapping scale without a second create contract.
- **Repository-host access as a third adapter axis or Emery-owned push/PR/merge** — host access is infrastructure, while publication remains operator-owned and RFC-89-defined.

