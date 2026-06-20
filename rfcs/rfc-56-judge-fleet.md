# RFC-56: The judge Fleet — Dispatch, Router, and Frontier Backends

> Status: Draft (skeleton) · Implements: the effect-oriented architecture (the model fleet) · Depends: RFC-52 (the `judge` interface + replay backend) · Parallel to: RFC-53 / RFC-55 (needs only the seam, not the runtime move) · Plugs into: [RFC-54](rfc-54-model-host.md) (the Omnia model-host backend contract) · Enables: [RFC-18](future/rfc-18-slm.md) (the SLM fleet member)

## Abstract

RFC-52 stops at a typed `judge` seam with a single real backend: the replay stub. This RFC turns that seam into the **model fleet** the architecture describes: the single `judge` host that fans out internally to a fleet, the **router** that picks a backend per call by difficulty and cost, and the **frontier backends** — a hosted inference API and a *spawned agent* CLI / SDK session. Standing these up is what delivers the **interactive** and **headless** deployment modes (CI already falls out of the RFC-52 replay backend), and it is the precondition for the cost ratchet that pushes work down onto cheaper backends.

## Motivation

The replay backend proves the seam is mockable; it does no real work. Every payoff the architecture hangs on `judge` — running real briefs interactively or at fleet scale, and migrating calls frontier → SLM → deterministic — needs real backends behind the one interface. Naming the fleet here, behind the RFC-52 contract, is what keeps "deployment modes are backend swaps" and "vendor coupling stays behind the interface" (law 2) true in practice rather than on paper.

## Scope

**In scope:** the fan-out (one `judge` host, an internal fleet — never a second host binding); the router and its decision key; the frontier inference-API backend; the spawned-agent backend and the **spawned** topology; and the **interactive** and **headless** deployment modes that fall out of them.

### Non-goals

- **The SLM member is out of scope.** The local SLM, constrained decoding, and the scorer live in [RFC-18](future/rfc-18-slm.md); this RFC defines the fleet it plugs into.
- **No new effect.** The fleet is the *backend* of the existing RFC-52 `judge` host; it adds no guest-visible interface and requires no second `judge` binding (architecture: one backend per host).
- **The embedded topology is explicitly not a goal.** Running `judge` inside the operator's live editor session re-couples judgment to the transcript; the architecture sheds that dependency deliberately.
- **No async commitment.** Streaming `judge` and concurrent slices are the only reasons to adopt the Component-Model async path; gate that to its own decision, not this RFC's baseline.

## The model (sketch)

The **model service** is the single materialization of the `judge` host; the fleet lives *inside* it. A call carries a `brief-path` handle and a typed `request`; the service routes it to a fleet member and returns a typed report — the guest never names a model. The service registers into Omnia's model-host slot as one `ModelBackend` ([RFC-54](rfc-54-model-host.md)) — Omnia sees a single backend; the fleet fans out within it.

- **Frontier LLM** — hard synthesis and review, reached either through a hosted inference API or by spawning a headless agent CLI / SDK session.
- **Spawned agent (topology).** Omnia spawns a *fresh, context-free* agent session as the backend, hands it the `brief-path` + `request`, and parses the typed report back. This is also the **interactive** path: an editor command shells out to the runtime, which spawns the session — a separate conversation, never the operator's transcript.
- **Headless.** The same call with the backend bound to a hosted API (or, via RFC-18, a local SLM) — no editor in the loop, the same operation at fleet scale.
- **Router.** Chooses a member per call; the decision keys on the `brief-path` or an abstract difficulty hint, **never a vendor model id**, so the choice stays behind the interface.

## Decisions to record (open until reviewed)

- **Routing key.** Confirm the router keys on `brief-path` / abstract difficulty, never a vendor id (the signature-level detail the architecture deferred from RFC-52 to the fleet). What carries the difficulty hint, and who sets it.
- **Spawned-agent protocol.** How a fresh session is spawned, handed the `brief-path` + `request`, sandboxed for filesystem reads, and made to return a schema-valid report; how failures map to a typed `adapter-error`.
- **Record / replay capture point.** Where the fleet records `(brief-path, request) -> output` so any backend's run is replayable (the RFC-52 backend is the consumer of these recordings).
- **Constrained-decoding hook.** The forward-compatible seam a non-agent completion backend (RFC-18's SLM) uses to keep typed reports schema-valid.
- **Ratchet bookkeeping.** How a call is marked eligible to migrate down the fleet once it proves reliably verifiable.

## Phased plan

1. Add the frontier inference-API backend behind the RFC-52 `judge` host; prove a real operation runs headless.
2. Add the spawned-agent backend and wire the interactive path (editor → runtime → spawned session).
3. Add the router (difficulty / cost) and the record / replay capture hook; prove backend selection is config-driven and recorded.

## Acceptance criteria

1. At least two real `judge` backends (e.g. inference API + spawned agent) sit behind the *one* RFC-52 interface, selected by deployment config — no second `judge` host.
2. The interactive and headless deployment modes both run a real operation; CI / replay still works unchanged.
3. The router keys on `brief-path` / difficulty, never a vendor model id; no vendor name appears in the contract (law 2).
4. Every backend's run is recordable and replays deterministically through the RFC-52 backend.
5. `make lint` and `cargo make ci` stay green.

## Risks and invariants

- **Vendor coupling behind the interface.** Any one brain is one backend, never the interface; regressing this breaks the LLM / SLM / deterministic fleet the architecture protects.
- **Router complexity.** The router is the one place that could accrete policy; keep its key abstract (difficulty), not vendor-specific.
- **Spawned-process management.** Session spawning must stay robust and context-free; a leaked transcript re-introduces the dependency context-independence sheds.
- **Async stays gated.** The synchronous ABI suffices for the baseline; adopt async only when streaming `judge` or concurrent slices are the goal.
