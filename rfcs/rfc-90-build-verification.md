# RFC-90: Build Verification

> Status: Draft — step 5 of the platform-migration series, scale track ([platform.md](platform.md))
>
> Owns: the engine-owned build phase machine, separate `target.build` / `target.repair` / `target.verify` / `target.review` WIT operations, bounded verification and repair rounds, typed intermediate reports, and removal of repair-loop control from adapter prose.
>
> Builds on implemented [RFC-86](rfc-86-change-facts.md) (recorded `base.yaml` pins, one-member waves at `targets/<target>/waves/<digest>.yaml`, and content-addressed `BuildRecord`s at `builds/<digest>.yaml`) and [RFC-87](rfc-87-working-trees.md). The observable sequential gate becomes [RFC-92](rfc-92-concurrent-execution.md)'s worker and conflict-domain convergence loop; [RFC-93](rfc-93-distributed-execution.md) transports its phase records without redefining it. [RFC-95](rfc-95-native-verification.md) owns the deferred deterministic native-verification follow-on.



## Intent

Move build verification and repair control out of opaque agent prose and into the workflow engine before making builds concurrent.

### Today

RFC-86 / RFC-87 already bracket each slice build: refine records `base.yaml` (including `target_base`); build opens a one-member target wave at `.emery/targets/<target>/waves/<digest>.yaml` (`target.wave.opened`), prepares a private workspace from that pin, dispatches one `target.build`, gates the report, captures a code patch, persists a content-addressed `BuildRecord` at `builds/<digest>.yaml`, and projects `built` from that record and wave facts (D27). Merge commits the wave and still uses interim `apply` until RFC-88.

Consume that substrate — do not reinvent it: `project::build_record::BuildRecord`, `project::wave::Wave`, and the build envelope in `crates/slice/src/orchestrate/target.rs` (open wave → prepare → dispatch → capture → write record). This RFC replaces the **inside** of that envelope's single `target.build` dispatch.

What this RFC changes is the **adapter call shape**, not that substrate. Inside today's single `target.build`, the Omnia target still runs preparation, generation, optional replay, verification and repair, standards review, and review remediation. Cargo commands, retry counts, failure routing, and stopping conditions live in prompt text. The engine observes only the final report.

That shape hides the most important control loop from lifecycle policy and makes RFC-92 concurrency unsafe: splitting writers before replacing the loop would remove the only mechanism that currently returns failed checks to the code writer.

The engine guest also cannot execute the target project's native toolchain. Its `workflow` world imports `source`, `target`, and `emery:exec-bits` (plus the standard `wasi:blobstore` for snapshot objects — the workspace kernel itself runs in-guest); neither Emery nor Omnia currently supplies native process execution. The `wasi-model` `verify` grant authorizes model tooling but is not an execution capability.

### Goal

The engine owns a closed sequence of **build phases** over one disposable RFC-87 workspace:

```text
build → verify ⇄ repair → review ⇄ repair → complete
```

The target adapter still owns specialist behavior: generation prompts, platform-specific checks, repair instructions, and engineering-standards review. Each adapter invocation is exactly one WIT operation — `build`, `verify`, `repair`, or `review` — and returns a typed **phase report**. The engine persists the report, decides the next operation, enforces repair budgets, and assembles the final build report.

Verification in this RFC remains **model-assisted**. During a `verify` call, the target's agent runs its declared commands inside the lent workspace and returns findings. Moving orchestration into the engine makes the loop bounded and observable; it does not make command selection, execution, or findings deterministic or trustworthy.

## Flow and terms

1. Under a covering `plan.execute.started` epoch, the engine opens one RFC-86 one-member target wave for the build envelope (D9: content-addressed write-once at `targets/<target>/waves/<digest>.yaml`, binding build-authorization epoch, pinned base, member spec digest, and `depends-on`). Unchanged pins/epoch/spec yield the same wave digest on re-entry — that is identity reuse, not a mutable or resumable wave. It then prepares one writable RFC-87 **build workspace** from the recorded `base.yaml` `target_base` pin, and seeds one attempt-local **artifact stage** from the slice tree. Product code writes go to the workspace; every target-authored slice-artifact write goes to the stage.
2. It dispatches `build` once. Adapter-internal preparation and target-specific writer ordering may remain inside that call, but generation may not run a verification or repair loop.
3. It dispatches `verify` once. The adapter runs one check pass and returns one phase report.
4. Blocking verification findings cause one `repair` dispatch with `origin: verification` carrying D4's deterministic bounded repair brief, followed by another `verify`, while the verification-repair budget remains. The complete verification report remains gate and audit authority.
5. A passing verification dispatches `review` once. Blocking review findings route through the same repair-brief projection and one `repair` dispatch with `origin: review`, then `verify`, then `review`, while the review-remediation budget remains. If the post-repair verification fails, it enters the ordinary verification-repair loop and consumes the same verification budget as any other failed verification.
6. The engine assembles `build/report.yaml` by the fixed projection in D2, applies the existing blocking-finding and output-existence gates, captures the workspace result, validates the staged artifact diff, atomically promotes that diff, and writes the content-addressed RFC-86 `BuildRecord` at `builds/<digest>.yaml` (base/result/`touched`, wave digest, terminal report). `built` projects from that record and wave facts (D27) — phase/attempt trees do not mint lifecycle authority.
7. Budget exhaustion returns a typed failed build report with the unresolved findings. Product-code and artifact workspaces are discarded without changing authoritative target code, slice intent, target-owned slice outputs, or lifecycle state. Engine-owned attempt reports remain as audit evidence under `build/attempts/`; a failed attempt does not write or replace a successful `BuildRecord`.

A **build phase** is one engine-selected target operation: `build`, `verify`, `repair`, or `review`. A **phase report** is the typed result of exactly one operation. A **repair round** is one repair dispatch followed by the verification and, when applicable, review needed to decide whether that repair succeeded. A **terminal report** is the engine-authored `BuildReport` projected from the build phase plus the latest verification and review reports; superseded failed rounds remain only in the attempt record.

## Worked example

Suppose an Omnia-bound build generates a payment handler. The engine dispatches `build`, then `verify`. The verification agent runs the adapter's existing Rust checks and returns a blocking finding for an avoidable clone in `crates/payments/src/handler.rs`.

The engine persists that verification report and dispatches `repair` with the exact finding and the generation continuation. The adapter performs one repair pass and returns. The engine—not prompt prose—then dispatches `verify` again, decrements the verification-repair budget, and records the result.

After verification passes, the engine dispatches standards `review`. If review reports a blocking API-design finding, the engine permits the configured review-remediation round, verifies the repaired tree again, and reruns review. If the finding remains, the final build report fails with that finding. Every phase is visible even though the agent still owns the Cargo shell calls.

## Decisions



### D1 — The engine owns the build phase machine

Operation order, repair routing, budgets, terminal success, and terminal failure are deterministic engine policy. A target adapter cannot select its next operation, reset a budget, silently retry, or claim completion while a blocking phase report remains.

The initial policy permits at most three verification repair dispatches and one review remediation dispatch. These are engine constants, not fields supplied by a model or adapter.

Those constants are conservative first-run pins, not claims of optimality. Retained phase histories and blind live-evaluation results measure success after each repair round, unresolved-finding recurrence, latency, and model usage. A later RFC may revise a constant from that evidence; adapters and models still never choose or reset it at runtime.

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
        dispatch repair(origin: verification, findings: repair-brief(verify-report))
        verification-repairs += 1
        goto verification

review:
    review-report = dispatch review
    if review-report has no blocking findings:
        succeed
    if review-remediations == 1:
        fail with review-report findings
    dispatch repair(origin: review, findings: repair-brief(review-report))
    review-remediations += 1
    goto verification
```

A returned `repair` report never selects the next operation: after any completed or non-applicable repair, the engine dispatches `verify`. A repair dispatch error terminates the attempt. Blocking findings on a repair report are persisted as evidence but are superseded for terminal routing by the required verification that follows. Repair-brief selection never suppresses a gate finding: the complete source report remains persisted and the next verification evaluates the complete candidate again. A non-applicable `verify` or `review` report is a passing report with no blocking findings; a non-applicable repair still consumes the budget for the origin that caused it.

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
- `phase-finding`, an isomorphic WIT projection of the shared `Diagnostic` fields `id`, `rule-id?`, `related-rule-ids[]`, `title`, `severity`, `source`, `kind`, `artifact`, `location?`, the closed `snippet | digest | structured` evidence union, `impact`, `remediation`, `confidence?`, and `fingerprint`. The engine stamps `target-adapter`, `slice`, and change identity and verifies or recomputes the fingerprint. It groups by fingerprint and retains the strongest-severity representative, breaking a same-severity tie by the lexicographically least canonical JSON of the complete stamped finding with `id` omitted. It then sorts representatives by `(severity rank, location presence, path, line, column, fingerprint)`, where severity is `critical → important → suggestion → optional`, located findings precede unlocated findings, and missing line/column values follow concrete values. Finally it renumbers report-local ids. No title/impact/remediation folding is permitted.
- `phase-report { outcome, source, findings, outputs, ui-surface, written, next-continuation }`. `outputs` and `ui-surface` retain their current typed shapes. `next-continuation` is `option<list<u8>>`: `none` preserves the current adapter-opaque continuation, `some([])` clears it, and a non-empty value replaces it. The engine rejects a continuation larger than 1 MiB before persistence.

`build` alone owns output declaration and UI-surface classification: its `outputs` and `ui-surface` become the candidate values for the final report. `repair`, `verify`, and `review` must return empty `outputs` and no `ui-surface`; changing code beneath a declared output does not change the declaration. A `not-applicable` report must carry no blocking findings and no writes. These coherence rules are engine gates, not prompt conventions.

`phase-source` is an assurance claim, not an execution selector. `deterministic` means no model or external tool contributed to the phase result; `model-assisted` means the result came from model judgment, including an agent invoking and interpreting native commands; `hybrid` means more than one assurance source contributed. In this RFC the only accepted hybrid is deterministic in-guest checks plus model judgment; RFC-95 extends the gate to deterministic checks plus host-attested tool reports. The envelope must cover every finding source: a deterministic report cannot carry model-assisted findings, and a report containing both deterministic and model-assisted findings must be hybrid. The engine rejects `tool` in this RFC rather than silently relabelling it. Text and JSON operator output name the terminal verification report's source even when it has no findings, so a clean pass is never presented as stronger evidence than it is.

The engine assembles the terminal `BuildReport` deterministically:

1. `status` is `success` only after a non-blocking build, latest verification, and latest review plus all engine gates pass; every other terminal path is `failure`.
2. `outputs` and `ui-surface` come only from the build report.
3. `findings` are the canonically ordered, fingerprint-deduplicated, stably renumbered union of the build report, the latest verification report, the latest review report when one ran, and engine-authored terminal gate findings. Findings from superseded verify/review rounds and repair reports remain in phase records but do not leak into the terminal report.
4. On verification-budget exhaustion, the latest verification's blocking findings are terminal. On review-budget exhaustion, the latest review's blocking findings are terminal. On a dispatch or structural gate failure, the engine-authored diagnostic is terminal.

There is no `build-scope` variant: every operation names a slice except `verify`, which receives only the candidate workspace. Domain identity for RFC-92 convergence stays engine-side; the adapter runs the same check pass against whatever workspace the engine lends.

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

Separating `verify` and `review` keeps mechanical checks and standards judgment as different product surfaces: different prompts, budgets, and (later) execution trust. RFC-92 reuses the same `verify` against a composed candidate workspace; domain id never crosses this seam.

### D3 — Every dispatch performs exactly one operation

`build` may perform target-specific preparation, scaffolding, writer ordering, and capture replay needed to produce the candidate. It must not verify, repair, or run standards remediation.

`verify` performs one check pass and reports its findings. Native command execution remains model-assisted; an adapter may also include deterministic in-guest validators and report the phase as `hybrid`, or `deterministic` when no model leg ran. `repair` receives the engine-selected unresolved findings plus their verification/review origin and performs one repair pass. `review` performs one standards review pass and reports its findings. Prompts may describe how to perform their single operation; they may not contain retry loops or choose another operation.

A model-assisted verification pass may execute tests stored in the candidate workspace, including tests authored or edited by the same build. A green result therefore means **the candidate passes its own reported checks**, not that it passed an independent or protected oracle. The report source and operator output must preserve that assurance boundary. RFC-92 may protect independently governed verification inputs through its write-ownership envelope; RFC-95 defines host-owned profile and protected-oracle evidence.

Write protection is not the same as held-out evaluation. Any input mounted into a build, repair, review, or verification workspace is visible to that operation even when the engine prevents mutation. Harness evaluation therefore keeps its blind acceptance inputs outside every target and model context, runs them only after the workflow attempt, and records their grade separately from `BuildReport`, `BuildRecord`, and lifecycle projection. A blind grade can compare harness or model mixes; it cannot turn a failed workflow gate into success or become an undisclosed production requirement.

### D4 — Complete reports gate; bounded repair briefs focus

Phase findings use the shared diagnostic shape and artifact-relative locations. The complete D2-canonical phase report remains immutable gate and audit authority. Before `repair`, the engine projects a deterministic repair brief:

1. retain only blocking findings in their D2 canonical order;
2. retain the first 16.

Sixteen is the initial engine constant, pinned by integration and live-evaluation fixtures rather than supplied by an adapter or model. Findings beyond the repair brief remain in the persisted phase report and terminal routing. The verification after repair inspects the complete candidate, so selection cannot turn an unresolved finding into success. Adapter prose receives the typed brief; it does not reconstruct failures from a transcript or choose which reported failures count.

Root-cause suppression is producer-specific and does not belong in the neutral engine projection. A verifier may suppress cascades before returning its complete report only when its deterministic normalization contract defines that relationship. RFC-95 owns that contract for host-tool profiles.

Because verification remains model-assisted, its findings use `source: model-assisted` or `source: hybrid` as appropriate. This RFC does not label them `source: tool`: the engine does not receive and parse trusted tool output.

### D5 — One workspace spans the complete loop

All phases for one build run against the same disposable RFC-87 product workspace and the same attempt-local artifact stage. Generation, verification, repair, and review therefore observe one candidate code tree and one candidate slice-artifact tree.

The target `workspace` record gains `artifact-stage { id, root }`, an agent-visible writable mirror rooted at the candidate slice tree alongside the existing read-only project-wide `artifacts` root. Build inputs beneath the active slice resolve against the stage, so later phases read prior staged writes; non-slice project context continues to resolve against `artifacts`. Target metadata gains typed `writable-artifacts[]` grants using D2's exact `file | tree` grammar. Omnia declares `tasks.md`; Vectis declares `tasks.md`, `composition.yaml`, and its build bookkeeping subtree; Contracts declares `tasks.md` and `contracts/`. Target operations must write product code only under `workspace.root` and target-owned slice artifacts only under `workspace.artifact-stage.root`. The authoritative slice tree remains read-only to every target operation.

The engine seeds the stage before `build`, derives its actual diff after every mutating phase, and rejects a change outside `writable-artifacts[]` even when the phase omits it from `written`. Later phases read the staged candidate, so review and verification observe prior target-authored artifact changes without making them authoritative.

The engine captures product code and promotes staged artifacts only after the terminal phase machine and final report gates pass. Artifact promotion is an engine-owned recoverable transaction: validate the complete diff first, prepare every replacement, commit the set all-or-none, and roll back before returning any promotion error. The engine writes the successful `BuildRecord` (and only then projects `built`) after code capture and artifact promotion both succeed. A dispatch error, budget exhaustion, blocking final report, scope violation, promotion failure, or cooperative cancellation discards both writable trees and leaves authoritative target code, target-owned slice artifacts, and lifecycle unchanged. Process death may leave local disposable directories for RFC-87 garbage collection; it cannot expose their contents as authoritative state.

Engine-owned request, phase, and terminal reports are audit records, not target-authored slice intent. They may persist for a failed attempt without violating this rule.

### D6 — Phase reports are durable; `BuildRecord` remains outcome authority

Each invocation receives the next monotonic attempt id and writes under `.emery/slices/<slice>/build/attempts/<attempt>/`. The engine copies the immutable `build/request.yaml` into that attempt, writes every returned report as `phases/<ordinal>-<operation>.yaml`, and appends a `slice.build.phase-completed` fact naming the attempt, ordinal, operation, source, report digest, and engine-measured `elapsed-ms`. That fact is a **new closed `EventKind` variant** in RFC-86's journal taxonomy — ordinal evidence inside the existing `slice.build.started` / `.succeeded` / `.failed` envelope, not a parallel log or side channel. Elapsed time is raw telemetry outside the report digest and lifecycle projection; distributions are derived from retained events rather than emitted as authority. A retry creates a new attempt; it never appends to, clears, or reuses a prior attempt or continuation.

The phase event is also the join anchor for model-backend telemetry when the deployment exposes it: effective route and model identity, input and output tokens, and reported cost. Missing usage data remains missing rather than estimated by the engine. These observations stay outside report digests and lifecycle projection; RFC-92's harness comparison derives cost per accepted result and worker spend induced by decomposition instead of optimizing per-call price.

The engine writes the attempt's terminal report beside its phases and projects that same body to the canonical `build/report.yaml`. A new terminal attempt atomically replaces the canonical projection; immutable attempt records retain the complete history. The engine alone assembles both copies.

On terminal success, the engine also writes the RFC-86 content-addressed `BuildRecord` at `.emery/slices/<slice>/builds/<digest>.yaml` — base/result/`touched`, the envelope wave's digest, and the terminal report body. That record remains build-outcome authority for merge revalidation and the `built` projection (D27). Attempt trees are orchestration audit; they do not replace `builds/<digest>.yaml`. A failed or abandoned attempt may leave attempt-local reports and a failed canonical `build/report.yaml` without writing or replacing a successful `BuildRecord`. Later lifecycle gates consume the canonical report; merge and wave revalidation consume the `BuildRecord`; RFC-92 domain convergence may cite immutable phase, attempt-report, or `BuildRecord` digests.

Attempt ids are zero-padded ordinals allocated by atomically creating the next absent attempt directory; an existing id is never reused, even when its attempt has no terminal report. Every request, phase report, and terminal report write is atomic. On re-entry, an attempt without a terminal report is **abandoned**: its returned phase reports remain immutable evidence, its continuation is never loaded, and the engine starts the next attempt from the recorded `base.yaml` pin in fresh product and artifact workspaces. Phases and attempts of one build envelope share one wave identity (D9): unchanged pins, covering epoch, member spec, and `depends-on` resolve to the same content-addressed wave digest; the engine does not invent a second wave lifecycle or resume a mutable wave. It does not try to infer whether an interrupted adapter call wrote unreported files. The canonical `build/report.yaml` remains the projection of the latest terminal attempt and is unchanged by an abandoned attempt.

An orderly failure writes the failed terminal report before discarding its writable trees. Abrupt process loss may leave an unterminated attempt and disposable workspace directories; the next invocation applies the abandonment rule and RFC-87 garbage collection removes the directories. Attempt records archive with the slice and follow the existing archive-retention policy; build re-entry never prunes them in place.

Phase reports are evidence of orchestration, not new lifecycle states. `built` still projects once from a successful `BuildRecord` and wave facts (D27); attempt and phase trees never become lifecycle authority.

### D7 — Existing target behavior moves without changing ownership

Omnia splits its current preparation/generation/replay work into `build`, its Cargo check pass into `verify`, one findings-directed writer pass into `repair`, and one engineering-standards pass into `review`. Vectis and contracts implement the same four operations with their own specialist behavior; an inapplicable operation returns a typed non-blocking report rather than inventing adapter-specific operation names.

The adapter repository continues to own prompts, engineering standards, target composition, and report content. Emery owns the operation vocabulary, loop, persistence, budgets, and final lifecycle gate.

### D8 — Deterministic native verification belongs to RFC-95

Closed command profiles, native process execution, sandbox and resource policy, trusted tool-output parsing, protected-oracle classification, verification-lineage caches, execution grants, and typed toolchain unavailability are not part of this RFC. [RFC-95](rfc-95-native-verification.md) owns that follow-on.

RFC-95 runs tools in the trusted native deployment below the component boundary and exposes only a custom host-verification WIT import (on the `emery:exec-bits` capability-crate shape). It does not wait on [`wasi:exec`](https://github.com/WebAssembly/WASI/issues/899) or any other standardized WASI process API. When landed, it replaces the model-assisted evidence inside `verify` while retaining this RFC's lifecycle, budgets, bounded repair-brief projection, target repair routing, and final report assembly. Its optional host mechanical-repair rung is an explicit phase-machine and write-authority amendment rather than an implicit part of `verify`.

## Implementation requirements

- Extend `wit/emery.wit`, target metadata, the adapter SDK `Target` trait and export macro, the guest provider, native provider, mock catalog, and wire DTOs with `repair`, `verify`, and `review`, the closed D2 phase vocabulary, `writable-artifacts[]`, and `workspace.artifact-stage`.
- Replace the single target-build dispatch inside `crates/slice/src/orchestrate/target.rs` with D1's exact engine-owned phase machine, including the shared verification-repair counter, review-remediation counter, repair-origin routing, continuation replacement rules, and deterministic terminal-report projection. Keep the landed wave-open / prepare / capture / `BuildRecord::write` envelope; do not re-home pins or invent a parallel outcome path.
- Canonicalize every accepted phase report, retain complete reports as gate authority, and project D4's fingerprint-deduplicated, ordered, 16-finding repair brief without suppressing terminal findings.
- Persist immutable attempt-scoped phase reports and continuations, abandon rather than resume unterminated attempts, add `slice.build.phase-completed` as a closed `EventKind` variant in `crates/project/src/journal/event.rs`, and keep canonical `build/report.yaml` as the latest terminal engine-assembled projection.
- On terminal success only, write the RFC-86 `BuildRecord` at `builds/<digest>.yaml` from the captured patch, envelope wave digest, and terminal report via `project::build_record::BuildRecord`. Preserve `slice.build.started` / `.succeeded` / `.failed` beside phase-completed facts.
- Keep one RFC-87 product workspace and one artifact stage for the complete loop, prepared from the recorded `base.yaml` pin under the envelope's one-member wave (`project::wave::Wave`); capture and transactionally promote only on terminal success, and discard both on every failure or cooperative cancellation path.
- Split Omnia, Vectis, and contracts target implementations across `build` / `repair` / `verify` / `review` without moving specialist prompts or engineering standards into Emery.
- Move Omnia's `tasks.md`, Vectis's `tasks.md` / `composition.yaml` / build bookkeeping, and Contracts' `tasks.md` / `contracts/` writes onto the artifact stage; reject undeclared target-authored slice writes.
- Delete verification, retry, writer re-entry, and review-remediation loops from adapter prose. An operation prompt may perform one pass only.
- Preserve the existing final blocking-finding, declared-output, patch-capture, wave, and `BuildRecord` gates; continue to project `built` only from a successful record and wave facts (D27).
- Require and validate report-level `phase-source`, include the terminal verification source in text and JSON operator output even on a clean pass, identify candidate-owned checks as self-consistency evidence rather than an independent oracle, and do not claim deterministic, sandboxed, or trusted native execution.
- Keep blind evaluation inputs outside every target/model workspace and lifecycle gate. Correlate their post-attempt grades with phase timing and available backend usage through retained event identities; do not expose the acceptance set to build, repair, verify, review, or decomposition.



## Acceptance criteria

1. One build produces an ordered persisted sequence of engine-selected `build`, `verify`, optional `repair`, and `review` phase reports before one final build report. On terminal success it also writes one RFC-86 `BuildRecord` at `builds/<digest>.yaml` from the captured patch, envelope wave digest, and terminal report; failed attempts do not replace a prior successful record. `built` projects from that record and wave facts only.
2. No target prompt contains a verification-repair or review-remediation retry loop; injected failures return to the engine after one operation.
3. Verification failures retain their full, unfolded typed report and route D4's deterministic bounded brief to `repair(origin: verification)`, then back through `verify`; review findings follow the same projection into `repair(origin: review)`, then through `verify → review`. Omitted brief findings remain gate-visible. A failed verification after review repair consumes the shared verification budget. The fourth verification repair and second review remediation are never dispatched.
4. The adapter cannot select the next operation, alter the attempt count, reset a budget, or write the terminal build report.
5. The same RFC-87 workspace id and artifact-stage id are used throughout one attempt. Failed, exhausted, cancelled, out-of-scope, and promotion-failed builds leave authoritative target code, target-owned slice artifacts, and lifecycle unchanged while retaining engine-owned attempt reports.
6. The phase-report gate rejects unknown outcome/source values, `tool` in this RFC, a source inconsistent with its findings, folded findings, malformed or escaping locations/writes, oversized continuations, non-build output/UI declarations, and a non-applicable report with blocking findings or writes. Clean verification output still identifies its report-level source.
7. Phase and terminal report assembly is byte-stable: D2 selects one canonical representative per fingerprint, sorts those representatives by the closed key, and then renumbers ids; only the build report supplies outputs/UI surface; only the latest verify/review rounds contribute gate findings; superseded and repair findings remain attempt-local.
8. Omnia/Rust preserves its current generation, replay, Cargo-check, repair, standards-review, output, and finding behavior through the four operations.
9. Vectis and contracts implement every operation, using typed non-applicability where an operation has no target-specific work, and all three first-party targets declare and obey their writable artifact scopes.
10. Native and Wasm integration suites cover success, verification repair, review remediation, post-review verification failure, both budget exhaustions, both repair origins, canonical ordering, repair-brief deduplication and truncation without gate suppression, continuation preserve/replace/clear and size rejection, attempt isolation, interrupted-attempt abandonment, canonical-report preservation across interruption, source-assurance validation, phase-report persistence, artifact promotion/rollback, workspace discard, and final report assembly.
11. Every phase-completed event records engine-measured elapsed milliseconds outside the report digest and can correlate available backend route, model, token, and reported-cost observations without inventing missing values. `wasm-omnia-r9k` records operation count, model calls, repair routing, phase timings, and available usage; every formerly hidden retry is visible as an engine phase.
12. A blind live-evaluation acceptance set is unavailable to decomposition, build, repair, verify, and review, and its post-attempt grade remains outside build reports, records, and lifecycle projection. Documentation describes the workflow gate as model-assisted, calls candidate-owned checks self-consistency evidence rather than an independent oracle, and identifies RFC-95 as the deterministic native-verification follow-on.



## Rejected alternatives

- **Wait for RFC-95 / native host verification before exposing the loop** — leaves retry authority hidden in prose and blocks RFC-92's observable worker orchestration on the native-verification follow-on. (RFC-95 itself does not wait on `wasi:exec`; rejecting the wait is about shipping the engine-owned phase machine first.)
- **Move the loop from prose into adapter Rust** — improves prompt size but leaves operation order, budgets, and retries adapter-owned and invisible to the workflow engine.
- **Have the engine repeat today's monolithic** `target.build` — replays preparation, generation, and review against a mutating workspace without identifying which action is intended.
- **One** `build` **method with a** `build-phase` **enum** — collapses generation, repair, verification, and standards review behind a stage parameter. Rejected so `verify` and `review` stay distinct product surfaces (different prompts, budgets, and later execution trust) and RFC-92 can call `verify` alone without a phase discriminator.
- **A** `build-scope` **(**`slice` **|** `domain`**) parameter on** `verify` — adapter check behavior does not branch on scope; domain identity belongs in engine-side round records. Deferred until something in the adapter needs it.
- **Let adapters define arbitrary operation names or next actions** — fragments workflow policy and allows an adapter response to control orchestration.
- **Call model-assisted shell execution deterministic verification** — overstates the trust boundary; the agent still controls command invocation, output interpretation, and reporting.
- **Invent a second wave lifecycle for attempts or repairs** — fights RFC-86 D9's content-addressed write-once wave; phases and abandoned attempts share the envelope wave digest when inputs are unchanged.
- **Make the build report an RFC-18 reward contract** — structured reports are reusable inputs, but a training scorer also grades traceability, layout, migration, and configuration concerns outside build verification. RFC-18 may project a reward from durable reports without making model-training rankability a lifecycle invariant.

