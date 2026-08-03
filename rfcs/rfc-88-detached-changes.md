# RFC-87: Detached Changes

> Status: Draft — step 2 of the platform-migration series ([platform.md](platform.md)); the complete single-node operator migrate/change story
>
> Owns: the complete single-node detached workflow — the change as the unit of location; the no-state-outlives-the-change invariant; the narrow forge provider used for discovery and repository creation; discovery in **migrate** and **change** modes; deterministic first-party source adapter selection; generated source identities; resolved projects recorded at approval; product-membership criteria; local ephemeral slot population; and greenfield repository provisioning when no project matches.
>
> Depends on completed [RFC-86](rfc-86-working-trees.md) (local slot materialization, read-only source trees, leases, immutable source snapshots, and the value↔tree boundary). Reuses [RFC-71](rfc-71-deployment.md)'s landed pull-on-miss adapter resolution.
>
> Consumed after completion by: [RFC-88](rfc-88-publication-sets.md) (publication identity over the recorded members) and [RFC-91](rfc-91-node-sync.md) (hosted execution of the same change-scoped workflow).

## Intent

Start from a bare directory with thin prior context — forge authentication, organisation, and source material — and run a whole change: discover the repositories that comprise the work, record them (creating repositories when none match), materialize them, execute the plan, publish, finalize — then delete the directory.

**Migrate** and **ongoing change** share this loop. Only discovery criteria differ:

| Job | Discovery finds | Greenfield |
| --- | --------------- | ---------- |
| Migrate a legacy platform | Legacy code and non-code inputs matching migrate criteria; proposes target topology | Primary — target repos often do not exist yet |
| Ongoing change to the migrated platform | Org repositories whose `.emery/project.yaml` declares `product:` (membership; optionally filtered to the change's product ids) | When the change needs a member that does not exist yet |

Emery today anchors multi-repo work in a permanent platform repo (`workspace: true`, committed `registry.yaml`, tended `workspace/<project>/` slots). Detached mode replaces that anchor with the change itself: the change directory is the one self-contained home for coordination state, valid exactly as long as the change is live. This RFC owns the complete intake path from source trees to approved member and target topology; ordinary in-place planning reuses its source-binding grammar and selector without adopting detached coordination.

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | **The change is the unit of location.** A detached change opens as one self-contained directory — plan, journal, slice artifacts, and materialized slots — created at open, valid for the change's duration, deletable after finalize. There is no platform repo and nothing to commit at the coordination level. | "Where does this change live?" has a one-directory answer on any machine. Ephemeral means *disposable after finalize*, not scratch: the directory is the durable-for-the-change home, and deleting it mid-change forfeits orchestration state (approvals, journal, lifecycle position) even though pushed branches survive. |
| D2 | **No Emery state outlives the change.** Durable outcomes land where they already land: baselines in member repos, code and PRs on the forge, identity reconstructible per [RFC-88](rfc-88-publication-sets.md) D3. Post-finalize retention of the change directory (or its archive) is an operator convenience under the existing prune posture, never a requirement. | The lifecycle authority story is unchanged — the CLI remains the single writer over `plan.yaml` and the journal for the change's duration; it just stops pretending those files are the product. Nothing new to back up, replicate, or migrate. |
| D3 | **Projects and sources are discovered, then recorded — before plan authoring.** `emery source discover` queries the forge by mode and criteria, fingerprints shallow read-only trees, applies this RFC's exact-one source filter, and emits an immutable candidate report. Operator approval atomically records the resolved `projects` map, exact revisions, target topology, and generated source bindings in `plan.yaml`; it is the gate for any `create-repository`. `/emery:plan` authors slices only over those bindings. | Dynamic population is not re-querying: re-materializing a half-done change resolves the recorded topology deterministically, and RFC-88 can derive publication members without a registry. |
| D4 | **The registry is removed from detached coordination.** Detached mode authors no `registry.yaml`; membership and location derive from the plan's resolved bindings, and any registry-shaped view is a projection. The committed platform-repo workspace posture and `emery init --workspace` are removed at this hard cut; regular in-place projects remain. | [RFC-88](rfc-88-publication-sets.md)'s member derivation is `plan.yaml` alone, and the forge markers become the only out-of-band reconstruction record. There is one multi-repository coordination model to implement and document. |
| D5 | **Slots are local ephemeral materializations.** Each member populates on demand under an RFC-86 lease, scoped to the change directory and torn down with it — during `plan execute`, not as a pre-plan fleet checkout. The worktree is per change while the host-owned bare mirror is shared. Baseline reads, builds, and merges route into the slot exactly as workspace routing does today. | The slot is a disposable leased worktree, not a checkout the operator tends. No remote placement or hosted lease exists in this RFC. |
| D6 | **Greenfield is a first-class, explicit discovery outcome.** When no repository matches, the candidate report proposes only repositories supplied by `--create`; absent that input, discovery fails with the required flag form. Approval renders ordinary `emery init` output into an initial tree, and the forge provider's only write operation — `create-repository` — creates the repository with that tree as its initial commit and returns the exact revision. Later branch push and PR merge remain operator-owned publication. | Greenfield projects have an immutable RFC-86 base before plan authoring, with no invented repository identity or uncommitted initialization state. The provisioning write stays one approval-gated, journaled operation. |
| D7 | **The journal is change-local and file-backed.** The change directory holds the only journal for the change's lifetime; all re-entry reads it from there. Deleting the directory before finalize forfeits orchestration state. | RFC-87 has no service, replica, remote journal, or durability mode. RFC-91 may host the completed contract later without leaving a phase unfinished here. |
| D8 | **Single-repo in-place mode keeps the same lifecycle.** A repository already carrying `.emery/` that the operator opens as the working directory never routes through discovery or detached materialization; it does adopt D11's keyless source-intake grammar and can reuse the deterministic selector for `plan author --source <path>`. | One workflow, two anchors. Surveying the org *for* repos that declare `product:` (D9 change mode) is discovery of *members*, not in-place single-repo mode. |
| D9 | **Two discovery modes; change-mode membership is `product:`.** **Migrate mode** criteria: org (required), plus target product ids, language / topics / manifest sentinels / path binds for non-code inputs; product ids label proposed targets but do not filter legacy sources. **Change mode** criteria: org (required); a repository is a candidate iff its `.emery/project.yaml` declares a non-empty `product:` list. Optional criteria intersect with requested product ids and may further filter by platforms, adapter name, forge topics, or path globs. Absence of `product:` means not a product member for discovery, even if `.emery/` and `platforms:` exist. | `product` is membership only. The build set remains `platforms:` — `core` plus presentation layers (`ios` / `android` / …), never the discovery key. |
| D10 | **Migrate and ongoing change share one intake and location model.** This RFC owns source-selection facts and filtering, candidate-report shape, target-topology proposals, member-recording approval, and greenfield initialization. | One product loop and one topology gate; ordinary single-project `plan author` reuses selection without gaining detached-change artifacts. |
| D11 | **Source keys are generated, never authored.** Source intake accepts a path, an adapter-qualified path, or an adapter-qualified value; Emery allocates a deterministic non-colliding key and persists it with the binding. The existing `<key>=<adapter>:<binding>` form is removed at this hard cut. | Keys remain visible stable references in plans, Evidence filenames, provenance, and amend commands, but operators copy them rather than design them. The CLI has no key-override grammar or compatibility alias. |
| D12 | **Forge access is a host provider capability, not a third adapter axis.** The engine consumes typed `query-repositories`, `read-repository`, and `create-repository(initial-tree)` operations; the shipped binary binds a GitHub implementation using operator credentials. Push, pull-request, merge, and publication operations are absent. | Discovery and initialized greenfield creation land completely in this RFC without confusing source/target adapter vocabulary or waiting for RM-17. Other forges can implement the same provider later. |

## Discovery and approval contract

The discovery grammar is one command:

```text
emery source discover
  --mode <migrate|change>
  --organization <org>
  --intent <text>
  [--repository <name>]…
  [--product <id>]…
  [--topic <topic>]…
  [--language <language>]…
  [--platform <platform>]…
  [--target-adapter <adapter>]…
  [--marker <relative-path>]…
  [--path-glob <glob>]…
  [--source <path|adapter=path|adapter=value:literal>]…
  [--create <repository>=<private|public>]…
  [--force]
```

All repeatable filters are OR within one flag and AND across flag kinds. In migrate mode `--product` labels proposed target rows and `--platform` / `--target-adapter` constrain topology without filtering legacy candidates; in change mode those flags filter existing members. `--source` uses this RFC's keyless intake grammar for local or value inputs that forge discovery cannot find. `--create` is the only source of greenfield repository names and visibility; the topology judgment may assign those rows but never invent another repository. When discovery needs a missing target and no create specification covers it, it fails with `discovery-greenfield-input-required` and prints the exact flag form. `--intent` is lowered to the ordinary generated `intent` value source and also feeds unresolved target-topology judgment; it never changes deterministic repository filtering or source-adapter selection, and `/emery:plan` does not ask for it again.

`emery source discover` atomically writes one typed report at `.emery/candidate.yaml` and prints that path. The report is review-only input to `emery change approve`; operators never edit it.

```yaml
version: 1
change: migrate-orders
mode: migrate
criteria:
  organization: acme
  products: [orders]

candidates:
  - repository: github.com/acme/legacy-orders
    revision: 7b6e…
    fingerprint: sha256:2da9…
    source-adapter: typescript
    disposition: selected
    reasons: [repository-match, language-match]

projects:
  legacy-orders:
    action: use
    repository: github.com/acme/legacy-orders
    revision: 7b6e…
    product: [orders]
  orders-api:
    action: create
    repository: github.com/acme/orders-api
    product: [orders]
    create:
      visibility: private
    target:
      adapter: omnia
      platforms: [core]

sources:
  intent:
    adapter: intent
    value: Migrate order processing
  legacy-orders:
    adapter: typescript
    project: legacy-orders
    path: .

topology-answer-digest: sha256:9cc4…
```

`candidates[]` records every forge result considered, its exact revision/fingerprint, `selected | excluded` disposition, and closed reason ids. `projects` is the proposed post-approval map: `action: use` requires an existing exact revision; `action: create` requires the fixed greenfield fields and receives its revision during approval. `sources` already carries final generated keys. The report digest is SHA-256 over canonical typed serialization of the whole document; `plan.yaml.candidate-digest` records that digest.

Migrate discovery invokes one generated, schema-gated topology answer after deterministic source selection. Its request contains operator intent, selected source facts, existing project configuration, requested product ids, and the bounded first-party target inventory. Its response contains only proposed project rows:

```yaml
kind: response
projects:
  - repository: github.com/acme/orders-api
    action: create
    product: [orders]
    target:
      adapter: omnia
      platforms: [core]
```

The generated answer schema lives with the change judgment answers and the prompt lives in the change prompt corpus. Existing project rows are immutable inputs; the answer may add target rows only from supplied `--create` specifications and cannot alter repository, revision, product, adapter, platforms, or visibility supplied as facts. Every proposed adapter must come from the supplied inventory, every target row must have a non-empty platform set including `core`, and any missing/unknown field enters the ordinary bounded repair loop then fails discovery if still unresolved. The validated answer is normalized into `candidate.yaml`; no free-form topology prose becomes state.

The bounded target inventory is engine-owned in this cut:

```yaml
- name: omnia
  purpose: general core software, services, libraries, and command-line tools
  platforms: [core]
- name: vectis
  purpose: native application UI and platform shells
  platforms: [core, ios, android]
  requires-any: [ios, android]
- name: contracts
  purpose: OpenAPI, AsyncAPI, and JSON Schema contract artifacts
  platforms: [core]
```

Auto-proposal and `--target-adapter` are limited to these identities. `--platform` values must fit the selected profile; the topology answer cannot widen them. Only adapters present in normalized project rows resolve/install through the ordinary pull-on-miss path during approval. Adding another automatically selectable target requires its publication, an inventory update, and an Emery release; dynamic or third-party target inventory remains RM-21 work.

`emery change approve` takes no topology flags. It reads the fixed report path, verifies its digest and current plan-shell name, performs idempotent `action: create` operations, journals each result, and atomically replaces the shell's empty `projects` / `sources` with the resolved maps plus `candidate-digest`. It refuses an edited report, stale candidate revision, unknown adapter, non-empty `slices`, or an already approved digest. Before approval, `emery source discover --force` may replace the report; after approval, replacement requires empty slices and a fresh report/approval cycle. The command is the only approval surface.

## Detached plan shape

Approval writes one closed, resolved topology into `plan.yaml` before plan authoring:

```yaml
name: migrate-orders
candidate-digest: sha256:8a17…

projects:
  legacy-orders:
    repository: github.com/acme/legacy-orders
    revision: 7b6e…
    product: [orders]
  orders-api:
    repository: github.com/acme/orders-api
    revision: a182…
    product: [orders]
    target:
      adapter: omnia
      platforms: [core]

sources:
  intent:
    adapter: intent
    value: Migrate order processing
  legacy-orders:
    adapter: typescript
    project: legacy-orders
    path: .

slices:
  - name: migrate-order-write-path
    project: orders-api
    sources: [intent, legacy-orders]
```

`emery change open` creates a detached plan shell (`name`, empty `projects` / `sources` / `slices`). Discovery approval fills `candidate-digest`, `projects`, and `sources`; `/emery:plan` then authors review prose and `slices` into that same file without replacing the approved topology. Only when anchored in a detached change directory does `plan author` recognize this typed shell; it requires its name argument to match and refuses any other existing plan as today. In-place project planning retains its existing create/force behavior. Detached `--force` may re-author slices over the same approved topology; replacing topology requires a new discovery and approval. Running `plan execute` remains approval of the fully authored plan.

`projects` replaces detached `registry.yaml`. Its generated map key is the repository basename, disambiguated with the same stable locator digest used for source ids. `repository` is the canonical forge locator and `revision` is the exact approved Git commit. `product` is the resolved membership snapshot. `target` is required for every project referenced by `slices[].project` and omitted for read-only source repositories.

Detached source bindings add optional `project:`. A project-bound source resolves `path` relative to that project's immutable RFC-86 snapshot; a binding without `project` remains a local path or value source. A project key and source key may share text but occupy different maps and types.

Existing initialized repositories supply authoritative target adapter, platforms, and product ids from `.emery/project.yaml`. Migrate discovery runs one schema-gated topology judgment only for unresolved target needs, using operator intent and selected source facts; it may propose new project rows but cannot override existing configuration. `[unknown]` target fields block approval rather than being guessed. Approval atomically persists `projects`, generated `sources`, and the candidate-report digest. Plan authoring may then add slices, whose singular `project` must reference a target-capable row.

This RFC owns the accompanying schema changes: `project.yaml.product` is an optional list of unique kebab-case ids; `emery init --product <csv>` writes it for greenfield projects; plan validation rejects unknown project/source references, a slice targeting a read-only project, and drift between an existing member's approved target fields and its `project.yaml`.

## Source adapter selection

Selection is deliberately a small first-party policy, not an adapter-discovery platform. The engine carries a bounded profile per automatically selectable source adapter:

```yaml
name: typescript
extensions: [ts, tsx, mts, cts, js, jsx, mjs, cjs]
markers: [package.json, tsconfig.json]
```

The first inventory contains `typescript`, `documentation`, `screenshots`, and `captures`; value-only `intent` remains explicit. A profile describes evidence sufficient to select an adapter, not every file that adapter can read. Adding or changing a first-party profile requires the corresponding adapter publication and an Emery release.

The profile lives with the engine inventory in this cut. It does not grow the WIT metadata contract, install components merely to inspect metadata, or add `adapters.yaml`, a registry query, trust policy, profile command, or recommendation artifact. Dynamic third-party inventory and publisher trust remain RM-21 work.

For each granted read-only source tree, Emery computes one ephemeral syntactic fingerprint: normalized relative paths, basenames, extension counts, root markers, and total regular-file count. It follows no symlink outside the granted root and ignores Git and Emery control trees. An adapter is eligible when a declared marker is present or its declared extensions account for a strict majority of regular files.

Selection proceeds only when exactly one profile is eligible:

- **one** — resolve that adapter through the existing local-first/pull-on-miss path and lower it to an ordinary `SourceBinding`;
- **zero** — for direct local intake, fail before any plan write with `source-adapter-no-match`; during forge discovery, record `disposition: excluded` / `reason: source-no-match` and continue, failing `discovery-no-match` only when no candidate remains;
- **more than one** — fail the direct intake or whole discovery with `source-adapter-ambiguous`, showing the repository/locator, eligible names, and explicit adapter form.

There is no ranking, tie-break, model fallback, intermediate report, or separate approval. An explicit adapter bypasses matching but not RFC-86's source-tree safety gates. Survey remains the semantic judgment leg.

## Source intake and generated keys

The repeatable `plan author --source` grammar is:

| Form | Meaning |
| ---- | ------- |
| `--source <path>` | Infer the adapter. |
| `--source <adapter>=<path>` | Use an explicit adapter. |
| `--source <adapter>=value:<literal>` | Use an explicit value-bound adapter. |

`--intent <text>` remains sugar for a single value-bound `intent` source. A token with no `=` is a path. With `=`, the left side must parse as an adapter; everything after the separator is the path or `value:` payload. Prefix a path whose first component itself looks like `<adapter>=` with `./`. The removed `<key>=<adapter>:<binding>` shape fails with `source-key-authored` (exit 2) and the equivalent keyless command; it is not a compatibility alias.

Key allocation runs over the complete input set before persistence:

1. For a path binding, derive a base by removing a terminal `.git`, converting the locator's final path segment to lowercase kebab-case, and falling back to `source` when no characters remain. For a value binding, use the adapter name.
2. Keep the base when it is unique.
3. On collision, append the shortest collision-free prefix (minimum eight lowercase hex characters) of SHA-256 over the selected adapter plus canonical identity bytes: project forge locator + normalized relative path for project sources, canonical local path for local sources, or value bytes.
4. Reject the same canonical source locator or identical adapter/value pair more than once with `source-binding-duplicate` (exit 2) rather than manufacturing two identities for one input.

The generated key is immutable for the life of the authored plan. Downstream source references use the persisted value; they never recompute it from a moved tree or changed binding. Discovery applies the same allocation after approval closes the member set, so local and forge intake cannot drift.

## Fixed implementation cut

- The command is `emery change open <dir>`; `emery init --detached` is not an alias.
- The approval command is `emery change approve`; it reads `.emery/candidate.yaml` and accepts no inline topology edits or alternate report path.
- Discovery criteria are structured flags only. Both modes require organisation and intent and accept optional product ids, repository, topic, language, platform, target-adapter, marker, path-glob, explicit source inputs, and explicit create specifications; product ids label targets in migrate mode and filter members in change mode.
- Discovery refuses more than 100 forge candidates with `discovery-too-broad` and asks for narrower criteria; it never truncates, ranks, or sends an unbounded organisation inventory to a model.
- Candidate reason ids are closed: `repository-match`, `repository-mismatch`, `product-match`, `product-mismatch`, `topic-match`, `topic-mismatch`, `language-match`, `language-mismatch`, `platform-match`, `platform-mismatch`, `target-adapter-match`, `target-adapter-mismatch`, `marker-match`, `marker-missing`, `path-match`, `path-missing`, and `source-no-match`. Selected rows carry every positive reason that admitted them; excluded rows carry the first deterministic mismatch in the CLI criteria order.
- Product ids are free kebab-case strings. A repository declaring any requested id is a member; requesting no ids matches every repository with a non-empty `product:` list.
- Change-local worktrees live under the change directory and use RFC-86's shared host mirror. Removing the change directory removes no mirror or forge state.
- Greenfield creation requires explicit `--create <repository>=<private|public>` under the required organisation. The forge default branch is accepted; license, branch-protection, team, and policy setup remain operator-owned.
- GitHub repository creation receives the preservation-safe `emery init` tree and commits it as the default branch's initial revision within the one `create-repository` operation. Approval records the returned revision.
- A candidate report is immutable input to one approval operation. Approval writes the closed project set, resolved target topology, and generated source bindings atomically; partial approval is a new discovery run, not mutation of the old report.
- Approval is resumable across forge side effects: each create uses a stable `(change, project)` idempotency key, its returned repository/revision is journaled, and only then does one atomic `plan.yaml` write publish the complete approved topology. Re-entry resumes missing creates or the final write without duplicating repositories.

## Lifecycle sketch

```text
emery change open <dir>                 # bare directory becomes the change home
emery source discover --mode migrate|change --organization … --intent … [filters]
                                        # forge query + fingerprint/source selection
                                        # → immutable candidate report
                                        # (report may propose create-repository)
emery change approve                    # records projects, revisions, topology, sources;
                                        # journals initialized create-repository for greenfield
/emery:plan                             # author slices over recorded projects only
emery plan execute                      # slots materialize on demand under leases;
                                        # refine → build → merge per entry
                                        # operator publishes (push, PRs, merge)
/emery:finalize                         # operator confirms publication, archive
rm -rf <dir>                            # nothing of record is lost
```

## Rejected alternatives

- **Permanent platform repo** (current workspace mode) — a durable coordination anchor for inherently change-scoped state; forces registry tending and slot hygiene between changes.
- **Durable out-of-tree change store** (`~/.emery/changes/…`) — recreates the platform repo one directory over; still state to back up and migrate.
- **Committing coordination state into a member repo** — pollutes members with cross-repo state and makes membership circular.
- **Re-resolving discovery at materialization time** — non-deterministic membership; recorded bindings exist so a half-done change re-materializes exactly.
- **Plan first, then discover/create** — leaves plan authoring without a closed member set; greenfield and recorded bindings must precede `/emery:plan`.
- **Membership = mere presence of `.emery/project.yaml`** — too broad; single-repo or non-platform Emery projects would be swept into every change-mode survey. D9 requires a declared `product:` field.
- **Membership = `platforms:`** — conflates the closed build set with product membership; rejected in favour of the `platforms` / `product` split.
- **Renaming the build set away from `platforms`** — unnecessary once membership uses `product:`.
- **A separate adapter-selection approval/report before discovery** — duplicates the candidate report and member-recording gate; selection is an inline deterministic leg of intake.
- **Full publication autonomy** (push / PR merge verbs) — collapses the operator-owned publication boundary that every other RFC preserves.

## Phased delivery

- **Phase A — Detached change home.** `emery change open`, change-scoped `plan.yaml` / journal / slice artifacts in the change directory; manual member binding (operator supplies repo URLs); ephemeral slots over [RFC-86](rfc-86-working-trees.md).
- **Phase B — Source intake, topology, and forge discovery.** Add the bounded first-party selector profiles, deterministic fingerprint/exact-one kernel, generated source/project ids, keyless `plan author --source` hard cut, `projects` / project-bound-source schemas, `project.yaml.product`, generated topology answer, typed `.emery/candidate.yaml`, and `emery change approve`; then `emery source discover --mode migrate|change` → immutable report → resolved existing-project topology (D3, D9–D12).
- **Phase C — Greenfield and hard cut.** Render `emery init --product` into an initial tree, add the forge provider's approval-gated initialized `create-repository`, record its returned revision, remove `emery init --workspace` and committed-registry coordination, and complete the single-node migrate/change loop (D4, D6, D12).

## Acceptance criteria

1. Local and shallow forge trees produce the same fingerprint and selector result; TypeScript, documentation, screenshot, capture, no-match, and ambiguous fixtures are covered through public integration surfaces.
2. Inference resolves only the selected adapter. Explicit adapter bindings bypass matching, while both paths pass RFC-86's canonicalization, read-only grant, symlink, and overlap gates.
3. Generated keys are deterministic under argument reordering, remain unchanged for unique basenames, disambiguate equal basenames without counters, and reject duplicate inputs. The removed authored-key grammar produces its targeted hard-cut diagnostic.
4. Discovery approval atomically records the candidate digest, closed `projects` map, exact revisions, target topology, and generated source bindings before plan authoring; re-entry never re-queries membership.
5. The GitHub forge provider queries and reads repositories without writing; `create-repository(initial-tree)` is available only through approval, requires every fixed input, creates the initialized default-branch commit, returns its exact revision, and is journaled as one provisioning operation.
6. `plan.yaml.projects`, project-bound sources, `project.yaml.product`, and `slices[].project` validate as one referential graph; existing project configuration wins and unresolved target topology blocks approval.
7. `candidate.yaml` round-trips through its public typed schema; its canonical digest changes on any semantic field edit; stale revisions, invalid topology answers, alternate adapters, and approval after slices exist fail before plan mutation or duplicate forge writes.
8. A complete single-node run opens a bare change directory, discovers or creates projects, approves, authors, executes through local ephemeral slots, survives process re-entry, confirms publication through the existing finalize flow, and leaves no required Emery state after directory deletion.
9. `emery init --workspace`, committed detached registries, remote journals, hosted leases, and multi-node placement are absent from the shipped surface.
10. `cargo make ci` is green with crate-level integration coverage for source intake, generated ids, discovery/report schema, topology repair, approval/idempotency, greenfield creation, materialization, re-entry, and finalization.
