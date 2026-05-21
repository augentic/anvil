# RFC-25 review — is the 2-step multi-source synthesis the right shape?

**Status:** review note, not a normative RFC. Synthesises three independent best-of-N analyses of [`rfc-25-workflow.md`](./rfc-25-workflow.md) and [`rfc-25-plan.md`](./rfc-25-plan.md) by `claude-opus-4-7-thinking-high`, `gpt-5.5-high`, and `composer-2.5-fast`, each run in an isolated worktree against `HEAD = 4e0b69f`.

**Question under review:** Is the 2-step multi-source analysis (`enumerate` → operator-fused `propose` → slice-time `extract` → core synthesis) introduced by RFC-25 the most effective way to produce specs for code generation? Or is there a better way?

## Verdict

**Keep the two-step.** Unanimous across all three runs. A pure single-step "read everything and synthesise" approach is strictly worse for Specify's actual charter (multi-source migration with auditable code generation): it forfeits the operator gate, reviewable slice boundaries, and on-disk provenance — the three features that most reduce catastrophic code-gen mistakes on legacy estates.

RFC-25 is **not** the maximum-accuracy path for code generation in the absolute. Executable behaviour (captured runtime fixtures) beats prose specs on fidelity, and graph-of-claims reconciliation would beat YAML evidence blocks on auditability at scale. But neither of those is a replacement for the two-step; they are *additions* on top of it. The leverage left is in sharpening four things, not redesigning the spine.

## What the two-step is actually doing (the load-bearing decisions)

Most of RFC-25's power comes from one deeper architectural choice that is easy to miss: **the slice is the join column across heterogeneous sources, and that join is made visible on disk before any LLM writes a `spec.md`.** The two-step is downstream of that.

Specifically:

1. **`enumerate` answers "what are the slice-sized units of work?"** It is cheap (one walk per source), produces headlines + stable ids, and writes [`discovery.md`](./rfc-25-workflow.md) `## Candidate inventory` blocks.
2. **`propose` fuses candidates across sources** into `slices[].sources[]` rows of `{ key, candidate }` tuples — making cross-source alignment a structured lookup rather than a free-form LLM judgement at synthesis time.
3. **Gate 1 (`plan.lifecycle: pending → reviewed`) is an on-disk operator stamp** that any CI / hosted runner / restarted agent can key off. `/spec:execute` refuses unless `reviewed`. The two-step exists in part to give Gate 1 something concrete to gate on — the `slices[]` array is the artifact under review.
4. **`extract` answers "what claims support this one slice?"** with `$SOURCE_DIR` read-only, no `$PROJECT_DIR` grant, no host env, no network. Bounded scope, schema-validated `Evidence`, per-axis cache.
5. **Synthesis fuses multi-source evidence into `spec.md` / `design.md` / `tasks.md`** with per-requirement `Sources:` lines, `Status: agreed | unknown | conflict | divergence`, and inline `[conflict]` / `[divergence]` / `[unknown]` tags. The closed authority enum (`intent > documentation > behaviour`) controls resolution; losers survive as commentary.
6. **`shape` is consumed, not authored.** Target adapters declare idiom but do not write `spec.md`. `Slice.target` is what `build` consumes; target-specific manifests (Vectis `composition.yaml`, contract files) are build outputs, not Specify artifacts.

The combined effect for code generation is that the chain from generated Rust → `tasks.md` → `design.md` → `spec.md` → `Evidence` → source claim is mechanically auditable, not "the model said so."

## Strengths (consensus)

- **Plan-time vs slice-time separation is load-bearing.** Decomposing legacy code by externally visible behaviour (routes, topics, jobs) is a different problem from spec-writing, and the two should not share one LLM pass. Without the split, extraction either runs on the whole monolith (context blow-up, wrong granularity) or fuses "what are the slices?" and "what does this slice do?" in one opaque inference.
- **The operator gate is observable, not procedural.** `plan.yaml.lifecycle` on disk, CLI as single writer of `reviewed`, `/spec:execute` refuses-unless-reviewed. Survives agent restarts and works for both humans and CI.
- **Slice-shaping is decoupled from evidence-gathering cost.** `enumerate` is cheap, `extract` is expensive. Re-planning at Gate 1 doesn't cost full re-extraction.
- **Provenance survives synthesis.** Every requirement carries `Sources:` and `Status:`; the parser cross-resolves provenance keys against plan-level bindings. The schema enforces this — there is no "untraced requirement" path.
- **Tag-and-proceed beats park-and-prompt.** `[conflict]` / `[divergence]` / `[unknown]` are written inline; the slice still transitions to `refined`. Hand-edit + move on. Parking on every ambiguity would deadlock the workflow at scale.
- **The `extract` sandbox is strong.** `$SOURCE_DIR` read-only, no project-root grant, no host env. Makes adapter behaviour reproducible and prevents leakage of project lifecycle state into source-side processing.
- **Target independence.** Source adapters never know about Omnia/Vectis/contracts. Swapping targets does not require re-running the source pipeline.
- **Per-axis cache and CLI writer-ownership are clean spine.** `.specify/.cache/{sources,targets}/<adapter>/` is read-only for adapters; lifecycle transitions and plan/discovery writes are CLI-only. Single source of truth for deterministic operations.

## Weaknesses and failure modes (consensus)

1. **Cross-source candidate alignment is structurally fragile.** `candidate.id` is doing two incompatible jobs: stable handle for re-enumeration (conservative, never changes) and join key for cross-source fusion (liberal, accepts aliases). `propose` fuses on three-line summary strings before any `Evidence` exists. The worked example in the RFC (`password-reset` vs `account-pwd-reset`) tags `divergence: likely`, which is the right signal but does not fix the underlying tension. Recovery via `specify plan amend --add-source` requires `pending` lifecycle; once a slice is `in-progress` operators must `slice transition dropped` and re-add.
2. **The authority enum is too coarse.** Three closed levels (`intent > documentation > behaviour`), declared per *adapter*, not per claim-kind. For migrations this means "docs always win over code," which is *wrong* at least half the time (legacy code is often the production truth). Per-claim authority and per-slice overrides are explicitly deferred to a future RFC; they will be needed sooner than that.
3. **N=1 pays double cost for no benefit.** Pure intent runs `intent.enumerate` (which echoes the input), `propose` (one slice), Gate 1, `intent.extract` (echoes again), then synthesis. Three CLI calls and a journal event for "fix a typo." The architectural stance — "N=1 is degenerate, not special" — is defensible but produces real ergonomic friction that will push operators toward muscle-memorising `specify plan transition <name> reviewed`.
4. **Plan-time fusion is underpowered.** RFC-25 admits authority does not apply at `propose` because there is no Evidence yet. The riskiest merge decision (do these two candidates from different sources describe the same slice?) happens with the least information.
5. **Synthesis quality has a ceiling set by claim shape, not source coverage.** Fusion is supposed to key on `claim-id`, but `claim-id` is only required on `requirement` / `criterion` kinds and is adapter-derived independently across sources. Most cross-source claim alignment ends up happening on LLM judgement during synthesis — the work `enumerate` was meant to do once at plan time.
6. **`[conflict]` and `[unknown]` do not stop `build`.** They are review signals, not gates. A conflict-tagged spec can still become generated code unless the operator notices. There is no `refined-but-blocked-build` state.
7. **`/spec:refine` is not idempotent against operator hand-edits.** The RFC is honest: "re-running it discards manual reconciliation." Operators who edit `spec.md` to resolve a `[divergence]` must never re-refine, or amend the plan and re-refine cleanly. There is no merge-edits-back-into-evidence story.
8. **Writer-ownership tension on `divergence: likely`.** [`AGENTS.md`](../AGENTS.md) and the RFC say `plan.yaml` is CLI-owned, but `plugins/spec/skills/plan/SKILL.md` directly appends `divergence: likely` because the CLI only accepts `accepted | rejected`. This is exactly the kind of exception that erodes the deterministic spine. The CLI should own `likely` too.
9. **Determinism vs cache invalidation are in tension.** "Same source, same brief, same tool version" needs an explicit fingerprint model (source path + adapter version + brief version + tool version + candidate id) if re-runs are to be trusted across model versions. "Deterministic" today means "byte-stable goldens against fixtures," not "byte-stable production runs."
10. **Extraction failure is unforgiving and the journal is event-thin.** Any `extract` failure keeps the slice in `refining`; there is no partial-synthesis fallback. The journal taxonomy fires on transitions and divergences but not on `extract.started`, per-claim provenance traces, or synthesis token cost — light for code-gen audit.

## Alternative architectures considered

### A. Pure single-step extract (no `enumerate`)

Skip `enumerate` and `propose` entirely. `/spec:plan` becomes operator intent + source bindings; synthesis decides slice boundaries holistically.

- **Pros:** much less ceremony; trivially good at N=1; aligns with how a human reviewer approaches a small migration; works tolerably on greenfield.
- **Cons:** slice boundaries are not reviewable before evidence is gathered; re-planning costs full re-extraction; cross-source join becomes a runtime LLM decision with no on-disk trace; at 10k+ LOC legacy the model context is the limit and you cannot decompose; Gate 1 stops being meaningful because it has no concrete `slices[]` artifact to gate on.
- **Verdict:** strictly worse for Specify's migration charter. Acceptable only for greenfield-only tools — which Specify is not.

### B. Iterative refine-loop with critique (`extract → synth → critique → re-extract`)

Replace the linear pipeline with a feedback loop: synthesise a draft, run a critic skill, feed disagreements back into a scoped second `extract` pass.

- **Pros:** likely improves quality on noisy sources; matches how humans actually write specs; can auto-resolve `[unknown]` tags by re-asking.
- **Cons:** loops are expensive and non-deterministic; budget control is hard; review goldens become impossible because termination drifts; Gate 1 weakens because the work doing the slicing happens *after* the gate; needs idempotent synthesis merge semantics that RFC-25 explicitly lacks.
- **Verdict:** good as an *optional* breakout flag (`/spec:refine --critique`) on slices the operator marks as high-risk. Bad as the default architecture.

### C. Test/example-first specs (behaviour-replay)

Specs derived from executable examples or recorded runtime behaviour rather than written summaries. The RT plugin's wiretap + replay pattern is already a partial instance, currently living in a sibling plugin rather than under `sources/`.

- **Pros:** ground truth is observable runtime behaviour, not docs the team forgot to update. `[divergence]` between docs and code resolves trivially in favour of "what actually happened in production." Generated code can be tested end-to-end against captured fixtures at `build` time, and the fixtures double as the regression suite. **The single strongest answer to code-gen accuracy on migration work.**
- **Cons:** only works for systems you can run; useless for greenfield; the fixture shape does not naturally fit `requirement` / `criterion` claim kinds; "tests as spec" is a known trap when production behaviour is itself wrong.
- **Verdict:** under-leveraged. RT should be promoted to a first-class source adapter, not kept as a sibling plugin. See recommendation #1.

### D. Graph-of-claims with explicit reconciliation

Replace flat `evidence/<source-key>.yaml` files with claims-as-nodes, sources-as-edges, authority-as-weights. Reconciliation runs as an explicit step between `extract` and synthesis.

- **Pros:** makes fusion inspectable ("why did REQ-007 resolve to the docs value, not the code value?"); gives per-claim authority overrides a natural home; supports cross-source claim-id alignment as a first-class operation.
- **Cons:** real engineering cost (new schema, validator, representation); operator UX gets harder if you expose the graph; markdown-and-YAML is "boring" in the good sense and easy to hand-edit; graph databases for LLM-emitted data are a notorious quagmire.
- **Verdict:** right *instinct*, wrong execution path. The pragmatic shape is a thin reconciliation *index* (see recommendation #3), not a graph database.

### E. Operator-as-source (operator writes spec; adapters validate)

Flip polarity. The operator authors `spec.md` by hand; source adapters run as validators that produce findings.

- **Pros:** the operator owns truth; eliminates "the model wrote nonsense" failure modes; pushes LLM cost from generation (unreliable) to validation (much better).
- **Cons:** does not scale to 10k+ LOC migrations where the spec is exactly the artifact you do not have yet; throws away most of Specify's value for the legacy-monolith use case it is built for.
- **Verdict:** wrong as the primary mode, but worth surfacing as a `/spec:plan --no-synth` option for operators who want adapters to validate hand-authored specs. Low-cost addition.

### F. DSL-first specs

Replace free-form markdown `spec.md` with a constrained DSL.

- **Pros:** machine-verifiable; deterministic lowering to code generation; enables refactoring and formal property checks.
- **Cons:** higher authoring friction; loses the markdown-hand-edit recovery path RFC-25 leans on heavily; the existing `ID:` / `Sources:` / `Status:` block grammar is already a degenerate DSL embedded in markdown, which is probably the correct compromise position.
- **Verdict:** rejected as the human authoring surface. Extending the existing block parser to enforce more invariants is the right direction. A future `spec lower` step could target a structured IR for codegen without changing the operator-facing artifact.

## Recommendation — keep the spine, sharpen four things

The two-step architecture is the right backbone. The work left is sharpening it, not replacing it.

### 1. Promote runtime/behaviour evidence to a first-class source adapter

This is the single biggest code-gen-accuracy lever available. The RT plugin's wiretap + replay pattern is roughly 70% of the implementation work; the missing piece is integrating it into the source-adapter contract.

- New adapter under `sources/code-runtime/` (or similar): `enumerate` walks captured fixtures and emits one candidate per observed endpoint / message handler / scheduled job; `extract` emits `Evidence` with a new `kind: example` claim shape anchored to fixture file + I/O snapshot.
- Default authority is `behaviour` (highest factual weight for production runtime).
- `Sources:` lines on `spec.md` requirements can cite the behaviour-replay source key alongside docs and intent.
- `build` runs generated code against captured fixtures during the slice loop; `merge` refuses on fixture-test failure.

This closes the gap where prose specs under-specify edge cases and turns the existing RT pattern from a parallel plugin into part of the spine.

### 2. Widen authority beyond the closed 3-class enum

Move `authority` from per-adapter-class to per-claim-kind, with per-slice overrides.

- Default behaviour stays: `intent > documentation > behaviour` per kind.
- Per-slice override surface in `plan.yaml`: `slices[].authority-override.<claim-kind>: <source-key>`.
- Per-claim-kind precedence lets `requirement` claims from documentation outrank `decision` claims from intent for the same slice — which is often the right move for compatibility-driven migrations.

RFC-25 §Non-goals defers this; bring it into v1. The "docs always beat code" default is wrong more often than it is right on legacy work.

### 3. Add a thin reconciliation index — not a graph database

Synthesis writes `slices/<slice>/fusion.yaml` listing every `REQ-*` id in `spec.md` and the contributing `(source-key, claim-id)` pairs plus the authority outcome.

This is the smallest possible answer to the "graph of claims" direction. It keeps the markdown-and-YAML story intact, but gives the operator a single inspectable artifact to consult when synthesis surprises them — without re-reading every `evidence/*.yaml`.

### 4. Smaller fixes on the way

- **Move `divergence: likely` ownership fully into the CLI writer path.** `specify plan create` / `specify plan amend` should accept `divergence: likely` directly so the `plan` skill stops writing `plan.yaml` to add the marker. Restores the "CLI is the single writer of `plan.yaml`" invariant.
- **Sharpen `candidate.id` as a join key with explicit aliases.** Add an optional `aliases: []` field on candidate blocks, adapter-authored or `propose`-fused. `slices[].sources[]` resolves a binding's `candidate` against `id` or any `alias`. Makes the cross-source fusion graph inspectable and amendable.
- **Narrow N=1 escape valve.** `specify plan create --auto-review`, valid only when the plan has exactly one slice with `sources: [intent]`, stamps `reviewed` atomically with `create`. Preserves the "operator stamps `reviewed`" invariant because `--auto-review` *is* the operator stamp at create time. Removes two CLI calls and a context switch from the most common ceremony complaint.
- **Define cache fingerprints explicitly.** `(source path, adapter version, brief version, tool version, candidate id)` as the cache key, journaled on every `extract`. Makes re-runs explainable and gives CI a basis for trusting cached evidence across model versions.

### Do not ship

- **Single-step holistic synthesis.** Forfeits the operator gate and on-disk provenance.
- **Iterative critique loop as the default.** Acceptable as an opt-in flag; not as the spine.
- **Full DSL spec language.** Premature schema design; loses the hand-edit recovery path.
- **Graph-of-claims as a new persistence layer.** Cost > benefit at current scale; the reconciliation index above gets most of the value at a fraction of the cost.
- **Collapsing `enumerate` and `extract` because model context is growing.** Context size does not fix slice-boundary errors; it only hides them until `build` fails.

## One-sentence summary

RFC-25's two-step is structurally correct because it puts the slice boundary on disk before any LLM writes a `spec.md`, and that is the right operator-trust seam for code-generation accuracy on multi-source migration; the work left is sharpening the join key, breaking the closed authority enum, lifting runtime behaviour into a first-class source, and adding a thin reconciliation index — not redesigning the spine.

## Provenance

Synthesis of three independent best-of-N analyses run in isolated worktrees:

- `claude-opus-4-7-thinking-high` — deepest analysis; identified the `id`-as-join-key vs stable-handle structural tension; proposed `aliases: []`, `fusion.yaml`, `code-runtime` source adapter, and `--auto-review`.
- `gpt-5.5-high` — caught the concrete `divergence: likely` writer-ownership inconsistency; pushed hardest on cache fingerprints and determinism.
- `composer-2.5-fast` — cleanest narrative framing; load-bearing argument that context size does not fix slice-boundary errors.

All three converged on: keep the two-step, sharpen authority + reconciliation + N=1 ergonomics, promote behavioural evidence to a first-class source, reject single-step / DSL / graph-database / critique-loop-as-default. Read against `HEAD = 4e0b69f`.
