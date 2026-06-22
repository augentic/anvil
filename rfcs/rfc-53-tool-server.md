# RFC-53: The Tool Server — the brief-I/O surface for a direct tool-use loop

> Status: Draft (skeleton) · Implements: the effect-oriented architecture — **the judgment spine** (the native tool-use loop) · Decides: **no `eval` effect** (judgment is the native tool-use loop); the tool-use loop and the tool facade are **native** for the in-process path, with **MCP / HTTP an optional transport** (a thin guest) for off-the-shelf agents and interop; the **model client is `genai`** behind a native `ModelClient` boundary; `verify` is a **native, sandboxed seam** over closed `check` profiles returning the shared `report` · Depends: [RFC-51](rfc-51-adapter-wit.md) (the `changeset` / `revision` records), [RFC-52](rfc-52-effect.md) (the `references` shelf this loop drives), [RFC-55](rfc-55-working-tree.md) (the working tree it reads / writes against, and the native `changes()` extraction) · Relates: [RFC-58](rfc-58-eval-fleet.md) (the `ModelClient` strategies, incl. the spawned-agent sibling), [RFC-54](rfc-54-orchestration.md) (which consumes this loop for judgment operations) · Framed by: [architecture.md](architecture.md)

## Abstract

This RFC defines the **brief-I/O surface** a non-filesystem model uses to execute one brief — `resolve` for the reference shelf, `read` / `list` to scan existing code, `write` to mutate, `verify` to check itself — entirely through typed callbacks, with no filesystem access of its own. The surface (`tools`, authored in the `augentic:tools` WIT) is a **thin facade over capabilities that are already Omnia host services and native orchestration** (`wasi:filesystem` / `wasi:blobstore` / `wasi:keyvalue`, the adapter-exported `references` shelf, plus the native working-tree `changes()` extraction and a native verify seam). It is driven by a **direct tool-use loop** — the "call the model API directly" path that is the architecture's **judgment mechanism**. There is **no `eval` effect**: the loop and the facade are **native** for the default in-process path, and **MCP over HTTP is an optional transport** — a thin wasm guest — needed only when you do *not* own the loop (off-the-shelf MCP agents), when the model platform calls tools remotely, or for cross-process interop. **If you own the loop, you do not need MCP.**

## Motivation

Reference resolution is now a real adapter export — the `references` **shelf** (`resolve(id) → bytes`, [`wit/specify.wit`](../wit/specify.wit) / [RFC-52](rfc-52-effect.md)) — and a build's writes ride the `local-path` escape hatch, a real OS path lent to a filesystem-capable spawned agent ([RFC-52](rfc-52-effect.md) Risks). The escape hatch alone does not serve a **model reached directly through its API** with no filesystem: such a model cannot follow a brief's relative links or write to a tree — it can only emit text and **tool calls**. So executing a brief over a raw model API needs an explicit, typed I/O surface the model pulls on — and that surface's `resolve` is exactly the shelf, session-scoped.

Two clarifications shape the design:

- **If you own the loop, MCP is optional.** When you write the tool-use loop yourself, you declare the tools in the model's native function-calling schema and dispatch each tool call wherever you like — directly into the facade, in-process. MCP (a wire protocol plus an ecosystem standard) earns its keep only when you do *not* own both ends: off-the-shelf agents, provider-driven remote tool calls, or cross-process interop. MCP is therefore a **transport**, not the core.
- **The surface is a facade, not a new subsystem.** `resolve` / `read` / `list` are host-service reads; `write` accumulates an `edit`; `verify` is a native subprocess; `commit` yields the portable `changeset`. The substance already exists as Omnia host services and the [RFC-55](rfc-55-working-tree.md) native orchestration; this RFC just gives it a model-facing shape.

Authoring that surface yields three things the current implicit handoff does not:

- **A filesystem-free model path** — the model never holds a descriptor or an OS path; every read and write is a typed callback.
- **The `local-path` escape hatch closes** — writes become `edit`s accumulated into a portable `changeset` ([RFC-51](rfc-51-adapter-wit.md)), so the operation is node-independent with no shared mount.
- **Record/replay is preserved without an effect** — with no `eval` to mock, the model-API leg of the loop and the typed `tools` boundary are both recordable, so a brief run is a replayable fixture (the architecture's CI-determinism goal), not an out-of-band agent trace.

## Scope

**In scope:**

- The `augentic:tools` WIT: the `tools` interface (`open` / `commit` / `resolve` / `read` / `list` / `write` / `verify`).
- **Where the facade lives** — native for the in-process path (Mode A, default); a thin `mcp-server` guest exporting `wasi:http/incoming-handler` for the optional MCP / HTTP transport (Mode B).
- The **session + `changeset` model**: a session binds a base `revision` and accumulates `edit`s, host-held in `kv` (instance-per-call), committed to a portable `changeset`.
- The **`verify` seam** — a native, sandboxed, toolchain-pinned set of closed `check` profiles returning the reused `report`, driving the verify-repair loop.
- The **direct tool-use loop** that drives a model API and routes its tool calls into the facade.
- **Where judgment lives** — no `eval` effect; where the loop, the model client, and the facade live, and where record/replay sits.

**Non-goals:**

- **MCP as mandatory.** MCP is an optional transport for the cases that need it; the default path uses none.
- **Lifecycle authority.** The facade is an I/O surface only; `transition` / `journal` / locks stay in the runtime's deterministic lifecycle host service (roadmap Non-Goal: "Do not put lifecycle authority in skills, MCP servers, hosted services, or adapters").
- **The read-only Specify-state MCP server** ([roadmap.md](roadmap.md) RM-13) — a different server (exposing `plan.yaml` / `registry.yaml` / slice metadata to agents). This RFC's surface is the brief-execution I/O surface; the two share only the protocol.
- **The macro orchestration loop** — which brief runs when, the verify-repair sequence, validation — is the component's ([RFC-54](rfc-54-orchestration.md)). This RFC is one brief step's I/O.
- **The working-tree materialization backend** — checkout, object acquisition, `slice → revision` resolution — is [RFC-55](rfc-55-working-tree.md). This facade *reads from and writes against* a materialized tree; it does not materialize it.
- **The spawned-agent backend** ([RFC-58](rfc-58-eval-fleet.md)) — the filesystem-capable sibling that follows links directly and needs no tool surface.

## The model (sketch)

### Topology

```text
native tool-use loop  (you own it — the architecture's judgment mechanism)
  ├─ calls the model API directly ──────────────────────────────────────┐
  │    model API (frontier LLM / SLM)                                    │
  │      └─ emits tool calls: resolve / read / list / write / verify     │
  └─ dispatches each tool call to the FACADE ───────────────────────────┤
       facade = the `tools` surface                                      │
         • Mode A (default): NATIVE module, in-process — no guest, no MCP │
         • Mode B (optional): thin wasm guest over MCP / HTTP            │
       backed by Omnia primitives:                                       │
         wasi:filesystem / wasi:blobstore → shelf + tree                 │
         wasi:keyvalue                    → session (base + edits)        │
         native verify + RFC-55 changes() → verify + commit              │
  ◄─ model returns validated answer; loop commits → changeset (vs base) ─┘
```

The model drives I/O *within* one brief; the component drives *which* brief and the verify-repair sequence. "Prose holism" ([RFC-54](rfc-54-orchestration.md)) holds: the loop hands over a **whole brief**, and the facade only serves the I/O that brief's prose calls for.

### The WIT (`augentic:tools`)

```wit
package augentic:tools@0.1.0;

/// The brief-I/O surface a model uses to execute one brief: navigate the
/// brief's reference shelf, scan and mutate a working tree, and verify. The
/// `tools` shape is shared by both facades — a native trait for the in-process
/// path (Mode A), a guest export for the MCP / HTTP path (Mode B).
interface tools {
  use augentic:specify/types.{ revision, changeset };
  use augentic:specify/target.{ report };   // reuse build/merge judgment: severity-tiered findings + outcome

  /// Opaque session handle. Binds a base revision and owns the accumulating
  /// changeset; host-held and keyed in `kv` (guests are instance-per-call).
  type session = string;

  /// A reference handle: a brief-relative path, or a content-addressed id.
  type handle = string;

  variant error {
    not-found(string),
    invalid-request(string),
    io(string),
    internal(string),
  }

  /// A vetted verification profile. The model names a check; the host owns the
  /// exact argv (mirrors the `cargo make ci` task set). NOT free-form argv — an
  /// LLM choosing a command line is an RCE + non-determinism surface.
  enum check {
    fmt,      // cargo +nightly fmt --all -- --check
    build,    // cargo build --target wasm32-wasip2 --release  (the deploy target)
    clippy,   // cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
    test,     // cargo nextest run --all --all-features
    doc,      // cargo doc --no-deps --workspace --all-features --locked
    vet,      // cargo vet --locked
    deny,     // cargo deny --workspace check
    ci,       // the full composite gate (all of the above)
  }

  // --- session lifecycle ---
  /// Open a session bound to a base revision.
  open:    func(base: revision) -> result<session, error>;
  /// Commit the session, returning the portable changeset (edits vs base).
  commit:  func(s: session) -> result<changeset, error>;

  // --- reads (reference shelf + working tree; see base ⊕ pending edits) ---
  /// Resolve one of the brief's internal references by handle. Session-scoped
  /// facade over the adapter's stateless `references.resolve` shelf
  /// (`augentic:specify`, RFC-52) — same id space, with the session for caching.
  resolve: func(s: session, ref: handle) -> result<list<u8>, error>;
  /// Read a working-tree file.
  read:    func(s: session, path: string) -> result<list<u8>, error>;
  /// List working-tree entries under a prefix. (`%list` — `list` is a WIT keyword.)
  %list:   func(s: session, prefix: string) -> result<list<string>, error>;

  // --- writes (one edit; absent content deletes) ---
  write:   func(s: session, path: string, content: option<list<u8>>) -> result<_, error>;

  // --- verify (native, sandboxed seam; absent on a toolchain-less host) ---
  /// Run a vetted profile over the pending changeset; returns the same
  /// severity-tiered `report` as build / merge (see Verification).
  verify:  func(s: session, c: check) -> result<report, error>;
}

/// Mode B (optional): MCP over HTTP. A thin guest does JSON-RPC framing and
/// forwards each `tools/call` to the host services below. Mode A needs no
/// world — its facade is native code mirroring `tools` (see "Where the tool
/// server lives"). Served by any wasi:http host (Wasmtime serve, Spin, jco).
world mcp-server {
  export wasi:http/incoming-handler@0.2.0;
  import wasi:blobstore/blobstore@0.2.0;   // reference shelf + write payloads (content-addressed)
  import wasi:filesystem/types@0.2.0;      // working tree / local shelf
  import wasi:keyvalue/store@0.2.0;        // session: base + accumulating edits
  import augentic:verify/toolchain;        // native seam (and changes() if commit diffs a tree)
}
```

### Where the tool server lives: native facade vs guest

The tool server is **not a heavyweight component** — its capabilities are already Omnia primitives, so where it "lives" is a question about the *facade*, not the substance:

- **The substance is host services + native orchestration.** `resolve` / `read` / `list` are reads over `wasi:filesystem` / `wasi:blobstore`; the session's edit log lives in `wasi:keyvalue`; `verify` is a native subprocess (no stock WASI host expresses it); `commit`'s `changeset` is the [RFC-55](rfc-55-working-tree.md) native `changes()` extraction. None of that moves between the modes.
- **The facade is native in the default (Mode A) path.** When you own the loop, the loop is native code, so the simplest facade is a **native Rust module shaped like `tools`** that the loop calls in-process and that forwards to the host services and the native verify / `changes()`. There is **no guest** — wrapping a thin forwarder in wasm buys nothing when the caller, the verify seam, and the changeset extraction are all native already.
- **The facade is a thin guest only for the MCP / HTTP transport (Mode B).** To serve off-the-shelf MCP clients you need an HTTP server; a wasm guest exporting `wasi:http/incoming-handler` is the portable way to be one (it can even run on a separate `wasi:http` host). That guest does MCP framing and forwards to the *same* host services, with custom host imports for the inherently-native pieces (`verify`, and `changes()` if commit diffs a materialized tree).

So: **the tool server is an Omnia-backend-shaped facade — native by default, a guest only when MCP / HTTP demands one.** The `tools` WIT interface is just the shared *shape*: a native trait in Mode A, a guest export in Mode B. (If you want the in-process facade sandboxed, it *can* be a guest exporting `tools` that the native loop instantiates — sandboxing at the cost of a wasm boundary over a forwarder; not the default.)

One linkage to flag: how much of the facade is *inherently* native depends on the open [working-tree source-of-truth](#decisions-to-record-open-until-reviewed) decision. If `write` accumulates explicit `edit`s in `kv`, then `commit` is a `kv` read (guest-able); if writes land in a materialized tree and `commit` diffs it (RFC-55 `changes()`), `commit` is native. `verify` is native either way.

### How it works with the tool-use loop

1. The native loop opens a session: `open(base)` → `s`. The base `revision` is the slice's working tree ([RFC-55](rfc-55-working-tree.md)).
2. The loop calls the model API with the **whole brief** (inlined, or fetched by the model's first `resolve(brief-handle)`) and the tool schema for `resolve` / `read` / `list` / `write` / `verify`, with `s` bound to the tool context.
3. The model emits tool calls. Each is dispatched to the facade, which loads the session from `kv`, executes against `blobstore` / `filesystem`, and persists. `resolve` delegates to the adapter's exported `references` shelf; `read` / `list` see `base ⊕ pending edits`; `write` appends an `edit`.
4. When the brief calls for it, the model emits `verify(<check>)`; the host materializes the pending changeset and runs that **vetted profile** in a sandbox, returning a severity-tiered `report` (see [Verification](#verification-the-one-native-seam)). The model repairs the error-level findings and loops.
5. The model returns its final answer; the loop validates it (against the operation's report schema) and `commit(s)` → `changeset`.
6. The judgment (the answer) returns up to the component; the `changeset` flows to `merge`. Neither the descriptor nor an OS path ever reached the model.

The loop (the model client, the tool-call round-trips, the validate-and-`commit`) is **native orchestration code** — see [Native judgment and replay](#native-judgment-and-replay). The model client is [`genai`](https://github.com/jeremychone/rust-genai) behind a native `ModelClient` boundary; the loop advertises `tools` to the model's function-calling as a schema **minus the `session` arg** (the loop holds `s` and injects it — the model never sees the handle), parses the returned tool calls, dispatches them to the facade, and feeds results back as tool messages (`list<u8>` payloads encoded UTF-8 or base64 for a text model).

### Verification (the one native seam)

`verify` is the only leg that does not fall out of a stock WASI host: compiling and checking code needs `rustc` / `cargo`, a real filesystem, and (for `vet` / `deny`) network — none of which a `wasm32-wasip2` guest has. So `verify` is a **native** capability, surfaced to a Mode-B guest as the `augentic:verify/toolchain` import and called directly in Mode A. Read one `verify` call as **a single, synchronous, sandboxed GitHub Actions job whose annotations feed the model instead of a PR** — the same checks your CI already runs ([`engine/Makefile.toml`](../engine/Makefile.toml) `cargo make ci`; [`.github/workflows/ci.yaml`](../.github/workflows/ci.yaml)).

- **Materialize, then run.** The host materializes `base ⊕ pending edits` ([RFC-55](rfc-55-working-tree.md)) into a scratch tree and runs `cargo` there — the changeset *is* the unit, exactly as a CI checkout is — caching `target/` per session so repair iterations stay incremental.
- **Pinned toolchain = CI parity.** Runs honor the project's `rust-toolchain.toml` (stable + `clippy` / `rustfmt`, the `wasm32-wasip2` target), so a green `verify` predicts a green CI and replay stays reproducible.
- **Named profiles, never free-form argv.** The `check` enum maps one-to-one onto the vetted `cargo make ci` argv plus the deploy-target build (`cargo build --target wasm32-wasip2 --release` — which a native `cargo check` would not catch, and where [Omnia guardrail](https://github.com/augentic/specify-adapters/blob/main/targets/omnia/references/guardrails.md) forbidden-crate / API breakage surfaces). The model *names* a check; the host *owns* the command line — the security boundary (an LLM choosing argv is RCE) and the determinism boundary in one.
- **Structured feedback.** Each profile runs under `--message-format=json`; the host parses it into the reused `report` — `finding.rule-id` is the compiler code / lint / advisory id (`E0382`, `clippy::needless_clone`, `RUSTSEC-2024-…`), `finding.severity` lets the loop tier (critical / important = must-fix; suggestion = optional), and `outcome = failure` if any error-level finding remains. The loop feeds those findings back as a tool message; the model `write`s fixes and re-`verify`s — the verify-repair loop ([RFC-54](rfc-54-orchestration.md)).
- **Sandbox the build — the real risk.** Building / testing *generated* code executes arbitrary code on the host (`build.rs`, proc-macros at compile time, `cargo test` binaries). CI gets isolation free from ephemeral runners; this path does not, so `verify` must run each subprocess locked down (ephemeral container / microVM, restricted user, egress-deny, scratch FS, CPU / memory / wall-clock limits) and gate `test` execution especially. Pair it with `cargo deny`'s ban-list to reject forbidden crates *before* they compile.
- **Tier by latency.** CI runs once per push; this loop runs `verify` many times per brief. Put `clippy` in the inner repair loop (seconds, warm `target/`) and reserve the full `ci` profile (test + doc + vet + deny) as the final gate before `commit`.
- **Determinism / replay.** `verify` is the least deterministic seam (timestamps, parallelism, `vet` / `deny` advisory-DB fetches). Mirror the `ModelClient` capture: pin the toolchain (done), pin or vendor the advisory-DB snapshot (or treat `vet` / `deny` as out-of-loop final gates), and record `(changeset-digest, check) → report` so replay serves recorded findings without spawning.

### Transport modes (local vs hosted)

`tools` is the same in both modes; only how the model reaches the facade differs.

- **Mode A — local (native, in-process).** The native loop dispatches the model's tool calls *directly into the native facade* — no JSON-RPC, no HTTP, no MCP; "MCP" would only ever be the tool *schema* advertised to the model API's function-calling. Lowest latency, no reachable endpoint, scales by replication (each worker runs its own loop + facade), and nothing central to secure.
- **Mode B — hosted (MCP over HTTP).** The facade is a thin guest served through `wasi:http/incoming-handler`. The model platform's remote MCP connector — or any external MCP client — drives the tool calls over JSON-RPC. This is the interoperable path (off-the-shelf MCP agents work) and the only one that *needs* MCP; it requires the endpoint be reachable by the caller. Two sub-flavors: *client-driven* (your native loop + `genai` still drive the model; an MCP client forwards tool calls over HTTP) and *provider-driven* (the platform's connector runs the whole loop against the endpoint) — the provider-driven flavor is the one `genai` may not cover (see [Decisions](#decisions-to-record-open-until-reviewed)).

**Guidance:** local desktop and own-the-loop fleet runs use **Mode A**; integrations with agents or platforms you do not control use **Mode B**. Reachability decides it — a model provider's servers cannot reach a desktop's localhost, so a provider-driven loop is only viable when the facade is hosted. Because `tools` is the same, an operation authored against Mode A runs unchanged under Mode B.

### Instance-per-call and host-held session state

Guests hold nothing between calls, and `wasi:http` serving instantiates per request, so the session (base + accumulating edits) lives in `wasi:keyvalue`, keyed by the session id. Each tool call is a fresh instance (Mode B) or a fresh native invocation (Mode A) that loads the session, applies the op, and writes it back — "stateless guests, host-held state" ([architecture.md](architecture.md)) applied to the tool loop.

## Native judgment and replay

**Judgment is the native tool-use loop — there is no `eval` effect.** The native orchestration layer drives the model API directly: no `eval` WIT import, no model host to bind. "Turn a brief into a model interaction and return a validated answer" is the loop's job, and it lives in native orchestration rather than behind an effect. The fleet ([RFC-58](rfc-58-eval-fleet.md)) is `ModelClient` strategies behind the native boundary, not backends in a host slot. The split is deliberate:

- **The model client lives in native orchestration.** The loop owns the model API conversation (egress, retries, the tool-call round-trips, validate-and-`commit`) as native code, alongside the other native-orchestration concerns (`slice → revision` resolution, `changeset` extraction, forge push — [RFC-55](rfc-55-working-tree.md)). The client is [`genai`](https://github.com/jeremychone/rust-genai) (one API over OpenAI / Anthropic / Gemini / Ollama / Groq / …) behind a small native `ModelClient` trait. That trait keeps the *model leg* orthogonal to the *MCP transport* — `genai` drives the conversation in Mode A and client-driven Mode B alike, while MCP framing (Mode B) is the facade-guest's job, not the client's — and it is the single seam carrying record/replay (below) and law 2 (the vendor model id is `ModelClient` config, never crossing `tools`).
- **The facade is native or a thin guest.** The portable, capability-sandboxed I/O implementation is native by default and a guest only for Mode B (see [Where the tool server lives](#where-the-tool-server-lives-native-facade-vs-guest)). Native owns the *loop*; the facade owns the *tools*.

**Record/replay rides the `ModelClient` boundary.** The replay seam is the **model-API leg of the loop** — record `(brief + tool-call transcript) → final answer` keyed by the inputs, with the facade deterministic given its session state. The typed `tools` boundary is independently recordable too. So the architecture's CI-determinism goal holds; the capture point is the model client. (Exact capture point is an open decision below.) Concretely the capture brackets the `ModelClient` boundary: a `Recording` impl logs `(request) → (response)` fixtures around `genai`, a `Replay` impl serves them — so determinism never rides on `genai`'s own wire.

**The cost (law 2).** A model client lives in Specify's native orchestration rather than behind an effect. This is acceptable because that code is the *Specify binary's* native layer, not Omnia core — Omnia core stays generic and model-agnostic, so [law 2](architecture.md) holds *at the runtime floor*. The limitation: switching frontier ↔ SLM ↔ spawned-agent is a native-orchestration change, not a backend swap behind a stable effect. In practice `genai` softens this: frontier ↔ SLM ↔ hosted-API is a model-id / config change *inside* the `ModelClient` impl, so only the ↔ spawned-agent shape (a different backend — [RFC-58](rfc-58-eval-fleet.md)) is a genuine native-orchestration change.

## Decisions to record (open until reviewed)

- **Resolved — no `eval` effect.** Judgment is the native tool-use loop; see [Native judgment and replay](#native-judgment-and-replay). Open sub-question: the exact **record/replay capture point** — the model-API leg of the loop (record `(brief + transcript) → answer`) vs the typed `tools` boundary, or both.
- **Resolved — facade location and transport.** Default = **Mode A**: a **native** facade the native loop calls in-process, no MCP. Optional = **Mode B**: a thin `mcp-server` guest exporting `wasi:http/incoming-handler`, for agents / platforms you do not own. See [Where the tool server lives](#where-the-tool-server-lives-native-facade-vs-guest). Open sub-question: whether a *sandboxed in-process guest* (a third option between native Mode A and HTTP Mode B) is ever worth it.
- **Model client — `genai` behind a native `ModelClient` boundary.** The default model client is [`genai`](https://github.com/jeremychone/rust-genai) (one API over OpenAI / Anthropic / Gemini / Ollama / Groq / …), wrapped by a small native `ModelClient` trait — the single seam for record/replay (the `Recording` / `Replay` impls) and law 2 (model id is config, never in `tools`). Open sub-questions: the **provider-driven Mode B gap** — a hosted *remote-MCP-connector* loop (the provider runs the tool calls against the endpoint) is a model-egress feature `genai` may not express, so that path may bypass the client or need a provider-specific param; and pinning a **pre-1.0** dependency that brings `reqwest` + `tokio` into the native binary (see [Risks](#risks-and-invariants)).
- **Reference id space.** Path-as-id (`resolve("../references/foo.md")`, the facade resolves against the shelf) vs content-addressed handle (`resolve("sha256:…")`, links rewritten at serve time — matches `artifact` in [`wit/specify.wit`](../wit/specify.wit)). The latter is cacheable and portable; the former is less work.
- **Working-tree source of truth.** Whether `read` / `list` / `write` operate over the [RFC-55](rfc-55-working-tree.md) materialized tree (via `wasi:filesystem`) or a `blobstore`-projected view, and how the committed `changeset` reconciles with RFC-55's `changes()`. This also decides how much of `commit` is native (see [Where the tool server lives](#where-the-tool-server-lives-native-facade-vs-guest)).
- **Resolved — the `verify` seam.** `verify` is a **native** capability (the native orchestration layer spawns the subprocess directly), surfaced to a Mode-B guest as the `augentic:verify/toolchain` import and called directly in Mode A. It takes a closed `check` profile (not free-form argv) the host maps to the vetted `cargo make ci` argv, parses `--message-format=json` into a reused `report`, and runs sandboxed (see [Verification](#verification-the-one-native-seam)). Absent on a toolchain-less host, it degrades Class-2 (build / merge) briefs — the same capability signal as RFC-52's `local-path: none`. Open sub-questions: the **sandbox mechanism** (ephemeral container / microVM vs restricted-user) and **advisory-DB pinning** for `vet` / `deny` replay determinism.
- **Error taxonomy.** The `error` variant shape and how it maps to MCP error responses (Mode B) and to the operation's typed `error`.
- **Session GC.** Session lifetime, idle eviction, and the relationship to `specify archive prune`.
- **Resolved — `report` reuse.** `verify` returns the existing `report` (`{ outcome, findings: list<finding> }`, `finding` carrying `severity`) from [`wit/specify.wit`](../wit/specify.wit) `interface target`, not a redeclared `diagnostics` — so verify shares the severity-tiered judgment currency of `build` / `merge`. The native implementation maps cargo's JSON onto the `specify-diagnostics` `Diagnostic` substrate (which already renders `json` / `pretty` / `github` / `compact`).

## Phased plan

1. Author the `augentic:tools` WIT (the `tools` interface), reusing `revision` / `changeset` from `augentic:specify`.
2. Implement the **native facade** (Mode A) over the host services plus the native verify / `changes()` — `open` / `resolve` / `read` / `list` / `write` / `commit`, session edits in `kv`, read-after-write overlay. No guest, no transport.
3. Build the native tool-use loop against a real model API; prove a **read-only** operation end-to-end (candidate: a source `extract` — `resolve` the shelf, emit Evidence, `commit` an empty changeset).
4. Add the `verify` seam — the `check` profiles over a sandboxed, toolchain-pinned scratch tree, parsing `--message-format=json` into `report`; prove a **build** operation with a verify-repair loop entirely over the facade (no `local-path`).
5. Add the optional `mcp-server` guest (Mode B) exporting `wasi:http/incoming-handler`; prove an off-the-shelf MCP client drives the *same* `tools` with no change to the loop's logic.
6. Add record/replay at the **model-API leg** of the loop (capture `(brief + tool transcript) → answer`); prove an operation replays deterministically with the model client in replay mode and the facade live.

## Acceptance criteria

1. A non-filesystem model executes a whole brief through the `tools` surface — navigating the reference shelf via `resolve`, scanning via `read` / `list`, mutating via `write` — with no descriptor and no OS path.
2. Writes accumulate into a portable `changeset` against the base `revision`; the `local-path` escape hatch is unused on this path.
3. Session state is host-held in `kv`; nothing holds it between calls (instance-per-call / per-invocation).
4. The facade carries **no** lifecycle authority and **no** adapter name or vendor model id (law 2; roadmap Non-Goal).
5. The default path runs **native in-process with no MCP** (Mode A); the *same* `tools` shape is reachable via the optional MCP / HTTP guest (Mode B) with no change to the loop's logic.
6. No `eval` effect is introduced; judgment runs through the native tool-use loop, and one operation replays deterministically with the model-API leg — the `ModelClient` boundary — in replay mode.
7. `make lint` and `cargo make ci` stay green.

## Risks and invariants

- **No lifecycle authority.** The facade is I/O only; transitions, journalling, and locks stay on the deterministic floor (roadmap Non-Goal).
- **Law 1 / 2 at the surface.** `tools` carries no adapter names, no taxonomy, no vendor model ids; judgment crosses as the model's typed answer, never as runtime knowledge.
- **MCP stays optional.** Treating MCP as mandatory re-introduces a wire and an endpoint you do not need when you own the loop; Mode A (native, in-process) is the default.
- **Prose holism.** The loop hands over whole briefs; the facade never fragments the prompt — it only serves the I/O the brief's prose drives ([RFC-54](rfc-54-orchestration.md)).
- **Session state is host-held.** Instance-per-call forbids in-memory session state; a leaked session is a regression.
- **`verify` is the one native seam.** Reads, writes, and reference resolution fall out of stock hosts; toolchain execution does not, and is simply absent on a toolchain-less host — Class-2 briefs degrade there.
- **Verify executes untrusted code — sandbox it.** Building / testing generated code runs `build.rs`, proc-macros, and test binaries with the host's privileges; an unsandboxed `verify` is the most dangerous surface in this RFC. Isolate every verify subprocess, gate `test` execution, and pair with `cargo deny`'s ban-list.
- **`verify` profiles stay closed.** The model names a `check`; it never supplies argv. Re-introducing free-form commands re-opens an RCE + non-determinism surface.
- **Record/replay must hold at the `ModelClient` seam.** With no `eval` to mock, determinism rides on capturing the model-API leg; a loop that bypasses the capture point silently breaks replay.
- **Keep the model client out of Omnia core.** The native loop puts a model client ([`genai`](https://github.com/jeremychone/rust-genai)) in Specify's native orchestration; it must stay there — behind the `ModelClient` boundary — and never leak a vendor SDK or model id into Omnia core or `tools` (law 2 at the floor).
- **The model client widens the native dependency surface.** `genai` pulls `reqwest` + a full `tokio` into the otherwise-lean native binary (today `ureq` + a minimal `tokio`), and the synchronous CLI must bridge to its async API (a `block_on` at the loop boundary). The wider `cargo-vet` / `cargo-deny` surface and a pre-1.0 pin are the price of not hand-rolling per-provider tool-call plumbing.
