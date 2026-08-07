# RFC-88: Detached Changes

> Status: Draft — step 3 of the platform-migration series ([platform.md](platform.md))
>
> Owns: the detached change home; discovery and immutable pinning of targets, sources, and adapters; capability-profile-bound conflict-domain decomposition; refinement feedback into focused child leads; the deterministic buildable-leaf projection into `plan.yaml`; and per-target execution over accepted CIDs.
>
> Builds on [RFC-86](rfc-86-change-facts.md)'s facts, explicit execution-authorization epoch, and per-leaf input fences and [RFC-87](rfc-87-working-trees.md)'s content-addressed trees and private workspaces. [RFC-89](rfc-89-publication-sets.md) publishes the results; [RFC-92](rfc-92-node-sync.md) transports them between nodes.
>
> Amends RFC-86 D1 (the in-place change home is `.emery/change/`, not all of `.emery/`) and D6 (closed-plan coverage transitively binds model-capability profiles). Amends RFC-87: location-backed sources use D2's read-only views, D4 and acceptance criterion 4 include the target repository's durable state in its tree, the tree identity is named a **CID** in plan and discovery artifacts (RFC-87's `SnapshotId` is that CID), and the interim `apply` is deleted.

## Intent

Decouple change coordination from product checkouts and permanent workspace infrastructure.

A change owns portable facts and artifacts, never product code or durable product state. Participating targets and independent source inputs are discovered and pinned as immutable values, surveyed as one closed input set, then recursively decomposed into conflict domains until every terminal domain is a buildable slice. Execution processes those leaves in disposable private workspaces. No permanent platform repository, committed registry, or operator-tended `workspace/<project>/` slots remain.

This gives migrate and ongoing change one multi-target pipeline that produces a final accepted CID for each touched target, carrying that repository's code and merged baseline together. New repositories are created on the forge by the operator (with `product:` so change-mode discovery can find them); Emery does not provision repositories in this cut. Publication remains [RFC-89](rfc-89-publication-sets.md)'s responsibility.

## Flow and terms

1. The operator runs detached `emery plan author` with an authoring scope (CLI args or `--scope`; see [Scope](#scope)). Change uses bounded discovery selector rows; migrate uses explicit sources and may also bound target candidates through discovery. Intent is optional inline source value.
2. Discovery pins exact repositories, CIDs, and adapter versions into `discovery.yaml`.
3. `plan author` surveys every source, recursively partitions the combined lead catalog into persisted conflict domains, and projects the terminal domains into `plan.yaml.slices`.
4. The operator reviews the decomposition and leaf plan together; `plan execute` appends `plan.execute.started` covering both and runs only the slice leaves in disposable private workspaces.
5. Each touched target ends with one accepted CID for RFC-89 to publish.

Nouns: **change home** (RFC-86 fact tree; no product code or durable product state); **target** (participating repo + initial CID + optional target-axis adapter); **source** (immutable CID or inline value); **CID** (`sha256:` + hex of RFC-87's tree manifest; wire field `cid`); **conflict domain** (a recursively identified scope whose child results may interact and therefore converge together); **accepted target CID** (latest successful merge result for one target).

`sources` and `targets` share one row shape (`adapter`, `locator` or `value`, `cid`). Durable product state stays `.emery/project.yaml` inside each repository.

## Scope

The authoring scope is an invocation envelope, not a change artifact. Supply it as flags or with `--scope`; prefer the file when lists grow. After authoring, `discovery.yaml` is topology authority, `decomposition.yaml` is conflict-domain authority, and `plan.yaml` is its executable leaf projection. Source keys and adapter pins are discovery outputs (D5, D6).

When present, `discover` is a non-empty list of selector rows. Each row has exactly one required `target` bound — an exact repository URL, or a repository-host namespace (an org, group, or subgroup) ending in `/*`; nothing else globs — and optional `products` and `topics` filters. Rows union; filters within a row intersect. A namespace row is a bounded candidate search; an exact row skips the search and asserts one candidate directly, subject to the same verification. The CLI composes rows: `--target` appends a selector row (exact URL or namespace `/*`); repeated `--product` / `--topic` flags fill the most recent row's filters; `--source` appends to `sources`; `--intent` sets `intent`. Exact clap names are illustrative. Scope supplies the bounded estate, never its decomposition.

Scope cannot create targets or prescribe source→target mappings. Create a forge repository with `product:` so discovery can find it; recursive authoring records each mapping in `decomposition.yaml` and renders its rationale into `change.md` or per-slice `design.md`. There is no `--create` or `create:` surface.

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
  decomposition.yaml           # conflict-domain hierarchy + leaf projection inputs
  decompositions/<digest>.yaml # immutable decomposition revisions once referenced
  discovery.yaml              # pinned topology from discovery
  leads.md                    # source-adapter lead inventory
  leads/<digest>.md            # immutable lead-catalog revisions once referenced
  planning/proposals/<digest>.yaml # validated but unapplied amendments
  domains/<domain>/rounds/<digest>.yaml # RFC-91 convergence records
  targets/<target>/waves/<digest>.yaml # immutable closed wave membership
  events/<actor>.jsonl        # facts, including plan.execute.started
  slices/<slice>/...
```

`discovery.yaml` records the pinned targets and sources: CIDs and exact adapter package pins. Candidates that do not match are omitted; match failures surface as ephemeral diagnostics, not change artifacts.

`leads.md` is an authoritative parsed catalog, not unbound review prose. Its canonical `leads-digest` covers every source key and, within that source, every lead id, synopsis, topic, parent relation, and source-local focus. Before a decomposition revision or build fact references that digest, Emery copies the exact document to `leads/<digest>.md`; later focused survey appends create another revision instead of changing the meaning of an existing reference.

`decomposition.yaml` records the reviewed conflict-domain hierarchy: its `leads-digest`, stable node ids, parent/child relations, contributing `(source, lead)` scopes, target bindings, ownership envelopes, dependencies, local gate kinds, and the terminal node → slice mapping. Internal nodes carry no lifecycle or claim. Once referenced by an execution or domain-round fact, the exact document is retained as `decompositions/<digest>.yaml`; later amendments produce a new revision and current view.

`plan.yaml` copies the discovered topology, carries matching `leads-digest` and `decomposition-digest`, and adds exactly the terminal slice projection. Domain dependencies compile into leaf `depends-on` edges; the deterministic projector expands a dependency from one domain to another as edges from the source domain's exit leaves (terminal descendants with no successor inside that domain) to the destination domain's entry leaves (terminal descendants with no predecessor inside that domain).

`domains/<domain>/rounds/` contains RFC-91's immutable convergence records, not domain lifecycle state. `planning/proposals/` contains validated but unapplied amendments proposed by overlap recovery, refinement boundary escalation, or the operator. RFC-91 adds target-decomposition escalation as another author. A proposal has no authority until `emery plan amend --proposal <digest>` validates and applies it.

`targets/<target>/waves/` contains RFC-86's immutable wave manifests. A manifest closes the member leaves, accepted base CID, planning revisions, dependency frontier, and build-authorization epoch before any member build starts. The committed fact separately names its closed-plan commit authorization, so a future streaming-built member may be reviewed and committed under a later epoch. RFC-86's initial executor creates one-member waves; RFC-91 may select several independent ready leaves into one manifest without changing merge facts or accepted-CID semantics.

`change.md` and the documents under `slices/<slice>/` keep their existing formats. `leads.md` is the current `discovery.md` format under a clearer name and also records any focused child leads with their parent lead. `discovery.yaml`, `decomposition.yaml`, immutable planning revisions, amendment proposals, and the RFC-88 additions to `plan.yaml` are the new document shapes.

In-place mode writes the same artifacts to `.emery/change/`. Durable product state (`project.yaml`, `specs/`, and `decisions/`) remains in `.emery/`, merges forward, and ships with the repository. The change home is temporary and is archived or deleted.

Operations therefore receive separate target (product) and change roots. A target tree excludes `.git` and any nested change home, but includes the rest of the repository. A detached change home needs neither `.emery/project.yaml` nor Git metadata. Versioning, backup, and review of it are operator concerns.

### D2 — Sources are generic immutable location bindings pinned as CIDs

Each source has a generated key, an exact adapter package pin, and exactly one of:

- `locator`: a Git reference (`url@revision`), change-relative path, external local path, or bounded HTTPS URL.
- `value`: inline content stored in `plan.yaml`.

Emery resolves each locator once, applies its optional `path` (default `.`), stages it temporarily, and stores the resulting file or tree under its CID. A file is represented as a one-file tree, so every location-backed source has the same read-only root shape. Inline values are already protected by the plan digest and are passed directly.

Git, local, and HTTPS locators all follow that path; none creates a persistent copy in the change home. Mutable Git refs become exact revisions, but every origin is provenance after the CID is recorded. Later runs use the recorded CID rather than rereading the origin. CIDs in `plan.yaml` remain store GC roots until the change ends. If one repository is both target and source, both roles reuse the same CID.

Source operations receive the source key, its read-only workspace or inline value, and read-only change artifacts. They never parse `plan.yaml`, assume that a source is a target, or capture a source workspace.

The initial `survey` runs with no focus and emits source-local planning leads; a lead is no longer promised to be a final slice boundary. To split a broad lead without guessing from its synopsis, the source contract lets the engine survey that lead as a focus and append stable child leads under it. The engine controls recursion and budgets; an adapter handles only one requested source scope.

Each leaf still binds at most one lead from any source because refinement persists exactly one `evidence/<source>.yaml`. Internal domains may retain broad parent leads as planning context, but a leaf that needs a focused child from that source binds only the child; the adapter's child scope includes the inherited parent context needed for extraction. Cross-cutting guidance may be multi-homed into several leaves, but never beside another lead from the same source in one leaf. `extract` names that one terminal `(source, lead)` pair, so Evidence remains focused and cannot overwrite a sibling extraction.

### D3 — Discovery, survey, and refinement feed recursive leaf authoring

Detached `emery plan author` initializes the change home when needed, then runs three internal phases:

1. **Discover topology** — query bounded repository-host candidates, resolve sources, select adapters, run topology judgment, and write its validated result to `discovery.yaml`.
2. **Survey sources** — survey every pinned source into `leads.md`; RFC-91 may fan these independent calls out without changing their stable merge order.
3. **Decompose and project** — create one root over the complete lead catalog, recursively partition it into conflict domains, persist the validated hierarchy in `decomposition.yaml`, and deterministically project its terminal domains into `plan.yaml.slices`.

Before decomposition, the engine resolves one closed, versioned model-capability profile for each target. A profile contains the complexity scoring policy and operation-specific thresholds for the configured model class. This RFC consumes its slice-split threshold. RFC-91 also consumes its task threshold. The profile is an engine input, not a model answer.

The profile scores a closed assessment of behavioural breadth, coupling, uncertainty, context volume, and verification surface. Each dimension is an integer from zero through ten. The judgment supplies those dimensions and a rationale. The engine computes the weighted sum and applies the operation threshold. Lines of code and advertised context-window size are not scoring policies by themselves.

`decomposition.yaml` records each profile's closed body, id, and digest. `plan.yaml` copies the ids and digests. Decomposition operation keys cover the relevant profile digests. Changing a profile creates new planning revisions and invalidates the old execution epoch.

For each open domain, the judgment response is typed `split` or `leaf`. The engine validates it before continuing.

A split must preserve at-least-once lead coverage, retain every cross-cutting lead on each child it informs, bind every child inside its parent's target set, and strictly reduce a normalized scope measure. It must also stay within fixed depth, node, repair, and judgment budgets.

Siblings predicted to touch the same ownership scope must carry an explicit order or fan-in child. Ambiguous overlap blocks authoring.

A leaf must bind exactly one target, state one coherent behavioural outcome, fit the target's bounded build and verification envelope, expose an ownership manifest and reviewable acceptance boundary, and carry at most one terminal lead per source. Its provisional complexity assessment is evaluated against the bound target's profile.

A provisional score above the slice-split threshold sends the candidate leaf through one bounded boundary review before it may close. When the review identifies separately acceptable source-local boundaries, the engine runs focused surveys and requeues the domain with their child leads. When no coherent split exists but the complete slice still fits the target envelope, the leaf may close with the rationale recorded. An over-envelope leaf that cannot split is unready and blocks authoring.

Containment and execution order are separate relations. Parent/child edges explain recursive decomposition. Dependency edges order siblings or domains. Only the latter compile into `plan.yaml.depends-on`; neither relation creates status for an internal node. A one-slice change is the degenerate root → leaf tree.

The deterministic engine owns the queue, termination tests, validation, and projection. Judgment proposes one bounded partition at a time; there is no lead agent that may recursively spawn arbitrary workers. Uncertain boundaries, estimates, or source-to-target assignments are preserved in `change.md` for operator review rather than silently accepted.

Refinement reassesses the leaf after `extract` has produced Evidence for every bound source. `extract` still returns Evidence for one terminal `(source, lead)` pair. It does not author child leads or mutate planning artifacts.

Before promoting `proposal.md`, `spec.md`, `design.md`, `tasks.md`, or `model.yaml`, the refinement judgment returns a typed `proceed | boundary-escalation` outcome. The boundary assessment uses the same closed complexity dimensions and pinned profile, now informed by the complete Evidence set.

Complexity is a trigger for boundary review, not sole authority over lifecycle shape. A score above the split threshold causes escalation only when the Evidence supports separately acceptable behavioural boundaries or shows that the complete slice exceeds the target's bounded verification envelope. If the work remains one coherent acceptance unit, refinement proceeds as one slice. RFC-91 may then divide its implementation into model-sized tasks.

A validated boundary escalation names the affected terminal `(source, lead)` pairs and gives a typed rationale. The engine runs focused `survey` for those parents, producing stable source-local child leads. It then reruns decomposition for the nearest affected domain.

The resulting candidate lead-catalog and decomposition revisions remain inert inside one amendment proposal. The current `leads.md`, `decomposition.yaml`, and `plan.yaml` do not change. Refinement promotes no synthesis artifacts, performs no `refined` transition, and starts no build work.

After the operator applies the proposal and starts a new execution epoch, each projected child slice follows the ordinary refinement path. `extract` runs again for its child `(source, lead)` pair. Evidence from the failed parent refinement is not reused as child Evidence.

Focused resurvey and re-decomposition use the ordinary depth, node, judgment, and repair budgets. Exhaustion parks the leaf for the operator instead of retrying indefinitely.

This decision owns phase order and the `discovery.yaml` / `decomposition.yaml` / `plan.yaml` contract, not dispatch concurrency. Independent Discover-topology reads may proceed concurrently under D9's discovery limits; results still merge into one validated `discovery.yaml`. RFC-91 owns parallel initial and focused survey calls and may evaluate independent open domains concurrently, but deterministic ordering and byte-stable artifacts do not change.

The discovery, lead-catalog, and decomposition digests cover schema-validated content and are independent of Markdown or YAML formatting. Both authoring and execution verify that `plan.yaml` matches `discovery.yaml`, that its `leads-digest` identifies the retained catalog revision, that its leaves and dependencies are the exact decomposition projection, and that recorded source, target, CID, and adapter pins remain valid. `--force` runs discovery and decomposition again. Raw judgment requests, responses, and repair attempts remain ephemeral; the validated hierarchy, rationales, uncertainty findings, and focused lead inventory persist.

The first implementation publishes `decomposition.yaml` and `plan.yaml` together only after the complete tree passes. The immutable revision layout is deliberately finer-grained than that policy: a future streaming execution epoch may publish closed domain branches and ready leaves while other surveys continue, and every build remains bound to the exact lead and decomposition revisions it saw. Removing `plan.execute.started` or inferring it from a claim is not required to add that mode.

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

For a one-leaf result, `decomposition.yaml` is still present:

```yaml
version: 1
leads-digest: sha256:…
model-capability-profiles:
  orders:
    id: frontier-large-v1
    digest: sha256:…
    weights:
      behavioural-breadth: 3
      coupling: 4
      uncertainty: 2
      context-volume: 1
      verification-surface: 3
    thresholds:
      slice-split: 80
      task: 35
root: orders-modernization
nodes:
  orders-modernization:
    children: [orders-api]
    sources:
      - source: orders-code
        lead: orders-api
  orders-api:
    parent: orders-modernization
    target: orders
    slice: orders-api
    ownership: [src/orders/**]
    sources:
      - source: orders-code
        lead: orders-api
```

`plan.yaml` copies the topology and binds only the terminal projection:

```yaml
name: orders-modernization
discovery-digest: sha256:…
leads-digest: sha256:…
decomposition-digest: sha256:…
targets:
  orders:
    adapter: emery:omnia@1.4.0
    locator: https://github.com/acme/orders@0123456789abcdef0123456789abcdef01234567
    cid: sha256:…
    model-capability-profile:
      id: frontier-large-v1
      digest: sha256:…
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

Every target is an existing forge repository at discovery time — including an empty or freshly initialized one the operator created by hand so change-mode filters can see its `product:`. Authoring scope has no `--create` or `create:` field, discovery has no `action: create`, and there is no execute-time repository provisioning in this cut. How surveyed source scopes map onto a target is recorded by conflict-domain nodes, rendered as design prose (`change.md`, per-slice `design.md`), and projected into `slices[].target`; it is not a scope create list.

In change mode, each `discover` row's `target` bound limits repository-host discovery and a server-side marker narrows namespace rows, but the pinned `.emery/project.yaml` decides membership through its non-empty `product:` list. A row's product filters intersect that list; an exact row nominates its repository without filters but passes the same membership check; `platforms:` remains the build set. In migrate mode, source locators and optional `discover` rows provide candidates; topology judgment may propose target bindings and fill adapter gaps only against discovered repositories. Existing `project.yaml` configuration wins. A slice may bind only a target that carries a target-axis adapter.

Target discovery and source selection are independent. One target may supply several sources, a source may have no target, and a source no-match never removes its target row.

### D6 — The host supplies the adapter catalog and exact versions are recorded

The host supplies a bounded, versioned catalog of source adapters and their recognition profiles, plus target adapters and their platform constraints. The engine performs deterministic matching and topology validation without owning the adapter inventory.

When a source omits `adapter`, Emery fingerprints its immutable value. One matching profile selects the adapter and pins it. No matches or several matches omit that source and surface `source-adapter-no-match` or `source-adapter-ambiguous` as ephemeral diagnostics until the operator names an adapter. There is no ranking or model fallback.

Discovery records every selected adapter as an exact package pin (`emery:<name>@<semver>`). Unversioned local components cannot enter detached topology. The initial catalog recognizes `typescript`, `documentation`, `screenshots`, and `captures`; `intent` is explicit; target-axis adapters are `omnia`, `vectis`, and `contracts`. Explicit third-party adapters are allowed when the resolver can produce the same exact identity.

Source keys are independent of targets. A locator uses its normalized basename; an inline value uses its adapter; `intent` is reserved. Unchanged bindings retain their keys, collisions receive stable digest suffixes, and duplicate bindings are rejected. The persisted key is authoritative downstream.

### D7 — Execution maintains one accepted CID per target

RFC-86 defines the independently deployable one-member wave and atomic merge fact; with RFC-87 snapshot values, this RFC makes that fact the accepted-CID transition for each detached target. `plan.yaml` stores only each target's initial CID. The serial scheduler selects only a leaf whose dependencies are already accepted, writes a closed target-wave manifest against the current accepted CID, and appends `target.wave.opened`. A dependent leaf therefore enters a later wave based on its producer wave's accepted result. A build fact names its wave digest, build authorization, exact planning revisions, and base/result CIDs; the committed fact names the closed-plan commit authorization.

Once every member result is present and its required gates pass, target-wave merge folds every member's delta spec in stable order and captures one final candidate CID. It then appends one `target.merge.wave-committed` fact with the frozen member set, base/result CIDs, finalized identity maps, and optional RFC-91 domain-round digests. That fact advances the accepted CID and projects every member `merged`; no prefix is authoritative. Emery computes the current CID from committed facts and rejects any broken wave chain.

`emery slice merge <slice>` resolves the slice's frozen wave and refuses until every member is complete; it can never manufacture a singleton prefix. In this RFC's serial executor every wave has one member. RFC-91's concurrent executor opens deterministic bounded ready antichains and the same command may therefore commit the complete multi-member wave containing the named slice. A result from an older base outside the open wave is stale and must rebuild; it cannot be silently attached.

Each code operation receives a fresh writable workspace prepared from the selected target's current accepted CID. Change artifacts are mounted separately and read-only. The product baseline already lives inside the identified tree, so there is no second baseline tree. Drift from a leaf's pinned baseline is reported rather than hidden.

Target-wave merge prepares the verified composed result, runs every member's slice-scoped target-adapter preflight in stable member order, folds every delta spec and identity map, captures the final CID, and atomically appends the committed fact. It then runs each member's slice-scoped postflight in the same order, persists completed reports, and resumes at the first missing report after a crash. When all reports exist, it appends either `target.merge.wave-succeeded` or one aggregate `target.merge.wave-postflight-failed` naming every failed member. A postflight failure is non-rollback: the accepted wave stands and `plan execute` stops until the existing acknowledgement path is invoked. A crash before commit leaves the prior CID authoritative. The change home is never prepared or captured as product code, and RFC-87's interim write-back operation, `apply`, is removed.

### D8 — Plan execution is the only privileged-start surface

`emery plan execute` verifies that the plan matches `discovery.yaml`, binds the retained lead revision, and is the exact leaf projection of `decomposition.yaml`. It then appends RFC-86's `plan.execute.started` with `closed-plan` coverage.

The detached event carries `plan-digest`, required `discovery-digest`, and sorted per-leaf spec coverage: an existing digest or `refine-under-epoch`. The plan digest transitively binds the plan's lead, decomposition, and model-capability profile digests. Execution validates those fields against the retained revisions but does not duplicate them on the event.

Only then may Emery execute slices. Any covered change requires the operator to run execute again. Covered changes include the lead catalog, decomposition, plan, discovery, model-capability profile, existing spec, and refinement output.

Runtime ownership overlap writes a validated amendment proposal naming the nearest domain, new dependency or fan-in leaf, expected planning digests, expected accepted CID per target, the complete committed leaf→wave set, and the affected open-wave and claim frontier.

Refinement boundary escalation writes a proposal containing the failed leaf, its assessment and profile digest, the candidate child-lead catalog, the candidate nearest-domain decomposition, and the same expected planning and execution frontiers.

Neither proposal can edit authority or execute hidden work. The operator first quiesces or retracts affected claims and uncommitted waves. The operator then applies the proposal with `emery plan amend --proposal <digest>`.

The command compare-and-sets every expected revision and accepted frontier. It refuses any live affected work. For a boundary proposal, it atomically activates the candidate lead catalog and decomposition before reprojecting `plan.yaml`.

Application preserves every committed leaf's identity, source binding, target, dependencies, and terminal mapping. It writes and retains the new planning revisions and invalidates the old closed-plan epoch.

Accepted leaves may gain new dependants. They cannot be removed, rebound, reordered behind new work, or disappear from publication membership.

Existing direct leaf `plan add` / `amend` / `remove` operations must lower to the same domain mutation and preservation checks. They refuse when no unambiguous hierarchy edit exists.

Repository-host writes remain out of scope. Create a new target on the forge when needed. RFC-89 owns push, branch, pull-request, merge, and other publication writes.

### D9 — Reads are bounded and the change home is disposable

Repository-host access is infrastructure, not an adapter axis. Each `discover` row's `target` bound selects the provider and bounds its namespace; for example, `https://github.com/acme/*` selects GitHub repositories owned by `acme`, while a GitLab group or subgroup `/*` bound covers that provider accordingly. The provider narrows each namespace row with a server-side marker and the row's product and topic filters, then verifies selected repositories at exact revisions; an exact row costs one verification read, not a search. The budgets below cover the union of rows; oversized searches fail with `discovery-too-broad` and suggest narrowing rows or filters.

A versioned policy limits candidates, API requests, concurrency, time, inspected bytes, imported trees, redirects, and HTTPS bodies. The concurrency bound covers Discover-topology reads (candidate queries, source and target resolve and CID capture, fingerprint reads), not source-adapter survey — that fan-out is RFC-91. Remote URLs require HTTPS, contain no credentials, and cannot target private networks. Tree reads run no hooks, submodules, LFS filters, or escaping symlinks. GitHub document pages resolve to raw content.

After RFC-89 publishes and archives the change, no coordination state is required. Product configuration, baselines, repositories, repository-host history, and caches may remain. Deleting an unreplicated change home loses its facts; retaining it preserves more audit history. Before RFC-92, copying the change home does not transport source or result tree objects.

## Implementation requirements

- The public workflow remains `emery plan author → emery plan execute → emery plan archive`; RFC-89 owns the seal and successful archive gate after execute.
- `Plan` gains `targets`, `discovery-digest`, `leads-digest`, `decomposition-digest`, model-capability profile ids and digests, exact source pins with `cid`, and singular `slices[].target`; `ProjectConfig` gains a unique kebab-case `product` list. The current `discovery.md` lead inventory becomes the canonically digestible `leads.md`. Validation checks discovery, retained lead revision, profiles, decomposition, and leaf projection as one unit.
- Add the closed `decomposition.yaml` shape, canonical digest, bounded recursive partition kernel, closed complexity assessment, profile-scored leaf-readiness gate, domain-dependency compiler, exact projection validator, and RFC-89 target-contraction cycle check. CLI plan mutations must update the decomposition and reproject the plan or refuse; hand-edited drift never executes.
- Retain every referenced lead and decomposition revision by digest. Add closed ownership and refinement-boundary proposal DTOs plus `emery plan amend --proposal <digest>`. Application compare-and-sets current planning digests, accepted-target frontiers, the committed-leaf set, and affected-work quiescence. Runtime recovery may author proposals but never apply them.
- Route every detached target through RFC-86's immutable one-member wave and accepted-CID projection. Preserve separate build and commit authorization anchors plus stable per-member preflight/postflight report sequencing so RFC-91 can widen membership without changing the merge WIT operation.
- Operations take explicit target (product) and change roots. Detached roots are unrelated; in-place changes use `<product>/.emery/change/`. Detached homes have no synthetic `project.yaml`.
- Target trees ignore only `.git` and a nested change home. `.emery/` is otherwise included.
- Repository ingestion accepts an exact revision; workspace preparation accepts an explicit target and CID. Builds no longer `freeze` ambient roots, and merges update the baseline inside the workspace. `Workspaces::apply` and its write-back machinery are removed.
- The source WIT receives either a read-only workspace or inline value. Extend `survey` with an optional parent-lead focus and stable child-lead response so the engine can request source-local detail without adding a third source operation. `extract` continues to consume a terminal lead and return Evidence. The refinement judgment adds the typed `proceed | boundary-escalation` outcome. Target-axis adapters continue to receive a prepared workspace and read-only change artifacts.
- Plan CIDs remain GC roots for the change lifetime. Resolution exposes exact adapter package pins; the host supplies bounded discovery access, the adapter catalog, and repository-host access.
- Discovery DTOs reject unknown fields and use typed canonical digests. Integration tests use local repository-host, HTTP, content-addressed store, and component fixtures.
- Artifact field `cid` is the RFC-87 tree identity; keep `SnapshotId` as the Rust type alias or rename in a follow-on cut — wire documents say `cid`.

## Acceptance criteria

1. An empty non-Git directory can author a detached change without `.emery/project.yaml`; in-place mode writes the same artifacts under `.emery/change/`. Durable product state remains outside the change home and inside target trees.
2. Every location-backed source resolves once to a CID; inline values remain under the plan digest. Source operations receive only the pinned read-only root or inline value. A repository used as both target and source reuses one CID.
3. Source inference selects exactly one adapter or omits the source with an ephemeral no-match or ambiguity diagnostic without removing its target. Generated keys are deterministic, stable across later collisions, and reject duplicate bindings.
4. Before survey, `discovery.yaml` records the pinned targets and sources with CIDs and adapter package pins. Edits, unknown fields, stale revisions, or changed adapter pins block execution.
5. A broad source lead can be focused into stable child leads. A multi-target migration recursively decomposes into at least three domain levels, preserves every lead, terminates within configured budgets, and projects byte-stable buildable leaves and `depends-on` edges. Every leaf has at most one terminal lead per source. The selected model-capability profiles and their digests are planning inputs, and changing one invalidates the old decomposition and execution epoch. Leaf cycles, target-contraction publication cycles, lost coverage, non-reducing splits, duplicate source bindings, ambiguous ownership, an unready leaf, and lead/decomposition/plan drift block authoring or execution.
6. Referencing a lead or decomposition digest retains byte-identical source content at its immutable revision path. Editing a source key, lead id, synopsis, topic, parent relation, focus, domain, or leaf creates a new digest and invalidates every closed-plan execution and result that consumed the old current view.
7. The serial executor opens a one-member wave before build. Exactly one committed fact advances the accepted CID and projects its member merged; failure before that fact projects no merge, while postflight failure after it leaves the accepted CID in force and records the resumable stop. A dependent leaf opens later against that result. The change home and ambient product trees are never write targets.
8. `plan execute` is the only privileged-start action (`plan.execute.started`). Its closed-plan coverage transitively binds the lead, decomposition, and model-capability-profile revisions and does not create forge repositories; a live claim without that epoch cannot build or merge.
9. A runtime overlap produces an inert ownership amendment proposal. A refinement whose Evidence reveals separately acceptable child boundaries produces one inert boundary proposal with focused child leads and a candidate nearest-domain decomposition. It promotes no synthesis artifacts and performs no `refined` transition. Applying either proposal through `plan amend --proposal` compare-and-sets the expected planning revisions, accepted target CIDs, committed leaf set, and affected open-work frontier; refuses live affected claims or waves; preserves every accepted leaf and prior revision; activates only legal candidate revisions; and requires a new execution epoch. Each new child slice then extracts its own terminal child lead without reusing parent Evidence. Stale, malformed, cyclic, accepted-history-changing, or ambiguous proposals change nothing.
10. Repository-host and source reads obey recorded limits and fail closed. Host markers narrow discovery but never override pinned `project.yaml.product`, and execution never repeats membership discovery.
11. Execution ends with one accepted CID per touched target and no commit or branch; RFC-89 publishes it. Copying the change home before RFC-92 does not transport source or result tree objects.
12. Removed concepts stay removed: workspace registries and slots, ambient product roots, authored source keys, engine-owned adapter inventories, separate discovery or approve commands, plan-approval vocabulary (`plan.approved`, projected `approved`), authoring-scope `--create` / discovery `action: create` / execute-time repository provisioning, a second source-digest scheme, discovery `mode` / catalog-digest / policy-digest / disposition fields, persisted exclusion rows, nested `adapter.package` / `adapter.component-digest`, the artifact field name `snapshot` for tree identity, baseline writes outside workspaces, `apply`, and repository-host publication writes.
13. `cargo make ci` passes with crate-level integration coverage for discovery, pinning, capability-profile binding, lead/decomposition revision retention, bounded decomposition, refinement boundary escalation, leaf projection, proposal application, immutable one-member wave creation, commit/postflight crash boundaries, execution, and `plan.execute.started` coverage.

## Rejected alternatives

- **Permanent platform repository, registry, or durable out-of-tree change store** — makes change-scoped coordination a platform to tend.
- **Asymmetric `projects:` topology with nested `repository:` / `target:` adapter fields** — invents a third noun beside the source/target axes; the isomorphic `targets:` / `sources:` maps keep one row shape and one adapter pin per binding.
- **A single `discover.root` with intersecting filters, or arbitrary repository globs** — one root cannot express the union "every namespace repository with this product, plus these exact repositories" in one change, and mid-path patterns (`acme/orders-`*) unbound the namespace guarantee name filters were never meant to carry. Selector rows union, filters within a row intersect, and the only glob is a trailing namespace `/*`.
- **Origin-specific source schemas or target-bound sources as the only source form** — cannot represent local documents, HTTPS material, inline intent, or several inputs from one repository uniformly.
- **One-shot reconciliation directly from flat leads to slices** — assumes each source already knows the global target and conflict boundaries, loses the hierarchy needed for bounded scheduling and convergence, and cannot focus an oversized lead without guessing from its synopsis.
- **Let complexity alone define slice boundaries** — model capacity sizes execution work, while behavioural coherence and independent acceptance define lifecycle units. A high score triggers boundary review but cannot manufacture a meaningful split.
- **Let the model choose its threshold, or derive it from lines of code or context-window size** — makes the same inputs produce different plans under an unrecorded policy. The engine applies a pinned, versioned capability profile to closed assessment dimensions.
- **Nested plans or lifecycle-bearing internal domains** — duplicate claims, status, approval, and archive semantics at every level. Domains explain partition and convergence; only terminal domains become ordinary slices under one plan.
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
- **Authoring-scope `--create` / discovery `action: create` / execute-time provisioning** — invents a privileged topology path before survey and decomposition can say where work belongs. Operator-created forge repos with `product:` plus conflict-domain mapping rationale scale without a second create contract.
- **Repository-host access as a third adapter axis or Emery-owned push/PR/merge** — host access is infrastructure, while publication remains operator-owned and RFC-89-defined.

