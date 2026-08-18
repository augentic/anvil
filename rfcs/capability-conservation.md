# Capability Conservation Ledger

> Status: **Re-scoped 2026-08-18** for the spec-generator programme ([ADR-0008](decisions/0008-spec-generator-programme.md)). This ledger preserves product capabilities, not current crates, files, commands, event kinds, or implementation mechanisms.
>
> Authority: [product.md](product.md) defines the product, accepted records under [decisions/](decisions/) define policy, and [target-architecture.md](target-architecture.md) defines the destination. This ledger never overrides those authorities.
>
> Retirement: after the generator ships, reduce this to a completed traceability record or delete it. The build programme starts its own ledger (or reopens the deferred entries below) rather than inheriting a 19-row Preserve list as skeleton scope.

Legacy implementation may be deleted from the live tree when:

1. the replacement passes the capability's acceptance evidence; or
2. an accepted ADR explicitly deletes the capability and names the consequence.

A green walking skeleton is not parity for capabilities deferred to the build programme. Silence never conserves.

## Classification

Every live entry uses exactly one of these classes:

- **Preserve** — required in the spec generator before the legacy implementation is deleted from the live tree.
- **Replace** — the current mechanism is deleted, but its observable guarantee remains.
- **Deferred with the build programme** — conserved design intent; not Phase 3 evidence; not a reason to keep archive crates on the live branch.
- **Intentionally deleted** — removed by an accepted ADR.

## Live entries (this programme)

### CC-01 — Source-to-specification fidelity

**Classification:** Preserve  
**Origin:** RFC-88, adapter contract, A8

Emery extracts declared sources and retains every structured claim field synthesis needs. Source adapters remain value-in and cannot acquire lifecycle authority or inspect the output home.

The replacement may change the claim DTOs and persistence format. It must not:

- silently discard claim extras such as statements, criteria, or replay digests;
- treat an inaccessible or failed source as successful empty output;
- let a source adapter read plan or lifecycle authority;
- lose the identity of the exact source value observed.

Survey-as-a-distinct-operation and the leads catalog are not conserved (ADR-0008).

**Acceptance evidence:**

- Structured claim extras survive the component seam and appear in the spec set (or its IR).
- Unreadable or malformed source output fails closed.
- An extracted claim is traceable to its source identity.
- `emery specify` over intent + one docs source produces `spec.md` / `design.md`.

### CC-04 — Specification-stage firewall

**Classification:** Preserve (as a negative)  
**Origin:** RFC-91

`specify` may extract, synthesise, and publish the spec set. It must not:

- open a target wave;
- create a product workspace;
- dispatch a target build operation;
- mutate an accepted product baseline.

The `build` half of the original firewall is destination text — there is no `build` verb in this programme.

**Acceptance evidence:**

- The journey test and route budget show no target dispatch.
- A failed synthesis cannot present a spec as complete.

### CC-05 — Reviewable specification

**Classification:** Preserve  
**Note:** strengthen — one reviewable spec set is a designed property the current artifact family does not yet satisfy (P8)  
**Origin:** Existing specification artifacts plus remediation P8; ADR-0008

The operator gets a spec set (`spec.md` / `design.md`) through which a human or reviewing agent can answer whether the system is specified correctly.

It contains or directly exposes:

- behavioral requirements;
- inline unknowns, conflicts, and divergences;
- a provenance summary;
- target-independent behavior.

Structured models, tasks, and composition documents may exist later but are subordinate — and `tasks.md` / `composition.yaml` are not first-wave artifacts (ADR-0008).

**Acceptance evidence:**

- A reviewer can approve or reject without opening a second artifact family.
- Re-mining produces a meaningful diff.
- Every conflict, divergence, and unknown appears inline.
- Full provenance remains one gesture away.

### CC-06 — Honest conflict and divergence

**Classification:** Replace  
**Origin:** RFC-88 authority resolution; [ADR-0004](decisions/0004-conflict-disposition.md) Option D  
**Conditional on:** ADR-0004 (proposed, re-scoped)

When sources disagree, a closed authority precedence (`intent > documentation > behaviour`) resolves what it can. An authority-resolved disagreement is recorded as a divergence, never silently won; only unresolvable disagreement escalates to a conflict. `[unknown]` stays `[unknown]`. Nothing is auto-deferred. There is no build gate.

**Acceptance evidence:**

- An injected source disagreement surfaces `[conflict]` or `[divergence]` in the spec.
- An authority-resolved disagreement remains reviewable as a divergence rather than disappearing into the winning claim.

Debt conservation, deferral lifecycle, and disposition-before-build wait with the build programme (original CC-06 remainder).

### CC-17 — Dynamic typed adapter boundary

**Classification:** Preserve  
**Note:** isolation profiles and dispatch budgets (D7/D8) are deferred with the build programme, not walking-skeleton evidence  
**Origin:** Existing WIT seam, ADR-0002, ADR-0008

A source adapter can be added at an exact version without rebuilding the host. The same engine core remains embeddable behind a desktop CLI and a service ingress; the conserved duality is architectural embeddability, not a live web service — the mutating HTTP surface stays disabled until an ingress design exists (C3).

The native shadow provider, bare-name resolution, pull-on-miss, cache seeding, marketplace machinery, and the target WIT world are not conserved in this programme.

The conserved capability requires:

- one WIT-defined type family;
- exact adapter identity;
- explicit admission;
- automated execution across the production component seam (extract, not survey+build).

**Acceptance evidence:**

- CI admits and dispatches one out-of-binary exact-pin source component; the specify journey crosses the component seam; native-only integration cannot become the authoritative test path.

## Deferred with the build programme

These remain design intent. They are not Phase 3 evidence and not a reason to keep archive crates on the live branch. Full text is at tag `v1` (`git show v1:rfcs/capability-conservation.md`). Reopen with a build-programme ADR.

| Id | Capability |
| --- | --- |
| CC-02 | Deterministic survey-to-slice decomposition / topology compiler |
| CC-03 | Reviewable and correctable topology; abandonment authority |
| CC-07 | Detached, explicitly bound delivery (ADR-0005 Option A) |
| CC-08 | Private workspace isolation |
| CC-09 | Accepted CID and living product baseline |
| CC-10 | Engine-owned phase protocol (`build → verify ⇄ repair → review ⇄ repair`) |
| CC-11 | Resumability from one authoritative state read (store-shaped) |
| CC-12 | Deterministic bounded concurrency |
| CC-13 | Atomic same-target waves |
| CC-14 | Verification of the exact composed result |
| CC-15 | Coordinated multi-target publication |
| CC-16 | Coverage-accounted archaeology and architecture projection |
| CC-18 | Deterministic and traceable authorization (waves, base CID, receipts) |
| CC-19 | Durable conversational correction (`fix`) |

CC-04's `build` half (refuse a missing specification receipt before creating a workspace) reopens with CC-08/CC-10.

## Intentionally not conserved

Unless a later ADR reopens them, this programme does not preserve:

- survey as a distinct WIT operation or operator verb (ADR-0008);
- the leads catalog, `decomposition.yaml`, `plan.yaml` topology, `discovery.yaml`;
- the journal as lifecycle authority (ADR-0001);
- the multi-writer claim protocol (architecture-review S4);
- stored mutable progress labels;
- a mandatory definition-home lifecycle (ADR-0003);
- hand-authored `scope.yaml` or `coverage.yaml` before first output (ADR-0003);
- `system.wave.reviewed` as a second authority plane;
- a live lease from a change home onto a definition tree (addendum P12);
- the in-place/detached mode distinction as a *live* surface (ADR-0005 Option C);
- merge-time checkout `apply` (deleted by implemented RFC-88);
- the native shadow provider (ADR-0002);
- bare adapter names, pull-on-miss, cache seeding, and pull-latest (ADR-0002);
- universal conflict auto-deferral (ADR-0004 Option D);
- `tasks.md` / `composition.yaml` as first-wave artifacts (ADR-0008);
- in-tree `crates-v1/` quarantine and a second-repo archive (ADR-0008);
- orchestration inside skill or adapter bodies — the CLI owns lifecycle;
- exact legacy artifact names or crate boundaries.

## Remediation gates

### Phase 3 exit (walking skeleton)

- CC-01 source fidelity (extract extras; intent + docs → `spec.md` / `design.md`);
- CC-04 specify-does-not-build;
- CC-05 reviewable spec set;
- CC-06 conflict/divergence visible inline;
- CC-17 production component seam (extract journey + exact-pin admission; not isolation or budgets).

### Phase 4 exit (generator reliable)

- First-party source adapters needed by Propellerhead (documentation, code, intent; others as required) over the conserved seam;
- graded eval against product.md's "time to first reviewable specification";
- re-mine diffs.

### Build-programme exit

Not this ledger's close-out. A later ADR reopens the deferred table and names its own skeleton.
