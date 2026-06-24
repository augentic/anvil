# RFC-53: The `wasi-model` Host — `eval`, the tool loop, and the model backend

> Status: Draft · Order 3 of 8 · Stage S2–S3 · Depends: [RFC-51](rfc-51-adapter-wit.md), [RFC-52](rfc-52-effect.md) · Enables: [RFC-54](rfc-54-orchestration.md), [RFC-58](rfc-58-model-backends.md) · Owns: judgment as a host effect

## Abstract

Judgment is a host effect. Omnia exposes a `wasi-model` host whose `eval` export a guest calls to have a prompt evaluated:

```wit
eval: func(prompt: prompt) -> result<answer, error>;
```

Behind the host sits a swappable **model backend** ([RFC-58](rfc-58-model-backends.md)). The backend runs an LLM tool-use loop: it drives a model through its API, advertises a typed tool surface, dispatches the model's tool calls, runs the verify-repair cycle, and returns a validated, typed answer to the calling guest — which treats `eval` like any other host call and never sees a model id.

## The tool surface

Within one `eval`, the backend services the model's tool calls:

- **`resolve(reference)`** — follow a brief's internal reference. The backend selects the adapter whose brief is being evaluated, instantiates a fresh instance, and calls its exported `references` shelf ([RFC-51](rfc-51-adapter-wit.md)) — host-mediated dynamic linking ([RFC-56](rfc-56-runtime-move.md)). Instance-per-call, so the resolution is isolated from the calling guest.
- **`read` / `list` / `write`** — scan and mutate the working tree ([RFC-52](rfc-52-effect.md)); `write` accumulates an `edit`. The model never holds a descriptor or an OS path. (A filesystem-capable spawned-agent backend instead reads and writes the tree directly through its `local-path`.)
- **`verify(check)`** — run a vetted, sandboxed check profile and feed the severity-tiered `report` back; the model repairs and re-verifies.

A session binds a base `revision` and its accumulating `edit`s, held in `wasi:keyvalue` (instance-per-call). On completion the backend validates the model's answer against the operation's report type, and the caller extracts the `changeset`.

## `verify` — the one native seam

`verify` compiles and checks generated code, which a `wasm32-wasip2` guest cannot do, so it is native. The model names a closed `check` profile (`fmt` / `build` / `clippy` / `test` / `doc` / `vet` / `deny` / `ci`) — never free-form argv (an LLM choosing a command line is an RCE surface). The host owns the argv (mirroring `cargo make ci` plus the `wasm32-wasip2` deploy build), runs each profile sandboxed (ephemeral isolation, egress-deny, resource limits), parses `--message-format=json` into the shared `report` (`finding.rule-id` = compiler / lint / advisory id; `finding.severity` tiers must-fix vs optional), and caches `target/` per session. Absent on a toolchain-less node — build / merge briefs degrade there, a clean capability signal.

## The model backend boundary

The backend is the single seam carrying the model API conversation, the vendor model id, and record/replay. The default backend drives [`genai`](https://github.com/jeremychone/rust-genai) (one API over OpenAI / Anthropic / Gemini / Ollama / …). A recording backend logs `(prompt + tool transcript) -> answer`; the replay backend serves them, making any operation a deterministic CI fixture. The full backend set and the router are [RFC-58](rfc-58-model-backends.md).

## Scope

- The `wasi-model` host interface (`eval`) and the model-backend trait it dispatches to.
- The tool loop: model API conversation, tool-call dispatch (`resolve` / `read` / `list` / `write` / `verify`), session state in `wasi:keyvalue`, answer validation.
- The `verify` seam: closed `check` profiles, sandboxed execution, `report` mapping.
- The record/replay capture point at the backend boundary.

## Acceptance criteria

1. A guest evaluates a brief by calling `eval`; the backend drives the model through the tool surface and returns a validated, typed answer.
2. `resolve` reaches the adapter's `references` shelf by host-mediated dynamic linking, instance-per-call; `read` / `list` / `write` operate over the working tree; the model holds no descriptor or OS path.
3. `verify` runs closed, sandboxed profiles and returns the shared `report`; the verify-repair loop converges.
4. The vendor model id lives only in the backend; one operation replays deterministically via the replay backend.
5. `make lint` and `cargo make ci` stay green.

## Risks and invariants

- **Law 2 at the floor.** The model id and vendor SDK live in the `wasi-model` backend, never in Omnia core or the typed contract.
- **`verify` executes untrusted code.** Every profile runs sandboxed; `test` is gated; `cargo deny` rejects forbidden crates before they compile.
- **Closed profiles.** The model names a `check`; it never supplies argv.
- **Instance-per-call.** Session state lives in `wasi:keyvalue`; a leaked in-memory session is a regression.
- **Optional MCP transport.** Off-the-shelf MCP agents or a pure-wasi host can reach the same tool surface through a thin `wasi:http` guest; this is a deferred transport, not the default path.
