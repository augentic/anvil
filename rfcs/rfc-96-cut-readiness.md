# RFC-96 cut readiness

> Status: Editing brief for [RFC-96](rfc-96-concurrent-execution.md). Not a new RFC. Fold the decided items into RFC-96, then delete this file.
>
> Verdict: **Phase-A staffable after a short freeze; not freeze-ready as written.** The product policy is closed and the [RFC-106](rfc-106-task-graphs.md) split removed the largest holes. A faithful first PR would still stall on which bytes enter the work-item digest, where an operation claim lives, where the pool actually runs, and what `plan status` projects when the singular cursor is gone.

## How to use this

Each finding is one RFC-96 hole plus a recommended edit. Accept, rewrite, or reject the recommendation in RFC-96 itself. Do not implement against this file.

Suggested fold order: F1–F4 first — they are the Phase-A freeze and change what the first PR builds. Then F5–F7 (Phase-A behaviour under failure). Then F8–F10 (Phase-B wire). Then F11–F12 (synthesis slice and criteria hygiene).

## Already decided

Do not reopen these in the fold. The current revision settled them, mostly by moving intra-slice work to RFC-106:

- Task graphs, `target.decompose`, task-scoped grants, ownership-envelope lowering to `file | tree`, and graph-attributable re-decomposition are [RFC-106](rfc-106-task-graphs.md), evidence-gated. RFC-96 adds no seventh target operation.
- RFC-90 D5 is unamended: one workspace and one artifact stage span one slice-build attempt. Per-operation rematerialization is RFC-106.
- The slice-build attempt is the dispatch unit and the future RFC-100 transport unit — not a phase, branch, or worktree.
- `guest.lock` stays as one execute supervisor per change home; the pool is in-process under that supervisor. RFC-86 D23 claims remain the cross-writer slice fence; wave commit keeps a single writer.
- Cap one is the reference mode, cap four the shipped default, and cap-one/four equivalence is an acceptance gate, not a delivery stage.
- Phase A needs no `compose`; cross-target leaves parallelize under one-member waves, and the same-target rebuild tax is accepted.
- `compose(base, patches)` is engine-private on the RFC-87 kernel — disjoint touched paths, no textual merge, no new WIT.
- Phase B retires `Wave::enforce_one_member` for the concurrent executor only; manifest schema and `target.merge.wave-committed` shape do not change.
- Git, `wasi-vcs`, publication worktrees, and remote placement are out (RFC-95 host surface, RFC-100).

## Recommended cut order

Three slices, as the RFC already says — but freeze F1–F4 inside Phase A before its first PR:

- **Phase A** — work-item identity and digest coverage (F1), claim home (F2), pool seam and cap surface (F3), the ready-set `StatusBody` (F4), writer injection (F5), stale-base requeue (F6), and cancellation policy (F7). Concurrent survey / extract / refine / plan-author fan-out lands on that substrate.
- **Phase B** — `compose`, domain records, multi-member waves, after F8–F10 name the record DTOs, events, and typed stops.
- **Synthesis (D9–D10)** — independent, live-eval-gated; F11 sketches its wire before staffing.

---

## F1 — The work-item input digest has no byte definition

**Where:** Flow and terms; D2; Phase A; AC1.

**Finding.** Every load-bearing mechanism — dispatch fencing, claim identity, duplicate detection, "a changed parent input creates a distinct work-item identity" (AC1) — hangs off `(slice, phase, input-digest)`, and the RFC never says which bytes enter the digest for each phase. RFC-91 already defines canonical slice-local projections (`entry`, `leads`, `decomposition`, per-artifact bundle digests) and the refinement manifest digest; the wave base and dependency frontier exist as facts. Without a stated composition rule, two implementers produce two incompatible identities, and cap-one/four equivalence (AC3) is untestable.

**Recommend.** Add a closed per-phase coverage table to D2, reusing existing digests rather than inventing new ones:

- `refine` — the RFC-91 manifest *input* pins: the leaf's canonical `entry` / `leads` / `decomposition` projections, ordered predecessor refinement digests, bound source CIDs, target-guidance digest.
- `build` — the leaf's fresh refinement-manifest digest, the covering `plan.execute.started` epoch digest, the wave base (accepted CID or first-wave freeze), and ordered accepted predecessor identities.
- `merge` — the successful `BuildRecord` digest and the current accepted-frontier CID for the target.

State that the digest is SHA-256 over the canonical encoding of that closed struct (the same rule as the refinement manifest), derived on demand — never stored on `plan.yaml`.

## F2 — Operation claims have no persistence home or event

**Where:** D2 ("a local operation claim names…"); D3; AC1.

**Finding.** The landed claim kernel (`crates/project/src/journal/claim.rs`) is journal-backed and slice-scoped: `slice.claimed` / `slice.released`, one writer per slice, `slice-claim-conflict` on a second writer. The new operation claim is finer — work item plus operation plus attempt — and the RFC says when it releases but not where it lives: a new journal `EventKind`, a change-home file, or supervisor memory. Journal-backed operation claims would flood the fact log with orchestration noise; in-memory claims are unstated and look like a gap.

**Recommend.** Decide it explicitly in D3: because `guest.lock` guarantees exactly one execute supervisor per change home, **Phase-A operation claims are an in-process registry inside that supervisor** — no new journal event, no on-disk claim file. The durable fences remain the existing facts: `slice.claimed` for cross-writer exclusivity, attempt directories for build re-entry, wave facts for merge. Add one sentence to the RFC-100 paragraph: durable, lease-backed operation claims are exactly what RFC-100 adds when a second supervisor becomes legal. AC1's "duplicate local claims fail" then tests the in-process registry, not a wire.

## F3 — The pool has no seam, and the cap has no configuration surface

**Where:** D1; D3; implementation requirements ("one isolated host pool").

**Finding.** "One isolated host pool" is ambiguous in this deployment. The orchestrations run in the wasm32 engine guest; model calls, source dispatches, and workspace I/O are async host-mediated imports. Two readings exist: bounded concurrent futures *inside* the guest (join over async imports), or a new host capability that schedules guest work. The second is a WIT change the RFC elsewhere forswears. Separately, "the shipped default cap is four" names no configuration surface — env, flag, `project.yaml`, or deployment constant — and D1 does not say whether cap one is operator-reachable for the reference mode.

**Recommend.** Amend D1 and the implementation requirements: the pool is **in-guest bounded concurrency** — one scheduler in the execute/refine/author orchestrations driving at most `cap` concurrent async dispatches over the existing host imports; no new WIT, no host-side scheduler, matching "the missing piece is orchestration, not a new host WIT" in the intent. The host contribution is only what D4 already names (writer injection) plus honouring concurrent in-flight model calls. Pin the cap as **deployment policy on the launcher** (an env-derived value on the same shape as `EMERY_WRITER` / `HTTP_ADDR`, clamped to a compiled maximum, default four, `1` legal), injected guest-visible at startup; never a `plan.yaml` field, so a change home stays portable.

## F4 — `StatusBody` cannot survive a ready set, and the RFC doesn't say what replaces it

**Where:** D2 ("status and selection do not depend on a singular `active` entry"); AC1.

**Finding.** The landed wire (`crates/project/src/plan/status.rs`) is singular throughout: `active: Option<String>`, one `next_action`, one `slice` / `target` / `current_step` / `last_completed`, one literal `resume` command. `docs/reference/cli-output-shapes.md` documents that shape, and the `/emery:status` skill relays it. A ready-set scheduler with several in-progress items breaks every one of those fields, and the RFC names the requirement without the replacement projection — a breaking public wire change left to the implementer.

**Recommend.** Add the replacement shape to D2: `StatusBody` gains `in-progress[]` (each row `slice`, `phase`, and stop detail when parked) and the singular fields become the *canonical head* of the ready set — the same deterministic selection order (target, topological layer, plan order, slice, phase) that feeds the pool, so `next_action` / `resume` remain a single honest "what one command makes progress" answer and cap one reproduces today's output byte-for-byte. Update `cli-output-shapes.md` in the same change, and state that additive-field evolution is the compatibility rule.

## F5 — Writer-identity injection names no mechanism

**Where:** D4.

**Finding.** D4 requires the launcher to inject the writer id into the guest "before dispatch" and notes the wasm32 guest cannot read process environment — but doesn't say how the value crosses. The deployment already has exactly one precedent: the launcher injects the pre-bound HTTP listener address as the guest-visible `HTTP_ADDR` (WASI env on the store, not process env). Leaving the mechanism open invites a second, divergent channel.

**Recommend.** One sentence in D4: the launcher injects `EMERY_WRITER` as a guest-visible WASI environment value on the same path as `HTTP_ADDR`; `journal::writer_id()` in-guest reads that injected value and keeps `local` as the fallback, so native and guest deployments resolve identically. Explicitly reject a WIT import for identity — it is deployment configuration, not a capability.

## F6 — The Phase-A same-target rebuild has no requeue rule

**Where:** Worked examples ("the second rebuilds if its base moved"); D2 readiness; Phase A.

**Finding.** Two same-target leaves build concurrently from the current accepted CID; the first merge advances the CID; the second's completed `BuildRecord` is now stale. The RFC accepts the rebuild tax but never states the mechanics: who detects staleness, whether the stale record is a typed failure or silently unconsumed, and whether a fresh build dispatches automatically. Merge readiness ("a current accepted frontier") implies the answer without stating it.

**Recommend.** Make it a D2 sentence: a moved accepted CID changes the `build` work item's input digest, so the stale `BuildRecord` is simply never consumed by merge — no typed failure, no retraction fact; the scheduler projects the new-digest `build` item as ready and the ordinary loop rebuilds. The superseded attempt and record remain immutable audit under RFC-90 D6, and D11's harness counts the discarded attempt as coordination cost. This keeps "readiness is projected, never stored" intact.

## F7 — Cancellation and sibling-failure policy are undefined

**Where:** D1; D5 ("cancellation reaps every focused survey if proposal assembly fails"); implementation requirements ("cancellation must reap every call"); AC2.

**Finding.** The RFC requires reaping without defining it for this backend: an in-flight worker is an awaited host future over a live model/agent session. It also never says what happens to healthy in-flight siblings when one work item fails — drain them to completion or cancel them — which decides both the stop card's content and how much paid model work is discarded. The old draft's worker-inactivity timeouts were dropped in the rewrite without a replacement.

**Recommend.** Add to D1: cancellation is **cooperative drop of the guest-side future plus best-effort host abort** of the underlying model call; a cancelled operation releases its claim, persists nothing authoritative, and journals nothing (RFC-90 already guarantees discard-on-failure for build workspaces). On a work-item failure the supervisor **stops admitting new items and drains in-flight siblings to their own terminal reports** — completed sibling work stays consumable on the next run — then emits one typed stop naming every parked item; outright cancellation of siblings is reserved for operator interrupt and proposal-assembly failure (D5), where partial results cannot compose. Reinstate a per-operation inactivity timeout as an engine constant with the budget-style "conservative pin, revised from retained telemetry" language.

## F8 — Phase B's protected-input closure reads fields that do not exist

**Where:** D8 ("every contributing descendant's covered protected set"); RFC-106 D3.

**Finding.** The domain closure intersects each leaf's covered `protected-verification-inputs[]` / `protected-oracles[]`. The landed `decomposition.yaml` `Node` has neither field, and authorship (target-metadata nomination at plan authoring, operator amendment) is described in RFC-106 — which is evidence-gated and may land *after* Phase B. As written, Phase B either blocks on RFC-106 or invents the fields.

**Recommend.** Add one sentence to D8 on RFC-91's existing rule: absent protected fields encode as **canonical empty sets** — the closure over leaves that declare nothing is the empty intersection, which D8 already declares valid — so Phase B ships complete domain records with empty closures and RFC-106 populates the fields without changing the record shape or operation key. Note that the `Node` DTO gains the two optional fields (absent-as-canonical-empty, digest-stable) in Phase B, while their write path stays RFC-106.

## F9 — The journal and record deltas are not enumerated

**Where:** D7; D8; implementation requirements ("derive closed domain schemas from Rust DTOs").

**Finding.** `EventKind` is a closed taxonomy, and the RFC names exactly one new fact (`domain.convergence.recorded`) while implying more: multi-member wave open/commit payloads (member *sets* where the landed events carry one member), and the domain record's on-disk home is unnamed (`.emery/change/` path, retention, archive behaviour). "Derive closed schemas from Rust DTOs" is the right rule but names no DTO.

**Recommend.** Add a wire subsection to D8: one `DomainRound` DTO (`kind: frontier | complete`, the nine listed fields, `deny_unknown_fields`), persisted content-addressed at `.emery/change/targets/<target>/domains/<digest>.yaml`, archived with the plan; one new `EventKind::DomainConvergenceRecorded` naming target, domain, kind, record digest, and verdict. State that `target.wave.opened` / `target.merge.wave-committed` keep their names and shapes with `members[]` simply carrying more than one entry — the schema already holds a list; only `enforce_one_member` retires. No other taxonomy change in this RFC.

## F10 — Typed stops and error discriminants are unnamed

**Where:** D7; D8; AC1; AC7; `docs/reference/diagnostics.md`.

**Finding.** The landed surface names every refusal (`guest-marker-held`, `slice-claim-conflict`, `plan-refinement-required`, `target-wave-member-count`, …). RFC-96 introduces new refusals with no discriminants: a duplicate in-process operation claim, a frontier-round failure blocking a frozen wave, a complete-round failure blocking drain, and the operator act that retracts an uncommitted multi-member wave ("operator amendment" — which verb?).

**Recommend.** Name them in the RFC so `plan status` stop cards and diagnostics stay typed: `domain-frontier-failed` (stop; resume is re-running execute after the named repair), `domain-complete-failed` (sticky stop blocking drain, on the postflight-acknowledge shape), and wave retraction as an effect of the existing `emery plan amend --proposal` compare-and-set path (RFC-88 D8 already says amendment retracts the whole uncommitted wave — cite it rather than adding a verb). A duplicate in-process claim needs no wire discriminant under F2's registry decision; say so explicitly so nobody adds one.

## F11 — The synthesis shelf and staged answer have no wire sketch

**Where:** D9; D10; synthesis delivery slice.

**Finding.** `/mcp/engine/synthesis` is a route sketch. The shipped MCP surface is per-adapter (`/mcp/<axis>/<name>` via `launcher::mcp_route`); an engine-owned shelf is a new route class with no stated grant plumbing, and D10's "outcome-only answer" replaces the current synthesis answer schema (generated from `slice::answers`) with an unspecified DTO plus a staging-tree lend that does not exist for synthesis today. Both are live-eval-gated, so this is not urgent — but it is not implementable either.

**Recommend.** Keep the slice gated and add the minimum wire: the shelf is one additional engine `http_paths` route registered by the launcher beside the adapter routes, serving the embedded corpus that `crates/slice/prompts/` already compiles in, granted to the synthesis judgment the same way an adapter grants its own references route. D10's answer is a closed `{ outcome, findings[] }` DTO generated into `slice::answers`; the staging tree is an ordinary RFC-87 writable workspace lent for the change-artifact bundle, validated then promoted by the existing atomic-writer path. Defer anything beyond that to the eval results the slice is already gated on.

## F12 — AC2 binds a parked RFC, and D11 quietly depends on RFC-92

**Where:** AC2 ("RFC-99 can publish a closed branch into that scheduler…"); D11; AC10.

**Finding.** RFC-99 is parked; platform.md's parking rule says no active implementation predeclares a parked wire shape, yet AC2 makes RFC-99 compatibility an acceptance criterion nobody can test without reopening it. Separately, D11's cost projections consume `model.usage.recorded`, which only exists after RFC-92 Cut A — the RFC handles the absence honestly ("missing provider usage remains unknown") but never states the dependency direction.

**Recommend.** Rewrite the AC2 clause as a design constraint, not a criterion: "Phase A adds no build authority to refinement work items" — which is testable — and move the RFC-99 sentence to the D2 prose where the extension point is already described. In D11, add one line: cost columns populate when RFC-92 Cut A's usage facts exist; until then the harness reports time, attempts, and rebuild counts only. Neither becomes a dependency edge in platform.md.
