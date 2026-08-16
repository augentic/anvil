# RFC-97 Readiness Review: Native Verification Profiles

> Scope: readiness assessment of [RFC-97](rfc-97-native-verification.md) for both delivery stages — Phase A (`slice-attempt` verification on implemented RFC-90) and Phase B (`frontier-domain | complete-domain` contexts on RFC-96).
>
> Inputs: RFC-97 against [RFC-90](rfc-90-build-verification.md) (implemented), [RFC-87](rfc-87-working-trees.md) (implemented), [RFC-96](rfc-96-concurrent-execution.md) (active, unimplemented), [RFC-92](rfc-92-model-policy.md) (draft), [RFC-98](rfc-98-behavioural-conservation.md) (draft follow-on), [RFC-106](rfc-106-task-graphs.md) (evidence-gated), [RFC-100](rfc-100-distributed-execution.md) (parked), [platform.md](platform.md), and the current engine workspace.
>
> Finding ids are stable (`A1…`, `B1…`, `G1…`, `M1…`). Severity: **blocker** (resolve in the RFC text before implementation starts), **major** (resolve before the affected decision is implemented), **minor** (fix opportunistically).

## Verdict

**Phase A: conditionally ready.** The trust boundary, assurance vocabulary, fail-closed posture, and normalization contract are well designed and internally consistent, and the implemented substrate is verified in place (see [What is verified ready](#what-is-verified-ready)). Four issues block implementation start: the `target.verify` wire shape is unspecified for the dual-mode contract (A1), the protected-input authoring and enforcement surface does not exist and is not defined (A2), the RFC silently amends RFC-90 D5/AC5 without declaring it or resolving the continuation question (A3), and the sandbox/cache contract as written is infeasible for real toolchains on the primary deployment (A4).

**Phase B: not ready as specified.** Beyond the declared (and correct) dependency on unimplemented RFC-96 Phase B, the Phase B text has two structural defects: binding implementation requirements and acceptance criteria to parked RFC-100 (B1), and leaking evidence-gated RFC-106 task-owner concepts into D7 and acceptance criterion 6 (B2). Phase B should be re-cut to depend on RFC-96 alone, with RFC-100 and RFC-106 material moved to conditional prose.

## What is verified ready

Checked against the current workspace, the substrate RFC-97 Phase A assumes is real:

- **RFC-90 phase machine** — `crates/slice/src/orchestrate/target/machine.rs` implements `build → verify ⇄ repair → review ⇄ repair` with engine constants `MAX_VERIFICATION_REPAIRS = 3`, `MAX_REVIEW_REMEDIATIONS = 1`, exactly as D1 assumes.
- **`phase-source: tool` reserved and gated** — `PhaseSource::Tool` exists in `crates/project/src/seam/wire.rs` and is rejected by `crates/slice/src/build/gate.rs` (`target-phase-source-tool`) with a comment naming RFC-97 as the opener. `Hybrid` is accepted. The gate must be opened, not invented.
- **Executable-bit parity** — the RFC's "real executable bits" implementation requirement is already satisfied: `Store::materialize` (`crates/project/src/workspace/store.rs`) applies genuine `chmod` through `emery:exec-mode` (`crates/wasi-exec/wit/exec-mode.wit`, two functions, exactly the capability-crate shape the RFC cites as its template).
- **Digest parity path** — snapshot objects ride `wasi:blobstore` with the `<2 hex>/<62 hex>` sharded convention; the native verifier can reach the same store through `FsObjects`, as the implementation requirements demand.
- **Journal substrate** — `slice.build.phase-completed` exists in the closed `EventKind` taxonomy; D9's new event is additive on a proven pattern.

Nothing else has started: no `emery:verification` package, no `verification-answer` type, no profile or protected-input metadata fields, no verifier capability, no mechanical-repair hook in the phase machine. That is expected — the review below is about whether the RFC text is complete enough to start.

## Phase A blocking issues

### A1 — The `target.verify` wire shape is unspecified for the dual-mode contract — **blocker**

D1 changes the `target.verify` result "from a target-authored phase report to a `verification-answer`", yet also permits targets that declare no host profiles to "retain RFC-90 model-assisted verification". Both cannot be true of one WIT signature without a closed shape the RFC never gives. Today `verify` returns `result<phase-report, error>` and the adapter worlds export their axis interface and import **nothing** — D3's host-verification import would be the first host import ever added to an adapter world, which is a breaking world redefinition for every published target component.

Unresolved:

- Is the return type a closed variant (e.g. `verify-answer = report(phase-report) | attested(verification-answer)`), or does `verification-answer` subsume the model-assisted case by carrying a full phase report? The engine gate that "may not fall back silently" needs a typed discriminant to gate on.
- Is this a hard cut for the target world (pre-1.0 posture says yes), and does the `emery-floor` metadata field carry the compatibility story for older published components?
- The header hedges the package name ("e.g. `emery:verification`"). A contract document should pin it.

**Recommendation.** Add the conceptual WIT to D1/D3 exactly as RFC-90 D2 did: the closed `verification-answer` record (handles, optional deterministic in-component findings), the closed verify return variant, the host-verification import signature (profile name + candidate handle in, attestation handle or typed unavailability out), and an explicit hard-cut statement for the target world. Pin the package name. State how the native provider supplies the equivalent capability to SDK targets (the scripted verifier in `crates/mock` implies a seam trait beside `project::seam::Workspaces`; name it).

### A2 — Protected inputs and oracles have no authoring or enforcement surface — **blocker**

D4 says "the closed execution epoch and target build request cover `protected-verification-inputs[]` … plus digest-bound `protected-oracles[]`", and AC5 asserts "no writer can modify its protected inputs". Nowhere does Phase A say:

- **Who authors the sets and where they live.** RFC-106 D3 gives the fullest answer (leaf-carried sets in the `file | tree` grammar, target-metadata-nominated defaults effective only through the admission-covered decomposition revision) — but RFC-106 is evidence-gated and unstaffed, and RFC-96 D8 only *consumes* per-member sets that Phase B "carries through member admission". Phase A is first to need the declaration surface and defines none: not the artifact (`plan.yaml`? decomposition entry? refinement manifest?), not the CLI verb that writes it, not the validation that catches an orphan path.
- **The enforcement point.** RFC-87 access manifests grant a writable code scope; making named in-tree paths read-only *inside* the build workspace is a new capability. The RFC must choose mount-time denial (extend `prepare`'s access manifest with per-path exclusions) or capture-time rejection (fail the attempt when captured touched paths intersect the protected set), and say so — the two have different failure timing and different costs. RFC-106 chose "materialized read-only … any operation that changes a protected path fails the attempt"; Phase A should state its own rule rather than inherit one from an unstaffed RFC.

Without this, D4's oracle-assurance ladder has no Phase A input: nothing can legitimately attest `protected` or `mixed`, and the only demonstrable Phase A value is `candidate` — which materially weakens the assurance story the platform document quotes ("'Verified' must name the profile and both assurances").

**Recommendation.** Add a Phase A decision that (a) names the declaring artifact and the plan-time validation for `protected-verification-inputs[]` / `protected-oracles[]`, (b) fixes the enforcement point (capture-time rejection is the smaller cut: no RFC-87 amendment, reuses authoritative touched paths), and (c) states explicitly that RFC-98 corpus admission is the first `protected-oracles[]` producer if generic Phase A oracle declaration is deferred — in which case say Phase A ships with `oracle-assurance: candidate` only and the declaration surface lands with RFC-98, so the gap is a recorded decision rather than an omission.

### A3 — Undeclared amendment of RFC-90 D5/AC5, and the continuation question — **blocker**

RFC-90 D5 pins "one workspace spans the complete loop" and its AC5 requires "the same RFC-87 workspace id … throughout one attempt". RFC-97 breaks this three ways without declaring an amendment (contrast RFC-106's header, which explicitly lists the RFC-90 decisions it amends):

1. Flow step 3 captures the attempt workspace mid-attempt and lends "a fresh RFC-87 materialization to the verifier" — a second workspace within one attempt.
2. D7 applies a mechanical-repair group "in a fresh workspace" — a third.
3. D7 then says "the tentative snapshot **replaces the slice attempt workspace** through the same RFC-87 capture/materialize boundary" — the attempt continues in a different materialization than it started in.

D8 even restates the invariant it is breaking ("RFC-90 keeps one workspace per slice-build attempt until RFC-106 rematerializes per task").

The consequential gap: RFC-90 D2 binds the adapter-opaque continuation to "the same resolved target identity, attempt, and build workspace" and says it "survives no workspace loss". After an accepted mechanical repair replaces the workspace, is the continuation preserved (the model-repair session context survives, but the binding rule is violated) or cleared (the next model repair starts cold, which changes repair economics)? The RFC is silent, and either answer changes observable behaviour.

Also unstated: **when** candidates are captured across the lineage. Step 3 reads as a single capture, but D8's "verification lineage" has one candidate per repair revision — the rule should be "capture before every `verify` dispatch", said once.

**Recommendation.** Add an explicit amendment clause to the RFC header and D1 ("amends RFC-90 D5 and acceptance criterion 5: the attempt has one *logical* candidate; the verifier and mechanical repair receive fresh materializations; an accepted mechanical repair advances the logical candidate"), mirroring RFC-106 D1's logical-candidate language so the two amendments compose. Decide and state the continuation rule (recommended: continuation survives — it is bound to the logical candidate, not the materialization; RFC-106 already needs the same reading). State the capture point per verify dispatch.

### A4 — The sandbox and cache contract is infeasible as written for real toolchains — **blocker**

Three D3/D8 rules are individually sound and jointly unimplementable for the first-party profiles on the primary deployment (an operator's macOS/Linux desktop running Cargo, Xcode, or Gradle):

- **Egress denial vs dependency fetching.** D3 denies network egress "unless the profile policy grants an exact destination". A Cargo `build`/`test` must reach the registry index and CDN; Gradle and CocoaPods/SwiftPM similarly. "Exact destination" is expressible, but the RFC has no vendoring/offline posture and no statement of what the first-party Omnia policy actually grants.
- **Cold confirmation vs empty cache.** D8 requires every warm passing required profile to be rerun "once with an empty cache". An empty cache means re-fetching the entire dependency closure — under denied egress the cold confirmation *cannot pass*, and with egress granted it re-downloads the world on every successful verification phase. The two decisions collide head-on.
- **Lineage-private cache vs immutable dependency artifacts.** D8 forbids reuse "from a slice or another domain key", which as written also forbids sharing the read-only, content-addressed download cache (crates, wheels, poms) across lineages. Registry artifacts are immutable and verifiable; treating them as poisonable incremental state makes every lineage cold-download for no assurance gain.

Secondary feasibility notes in the same decision: CPU/memory/process-count limits and process-tree reaping are platform-specific (cgroups on Linux; macOS has no supported equivalent of comparable strength), and daemon-mode tools (Gradle daemon, Xcode build service) conflict with process-count limits and lineage-private state unless policies force daemonless operation. Preflight's "sandbox feature" check gives a typed escape (`unavailable`), but if the strict contract preflights false on macOS, Phase A cannot run anywhere the operators actually work.

**Recommendation.** Split D3/D8 inputs into three classes with different rules: (1) **mutable incremental tool state** — lineage-private, cold-confirmed, exactly as D8 says; (2) **immutable dependency artifacts** — a shared read-only content-addressed store, verified on read, exempt from cold-confirmation emptying and permitted under egress denial (pre-populated by a declared, network-granted `fetch` step or preflight); (3) **network egress during checks** — denied, with the fetch step as the only granted path. Add a per-platform statement of the minimum enforceable sandbox set and make the attestation record which sandbox features were actually enforced, so a weaker platform yields an honest weaker attestation rather than a false-uniform claim. Require the first-party Omnia policy (commands, granted destinations, cache roots) as a Phase A deliverable so feasibility is proven on the reference target.

## Phase B blocking issues

### B1 — Binding to parked RFC-100 violates the programme's parking rules — **blocker**

[platform.md](platform.md#parked-programme) parks RFC-100 and states "no active RFC depends on the parked item; no active implementation predeclares its wire shape unless an already-active seam requires an opaque extension point". RFC-97 crosses that line three times:

- The header scopes Phase B to "protected-input closure, and distributed placement".
- D3 specifies worker-side attestation publication "through the authenticated value/coordination transport under the fenced verification operation" — RFC-100 transport mechanics.
- The Phase B implementation requirement "bind placement/publishing under RFC-100" and acceptance criterion 8's "local/remote RFC-100 placement produce the same normalized domain reports" make parked work a **completion condition**: Phase B can never be accepted without implementing a parked RFC.

RFC-96 — the actual Phase B dependency — is explicitly single-node ("Add remote workers" is one of its rejected alternatives).

**Recommendation.** Re-cut Phase B to depend on RFC-96 alone. Move the D3 distributed-publication paragraph and the placement implementation requirement into a clearly marked conditional ("if RFC-100 is reopened, attestation publication rides its coordination plane; nothing here predeclares that wire"). Rewrite acceptance criterion 8's second sentence to cap-one/cap-four equivalence only. The attestation record's context/candidate/policy binding is already placement-neutral, so nothing of substance is lost. Also fix platform.md's Phase B parenthetical ("distributed placement") in the same change.

### B2 — RFC-106 task-owner concepts leak into D7 and acceptance criterion 6 — **blocker**

D11 states correctly that "task owners exist only if RFC-106 is staffed", and RFC-106 is evidence-gated off the active map. Yet:

- D7 scopes mechanical-repair edits to "the slice's Phase A write grant **or one Phase B validated task owner's paths**", and its application step says "in Phase B the engine composes the patch under the unique owner's grant". D7 simultaneously prohibits mechanical repair in Phase B domain contexts ("never available to Phase B frontier or complete domain verification, where no writer is authorized") — and a Phase B *slice attempt* without RFC-106 has exactly Phase A's slice grant. The task-owner branch is unreachable in both phases as scoped.
- Acceptance criterion 6 tests the concept directly: "One exact-preimage machine-applicable group **under one task owner** … Partial, **cross-owner**, protected-path, **unowned** … groups leave the source candidate unchanged". None of `task owner`, `cross-owner`, or `unowned` is testable in Phase A or Phase B without RFC-106.

**Recommendation.** Rewrite D7 and acceptance criterion 6 in slice-grant terms (the group applies within the slice's write grant and touches no protected input; out-of-grant and protected-path groups are ineligible). Move the task-owner refinement to one conditional sentence: "if RFC-106 is staffed, the eligible write scope narrows to one validated task owner's grant" — matching how RFC-96 and RFC-100 reference RFC-106.

### B3 — Phase B has no self-contained acceptance path — major

Phase B is correctly attached to RFC-96, but RFC-96 itself is unimplemented (the workspace has no scheduler, `compose`, or `DomainRound`), and RFC-96 D8's protected-input closure — which D11 and the Phase B implementation requirements consume — depends on per-member protected sets whose declaration surface is the A2 gap. Phase B's spec completeness is therefore hostage to A2's resolution plus RFC-96 Phase B delivery. This is sequencing reality rather than a defect, but the RFC should say which document owns the closure inputs' authoring so the two RFCs do not each assume the other defines it.

**Recommendation.** After resolving A2, add one sentence to D11 stating that the member-admission protected sets Phase B intersects are the Phase A declaration surface's values — one owner, no circular assumption.

## Major gaps (both phases)

### G1 — Mechanical suggestion groups are underspecified

D7 and the Terms section define the group's *identity* (host-attested, atomic, path-bounded, preimage-digested) but not:

- **The edit encoding.** "Each edit names its source preimage digest" — but the content shape (replacement bytes? unified hunk? whole-file result?) is undefined, and it is a wire shape the attestation store, the engine applier, and parsers must share.
- **Which profiles may produce groups.** D2 defines `fmt` as "formatting conformance without source mutation", so groups must come from tools run in a suggest/diff mode — the profile policy needs a declared suggestion channel (e.g. rustfmt diff output, `clippy --fix` dry-run JSON), and the parser contract must cover it.
- **Bounds.** No size cap on a group (edits, bytes, files). Every other bounded surface in this programme has an engine constant.
- **The improvement predicate.** "Keeps the patch only when the originating profile strictly improves and no profile regresses" — D5 defines `regression(candidate, best)` precisely; "strictly improves" is not pinned to it. State both predicates in D5's vocabulary (e.g. improvement = strict lexicographic decrease of the originating profile's severity counts, or strict shrinkage of its blocking fingerprint set; no-regression = D5's predicate false for every required profile).
- **Warm/cold authority in the D7 rerun.** D7 "reruns every required profile" — warm or cold? If the keep decision may be made on warm reports, D8's rule that only cold reports are gate authority must still hold for the verification phase the kept candidate feeds. State the ordering (recommended: warm rerun decides keep/discard; the kept candidate's next verification phase applies D8 cold confirmation as usual).
- **Persistence and ordinals.** RFC-90 D6 names attempt-tree file shapes precisely (`phases/NN-<operation>.yaml`); mechanical repair is "not a target operation", so its record location, whether it occupies a phase ordinal, and whether it emits `slice.build.phase-completed` are all open. D9 gives it "its own completed event" but never names the `EventKind` — a closed taxonomy needs the name (e.g. `slice.build.mechanical-repair-completed`).

**Recommendation.** Give D7 the same specification density RFC-90 D6 has: edit wire shape, producing-profile contract, engine bound on group size, both predicates by reference to D5, warm/cold ordering, attempt-tree location, ordinal treatment, and the named event kind.

### G2 — Warm failures can burn bounded repair budget on stale-cache phantoms

D8 cold-confirms warm **passes** but not warm **failures**. A stale or candidate-poisoned lineage cache can produce a spurious blocking finding, which routes to model repair (three-dispatch budget) or mechanical repair, spending real budget on a defect that does not exist cold. The asymmetry is defensible (failures are cheap to iterate; passes gate lifecycle), but the risk is real for incremental compilers.

**Recommendation.** Either state the asymmetry as an accepted trade-off with the mitigation (a repair round whose findings vanish cold is recoverable evidence, and `unchanged-failure-set` will not fire on a phantom), or add one cheap rule: when a warm required profile fails and the attempt would thereby fail terminally (budget exhausted), rerun cold once before minting the terminal report, so no attempt is *terminated* by cache state.

### G3 — Typed outcomes are demanded but never named

D3 requires "distinct typed outcomes" for sandbox setup failure, tool absence, parser absence, limit exhaustion, cancellation, unsupported platform, and attestation persistence failure; preflight returns "typed `unavailable`"; D1 types incomplete preflight, unresolved/duplicate attestation, context mismatch, and execution failure. The CLI wire contract elsewhere names every kebab-case discriminant (`plan-refinement-required`, `target-phase-source-tool`, `adapter-digest-mismatch`, …). RFC-97 names none.

**Recommendation.** Add the closed discriminant table (e.g. `verification-profile-unavailable`, `verification-sandbox-denied`, `verification-tool-missing`, `verification-parser-missing`, `verification-limit-exhausted`, `verification-cancelled`, `verification-platform-unsupported`, `verification-attestation-mismatch`, `verification-attestation-duplicate`, `verification-attestation-persist-failed`, `verification-profiles-incoherent` for D2's `ci` overlap) and their exit-code mapping, as every implemented RFC in this set does.

### G4 — Report assembly and storage have unstated seams

- **Multi-profile merge.** One verify phase resolves N profile reports into one RFC-90 phase report. The merge rule (concatenate in required-profile order, then apply RFC-90 D2 canonicalization — fingerprint dedupe across profiles included?) is unstated. Cross-profile duplicates are real (`clippy` and `ci` are excluded from coexisting, but `build` and `test` both surface compile errors).
- **Multi-platform mapping.** Metadata declares ordered profiles "per supported platform", and a Vectis project may declare `ios` + `android`. Does one verify phase run the union of both platforms' profile sets, with per-platform attestations? The preflight and telemetry text implies per-platform execution, but the phase-report consequence (one report covering all platforms) is never stated.
- **Attestation durability.** D3 names an "immutable attestation store" and D3/D9 say records "archive with the slice attempt or domain round" — but the live location (host-owned store? attempt tree beside `phases/`?) and its relation to the fact-substrate audit posture are unstated. RFC-90 D6's precision is the model to match.

**Recommendation.** State the merge rule (resolve in required-profile order, project each normalized report's findings into `phase-finding`, then apply RFC-90 D2 canonicalization once over the union); state the multi-platform rule (all declared project platforms' required sets execute in one verify phase, each attestation platform-bound); name the attestation persistence location and its archive path.

### G5 — No live-evaluation acceptance criterion despite the evidence posture

Phase A doubles the cost of every passing verification (cold confirmation) and adds sandbox and snapshot-capture overhead per lineage revision. The platform evidence posture says "prefer a measurement to an assumption whenever the measurement is cheap", and sibling RFCs pin it (RFC-96 AC8 and RFC-106 AC7 require the `omnia-r9k` live grade not to regress). RFC-97's acceptance criteria cover suites and telemetry shape but no live rung and no cost/wall-clock comparison against the RFC-90 baseline.

**Recommendation.** Add an acceptance criterion: the `wasm-omnia-r9k` (or successor) live fixture passes under Phase A host verification with no final-grade regression, and reports the verification wall-clock and cold-confirmation overhead against the model-assisted baseline from the same D9 telemetry.

## Minor issues

- **M1 — No Rejected alternatives section.** Every peer RFC (90, 92, 96, 98, 100, 106) closes with one; RFC-97 makes several contestable calls silently (adapter-relayed handles versus engine-direct profile dispatch; eight names versus target-declared arbitrary profiles; cold-confirmation versus snapshot-keyed caching; one mechanical group versus a fix queue). Recording why the adapter stays in the verify loop at all (deterministic in-component findings; preserving the operation shape) is the most valuable entry.
- **M2 — Silent ignoring conflicts with the fail-closed posture.** D1: "Adapter-supplied tool findings are ignored". Everywhere else, an illegitimate claim is a typed failure (the existing gate rejects `phase-source: tool`). An adapter asserting tool findings it cannot have produced should fail the report gate, not be quietly dropped. Recommend: reject typed (`target-phase-finding-source-tool` or reuse of the existing gate at finding granularity).
- **M3 — `ci` composition is closed to the wrong set.** D2 makes `ci` "mutually exclusive with the seven constituent names". RFC-98 adds `conserve` by patch ownership; whether `conserve` is inside `ci`'s aggregate, and whether the exclusion generalizes ("mutually exclusive with every other profile name"), should be stated so RFC-98 does not have to reopen D2's arithmetic.
- **M4 — platform.md dependency map obscures Phase A independence.** The active map draws `R96 → R97` as the only inbound edge, while both documents say Phase A depends only on implemented RFC-90 and "may proceed beside RFC-88". A staffing reader following the map would wrongly serialize Phase A behind RFC-96. Recommend annotating the map or the RFC-97 entry.
- **M5 — Preflight-to-execution drift is unhandled.** Preflight runs before the first build model call; toolchains can change mid-run (a background `rustup update`). The policy digest in every result surfaces drift after the fact; the RFC should say whether execution re-verifies the pinned toolchain identity per profile run or accepts post-hoc detection.
- **M6 — Execution-assurance locus is ambiguous.** The Terms define both assurances as *projected*, but acceptance criterion 5 says reports "report … `execution-assurance: host-attested`", implying persisted fields. State one rule (recommended: profile reports persist oracle assurance per D4; execution assurance is projected from `phase-source` plus resolution results, never stored — matching the projected-status house rule).
- **M7 — Doc nits.** The D7 mermaid label uses `\n` inside a node (`E{suggestion\ngroup?}`), which renders literally in most mermaid versions — use `<br/>`. Several double blank lines between sections. Flow step 6 references "D7's explicit host-mechanical-repair phase" before D7 is introduced (acceptable RFC style, but a forward link would help).

## Acceptance-criteria coverage notes

Beyond the criteria already flagged (AC6 in B2, AC8 in B1):

- **Missing: retained model-assisted path.** D1 permits non-declaring targets to keep RFC-90 verification; no criterion pins that a non-declaring target's behaviour is byte-unchanged after Phase A lands. Add one — it is the regression guard for every existing target.
- **Missing: `ci` incoherence refusal.** D2's resolution-time failure for `ci` + constituent overlap has no criterion; fold into AC2's typed-failure list.
- **Missing: cache lifecycle.** D8's disposal points (attempt completion, abandonment, policy change, GC) have no criterion; AC7 covers isolation but not disposal.
- **AC5 is doing too much.** It currently carries four distinct assertions (candidate-check labelling, protected upgrade conditions, write denial, cold-confirmation universality). Splitting write-denial enforcement (the A2 mechanism) into its own criterion will make the A2 resolution testable.

## Recommended resolution order

| # | Finding | Severity | Phase | Action owner |
| --- | --- | --- | --- | --- |
| 1 | B1 RFC-100 binding | blocker | B | Edit RFC-97 (header, D3, impl reqs, AC8) + platform.md parenthetical |
| 2 | B2 task-owner leakage | blocker | B | Edit RFC-97 (D7, AC6) |
| 3 | A3 RFC-90 D5/AC5 amendment + continuation rule | blocker | A | Edit RFC-97 (header amendment clause, D1, D7) |
| 4 | A1 wire shape + hard-cut posture | blocker | A | Edit RFC-97 (D1, D3, impl reqs) |
| 5 | A2 protected-input authoring/enforcement | blocker | A | New decision in RFC-97; coordinate with RFC-98 corpus admission |
| 6 | A4 sandbox/cache feasibility | blocker | A | Edit D3/D8 (three input classes, per-platform enforcement floor, first-party Omnia policy deliverable) |
| 7 | G1 mechanical-repair specification | major | A | Expand D7 to RFC-90 D6 density |
| 8 | G3 typed discriminant table | major | A | Add closed table + exit mapping |
| 9 | G4 assembly/storage seams | major | A/B | Edit D1/D3/D5 |
| 10 | G2 warm-failure budget | major | A | One rule or one recorded trade-off in D8 |
| 11 | G5 live-eval criterion | major | A | Add acceptance criterion |
| 12 | B3 closure-input ownership | major | B | One sentence in D11 after A2 resolves |
| 13 | M1–M7 | minor | — | Opportunistic in the same edit |

Resolving items 1–6 makes Phase A implementable and Phase B's specification self-contained on RFC-96. Items 7–11 should land in the same revision so the implementation does not have to invent the missing contracts ad hoc.
