# RFC-103: Operator Attribution

> **Status:** Draft. Depends only on implemented [RFC-86](rfc-86-change-facts.md), so it is **unblocked now** despite its position in the series — the number is the next free one, not a sequencing claim (see [platform.md § The series](platform.md#the-series)).
>
> **Owns:** the typed **actor record** carried by every fact — the closed actor class (`human | agent`), the declared driver identity, and the **attestation level** (`unattested | declared | attested`) that states how much the record is worth — plus the deployment surface that declares it and the projections that surface it where decisions are read.
>
> **Builds on** RFC-86's fact substrate (per-writer logs, closed event taxonomy, computed status) and [RFC-86a](rfc-86a-gap-deferral.md)'s durable disposition facts, whose `origin` field it composes with rather than replaces.
>
> **Patch ownership:** this RFC amends RFC-86's event shape after RFC-86 has landed, by adding a required actor record to every appended fact and clarifying D23 (the writer id is a claim-ownership and log-partitioning key, never an actor identity). It does not revise RFC-86, and it does not join the frozen 86–91 range. D2 (status computed, not stored), D6 (no `approve` verb, no projected `approved` rung), and RFC-86's non-goal on multi-operator countersign remain unchanged — this RFC records who acted and gates nothing.

## Intent

Make the fact log answer *who decided this*, now that the operator may not be a person.

Emery's engine is a deterministic state machine and the operator sits outside it as a caller. That separation is deliberate and it is what distinguishes this architecture from an agent-orchestrated one: an agent may **drive** the engine, but an agent may not **be** the engine. The consequence is that operator identity is orthogonal to the architecture — an autonomous driver can issue the same verbs a person can, and today the resulting journal is byte-identical either way.

That invariance is exactly what makes replay trustworthy. It is also why the actor is unrecoverable. This RFC adds the missing half: the trail stays invariant in what it proves about *inputs and results*, and becomes explicit about *who asked*.

## Why the writer id is not an actor

`journal::writer_id()` resolves a non-empty `EMERY_WRITER` or falls back to `local`. It is validated only as a single path segment, because its job is to name a file that one process appends to alone; readers union every log by `(timestamp, writer, sequence)`. It is a concurrency partition and an ordering key.

RFC-86's own worked example writes `"writer":"operator-a"`, which reads like a person. Nothing enforces that, nothing authenticates it, and the shipped default makes every desktop run `local`.

| Problem | In plain terms |
| ------- | -------------- |
| The writer is a partition, not a person | Free-form, unauthenticated, defaulted to `local`, and structurally required to stay cheap — it is claimed per slice and appears in a path. Overloading it with identity would make a filename carry an audit claim. |
| Mechanism attribution is not actor attribution | RFC-86a's `DeferralOrigin` distinguishes the explicit `plan defer` act from a gate-minted policy deferral. That says which code path minted the fact, not who was behind the run. |
| The operator may now be an agent | Nothing in the CLI requires a human: there are no TTY prompts, `--force` is a flag rather than a confirmation, `--format json` is global, and `plan status` projects a closed next-action enum. The eval case runner already drives `init → plan author → plan execute` with no human gesture. |
| Social review needs an actor | RFC-86 deliberately rejects an engine four-eyes gate and states that shared-directory collaboration is "social review of the fact log". Social review only works if the log says who. |

## The reframe: attribution, not permission

An agent operator can defer every open gap with a generated reason, `plan drop` an entry to clear a stop, override authority on a conflict, or pass `--force`. The tempting response is a permission surface over the operator verbs.

That would cut against the grain twice. RFC-86a already replaced *permission to build over* a gap with a *disposition* that rides the fact log, precisely because per-epoch permission re-taxes decisions already made. And RFC-86 already refuses a countersign gate in favour of social review.

So the answer to "an agent can now do all of that" is not that the engine stops it. It is that the log says an agent did it, under what declaration, and how much that declaration is worth. Prevention stays where it already lives — in gates over *artifacts and results* (digest-bound coverage, gap gates, phase budgets, commit admission), which constrain outcomes regardless of who called.

## Decisions

### D1 — Every fact carries an actor record

The record is a required field on every appended event, not a property of the run or the epoch. A union read reconstructs the actor for any fact without replaying command history or correlating against a separate log — the same reason RFC-86a made deferrals digest-bound facts rather than epoch state.

Uniform coverage is deliberate. A rule that attributes only "decision" facts needs revisiting every time the taxonomy grows, and invites the question of why a given fact was judged unimportant.

### D2 — The actor class is closed: `human | agent`

`human` means a person issued the verb. `agent` means an autonomous driver issued it. There is no `policy` variant: policy is a property of *how a fact was minted*, which RFC-86a's `origin` already carries, and duplicating it here would produce two fields that disagree.

The two compose exactly as an auditor needs. A deferral minted at the gate under `--gap-policy defer` during an agent-driven run reads `actor.class: agent` with `origin: policy` — an autonomous driver started a run under a declared policy, and the gate minted this row. Neither field alone says that.

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

No verb is refused, no budget is scaled, and no policy admission consults the actor class. An agent operator may do everything a human operator may. This RFC adds a field and four projections; it adds no error variant that can block a run.

### D7 — Projections surface the actor where decisions are read

The record is worthless if it only exists on disk. It appears in the four places a reviewer asks "who decided this":

- `emery journal show` gains an actor filter alongside its existing `--filter` / `--limit`.
- `emery debt` shows the class that accepted each carried row — the question a regulated client asks about every piece of debt in a delivered baseline.
- `plan archive`'s carried-debt summary and `plan gaps`' disposition rows carry it for the same reason.

Attestation level renders with the class wherever it is shown. A class without its level is a misleading projection.

## Amendments to RFC-86 (explicit)

- **Event shape.** Every event gains a required actor record. The taxonomy, wire ids, per-writer log layout, and union ordering are unchanged.
- **D23 clarified.** The writer id is claim ownership and log partitioning. RFC-86's `"writer":"operator-a"` example reads as an actor and must not be relied on as one; attribution is D1's field.
- **Unchanged:** D2 (status computed from artifacts and facts — the actor record is a fact field, never a status input), D6 (no `approve` verb, no `approved` rung), and the standing non-goal on multi-operator countersign.

## Implementation requirements

1. Actor class, identity, and attestation level are closed types in `crates/project/src/journal.rs`, serialized kebab-case like the rest of the taxonomy, with the goldens regenerated.
2. The value is captured once at each composition root and carried on the existing handler context — no `std::env` read below the root, and no process-global.
3. The append boundary requires the record, so a new event kind cannot be added without one.
4. Journal reads tolerate facts written before this RFC: an absent record projects as `unattested` with no class, which is the truth about those rows.
5. No new blocking error variant. A malformed declaration falls back to `unattested` and warns on stderr rather than failing the run.

## Acceptance criteria

1. A fully agent-driven run — `init → plan author → plan refine → plan execute → plan archive` with no human gesture — produces a log in which every fact reads `agent` / `declared`, and `emery debt` attributes every carried row to the driver.
2. The same sequence run by a person with nothing declared reads `unattested` throughout, and no projection claims a human was present.
3. A `defer`-policy run started by an agent yields deferrals reading `actor.class: agent` with `origin: policy`, distinguishable from an explicit `plan defer` by the same driver.
4. Pre-RFC logs union with post-RFC logs and project without error.
5. Removing the declaration changes no exit code and blocks no verb.

## Open questions

1. **Identity format.** Free-form opaque label, or a constrained shape (namespace, driver name, version)? A constrained shape is more projectable; free-form avoids inventing a registry. Leaning free-form, since the non-goal on PII is a deployment policy Emery cannot enforce either way.
2. **What `attested` requires.** Host-vouched identity is the minimum. Whether it also implies a signed fact, and who holds the key, belongs to [RFC-101](rfc-101-platform-readiness.md)'s deployment identity rather than being pre-specified here.

**Closed — does the new field invalidate any recorded digest?** No. Events *carry* digests of other artifacts (phase reports, wave manifests, spec trees) but no event is itself hashed, and no coverage payload digests the journal. The taxonomy can therefore grow a required field without invalidating recorded coverage — which is why D1 can require it uniformly rather than adding it as an optional tail.

## Rejected alternatives

- **Overload `EMERY_WRITER` as the identity.** It names a file, it is claimed per slice, and it must stay a cheap path segment. An audit claim in a filename is the wrong home, and it would collide with RFC-86 D23's concurrency meaning.
- **Gate operator verbs by actor class.** A permission surface over `defer` / `drop` / `amend` / `--force` contradicts RFC-86a's disposition-not-permission reframe and RFC-86's refusal of a countersign gate. Constraint belongs on artifacts and results, where it already is.
- **Infer the actor from TTY presence.** `is_terminal` is used today only to decide stderr colour. Inferring humanity from a file descriptor is wrong under CI, wrong under a driver that allocates a PTY, and manufactures confidence the record then reports as fact.
- **Sign facts now.** Non-repudiation needs a key authority and a revocation story that a desktop binary does not have. D3's `attested` level is the seam for it; RFC-101 owns the deployment identity behind it.
- **Record the actor only on decision-shaped facts.** A subset rule needs relitigating whenever the taxonomy grows, and every omission reads as a judgment that the fact did not matter.
- **Declare the actor per epoch.** RFC-86a's waiver lesson: anything re-supplied per run is dropped on a resume and re-taxes the operator for a decision already made.

## Non-goals

- **Authentication.** Emery does not verify a declaration; D3 records that it did not.
- **Authorization.** No actor class grants or withholds any capability (D6).
- **User identity or PII.** The identity is an opaque driver label, not a person's name, email, or account. A deployment that wants a human name is choosing to put it there.
- **Four-eyes review.** Unchanged from RFC-86: collaboration is social review of the fact log, which this RFC makes possible rather than automates.
- **Per-keystroke or per-model-call provenance.** The unit is the fact. Model-call accounting belongs to [RFC-92](rfc-92-operation-model-policy.md).
