# Acceptance Runner

> Status: Contract docs only. The runner is not implemented yet; this directory anchors the layout that follow-up changes in the [implementation plan](../../rfcs/rm-01-acceptance-framework-implementation-plan.md) will fill in.

The runner is the shared execution surface for every acceptance suite in this repo. It prepares isolated workspaces, invokes a backend, captures evidence, and hands final state to assertion modules. It is **not** the test oracle — assertions live next door under [`../assertions/`](../assertions/README.md).

A future runner entrypoint is expected to land at `acceptance/runner/main.ts` (Deno TypeScript), invoked via `make acceptance-smoke` and `make acceptance-cross-repo` once the underlying suites can run reliably. See [`../README.md`](../README.md#command-tiers) for the command tier policy.

## Responsibilities

When a suite runs, the runner is responsible for:

- creating a temp project root outside the repo tree (under the OS temp directory by default),
- initialising hub or project `.specify/` state through the `specify` CLI,
- creating fixture source documents and, where the suite needs them, fixture project repositories with local bare remotes,
- installing fake `gh` and other forge fakes when the suite tests forge handoff,
- discovering the scenario file (see scenario discovery in [`../README.md`](../README.md#scenario-discovery)),
- invoking the chosen backend with the scenario's invocation,
- collecting stdout, stderr, transcript, tool calls, file tree, relevant JSON command output, and final Git state,
- running the assertion modules requested by the scenario,
- writing a compact summary plus per-suite evidence files.

The runner must not hand-edit `.specify/` lifecycle state to drive scenarios. It may seed scenario *inputs* (briefs, source trees, registry shape via `specify registry *`, fake-`gh` config). Lifecycle transitions still go through the CLI. See the [CLI-Authoritative Invariant](../README.md#cli-authoritative-invariant) for the full rule.

## Backend Interface

Every suite picks one backend. The backend interface is documented as part of the runner skeleton change in the plan; this README only fixes the backend taxonomy so suites and assertion modules can reference it consistently.

- **Manual backend.** Documentation-only: the runner prints the next operator action (or hands the scenario to a human/agent following the prose) and records reported results. This is the current contracts harness behavior, lifted into the runner so its evidence shape matches automated runs.
- **Deterministic stub backend.** Performs known phase effects without invoking a live skill. Useful for `/change:execute loop` coverage where the goal is to prove plan transitions, route selection, branch preparation, and evidence capture without paying generation cost. Stubbed phases must be declared in scenario metadata so the run summary can record what was deterministic.
- **Agent runtime backend.** Invokes slash-command workflows (`/change:plan`, `/change:execute loop`, `/spec:define`, `/spec:build`, `/spec:merge`) through a pinned agent runtime when programmatic execution is available. Live runs assert structural outcomes only; they must not compare full transcripts byte-for-byte.
- **Recorded transcript backend.** Replays known tool decisions from a previously trusted live run, asserting stable tool-call intent and final structural state. Complementary coverage, not a replacement for periodic live outside-in runs.

A scenario declares its backend through scenario metadata (the `backend:` frontmatter field once frontmatter is adopted; until then, by convention in the scenario's `Workspace` section).

## Run Directories And Evidence

Each run gets its own directory under a temp root, never inside the repo tree. The exact filenames are owned by the suite that emits them, but every run is expected to write at least the shared shape from [`../README.md`](../README.md#run-evidence-policy):

- `summary.md` — short human-readable run summary, ending in a pass/fail verdict and a fault-domain hint on failure.
- `scenario.md` — the executed scenario, copied verbatim so the run is self-describing.
- `assertions.json` — structured pass/fail per assertion, suitable for tooling.
- `stdout.log`, `stderr.log` — raw process output.
- `final-tree.txt` — recursive listing of the temp project root after the run, minus large binary or generated noise.

Backends may add their own files (`transcript.md` for agent runs, `tool-calls.jsonl` for recorded replays). RM-01-shaped suites add multi-repo evidence (registry, plan, workspace status, push/finalize JSON, hub and project Git logs, fake forge state). Each suite documents its own evidence shape in its `README.md`.

By default the run directory is preserved only on failure. An explicit `--preserve` flag preserves all runs. Suites must not commit run output. See the [Run Evidence Policy](../README.md#run-evidence-policy) for the full retention rules.

## Failure Reporting

The runner is the layer best placed to give a maintainer the fault-domain hint that distinguishes substrate drift from skill drift. A failure summary should classify the likely fault domain — at least one of:

- CLI substrate (a `specify` verb returned an unexpected status or output),
- skill orchestration (a slash command did not invoke the expected CLI verbs in the expected order),
- capability brief (a brief failed to write the artifacts the scenario asserted),
- specialist generation (an Omnia/Vectis/contracts generator produced unexpected structure),
- runner setup (temp hub, registry, workspace, or fake forge fixtures failed before the scenario ran),
- external fake boundary (fake `gh` or fake remotes returned an unexpected state),
- live-agent nondeterminism (an agent backend produced output that does not match a structural assertion but does not indicate orchestration drift).

Suites are encouraged to surface as much of this as their assertion vocabulary supports. The runner should at minimum record which phase emitted the failing assertion and which backend executed.

## Out Of Scope For This Directory

- The runner does not own scenario validation; that lives in `scripts/checks.ts` as opt-in checks added by a follow-up change in the plan.
- The runner does not own assertion logic; structural assertions and their helpers live under [`../assertions/`](../assertions/README.md).
- The runner does not provide a Specify lifecycle implementation. It calls `specify` and reads back state.
- The runner does not embed forge logic. Fake-forge support is a thin shim modeled on the strategy already used in `specify-cli/tests/cross_repo.rs`; the real forge is never touched by an acceptance run.
