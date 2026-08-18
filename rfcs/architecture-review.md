# Architecture Review: Emery as implemented

> Status: Review, not an RFC. Findings and corrective recommendations against the standing architecture ([architecture.md](architecture.md)), the services programme ([platform.md](platform.md)), and the code in `augentic/emery` plus `augentic/emery-adapters`.
>
> Scope: the whole product — engine crates, WIT, launcher, native and Wasm providers, first-party adapters, eval and wasm examples, operator skills, and the docs that claim to describe them. Not a two-week delta, not a patch list.
>
> Finding ids are stable (`P1…`, `S1…`, `D1…`, `A1…`, `T1…`, `C1…`, `R1…`). Severity: **blocker** (the services promise cannot be kept until this is resolved), **major** (resolve before staffing the next programme RFC), **minor** (fix opportunistically in the same cuts).
>
> A second whole-codebase pass is folded into this file (findings `P10+`, `S12+`, `D12+`, `A8+`, `T6`, `C3+`). The corrective programme is executed through [remediation-plan.md](remediation-plan.md), gated by [product.md](product.md) and the ADRs in [decisions/](decisions/) — live scope is the specification generator ([ADR-0008](decisions/0008-spec-generator-programme.md)). Cuts 1–5 as written below are evidence, not the execution sequence.

## Verdict

**The implementation has not failed locally. It has failed to stay one product.**

Individual RFCs (86, 87, 88, 90, 91, 86a, 95, 96, 104) are carefully typed and locally coherent, with strong fail-closed loads on several core artifacts. The regression is compositional. Emery now presents a simple services promise — recover a bounded estate, deliver one reviewed wave, leave living baselines — while internally coordinating three coupled products:

1. An estate-definition and architecture-modelling product (`crates/system`, RFC-104).
2. A change-delivery engine (`crates/change` + `crates/slice` over a 25k-line `crates/project`).
3. A component-deployment platform (launcher, dual native/Wasm providers, resolver matrix, MCP/HTTP prose hosting).

Those three do not compose into the promised loop. Definition does not freeze the trees delivery later extracts. Delivery does not write accepted results back into the architecture. Reviewed delivery mappings are discarded before decomposition. The production Wasm seam is the one path no automated test runs, and its advertised adapter isolation is descriptive rather than enforced. Status and dispatch share a partial reducer but still derive schedulability separately. The journal is documented as observability and used as the database without a durable change-generation identity.

This is the right moment for ground-up cuts. The product is not in production use. Further programme RFCs (92, 94, 97, 98) will multiply the same substrates. Staff **subtraction and lifecycle closure** before adding another plane.

## The yardstick this review holds — and the one it does not

This review audits the implementation against [platform.md](platform.md). It does not audit platform.md against the product. The product definition is simpler than the programme spine: mine legacy code, documentation, API contracts, operator intent, screenshots, and designs into structured specifications that humans **and** agents can review quickly; optionally deliver the specified application (Omnia service or Vectis app) slice by slice — fast, reliable, resumable on failure, parallelisable, and correctable by conversation when a slice sticks. Held to that definition, much of the audited complexity is not implementation drift; it is faithful implementation of over-scoped requirements.

Scale is the plainest evidence. The engine is ~101k lines of Rust plus ~27k lines of embedded adapter prose. Omnia — the entire runtime platform beneath it, including twelve WASI host-capability crates, the guest SDK, macros, and a conformance suite — is ~30k lines. A workflow CLI 3.4× the size of its runtime is a scope symptom before it is an engineering one. Omnia's coherence came from being built to a settled, bounded architecture; repair alone does not produce that property.

Five product decisions therefore precede Cuts 1–5, and a target-architecture document recording them — written forward from the operator journey, naming the kept kernels and the deleted planes — must lead the corrective programme. Findings-driven repair without that destination converges on the same system, hardened.

1. **State model** — one transactional state store per change home vs a hardened event store (S1; the amended recommendation there names the dissolved findings).
2. **Deployment** — native-only now vs dual native/Wasm (D1; decide, do not defer — it is the highest-leverage subtraction available). *Decided:* [ADR-0002](decisions/0002-deployment.md) (accepted 2026-08-17) resolves this **Wasm-primary** — the component seam is foundational (dynamic adapter admission; desktop + web-service duality, which a prior non-Wasm generation failed on); the subtraction taken is the native provider and the resolution matrix, not the platform.
3. **Lifecycles** — one spec-mining loop over ordinary sources vs a mandatory definition product upstream of delivery (P1).
4. **Conflict disposition** — an operator gate for `[conflict]` vs auto-defer (P3).
5. **Change-home shape** — detached-only, decided now rather than discovered at Cut 5 (D2).

Cut 0 is containment and proceeds regardless of these decisions. Cuts 1–5 are re-derived from the recorded decisions, not executed as written.

## What this review is not

- Not a claim that RFC-86a's auto-deferral is an accidental bug. It is a recorded policy. The finding is that the policy inverts the spec-first story the rest of the product still advertises.
- Not a claim that private workspaces, computed-never-stored status, or the engine-owned build machine should be undone. Those are load-bearing and mostly sound. (The *substrate* behind computed status — the fact union — is questioned in S1; the principle survives either answer there.)
- Not a request to staff parked RFC-99…102 or evidence-gated RFC-106. The problem is too many active planes, not too few future ones.

## Scale of what is there

Approximate, from the trees as read:

| Surface | Size |
| --- | --- |
| Engine Rust | ~99k lines across 17 crates; `crates/project` ~33k, `change` ~16k, `slice` ~15k, `system` ~7k |
| Closed `EventKind` | 46 serde-renamed variants in one enum (`crates/project/src/journal/event.rs`) |
| `Error::Diag` construction sites | 100+ files; the operational taxonomy is stringly typed |
| Adapter contract copies | WIT + SDK seam + engine seam + guest conversions + native conversions |
| First-party adapter Rust | vectis ~10k, omnia ~700, contracts ~600, five sources ~700 combined |
| Embedded adapter prose | ~27k lines |
| Operator skills | 9 ultrathin wrappers |
| CLI routes | 29 routed commands |
| `$EMERY_HOME` trees | `store/`, `cache/`, `snapshots/`, `workspaces/`, `staging/`, `publication/` |

`crates/change/src/orchestrate/decompose.rs` is 1,150+ lines. `crates/guest/src/provider.rs` is ~900. `crates/project/src/journal/event.rs` is ~880. These are coupling hubs, not just large files.

## What is actually good

Keep these. Several of the recommended cuts exist to *protect* them.

- **Typed WIT adapter seam, no resources, identity as data.** `wit/emery.wit` is a real contract. Source adapters stay value-in. The engine-owned RFC-90 phase machine (`build → verify ⇄ repair → review ⇄ repair`) is implemented as specified in `crates/slice/src/orchestrate/target/machine.rs`, and first-party targets honor one pass per dispatch on the *build* path.
- **Content-addressed snapshot identity, store-neutral.** Accepted CIDs are not Git SHAs. Both providers use the same canonical manifest algorithm. That identity is load-bearing for detached delivery and publication export. The kernel is sounder than its current global-GC and adapter-confinement deployment.
- **Refinement as a separate stage.** RFC-91's "execute never refines" is held in the execute loop. The stage boundary should survive the corrective cuts, but the current completion projection can still let a synthesis event outrank a missing manifest (S9).
- **Ultrathin skills.** The nine `/emery:*` wrappers are honest invoke-and-relay. The CLI owns lifecycle. That split should survive every cut below.
- **Fail-closed artifact loads where implemented.** Typed serde parsing, closed handoff DTOs, and verify-on-read for component and snapshot objects are sound disciplines. They are not yet universal: build records, domain rounds, debt projections, project configuration, contract documents, and handoff derivation have fail-open or under-validated paths.
- **Honest docs about the untested Wasm seam.** Both repos state that component-boundary execution is operator-invoked. The gap is known; it has not been closed.

## The services promise vs the implemented journey

[platform.md](platform.md) states one promise:

> Establish or recover what a critical system must do, deliver it in bounded, reviewable waves, and preserve the result as the basis for future change.

The implemented operator journey is:

```text
hand-author scope.yaml + coverage.yaml
→ emery system survey
→ inspect generated as-is
→ emery system plan
→ inspect migration, diagrams, handoff
→ emery system review
→ emery plan author --from --wave
→ inspect topology (and possibly decompose/correct)
→ emery plan refine
→ inspect specifications
→ emery plan execute          # auto-defers remaining gaps
→ operator Git commit / push / PR / merge
→ emery plan archive
```

There is no unified status across definition and delivery. `system status` is CLI-only; there is no `/emery:system-status`. [docs/explanation/layered-stack.md](../docs/explanation/layered-stack.md) still describes three layers (config, slice, change) and does not place the definition loop. After archive, nothing writes accepted CIDs or delivered-wave state back into the definition home. The architecture baseline is upstream of delivery, not living.

N=1 intent-only work still walks most of this machinery: journal union, epoch coverage, gap gate, waves, publication members, pool scheduling. Degenerate is not simple.

---

## P — Product and operator journey

### P1 — RFC-104 is a second product, not an upstream stage — **blocker**

`crates/system` is a parallel workflow engine: its own `Layout` (explicitly "no `.emery/` tree and no `project.yaml`", `crates/system/src/layout.rs`), authored schemas, survey, extract, correlation judgment, plan judgment, review fact, status projection, content-addressed handoffs, architecture renderers, migration waves.

It duplicates substantial source-axis orchestration policy. `crates/system/src/orchestrate.rs` and the delivery survey/refinement path independently pool-fan adapter work, validate leads, persist source results, and fold timeout/cancel. Extraction itself belongs to the slice path rather than `change::orchestrate::survey`; the duplication is therefore not literally two identical survey-plus-extract functions. The policy and lifecycle duplication remains.

`HandoffWave` (`crates/system/src/handoff.rs`) carries sixteen parallel `Vec<Ref>` categories (preconditions, state movements, coexistence, cutover, rollback, operational readiness, acceptance, verification, conservation, gaps, assumptions, decisions, …). `plan author` consumes targets, evidence scopes, and selected digests; it drops delivery mappings (P5). The rest is a consulting-report table of contents with no delivery consumer.

The crate graph already shows the strain: `current_definition` lives in `change` so `project` does not depend on `system`.

**Recommendation.** Treat RFC-104 as an *optional estate-recovery compiler*, not a mandatory second lifecycle. Extract one survey/extract kernel. Collapse `HandoffWave` to what delivery reads plus one opaque section map until a consumer exists. A one-wave intent-only engagement must not require hand-authored `scope.yaml` / `coverage.yaml`.

The stronger form — preferred under the product definition above — is **one lifecycle**: legacy code, documentation, contracts, captures, and designs are ordinary sources feeding one spec-mining loop, "archaeology" is running that loop with code and documentation sources, and the architecture model becomes an optional projection of the same evidence corpus rather than a second product with its own layout, events, and status. Hand-authoring YAML before anything runs fails the "extremely simple for an operator" test outright.

### P2 — A reviewed wave does not freeze delivery inputs, and delivery does not close the architecture — **blocker**

`plan author` re-resolves locators and mints fresh delivery CIDs from `evidence_scopes` (`crates/change/src/orchestrate/author.rs`, `bind_sources`). A handoff `observed-cid` is imported provenance. Delivery extract can run against a different tree than the one that was reviewed.

After execute and archive, `change` / `slice` do not write accepted CIDs, delivered-wave state, or a new as-is into the definition home. `migration.yaml` does not advance. The definition loop ends at `system.wave.reviewed`.

**Recommendation.** Either bind delivery to the reviewed source CIDs (drift is a second review event), or drop the claim that `system review` authorizes a wave's evidence. After archive, record a delivered-wave fact and reproject the definition baseline from accepted CIDs. "Living behavioural and architectural baselines" is otherwise a slogan.

### P3 — Auto-deferral at the build gate inverts spec-first — **major** (policy, not a bug)

RFC-86a is explicit: every open `[unknown]` / `[conflict]` is minting as `gap.deferred` at the build gate, then build proceeds. The implementation matches:

```69:77:crates/change/src/orchestrate/gap_gate.rs
    let facts = deferrals(&open, now, epoch);
    if !facts.is_empty() {
        journal::append_batch(layout, &facts)?;
        tracing::info!(
            "dispositioned {} open gap row(s) on `{slice}` at the build gate",
            facts.len()
        );
    }
```

The synthesized reason is `"deferred at the build gate under epoch {epoch}"` — no requirement heading, no operator text. `[conflict]` (the strongest signal the spec is wrong) receives the same silent treatment as a missing detail.

The *conservation* half is good: deferred rows leave `BuildRequest` scope and become debt rather than invented behaviour. The *authorization* half is not: `Ready` is advisory; execute manufactures readiness by deferral; nothing in the privileged path requires the operator to have looked at `plan gaps`. Docs and AGENTS.md still talk as if specification review is a gate. The code implements "keep going."

The original RFC-86a design had `emery plan defer` and a `strict | defer` policy. Those were deleted after landing so unattended eval could finish. That is a product choice that optimized the lab loop at the expense of the services promise.

**Recommendation.** Restore an explicit disposition act for `[conflict]`, or stop calling unresolved specifications a readiness gate. Unknowns may auto-defer; conflicts should not. Carry the requirement heading in the fact reason either way. If unattended eval needs a bypass, make it a lab flag, not the only production policy.

### P4 — The operator surface obscures the advertised journey — **major** (upgraded: the surface *is* the product)

Nine skills, twenty-nine routes, two lifecycle namespaces, CLI-only recovery verbs (`plan drop`, `plan amend --proposal`, journal show, archive prune, source survey/extract debug). Transport `long_about` strings duplicate docs and AGENTS.md. Target namespace help still says "guidance + build + merge" (`crates/transport/src/command/routes.rs`) after RFC-90 added verify / repair / review. Sibling `emery-adapters/AGENTS.md` has the same three-operation target description.

AGENTS.md has become glossary, architecture, RFC index, negative compatibility ledger, module map, and operator manual in one file. Repeated "there is no…" clauses are the symptom: the conceptual model is no longer inferable from the product surface.

**Recommendation.** Keep the CLI as the intentional lifecycle owner, but advertise one journey (`recover → deliver → finalize/status`). Put debug, resolver, journal, and GC verbs behind an advanced namespace. Generate CLI reference from the router. Cut AGENTS.md back to a spine that reaches a standard in three hops.

The target-architecture document should state the concept budget explicitly: on the order of four operator verbs (`spec`, `build`, `status`, `fix`) and a countable set of nouns an operator must know. Twenty-nine routes and two lifecycle namespaces are not a documentation problem to be tidied; against "extremely simple for an operator" they are a product defect this review originally under-rated as minor.

### P5 — Reviewed delivery mappings are discarded before decomposition — **blocker**

`HandoffWave.delivery_mappings` is projected from `migration.yaml`, content-addressed into the handoff, and covered by `system.wave.reviewed` (`crates/system/src/handoff.rs`). `plan author` imports the target catalog and `evidence_scopes`, but never reads those mappings (`crates/change/src/orchestrate/author.rs`). The imported leads enter decomposition without their reviewed target assignment.

The decomposition judgment can therefore route a reviewed `(source, lead)` to any target in the handoff. Existing integration coverage constructs a reviewed `intent → app` mapping and accepts an answer that routes the same lead to `other` (`crates/change/tests/plan_author_decompose.rs`). Product mutation can occur in a repository the architecture review did not authorize.

**Recommendation.** Persist normalized delivery mappings as immutable decomposition constraints. Every imported scope must have the required mapping cardinality. Split, leaf, reconciliation, force, resume, and topology-amend paths must reject any answer that changes the reviewed assignment. Carry the mapping digest into `discovery.yaml` and `decomposition.yaml`.

### P6 — A fresh review may knowingly cover stale Evidence — **blocker**

System survey intentionally preserves the prior successful Evidence corpus when a source's current survey or extraction fails, adding `survey-error` to `coverage.yaml` (`crates/system/src/orchestrate.rs`). `read_corpus`, handoff projection, and `system review` do not exclude that source or require a stale-Evidence disposition. The new `coverage.yaml` digest makes the handoff current even though the claims come from an older observation.

Retention is a defensible recovery policy; silently treating retained bytes as current authority is not. For a critical-system review, `survey-error` must change the authority of the retained corpus, not merely annotate it.

**Recommendation.** Choose one explicit policy per failed source: exclude it, block review, or record a digest-bound operator decision accepting Evidence from a named prior CID. Carry that decision and observation identity into the handoff and delivery bundle.

### P7 — Force authoring can bind a new locator to an old target CID — **blocker**

During target binding, `plan author --force` looks up the old target row by target id and offers its CID to `fetch_locator` as the recorded snapshot before establishing that the old and new locators are identical (`crates/change/src/orchestrate/author.rs`). Source reuse is locator-keyed; target reuse is id-keyed. A new reviewed handoff can therefore name a changed locator under the same target id while delivery reuses the old bytes.

This is more than undocumented stickiness: it breaks the claim that force authoring rebinds the reviewed handoff.

**Recommendation.** Cache ingestion strictly by canonical locator plus immutable revision and credential policy. Never reuse a CID by semantic target id. Record both requested and resolved locator identity in the binding receipt, and add a force-rebind test where the target id remains constant while the repository or revision changes.

### P8 — The specification artifact set is not audited for reviewability — **blocker** (for the product deliverable)

The product's primary deliverable is a specification humans and agents can review quickly. This review audits whether artifacts are internally consistent (S3, A5) but never whether the artifact *set* is reviewable. An operator inspecting one slice can encounter `proposal.md`, `spec.md`, `design.md`, `tasks.md`, `model.yaml`, `refinement.yaml`, and per-source Evidence — beside change-level `change.md`, `plan.yaml`, `discovery.yaml`, `leads.md`, and `decomposition.yaml`. No single document answers "is this slice specified correctly?" with a yes or no; the gap inventory lives in a separate verb (`plan gaps`) and the provenance view in another (`slice provenance`).

**Recommendation.** Make reviewability a designed property: one canonical, diff-friendly review document per slice that folds requirements, provenance, open gaps, and conflicts into a single human-orderable surface, with the structured artifacts derived from or subordinate to it. Measure review time as a product metric. The artifact taxonomy shrinks to what that surface needs.

### P9 — There is no conversational correction surface for a stuck slice — **major** (product gap, new design work)

The product requires human conversational input to revise or correct stuck build elements. Today the only steering surface is `plan correct`, which operates at decomposition-domain level; slice-level recovery is "fix inputs and re-run the stage." A stuck build offers the operator a typed stop reason and nothing to say back.

**Recommendation.** Extend the `plan correct` pattern to the slice: a stuck slice exposes its typed stop and repair brief, and accepts operator guidance recorded as a durable, digest-bound fact that becomes a hard input to the retry at refine or build. Conversation stays at the call site and never gains lifecycle authority — the guidance fact does. This is additive design work the corrective cuts below do not otherwise produce.

---

## S — State, authority, and recovery

### S1 — The journal is the database and is not built as one — **blocker**

Comments still say the journal is observability:

```11:16:crates/project/src/journal/emit.rs
/// The journal is observability, not the source of truth, so a failed
/// append is intentionally swallowed — it can never change the calling
/// verb's exit code.
```

Readers do not encode that distinction. Authoritative facts include `plan.execute.started` (authorization), `slice.claimed` (ownership), `target.merge.wave-committed` (accepted CID and terminal status), `gap.deferred` (build scope), `plan.publication.materialized` (drain), `slice.archive.created` (lifecycle). Some writes are strict `append_one`; others are `emit_best_effort`. Projections fold both.

The substrate:

- Per-writer unlocked NDJSON. `append` is read-modify-write of the next sequence (`crates/project/src/journal/append.rs`), then append bytes. Two processes sharing `EMERY_WRITER` race.
- Writer id is an env var (`journal.rs` `writer_id`), "never a capability."
- Union order is `(timestamp, writer, sequence)` — wall clock across writers. Accepted-CID projection walks that order as a strict chain (`crates/project/src/wave/accepted.rs`). Skewed clocks can permanently poison the chain; there is no repair verb.
- Unparseable lines are skipped (`read_union_dir`). A corrupt wave-commit disappears rather than failing closed — then manifests as a broken chain at a distance.
- `guest.lock` guards refine and execute, but not authoring, topology mutation, survey, or archive. It is not self-healing after a crash and requires deleting a file by hand — contradicting "never hand-edit `.emery/`" and "re-run the stopped stage is resume."

`project_ladders` documents "`done` comes from archive / postflight-failed facts" and then also terminalizes on `TargetMergeWaveCommitted` (`crates/project/src/plan/execution.rs`). Comment and code disagree; the code is the authority.

**Recommendation.** Choose one model:

- **Event store:** schema/versioning, per-writer lock or single-shot append with recovery, causal order on chain-forming events (explicit prior CID, not wall clock), fail-closed reads for commit/epoch/claim kinds, a `change_id` on every workflow-authoritative fact, compaction.
- **Observability log:** explicit state manifests (wave, build record, refinement, publication record) are authoritative; the journal never gates lifecycle.

A hybrid that documents the second and implements the first is how recovery became a matrix of typed stops plus two cases of forbidden filesystem surgery (stale lock, duplicate build records).

**Preferred resolution.** For a single-node, single-operator CLI already guarded by a global lock, choose the second model in its strongest form: **one transactional state store per change home** (SQLite, or a single atomically swapped state document), with the journal demoted to pure observability. That one decision *dissolves* — rather than fixes — S2 (the execution snapshot becomes a query), S3, S6, S7, S8, S10, S11, D9, and D10: ten findings, six of them blockers, for less machinery than the event-store option adds. Generation identity, authorization tokens, receipts, manifests, and leases are the cost of keeping the event store; do not pay it without deciding you need an event store. "Resumable on failure" is what transactions are; "parallelisable" later means concurrent readers of one store, not a multi-writer fact-union protocol (S4 already notes no CAS backend exists).

### S2 — Status and dispatch do not consume one execution snapshot — **blocker**

Execute loads `plan_status_body`, maps it through `dispatch_status`, then independently computes `ready_set` and may dispatch a different slice/phase. When status targets a phase and the ready set is empty, the loop invents `StopReason::Stuck`:

```147:157:crates/change/src/orchestrate/execute.rs
        if builds.is_empty() {
            // The status projection targeted a phase but the ready set
            // is empty — plan state moved underneath us. Surface it as
            // the stuck stop rather than spinning.
            return Ok(ExecuteOutcome::Stopped {
                reason: StopReason::Stuck,
                ...
```

`Stuck` exists because the two computations can diverge. Status now imports `project_ladders`, `resolve_entry`, and related kernels from `execution.rs`, so these are no longer two wholly independent state machines. The remaining split is still architectural: status projects one action, `schedule::ready_set` independently re-derives freshness, eligibility, phase, and work identity, and the execute loop adds publication, domain, and postflight interceptions.

The three-rung `Status` ladder (`pending | in-progress | done`) is too coarse for refine/build/merge, so both projections re-derive finer phase from artifacts independently. Wave-commit marks members `done` before archive and postflight run (`merge.rs` `emit_wave_committed` then `archive_member` then `postflight_members`). `ready_set` skips `done`. `resume_postflight` exists on the merge orchestration; the scheduler can make it unreachable except via the sticky postflight special case. A crash after commit and before postflight can project drained/materialize with an unarchived slice tree.

**Recommendation.** One immutable `ExecutionSnapshot`, computed from one validated plan/fact/artifact read: typed per-slice state, authorization, readiness or blocked reason, work identity, and global actions. Both `plan status` and dispatch consume it. `Stuck` should be unrepresentable. Model merge as explicit durable stages (`candidate-captured` → `wave-committed` → `members-archived` → `postflight-completed` → `terminal`).

### S3 — Five authority planes, no transaction — **major**

Authority is deliberately plural:

| Question | Authority |
| --- | --- |
| Topology | `plan.yaml` + `decomposition.yaml` |
| Spec inputs | `refinement.yaml` pins, recomputed live |
| Built? | `builds/<digest>.yaml` |
| Merged / done? | journal facts (several kinds) |
| Product bytes | accepted CID fold over wave-commit |
| Authorized? | `plan.execute.started` coverage |
| Domain converged? | `DomainRound` files |
| Debt? | gap facts *and* wave-commit `deferred[]` *and* prose notes in baseline `spec.md` |
| In scope? | `metadata.yaml.dropped_at` — stored, despite "progress is never stored" |

Merge is non-transactional across preflight, workspace commit, capture, wave-commit append, archive, postflight. Build promotes staged artifacts then writes the build record (`crates/slice/src/orchestrate/target/machine.rs`); a crash between leaves promoted outputs without a record. Epoch coverage pins *digests*; build assemble reads *live* slice files. System proposal persists `system.yaml` then `migration.yaml` as two acts; a crash skips initial proposal on rerun because a target is present.

Debt is triple-encoded. Archive already warns when a wave snapshot member has no covering `gap.deferred` fact. `emery debt` parses magic strings out of baseline notes and degrades on mangled prose.

**Recommendation.** Single debt authority (facts; notes and wave snapshots derived, never parsed back). Pin covered spec bytes through the snapshot store at epoch digests. Make build promotion transactional with the record. Make system proposal/survey replacement atomic (write to a staging tree, rename).

### S4 — RFC-96 concurrency landed before serial recovery was sound — **major**

Concurrency is real: default pool cap 4, max 8; builds overlap; refinements, survey, and decomposition also use the pool; same-target ready groups freeze multi-member waves. Merges are globally serial. A landed merge requeues stale-base sibling builds — thrown-away live-model work by design.

The product remains a single-supervisor CLI. Multi-writer claims are check-then-append with last-writer-wins projection (`crates/project/src/journal/claim.rs`). No production path emits `SliceReleased`. The global `guest.lock` already excludes a second execute. Emery pays distributed-ownership complexity without an atomic claim backend.

Domain convergence (`crates/change/src/orchestrate/converge.rs`) is a second target-verify protocol outside the RFC-90 machine: no repair budget, no attempt tree, report stored as a digest. Failed rounds reuse by identity, so "re-run execute" does not retry verification unless some other input is changed.

**Recommendation.** Default execution to cap one until failure side-effects are deterministic and the serial transaction model (S1–S3) is sound. Treat cap>1 as opt-in. Fold domain verification into the engine-owned phase protocol and persist full reports. Delete multi-writer claims unless a real CAS backend exists; the lock is enough.

### S5 — Publication "verified" does not compare content to the accepted CID — **major**

Three unlinked identities: accepted CID (snapshot store), publication worktree git state (operator-owned), forge merge commit (archive observation). RFC-95 D5 records that an operator who amends the commit archives green. The kernel already records the engine-staged tree under `refs/emery/publication/<plan>` and never reads it at archive.

Publication also gates `drained`: export machinery (clones, dirty worktrees, network) is on the critical path of workflow completion. `$EMERY_HOME/publication/` is never swept.

**Recommendation.** Grow the materialized fact with the staged tree SHA; at archive, advisory-compare landed content (keep D5's non-gating posture if the operator must be allowed to amend, but stop calling an uncompared forge row "verified"). Sweep publication slots. Do not let export failures masquerade as incomplete delivery of the accepted CID — the CID already exists.

### S6 — Historical facts leak across plan generations — **blocker**

The change journal is append-only across authoring and force-authoring, but most execution facts do not carry a plan revision or change-generation id. `project_ladders` marks a current entry done from any historical `slice.archive.created` or `target.merge.wave-committed` fact naming that slice, and `ensure_authored` accepts any historical `plan.reconcile.completed` carrying the same plan name (`crates/project/src/plan/execution.rs`).

Re-authoring or removing and re-adding a slice name can therefore inherit completion and authorization-adjacent history from a different topology. Digest checks on selected artifacts do not solve the reducer's namespace collision.

**Recommendation.** Mint an immutable `ChangeGenerationId` when authoring creates or force-rebinds a plan. Stamp every lifecycle event, receipt, archive, build record, domain round, and publication record with it. Reducers must reject mixed-generation facts. A force rebind starts a new generation; it must never reinterpret the prior generation's terminal events.

### S7 — `plan drop` does not reliably remove the entry from scope — **blocker**

`plan drop` calls `slice::discard`, which writes `dropped_at` and renames the live slice directory into the archive (`crates/change/src/plan/handlers/drop.rs`, `crates/slice/src/actions/discard.rs`). Scope projection subsequently looks for metadata at the now-missing live path, and `in_scope` treats missing metadata as in scope (`crates/project/src/plan/scope.rs`).

The implementation destroys the marker the reducer needs. A dropped slice can remain eligible, and archived metadata cannot be the authority for a live-plan scope decision.

**Recommendation.** Represent drop as a generation-scoped, digest-bound plan fact or explicit tombstone retained in the live authority set. Project scope from that authority before artifact existence. Archive movement is cleanup after the state transition, not the transition itself.

### S8 — A merge may commit after the authorization coverage is stale — **blocker**

Execute checks epoch coverage when the drain starts and before claims. A slice that is already built can pass through merge without revalidating that the current plan digest, refinement digest, topology, base pin, and target policy are still the authorized set (`crates/change/src/orchestrate/execute.rs`, `crates/project/src/plan/epoch.rs`). Mutable plan fields such as `allow_composition_replace` are reloaded later and can influence merge under an older epoch.

The current lock reduces same-process races but does not turn an old epoch into current authority after an intervening legal mutation or interrupted resume.

**Recommendation.** Every irreversible transition, especially candidate capture and wave commit, must compare a single authorization token covering generation, plan digest, entry digest, refinement digest, base CID, target identity, and merge policy. Topology or policy mutation revokes outstanding tokens. Never mix fields reloaded from a newer plan with an older token.

### S9 — A synthesis event can falsely project refinement completion — **major**

Refinement journals `slice.synthesize.completed` before full slice validation and before atomically publishing `refinement.yaml` (`crates/slice/src/orchestrate/refine.rs`). Execution projection treats that event as enough to advance the refinement ladder. A failure in the remaining steps can make status report progress that the authoritative manifest does not support.

**Recommendation.** Publish the validated bundle and refinement manifest as one committed receipt, then emit the completion event from that receipt. Projection should derive `refined` only from a verified manifest whose input and output digests match; intermediate judgment events are observability.

### S10 — Build authority is unverified filesystem presence — **major**

`BuildRecord::present` treats any YAML file in the records directory as evidence that a build exists, while `load_all` parses files without checking that the filename agrees with the record's content digest (`crates/project/src/build_record.rs`). Stray, stale, copied, or renamed YAML can affect readiness and merge eligibility.

**Recommendation.** Store records under a generation-scoped content address, verify filename-to-content identity on read, and index them through a committed manifest. Directory presence is never lifecycle authority.

### S11 — Decision promotion is neither atomic nor retry-idempotent — **major**

Decision promotion computes a batch, then writes each new decision file and each superseded baseline file sequentially (`crates/project/src/decisions.rs`). A mid-batch failure leaves a prefix committed. On retry, the `(slice, slug)` guard skips that prefix and recomputes numbering and supersession against the partially changed baseline, so the retried result need not equal the originally intended batch.

**Recommendation.** Stage the complete decision catalogue in a sibling directory, validate all ids and supersession edges against the staged result, then atomically swap a manifest or directory pointer. Give the promotion a receipt keyed by generation, slice, and input digest; an identical retry returns the original assigned ids.

---

## D — Deployment, dual paths, Omnia thesis

### D1 — Native and Wasm are two products; CI proves the shadow — **blocker**

Two full providers (`crates/native/src/provider.rs` ~690 lines, `crates/guest/src/provider.rs` ~900) implement the same capability traits. Native tests disclaim component ABI, WIT mapping, isolation, digests, and the store. Graded eval uses the native provider. The wasm example is operator-invoked, live-model, ungraded. Guests get `cargo check --target wasm32-wasip2` only.

Semantic divergence already exists:

- Adapter resolution (catalog exact-match vs cache/store/pull-on-miss).
- MCP paths (`/mcp/<name>` vs `/mcp/<axis>/<name>`).
- Guest shelf URL hard-codes `127.0.0.1` and parses `HTTP_ADDR`.
- Exec-mode `resolve_root` accepts `.` and workspace roots only (`crates/wasi-exec/src/host/default_impl.rs`). Guest bind of a remote locator snapshots `/emery-staging/…`, which is neither — native `FsExecMode` succeeds; the shipped deployment should fail. Verify with `cargo make wasm-run`; from the code this is a bug, not a choice.

**Recommendation.** Decide this now, at the decision gate — do not defer it as "a product decision" while paying the bill. The product definition contains no third-party adapter requirement: both adapter families are first-party, in one sibling repository, maintained by the same team. The default should therefore be **native-only**: adapters as compiled-in crates behind the existing `adapter::Source` / `adapter::Target` traits. That deletes `guest` (~1.2k lines), the launcher install/resolver matrix, `wasi-exec`, `wasi-vcs` trampolines, four of the five hand-maintained DTO families (A1 mostly evaporates), D5, D6, D7, D8, half of `project::adapter` — and T1, because the untested seam ceases to exist. The WIT contract and trait seam survive as the *shape*, so componentization can return behind the same traits when a third-party adapter actually exists.

If the decision instead retains Wasm, the minimum is a CI-runnable Wasm smoke test with a *scripted* model: metadata, one source round-trip, one target phase report, one `read_doc` MCP hop — the single cheapest de-risking in either repo.

**Resolution.** [ADR-0002](decisions/0002-deployment.md) (accepted 2026-08-17) takes the second branch, strengthened: **Wasm-primary**. Dynamic adapter admission and desktop/web-service duality are foundational requirements a prior non-Wasm generation failed on, so the duality is removed in the other direction — the native provider and the five-mode resolution matrix are deleted, the component seam becomes the sole (and CI-tested) seam, first-party components embed in the binary, and D7/D8/T1 are re-priced as scheduled platform-hardening features rather than deferred costs. This finding's diagnosis (two products; the tested seam is not the shipped seam) stands; only the default remedy proposed above is superseded.

### D2 — In-place vs detached is encoded five times — **major**

Mode is decided in `Roots::resolve`, then re-encoded as path-equality on `Layout`, an explicit `detached` bool on `ExecutionPaths`, ambient env (`EMERY_DETACHED` / `EMERY_CHANGE_ROOT` / `EMERY_PROJECT_ROOT` via `unsafe set_var`), `allow-in-place` on the worktree WIT record, and duplicated `target-base-freeze-detached` strings in both providers.

No `--change-dir` and no `project.yaml` ancestor: the invoked directory silently becomes a detached change home and the launcher pre-creates directories. Running `emery` in the wrong place scaffolds state.

**Recommendation.** Make every change home detached-shaped. The product checkout is an explicit binding, not ambient anchoring. Kill the null-object `ProjectConfig`, path-equality `is_detached`, the env-var channel, and silent scaffold-in-any-directory (error unless `--change-dir` is explicit).

### D3 — Workspace-kernel placement is unmeasured — **major**

The snapshot store is a justified second content-addressed identity (ignore policy ≠ git; wasm-clean; digest parity). Running that kernel *in the guest* introduced `emery:exec-mode`, a blobstore backend, and 4 KB `wasi:io` streaming (`crates/guest/src/workspace.rs`). RFC-95 demonstrates an alternative host-capability pattern for worktree export, but the repository has no benchmark comparing throughput, memory, isolation, or implementation complexity. The review should not call the host placement better without that evidence.

Meanwhile every workflow also materializes git object databases in `staging/` and `publication/`. Worktree export is store→git transcode. Manifests are flat (no subtree sharing). Hardened bind clones full history; the ambient archaeology leg uses `--depth 1`.

**Recommendation.** Benchmark guest versus host snapshot I/O and capability complexity before committing to this cut. If the current cost is confirmed, re-home the workspace kernel as a host capability (restore `emery:workspaces`, or fold into an effect-shaped host seam). Keep the store-neutral CID. Independently add `--depth 1` or a tree filter to the bind clone.

### D4 — Host WIT carries Emery workflow nouns — **major**

The Omnia thesis: runtime core knows only typed effects. The WIT signatures are close. The deployment is not.

`emery:vcs` `worktree.request` carries `plan`, `target`, `allow-in-place`, and its doc comment bakes in `$EMERY_HOME/publication/<plan>/<target>/` (`crates/wasi-vcs/wit/vcs.wit`). `wasi-vcs` and `wasi-exec` depend on `project` for `ExecutionPaths`, mount constants, and workspace-id grammar. Six untyped env strings cross the boundary. The launcher pre-parses argv to decide mounts, refresh sets, and definition roots — deployment policy re-deriving domain routing.

Capability crates are Emery policy linked twice (guest + host), not swappable generic effects.

**Recommendation.** WIT requests should be effect-shaped (`repository`, `cid`, `destination`, `credentials`). Placement policy stays in the engine guest. Move env anchoring onto a typed host record. Accept that Omnia core stays generic only because Emery-specific policy was pushed into `launcher` — and then stop adding workflow nouns to WIT.

### D5 — Adapter resolution has five modes and no durable settled identity for bare names — **major**

Selector kinds: package pin, bare name, component path. Behaviors: pull-on-miss pin install, cache seed, newest store version, pull-latest, cache mirror. `resolve_*` is documented read-only; the pin path installs during a metadata dispatch. Bare versions stay `None` in the guest; the settled identity is a stderr log. `plan.yaml` for a bare-bound run is not reproducible from the change home. Cache seeds beat `adapter upgrade` with a warning.

**Recommendation.** Persist exact pins. Local components are an explicit development path. Pull-latest is `adapter upgrade` only. Journal `(name, version, origin)` when a bare dispatch settles, until bare names are gone.

### D6 — Reference hosting is a five-hop network path for static markdown — **minor** (keep architecture, fix fragility)

Embed-time walk → compiled doc table → per-guest MCP over `wasi:http` → host listener + `HTTP_ADDR` → `mcp_route` → grant to the spawned agent. Plus a native hosting stack. The lazy-fetch design is sound (RFC-96 D9). Guest-side URL reassembly is not.

**Recommendation.** Inject a fully-formed `MCP_URL_BASE` from the host. Cover one `read_doc` round-trip in the Wasm smoke test (D1).

### D7 — Adapter isolation is not a capability boundary — **blocker**

The shipped runtime registers broad host capabilities and global mounts at the composition root (`src/main.rs`). The resolver verifies component identity and then returns arbitrary component bytes to Omnia (`crates/launcher/src/resolver.rs`), but Emery does not define or test a per-axis capability profile proving that a source adapter cannot reach VCS, blobstore, exec-mode, listener, or unrelated filesystem resources.

The source contract says "no change-home filesystem grant." That is currently a convention at the operation DTO, not a deployment-enforced security property. A compromised or simply defective component must be contained by its instantiated world and preopens, not by prompt prose.

**Recommendation.** Instantiate each adapter axis under an explicit least-privilege profile. Sources receive model plus the typed input value; targets receive only the private workspace, artifact stage, and operation-specific effects; the engine guest alone receives workflow state. Add malicious fixture components that attempt every forbidden import, path, environment variable, and network route, and require deterministic denial in CI.

### D8 — Wasm execution has no enforceable resource or liveness budget — **blocker**

Model work has logical repair and inactivity budgets, but component execution has no architecture-level wall-clock deadline, fuel or CPU quota, memory ceiling, output-byte cap, or cancellation guarantee. `project::pool` checks inactivity only when the future is polled; it does not install a timer wake (`crates/project/src/pool.rs`). A silent pending task can therefore prevent the timeout condition from ever being observed.

**Recommendation.** Put budgets at the host dispatch boundary: wall-clock deadline with a real timer, fuel/epoch interruption, memory/table limits, response-size limits, and cancellation that terminates the guest. Treat model inactivity as telemetry, not the only watchdog. Test silent-pending, infinite-loop, memory-growth, and output-flood fixtures through Wasm.

### D9 — Domain verification cache identity is incomplete and ambiguous — **blocker**

`DomainRound::same_key` compares domain, kind, targets, revision, bases, children, and waves, but omits `results` and `protected_inputs` — fields that describe the candidate actually verified (`crates/project/src/domain.rs`). `find` returns the first matching file from unsorted directory iteration. Two rounds with the same partial key can therefore select different verification results nondeterministically.

**Recommendation.** Define one canonical `DomainAttemptId` over every verification input, including generation, accepted or candidate results, protected-input closure, target identities, and verifier version. Verify filenames against content digests, sort only for deterministic diagnostics, and reject duplicate ids with non-identical bytes. A failed attempt may be cached, but retry policy must mint an explicit new attempt rather than depend on incidental input changes.

### D10 — Publication can accept a stale materialization fact — **blocker**

`materialized_fact` matches plan name, target, and accepted CID, then returns the recorded plan digest without requiring it to equal the current plan digest (`crates/project/src/plan/publication.rs`). If topology or publication ordering changes while the accepted target CID remains the same, an old worktree can satisfy the current publication projection.

**Recommendation.** Key materialization by change generation, exact plan digest, publication-set digest, target, accepted CID, destination identity, and staged tree SHA. Recompute or refuse on any mismatch. The topology lock should consume the same identity rather than a looser event scan.

### D11 — Ambient-credential archaeology bypasses declared fetch limits — **major**

The `trees.fetch` request carries `TreeLimits`, and the credential-free delivery path enforces them. The ambient-credential system-survey branch calls `ambient(staging, locator)` without forwarding `limits`; Git clone and HTTPS download therefore have no byte, file-count, or traversal budget (`crates/project/src/vcs/host.rs`). The most privileged fetch path is the least bounded.

**Recommendation.** Apply one metered staging contract after every transport, regardless of credential policy: shallow or filtered clone, byte and file-count ceilings, regular-file and symlink policy, cancellation, and guaranteed cleanup. Credential choice controls authentication only; it must not select a weaker resource policy.

---

## A — Adapter contract and first-party adapters

### A1 — One contract, five hand-maintained type families — **blocker** (for change velocity)

1. `wit/emery.wit` (~710 lines)
2. `crates/adapter/src/seam.rs` (~820)
3. `crates/project/src/seam.rs` + `seam/wire.rs` (~1,300)
4. `crates/guest/src/provider.rs` mappings (~900)
5. `crates/native/src/convert.rs` (~530), self-described as "the one native copy of the mapping the wasm guest shim applies"

Plus wasm export `From` impls in `adapter/src/{source,target}.rs`. A new finding-artifact variant is a seven-file edit. Compiler exhaustiveness catches missing variants, not transposed fields. SDK⇄engine parity is tests, not types.

The WIT combined `world adapter` exists for dual-axis fixtures (`wit/emery.wit`). That fixture world is not itself a production identity contradiction: routed ids remain axis-qualified. The real contract question is narrower — the axis-neutral store requires names to remain unique, and that invariant must be enforced at package admission rather than inferred from fixture behavior.

**Recommendation.** Generate SDK and engine mirrors from the WIT, or collapse them into one wasm-clean seam crate both sides import. Native/guest conversions shrink to error-widening. Decide dual-axis legality once and enforce it. Do not rebuild the WIT seam itself — it is the healthiest boundary in the system.

### A2 — Where the contract is incomplete, adapters grew private engines — **major**

RFC-90 cleaned the *build* loop. Merge sits outside it. All three first-party targets improvised a retry: omnia `gate_report`, vectis `gate::merge_gate` ("the one surface outside the RFC-90 build loop that still folds a bounded repair leg"), contracts postflight ("Repair the baseline files in place") over a documented read-only snapshot view. A broken merged baseline can be reported repaired when nothing durable changed.

Vectis is four engines behind one crate (~10k Rust + ~10k prose): validate/lint, SVG/PNG materialize (resvg in wasm), clustering infer, scaffold copy. `validate`/`verify` return `serde_json::Value` that the same crate re-parses. `scaffold::materialize::run` is production-dead — tested deterministic copy that operations never call — because the guest cannot see the exemplar; the build agent is prompted to clone `vectis-exemplar` from GitHub instead. The engine grew `emery:vcs` `trees` for exactly this fetch. Adapters route around it.

`change_home()` (probe for `plan.yaml`) is duplicated in contracts and vectis. Vectis reads `PROJECT_DIR` and walks for `.emery/`. Merge artifact classes are hardcoded in the engine (`crates/slice/src/merge/artifact_class.rs` `"contracts"` class; comment says future metadata should drive this). Five source adapters share ~95% identical `terminal` / `content_note` / `survey_user` Rust.

**Recommendation.** Extend RFC-90 treatment to merge (engine-dispatched repair, engine budgets). Put platform set, change-home shape, staged paths, and catalog location on `BuildContext` / `Workspace` — adapters must not discover `.emery`. Route exemplars through `trees.fetch` with pinned locators on target metadata; make `scaffold::materialize` the production path; delete clone-by-prompt. Move merge classes onto `TargetMetadata`. Split vectis into wasm-free validate / materialize / infer crates with typed findings. Collapse the five sources onto an SDK prompt-assembly kernel; hoist `check_pass`.

### A3 — Engineering standards are a suggestion corpus — **minor**

`codex/rules` symlinks and MCP serving work. Contracts embeds rules then hardwires `review` as `not_applicable`. Rule IDs are inconsistent (`OMNIA-*` declared, omnia files unprefixed; vectis mixes `VECTIS-006` with unprefixed names). Enforcement is the model volunteering an id.

**Recommendation.** One rule-ID grammar with a check. Contracts review must apply contracts rules, or stop embedding them as review rules. This is not the programme bottleneck.

### A4 — Contracts merge validation fails open on unreadable or malformed documents — **blocker**

The contracts adapter's deterministic validator silently skips directory read failures, entry metadata failures, file read failures, YAML parse failures, and YAML documents without a recognized top-level marker (`targets/contracts/src/validate/parse.rs`). That validator is the preflight and postflight merge gate (`targets/contracts/src/operations.rs`). The comment delegates malformed YAML to a "format verifier," but no deterministic format verifier participates in those merge paths.

A malformed contract can therefore disappear from the document set and produce zero blocking findings.

**Recommendation.** Make traversal and parse failures first-class blocking findings with stable rule ids. Classify every supported file exactly once as valid, invalid, or explicitly ignored. Run deterministic OpenAPI, AsyncAPI, and JSON Schema parsing in both preflight and postflight; model review may add findings but cannot establish structural validity.

### A5 — Refinement freshness does not cover all claimed inputs — **blocker**

`refinement.yaml` describes an exact closed input set, but freshness cannot recompute `inputs.target-guidance`, leaves `inputs.observations` as a canonical-empty placeholder, and accepts any live baseline equal to the newest journaled post-merge baseline even when it differs from the manifest's covered baseline (`crates/project/src/refinement.rs`, `crates/project/src/refinement/freshness.rs`). A target upgrade can change guidance without staling a refinement, and a sibling merge can change overlapping baseline semantics while the old specification remains authorized.

**Recommendation.** Make every consumed input durably addressable and recomputable: exact target component identity plus guidance digest, observation-set digest, exact baseline snapshot or declared dependency transition, source CIDs, planning constraints, and tool/prompt schema version. Freshness is equality over one canonical input record; carve-outs become explicit rebases that create a new refinement receipt.

### A6 — Persisted authority types admit invalid and silently ignored states — **major**

`ProjectConfig` uses free strings for names, adapter identity, and CLI version and intentionally does not deny unknown fields (`crates/project/src/config.rs`). Validation is distributed across loaders and command handlers rather than represented by a validated authority type. Similar path/string/digest coupling recurs in plan and journal DTOs.

This weakens schema evolution: a misspelled authority key can be silently discarded, and code that receives a deserialized struct cannot know which invariants have run.

**Recommendation.** Split wire DTOs from validated domain types. Authority documents deny unknown fields, parse selectors and semantic versions at the boundary, use constrained names and relative-path types, and expose no constructor that bypasses invariants. Forward-compatible extension belongs in an explicit versioned `extensions` map, not accidental serde permissiveness.

### A7 — Capture Evidence promises replay from data delivery does not retain — **blocker**

The captures source omits `input` and `output` above 64 KiB and tells downstream consumers to replay from `path` plus `replay-digest` (`sources/captures/prose/references/extraction-mapping.md`). Those paths are relative to the source CID view available during extraction. The engine persists the Evidence document, not the capture bytes or a capability to reopen that exact source tree during target build. The target receives a private product workspace and slice artifacts, so path-only replay is impossible.

**Recommendation.** Treat large replay payloads as content-addressed Evidence attachments. Extraction imports the exact bytes into the snapshot store and records an attachment CID, media type, size, and digest in Evidence. Build requests grant read-only access only to the referenced attachments. Remove prose that implies an ephemeral source path remains available downstream.

---

## T — Testing and evidence

### T1 — The production seam has no automated rung — **blocker** (assurance)

Both testing docs state this. Per-push CI is native mock catalog + scripted doubles. Eval is live-model and starts at `plan author` (adapters eval copies a prebuilt definition home; no case runs `system survey` / `plan` / `review`). Wasm examples are operator-invoked.

The conversion layers in A1 and the staging-root defect in D1 are exactly the code only the untested seam exercises.

**Recommendation.** Same as D1: scripted-model Wasm smoke in CI. Add one definition-loop eval case before calling RFC-104 implemented as a product. Measure `omnia-r9k` (or successor) as the services-loop grade; do not staff RFC-97 until that grade is a gate, not a memory.

### T2 — Mock targets do not reproduce shipping targets' hybrid shape — **major**

Engine machine tests cover verification repair, review remediation, continuation replacement, artifact staging, malformed reports, and budget exhaustion. Shipping adapters additionally run multi-leg hybrid builds with deterministic preludes and model-assisted report composition (Vectis validation/materialization, Omnia scaffold). The mock does not reproduce that shape, so the engine's integration rung still misses the behavior production adapters rely on.

**Recommendation.** A mock target that emits multi-leg hybrid reports. Cheap, and it makes RFC-90 tests honest.

### T3 — The operator-invoked Wasm examples no longer exercise a legal workflow — **major**

The root Wasm script authors a plan and immediately executes it (`examples/Makefile.toml`). Execute now requires fresh refinement manifests and explicitly never refines. The example omits `emery plan refine`, so the only production-seam scenario stops before target build and merge.

**Recommendation.** Repair the script first, then promote its scripted, offline core into CI. Keep the live-model example as a separate operator rung. Test commands should be assembled from the same public workflow contract as help/docs so lifecycle cuts cannot silently stale the assurance path.

### T4 — Supply-chain verification regenerates its own policy in the gate — **major**

The `vet` task runs `cargo vet regenerate imports`, `regenerate exemptions`, and `regenerate unpublished` immediately before `cargo vet --locked` (`Makefile.toml`). A verification gate should check committed review state, not rewrite that state from current dependencies and upstream imports before checking it.

**Recommendation.** CI runs read-only `cargo vet --locked` and fails on drift. Regeneration is an explicit contributor maintenance task whose diff is reviewed. Pin imported audit sources and test that the repository is clean after every verification task.

### T5 — "Quick and reliable" are unmeasured qualities — **major**

The product promise is speed and reliability, and neither is a number anywhere in either repository. There is no time-to-first-specification or time-to-built-slice measurement, no per-operation success-rate tracking, and no cost or latency telemetry (RFC-92 is deferred). The prompt corpus — ~27k lines, which *is* the product's reliability surface — has no regression harness: eval is live-model, operator-invoked, and its grade is a memory (T1). The workflow examples went silently illegal (T3) because nothing measured them.

**Recommendation.** A graded eval suite as a release gate with tracked per-operation success rates, plus wall-clock and cost telemetry scoped to the product promise (a scoped-down RFC-92: attribution for tuning, not policy machinery). Reliability claims that are not gated will regress silently again.

---

## C — Crate graph and error taxonomy

### C1 — `project` is five products — **major** (mechanical split)

`crates/project/src/lib.rs` exports adapter, binding, build_record, config, decisions, domain, handler, init, journal, plan, pool, profile, refinement, seam, slice, snapshot, vcs, wave, workspace. Acyclic, not layered. Anything two upper crates need migrates here: `SystemWaveReviewed`, `DefinitionIdentity`, VCS fetch, HTTP GitHub, pool scheduling, deployment resolver.

**Recommendation.** Split along existing modules — low risk, high compile and reasoning value:

- `emery-seam` — seam + wire + pool + handler
- `emery-workspace` — snapshot + workspace
- `emery-vcs` — fetch, forge, worktree kernels (host-side)
- `emery-adapter-resolve` — selectors, store, ensure
- `project` — plan/slice/wave/domain/publication data model only

Do this after or with A1 (one seam crate), not as a sixth parallel copy.

### C2 — 227 stringly-typed `Diag` codes, some used as control flow — **major**

`Error` itself is small. The real taxonomy is `'static str` codes. `crates/system/src/orchestrate/plan.rs` matches `"system-model-missing"` / `"system-migration-missing"` to branch. A typo silently changes behaviour. Hints and install-script URLs live in the dependency-leaf crate, so renaming a verb edits `crates/error`.

**Recommendation.** Closed generated registry (same posture as `answers.rs` goldens): declare once, match as variants, kebab on the wire. Move hints to transport. Stop matching strings for absent-vs-corrupt — return `Option`.

---

## R — Systemic causes and process countermeasures

The findings above are symptoms. The codebase was substantially agent-built, and the development loop that produced the mess will faithfully reproduce it on whatever replaces it unless the loop itself changes. Four causes, each with a countermeasure that belongs in the programme's standing rules.

### R1 — RFC-at-a-time implementation with no walking skeleton — **blocker** (process)

Each RFC landed locally coherent; nothing forced the *composed* operator journey to keep working. The verdict of this review — "failed to stay one product" — is the predictable output of optimizing per-RFC with no end-to-end gate. The wasm example silently became an illegal workflow (T3) and nobody noticed until this review.

**Countermeasure.** The full operator journey runs scripted and offline in CI at all times and is the definition of done for every change. Stronger than the per-seam smoke test in D1: the *journey*, not the seam.

### R2 — Docs are the load-bearing conceptual model — **major**

AGENTS.md grew into glossary, architecture, RFC index, negative-compatibility ledger, module map, and operator manual because the conceptual model stopped being inferable from the product surface (P4). Agents faithfully extend whatever prose exists, so prose sprawl and code sprawl compound each other. The repeated "there is no…" clauses are a negative-space spec — the most expensive kind to maintain.

**Countermeasure.** A short human-owned constitution (one to two pages) carries the invariants; AGENTS.md becomes derivable from the surface and shrinks to navigation. If a rule cannot be reached from the product surface, the surface is wrong (as "What not to do next" already says — make it enforceable, not aspirational).

### R3 — Lab pressure rewrote product policy without a decision record — **major**

P3 documents it: `emery plan defer` and the `strict | defer` policy were deleted *after landing* so unattended eval could finish. A lab convenience overwrote a designed operator gate, and the change carried no decision record distinguishing it from a bug fix.

**Countermeasure.** Policy changes — anything altering an operator gate, authority rule, or disposition — require an RFC amendment with the trade-off stated, never a follow-up commit. Lab needs get lab flags.

### R4 — Addition-only programme; agents do not push back on scope — **major**

Every RFC added a plane; none deleted one. Dual providers, five resolver modes, multi-writer claims, and component-marketplace machinery all shipped before a single production use. Agents implement what is asked; scope discipline must be structural.

**Countermeasure.** Delete-before-add: every RFC names its deletions and its net effect on the operator-visible concept count. Fitness functions in CI: crate-layering checks, seam-copy count, route-count and LOC budget alarms, no new `EventKind` (or state-store table) without a reducer/consumer test.

---

## Rebuild versus refactor

The corrective programme below is a six-cut refactor across ~130k lines (engine, prose, adapters), each cut preserving behavior nobody depends on yet — the product is pre-production. The alternative it must be costed against: write the target-architecture document, then rebuild the core loop as a walking skeleton that reuses the kept kernels — the content-addressed snapshot store, the RFC-90 phase machine, the artifact parsers, the adapter traits, and the adapter prose corpus (which ports nearly intact) — deleting the rest. Given that agent labor produced 100k lines against incoherent requirements, regenerating ~40–50k against a settled spec is plausibly cheaper and safer than six sequenced surgeries, and it is the only path that reliably yields the property Omnia has: coherence from being built to a settled architecture rather than repaired toward one.

At minimum, Cuts 1–2 (the state model and executor) should be **greenfield-in-place**: new store, new executor, old code deleted rather than migrated. The pre-1.0 reset already recommended in Cut 1 makes this cheap; there is no historical state worth a migration path.

---

## What not to do next

Do not staff RFC-92, RFC-94, or RFC-97 against this substrate until generation-scoped authority, the single execution snapshot, review-to-delivery constraints, and the confined Wasm CI rung exist (P5, S1–S2, S6, S8, D1, D7–D8). RFC-97's own review already found the `target.verify` dual-mode wire unspecified and the sandbox contract infeasible; implementing it on a journal that is not a database and a Wasm seam that is neither confined nor in CI will freeze the wrong shapes.

Do not add RFC-106 task graphs. Vectis already has an untracked sub-phase machine inside one `build`. Intra-slice graphs would formalize that opacity rather than lift it into the engine.

Do not grow AGENTS.md. If a rule cannot be reached from the product surface, the surface is wrong.

Do not add another host WIT package (`emery:verification`, `emery:publication`) until D4 has made the existing capabilities effect-shaped and Cut 4 has established one generated seam and explicit capability profiles.

---

## Corrective programme

This is a dependency order, not a new RFC stack. Freeze feature work while Cuts 0–3 land. Prefer hard replacement over compatibility layers: the product is pre-production, and preserving ambiguous historical state would defeat the repair.

A **decision gate** sits between Cut 0 and Cut 1 (see "The yardstick" above): the product-definition and target-architecture documents, plus the recorded decisions on state model (S1), deployment (D1), lifecycles (P1), conflict disposition (P3), and change-home shape (D2). The gate artefacts now exist: [product.md](product.md), [decisions/](decisions/) ADR-0001…0008, [target-architecture.md](target-architecture.md) (draft v0.1), [CONSTITUTION.md](../CONSTITUTION.md), and [remediation-plan.md](remediation-plan.md), which sequences execution. **Live execution is the spec-generator programme (ADR-0008), not Cuts 1–5 as written.** Cuts 1–5 assume answers the programme has since narrowed; they remain the evidence base for the deferred annex.

The [addendum](architecture-review-addendum.md) additionally widens Cuts 1–5 — the authoring generation set (S19/S25/S26), wave/refine/publication as first-class executor stages (S32–S40), the inverse definition coupling and observation receipts (P11/P12, S41–S45), the one open claim family (A8/A16), and one topology compiler (P10/S31) — see its "Corrective programme — what changes". Its exceptions table also bounds the state-store dissolution claim: missing *types and seams* do not dissolve and stay on the cuts.

### Cut 0 — Contain known unsafe behavior (days)

1. Disable `plan author --force` target-CID reuse unless canonical locator and revision match; add the changed-locator regression (P7).
2. Fix `plan drop` so a durable tombstone excludes the entry before archival movement (S7).
3. Make contracts traversal and parse failures block preflight and postflight (A4).
4. Repair the Wasm example with `plan refine`; put its scripted offline path in CI (D1, T1, T3).
5. Default the pool to one and add a real timer wake for inactivity (S4, D8).
6. Enforce fetch limits on ambient-credential archaeology and guarantee staged-tree cleanup (D11).
7. Make `cargo vet` read-only in CI; split regeneration into a maintenance task (T4).
8. Reject detached-home inference without explicit `--change-dir`; pin settled adapter identities; fix the staging-root and MCP URL defects (D1, D2, D5, D6).

Widened by the [addendum](architecture-review-addendum.md) second pass:

9. Disable or park `plan correct` as a hard-constraint plane until a `CorrectionTarget` and a generation exist; if the verb ships, fact-only notes with no tail enforcement (addendum P16, S25).
10. Fail `--force` that would park without minting a new authoring generation; force-then-park must resume, not no-op (addendum S26).
11. Fail extract (or persist extras) when the model emitted per-kind body fields the seam would drop — do not wait for Cut 4 to stop losing `statement` / `criterion` / `replay-digest` (addendum A8).
12. Disable the guest HTTP mutating catch-all; MCP shelves only (addendum C3).
13. Job-scoped workspace discard on timeout/cancel; startup sweep of `$EMERY_HOME/workspaces/` (addendum S37, D12).
14. No fallback platform set; missing `project.yaml.platforms` is a typed refuse, not both shells (addendum A14, A15).
15. Probe `invoke` accepts exit 2 as a typed stop; do not grade the build-phase back door as a workflow (addendum T6).
16. Fix the always-applied plugin rule so it cannot name deleted flags (addendum P14).
17. Gap / merge-resume journal I/O failures are `Err`, not empty — the accepted-CID chain poisoning is permanent and has no repair verb (addendum S13, S23).

These are containment changes, not the target architecture. Where a correct local repair would encode more legacy state, fail closed until the following cut replaces it.

### Cut 1 — Generation-scoped authority (ground-up)

Introduce one `ChangeGenerationId` and one canonical `AuthorizationToken` (S6, S8). Every authoritative fact and artifact names its generation. Force authoring starts a new generation. Reducers reject mixed generations. Candidate capture and wave commit compare the full authorization token immediately before the irreversible effect.

The authority model in S1 is chosen at the decision gate, and this cut's shape follows from it. Under the preferred transactional state store, the cut collapses to implementing that store greenfield-in-place and deleting the fact-union reducers: generation identity becomes a column, not a protocol, and the token comparison becomes a transaction precondition. Only if the event store is retained is the full shape required:

- append-only journal for audit and causal history;
- content-addressed, generation-scoped receipts for operational authority;
- one atomically replaced manifest naming the currently committed receipts;
- a self-recovering lease for each mutating change operation.

Replace directory-presence and partial-key discovery with verified indexes for build records, domain attempts, and publication materializations (S10, D9, D10). Make decision promotion an atomic receipt (S11). Add a one-time pre-1.0 reset command rather than teaching the reducer to reinterpret old unscoped facts.

### Cut 2 — One execution snapshot and resumable transaction (ground-up)

Build one immutable `ExecutionSnapshot` from a validated generation, plan, receipt manifest, and artifact set (S2) — under the state-store answer this is a query, not a new projection layer. Status and dispatch consume the same snapshot; ready-set is not recomputed elsewhere. Encode explicit merge and publication stages, with a durable receipt before and after every external effect. `Stuck` disappears; resume is a table over receipts.

Make artifact publication and phase transition one transaction boundary (S3, S9). Reduce debt to one authority. Require explicit operator disposition for conflicts while retaining policy-driven handling for unknowns (P3). Keep cap one until crash injection proves every stage idempotent; only then reconsider parallel scheduling.

### Cut 3 — Close review-to-delivery authorization (ground-up)

Make the reviewed handoff an executable constraint bundle:

- immutable delivery mappings constrain decomposition and topology edits (P5);
- every target and source binding carries requested locator, resolved locator, exact CID, adapter pin, and review identity (P2, P7);
- retained stale Evidence is excluded, blocks review, or carries a digest-bound acceptance decision (P6);
- refinement covers a fully recomputable canonical input record, including guidance and baseline transition (A5);
- large Evidence bodies become content-addressed attachments available to authorized target builds (A7).

Archive writes a delivered-wave fact back to the definition home and reprojects the living architecture baseline (P2). This cut is complete only when a changed reviewed mapping, locator, Evidence observation, guidance body, or baseline invalidates the downstream authorization deterministically.

### Cut 4 — Enforce the runtime boundary (ground-up)

Generate the Rust seam and conversion code from one contract, or make the WIT-generated types the sole wire family (A1). Instantiate source, target, and engine guests under separate least-privilege profiles (D7). Put wall-clock, fuel, memory, table, response-size, and cancellation budgets at host dispatch (D8). Add malicious and resource-exhaustion components to the automated Wasm rung.

Make host WIT effect-shaped and remove workflow nouns (D4). Persist exact adapter pins and make pull-latest an explicit upgrade act (D5). Decide D3 from benchmark evidence: move snapshot work host-side only if that measurement justifies the extra capability. Split `project` after the seam is singular (C1), and generate the diagnostic registry while moving hints to transport (C2).

### Cut 5 — Product and adapter closure (ground-up)

Make definition an optional compiler over the shared source-axis kernel (P1) — or, under the one-lifecycle answer, fold estate inputs in as ordinary sources and delete the second lifecycle outright. Keep the CLI as the lifecycle owner and present one operator journey (P4), stated against the concept budget. Prefer detached-only change homes (D2). Design the per-slice review document (P8) and the conversational correction surface for stuck slices (P9) — the two product deliverables no earlier cut produces.

Move merge verification and repair onto the engine phase protocol; pass typed build context; route exemplar fetch through a declared effect; split Vectis's deterministic compiler functions from model judgment; remove adapter-local change-home discovery and duplicated source operations (A2). Make the shipping-shape mock target part of the engine integration suite (T2).

### Cut 6 — Then measure

Only after Cuts 0–5: RFC-92 (routes and cost on a substrate that can attribute), RFC-97 Phase A (host verify on a confined seam that CI runs), and RFC-94 (admission). Re-read [rfc-97-native-verification.md](rfc-97-native-verification.md) and [rfc-97-phase-a-plan.md](rfc-97-phase-a-plan.md) before touching verify. A feature RFC that requires a new lifecycle state before these cuts is evidence the cut is incomplete.

The reliability gate (T5) does not wait for this cut: the graded eval suite lands with Cut 0's CI work and gates every cut after it; Cut 6 adds the cost and latency attribution on top.

## Ground-up vs patch summary

| ID | Finding | Verdict | Cut |
| --- | --- | --- | --- |
| P1 | RFC-104 second product | Ground-up: optional compiler + one source kernel | 5 |
| P2 | Review does not freeze delivery or advance a living baseline | Ground-up: binding receipts + delivered-wave fact | 3 |
| P3 | Auto-defer inverts spec-first for conflicts | Policy correction inside execution rewrite | 2 |
| P4 | Operator journey is obscured | Product-surface simplification against a stated concept budget | gate/5 |
| P5 | Reviewed delivery mappings are discarded | Ground-up: constraint-carrying handoff | 3 |
| P6 | Fresh review can cover stale Evidence | Ground-up: explicit stale-Evidence authority | 3 |
| P7 | Force rebind can reuse the wrong target CID | Contain, then generation-scoped binding receipt | 0/3 |
| P8 | Specification set is not reviewable as one surface | New design: one review document per slice | 5 |
| P9 | No conversational correction for a stuck slice | New design: digest-bound slice guidance facts | 5 |
| S1 | Journal is a weak database | Decide at gate: transactional state store preferred (dissolves S2, S3, S6–S8, S10, S11, D9, D10) | gate/1 |
| S2 | Status and dispatch consume different projections | Ground-up: one immutable execution snapshot | 2 |
| S3 | Five planes, no transaction | Ground-up receipt manifest and artifact transaction | 1–2 |
| S4 | Concurrency before serial soundness | Contain at cap one; re-enable only after crash proof | 0/2 |
| S5 | Verified publication is uncompared | Content identity in publication receipt | 1–2 |
| S6 | Historical facts cross plan generations | Ground-up generation identity | 1 |
| S7 | Drop loses its own scope authority | Contain with tombstone; fold into generation state | 0/1 |
| S8 | Merge can use stale authorization | Ground-up authorization token | 1–2 |
| S9 | Synthesis event can project false completion | Receipt-after-commit transition | 2 |
| S10 | Build authority is directory presence | Verified content-addressed index | 1 |
| S11 | Decision promotion is not retry-idempotent | Atomic catalogue receipt | 1 |
| D1 | Native proven, Wasm shipped | **Decided (ADR-0002): Wasm-primary** — delete native provider + resolver matrix; CI seam rung; D7/D8 scheduled | gate/0 |
| D2 | In-place/detached × 5 | Ground-up: detached-only | 5 |
| D3 | In-guest workspace cost is unproven | Benchmark, then retain or move | 4 |
| D4 | Workflow nouns in host WIT | Ground-up effect-shaped seam | 4 |
| D5 | Five resolver modes | Contain, then exact pins only | 0/4 |
| D6 | MCP hop fragility | Patch | 0 |
| D7 | Adapter isolation is not enforced | Ground-up per-axis capability profiles | 4 |
| D8 | Wasm has no resource or liveness budget | Timer containment + host budgets | 0/4 |
| D9 | Domain cache identity is incomplete | Ground-up attempt identity and index | 1 |
| D10 | Publication accepts stale materialization | Ground-up publication identity | 1 |
| D11 | Privileged VCS fetch bypasses limits | Meter every transport | 0 |
| A1 | Five DTO copies | Ground-up: generate or share one family | 4 |
| A2 | Adapter-private engines | Ground-up phase/context/effect split | 5 |
| A3 | Rules unenforced | Patch | 5/opportunistic |
| A4 | Contracts validation fails open | Fail-closed containment | 0 |
| A5 | Refinement does not cover claimed inputs | Ground-up canonical input receipt | 3 |
| A6 | Authority DTOs admit invalid states | Validated domain types | 4 |
| A7 | Capture replay bytes are not retained | Content-addressed Evidence attachments | 3 |
| T1 | No Wasm CI | Patch, highest value | 0 |
| T2 | Mock does not reproduce shipping hybrid shape | Integration fixture | 5 |
| T3 | Wasm example skips required refine | Patch before promotion to CI | 0 |
| T4 | Supply-chain gate regenerates policy | Read-only gate | 0 |
| T5 | Reliability and latency are unmeasured | Graded eval gate + scoped telemetry | 0/6 |
| C1 | `project` dumping ground | Mechanical split after A1 | 4 |
| C2 | String `Diag` control flow | Generated closed registry | 4 |
| R1 | No walking skeleton; RFC-at-a-time drift | Process rule: full journey scripted offline in CI, always | gate |
| R2 | Docs are the load-bearing conceptual model | Process rule: short human-owned constitution; AGENTS.md derivable | gate |
| R3 | Lab pressure rewrote policy without a record | Process rule: policy changes require an RFC amendment | gate |
| R4 | Addition-only programme | Process rule: delete-before-add, concept budget, CI fitness functions | gate |

## Acceptance for this review

The programme has succeeded when:

1. A contributor can describe the operator journey without a 500-line AGENTS.md paragraph.
2. One `ExecutionSnapshot` is the only source of status and dispatch decisions; disagreement is unrepresentable.
3. Every lifecycle authority is scoped to one change generation and every irreversible effect verifies one complete authorization token — or, under the state-store answer, is a transaction against the one store.
4. Crash injection at every write and external-effect boundary proves that re-running the stopped verb is sufficient recovery; no manual deletion, duplicate-record surgery, partial decision promotion, or broken accepted-CID state remains.
5. One offline CI job runs the full operator journey — sources to specification to built, merged slice — on every push; if the Wasm deployment is retained, that job crosses the WIT seam and detects conversion drift.
6. If the Wasm deployment is retained: malicious Wasm fixtures cannot access undeclared files, environment, network, VCS, blobstore, or exec-mode capabilities; resource-exhaustion fixtures terminate within enforced budgets; every VCS credential mode obeys the same staging limits.
7. A reviewed delivery mapping, locator, CID, stale-Evidence decision, guidance digest, baseline identity, and refinement receipt form one traceable authorization chain through wave commit.
8. Archive leaves both the product baseline and, when a definition home exists, the architecture model advanced by an immutable delivered-wave fact.
9. Build records, domain rounds, publication materializations, decisions, and Evidence attachments verify their content identity on every load.
10. An intent-only N=1 change does not require a definition home, a publication clone, or a multi-writer claim protocol.
11. A human can review one document per slice and answer "is this slice specified correctly?" without opening a second artifact (P8), and review time is measured.
12. A stuck slice can be corrected by durable, digest-bound operator guidance without hand-editing state or abandoning the slice (P9).
13. Time-to-first-specification, time-to-built-slice, and per-operation success rates are tracked, and the graded eval suite gates release (T5).
14. The target-architecture document exists, records the five gate decisions, and every subsequent RFC names its deletions and concept-count effect against it (R1–R4).

From the [addendum](architecture-review-addendum.md) second pass:

15. Force-then-park resumes; a historical `plan.reconcile.completed` cannot make `plan author` a no-op on a parked tree.
16. `plan correct --constraint` is honored on the node that is actually recut, or the verb refuses; historical corrections do not bind a new generation.
17. A reviewed handoff imported into a change home remains executable after the definition home is resurveyed; revocation is an explicit rebase.
18. Extract extras required by synthesis (`statement`, `criterion`, `replay-digest`) survive conversion and appear in persisted Evidence.
19. Wave membership is the antichain, not the ready batch; the A-fails-B-succeeds-C-joins case either merges B or emits a typed retract, never a silent orphan.
20. `emery plan refine` refuses a leaf that holds a build record or an uncommitted wave membership.
21. Frontier and complete verify the same captured CID that wave-commit records.
22. HTTP does not expose mutating plan verbs without auth and a lease; CLI and HTTP share one `Input` and one error class table.
23. The probe runner treats exit 2 as a stop and grades `plan status` + `slice validate`, not `exit == 0` after a back-door build.
24. An intent-only N=1 change uses one topology compiler, no definition-home lease, no publication clone, and no multi-writer claim protocol.

Until then, further RFCs are completeness, not product.

## See also

- [product.md](product.md) — the product yardstick this review's later amendments audit against
- [remediation-plan.md](remediation-plan.md) — the plan of record executing this review's programme
- [target-architecture.md](target-architecture.md) — the destination the cuts are re-derived from
- [decisions/](decisions/) — the decision log (ADR-0001…0008)
- [CONSTITUTION.md](../CONSTITUTION.md) — standing invariants and mechanical anti-reversion enforcement
- [platform.md](platform.md) — programme spine this review holds to account
- [architecture.md](architecture.md) — Omnia thesis vs deployment reality (D3, D4)
- [rfc-97-native-verification.md](rfc-97-native-verification.md) / [rfc-97-phase-a-plan.md](rfc-97-phase-a-plan.md) — do not implement native verification until Cuts 0–5 land
- [rfc-86a-gap-deferral.md](archive/rfc-86a-gap-deferral.md) — recorded auto-deferral policy (P3)
- [rfc-104-system-archaeology.md](archive/rfc-104-system-archaeology.md) — definition loop (P1, P2, P5, P6)
- [rfc-96-concurrent-execution.md](archive/rfc-96-concurrent-execution.md) — pool and domain rounds (S4, D8, D9)
