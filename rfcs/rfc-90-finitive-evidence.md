# RFC-90 scoping input: evidence from the Finitive local-model harness

> Status: Accepted evidence input — **not an RFC** and not the series RFC-90. The in-force series document is [RFC-90: Build Verification](rfc-90-build-verification.md); [RFC-93: Host Verification Profiles](future/rfc-93-host-verification.md) owns its deterministic native-verification follow-on. This note records the external evidence and the disposition of each recommendation. It also informs [RFC-91](rfc-91-concurrent-execution.md) and [RFC-18](future/rfc-18-slm.md) where noted.
>
> Purpose: give an implementing agent or engineer the empirical context from an external local-model harness experiment ("Finitive") and record where each resulting recommendation landed. The experiment's code is not available to readers of this document; every finding it contributes is described inline and this document is the complete record.

## Context

**Series RFC-90 (Build Verification)** lands the engine-owned `build` / `repair` / `verify` / `review` phase machine with **model-assisted** verification. Its agent still chooses, runs, and interprets native commands. D8 hands deterministic native verification to RFC-93.

**RFC-93 (Host Verification Profiles)** defines the deferred host-owned gate: target code requests only a closed profile name (`fmt`, `build`, `clippy`, `test`, `doc`, `vet`, `deny`, `ci`); deployment policy selects vetted commands from the bound target/platform, runs them in a disposable RFC-87 workspace against the RFC-91 candidate snapshot, and returns normalized findings (`source: tool`, `kind: violation`). The model never supplies commands. RFC-90's engine-owned repair budget remains in force.

**Finitive** is a private research harness (Python) for spec → Rust code generation with a small local model (`qwen3-coder:30b` served by Ollama), actively developed along a research path with every design decision pinned by instrumented A/B runs. Its architecture, in enough detail to evaluate the evidence without repo access:

- **No agentic tool loop.** The model produces one-shot file draws from a single assembled prompt; resampling (temperature-ramped attempts × draws, defaults 4 × 3) is the search engine.
- **Host-owned deterministic gates.** Acceptance is never the model's claim: every draw is screened (path bounds), written, then gated by `cargo check`, a frozen oracle test suite (`cargo test` against tests the model cannot write), and structural scans. All green → a git commit is the acceptance record; red → structured feedback into the next draw; draws exhausted → hard rollback (`git reset --hard`).
- **Structured diagnostics.** Compile failures are re-read via rustc `--message-format=json` for error codes, spans, and machine-applicable suggestions; a bounded (~4,000-char) human-readable tail is the fallback channel.
- **Measured decisions.** Every loop-policy mechanism (feedback shaping, budget rules, fix application) was adopted or rejected via A/B runs read as *distributions* of draws-to-pass, recorded in an append-only decision log. Several plausible mechanisms were falsified and removed.

Roughly: a working, instrumented prototype of RFC-93's host verify-and-repair premise, minus Emery's sandbox, fact, snapshot, and adapter boundaries.

**Headline:** Finitive independently converged on RFC-93's core premise — the model never supplies commands; closed host gates judge everything; repair runs within a bounded budget — and demonstrates it working with a small local model. It corroborates RFC-90's engine-owned bounded loop, but it does **not** make RFC-90's current model-assisted command execution deterministic. The evidence below explains the scope changes accepted into RFC-90, RFC-91, and RFC-93.

## Part 1 — Decisions the evidence directly validates

| Host-verification concern | Finitive corroboration | Emery owner |
| --- | --- | --- |
| Closed profiles, host-selected commands | Finitive's gates are a closed, named set the host runs; the model never emits argv, flags, or toolchain choices. Works in practice, even for a 30B local model. | RFC-93 D1–D3 |
| Structured parsers, bounded raw fallback | Rustc `--message-format=json` is the primary diagnostics channel; a bounded ~4,000-character raw tail is the fallback into repair feedback. | RFC-93 D5 |
| Repeated verify within a repair budget | Mature implementation: attempts × draws budget knobs, measured and pinned through A/B runs. | RFC-90 D1 |
| Typed `unavailable`, never silent success | Finitive's preflight capability blockers raise infeasibility before model spend. | RFC-93 D1/D3 |

## Part 2 — Recommended changes to scope

### R1. Name a findings-shaping layer above normalization

**Evidence.** Finitive's largest measured wins came not from having normalized diagnostics but from *shaping* which findings reach the model: root-first compile focus (fix the first root error, suppress cascades), deduplication of repeated findings, per-pass bounding, and lean repair envelopes. Its doctrine: "a task that does not converge by attempt 2 is almost always a delivery problem, not a model ceiling."

**Disposition.** Accepted in two layers. RFC-90 D2 canonicalizes every complete phase report; D4 filters its blocking findings to an initial 16-finding repair cap. The complete report remains gate authority. RFC-93 D5/D6 lets a profile-specific deterministic normalizer suppress known cascades before the neutral projection. Adapter prompt text owns neither layer.

**Implementation pointer.** The neutral projection is a pure function over the `diagnostics` crate's `DiagnosticReport`; orchestration supplies the fixed cap. Tool-specific causal knowledge remains in RFC-93's profile normalizer rather than the dependency-neutral diagnostics crate.

### R2. Canonicalize verify output; add repeat/regression detection primitives

**Evidence.** Finitive canonicalizes test output (strips thread ids and durations, sorts result lines and failure blocks) specifically so repair passes are comparable. That determinism is what makes its budget policy mechanisms work: **stop-on-repeat** (identical failure twice → stop spending draws) and a **compile high-water-mark latch** (regression below best-so-far → rollback rather than continue). These are measured, pinned policies in its decision log, not heuristics.

**Disposition.** Partially accepted in RFC-90 and completed in RFC-93. RFC-90 canonicalizes accepted phase findings and terminal unions, closing its existing byte-stability ambiguity. RFC-93 removes tool-specific volatile data before fingerprinting and defines *unchanged failure set* and *regression vs. best pass* over consecutive candidate revisions in one lineage. The predicates are recorded first; they do not silently replace RFC-90's fixed repair budget.

**Implementation pointer.** Diagnostic fingerprints already exist in `crates/diagnostics`. Canonicalization must precede fingerprinting because evidence payloads enter the fingerprint; stable list ordering alone is insufficient.

### R3. Deterministic mechanical-repair rung below the model loop

**Evidence.** Finitive applies rustc `MachineApplicable` suggestions host-side (engine-applied rustfix) before spending a model draw, with two hard-won constraints from its decision log: fixes must apply in **atomic groups** (a measured incident showed half-applied fix sets made outcomes worse than applying none), and keep/rollback is decided by a severity ordering over the resulting error set.

**Disposition.** Accepted into RFC-93 D7 as a new, explicit write-authority phase, not RFC-90. Trusted tool output and native execution do not exist at RFC-90's boundary. RFC-93 permits one exact-preimage suggestion group owned by one RFC-91 task, captures a tentative snapshot, and keeps it only when the originating profile improves and the complete profile set does not regress. Domain verification has no such writer, and the phase cannot become an unbounded second repair loop.

**Scoping pointer.** The phase runs entirely through RFC-87 workspaces and RFC-91's reviewed grant. It adds bounded host write authority but no lifecycle or merge authority; its accepted result is an ordinary captured and composed candidate patch.

### R4. Air-gap the judge: verify-relevant test sets vs. the model-writable envelope

**Evidence.** Finitive's hardest invariant is path freezing: the model cannot write `tests/` — "a model that can edit its own oracle has no oracle." Its accepted-task signal is meaningful *because* the judging tests are outside the model's writable set.

**Gap.** RFC-90's command channel is **not** closed: its verification agent chooses, runs, and interprets commands. Those commands may execute tests in the candidate snapshot that the model authored or edited. That is coherent as a self-consistency check but categorically weaker than Finitive's frozen-oracle green.

**Disposition.** Accepted with corrected authority. RFC-90 now calls candidate-owned checks self-consistency evidence. RFC-91 adds operator-reviewed protected verification inputs that no worker may change. Target metadata may nominate target/platform defaults during plan authoring, but only the reviewed RFC-88 decomposition and execution epoch authorize protection. RFC-93 D4 records whether the executed host policy bound candidate, protected, or mixed inputs; declaration alone cannot upgrade assurance.

### R5. Emit verify telemetry as distributions from day one

**Evidence.** Finitive's measurement doctrine — per-draw gate timings, draws-to-pass distributions, "read distributions, never one green/red as proof" — is what made its loop-policy decisions falsifiable; its decision log records several plausible mechanisms killed by A/B nulls (for example, adding "plumbing" unit tests that went green without reducing failure recurrence — origin of its rule that a green unit test proves wiring, not outcome — and a lookup-tool experiment where letting the model pull reference material showed no win over host-pushed context).

**Disposition.** Partially accepted in RFC-90 and completed in RFC-93. RFC-90's phase-completed event records the attempt, ordinal, operation, source, report digest, and engine-measured elapsed milliseconds. Current RFC-90 has no closed profile and its one mutable workspace does not naturally produce a snapshot per pass, so those fields are not falsely invented. RFC-93 D9 adds one event per host profile with candidate snapshot, policy, assurance, severity counts, cache disposition, and elapsed time.

**Implementation pointer.** Events contain raw observations; distributions are projections over retained events. Timing and cache data stay outside report fingerprints and lifecycle authority.

### R6. Design the report to double as RFC-18's reward function (strategic alignment, low cost)

**Evidence.** Finitive is, functionally, a working prototype of part of RFC-18's proposed loop: local code model + deterministic scorer + repair budget + frontier authoring. RFC-18's proposed `score-crate` gate overlaps host verification but also grades concerns outside it.

**Disposition.** Rejected as an RFC-90 or RFC-93 invariant. Canonical reports and telemetry are reusable inputs, and RFC-18 may project a scorer from them. They are not a complete reward: RFC-18 also grades traceability, guardrails, file layout, configuration, and migration concerns outside native verification. Coupling lifecycle reports directly to a training reward would invite reward-driven schema constraints and overstate what severity counts measure.

## Part 3 — Cache-lineage correction for RFC-93

### Snapshot-only cache keying vs. repair-loop latency

Series RFC-90 defines no verification cache; its D5 is the one-workspace rule and D8 explicitly defers cache policy. This recommendation therefore belongs only to RFC-93.

**Problem in the deferred design:** every model repair pass produces a new candidate snapshot. Keying only by snapshot would discard warm incremental state on every iteration and make each repair pass a cold build.

**Evidence:** Finitive deliberately preserves `target/` across failed-attempt rollbacks (`git clean -fd` without `-x`) and its structured-diagnostics pass depends on the warm incremental re-run being cheap. Warm incremental compile is load-bearing for repair-loop wall-clock viability — this is the loop RFC-90 exists to serve.

**Disposition.** Accepted in RFC-93 D8 with RFC-91's workspace semantics accounted for. The key is the globally scoped slice verification lineage plus resolved target, platform, profile-policy, toolchain, and sandbox/environment-policy identities. RFC-91 still materializes a fresh workspace for every operation; the host privately mounts or copies the lineage cache. Domain contexts inherit no slice cache, and every warm passing required profile receives a cold confirmation before it can gate success. Nothing crosses verification contexts, enters snapshot capture, or becomes report or lifecycle authority.

## Part 4 — What deliberately does not transfer

State these so the evidence is not over-read:

- **No sandbox evidence.** Finitive has no OS sandboxing, no egress denial, no CPU/memory bounds — its isolation is git rollback and path freezes on an in-place repo, which is precisely the posture RFC-87/90 reject. **RFC-93 D3 (deny-by-default sandbox, resource limits, execution grants) remains genuinely novel work that this experiment does not de-risk.** Plan its verification through RFC-93's sandbox-denial acceptance tests.
- **"No agentic tool loop; resampling is the search engine"** is Finitive's coping strategy for a small local model, not an architectural import. Emery's model loop shape is settled elsewhere; RFC-90 only defines the verify seam inside it.
- **Git-as-ledger acceptance** is replaced wholesale by RFC-86 facts + RFC-87 snapshots in Emery. No transfer.
- **Finitive's structural gates** (checklist/keep/markers/panic-safety scans) are target-adapter review territory in Emery (adapter-embedded rules), not RFC-90 profiles. No transfer into the profile table.

## Disposition summary

1. **RFC-90:** neutral findings shaping, canonical phase/terminal ordering, honest candidate-check assurance, and operation elapsed time.
2. **RFC-91:** operator-reviewed protected verification inputs and write exclusion.
3. **RFC-93:** closed host profiles, sandboxed execution, canonical tool normalization and comparison, protected-oracle attestation, bounded mechanical repair, lineage caches, and per-profile telemetry.
4. **RFC-18:** optional scorer projection over reports; no reward authority in RFC-90 or RFC-93.

## Evidence sources

- **Finitive** is a private experiment; its repository is not available to readers of this document. All findings it contributes are stated inline above — treat this document as the complete record of that evidence. Questions about the underlying runs or decision log go to the experiment's owner.
- Emery: [`rfcs/rfc-90-build-verification.md`](rfc-90-build-verification.md) (engine-owned model-assisted phase machine), [`rfcs/rfc-91-concurrent-execution.md`](rfc-91-concurrent-execution.md) (candidate and ownership model), [`rfcs/future/rfc-93-host-verification.md`](future/rfc-93-host-verification.md) (host-verification authority), [`rfcs/future/rfc-18-slm.md`](future/rfc-18-slm.md) (`score-crate`, reward projection), `crates/diagnostics/` (report substrate), `crates/project/src/journal.rs` (event taxonomy), and `crates/project/src/workspace/` (RFC-87 kernel).
