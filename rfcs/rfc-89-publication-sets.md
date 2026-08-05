# RFC-89: Publication Sets

> Status: Draft — step 4 of the critical path of the platform-migration series ([platform.md](platform.md)).
>
> Owns: the publication set — the durable identity that binds one change's final project snapshots, sealed commits, branches, and pull requests across every repository it touches: member derivation from `plan.yaml`, one local project seal per member, branch and PR marker conventions, publication-order declaration, verification at archive, and the machine-readable publication projection external producers can emit.
>
> Depends on completed [RFC-88](rfc-88-detached-changes.md), which records the member set and supplies the forge provider this RFC extends with publication reads. Related: [RFC-92](rfc-92-node-sync.md) coordinates one change's execution across nodes before publication. First external producer: RFC-81 (`augentic/remedium` `rfcs/rfc-81-cloud-alert-remediation.md`).
>
> Defers: performing pushes / PR creation / PR merges (publication stays operator-owned); atomic cross-repo submission; cross-checkout CI; parallel member preparation ([RFC-91](rfc-91-concurrent-execution.md) D4).

## Intent

Promote the change — not the repository — to the unit of coordination when one change spans several repositories, from a backend/frontend pair to an 80-repo platform.

One-line summary:

```text
change-id → { repo₁: sealed commit + branch + PR, repo₂: sealed commit + branch + PR, … } + declared order;
Emery seals locally, derives, tracks, and verifies — the operator publishes and the forge merges.
```

Every serious multi-repo coordination system is an instance of this one pattern (Gerrit topics, Zuul `Depends-On`, Sourcegraph Batch Changes). After RFC-88, Emery already has most of the object: a plan is a named change whose slices bind recorded `plan.yaml.projects` rows and are ordered by `depends-on`. What is missing is the publication half — nothing seals each project's final snapshot into one committable result, records which branches and PRs constitute the change on the forge, declares the order they must land in, or lets `emery plan archive` verify that publication happened.

## Background

### Before the critical-path hard cut

- The current workspace implementation resolves each singular `slices[].project` through `registry.yaml` into a tended `workspace/<project>/` slot. RFC-88 removes that path before this RFC starts and replaces it with `plan.yaml.projects` plus operation-local RFC-87 workspaces.
- `depends-on` orders plan entries; the existing `plan author` validation tail rejects `cycle-in-depends-on`.
- Publication is operator-owned by contract ([cli-contract](../docs/standards/cli-contract.md)). D11 makes one narrow hard-cut change: Emery seals the final project snapshot into a local commit and branch; pushing that branch and completing the pull request remain operator-owned.
- `emery plan archive` closes the workflow. Today it relies on operator confirmation of publication, but that confirmation is prose — there is no verification.

### Vocabulary

`emery slice merge` folds delta specs into the baseline — a lifecycle transition, not a git operation. This RFC is about **publication**: the branches, pull requests, and forge merges that carry a change's diffs into member repositories. Its **publication set** is a forge-side record, distinct from RFC-87's **code patch** (the relation between base and result snapshots). *Merge* stays lifecycle; *publish* and *land* are forge-side.

### The gap

When a change touches three repositories, the fact "these three PRs are one change" lives nowhere durable. The operator carries it in their head; archive cannot check it; an agent or a second operator cannot reconstruct it; and Remedium (RFC-81) is about to need the identical object for alert-remediation proposals that span repos. Meanwhile the [roadmap's cross-repo coordination note](roadmap.md#cross-repo-coordination) and [RFC-77](rfc-77-release-process.md)'s WIT-breaking release shape describe ordered multi-repo landings with no mechanism behind the ordering — a human checklist.

### Prior art

| Model | Shape | Verdict for Emery |
| ----- | ----- | ----------------- |
| [Gerrit topics](https://gerrit-review.googlesource.com/Documentation/cross-repository-changes.html) | Shared topic; `submitWholeTopic` merges the set together | The right *identity* model; atomic submission has no GitHub equivalent — do not emulate |
| Zuul `Depends-On` | Commit trailer declares cross-repo dependencies; CI tests the union, gate enforces order | The right *ordering* model; trailer-in-PR convention adopted here, CI union deferred |
| Sourcegraph Batch Changes | Declarative spec → N tracked changesets, dashboard to done | The right *tracking* model; an external system owning workflow state violates "CLI authoritative for workflow state" |
| Monorepo | One repo, atomic commits | Not available: org boundaries, independent lifecycles, 80-repo platforms that exist already |
| Meta-repo / submodules | Pointer repo pins member SHAs | Widely regretted; couples clone/CI mechanics to solve an identity problem |

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | **The plan is the publication set.** Change identity = plan name; publication members = the distinct target-capable `slices[].project` references. Approved read-only or unused projects are not publication members. | No new lifecycle noun, no second lifecycle authority, no parallel artifact beside `plan.yaml`. A single-repository plan is the degenerate one-member publication set. |
| D2 | The publication record is **derived, never authored**: member identity from `slices[].project`, repository location and initial revision from the resolved RFC-88 project projection (`plan.yaml.projects` plus any greenfield provisioning receipt), the sealed commit from D11's fact, and remote refs and publication status from the forge. | One authored home per fact. There is no registry bridge and no publication-set state committed into member repositories. |
| D3 | Branch convention `change/<plan>` in every member repository; every member PR carries the trailer `Emery-Change: <plan>` in its body. | The set is reconstructible from the forge alone — no Emery state required. Operators and external tools mark; Emery reads. A forge label may mirror the trailer; the trailer is authoritative. |
| D4 | Publication order **derives from `depends-on`** projected onto projects (topological order of the entries' project bindings). No second ordering surface. | The order the work was planned in is the order it lands in. Cross-project `depends-on` edges become meaningful beyond scheduling; unordered members may land in any order. |
| D5 | Emery **tracks and verifies; it never pushes, opens, or merges PRs.** `plan archive` upgrades from prose confirmation to verification: every member PR exists, is merged, and landed in an order consistent with D4. | Preserves the cli-contract's operator-owned publication boundary. The gate is observation, not action. |
| D6 | Promise **coordinated convergence, not atomicity.** No `submitWholeTopic` emulation; contract movement inside a publication set uses expand/contract expressed as ordered members. | An out-of-order landing is a journaled finding at verification, not a rollback — merged PRs cannot be unwound. Deployment-compatibility discipline stays with the operator and the target adapters. |
| D7 | `emery plan archive` derives a typed publication projection before it mutates state: members, sealed commit ids, branch refs, PR refs, per-member publication status, derived order, and verification verdict. | Archive renders the verification result and fails before mutation when publication is incomplete. The projector remains reusable internally and by external producers without adding a CLI subcommand. |
| D8 | External producers emit the **same record shape** for non-plan changes. | A Remedium remediation (RFC-81) spanning N repos is a publication set with no plan behind it — same trailer, same projection schema, no Emery lifecycle claim. The object is shared; the authority is not. |
| D9 | Member publication checks run serially over RFC-88's resolved plan bindings and forge state; they do not materialize workspaces. | The read-only projection is deterministic and complete without coupling publication verification to a checkout. |
| D10 | Forge reads extend RFC-88's host forge provider with `find-pull-request` and `read-pull-request`; the shipped GitHub binding implements both in this RFC. | There is no interim `gh` probe and no dependency on RM-17. No forge write capability is introduced. |
| D11 | **One project seal creates the committable result.** When every entry for a project has crossed the serial merge gate, Emery creates one local Git commit whose parent is the execution-recorded initial project revision and whose tree is the final accepted project snapshot — code and merged baseline together, since [RFC-88](rfc-88-detached-changes.md) folds the baseline inside the workspace — updates local `change/<plan>`, and records the binding as a fact. | Workers never commit and intermediate snapshots never become repository history. Every publication member has exactly one committable result before the operator pushes anything. |

## Project seal

The project seal is the boundary between convergence and publication:

```text
recorded initial project revision + final accepted snapshot
    → one local commit on change/<plan>
    → plan.publication.project-sealed fact
    → operator pushes the branch and completes its PR
```

The seal runs once when a project's entries are drained. It creates the commit directly from the immutable tree in the host-owned Git object store; it does not materialize a workspace or touch the operator's checkout. The tree needs no assembly: RFC-88's accepted snapshot already carries both the code and the merged `.emery/specs/` baseline, so the seal never synthesizes content that no snapshot verified. Commit author, message, and timestamp derive deterministically from the plan and its execution approval fact, so the recorded parent plus final tree always produces the same commit id. The fact records the project, parent revision, final snapshot id, commit id, and branch.

This is preparation for publication, not forge publication: Emery creates no remote ref, PR, merge, or revert.

## The publication projection

Illustrative typed projection derived by `emery plan archive` before mutation (standard envelope omitted):

```json
{
  "change": "checkout-v2",
  "members": [
    {
      "project": "payments-api",
      "repository": "github.com/example/payments-api",
      "commit": "8e43c…",
      "branch": "change/checkout-v2",
      "pull-request": "https://github.com/example/payments-api/pull/412",
      "publication": "merged",
      "order": 1
    },
    {
      "project": "web-frontend",
      "repository": "github.com/example/web-frontend",
      "commit": "c71a9…",
      "branch": "change/checkout-v2",
      "pull-request": "https://github.com/example/web-frontend/pull/98",
      "publication": "open",
      "order": 2
    }
  ],
  "verification": "pending — web-frontend unmerged"
}
```

`publication` is a closed enum: `unpublished | open | merged | closed`. `order` is the D4 derivation; members with no cross-project `depends-on` path between them share no ordering constraint.

## Archive verification

`emery plan archive` derives and renders the publication projection before its existing archive transition:

1. Reconstruct the member set (D2) and read forge state (D10).
2. Verify: every member PR merged; landing order consistent with D4; every member PR carries the `Emery-Change` trailer.
3. Render the card. On full verification archive proceeds. On failure, archive halts with the standard error envelope (`publication-unverified`, exit 1) naming each failing member.

An explicit `--unverified` escape hatch keeps publication operator-owned — the operator can archive over a red card, and the override is journaled (`plan.publication.unverified-archive`). Verification is a gate with a named bypass, not a hard wall.

## First dogfood: seam movement

The [roadmap's cross-repo coordination note](roadmap.md#cross-repo-coordination) fixes steady state: four repos coordinated only through versioned WIT seams, never a lockstep release. It is silent on the *transition moment* when a seam itself moves. RFC-77's WIT-breaking shape — publish `emery:adapter@…`, engine publish, adapters bump pin and ship — is exactly a publication set with a declared order across `augentic/emery` and `augentic/emery-adapters`. That release shape is the first in-house use, replacing checklist discipline with a verifiable record. This complements the steady-state principle; it does not amend it.

## Relationship to existing RFCs

- **[RFC-86](rfc-86-change-facts.md)** — supplies the recorded approval this RFC's verification closes the loop on: archive can state not only that every member PR landed in order, but that what landed traces to approved artifact digests. Publication facts (`plan.publication.*`) ride the per-actor logs like every other event.
- **[RFC-88](rfc-88-detached-changes.md)** — records the member set; its D4 demotes the committed registry, simplifying this RFC's member derivation to `plan.yaml` alone and making the forge markers (D3 here) the only out-of-band record. Migrate and ongoing change share that location model; this RFC binds their publication.
- **[RFC-87](rfc-87-working-trees.md)** — each member operation uses a disposable private workspace and returns an immutable result without committing. D11 seals the final project snapshot once, after the workspaces are gone.
- **[RFC-77](rfc-77-release-process.md)** — the WIT-breaking coordination order becomes the first publication set (above). No change to its decisions.
- **[RFC-91](rfc-91-concurrent-execution.md)** — scale track; its D4 concurrency substrate is what would later let one publication set's members build in parallel (D9).
- **RM-17 / RM-20 ([roadmap](roadmap.md))** — RM-17 may extend the settled forge provider with publication handoff or additional forges; RFC-89 does not wait for it. RM-20 ("catalog-backed initiatives across many repositories") gains its coordination semantics from this RFC rather than defining its own.
- **RFC-81 (`augentic/remedium`)** — first external producer (D8): a correlated alert whose fix spans repos emits one publication-set record; its propose stage's "draft PR" becomes "publication-set member, usually singular."

## Phased delivery

### Phase A — Identity and projection

1. Implement the idempotent project seal over the final accepted snapshot, local `change/<plan>` ref, and `plan.publication.project-sealed` fact.
2. Document the `change/<plan>` branch and `Emery-Change` trailer conventions in the operator docs.
3. Implement the typed publication projector used by `emery plan archive`, reading members and locations from RFC-88's resolved `plan.yaml` bindings, sealed commits from facts, and forge state through the extended GitHub provider.
4. Journal events: `plan.publication.projected`, `plan.publication.member-landed`.

### Phase B — Verification and shared record

1. Derive publication order from `depends-on` (D4) and render it in the projection.
2. Add the verification card and `publication-unverified` gate to `emery plan archive`, with the journaled `--unverified` bypass.
3. Publish the publication-set record schema so external producers emit conforming records, and validate imported records through the same projector.
4. Exercise the WIT-breaking release shape over fixture repositories as the completion case. The first real release then dogfoods the settled path without gating RFC completion.

## Rejected alternatives

**Monorepo consolidation.** Answers a different question, and is unavailable for platforms whose repo boundaries mirror org boundaries. Coordination must work over the fleet that exists.

**Meta-repo / submodules.** Pins member SHAs to solve an identity problem, and couples clone, CI, and review mechanics in the process. The publication set needs a name and a member list, not a super-repository.

**Multi-project slices.** Letting one slice bind several projects breaks the singular `slices[].project` schema and the synthesis kernel's per-project header derivation—for no gain, since the plan already spans projects across slices.

**`submitWholeTopic` emulation.** GitHub has no atomic cross-repo merge; simulating one (merge-all-or-revert) would require Emery to perform merges and reverts, violating D5 and the cli-contract. Convergence with declared order is the honest contract.

**External system as record owner (Sourcegraph Batch Changes, or a Remedium-owned store).** The tracking model is right; the ownership is wrong. Workflow state authority stays in the CLI; external systems may *emit* publication-set records (D8), never own the plan-backed ones.

**Registry as the publication-set home.** RFC-88 removes the detached registry. A publication set is per-change state derived from `plan.yaml` and the forge; reintroducing a registry would create a second member authority.

## Non-goals

- Pushing branches, opening PRs, or merging PRs from Emery — D11 stops at the local sealed commit and branch.
- Atomic cross-repo submission or automated rollback of landed members.
- Changing `slices[].project` cardinality or any slice lifecycle transition.
- A second ordering surface beside `depends-on`.
- Forge-side platform grouping (GitHub custom properties, topics) — complementary membership metadata, out of scope here.
- Replacing RFC-88's detached location and source-selection model.
- A version solver or cross-repo dependency resolution.
- Cross-checkout CI or speculative union testing — [RFC-92](rfc-92-node-sync.md)'s trial-integration gate owns pre-merge composition.

## Acceptance criteria

1. Draining a project seals its final snapshot into exactly one local commit whose parent is the recorded initial revision, updates local `change/<plan>`, and records an idempotent fact without touching the operator checkout or forge.
2. `emery plan archive` derives every member and repository location from RFC-88 plan bindings, each sealed commit from its fact, and branch/PR state through the forge provider before changing archive state.
3. The projection is byte-stable for unchanged facts, plan, and forge state; a single-repository plan produces the same schema with one member and one sealed commit.
4. Publication order derives only from `depends-on`; unordered members carry no invented order.
5. Archive verifies trailers, merged state, and landing order; failures name each member and `--unverified` archives only with its journal event.
6. External records validate against the published schema and project through the same read surface without acquiring plan lifecycle authority.
7. A WIT-breaking engine/adapter release fixture is represented and verified as a multi-member publication set.
8. `cargo make ci` is green with integration coverage for project sealing, one/many members, missing/open/closed/merged PRs, ordering, trailer mismatch, provider failure, external records, and the unverified bypass.

## References

- [Gerrit — submitting changes across repositories by using topics](https://gerrit-review.googlesource.com/Documentation/cross-repository-changes.html)
- Zuul `Depends-On` cross-repository dependencies
- [Sourcegraph Batch Changes](https://sourcegraph.com/docs/batch-changes)
- [roadmap.md — cross-repo coordination](roadmap.md#cross-repo-coordination) · [RM-17 / RM-20](roadmap.md#rm-17-operator-owned-forge-integration)
- [RFC-88 Detached Changes](rfc-88-detached-changes.md) · [RFC-87 Private Workspaces](rfc-87-working-trees.md) · [RFC-77 Release Process](rfc-77-release-process.md) · [RFC-91 Concurrent Execution](rfc-91-concurrent-execution.md)
- RFC-81 Cloud Alert Remediation Platform (`augentic/remedium` `rfcs/rfc-81-cloud-alert-remediation.md`)
- [cli-contract — operator-owned publication](../docs/standards/cli-contract.md)

## Review ask

Confirm D1–D11: the plan is the publication set (no new lifecycle noun); the record is derived, never authored; `change/<plan>` branches and `Emery-Change` PR trailers make the set forge-reconstructible; publication order derives from `depends-on`; Emery seals locally, tracks, and verifies but never performs forge writes; convergence not atomicity; archive-time projection with no new CLI subcommand; the same record shape for external producers; serial reads; forge state through RFC-88's provider.

The decisions that must not be deferred are D5 and D11: Emery owns one local committable result per project, while the operator still owns every forge write.
