# RFC-91: Cross-Repo Changesets

> Status: Draft — nothing landed. Formerly RFC-82; renumbered as step 7 of the platform-migration series ([next-stage.md](next-stage.md)).
>
> Owns: the changeset — the durable identity that binds one change's branches and pull requests across every repository it touches: member derivation from `plan.yaml`, branch and PR marker conventions, publication-order declaration, verification at finalize, and the machine-readable changeset projection external producers can emit.
>
> Complements: [RFC-85](rfc-85-migration-program.md) (the orthogonal axis — N repos, N independent changes; this RFC is one change, N repos) and [RFC-89](rfc-89-node-sync.md) (that RFC coordinates one change's *execution* across nodes; this one binds its *publication* across repositories). Consumes: RM-17 forge adapter verbs when they land. First external producer: RFC-81 (`augentic/remedium` `rfcs/rfc-81-cloud-alert-remediation.md`).
>
> Defers: performing pushes / PR creation / PR merges (publication stays operator-owned); atomic cross-repo submission; cross-checkout CI; managed workspace slots ([RFC-86](rfc-86-working-trees.md)); parallel member preparation ([RFC-88](rfc-88-concurrent-execution.md) D4).

## Intent

Promote the change — not the repository — to the unit of coordination when one change spans several repositories, from a backend/frontend pair to an 80-repo platform.

One-line summary:

```text
change-id → { repo₁: branch + PR, repo₂: branch + PR, … } + declared order;
Emery derives, marks, tracks, and verifies — the forge merges.
```

Every serious multi-repo coordination system is an instance of this one pattern (Gerrit topics, Zuul `Depends-On`, Sourcegraph Batch Changes). Emery already has most of the object: a plan is a named change whose slices each bind a registry project, ordered by `depends-on`. What is missing is the publication half — nothing records which branches and PRs constitute the change on the forge, nothing declares the order they must land in, and `/emery:finalize` takes the operator's word that publication happened.

## Background

### What already exists

- A plan spans projects: each `slices[]` entry carries a singular `project`, resolved through `registry.yaml` (membership and location only) into a `workspace/<project>/` slot.
- `depends-on` orders plan entries; `plan validate` already rejects `cycle-in-depends-on`.
- Publication is operator-owned by contract ([cli-contract](../docs/standards/cli-contract.md)): "Materializing slots, preparing branches, committing, publishing, and completing pull requests are operator-owned operations outside Emery."
- `/emery:finalize` confirms publication is complete, then `emery plan archive` sweeps the plan. The confirmation is prose — there is no verification.

### Vocabulary

`emery slice merge` folds delta specs into the baseline — a lifecycle transition, not a git operation. This RFC is about **publication**: the branches, pull requests, and forge merges that carry a change's diffs into member repositories. The two never share a word here: *merge* stays lifecycle; *publish* and *land* are forge-side.

### The gap

When a change touches three repositories, the fact "these three PRs are one change" lives nowhere durable. The operator carries it in their head; finalize cannot check it; an agent or a second operator cannot reconstruct it; and Remedium (RFC-81) is about to need the identical object for alert-remediation proposals that span repos. Meanwhile the [roadmap's cross-repo coordination note](roadmap.md#cross-repo-coordination) and [RFC-77](rfc-77-release-process.md)'s WIT-breaking release shape describe ordered multi-repo landings with no mechanism behind the ordering — a human checklist.

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
| D1 | **The plan is the changeset.** Change identity = plan name; members = the distinct `slices[].project` values. | No new lifecycle noun, no second lifecycle authority, no parallel artifact beside `plan.yaml`. A single-repo plan is a one-member changeset — the degenerate case costs nothing. |
| D2 | The changeset record is **derived, never authored**: members from `plan.yaml`, locations from `registry.yaml`, refs and publication status from the forge. | One authored home per fact (roadmap principle). The registry stays membership/location only; no changeset state is committed into member repositories. |
| D3 | Branch convention `change/<plan>` in every member repository; every member PR carries the trailer `Emery-Change: <plan>` in its body. | The set is reconstructible from the forge alone — no Emery state required. Operators and external tools mark; Emery reads. A forge label may mirror the trailer; the trailer is authoritative. |
| D4 | Publication order **derives from `depends-on`** projected onto projects (topological order of the entries' project bindings). No second ordering surface. | The order the work was planned in is the order it lands in. Cross-project `depends-on` edges become meaningful beyond scheduling; unordered members may land in any order. |
| D5 | Emery **tracks and verifies; it never pushes, opens, or merges PRs.** Finalize upgrades from prose confirmation to verification: every member PR exists, is merged, and landed in an order consistent with D4. | Preserves the RFC-85 non-goal ("Moving publication / PR merge into Emery") and the cli-contract's operator ownership. The gate is observation, not action. |
| D6 | Promise **coordinated convergence, not atomicity.** No `submitWholeTopic` emulation; contract movement inside a changeset uses expand/contract expressed as ordered members. | An out-of-order landing is a journaled finding at verification, not a rollback — merged PRs cannot be unwound. Deployment-compatibility discipline stays with the operator and the target adapters. |
| D7 | `emery plan changeset [--format json]` is the read-only projection: members, branch refs, PR refs, per-member publication status, derived order, verification verdict. | Machine-readable surface for `/emery:finalize`, agents, and dashboards. Schema is versioned and owned by this RFC. |
| D8 | External producers emit the **same record shape** for non-plan changes. | A Remedium remediation (RFC-81) spanning N repos is a changeset with no plan behind it — same trailer, same projection schema, no Emery lifecycle claim. The object is shared; the authority is not. |
| D9 | Serial member preparation in this cut, operator-prepared slots. | Matches RFC-85 C6 posture. Parallel preparation waits on [RFC-86](rfc-86-working-trees.md) slots and [RFC-88](rfc-88-concurrent-execution.md) D4 concurrency. |
| D10 | Forge reads go through the RM-17 forge adapter when it lands; until then a thin `gh`-backed read-only probe. | No forge write capability is introduced by this RFC in any phase. |

## The changeset projection

Illustrative `emery plan changeset --format json` body (standard envelope omitted):

```json
{
  "change": "checkout-v2",
  "members": [
    {
      "project": "payments-api",
      "repository": "github.com/example/payments-api",
      "branch": "change/checkout-v2",
      "pull-request": "https://github.com/example/payments-api/pull/412",
      "publication": "merged",
      "order": 1
    },
    {
      "project": "web-frontend",
      "repository": "github.com/example/web-frontend",
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

## Finalize verification

`/emery:finalize` today asks the operator to confirm publication and runs `emery plan archive`. With the changeset projection it gains a verification card before the confirmation gate:

1. Reconstruct the member set (D2) and read forge state (D10).
2. Verify: every member PR merged; landing order consistent with D4; every member PR carries the `Emery-Change` trailer.
3. Render the card. On full verification the confirmation proceeds as today. On failure, finalize halts with the standard error envelope (`changeset-unverified`, exit 1) naming each failing member.

An explicit `--unverified` escape hatch keeps publication operator-owned — the operator can archive over a red card, and the override is journaled (`plan.changeset.unverified-archive`). Verification is a gate with a named bypass, not a hard wall.

## First dogfood: seam movement

The [roadmap's cross-repo coordination note](roadmap.md#cross-repo-coordination) fixes steady state: four repos coordinated only through versioned WIT seams, never a lockstep release. It is silent on the *transition moment* when a seam itself moves. RFC-77's WIT-breaking shape — publish `emery:adapter@…`, engine publish, adapters bump pin and ship — is exactly a changeset with a declared order across `augentic/emery` and `augentic/emery-adapters`. That release shape is the first in-house use, replacing checklist discipline with a verifiable record. This complements the steady-state principle; it does not amend it.

## Relationship to existing RFCs

- **[RFC-85](rfc-85-migration-program.md)** — orthogonal axes, both kept: the program is N repos × N independent changes, repository-at-a-time; a changeset is one change × N repos. D5 preserves RFC-85's "no PR merge in Emery" non-goal by making the changeset track-and-verify only. The previously unstated assumption that one plan's publication is one repository's PR is now explicit — and relaxed.
- **[RFC-86](rfc-86-working-trees.md)** — unchanged but under new pressure: a three-member changeset wants three prepared slots. Operator-prepared slots suffice for this cut; changesets become the second motivation (after migration) for pulling in managed materialization.
- **[RFC-77](rfc-77-release-process.md)** — the WIT-breaking coordination order becomes the first changeset (above). No change to its decisions.
- **[RFC-88](rfc-88-concurrent-execution.md)** — untouched now; its D4 concurrency substrate is what would later let one changeset's members build in parallel (D9).
- **[RFC-90](rfc-90-detached-changes.md)** — its D4 demotes the committed registry, simplifying this RFC's member derivation to `plan.yaml` alone and making the forge markers (D3 here) the only out-of-band record.
- **RM-17 / RM-20 ([roadmap](roadmap.md))** — RM-17's forge adapter is the read surface D10 wants (PR state, mergeability, merged-state verification). RM-20 ("catalog-backed initiatives across many repositories") gains its coordination semantics from this RFC rather than defining its own.
- **RFC-81 (`augentic/remedium`)** — first external producer (D8): a correlated alert whose fix spans repos emits one changeset record; its propose stage's "draft PR" becomes "changeset member, usually singular."

## Phased delivery

### Phase A — Identity and projection

1. Document the `change/<plan>` branch and `Emery-Change` trailer conventions in the operator docs and finalize skill body.
2. Implement the derived changeset projection (`emery plan changeset`), reading member sets from `plan.yaml` + `registry.yaml` and forge state through a read-only `gh` probe.
3. Journal events: `plan.changeset.projected`, `plan.changeset.member-landed`.

### Phase B — Verification at finalize

1. Derive publication order from `depends-on` (D4) and render it in the projection.
2. Add the verification card and `changeset-unverified` gate to `/emery:finalize`, with the journaled `--unverified` bypass.
3. Use the WIT-breaking release shape as the first dogfood changeset.

### Phase C — Shared schema and CI union (only when pulled)

1. Publish the changeset record schema so external producers (Remedium) emit conforming records.
2. Cross-checkout CI experiment: member CI prefers sibling `change/<id>` branches over defaults when they exist (the Zuul union) — gated on real demand from a multi-member changeset that keeps breaking at integration.
3. Swap the `gh` probe for the RM-17 forge adapter when it lands.

## Rejected alternatives

**Monorepo consolidation.** Answers a different question, and is unavailable for platforms whose repo boundaries mirror org boundaries. Coordination must work over the fleet that exists.

**Meta-repo / submodules.** Pins member SHAs to solve an identity problem, and couples clone, CI, and review mechanics in the process. The changeset needs a name and a member list, not a super-repository.

**Multi-project slices.** Letting one slice bind several projects breaks the singular `slices[].project` schema, the synthesis kernel's per-project header derivation, and RFC-85 C1's one-target-per-repository topology decision — for no gain, since the plan already spans projects across slices.

**`submitWholeTopic` emulation.** GitHub has no atomic cross-repo merge; simulating one (merge-all-or-revert) would require Emery to perform merges and reverts, violating D5 and the cli-contract. Convergence with declared order is the honest contract.

**External system as record owner (Sourcegraph Batch Changes, or a Remedium-owned store).** The tracking model is right; the ownership is wrong. Workflow state authority stays in the CLI; external systems may *emit* changeset records (D8), never own the plan-backed ones.

**Registry as the changeset home.** `registry.yaml` carries membership and location only (roadmap principle). A changeset is per-change state; it derives from `plan.yaml` and the forge, and is never written into the registry.

## Non-goals

- Pushing branches, opening PRs, or merging PRs from Emery — publication stays operator-owned in every phase.
- Atomic cross-repo submission or automated rollback of landed members.
- Changing `registry.yaml`, `slices[].project` cardinality, or any slice lifecycle transition.
- A second ordering surface beside `depends-on`.
- Forge-side platform grouping (GitHub custom properties, topics) — complementary membership metadata, out of scope here.
- Replacing RFC-85's program coordinator or absorbing its migration semantics.
- A version solver or cross-repo dependency resolution.

## References

- [Gerrit — submitting changes across repositories by using topics](https://gerrit-review.googlesource.com/Documentation/cross-repository-changes.html)
- Zuul `Depends-On` cross-repository dependencies
- [Sourcegraph Batch Changes](https://sourcegraph.com/docs/batch-changes)
- [roadmap.md — cross-repo coordination](roadmap.md#cross-repo-coordination) · [RM-17 / RM-20](roadmap.md#rm-17-operator-owned-forge-integration)
- [RFC-85 Migration Program](rfc-85-migration-program.md) · [RFC-86 Working Trees](rfc-86-working-trees.md) · [RFC-77 Release Process](rfc-77-release-process.md) · [RFC-88 Concurrent Execution](rfc-88-concurrent-execution.md)
- RFC-81 Cloud Alert Remediation Platform (`augentic/remedium` `rfcs/rfc-81-cloud-alert-remediation.md`)
- [cli-contract — operator-owned publication](../docs/standards/cli-contract.md)

## Review ask

Confirm D1–D10: the plan is the changeset (no new noun); the record is derived, never authored; `change/<plan>` branches and `Emery-Change` PR trailers make the set forge-reconstructible; publication order derives from `depends-on`; Emery tracks and verifies but never publishes; convergence not atomicity; a read-only `emery plan changeset` projection; the same record shape for external producers; serial preparation now; forge reads via a `gh` probe until RM-17.

The one decision that must not be deferred is D5 (track-and-verify vs perform): it is where this RFC either respects or amends RFC-85's non-goal and the cli-contract's operator ownership. This draft respects both.
