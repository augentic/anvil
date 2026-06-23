# RFC-58: Model Backends — frontier, spawned agent, SLM, and replay behind `wasi-model`

> Status: Draft · Order 8 of 8 · Parallel (after [RFC-53](rfc-53-wasi-model.md)) · Depends: [RFC-53](rfc-53-wasi-model.md) · Enables: [RFC-18](future/rfc-18-slm.md) · Owns: the model backend set and the router

## Abstract

The `wasi-model` host ([RFC-53](rfc-53-wasi-model.md)) dispatches `eval` to a model backend. This RFC is the backend set: a **frontier / hosted** backend (via [`genai`](https://github.com/jeremychone/rust-genai)), a **spawned-agent** backend, the **replay** backend for CI, and (via [RFC-18](future/rfc-18-slm.md)) a **local SLM**; plus a **router** that picks one per call by difficulty and cost. These deliver the interactive and headless deployment modes and the cost ratchet that migrates work down the fleet. It needs only the `wasi-model` host, so it proceeds in parallel with the runtime move.

## The model

The backend is the single seam the model is reached through; the fleet lives inside it, and the model id never crosses `eval`.

- **Frontier / hosted** — hard synthesis and review, through a hosted API via `genai` (one API over OpenAI / Anthropic / Gemini / Ollama / …). Switching frontier ↔ hosted ↔ SLM is a config change inside this backend.
- **Spawned agent** — the native layer spawns a fresh, context-free agent session, hands it the brief, and parses the validated answer. It owns its own tool loop and reads and writes the working tree through the `local-path` it is lent ([RFC-55](rfc-55-working-tree.md)). This is also the interactive path: an editor command shells out to the binary, which spawns the session — a separate conversation, never the operator's transcript.
- **Replay** — serves recorded `(prompt + tool transcript) -> answer` fixtures; a recording backend captures them around the live model. This is the CI / testing mode.
- **Router** — picks a backend per call, keyed on the brief `path` or an abstract difficulty hint, never a vendor model id.

## Deployment modes

- **Interactive** — frontier API or spawned agent, against concrete artifacts.
- **Headless** — hosted API or local SLM at fleet scale, no editor in the loop.
- **CI / testing** — the replay backend serves recorded fixtures.

## Scope

- The frontier (`genai`), spawned-agent, and replay backends behind the `wasi-model` boundary.
- The router and its abstract decision key.
- The interactive and headless deployment modes.

## Open questions

- The routing key (brief `path` / difficulty) and what carries the difficulty hint.
- The spawned-agent protocol: how a session is spawned, handed the brief, and made to return a schema-valid answer, and how it consumes the prose shelf.
- The record/replay capture point for a spawned-agent backend that owns its own loop.
- The constrained-decoding hook a non-agent SLM backend uses to keep typed reports schema-valid ([RFC-18](future/rfc-18-slm.md)).

## Acceptance criteria

1. At least two real backends (e.g. frontier API + spawned agent) sit behind the one `wasi-model` boundary, selected by config.
2. The interactive and headless modes both run a real operation; CI replays unchanged via the replay backend.
3. The router keys on brief `path` / difficulty, never a vendor model id; no vendor name reaches Omnia core.
4. Every backend's run is recordable and replays deterministically.
5. `make lint` and `cargo make ci` stay green.

## Risks and invariants

- **Vendor coupling stays behind the boundary.** Any one model is one backend behind `wasi-model`, never above it.
- **Router stays abstract.** Its key is difficulty, not a vendor id.
- **Spawned process management.** Sessions stay robust and context-free; a leaked transcript reintroduces the dependency the architecture sheds.
- **The embedded topology is a non-goal.** Judgment never runs inside the operator's live editor session.
