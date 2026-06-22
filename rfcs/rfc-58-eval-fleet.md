# RFC-58: The Model Fleet — `ModelClient` Strategies, Router, and Frontier Backends

> Status: Draft (skeleton) · Implements: the effect-oriented architecture (the model fleet) · Depends: [RFC-53](rfc-53-tool-server.md) (the native tool-use loop + the `ModelClient` boundary) · Parallel to: RFC-54 / RFC-56 (needs only the `ModelClient` boundary, not the runtime move) · Enables: [RFC-18](future/rfc-18-slm.md) (the SLM fleet member)

## Abstract

The native tool-use loop ([RFC-53](rfc-53-tool-server.md)) reaches a model through a small native **`ModelClient`** trait. This RFC turns that single boundary into the **model fleet** the architecture describes: the native **router** that picks a strategy per call by difficulty and cost, and the **strategies** behind it — a hosted inference API (via [`genai`](https://github.com/jeremychone/rust-genai)), a *spawned agent* CLI / SDK session, the `Replay` strategy for CI, and (via [RFC-18](future/rfc-18-slm.md)) a local SLM. Standing these up is what delivers the **interactive** and **headless** deployment modes (CI already falls out of the `Replay` strategy), and it is the precondition for the cost ratchet that pushes work down onto cheaper strategies.

## Motivation

The `Replay` strategy proves the boundary is mockable; it does no real work. Every payoff the architecture hangs on judgment — running real briefs interactively or at fleet scale, and migrating calls frontier → SLM → deterministic — needs real strategies behind the one `ModelClient` boundary. Naming the fleet here, behind that native boundary, is what keeps "deployment modes are strategy swaps" and "the model id stays behind the boundary" (law 2 at the floor) true in practice rather than on paper.

## Scope

**In scope:** the fleet behind one `ModelClient` boundary (strategies selected by config, never a guest-visible interface); the router and its decision key; the frontier inference-API strategy (via `genai`); the spawned-agent strategy and the **spawned** topology; and the **interactive** and **headless** deployment modes that fall out of them.

### Non-goals

- **The SLM member is out of scope.** The local SLM, constrained decoding, and the scorer live in [RFC-18](future/rfc-18-slm.md); this RFC defines the fleet it plugs into.
- **No effect, no host.** The fleet is native code behind the `ModelClient` boundary ([RFC-53](rfc-53-tool-server.md)); it adds no guest-visible interface, no `eval` effect, and no Omnia host. The model id never reaches Omnia core (law 2 at the floor).
- **The embedded topology is explicitly not a goal.** Running judgment inside the operator's live editor session re-couples it to the transcript; the architecture sheds that dependency deliberately.
- **No async commitment.** Streaming output and concurrent slices are the only reasons to adopt the Component-Model async path; gate that to its own decision, not this RFC's baseline.

## The model (sketch)

The **`ModelClient`** is the single boundary the native loop reaches the model through; the fleet lives *inside* it. The loop hands it a brief and the tool surface; the router picks a strategy and returns a validated answer — the loop's callers never name a model. There is no Omnia host and no second binding: the router is native code behind one trait.

- **Frontier LLM** — hard synthesis and review, reached either through a hosted inference API (via `genai`) or by spawning a headless agent CLI / SDK session.
- **Spawned agent (topology).** The native layer spawns a *fresh, context-free* agent session as the strategy, hands it the brief, and parses the validated answer back. This is also the **interactive** path: an editor command shells out to the binary, which spawns the session — a separate conversation, never the operator's transcript. It is the one strategy with a shape distinct from the `genai`-driven API strategies: it owns its own tool loop and reads / writes the working tree through the `local-path` the native layer provisions ([RFC-55](rfc-55-working-tree.md)).
- **Headless.** The same loop with the strategy bound to a hosted API (or, via RFC-18, a local SLM) — no editor in the loop, the same operation at fleet scale.
- **Router.** Chooses a strategy per call; the decision keys on the brief `path` or an abstract difficulty hint, **never a vendor model id**, so the choice stays behind the boundary.

## Decisions to record (open until reviewed)

- **Routing key.** Confirm the router keys on the brief `path` / abstract difficulty, never a vendor id. What carries the difficulty hint, and who sets it.
- **Spawned-agent protocol.** How a fresh session is spawned, handed the brief, sandboxed for filesystem reads, and made to return a schema-valid answer; how failures map to a typed `error`.
- **Record / replay capture point.** Settled in [RFC-53](rfc-53-tool-server.md): the `ModelClient` boundary records `(brief + tool transcript) → answer`, and the `Replay` strategy serves them. Open: whether a spawned-agent strategy (which owns its own loop) records at the same boundary or needs a transcript capture of its own.
- **Constrained-decoding hook.** The forward-compatible seam a non-agent completion strategy (RFC-18's SLM) uses to keep typed reports schema-valid.
- **Ratchet bookkeeping.** How a call is marked eligible to migrate down the fleet once it proves reliably verifiable.

## Phased plan

1. Add the frontier inference-API strategy (via `genai`) behind the `ModelClient` boundary; prove a real operation runs headless.
2. Add the spawned-agent strategy and wire the interactive path (editor → binary → spawned session).
3. Add the native router (difficulty / cost); prove strategy selection is config-driven and that runs record / replay at the `ModelClient` boundary.

## Acceptance criteria

1. At least two real strategies (e.g. inference API + spawned agent) sit behind the *one* `ModelClient` boundary, selected by deployment config — no `eval` effect, no Omnia model host.
2. The interactive and headless deployment modes both run a real operation; CI / replay still works unchanged via the `Replay` strategy.
3. The router keys on the brief `path` / difficulty, never a vendor model id; no vendor name reaches Omnia core (law 2 at the floor).
4. Every strategy's run is recordable and replays deterministically at the `ModelClient` boundary ([RFC-53](rfc-53-tool-server.md)).
5. `make lint` and `cargo make ci` stay green.

## Risks and invariants

- **Vendor coupling behind the boundary.** Any one brain is one strategy behind the `ModelClient` trait, never above it; regressing this leaks a model id toward Omnia core and breaks the LLM / SLM / deterministic fleet the architecture protects.
- **Router complexity.** The router is the one place that could accrete policy; keep its key abstract (difficulty), not vendor-specific.
- **Spawned-process management.** Session spawning must stay robust and context-free; a leaked transcript re-introduces the dependency context-independence sheds.
- **Async stays gated.** The synchronous path suffices for the baseline; adopt async only when streaming output or concurrent slices are the goal.
