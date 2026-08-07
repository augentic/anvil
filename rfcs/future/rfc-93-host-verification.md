# RFC-93: Host Verification Profiles

> Status: Future draft — deterministic native-verification follow-on, outside the RFC-86…RFC-92 platform-migration series
>
> Owns: closed host verification profiles, native execution and sandbox policy, canonical tool diagnostics, protected-oracle assurance, report comparison primitives, one explicit bounded host-mechanical-repair phase, verification-lineage caches, and per-profile telemetry.
>
> Depends on [RFC-87](../rfc-87-working-trees.md), [RFC-90](../rfc-90-build-verification.md), and [RFC-91](../rfc-91-concurrent-execution.md), plus a standardized WASI execution capability that can enforce this RFC's working-directory, environment, stdio, cancellation, resource, and sandbox contract.
>
> Evidence: [Finitive local-model harness input](../rfc-90-finitive-evidence.md).

## Intent

Replace RFC-90's model-assisted command execution with host-attested native verification while preserving its lifecycle, budgets, and model-repair routing. Add one explicit, separately bounded host-mechanical-repair phase rather than hiding tool-authored writes inside `verify`.

RFC-90 makes operation order, repair budgets, persistence, and lifecycle gates observable, but its `verify` agent still chooses commands, invokes them, interprets output, and reports what happened. A green result is useful model-assisted self-consistency evidence, not deterministic proof.

This RFC closes that trust gap. Target code requests only closed semantic profile names. Deployment policy chooses exact commands and parsers, runs them in a denied-by-default sandbox over an immutable RFC-91 candidate, and returns host-attested normalized findings. The adapter cannot supply argv, silently skip a required profile, relabel model output as tool evidence, or turn unavailability into success.

## Flow and terms

1. Target resolution yields an ordered required profile set for each target/platform pair.
2. Before the first build model call, the engine asks the host verifier to preflight every required profile, toolchain, sandbox feature, protected input, and cache policy. A domain verification preflights against its own operation key before execution. Any missing capability returns typed `unavailable`.
3. RFC-91 composes one immutable logical candidate and materializes a fresh verification workspace.
4. During `target.verify`, deterministic adapter code requests each declared profile by name through the host-verification capability. The host maps the pre-bound `(target identity, platform, profile, policy digest)` to vetted commands and parsers.
5. The host executes each profile in the sandbox, normalizes its output, canonicalizes findings, persists an immutable attested profile report, and returns an opaque attestation handle plus a read-only report copy.
6. The adapter returns a `verification-answer` containing the handles and any deterministic in-component findings. The engine resolves every handle directly through the verifier provider, verifies exact profile/context/candidate/policy coverage, and assembles the RFC-90 verification phase report from the host records rather than trusting adapter-projected tool findings.
7. When a slice-scoped failing profile carries one eligible machine-applicable suggestion group, the engine may run D7's separately recorded host-mechanical-repair phase before spending a model repair.
8. Remaining blocking findings pass through RFC-90 D4's bounded repair-brief projection. RFC-90 and RFC-91 retain repair routing, budgets, candidate capture, terminal-report assembly, and lifecycle authority.

A **verification profile** is a closed semantic gate: `fmt`, `build`, `clippy`, `test`, `doc`, `vet`, `deny`, or `ci`. A profile name never carries argv, shell text, environment overrides, or parser selection.

A **profile policy** is deployment-owned data binding a resolved target identity, platform, and profile to exact commands, toolchain identity, parser, environment allowlist, resource limits, network policy, protected-input handling, and cache policy. Its digest enters every result. The policy never chooses the operator-reviewed protected-input identities.

A **profile report** is the host-attested normalized result of one profile against one candidate snapshot. A **verification assurance** is `candidate | protected | mixed`: candidate checks may consume model-writable inputs; protected checks consume at least one engine-enforced read-only oracle; mixed reports both without collapsing the distinction.

A **verification context** is `slice-attempt | frontier-domain | complete-domain`. A slice context is identified by `(change, target, slice, attempt)`; a domain context by `(change, target, domain, round-kind, operation-key)`. A **verification lineage** is that globally scoped context's ordered candidate snapshots. Slice attempts may contain repair revisions; a domain context has one candidate unless a later operator-authorized round creates a new operation key.

A **profile attestation** is an opaque host-issued handle to an immutable normalized report bound to verifier identity, verification context, candidate snapshot, target identity, platform, profile, policy digest, and protected input/oracle digests. Only the verifier provider resolves it. The adapter may relay a handle but cannot mint or rewrite its record.

A **mechanical suggestion group** is one host-attested atomic list of path-bounded edits. Every edit names its exact source preimage digest; stale or partial application fails before verification.

## Decisions

### D1 — Host verification replaces command judgment, not the phase machine

RFC-90's engine still selects `verify`, routes findings into `repair`, enforces budgets, persists phase reports, and assembles the terminal report. RFC-91 still owns logical candidates, fresh operation workspaces, task-grant routing, and composition.

This RFC changes the `target.verify` result from a target-authored phase report to a `verification-answer` carrying opaque profile-attestation handles plus optional deterministic in-component findings. The adapter contains no model leg and executes no native command itself. The engine resolves the host records and assembles the phase report. A host-only phase reports `phase-source: tool`; a phase combining host records with deterministic in-component findings reports `hybrid`, extending RFC-90's gate without changing the wire enum.

Targets that do not declare host profiles may retain RFC-90 model-assisted verification. A target that declares any required host profile may not fall back silently: incomplete preflight, unresolved/duplicate attestation, context mismatch, or execution failure is typed and fails the attempt. Adapter-supplied tool findings are ignored; only resolved host records contribute `source: tool`. Operator output identifies the verification mode and assurance.

### D2 — Profiles are semantic names; deployment policy owns commands

Target metadata declares the ordered required profile names per supported platform. The host owns the profile registry and exact command mapping. Project files, prompts, models, adapter responses, and source Evidence cannot add flags, substitute binaries, alter environment, select parsers, or redirect the registry.

The initial closed names are:

- `fmt` — formatting conformance without source mutation
- `build` — compile or platform-build viability
- `clippy` — language/platform static analysis
- `test` — target test execution
- `doc` — documentation build and documentation tests
- `vet` — dependency trust policy
- `deny` — dependency and licence policy
- `ci` — the target's complete required local gate

Names are cross-platform semantics, not Cargo commands. A Vectis `build` policy may invoke Xcode or Gradle while an Omnia `build` policy invokes Cargo. Unsupported target/platform/profile tuples fail preflight.

`ci` is an aggregate profile and is mutually exclusive with the seven constituent names in one target/platform requirement set. Metadata that combines them is incoherent and fails resolution rather than running the same gate twice.

### D3 — Native execution is a denied-by-default host capability

The adapter target world imports a host-verification capability that accepts only a declared profile name and opaque candidate-workspace handle. The host takes target identity, platform, verification context, policy digest, and protected-input handles from the engine-prebound dispatch context rather than adapter-supplied values. No string command, arbitrary executable, policy selector, or protected-input selector crosses WIT.

Every execution enforces:

- candidate workspace as the working tree
- an explicit environment allowlist
- no inherited credentials
- denied network egress unless the profile policy grants an exact destination
- bounded wall time, CPU, memory, process count, and stdout/stderr
- cancellation that reaps the complete process tree
- writes limited to declared ephemeral build/cache roots
- read-only protected inputs

Sandbox setup failure, tool absence, parser absence, limit exhaustion, cancellation, unsupported platform, and attestation persistence failure are distinct typed outcomes. None deserialize as a passing report. A successful call stores its normalized report in the host verifier's immutable attestation store before returning its opaque handle.

Under RFC-92 placement, the worker-side verifier provider publishes that digest-bound record through the authenticated value/coordination transport under the fenced verification operation before the handle becomes visible to the engine. The receiving provider verifies producer identity, fence, context, and digest before resolution. Adapter code receives neither publication credentials nor a writable attestation channel. Attestation records archive with the slice attempt or domain round that consumes them.

### D4 — Candidate checks and protected oracles remain distinguishable

RFC-91's operator-reviewed `protected-verification-inputs[]` is the authority for in-tree baseline tests and fixtures that workers cannot change. Digest-bound external material must appear in `protected-oracles[]` before a profile may mount it read-only outside the candidate tree. Deployment policy controls how an authorized input or oracle is mounted and consumed; it cannot introduce another identity.

Candidate-authored tests remain useful: they demonstrate that the candidate passes its own checks. They do not become an independent oracle merely because the host ran them. Every profile report records:

- `assurance: candidate | protected | mixed`
- candidate snapshot id
- protected input/oracle digests bound by the executed profile policy
- profile-policy digest

The host, not the model or adapter, derives assurance from the executed profile policy's closed input contract and the digest-matched inputs mounted for that run. A protected declaration alone does not upgrade the report; only a host policy that binds the protected input into the executed command may attest `protected` or `mixed`.

### D5 — Normalization is canonical before fingerprinting

Each profile policy names one deterministic parser. Structured tool output is preferred. Raw output is a bounded fallback represented by a digest plus a short, secret-filtered tail.

Normalization removes volatile data that does not identify a defect, including durations, process ids, thread ids, temporary workspace roots, and nondeterministic test ordering. It converts paths to candidate-relative `/` form, applies profile-defined cascade suppression, computes fingerprints, deduplicates, and sorts by RFC-90's closed finding key.

The complete normalized profile report remains gate authority. RFC-90 D4 independently projects its bounded repair brief.

Two pure host-computable predicates are part of the report contract:

- `unchanged-failure-set(a, b)` — equality of blocking fingerprint sets for consecutive candidate revisions in one lineage
- `regression(candidate, best)` — lexicographic worsening of `(critical, important, suggestion, optional)` counts after profile-defined normalization

The first cut records these predicates but does not alter RFC-90's repair budgets. A later evidence-backed policy may stop on repeat or restore a high-water candidate without changing the report wire shape.

### D6 — Findings shaping has a neutral and a profile-specific layer

Profile normalization may suppress known cascades and assign severities because it understands the tool's structured output. It may not discard an independent blocking root finding to fit a model context.

After normalization, RFC-90 D2 verifies fingerprints and canonicalizes the complete phase report; D4 filters its blocking findings to the 16-finding repair cap. The adapter receives that brief plus the complete report digest. Terminal success still depends on complete subsequent verification, never on the selected subset.

### D7 — Mechanical repair is atomic, bounded, and verified

Host mechanical repair is an explicit engine-selected phase between one failed slice verification and RFC-90's model repair. It is not a target operation and is never available to RFC-91 frontier or complete domain verification, where no writer is authorized.

One failed slice verification may offer at most one group. Eligible edits must come from one attested profile report, apply atomically against that exact candidate snapshot, remain inside the reviewed slice ownership envelope, intersect no protected input, and resolve to exactly one RFC-91 task owner. A group spanning owners or touching an unowned path is not offered. When several groups are eligible, required-profile order wins, then canonical suggestion-group digest; the engine selects exactly the first.

The engine then:

1. prepares a fresh workspace from the source candidate;
2. applies every exact-preimage edit in the group or none;
3. captures a tentative candidate snapshot before running any profile;
4. runs the complete required profile set against that snapshot;
5. accepts the patch only when the originating profile strictly improves and no required profile regresses against the source candidate;
6. composes the patch under the unique owner's grant and clears that owner's continuation;
7. resolves the tentative run's attestations and assembles the next ordinal RFC-90 verification phase report.

A stale, partial, unchanged, locally improved but globally regressing, or otherwise failed group discards the tentative snapshot and leaves the source candidate and original failed verification report current; RFC-90 D4 projects the model-repair brief from that original report. On acceptance, the tentative candidate becomes current and its engine-assembled phase report becomes the latest verification: a clean report advances to `review`, while a blocking report supplies D4's next model-repair brief. Mechanical repair consumes no model-repair dispatch. An accepted candidate's complete-profile result cannot offer another group before the next model repair dispatch; this hard bound prevents an implicit second repair loop. The engine persists the replacement verification report plus a `host-mechanical-repair` phase record containing owner, patch, source/tentative snapshot ids, before/after profile-report digests, and decision.

### D8 — Incremental caches are private to one verification lineage

A strict snapshot-keyed cache would make every slice repair revision cold. The host may therefore preserve incremental tool state across candidate snapshots within one slice-attempt verification lineage. A frontier or complete domain operation has a singleton lineage and never reuses cache state from a slice or another domain key.

The cache key contains:

- verification-context kind and id
- resolved target name and version
- platform
- profile-policy digest
- toolchain identity
- sandbox/environment-policy digest

RFC-91 still materializes a fresh workspace for every operation. The host mounts or copies the private cache into that execution without exposing it as a shared writable product tree. Every profile still executes its command and parser; cached verdicts or reports are forbidden. Cache contents never enter snapshot capture, report fingerprints, lifecycle authority, or another lineage.

Every warm passing required profile is provisional. Before it may contribute to a successful verification phase, the host reruns it once with an empty cache and uses the cold report as authority. This preserves warm failure/repair iteration without letting stale or candidate-poisoned incremental state certify any lifecycle gate. Attempt completion, abandonment, policy change, or verification-lineage garbage collection makes the cache disposable.

### D9 — Telemetry is raw evidence, not authority

RFC-90's phase-completed timing remains the slice-operation envelope. This RFC additionally emits one `target.verify.profile-completed` event per profile with:

- verification-context kind and id
- phase ordinal when slice-scoped
- profile and platform
- run kind (`primary | cold-confirmation`)
- candidate snapshot id
- policy and report digests
- assurance
- elapsed milliseconds
- finding counts by severity
- cache disposition (`disabled | cold | warm`)
- mechanical-repair disposition (`not-offered | rejected | accepted`)

Events contain raw observations. Distribution, scheduler, and model-economics views are projections over retained events; no metric changes lifecycle state or report success.

The host-mechanical-repair phase emits its own completed event with source/tentative snapshot ids and `rejected | accepted`; profile events do not imply write authority.

### D10 — Verification reports are reusable, not an RFC-18 reward contract

Canonical reports and telemetry are suitable inputs to evaluation, synthetic filtering, or model-selection experiments. RFC-18 may project its own score from them.

This RFC does not promise that severity-count ordering is a complete code-quality reward. RFC-18 also needs traceability, guardrail, layout, configuration, and migration dimensions outside verification. Training rankability cannot weaken a workflow gate or stabilize a field that verification itself does not need.

### D11 — Slice and domain verification share profiles, not repair authority

RFC-91's slice, frontier-domain, and complete-domain calls use the same target/platform profile set, host policy, normalization, and attestation gate. Their verification context and candidate snapshot enter every attestation and telemetry event.

Only a slice attempt has task owners, RFC-90 model-repair budgets, and D7 host-mechanical-repair authority. A blocking frontier report fails the frozen wave; a blocking complete report preserves accepted waves and blocks dependants and drain, exactly as RFC-91 specifies. A domain verifier never synthesizes a writer, reuses a slice continuation, opens a repair budget, or carries cache state from its child slices.

RFC-91 D11 derives and persists the domain's canonical protected-input closure by intersecting every contributing descendant's reviewed sets and subtracting touched paths. Its digest enters the domain operation key, verifier context, and attestation. The host may attest domain-level `protected` or `mixed` only from that closure; a path protected by only one child contributes no domain assurance.

## Implementation requirements

- Extend target metadata with ordered platform-specific required profile names, change host-backed `target.verify` to return `verification-answer`, and add the host-verification import to the target world. No argv-, policy-, target-, or protected-input-shaped selector enters that import.
- Add a deployment-neutral verifier capability to the engine/provider seam with pre-bound execution context, immutable attestation storage, opaque handles, and direct engine resolution. Native tests use a scripted verifier with the same typed outcomes.
- Ship the first-party profile-policy registry, exact command mappings, parsers, and sandbox binding in Emery's deployment-provider layer. Target adapters declare semantic profile names and deterministic in-component checks only; engine crates carry no concrete adapter branch.
- Preflight all required profiles before the first slice build model call and before each domain operation. Reject missing tools, policies, parsers, sandbox features, reviewed protected inputs, and unsupported tuples as typed `unavailable`.
- Add host-attested profile reports, assurance, policy/context/candidate binding, canonical normalization, report comparison predicates, and engine assembly from the exact resolved attestation set. Ignore adapter-authored tool findings.
- Extend RFC-91 candidate materialization with read-only protected-input handles and private verification-lineage caches without sharing product workspaces. Cold-confirm every warm passing required profile.
- Implement D7 as an explicit slice-only host-mechanical-repair phase through RFC-87 prepare/capture/discard and RFC-91 unique-owner composition. Domain verification has no host or model repair writer.
- Emit D9's context-aware per-profile and mechanical-phase events while keeping timing and cache data outside report fingerprints and lifecycle projection.

## Acceptance criteria

1. A host-backed Omnia verification executes the required profile set without any model call or adapter-supplied argv. The engine resolves the opaque handles directly, assembles the phase report, and reports `phase-source: tool`.
2. Missing tools, unsupported platforms, sandbox failures, timeouts, cancellation, parser failure, forged/duplicate/tampered/mismatched attestations, and incomplete profile sets fail closed with distinct typed outcomes before false success; slice preflight failures occur before build model spend.
3. Equivalent structured output with different durations, process/thread ids, temporary roots, or result ordering produces byte-identical normalized reports and fingerprints.
4. Consecutive candidate reports compute stable unchanged-set and severity-regression predicates without changing RFC-90's repair budget.
5. Candidate-owned tests report `assurance: candidate`. A host policy that binds an operator-reviewed protected in-tree or external oracle into the executed command reports `protected` or `mixed`; no worker can modify its protected inputs and a declaration alone cannot upgrade assurance. No warm passing required profile, of any assurance, can gate without cold confirmation.
6. One exact-preimage machine-applicable group under one task owner is captured as a tentative snapshot and kept only after strict originating-profile improvement plus no regression across the complete profile set. Partial, cross-owner, protected-path, unowned, unchanged, and globally regressing groups leave the source candidate unchanged and do not consume a model repair. Domain verification never offers the phase.
7. Successive slice repair candidates in one verification lineage reuse a warm private cache. Another context, target identity, platform, toolchain, or policy cannot observe it; domain operations inherit no slice cache; cache contents never affect captured snapshots or provide a cached verdict.
8. Every slice and domain profile execution emits the closed D9 telemetry event with its verification context. Cap-one/four RFC-91 execution and local/remote RFC-92 placement produce the same normalized reports for the same scripted tool outputs.
9. Native and Wasm integration suites cover direct attestation resolution, profile-set completeness, command-injection refusal, environment and egress denial, resource limits, process-tree cancellation, protected-input write denial, canonical parsing, raw fallback bounds, unique-owner mechanical rollback, domain no-repair behavior, cold confirmation for every assurance class, cache isolation, and telemetry.

## Rejected alternatives

- **Let the model supply commands or flags** — recreates the trust gap this RFC exists to close.
- **Store command profiles in project configuration** — lets candidate-controlled state redefine its own judge and makes policy non-portable across deployments.
- **Call candidate-owned tests an oracle** — conflates self-consistency with independent acceptance evidence.
- **Key incremental caches only by snapshot id** — forces cold verification after every repair even though all revisions remain inside one disposable attempt.
- **Share caches across verification contexts or unrelated lineages** — crosses the isolation boundary and lets untrusted build state influence unrelated candidates.
- **Trust an adapter-assembled tool report** — allows omission, policy substitution, or finding mutation between host execution and the lifecycle gate; the engine assembles only from directly resolved attestations.
- **Apply machine suggestions individually or keep them without reverification** — measured partial-fix failures can worsen the candidate; atomic apply plus strict improvement is the minimum safe rung.
- **Make repeat/regression predicates lifecycle authority immediately** — canonical data should land before a stopping policy; RFC-90's fixed budgets remain the conservative first policy.
- **Treat verification output as the complete SLM reward** — rewards need product-quality dimensions outside native tool verification.
