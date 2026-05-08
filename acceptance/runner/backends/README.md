# Runner Backends

Backends plug into the runner through one small TypeScript interface
(`Backend` in [`../types.ts`](../types.ts)). Suites pick a backend through
their scenario frontmatter (`backend: manual | stub | scripted-plan | scripted-execute | scripted-finalize | contracts-build | omnia-build | vectis-build | agent | recorded | fixture`).

Per the framework contract in
[`../README.md`](../README.md) and
[`../../README.md` §Run Evidence Policy](../../README.md#run-evidence-policy),
backends:

- read the `Scenario` and the `RunPaths` (a temp workspace plus an
  evidence run directory),
- run their own logic in `prepare` / `invoke` / `teardown`,
- return a `BackendResult` (verdict, optional fault-domain hint, notes,
  assertion records, optional `evidence` payload for the runner-owned
  `assertions` stage),
- never mutate `.specify/` lifecycle state directly. Lifecycle transitions
  go through the `specify` CLI per
  [`../../README.md` §CLI-Authoritative Invariant](../../README.md#cli-authoritative-invariant).

The backend taxonomy itself — manual, deterministic stub, agent runtime,
recorded transcript, fixture — is documented in
[`../README.md` §Backend Interface](../README.md#backend-interface).

## Available Backends

| Backend    | File         | Status                                                                                                                                |
| ---------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------- |
| `manual`   | `manual.ts`  | Implemented. Prints the operator briefing. Default verdict `pending-operator`; pass `--operator-results <path>` for a real pass/fail. |
| `fixture`  | `fixture.ts` | Implemented (C05). Materialises a known expected-artifact set from `acceptance/fixtures/<scenario-id>/expected/` and lets the runner-owned `assertions` stage validate it. Used by `make acceptance-smoke`. |
| `stub`     | `stub.ts`    | Implemented (C08). Deterministic *workflow* stub: runs `specify init`, `specify slice {create, transition, merge run}` against a real `specify` binary and (optionally) materialises a per-stage fixture directory into the workspace. Distinct from `fixture`, which materialises a single artifact set without calling the CLI. Used by `make acceptance-stub-smoke`. |
| `scripted-plan` | `scripted-plan.ts` | Implemented (C09). Cross-repo *plan-shape* stand-in: calls `setupHub` (hub + bare remotes + fake `gh`) then a fixed sequence of `specify change plan {create, add}` calls so the role-based RM-01 plan assertions exercise end-to-end. Does **not** prove `/change:plan` itself does the right thing on the brief — that requires the reserved `agent` backend. Used by `make acceptance-cross-repo-plan-smoke`. |
| `scripted-execute` | `scripted-execute.ts` | Implemented (C10). Cross-repo *execute-shape* stand-in: composes `scripted-plan` setup + plan creation with a deterministic loop driver (the moral equivalent of `/change:execute loop`). Drives `change plan next` → `workspace prepare-branch` → per-slice baseline/residue commit pair → `change plan transition done` until `all-done`. Does **not** prove `/change:execute loop` itself does the right thing — that requires the reserved `agent` backend. Used by `make acceptance-cross-repo-execute-smoke`. |
| `scripted-finalize` | `scripted-finalize.ts` | Implemented (C11). Cross-repo *landing-path* stand-in: composes `scripted-plan` setup + plan creation with the C10 deterministic loop driver, then layers `workspace push` → optional pre-merge negative probe → fake-`gh` mark-merged (via `markPrMerged` from `fake-gh.ts`) → `change finalize` → second-call idempotency probe on top. Captures push/finalize JSON into the run dir (`push-output.json`, `finalize-output.json`, `finalize-output.second-call.json`, optional `finalize-output.pre-merge.json`). Does **not** prove the post-execute orchestration skill itself — that requires the reserved `agent` backend. Used by `make acceptance-cross-repo-finalize-smoke`. |
| `contracts-build` | `contracts-build.ts` | Implemented (C13). Cross-repo *contracts-build* stand-in: composes `scripted-plan` setup + plan creation with the C10 deterministic loop driver, but uses the C13 per-slice phase-driver dispatch (`phaseDriverFor`) so the contract slice runs through `ContractsBuildPhaseDriver` (deterministic but realistic OpenAPI 3.1 + JSON Schema emission) while implementation slices keep `StubPhaseDriver`. The emitted contract bundle passes the contracts WASI tool unchanged. Execute-only by design — push / finalize coverage stays on `scripted-finalize` / `agent`. Used by `make acceptance-cross-repo-contracts-build-smoke`. |
| `omnia-build` | `omnia-build.ts` | Implemented (C14a). Cross-repo *omnia-build* stand-in: composes `scripted-plan` setup + plan creation with the C10 deterministic loop driver, but extends the C13 per-slice phase-driver dispatch so the contract slice still runs through `ContractsBuildPhaseDriver` (Omnia builds need real contract YAML to consume), Omnia-capability slices run through `OmniaBuildPhaseDriver` (deterministic Rust crate skeleton: `Cargo.toml` + `src/lib.rs` + `src/providers.rs`, scrubbed against `plugins/omnia/references/guardrails.md`), and other slices keep `StubPhaseDriver`. Execute-only by design — push / finalize coverage stays on `scripted-finalize` / `agent`. Used by `make acceptance-cross-repo-omnia-build-smoke`. |
| `vectis-build` | `vectis-build.ts` | Implemented (C14b). Cross-repo *vectis-build* stand-in: composes `scripted-plan` setup + plan creation with the C10 deterministic loop driver, but extends the C13 per-slice phase-driver dispatch so the contract slice still runs through `ContractsBuildPhaseDriver` (Vectis builds need real contract YAML to consume), Vectis-capability slices run through `VectisBuildPhaseDriver` (deterministic Vectis composition + SwiftUI shell: `composition.yaml` at the project root validating against `capabilities/vectis/composition.schema.json`, plus `apps/mobile/<screen>.swift` residue), and other slices keep `StubPhaseDriver`. Execute-only by design — push / finalize coverage stays on `scripted-finalize` / `agent`. Used by `make acceptance-cross-repo-vectis-build-smoke`. |
| `agent`    | `agent.ts`   | Implemented (C12). Cross-repo *real-define* driver: composes `scripted-finalize`'s setup + plan creation + push + finalize phases, but plugs an `AgentPhaseDriver` (operator-manual today; Cursor SDK option deferred) into the per-slice loop driver instead of `StubPhaseDriver`. Reads `--operator-results <path>.json` for per-slice define-stage bodies, falls back to stub bodies for missing slices, and exits 0 with `pending-operator` when no operator results are supplied. Used by `make acceptance-cross-repo-define-smoke`. |
| `recorded` | `recorded.ts` | Implemented (C15). Cross-repo *cli-substrate regression pin*: composes `prepareScriptedHub` from [`scripted-shared.ts`](scripted-shared.ts) with a recorded JSONL trace at `--recorded-trace <path>.jsonl` (or `acceptance/recorded/<scenario-id>/<trace-id>.jsonl` by convention). For each recorded `RecordedAction` with a `command` argv, replays it via `runSpecify` and compares the live exit code to the recorded value; mismatches fail with `cli-substrate` (recorded 0 → live non-zero) or `live-agent-nondeterminism` (any other delta). Records without a `command` are book-keeping (e.g. `scripted-execute-loop` per-slice markers) and are tracked as `synthetic-skipped`. Used by `make acceptance-cross-repo-recorded-smoke`. |

## Lifecycle Stages

Every run goes through four stages:

1. **`prepare(ctx)`** — backend seeds scenario inputs (briefs, fixture
   files, fake `gh` config) into `ctx.paths.workspace`.
2. **`invoke(ctx)`** — backend executes the scenario's invocation and
   returns a `BackendResult`. The result may include an optional
   `evidence` payload (`BackendEvidence`) carrying captured stdout,
   materialised paths, etc.
3. **`assertions` (runner-owned)** — between `invoke` and `teardown`
   the runner dispatches each id in `scenario.frontmatter.assertions`
   to a handler registered in [`../assertions.ts`](../assertions.ts).
   Handlers live under [`../../assertions/`](../../assertions/README.md)
   so they can be reused without pulling in backend code. Backend
   records and dispatcher records are merged into one
   `assertions.json` payload; dispatcher records win when ids collide.
4. **`teardown(ctx)`** — backend releases external resources (kill
   child processes, close fake-`gh` sockets). Filesystem cleanup is
   the runner's job so retention rules stay centralised.

The `assertions` stage is **not** part of the `Backend` interface.
Backends must not run their own assertion helpers ad hoc; the runner
calls helpers in one place so every backend produces comparable
evidence.

## Backend Mismatch Policy

Passing `--backend X` for a scenario whose frontmatter declares a
different backend is a **hard error** (exit code 2). Override with
`--allow-backend-mismatch` when this is intentional — for example,
running a `manual` scenario through the `fixture` backend for
non-interactive smoke coverage in CI. `make acceptance-smoke` uses the
override flag to drive `contracts-describe` (declared `backend: manual`)
through the fixture backend.

## Stub Backend (C08)

`backend: stub` is the deterministic workflow stub. It exists so the
runner can prove execution mechanics — slice create, transitions,
spec-merge, archive — without driving a live agent. Two concepts in
the scenario frontmatter govern its behaviour:

- `stubbed-stages: [define, build, merge]` — the subset of `stages`
  the stub drives via `specify`. `scripts/checks.ts` enforces that
  every entry is also in `stages` and that the field only appears
  with `backend: stub`.
- `stub-fixtures: { build: <repo-relative-dir> }` — optional per-stage
  fixture directory the stub copies into the workspace at the start of
  the stubbed stage. Required when `build` is stubbed AND
  `expected-artifacts:` is non-empty (the stub has nothing to
  materialise otherwise; failure surfaces as `runner-setup`).

Disclosure shape: every action the stub performs is recorded in
`BackendResult.evidence.extras.stubbed`, an object with this shape:

```ts
interface StubEvidence {
  scenario: string;                 // scenario id
  slice: string;                    // slice name (defaults to scenario id)
  stubbedStages: ("define" | "build" | "merge")[];
  skipped?: boolean;                // true when no `specify` was resolvable
  reason?: string;                  // skip rationale
  actions: Array<{
    phase: "setup" | "define" | "build" | "merge";
    slice: string;
    action: string;                 // e.g. "specify-slice-create"
    command?: string[];             // argv passed to `specify`
    artifacts: string[];            // workspace-relative paths written
    ts: string;                     // ISO 8601 timestamp
    exitCode?: number;
  }>;
}
```

The runner persists this payload two ways:

1. `summary.md` includes a "Stub Backend Disclosure" block with the
   stage list, action count, skip note, and a per-action table.
2. `stub-actions.jsonl` is written next to `assertions.json` with one
   JSON record per line — a leading `stub-actions-header` line carries
   the scenario / slice / stage metadata, followed by one record per
   `StubAction`. `jq` and `grep` are happy.

### Skip-with-explanation

The stub backend resolves `specify` via `findSpecifyBin()` — `SPECIFY_BIN`
overrides PATH. When neither resolves, `prepare()` records a skip
reason, `invoke()` returns `verdict: pending-operator` with an
explanatory note, and sets `evidence.extras.skipAssertions = true` so
the runner's assertion stage does not fire `files-exist` against an
empty workspace. The runner exits 0 and `make acceptance-stub-smoke`
is therefore safe to run on systems without a built `specify`.

### Failure modes

| Trigger                                                               | Verdict   | Fault domain      |
| --------------------------------------------------------------------- | --------- | ----------------- |
| `stubbed-stages` empty under `backend: stub`                          | `failed`  | `runner-setup`    |
| `build` stubbed with non-empty `expected-artifacts` but no `stub-fixtures.build` | `failed`  | `runner-setup`    |
| `stub-fixtures.<stage>` is absolute or missing                        | `failed`  | `runner-setup`    |
| `specify init` / `slice create` / `slice transition` returns non-zero | `failed`  | `cli-substrate`   |
| Workspace dirty after stubbed merge                                   | `failed`  | `runner-setup`    |

Illegal-transition coverage falls out of the CLI for free: a scenario
that stubs `[build, merge]` but skips `define` will `slice create` (slice
is in `defining`) and then ask for `slice transition <slice> building`
— the CLI enforces the legal transition graph and exits non-zero, which
the stub maps to a `cli-substrate` fault.

### Capability resolution

For capability-owned scenarios (the common case), the stub computes a
`file://<repo-root>/capabilities/<owner>` URI and passes it to
`specify init`. Multi-repo / hub-driven scenarios are out of scope for
C08; that path lands with C10 once the `/change:execute loop` driver is
wired against the same stub.

## Operator-Results File (Manual Backend)

The manual backend accepts a `--operator-results <path>` flag pointing
at a JSON file an operator (or upstream agent) populated after running
the scenario's Invocation block. With the file supplied, the manual
backend produces a real pass/fail verdict instead of the default
`pending-operator`. The file shape is documented at the top of
[`manual.ts`](manual.ts) and includes:

- `scenario` (optional id check),
- `completed` (boolean, defaults to `true`),
- `notes` (free-form text surfaced in the run summary),
- `verifierStdout` (forwarded to the verifier assertion handler),
- `assertions[]` (per-id verdicts the operator already collected).

The runner-owned `assertions` stage still examines the workspace and
merges its own records, so on-disk truth always wins over a stale
operator report.

## Scripted-Plan Vs Real-Agent Boundary (C09)

The `scripted-plan` backend exists because real `/change:plan` is a
Cursor slash-command skill, not a CLI subcommand: it requires an agent
runtime to interpret a brief and emit `specify change plan {create,
add, amend}` calls. C09 ships the **assertion plumbing** for the RM-01
cross-repo suite plus a deterministic stand-in that exercises it end
to end:

- The scripted-plan backend hard-codes the plan shape (`oauth-login`
  change, contract slice + backend slice routed to `shop-backend` +
  mobile slice routed to `shop-mobile`). It never reads the body of
  `inputs/docs/oauth-login.md`.
- The role-based assertions in `acceptance/assertions/plan-roles.ts`
  score the resulting plan against the rules in
  `acceptance/suites/rm01-cross-repo/expected/plan-roles.md`.
  Identical scoring will apply when the real agent backend lands.

A reader inspecting an `acceptance-cross-repo-plan-smoke` run dir is
seeing the assertion plumbing pass, **not** the planner skill.
Tightening the proof — variation in slice naming, real brief
interpretation, project-assignment heuristic exercise — requires
plugging the `agent` backend into the same scenario. The C09 plan
amendment for C10/C12/C15 explicitly reserves that work; do not
backfill it into the scripted-plan backend.

To run real `/change:plan` against the same scenario today, an
operator can:

1. invoke `make acceptance-cross-repo-setup-smoke` to land a fresh
   hub, then `cd` into the temp `shop-platform/` dir,
2. invoke `/change:plan oauth-login source brief=docs/oauth-login.md`
   in their Cursor session,
3. capture the resulting plan into an operator-results JSON file and
   re-run the suite via `--backend manual --operator-results <path>`.

That manual loop will be subsumed by the `agent` backend once a Cursor
SDK / CLI integration is wired up.

## Scripted-Execute Vs Real-Agent Boundary (C10)

The `scripted-execute` backend extends the C09 plumbing pattern: it
proves the **execute-stage assertion plumbing** (setup → plan-shape
→ deterministic loop driver → execute-* rules) end to end against a
fixed CLI sequence, without depending on a Cursor agent runtime.

It deliberately:

- composes `scripted-plan`'s `prepareScriptedHub` + `runPlanCreationSequence`
  helpers from [`scripted-shared.ts`](scripted-shared.ts), so plan
  creation is byte-for-byte identical between the plan-only and
  execute smokes,
- iterates `specify --format json change plan next` as the loop
  oracle (mirroring `specify-cli/tests/cross_repo.rs`'s `next_entry`),
- delegates per-entry phase outcomes to
  [`StubBackend.driveSlice`](stub.ts) — the loop driver lives in C10;
  the stub stays a passive lifecycle executor (C10 amendment §"loop
  driver lives in C10"),
- writes one baseline merge commit (`specify: merge <slice>`) and
  one residue commit (`specify: residue <slice>`) per routed slice
  inside the workspace clone, exactly as the Layer 0 substrate test
  does.

It does NOT:

- read the body of the fixture brief,
- vary slice names, residue paths, or routing based on prose,
- invoke any Cursor slash-command skill,
- exercise C11 territory (`specify workspace push`, fake-`gh` PR
  creation, `specify change finalize`).

A reader inspecting an `acceptance-cross-repo-execute-smoke` run dir is
seeing the assertion plumbing pass against a deterministic baseline,
**not** the loop skill itself. Tightening the proof — variation in
slice naming, brief-driven residue path selection, real
`/spec:define`/`/spec:build` outputs — requires plugging the `agent`
backend into the same scenario. The C10 plan amendment for C12-C14
explicitly reserves that work; do not backfill it into the
scripted-execute backend.

### Composition Pattern (For C11 / Future Backends)

`ScriptedExecuteBackend` follows the **composition** pattern (C10
amendment §"Recommendation: composition"): it does not subclass or
extend `ScriptedPlanBackend`; instead, both backends import the same
helpers from [`scripted-shared.ts`](scripted-shared.ts) and stack
their own phase logic on top.

C11 (push + finalize) lands the same pattern: `ScriptedFinalizeBackend`
reuses `prepareScriptedHub` + `runPlanCreationSequence` and instantiates
`ScriptedExecuteBackend` to drive the per-slice loop to `all-done`,
then layers `workspace push` + fake-`gh` mark-merged + `change finalize`
on top. New CLI surface goes into shared helpers when more than one
backend needs it; per-backend logic stays in the backend file.

## Scripted-Finalize Vs Real-Agent Boundary (C11)

The `scripted-finalize` backend extends the C10 pattern: it proves the
**landing-path assertion plumbing** (setup → plan-shape → loop driver
→ push → mark-merged → finalize → idempotency probe) end to end,
without a Cursor agent runtime.

It deliberately:

- composes `prepareScriptedHub` + `runPlanCreationSequence` from
  [`scripted-shared.ts`](scripted-shared.ts) and instantiates
  [`ScriptedExecuteBackend`](scripted-execute.ts) to drive the
  per-slice loop to `all-done` (so the C10 baseline/residue commit
  pair lands unchanged),
- runs `specify --format json workspace push` from the workspace
  clone and captures the JSON to `push-output.json`,
- (optionally, when `runPreMergeProbe` is enabled) runs `specify
  --format json change finalize` BEFORE marking PRs merged and
  captures the result to `finalize-output.pre-merge.json` so the
  `finalize-runs-before-prs-merged` negative expectation can be
  verified,
- mutates each routed project's fake-`gh` PR-state file to `MERGED`
  via [`markPrMerged`](../fake-gh.ts) (the load-bearing 5-field,
  pipe-separated format documented in `fake-gh.ts`),
- runs the first `specify --format json change finalize` to land the
  archive commit and capture `finalize-output.json`,
- runs a second `specify --format json change finalize` for the
  idempotency probe and captures `finalize-output.second-call.json`,
- records every CLI step into
  `BackendResult.evidence.extras.scriptedFinalize.actions`, persisted
  by the runner to `scripted-finalize-actions.jsonl`.

It does NOT:

- read the body of the fixture brief,
- exercise a real forge (the scripted backend's PR mutation is the
  full extent of "merge" — no `gh` API call, no upstream merge
  commit beyond what `specify` itself writes),
- vary slice naming, residue paths, or routing based on prose,
- invoke any Cursor slash-command skill.

A reader inspecting an `acceptance-cross-repo-finalize-smoke` run dir
is seeing the assertion plumbing pass against a deterministic
baseline, **not** the post-execute orchestration skill itself.
Tightening the proof — variation in slice naming, real
`/change:execute loop` outputs feeding push, real forge integration
— requires plugging the `agent` backend into the same scenario. The
C11 plan amendment for C12-C16 explicitly reserves that work; do not
backfill it into the scripted-finalize backend.

## Agent Backend (C12)

The `agent` backend is the C12 hand-off for real `/spec:define`
execution. It composes `scripted-finalize`'s setup + plan creation
+ push + finalize phases — so every existing setup-* / plan-* /
execute-* / push-* / finalize-* assertion still asserts unchanged —
but swaps the per-slice phase-outcome producer from `StubPhaseDriver`
to `AgentPhaseDriver`. Both drivers route through
`driveSliceWithBodies` in [`phase-driver.ts`](phase-driver.ts) so the
CLI sequence and commit shape stay byte-for-byte identical; only the
artifact bodies vary.

### `PhaseDriver` Interface (C11/C12 amendment)

```ts
interface PhaseDriver {
  readonly name: string;
  driveSlice(opts: DriveSliceOpts): Promise<DriveSliceResult>;
}
```

`DriveSliceOpts` carries everything the loop driver already collected
(resolved `specify` binary, hub dir, slice name, routed project,
residue path) plus a `capabilityName` hint the
`slice-has-design-when-required` assertion handler reads. The C10
`ScriptedExecuteBackend` and C11 `ScriptedFinalizeBackend` both accept
a `phaseDriver?: PhaseDriver` constructor option that defaults to
`new StubPhaseDriver()`; the agent backend instantiates a
`ScriptedFinalizeBackend` with `phaseDriver: new AgentPhaseDriver(...)`.

### Per-Slice Phase Driver Dispatch (C13 amendment)

C13 needs `ContractsBuildPhaseDriver` for the contract slice while
keeping implementation slices on `StubPhaseDriver` (real Omnia /
Vectis builds are deferred to C14a / C14b). Solving this with a
single `phaseDriver` per backend would force every backend
permutation into a separate class; the C13 amendment instead adds a
**per-slice dispatch callback** to both scripted backends:

```ts
export interface PlanEntry {
  /** Slice name (matches the loop driver's plan entry). */
  name: string;
  /** Routed project name; `null` for the projectless contract slice. */
  project: string | null;
  /** Capability brief in play (e.g. `contracts`, `omnia`, `vectis`). */
  capability?: string;
}

export interface ScriptedExecuteBackendOptions {
  /** Default driver used when `phaseDriverFor` is absent or returns nothing. */
  phaseDriver?: PhaseDriver;
  /** Per-slice driver selector. Overrides `phaseDriver` when set. */
  phaseDriverFor?: (entry: PlanEntry) => PhaseDriver;
  /** Optional capability lookup for slice → capability mapping. */
  capabilityForSlice?: (sliceName: string) => string | undefined;
}
```

`ScriptedFinalizeBackendOptions` carries the same `phaseDriverFor`
field. When unset, both backends fall back to the default
`phaseDriver` (preserving the C10/C11/C12 single-driver behaviour
without changes), so existing scenarios that rely on the global stub
or the agent driver keep working unchanged. When set, the loop
driver invokes `phaseDriverFor(entry)` per iteration to pick a
driver. The C13 `ContractsBuildBackend` wires this up as:

```ts
const stubDriver = new StubPhaseDriver();
const contractsDriver = new ContractsBuildPhaseDriver();
new ScriptedExecuteBackend({
  phaseDriverFor: (entry) =>
    entry.name === SLICE_CONTRACT ? contractsDriver : stubDriver,
});
```

C14a now ships `OmniaBuildPhaseDriver` (real Rust crate skeleton
emission) and the matching `OmniaBuildBackend`, which extends the
dispatch by chaining capability checks:

```ts
const stubDriver = new StubPhaseDriver();
const contractsDriver = new ContractsBuildPhaseDriver();
const omniaDriver = new OmniaBuildPhaseDriver();
new ScriptedExecuteBackend({
  phaseDriverFor: (entry) => {
    if (entry.name === SLICE_CONTRACT) return contractsDriver;
    if (entry.capability === "omnia") return omniaDriver;
    return stubDriver;
  },
});
```

C14b now ships `VectisBuildPhaseDriver` (deterministic Vectis
composition + SwiftUI shell emission) and the matching
`VectisBuildBackend`, which extends the dispatch the same way:

```ts
const stubDriver = new StubPhaseDriver();
const contractsDriver = new ContractsBuildPhaseDriver();
const vectisDriver = new VectisBuildPhaseDriver();
new ScriptedExecuteBackend({
  phaseDriverFor: (entry) => {
    if (entry.name === SLICE_CONTRACT) return contractsDriver;
    if (entry.capability === "vectis") return vectisDriver;
    return stubDriver;
  },
});
```

A future "real builds for everything" backend composes all three by
chaining capability checks in `phaseDriverFor`. The dispatch is
per-iteration, not per-backend, so adding a new specialist driver is
always one new class plus a new backend that wires it; no need to
touch the loop driver or the shared phase-driver helpers.

### Two Driver Shapes

- **Option (B) — Operator-manual / pre-collected results.** The
  default. Operators run `/spec:define <slice>` themselves (or replay
  a recorded SDK transcript), capture the per-slice bodies into an
  `AgentOperatorResults` JSON file, and re-invoke the runner with
  `--operator-results <path>.json`. Schema:
  [`.cursor/schemas/operator-results.schema.json`](../../../.cursor/schemas/operator-results.schema.json).
  Sample for the RM-01 fixture:
  [`acceptance/suites/rm01-cross-repo/operator-results.example.json`](../../suites/rm01-cross-repo/operator-results.example.json).
- **Option (A) — Cursor SDK programmatic invocation. Deferred.**
  Documented in `~/.cursor/skills-cursor/sdk/SKILL.md`. The SDK
  driver should re-use this backend's `prepare` hook and only swap
  the `AgentPhaseDriver` constructor input (a recorded
  `AgentOperatorResults` payload built from live agent transcripts
  rather than a hand-authored JSON file). C12 deliberately defers
  the SDK wiring so an experimental integration cannot block the
  rest of the chunk; the operator-manual path is the load-bearing
  fallback.

### Skip Policy

The agent backend prepares cleanly and returns `pending-operator`
with `evidence.extras.skipAssertions = true` when no
`--operator-results` is supplied. The smoke driver
([`smoke-cross-repo-define.ts`](../smoke-cross-repo-define.ts))
translates that to a `[c12 skip] AgentBackend requires either Cursor
SDK (--cursor-sdk) or operator results (--operator-results <path>);
skipping` message and exits 0, so CI stays green when no operator has
authored a real `/spec:define` transcript.

### Per-Slice Body Lookup

The `AgentPhaseDriver` looks up `results.slices[<slice-name>]` for
each slice the loop driver visits. Missing fields fall back to the
stub body for that artifact, so partial operator results still produce
well-formed evidence. `design: null` explicitly skips `design.md`
(useful for the contracts capability whose brief has no design step);
omitting `design` defers to the per-capability map in
[`phase-driver.ts::CAPABILITY_REQUIRES_DESIGN`](phase-driver.ts).

### Define-Stage Assertions (C12)

The seven new assertion ids the agent backend exercises live in
[`acceptance/assertions/define.ts`](../../assertions/define.ts) and
are documented as `Rule:` blocks in
[`acceptance/suites/rm01-cross-repo/expected/plan-roles.md`](../../suites/rm01-cross-repo/expected/plan-roles.md):

- `slice-has-proposal`
- `slice-has-spec`
- `slice-has-design-when-required`
- `slice-has-tasks`
- `slice-baseline-promoted`
- `slice-archived`
- `implementation-slice-reads-baseline-contract`

All seven also pass when run under the existing
`scripted-execute` / `scripted-finalize` smokes — the stub bodies
produced by `StubPhaseDriver`'s `stubBodyFactory` are valid artifact
shapes that satisfy the structural checks. This is intentional: the
assertion handlers prove they work against stub-quality artifacts
*before* the agent backend ever runs, so an operator who supplies
real `/spec:define` output can trust the same handlers will catch
artifact-body regressions.

## Agent Backend Vs Real-Agent Boundary (C12)

The `agent` backend is the C12 implementation of the **real-define
runtime**. Like the scripted backends, it exists to land assertion
plumbing end-to-end — but its define-stage outputs come from outside
the runner instead of from a deterministic stub. Two driver modes
share the same `AgentPhaseDriver` interface:

- **Operator-manual driver (B; implemented today).** The backend
  consumes a `--operator-results <path>.json` file (see schema and
  shape below). Per-slice bodies are written into the workspace
  clones as part of the `define` phase; merge / build phases reuse
  the deterministic stub primitives so no live agent is required for
  CI. This is the **reliable fallback** the smoke target uses.
- **Cursor SDK driver (A; deferred to a future amendment).** The
  same `AgentPhaseDriver` interface will accept a `CursorSdkDriver`
  that programmatically invokes `/spec:define <slice>` per slice.
  When wired up, swapping it in does not require touching the
  scenario, the assertion handlers, or the smoke target — only the
  driver instantiation in [`agent.ts`](agent.ts) changes.

The backend deliberately:

- composes `prepareScriptedHub` + `runPlanCreationSequence` from
  [`scripted-shared.ts`](scripted-shared.ts) and instantiates
  [`ScriptedFinalizeBackend`](scripted-finalize.ts) with a
  non-default `phaseDriver: AgentPhaseDriver`, so the C09/C10/C11
  shape primitives (plan creation, branch prep, baseline/residue
  commit pair, push, mark-merged, finalize, idempotency probe) all
  reuse byte-for-byte the scripted code path,
- materialises operator-supplied `proposal.md`, `spec.md`,
  `tasks.md`, optional `design.md`, and optional residue file per
  slice, falling back to the deterministic stub bodies for any
  slice the operator did not pre-populate (so a partial
  `operator-results.json` still yields a runnable scenario),
- forwards any operator-recorded assertion records into the run
  context so they merge with the runner-owned dispatcher results,
  per the standard `BackendResult.assertions[]` shape — but the
  runner's on-disk define-* handlers always re-score the live
  workspace, so a stale operator report never overrides on-disk
  truth (mirrors the manual backend's contract).

It does NOT:

- read the body of the fixture brief,
- vary slice names, residue paths, or routing based on prose,
- exercise a real forge (push / merge mechanics reuse the scripted
  backend's fake-`gh` flow),
- invoke `/spec:build` for capability-specific build outputs (C13 /
  C14a / C14b reserve that work for the build-phase hook on
  `PhaseDriver`).

A reader inspecting an `acceptance-cross-repo-define-smoke` run dir
is seeing the assertion plumbing pass against operator-replayed
artifact bodies, **not** a live `/spec:define` execution. Tightening
the proof requires the deferred SDK driver (option A) or an operator
running `/spec:define` for real and recording the bodies into
`operator-results.json` for replay. The C12 plan amendment for C13 /
C14a / C14b explicitly reserves the build-phase hook on
`PhaseDriver`; do not backfill specialist build invocations into the
agent backend's define driver.

### Operator-Results JSON Shape

The runner accepts `--operator-results <path>` for the `agent`
backend. The file is JSON; the canonical schema lives at
[`.cursor/schemas/operator-results.schema.json`](../../../.cursor/schemas/operator-results.schema.json)
and a sample populated for the RM-01 cross-repo suite lives at
[`acceptance/suites/rm01-cross-repo/operator-results.example.json`](../../suites/rm01-cross-repo/operator-results.example.json).

```jsonc
{
  "scenario": "rm01-cross-repo",         // optional id check; logged as a warning if it disagrees
  "completed": true,                     // operator's coarse pass/fail; surfaced in summary.md
  "notes": "...",                        // free-form text; surfaced in summary.md
  "slices": {                            // per-slice define-stage bodies (keys are plan-entry names)
    "<slice-name>": {
      "proposal": "# Proposal: ...",     // written to .specify/specs/<slice>/proposal.md
      "spec":     "# Spec: ...",         // written to .specify/specs/<slice>/spec.md
      "tasks":    "- [ ] ...",           // written to .specify/specs/<slice>/tasks.md
      "design":   "# Design: ..." | null, // omit / null for capability briefs that don't need design.md (e.g. contracts)
      "residue":  "// build output"       // optional file body for the residue commit (capability-specific path)
    }
  },
  "assertions": [                        // optional per-id verdicts the operator already collected
    {
      "id": "<assertion-id>",
      "verdict": "pass" | "fail" | "skip" | "pending-operator",
      "evidence": "...",                  // free-form
      "description": "..."                // free-form
    }
  ]
}
```

Per-slice rules:

- A slice missing from `slices` falls back to the deterministic
  stub body for every artifact, so a partial recording still yields
  a runnable scenario.
- `design` is `null` (or omitted) for capability briefs that do not
  produce a design.md (today: `contracts`). The capability policy
  table in [`../../assertions/define.ts`](../../assertions/define.ts)
  is the single source of truth for which slices require design.md.
- `residue` is optional. When omitted, the agent backend uses the
  deterministic stub's residue path / body so the C10/C11 commit
  shape rules continue to hold.
- The runner-owned define-* assertions always re-score on-disk
  truth — operator-recorded assertion records are merged in for
  audit purposes only, never as a substitute for verification.

## Recorded Backend (C15)

The `recorded` backend is the C15 hand-off for cheap regression
coverage. It is a strict *narrow-coverage* alternative to the
scripted-* backends: instead of re-deriving the CLI sequence from a
brief or a loop driver, it replays a frozen JSONL trace produced by a
previous trusted run. That makes it a tight pin on the underlying
`specify` CLI substrate — exit-code drift in a recorded argv is a
`cli-substrate` regression — while keeping the scripted backends as
the source of truth for "the loop logic does the right thing".

The backend does NOT:

- record live agent transcripts (that work is reserved for a future
  Cursor SDK / agent-recording change),
- diff transcript prose byte-for-byte (only argv + exit code +
  optional final-state paths),
- replace any existing smoke. Live `make acceptance-cross-repo-{plan,
  execute, finalize}-smoke` runs continue to be the source of the
  trace's correctness; a corrupted recording will pass replay
  vacuously, so periodic regeneration is part of the discipline.

### Recorded Trace Format

Every line of a trace file is a single JSON object. Three record
kinds are recognised; the parser tolerates extras for forward
compatibility.

```ts
interface RecordedTraceHeader {
  kind: "recorded-trace-header";
  schemaVersion: 1;
  sourceBackend: BackendName;     // backend that produced the trace
  sourceRunId: string;            // run dir id from the source run
  sourceTimestamp: string;        // ISO 8601
  scenarioId: string;             // scenario whose argv the trace replays
}

interface RecordedAction {
  kind:
    | "stub-action"
    | "scripted-plan-action"
    | "scripted-execute-action"
    | "scripted-finalize-action"
    | "synthetic";
  ts?: string;                    // optional ISO 8601 timestamp from the source run
  phase?: string;                 // setup | define | build | merge (when applicable)
  slice?: string;                 // slice the action targeted (when applicable)
  action?: string;                // human-readable label
  command?: string[];             // argv when the action invoked the CLI; absent for synthetic records
  cwd?: string;                   // recorded cwd; literal `<hubDir>` is substituted at replay time
  exitCode?: number;              // recorded exit code; replay must match
  artifacts?: string[];           // workspace-relative paths the source run wrote (informational)
  extras?: Record<string, unknown>; // backend-specific extensions tolerated by the parser
}

interface RecordedTraceFinalState {
  kind: "recorded-trace-final-state";
  expectedPaths: string[];        // hub-relative paths that must exist after replay
}
```

The `<hubDir>` placeholder is the only path-portability device today;
operators substituting traces by hand should always rewrite the
recorded `cwd` to `<hubDir>` so the trace is portable across temp
directories. The recorded backend will also tolerate any cwd whose
basename is the scenario's hub name (`shop-platform` for RM-01).

### Regenerating A Recorded Trace

The checked-in baseline at
[`acceptance/recorded/rm01-cross-repo/baseline.jsonl`](../../recorded/rm01-cross-repo/baseline.jsonl)
was produced from a `scripted-execute` smoke run. To regenerate (after
a CLI surface change, a backend rewrite, or a captured live-agent
session):

1. Run a trusted source smoke with `--preserve` so the run dir is
   kept on disk:
   ```bash
   SPECIFY_BIN=/path/to/specify \
     ~/.deno/bin/deno run --allow-read --allow-write --allow-env --allow-run \
       acceptance/runner/main.ts --suite rm01-cross-repo \
       --backend scripted-execute --preserve
   ```
2. Locate the run dir (`Run directory: ...` in the smoke output) and
   identify the JSONL writer files:
   - `scripted-plan-actions.jsonl` — the `specify change plan {create,
     add, status, next}` argv set,
   - `scripted-execute-loop.jsonl` — per-slice loop markers
     (synthetic, no `command`).
3. Concatenate them in order, prepending a `recorded-trace-header`
   line and appending an optional `recorded-trace-final-state` line.
   Replace the absolute hub-dir prefix in every `cwd` with the literal
   string `<hubDir>` so the trace is portable.
4. Drop a `kind:` discriminator on every line that doesn't already
   have one (header lines from the source files map to `synthetic`
   records and are book-keeping; the parser preserves them under
   `extras` for audit).
5. Save to `acceptance/recorded/<scenario-id>/<trace-id>.jsonl`.
   Convention: `baseline.jsonl` for the canonical recording; named
   alternatives (`refactor-2026-05-08.jsonl`) for staging a candidate
   replacement before swapping it in.

A regenerator script is intentionally not shipped today — the manual
loop is small enough that scripting it would add a layer of trust to
the trace file (the script could rewrite history without anyone
noticing). Operators should diff the resulting trace against the
checked-in baseline before promoting it.

### Skip Policy

The recorded backend prepares cleanly and returns
`pending-operator` with `evidence.extras.skipAssertions = true` when
no `--recorded-trace` is supplied OR when the supplied trace file is
absent. The smoke driver
([`smoke-cross-repo-recorded.ts`](../smoke-cross-repo-recorded.ts))
translates that to a `[c15 skip] ...` message and exits 0, so CI
stays green for fresh checkouts that have not yet regenerated the
baseline. The smoke driver also self-skips when `specify` is missing
or pre-RFC-9 (same policy as C09/C10/C11).

### Failure Attribution

| Scenario                                                | Verdict | Fault domain                  |
| ------------------------------------------------------- | ------- | ----------------------------- |
| Recorded 0, live non-zero                               | failed  | `cli-substrate`               |
| Recorded non-zero, live 0 (CLI got more permissive)     | failed  | `live-agent-nondeterminism`   |
| Any other exit-code delta                               | failed  | `live-agent-nondeterminism`   |
| Replay raised before producing an exit code             | failed  | `runner-setup`                |
| Trace file malformed (parse error, missing schema 1)    | failed  | `runner-setup` (in `prepare`) |
| Final-state path missing on disk                        | failed  | `cli-substrate`               |

The `recorded-trace-replays-cleanly` /
`recorded-trace-final-state-matches` /
`recorded-trace-no-extra-actions` assertion handlers in
[`acceptance/assertions/recorded.ts`](../../assertions/recorded.ts)
read replay outcomes from `RunContext.recordedEvidence` (set by the
backend's `invoke`) and self-skip cleanly when run under any other
backend. The full per-step log is persisted to
`replayed-actions.jsonl` next to `assertions.json` for `jq`/diff
review.

## Adding A Backend

1. Implement the `Backend` interface from [`../types.ts`](../types.ts) in a
   new file under this directory (`stub.ts`, `agent.ts`, `recorded.ts`).
2. Wire it into the backend lookup in [`../main.ts`](../main.ts) so
   scenarios with the matching `backend:` field route to it.
3. Add the new backend name to `BackendName` in
   [`../types.ts`](../types.ts) and the `backend.enum` in
   [`.cursor/schemas/scenario.schema.json`](../../../.cursor/schemas/scenario.schema.json).
4. If the backend produces transcript or tool-call evidence, write into
   the `transcriptMd` / `toolCallsJsonl` paths reserved on `RunPaths` so
   evidence file names stay stable across backends.
5. If the backend captures output the assertion stage should consume
   (verifier stdout, materialised paths), populate
   `BackendResult.evidence` instead of running helpers from inside the
   backend.
6. Keep lifecycle authority with the `specify` CLI. A backend may seed
   scenario *inputs* (briefs, fixture source trees, fake `gh` config),
   but must not hand-edit `.specify/` files.

The interface is intentionally minimal. If a backend needs more from the
runner core, raise it as a plan amendment rather than expanding the
interface ad hoc.
