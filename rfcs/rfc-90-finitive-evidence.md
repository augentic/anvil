# RFC-90 scoping input: evidence from the Finitive local-model harness

> Status: Discussion input — not an RFC. Feeds scoping for [RFC-90 Verify Profiles](rfc-90-verify-profiles.md); touches [RFC-91](rfc-91-concurrent-execution.md) and [RFC-18](future/rfc-18-slm.md) where noted.
>
> Purpose: give an implementing agent or engineer the empirical context from an external local-model harness experiment ("Finitive") and a concrete list of recommended RFC-90 adjustments to scope. The experiment's code is not available to readers of this document; every finding it contributes is described inline and this document is the complete record. Each recommendation names the RFC decision it touches and the Emery seam it lands in.

## Context

**RFC-90 (Verify Profiles)** defines host-owned verification: the model requests a closed profile name (`fmt`, `build`, `clippy`, `test`, `doc`, `vet`, `deny`, `ci`), the host selects vetted commands from the bound target/platform, runs them in a disposable RFC-87 workspace against the candidate snapshot, and returns normalized findings (`source: tool`, `kind: violation`). The model repairs within a budget. Decisions D1–D8 in the RFC.

**Finitive** is a private research harness (Python) for spec → Rust code generation with a small local model (`qwen3-coder:30b` served by Ollama), actively developed along a research path with every design decision pinned by instrumented A/B runs. Its architecture, in enough detail to evaluate the evidence without repo access:

- **No agentic tool loop.** The model produces one-shot file draws from a single assembled prompt; resampling (temperature-ramped attempts × draws, defaults 4 × 3) is the search engine.
- **Host-owned deterministic gates.** Acceptance is never the model's claim: every draw is screened (path bounds), written, then gated by `cargo check`, a frozen oracle test suite (`cargo test` against tests the model cannot write), and structural scans. All green → a git commit is the acceptance record; red → structured feedback into the next draw; draws exhausted → hard rollback (`git reset --hard`).
- **Structured diagnostics.** Compile failures are re-read via rustc `--message-format=json` for error codes, spans, and machine-applicable suggestions; a bounded (~4,000-char) human-readable tail is the fallback channel.
- **Measured decisions.** Every loop-policy mechanism (feedback shaping, budget rules, fix application) was adopted or rejected via A/B runs read as *distributions* of draws-to-pass, recorded in an append-only decision log. Several plausible mechanisms were falsified and removed.

Roughly: a working, instrumented prototype of RFC-90's verify-and-repair loop, minus the sandbox.

**Headline:** Finitive independently converged on RFC-90's core premise — the model never supplies commands; closed host gates judge everything; repair runs within a bounded budget — and demonstrates it working with a small local model. The premise is validated. The evidence below is about what the RFC under-specifies and one place it over-specifies.

## Part 1 — Decisions the evidence directly validates (no change)

| RFC-90 decision | Finitive corroboration |
| --- | --- |
| **D1/D2** closed profiles, host-selected commands | Finitive's gates are a closed, named set the host runs; the model never emits argv, flags, or toolchain choices. Works in practice, even for a 30B local model. |
| **D6** structured parsers preferred, bounded raw fallback | Independently converged on the same rule: rustc `--message-format=json` is the primary diagnostics channel; a bounded ~4,000-char raw tail is the fallback into repair feedback. |
| **D7** repeated verify within a repair budget | Mature implementation: attempts × draws budget knobs, measured and pinned through A/B runs. |
| **D8** typed `unavailable`, never silent success | Same abstraction as Finitive's preflight capability blockers: infeasibility is a typed, up-front signal raised before any model spend. |

## Part 2 — Recommended changes to scope

### R1. Name a findings-shaping layer above normalization (extends D6)

**Evidence.** Finitive's largest measured wins came not from having normalized diagnostics but from *shaping* which findings reach the model: root-first compile focus (fix the first root error, suppress cascades), deduplication of repeated findings, per-pass bounding, and lean repair envelopes. Its doctrine: "a task that does not converge by attempt 2 is almost always a delivery problem, not a model ceiling."

**Recommendation.** Keep D6's normalization host-owned as written. Add a named follow-on concern — findings **selection and ordering per repair pass** (root-cause-first, deduped, bounded) — as an explicit seam rather than leaving it implicit in adapter prompt text. Decide where it lives: the engine (as a projection over the normalized report) or the target adapter's repair brief. Either is defensible; undefined is not, because RFC-91 workers and RFC-18 both consume this seam.

**Scoping pointers.** The normalized report already lands in the `diagnostics` crate's `Diagnostic` / `DiagnosticReport` substrate. A shaping projection would be a pure function over `DiagnosticReport` (ordering, dedupe by fingerprint, cap) — candidate home: `crates/diagnostics` (neutral, no engine deps) with policy inputs from the verify orchestration.

### R2. Canonicalize verify output; add repeat/regression detection primitives (extends D6/D7, feeds RFC-91)

**Evidence.** Finitive canonicalizes test output (strips thread ids and durations, sorts result lines and failure blocks) specifically so repair passes are comparable. That determinism is what makes its budget policy mechanisms work: **stop-on-repeat** (identical failure twice → stop spending draws) and a **compile high-water-mark latch** (regression below best-so-far → rollback rather than continue). These are measured, pinned policies in its decision log, not heuristics.

**Recommendation.** Require the normalized report to be canonical (stable ordering, no volatile fields like durations/thread ids in the comparable portion) and specify two host-computable predicates over consecutive reports for the same candidate: *unchanged failure set* and *regression vs. best pass*. RFC-90 does not need to define the budget policy that consumes them — that is RFC-91 convergence-gate territory — but the report shape must support them from day one. Retrofitting canonicality after adapters and fixtures exist is expensive.

**Scoping pointers.** Diagnostic fingerprints already exist in `crates/diagnostics` (the fingerprint algorithm); the work is (a) guaranteeing canonical report ordering at normalization time and (b) a report-diff kernel (set comparison over fingerprints plus a severity-ordered high-water measure — Finitive orders lexicographically by parse-breaking error count, then total error count; a useful reference shape).

### R3. Deterministic mechanical-repair rung below the model loop (new, within D4's trust model)

**Evidence.** Finitive applies rustc `MachineApplicable` suggestions host-side (engine-applied rustfix) before spending a model draw, with two hard-won constraints from its decision log: fixes must apply in **atomic groups** (a measured incident showed half-applied fix sets made outcomes worse than applying none), and keep/rollback is decided by a severity ordering over the resulting error set.

**Recommendation.** Scope an optional host-owned mechanical repair pass inside the verification workspace: apply toolchain-suggested machine-applicable fixes atomically, re-run the profile, keep the result only if the error measure strictly improves. This is squarely inside RFC-90's trust model — fix content comes from the vetted toolchain, not model output — and it reduces repair budget spent on trivia. Can be a fast-follow rather than part of the first vertical cut; the decision to scope is what matters now.

**Scoping pointers.** Runs entirely within the disposable RFC-87 workspace; the result, if kept, is a new candidate snapshot via the existing `capture` path (`project::workspace`). No new authority surface: the model still sees only a normalized report plus (optionally) a note that mechanical fixes were applied.

### R4. Air-gap the judge: verify-relevant test sets vs. the model-writable envelope (D4 interaction, lands in RFC-91)

**Evidence.** Finitive's hardest invariant is path freezing: the model cannot write `tests/` — "a model that can edit its own oracle has no oracle." Its accepted-task signal is meaningful *because* the judging tests are outside the model's writable set.

**Gap in RFC-90 as written.** The command channel is closed, but the `test`/`ci` profiles execute tests living in the candidate snapshot — tests the model itself may have authored or edited during build. That is coherent as a self-consistency check but is a categorically weaker signal than Finitive's frozen-oracle green, and nothing in the current text distinguishes the two.

**Recommendation.** Do not redesign `test` in RFC-90. Instead: (a) record in RFC-90 that `test`-green means "candidate passes its own tests," and (b) scope, for RFC-91's write-ownership envelope, the ability to mark test paths as **frozen relative to a worker** so a slice's convergence gate can require green against tests the worker could not touch. Target adapters are the natural owners of which paths are gate-defining.

### R5. Emit verify telemetry as distributions from day one (extends acceptance criteria, feeds RFC-91 + RFC-18)

**Evidence.** Finitive's measurement doctrine — per-draw gate timings, draws-to-pass distributions, "read distributions, never one green/red as proof" — is what made its loop-policy decisions falsifiable; its decision log records several plausible mechanisms killed by A/B nulls (for example, adding "plumbing" unit tests that went green without reducing failure recurrence — origin of its rule that a green unit test proves wiring, not outcome — and a lookup-tool experiment where letting the model pull reference material showed no win over host-pushed context).

**Recommendation.** Journal per-pass verify telemetry from the first cut: profile, snapshot id, wall time, finding counts by severity, pass ordinal within the repair loop. Nearly free at the point the report is already normalized; RFC-91's scheduler and RFC-18's economics both need these distributions and cannot reconstruct them later.

**Scoping pointers.** Extend the closed journal event taxonomy (`crates/project/src/journal.rs`) with the verify events RFC-90 introduces anyway; put the distribution-bearing fields on those events rather than inventing a metrics side-channel.

### R6. Design the report to double as RFC-18's reward function (strategic alignment, low cost)

**Evidence.** Finitive is, functionally, a working prototype of RFC-18's proposed loop: local code model + deterministic scorer + repair budget + frontier authoring. RFC-18's proposed `score-crate` gate is RFC-90's verify report under another name.

**Recommendation.** Ensure the normalized report is machine-**rankable**, not just human-readable: stable severity tiers, deterministic counts, canonical ordering (R2 delivers most of this). Then two outputs of the same prompt can be compared mechanically — which is exactly the filter for synthetic training pairs and the reward for reject-sampled DPO. This collapses RFC-18's "build the eval harness first" phase into "RFC-90 shipped it" and de-risks the SLM lever behind RFC-91's per-worker model-selection hook. No new work beyond R2 + R5 discipline; the deliverable is a sentence in RFC-90 acknowledging the constraint so it isn't accidentally broken.

## Part 3 — One conflict with the RFC as written

### D5 cache keying vs. repair-loop latency

**D5 as written:** cache key includes the **snapshot id**; no cache shared across candidate snapshots.

**The problem:** every model repair pass produces a new candidate snapshot. A strict reading discards the warm `target/` on every iteration, making each repair pass a cold build.

**Evidence:** Finitive deliberately preserves `target/` across failed-attempt rollbacks (`git clean -fd` without `-x`) and its structured-diagnostics pass depends on the warm incremental re-run being cheap. Warm incremental compile is load-bearing for repair-loop wall-clock viability — this is the loop RFC-90 exists to serve.

**Recommendation:** rescope the cache key from *snapshot id* to *candidate lineage within one verification workspace* (e.g. workspace id + profile + toolchain identity). The security boundary D5 actually argues for — nothing crosses candidate boundaries, nothing survives workspace discard, no cache becomes an authority outside the workspace — is preserved exactly; only successive repair passes of the *same* candidate stay warm. The rejected-alternatives entry ("caches shared across snapshots") should be reworded to "caches shared across candidates/workspaces" to match the real invariant.

## Part 4 — What deliberately does not transfer

State these so the evidence is not over-read:

- **No sandbox evidence.** Finitive has no OS sandboxing, no egress denial, no CPU/memory bounds — its isolation is git rollback and path freezes on an in-place repo, which is precisely the posture RFC-87/90 reject. **D4 (deny-by-default sandbox, resource limits, execution grants) remains genuinely novel work that this experiment does not de-risk.** Plan its verification (sandbox-denial integration tests per acceptance criterion 8) accordingly.
- **"No agentic tool loop; resampling is the search engine"** is Finitive's coping strategy for a small local model, not an architectural import. Emery's model loop shape is settled elsewhere; RFC-90 only defines the verify seam inside it.
- **Git-as-ledger acceptance** is replaced wholesale by RFC-86 facts + RFC-87 snapshots in Emery. No transfer.
- **Finitive's structural gates** (checklist/keep/markers/panic-safety scans) are target-adapter review territory in Emery (adapter-embedded rules), not RFC-90 profiles. No transfer into the profile table.

## Suggested scoping order

1. **In the RFC-90 first vertical cut:** R2 (canonical report + diff primitives), R5 (telemetry on the journal events), R6 (rankability constraint — one paragraph), and the Part 3 cache-key fix. These are cheap now and expensive later.
2. **Fast-follow within RFC-90:** R1 (findings-shaping seam — decide engine vs. adapter placement), R3 (mechanical-repair rung).
3. **Hand to RFC-91 scoping:** R4 (frozen-test paths in the write-ownership envelope), plus the budget policies (stop-on-repeat, high-water latch) that consume R2's primitives.

## Evidence sources

- **Finitive** is a private experiment; its repository is not available to readers of this document. All findings it contributes are stated inline above — treat this document as the complete record of that evidence. Questions about the underlying runs or decision log go to the experiment's owner.
- Emery: [`rfcs/rfc-90-verify-profiles.md`](rfc-90-verify-profiles.md) (D1–D8), [`rfcs/platform.md`](platform.md) (series fit; RFC-90 → RFC-91 gate), [`rfcs/future/rfc-18-slm.md`](future/rfc-18-slm.md) (`score-crate`, reward shape), `crates/diagnostics/` (report substrate), `crates/project/src/journal.rs` (event taxonomy), `crates/project/src/workspace/` (RFC-87 kernel).
