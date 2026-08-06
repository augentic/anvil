# RFC-90: Build Verification

> Status: Draft — step 5 of the platform-migration series, scale track ([platform.md](platform.md))
>
> Owns: the engine-owned build phase machine, separate `target.build` / `target.repair` / `target.verify` / `target.review` WIT operations, bounded verification and repair rounds, typed intermediate reports, and removal of repair-loop control from adapter prose.
>
> Builds on completed [RFC-87](rfc-87-working-trees.md). The observable sequential gate becomes [RFC-91](rfc-91-concurrent-execution.md)'s worker and conflict-domain convergence loop; [RFC-92](rfc-92-node-sync.md) transports its phase records without redefining it. Deterministic native-tool verification is deferred until WASI exposes an execution API suitable for host toolchains.



## Intent

Move build verification and repair control out of opaque agent prose and into the workflow engine before making builds concurrent.

### Today

The engine dispatches one `target.build` call for a slice. Inside that call the Omnia target runs preparation, generation, optional replay, verification and repair, standards review, and review remediation. Cargo commands, retry counts, failure routing, and stopping conditions live in prompt text. The engine observes only the final report.

That shape hides the most important control loop from lifecycle policy and makes RFC-91 concurrency unsafe: splitting writers before replacing the loop would remove the only mechanism that currently returns failed checks to the code writer.

The engine guest also cannot execute the target project's native toolchain. Its `workflow` world imports `source`, `target`, and `workspaces`; neither Emery nor Omnia currently supplies native process execution. The `wasi-model` `verify` grant authorizes model tooling but is not an execution capability.

### Goal

The engine owns a closed sequence of **build phases** over one disposable RFC-87 workspace:

```text
build → verify ⇄ repair → review ⇄ repair → complete
```

The target adapter still owns specialist behavior: generation prompts, platform-specific checks, repair instructions, and engineering-standards review. Each adapter invocation is exactly one WIT operation — `build`, `verify`, `repair`, or `review` — and returns a typed **phase report**. The engine persists the report, decides the next operation, enforces repair budgets, and assembles the final build report.

Verification in this RFC remains **model-assisted**. During a `verify` call, the target's agent runs its declared commands inside the lent workspace and returns findings. Moving orchestration into the engine makes the loop bounded and observable; it does not make command selection, execution, or findings deterministic or trustworthy.

## Flow and terms

1. The engine prepares one writable RFC-87 **build workspace** from the recorded target base and one attempt-local **artifact stage** seeded from the slice tree. Product code writes go to the workspace; every target-authored slice-artifact write goes to the stage.
2. It dispatches `build` once. Adapter-internal preparation and target-specific writer ordering may remain inside that call, but generation may not run a verification or repair loop.
3. It dispatches `verify` once. The adapter runs one check pass and returns one phase report.
4. Blocking verification findings cause one `repair` dispatch with `origin: verification` carrying those exact findings, followed by another `verify`, while the verification-repair budget remains.
5. A passing verification dispatches `review` once. Blocking review findings route through one `repair` dispatch with `origin: review`, then `verify`, then `review`, while the review-remediation budget remains. If the post-repair verification fails, it enters the ordinary verification-repair loop and consumes the same verification budget as any other failed verification.
6. The engine assembles `build/report.yaml` by the fixed projection in D2, applies the existing blocking-finding and output-existence gates, captures the workspace result, validates the staged artifact diff, atomically promotes that diff, and completes the `built` transition.
7. Budget exhaustion returns a typed failed build report with the unresolved findings. Product-code and artifact workspaces are discarded without changing authoritative target code, slice intent, target-owned slice outputs, or lifecycle state. Engine-owned attempt reports remain as audit evidence under `build/attempts/`.

A **build phase** is one engine-selected target operation: `build`, `verify`, `repair`, or `review`. A **phase report** is the typed result of exactly one operation. A **repair round** is one repair dispatch followed by the verification and, when applicable, review needed to decide whether that repair succeeded. A **terminal report** is the engine-authored `BuildReport` projected from the build phase plus the latest verification and review reports; superseded failed rounds remain only in the attempt record.

## Worked example

Suppose an Omnia-bound build generates a payment handler. The engine dispatches `build`, then `verify`. The verification agent runs the adapter's existing Rust checks and returns a blocking finding for an avoidable clone in `crates/payments/src/handler.rs`.

The engine persists that verification report and dispatches `repair` with the exact finding and the generation continuation. The adapter performs one repair pass and returns. The engine—not prompt prose—then dispatches `verify` again, decrements the verification-repair budget, and records the result.

After verification passes, the engine dispatches standards `review`. If review reports a blocking API-design finding, the engine permits the configured review-remediation round, verifies the repaired tree again, and reruns review. If the finding remains, the final build report fails with that finding. Every phase is visible even though the agent still owns the Cargo shell calls.

## Decisions



### D1 — The engine owns the build phase machine

Operation order, repair routing, budgets, terminal success, and terminal failure are deterministic engine policy. A target adapter cannot select its next operation, reset a budget, silently retry, or claim completion while a blocking phase report remains.

The initial policy permits at most three verification repair dispatches and one review remediation dispatch. These are engine constants, not fields supplied by a model or adapter.

The complete transition algorithm is:

```text
verification-repairs = 0
review-remediations = 0

build-report = dispatch build
if build-report has blocking findings:
    fail with build-report findings

verification:
    verify-report = dispatch verify
    if verify-report has blocking findings:
        if verification-repairs == 3:
            fail with verify-report findings
        dispatch repair(origin: verification, findings: verify-report blocking findings)
        verification-repairs += 1
        goto verification

review:
    review-report = dispatch review
    if review-report has no blocking findings:
        succeed
    if review-remediations == 1:
        fail with review-report findings
    dispatch repair(origin: review, findings: review-report blocking findings)
    review-remediations += 1
    goto verification
```

A returned `repair` report never selects the next operation: after any completed or non-applicable repair, the engine dispatches `verify`. A repair dispatch error terminates the attempt. Blocking findings on a repair report are persisted as evidence but are superseded for terminal routing by the required verification that follows. A non-applicable `verify` or `review` report is a passing report with no blocking findings; a non-applicable repair still consumes the budget for the origin that caused it.

Only a returned phase report advances the machine. A WIT error, invalid phase report, continuation violation, staged-artifact scope violation, or engine gate failure terminates the attempt and produces a failed terminal report carrying an engine-authored blocking diagnostic. The fourth verification repair and second review remediation are never dispatched.

### D2 — Separate WIT operations; no `build-phase` enum

The target axis grows from three operations to six: `guidance`, `build`, `repair`, `verify`, `review`, and `merge`. Generation, repair, verification, and standards review are distinct methods. There is no `build-phase` enum and no phased overload of a single `build` call.

`wit/emery.wit` adds the following closed shared vocabulary:

- `phase-outcome = completed | not-applicable`. There is no adapter-selected `success | failure`: blocking findings and dispatch errors determine failure.
- `phase-source = deterministic | model-assisted | hybrid | tool`. This required report-level field states how the pass was produced even when `findings` is empty. `tool` is reserved on the wire but rejected by RFC-90's engine gate until a trusted host-tool execution seam exists.
- `repair-origin = verification | review`. It tells `repair` which engine gate supplied the findings without allowing the adapter to select the next phase.
- `phase-root = workspace | artifacts` and `phase-write { root, path }`. Paths are relative to the writable product workspace or writable artifact stage; absolute paths and `..` are invalid. `written` is audit evidence, while RFC-87 capture and the staged-artifact diff remain the authoritative write records.
- `writable-artifact-kind = file | tree` and `writable-artifact { path, kind }`. A `file` grant names exactly one slice-relative file; a `tree` grant names that directory and its descendants. Paths use `/`, must be relative, and admit no glob or `..` grammar.
- `phase-location { path, line?, column?, end-line?, end-column? }`, with project-relative `/`-separated paths. Product paths resolve in the candidate workspace; change-artifact paths retain their project-relative `.emery/...` form.
- `phase-finding`, an isomorphic WIT projection of the shared `Diagnostic` fields `id`, `rule-id?`, `related-rule-ids[]`, `title`, `severity`, `source`, `kind`, `artifact`, `location?`, the closed `snippet | digest | structured` evidence union, `impact`, `remediation`, `confidence?`, and `fingerprint`. The engine stamps `target-adapter`, `slice`, and change identity, verifies or recomputes the fingerprint, and renumbers report-local ids. No title/impact/remediation folding is permitted.
- `phase-report { outcome, source, findings, outputs, ui-surface, written, next-continuation }`. `outputs` and `ui-surface` retain their current typed shapes. `next-continuation` is `option<list<u8>>`: `none` preserves the current adapter-opaque continuation, `some([])` clears it, and a non-empty value replaces it. The engine rejects a continuation larger than 1 MiB before persistence.

`build` alone owns output declaration and UI-surface classification: its `outputs` and `ui-surface` become the candidate values for the final report. `repair`, `verify`, and `review` must return empty `outputs` and no `ui-surface`; changing code beneath a declared output does not change the declaration. A `not-applicable` report must carry no blocking findings and no writes. These coherence rules are engine gates, not prompt conventions.

`phase-source` is an assurance claim, not an execution selector. `deterministic` means no model or external tool contributed to the phase result; `model-assisted` means the result came from model judgment, including an agent invoking and interpreting native commands; `hybrid` means deterministic in-guest checks and model judgment both contributed. The envelope must cover every finding source: a deterministic report cannot carry model-assisted findings, and a report containing both deterministic and model-assisted findings must be hybrid. The engine rejects `tool` in this RFC rather than silently relabelling it. Text and JSON operator output name the terminal verification report's source even when it has no findings, so a clean pass is never presented as stronger evidence than it is.

The engine assembles the terminal `BuildReport` deterministically:

1. `status` is `success` only after a non-blocking build, latest verification, and latest review plus all engine gates pass; every other terminal path is `failure`.
2. `outputs` and `ui-surface` come only from the build report.
3. `findings` are the fingerprint-deduplicated, stably renumbered union of the build report, the latest verification report, the latest review report when one ran, and engine-authored terminal gate findings. Findings from superseded verify/review rounds and repair reports remain in phase records but do not leak into the terminal report.
4. On verification-budget exhaustion, the latest verification's blocking findings are terminal. On review-budget exhaustion, the latest review's blocking findings are terminal. On a dispatch or structural gate failure, the engine-authored diagnostic is terminal.

There is no `build-scope` variant: every operation names a slice except `verify`, which receives only the candidate workspace. Domain identity for RFC-91 convergence stays engine-side; the adapter runs the same check pass against whatever workspace the engine lends.

Conceptually:

```wit
/// Generation only: preparation, writers, capture replay.
/// Must not verify, repair, or run standards remediation.
build: async func(
  id: adapter-id,
  slice: string,
  inputs: list<input>,
  context: build-context,
  workspace: workspace,
) -> result<phase-report, error>;

/// One model-assisted check pass on the lent workspace.
verify: async func(
  id: adapter-id,
  workspace: workspace,
) -> result<phase-report, error>;

/// One findings-directed repair pass.
repair: async func(
  id: adapter-id,
  slice: string,
  origin: repair-origin,
  findings: list<phase-finding>,
  continuation: option<list<u8>>,
  workspace: workspace,
) -> result<phase-report, error>;

/// One engineering-standards review pass.
review: async func(
  id: adapter-id,
  slice: string,
  continuation: option<list<u8>>,
  workspace: workspace,
) -> result<phase-report, error>;
```

Continuation state is an adapter-opaque byte payload that may represent several writer or reviewer sessions. The engine persists and echoes the current value only to the same resolved target identity, attempt, and build workspace. `verify` cannot mutate it; `build`, `repair`, and `review` may return a replacement. It never crosses attempts, survives no workspace loss, and is never interpreted or treated as lifecycle authority.

Separating `verify` and `review` keeps mechanical checks and standards judgment as different product surfaces: different prompts, budgets, and (later) execution trust. RFC-91 reuses the same `verify` against a composed candidate workspace; domain id never crosses this seam.

### D3 — Every dispatch performs exactly one operation

`build` may perform target-specific preparation, scaffolding, writer ordering, and capture replay needed to produce the candidate. It must not verify, repair, or run standards remediation.

`verify` performs one check pass and reports its findings. Native command execution remains model-assisted; an adapter may also include deterministic in-guest validators and report the phase as `hybrid`, or `deterministic` when no model leg ran. `repair` receives the engine-selected unresolved findings plus their verification/review origin and performs one repair pass. `review` performs one standards review pass and reports its findings. Prompts may describe how to perform their single operation; they may not contain retry loops or choose another operation.

### D4 — Findings are the repair currency

Phase findings use the shared diagnostic shape and artifact-relative locations. The engine routes the exact unresolved finding set into `repair`; prose does not reconstruct failures from a transcript or decide which failures count.

Because verification remains model-assisted, its findings use `source: model-assisted` or `source: hybrid` as appropriate. This RFC does not label them `source: tool`: the engine does not receive and parse trusted tool output.

### D5 — One workspace spans the complete loop

All phases for one build run against the same disposable RFC-87 product workspace and the same attempt-local artifact stage. Generation, verification, repair, and review therefore observe one candidate code tree and one candidate slice-artifact tree.

The target `workspace` record gains `artifact-stage { id, root }`, an agent-visible writable mirror rooted at the candidate slice tree alongside the existing read-only project-wide `artifacts` root. Build inputs beneath the active slice resolve against the stage, so later phases read prior staged writes; non-slice project context continues to resolve against `artifacts`. Target metadata gains typed `writable-artifacts[]` grants using D2's exact `file | tree` grammar. Omnia declares `tasks.md`; Vectis declares `tasks.md`, `composition.yaml`, and its build bookkeeping subtree; Contracts declares `tasks.md` and `contracts/`. Target operations must write product code only under `workspace.root` and target-owned slice artifacts only under `workspace.artifact-stage.root`. The authoritative slice tree remains read-only to every target operation.

The engine seeds the stage before `build`, derives its actual diff after every mutating phase, and rejects a change outside `writable-artifacts[]` even when the phase omits it from `written`. Later phases read the staged candidate, so review and verification observe prior target-authored artifact changes without making them authoritative.

The engine captures product code and promotes staged artifacts only after the terminal phase machine and final report gates pass. Artifact promotion is an engine-owned recoverable transaction: validate the complete diff first, prepare every replacement, commit the set all-or-none, and roll back before returning any promotion error. The `refined → built` transition occurs only after code capture and artifact promotion both succeed. A dispatch error, budget exhaustion, blocking final report, scope violation, promotion failure, or cooperative cancellation discards both writable trees and leaves authoritative target code, target-owned slice artifacts, and lifecycle unchanged. Process death may leave local disposable directories for RFC-87 garbage collection; it cannot expose their contents as authoritative state.

Engine-owned request, phase, and terminal reports are audit records, not target-authored slice intent. They may persist for a failed attempt without violating this rule.

### D6 — Phase reports are durable and the final report stays authoritative

Each invocation receives the next monotonic attempt id and writes under `.emery/slices/<slice>/build/attempts/<attempt>/`. The engine copies the immutable `build/request.yaml` into that attempt, writes every returned report as `phases/<ordinal>-<operation>.yaml`, and emits a `slice.build.phase-completed` event naming the attempt, ordinal, operation, source, and report digest. A retry creates a new attempt; it never appends to, clears, or reuses a prior attempt or continuation.

The engine writes the attempt's terminal report beside its phases and projects that same body to the canonical `build/report.yaml`. A new terminal attempt atomically replaces the canonical projection; immutable attempt records retain the complete history. The engine alone assembles both copies. Later lifecycle gates consume the canonical report, while RFC-91 domain convergence may cite immutable phase or attempt-report digests.

Attempt ids are zero-padded ordinals allocated by atomically creating the next absent attempt directory; an existing id is never reused, even when its attempt has no terminal report. Every request, phase report, and terminal report write is atomic. On re-entry, an attempt without a terminal report is **abandoned**: its returned phase reports remain immutable evidence, its continuation is never loaded, and the engine starts the next attempt from the recorded base in fresh product and artifact workspaces. It does not try to infer whether an interrupted adapter call wrote unreported files. The canonical `build/report.yaml` remains the projection of the latest terminal attempt and is unchanged by an abandoned attempt.

An orderly failure writes the failed terminal report before discarding its writable trees. Abrupt process loss may leave an unterminated attempt and disposable workspace directories; the next invocation applies the abandonment rule and RFC-87 garbage collection removes the directories. Attempt records archive with the slice and follow the existing archive-retention policy; build re-entry never prunes them in place.

Phase reports are evidence of orchestration, not new lifecycle states. The slice still transitions once from `refined` to `built`.

### D7 — Existing target behavior moves without changing ownership

Omnia splits its current preparation/generation/replay work into `build`, its Cargo check pass into `verify`, one findings-directed writer pass into `repair`, and one engineering-standards pass into `review`. Vectis and contracts implement the same four operations with their own specialist behavior; an inapplicable operation returns a typed non-blocking report rather than inventing adapter-specific operation names.

The adapter repository continues to own prompts, engineering standards, target composition, and report content. Emery owns the operation vocabulary, loop, persistence, budgets, and final lifecycle gate.

### D8 — Deterministic native verification is deferred

Closed command profiles, native process execution, sandbox and resource policy, trusted tool-output parsing, cache isolation across policies, execution grants, and typed toolchain unavailability are not part of this RFC.

That work depends on [`wasi:exec`](https://github.com/WebAssembly/WASI/issues/899) or another standardized WASI capability that can run native toolchains with a working directory, controlled environment, bounded stdio, cancellation, resource limits, and enforceable sandbox policy. The current `wasi:exec` discussion does not yet guarantee those semantics.

When that capability exists, a follow-up replaces the model-assisted implementation of `verify`; the engine phase machine, repair routing, separate WIT operations, workspace bracket, and final report assembly remain unchanged.

## Implementation requirements

- Extend `wit/emery.wit`, target metadata, the adapter SDK `Target` trait and export macro, the guest provider, native provider, mock catalog, and wire DTOs with `repair`, `verify`, and `review`, the closed D2 phase vocabulary, `writable-artifacts[]`, and `workspace.artifact-stage`.
- Replace the single target-build dispatch in `slice::orchestrate` with D1's exact engine-owned phase machine, including the shared verification-repair counter, review-remediation counter, repair-origin routing, continuation replacement rules, and deterministic terminal-report projection.
- Persist immutable attempt-scoped phase reports and continuations, abandon rather than resume unterminated attempts, add the enriched `slice.build.phase-completed` event, and keep canonical `build/report.yaml` as the latest terminal engine-assembled authority.
- Keep one RFC-87 product workspace and one artifact stage for the complete loop; capture and transactionally promote only on terminal success, and discard both on every failure or cooperative cancellation path.
- Split Omnia, Vectis, and contracts target implementations across `build` / `repair` / `verify` / `review` without moving specialist prompts or engineering standards into Emery.
- Move Omnia's `tasks.md`, Vectis's `tasks.md` / `composition.yaml` / build bookkeeping, and Contracts' `tasks.md` / `contracts/` writes onto the artifact stage; reject undeclared target-authored slice writes.
- Delete verification, retry, writer re-entry, and review-remediation loops from adapter prose. An operation prompt may perform one pass only.
- Preserve the existing final blocking-finding, declared-output, patch-capture, and `refined → built` gates.
- Require and validate report-level `phase-source`, include the terminal verification source in text and JSON operator output even on a clean pass, and do not claim deterministic, sandboxed, or trusted native execution.



## Acceptance criteria

1. One build produces an ordered persisted sequence of engine-selected `build`, `verify`, optional `repair`, and `review` phase reports before one final build report.
2. No target prompt contains a verification-repair or review-remediation retry loop; injected failures return to the engine after one operation.
3. Verification failures route as full, unfolded typed findings to `repair(origin: verification)`, then back through `verify`; review findings route to `repair(origin: review)`, then through `verify → review`. A failed verification after review repair consumes the shared verification budget. The fourth verification repair and second review remediation are never dispatched.
4. The adapter cannot select the next operation, alter the attempt count, reset a budget, or write the terminal build report.
5. The same RFC-87 workspace id and artifact-stage id are used throughout one attempt. Failed, exhausted, cancelled, out-of-scope, and promotion-failed builds leave authoritative target code, target-owned slice artifacts, and lifecycle unchanged while retaining engine-owned attempt reports.
6. The phase-report gate rejects unknown outcome/source values, `tool` in this RFC, a source inconsistent with its findings, folded findings, malformed or escaping locations/writes, oversized continuations, non-build output/UI declarations, and a non-applicable report with blocking findings or writes. Clean verification output still identifies its report-level source.
7. Terminal report assembly is byte-stable: only the build report supplies outputs/UI surface; only the latest verify/review rounds contribute gate findings; superseded and repair findings remain attempt-local; fingerprints deduplicate and ids renumber deterministically.
8. Omnia/Rust preserves its current generation, replay, Cargo-check, repair, standards-review, output, and finding behavior through the four operations.
9. Vectis and contracts implement every operation, using typed non-applicability where an operation has no target-specific work, and all three first-party targets declare and obey their writable artifact scopes.
10. Native and Wasm integration suites cover success, verification repair, review remediation, post-review verification failure, both budget exhaustions, both repair origins, continuation preserve/replace/clear and size rejection, attempt isolation, interrupted-attempt abandonment, canonical-report preservation across interruption, source-assurance validation, phase-report persistence, artifact promotion/rollback, workspace discard, and final report assembly.
11. `wasm-omnia-r9k` records operation count, model calls, repair routing, and wall time; every formerly hidden retry is visible as an engine phase.
12. Documentation describes this gate as model-assisted and identifies deterministic native verification as deferred on a suitable WASI execution capability.



## Rejected alternatives

- **Wait for** `wasi:exec` **before exposing the loop** — leaves retry authority hidden in prose and blocks RFC-91's observable worker orchestration on an unsettled external API.
- **Move the loop from prose into adapter Rust** — improves prompt size but leaves operation order, budgets, and retries adapter-owned and invisible to the workflow engine.
- **Have the engine repeat today's monolithic** `target.build` — replays preparation, generation, and review against a mutating workspace without identifying which action is intended.
- **One** `build` **method with a** `build-phase` **enum** — collapses generation, repair, verification, and standards review behind a stage parameter. Rejected so `verify` and `review` stay distinct product surfaces (different prompts, budgets, and later execution trust) and RFC-91 can call `verify` alone without a phase discriminator.
- **A** `build-scope` **(**`slice` **|** `domain`**) parameter on** `verify` — adapter check behavior does not branch on scope; domain identity belongs in engine-side round records. Deferred until something in the adapter needs it.
- **Let adapters define arbitrary operation names or next actions** — fragments workflow policy and allows an adapter response to control orchestration.
- **Call model-assisted shell execution deterministic verification** — overstates the trust boundary; the agent still controls command invocation, output interpretation, and reporting.

