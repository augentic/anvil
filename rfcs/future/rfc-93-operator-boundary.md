# RFC-93: Operator Boundary

> **Status:** Draft — parallel assurance track over implemented [RFC-86](rfc-86-change-facts.md) in the [Services Delivery Programme](platform.md). Startable now, but not on the product critical path; do not staff ahead of [RFC-88](rfc-88-detached-changes.md) or [RFC-92](rfc-92-model-policy.md). Host operator grants land when an engagement needs caller refusal, not as programme step one.
>
> **Owns:** the typed **actor record** carried by every fact — the closed actor class (`human | agent`), declared driver identity, and **attestation level** (`unattested | declared | attested`) — plus the deployment-owned **operator grant** that may refuse an otherwise legal CLI act before guest dispatch. Attribution answers who acted; authorization answers what that caller was allowed to request. Neither becomes lifecycle state.
>
> **Builds on** RFC-86's fact substrate (per-writer logs, closed event taxonomy, computed status). [RFC-86a](rfc-86a-gap-deferral.md)'s durable gate-minted disposition facts ride that substrate.
>
> **Patch ownership:** this RFC amends RFC-86's event shape after RFC-86 has landed by adding required actor and grant records to every appended fact and clarifying D23 (the writer id is a claim-ownership and log-partitioning key, never an actor identity). It adds a host-dispatch refusal before the guest sees an invocation. RFC-86 D2 (status computed, not stored), D6 (no `approve` verb, no projected `approved` rung), and the non-goal on multi-operator countersign remain unchanged.

## Intent

Make Emery answer *who requested this, and was that caller allowed to request it?*, now that the operator may not be a person.

Emery's engine is a deterministic state machine and the operator sits outside it as a caller. That separation is deliberate and it is what distinguishes this architecture from an agent-orchestrated one: an agent may **drive** the engine, but an agent may not **be** the engine. The consequence is that operator identity is orthogonal to the architecture — an autonomous driver can issue the same verbs a person can, and today the resulting journal is byte-identical either way.

That invariance is exactly what makes replay trustworthy. It is also why the actor and the caller's authority are unrecoverable. This RFC adds both without putting either into lifecycle projection: the trail stays invariant in what it proves about *inputs and results*, becomes explicit about *who asked*, and records which deployment grant admitted the request.

## Why the writer id is not an actor

`journal::writer_id()` resolves a non-empty `EMERY_WRITER` or falls back to `local`. It is validated only as a single path segment, because its job is to name a file that one process appends to alone; readers union every log by `(timestamp, writer, sequence)`. It is a concurrency partition and an ordering key.

RFC-86's own worked example writes `"writer":"operator-a"`, which reads like a person. Nothing enforces that, nothing authenticates it, and the shipped default makes every desktop run `local`.

| Problem | In plain terms |
| ------- | -------------- |
| The writer is a partition, not a person | Free-form, unauthenticated, defaulted to `local`, and structurally required to stay cheap — it is claimed per slice and appears in a path. Overloading it with identity would make a filename carry an audit claim. |
| Mechanism attribution is not actor attribution | A `gap.deferred` event says the build gate minted a disposition. It does not say who started the run that reached the gate. |
| The operator may now be an agent | Nothing in the CLI requires a human: there are no TTY prompts, `--force` is a flag rather than a confirmation, `--format json` is global, and `plan status` projects a closed next-action enum. The eval case runner already drives `init → plan author → plan refine → plan execute` with no human gesture. |
| Social review needs an actor | RFC-86 deliberately rejects an engine four-eyes gate and states that shared-directory collaboration is "social review of the fact log". Social review only works if the log says who. |

## The reframe: attribution and authorization are orthogonal

An agent operator can start execution that auto-defers every open gap with a synthesized gate reason, `plan drop` an entry to clear a stop, override authority on a conflict, or pass `--force`. Recording those acts and authorizing the caller to request them are different concerns.

This RFC does not make actor class a permission bit. Doing so would make the same artifact result legal for a declared human and illegal for a declared agent, despite desktop declarations being forgeable. RFC-86a already replaced *permission to build over* a gap with a durable disposition, and RFC-86 already refuses a countersign lifecycle gate.

The actor record therefore says who acted, under what declaration, and how much that declaration is worth. A separate operator grant may refuse a CLI act, product, target, or change before dispatch. Artifact and result prevention stays in the engine's digest-bound coverage, gap gate, phase budgets, and commit admission. Caller authorization stays at the host boundary. Neither mechanism treats self-declared actor class as proof or changes which artifact states are legal.

## Terms

- An **actor record** is the class, driver identity, and attestation level inherited by every fact emitted under one invocation.
- An **operator act** is one member of the closed host-visible command set derived from the typed CLI router, such as `plan.author`, `plan.refine`, `plan.execute`, `plan.drop`, or `plan.amend-authority`. A forced variant is a distinct act: a verb invoked with `--force` derives its own act key, so a grant may admit the plain form without the forced one.
- An **operator grant** is immutable deployment policy binding a principal or local process class to allowed acts, product/target/change scope, expiry, and optional external-approval requirements.
- A **grant record** is the grant id and digest carried beside the actor record on every resulting fact.
- An **external approval attestation** is an opaque host-verified value satisfying one grant condition. It is authorization evidence, not a workflow approval state.

## Decisions

### D1 — Every fact carries an actor record

The record is a required field on every appended event, not a property of the run or the epoch. A union read reconstructs the actor for any fact without replaying command history or correlating against a separate log — the same reason RFC-86a made deferrals digest-bound facts rather than epoch state.

Uniform coverage is deliberate. A rule that attributes only "decision" facts needs revisiting every time the taxonomy grows, and invites the question of why a given fact was judged unimportant.

### D2 — The actor class is closed: `human | agent`

`human` means a person issued the verb. `agent` means an autonomous driver issued it. There is no `policy` variant: policy or engine mechanism is a property of *how a fact was minted*, represented by its event kind and covered policy identities, not by the actor behind the invocation.

The two compose exactly as an auditor needs. An auto-deferral minted at the build gate during an agent-driven run is a `gap.deferred` fact with `actor.class: agent`: the event kind says which mechanism acted, and the actor record says who drove the invocation.

### D3 — Attestation is honest about what Emery can know

Emery is a local binary. Anything the caller declares about itself can be forged by whoever runs it. Recording a self-declaration as though it were proof would make the audit trail *worse* than recording nothing: it would read as authoritative while being trivially forgeable.

So the record carries its own confidence:

| Level | Meaning |
| ----- | ------- |
| `unattested` | Nothing was declared. The default, and the honest state for a person at a terminal — Emery cannot know who ran it. |
| `declared` | The caller declared a class and identity. Recorded as a claim, never as proof. |
| `attested` | A host vouched for the identity. Unavailable on a desktop; arrives with [RFC-101](rfc-101-platform-readiness.md) deployment identity. |

A desktop human run is `unattested` throughout and never claims otherwise. The common and valuable case is an agent driver declaring itself, which is `declared` — enough for review, and labelled so nobody mistakes it for non-repudiation.

### D4 — The writer id stays a concurrency key

`EMERY_WRITER` is not repurposed and its semantics do not change. The actor record is a separate field on the event. RFC-86 D23's per-slice claim exclusivity continues to key on the writer, and a writer id remains a valid path segment by construction.

### D5 — Declared by the deployment, once

The class and identity are read at the composition root — an `EMERY_ACTOR` / `EMERY_ACTOR_ID` pair beside the existing `Locations::from_env` capture — and travel explicitly into the engine, like every other environment-derived value. Kernels never read `std::env`, and the wasm32 guest has no process environment, so the value is captured host-side and passed through the seam.

Not a per-command flag. An autonomous driver declares itself once for its process; re-supplying an identity on every invocation is the per-epoch tax RFC-86a removed.

### D6 — Attribution gates nothing

No verb is refused, no budget is scaled, and no policy admission consults the actor class. Removing or changing an actor declaration changes attribution only. A deployment may independently refuse the caller under D8's operator grant, but it never infers authority from the actor record.

### D7 — Projections surface the actor where decisions are read

The record is worthless if it only exists on disk. It appears in the four places a reviewer asks "who decided this":

- `emery journal show` gains an actor filter alongside its existing `--filter` / `--limit`.
- `emery debt` shows the class that accepted each carried row — the question a regulated client asks about every piece of debt in a delivered baseline.
- `plan archive`'s carried-debt summary and `plan gaps`' disposition rows carry it for the same reason.

Attestation level renders with the class wherever it is shown. A class without its level is a misleading projection.

### D8 — The host resolves one operator grant before guest dispatch

The launcher or hosted ingress resolves the caller's grant before invoking the engine guest. The grant carries:

- stable id and canonical digest;
- bound principal or local process class;
- allowed operator acts;
- optional product, target, change, and path scope;
- issuance and expiry;
- optional external-approval requirements.

An act outside the grant fails before guest dispatch with `operator-act-denied`. Missing, malformed, expired, scope-mismatched, or unverifiable grant data fails closed on a restricted ingress. The desktop default is an explicit `local-owner` grant admitting every local act; it is convenient, not attested, and cannot satisfy a deployment policy that requires authenticated authority.

The host passes only the actor record, grant record, allowed act, and verified scope into invocation context. Credentials, policy-registry locations, and external approval bodies never enter the guest.

### D9 — Grants authorize requests, not results

An operator grant cannot waive a gap, fabricate epoch coverage, raise a repair budget, weaken verification, alter authority resolution, or replace commit admission. It answers only whether the caller may request an otherwise legal engine operation.

The same invocation over the same artifacts therefore reaches the same engine gates regardless of actor class or grant. A broader grant makes more requests dispatchable; it does not make another artifact state valid. Changing a grant creates a new grant digest on future facts and rewrites no history.

### D10 — Facts carry the grant that admitted their invocation

Every fact carries a grant record beside its actor record. Engine-emitted facts inherit both from invocation context, including gate-minted deferrals and phase events. A union read can therefore answer who drove an act and under which grant without correlating a separate ingress log.

Pre-RFC facts project an explicit unknown grant. They are not retroactively treated as `local-owner`.

## Amendments to RFC-86 (explicit)

- **Event shape.** Every event gains required actor and grant records. The taxonomy, wire ids, per-writer log layout, and union ordering are unchanged.
- **D23 clarified.** The writer id is claim ownership and log partitioning. RFC-86's `"writer":"operator-a"` example reads as an actor and must not be relied on as one; attribution is D1's field.
- **Unchanged:** D2 (status computed from artifacts and facts — actor and grant records are fact fields, never status inputs), D6 (no `approve` verb, no `approved` rung), and the standing non-goal on multi-operator countersign.

## Implementation requirements

1. Actor class, identity, attestation level, operator act, operator grant, and grant record are closed types, serialized kebab-case like the rest of the taxonomy, with the goldens regenerated.
2. Actor declarations are captured once at each composition root and carried on the existing handler context — no `std::env` read below the root, and no process-global.
3. Add the deployment-neutral operator-grant resolver at the host dispatch boundary. The native desktop binds `local-owner`; restricted and hosted ingress bind authenticated principals through deployment policy.
4. Derive the closed operator act from the typed command router before dispatch. Do not parse argv twice or let the guest reinterpret a broader act as a narrower one.
5. Require actor and grant records at the append boundary, so a new event kind cannot omit either.
6. Journal reads tolerate facts written before this RFC: absent actor and grant records project as `unattested` and `unknown`, which is the truth about those rows.
7. A malformed actor declaration falls back to `unattested` and warns on stderr. A malformed or expired grant on restricted ingress fails closed as `operator-act-denied`.
8. Add read-only grant identity to `journal show`, `plan gaps`, `debt`, and archive projections beside actor attribution.

## Acceptance criteria

1. A fully agent-driven run — `init → plan author → plan refine → plan execute → plan archive` with no human gesture — produces a log in which every fact reads `agent` / `declared` plus the exact admitting grant digest, and `emery debt` attributes every carried row to both.
2. The same sequence run locally by a person with nothing declared reads `unattested` under the explicit `local-owner` grant, and no projection claims a human was present or that the grant was authenticated.
3. A run started by an agent with open gaps yields gate-minted `gap.deferred` facts inheriting `actor.class: agent` and the run's grant digest.
4. A grant excluding `plan.drop`, authority override, `--force`, one target, or one change refuses that act before guest dispatch and emits no workflow fact.
5. A grant cannot make an unresolved gap, stale epoch, exhausted repair budget, blocking verification report, or inadmissible merge pass its engine gate.
6. Changing the actor declaration changes no authorization result. Changing the grant changes future grant records and dispatch outcomes without changing lifecycle semantics or historical facts.
7. Pre-RFC logs union with post-RFC logs and project without error, reporting unknown actor/grant truthfully.

## Open questions

1. **Identity format.** Free-form opaque label, or a constrained shape (namespace, driver name, version)? A constrained shape is more projectable; free-form avoids inventing a registry. Leaning free-form, since the non-goal on PII is a deployment policy Emery cannot enforce either way.
2. **What `attested` requires.** Host-vouched identity is the minimum. Whether it also implies a signed fact, and who holds the key, belongs to the future hosted deployment rather than being pre-specified here.

**Closed — does the new field invalidate any recorded digest?** No. Events *carry* digests of other artifacts (phase reports, wave manifests, spec trees) but no event is itself hashed, and no coverage payload digests the journal. The taxonomy can therefore grow a required field without invalidating recorded coverage — which is why D1 can require it uniformly rather than adding it as an optional tail.

## Rejected alternatives

- **Overload `EMERY_WRITER` as the identity.** It names a file, it is claimed per slice, and it must stay a cheap path segment. An audit claim in a filename is the wrong home, and it would collide with RFC-86 D23's concurrency meaning.
- **Gate operator verbs by self-declared actor class.** A forgeable `human | agent` label is not authority. D8's host-resolved grant binds permissions to deployment identity and scope before dispatch; the actor field remains attribution only.
- **Infer the actor from TTY presence.** `is_terminal` is used today only to decide stderr colour. Inferring humanity from a file descriptor is wrong under CI, wrong under a driver that allocates a PTY, and manufactures confidence the record then reports as fact.
- **Sign facts now.** Non-repudiation needs a key authority and revocation story that a desktop binary does not have. D3's `attested` level is the seam; a future hosted deployment may supply it.
- **Record the actor only on decision-shaped facts.** A subset rule needs relitigating whenever the taxonomy grows, and every omission reads as a judgment that the fact did not matter.
- **Declare the actor per epoch.** RFC-86a's waiver lesson: anything re-supplied per run is dropped on a resume and re-taxes the operator for a decision already made.
- **Put operator grants inside autonomy policy.** Caller authority is required for ordinary reviewed operation as soon as an agent can drive the CLI. It cannot wait for unattended merge, and result admission must not vary with caller identity.

## Non-goals

- **Universal authentication.** A desktop actor declaration remains forgeable and D3 records that honestly. Restricted deployments may bind authenticated identities to grants.
- **Actor-class authorization.** No `human | agent` value grants or withholds capability (D6).
- **User identity or PII.** The identity is an opaque driver label, not a person's name, email, or account. A deployment that wants a human name is choosing to put it there.
- **Four-eyes review.** Unchanged from RFC-86: collaboration is social review of the fact log, which this RFC makes possible rather than automates.
- **Per-keystroke or per-model-call provenance.** The unit is the fact. Model-call accounting belongs to [RFC-92](rfc-92-model-policy.md).
