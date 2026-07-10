# Running Evals

This is the single entry point for Specify evals. It defines what the eval sweep is, how an operator (or agent) runs it, the group ordering and halt gate, and the green-gate signal. The eval sweep is agent-based; specify's own Rust/deterministic surface (`cargo make ci`, including the framework-quality tests) runs separately in CI on every commit and is **not** part of running evals.

The scenario catalog — the canonical list of every scenario, its grouping, release-blocker status, and run status — lives in [`evals/scenarios/README.md`](../../evals/scenarios/README.md). This document does not duplicate that table.

## What the eval sweep is

The eval sweep is the **agent-based** proof surface. The operator-driven scenarios in [`evals/scenarios/`](../../evals/scenarios/) exercise the full `/spec:plan` → Gate 1 → `specify plan execute` → `/spec:finalize` rhythm against live `cursor-agent`, plus the per-target generated-output-correctness gate. A schema-valid `build/report.yaml` with `status: success` proves the build envelope held, **not** that the generated code compiles or replays — so each exercised target must also pass `cargo check` / `cargo test` / its replay suite (and the equivalent verification for non-Rust targets). A slice whose generated output fails these checks is not done, regardless of envelope validity.

Specify's own deterministic surface — `cargo make ci`, including the framework-quality and Rust-quality test suites — is a **separate** gate that runs in CI on every commit. It is not run, re-run, or reported as part of the eval sweep.

The eval sweep is intentionally **not** an automated harness: no runner, fake forge, recorded transcript, CI target, or golden-output comparison. That posture is encoded as the `negative-expectations` frontmatter on every scenario and is the one place this rationale is stated — individual scenarios do not repeat it in prose. It is operator-driven because it judges LLM-emitted output; the deterministic test suites do not pin synthesised bytes. The negative expectations constrain *driving*; **grading** is mechanical where it can be — the post-run probes in the [assertion taxonomy](../../evals/shared/assertions.md) violate none of them.

Multi-step execute scenarios also ship optional replay helpers under [`evals/drivers/`](../../evals/drivers/README.md): checked-in operator scripts that shell out to the real CLI (driver mutual exclusion is guest-owned via the `.specify/guest.lock` marker). They are not wired into CI and do not change the negative-expectation posture — they replace ad-hoc copies under the gitignored sandbox, not unattended automation.

**What makes something an eval scenario.** A behavior earns a catalog entry only when at least one assertion is irreducible to deterministic CLI/host behavior. Three admission categories, each with the deterministic substrate that *is* covered separately in the Rust workspace named alongside:

1. **LLM-prose judgment** — whether a synthesized `spec.md` / `design.md` reads correctly, or a plan decomposes a brief sensibly (e.g. `intent-only`, `documentation-one-slice`, `single-project-plan`, `target-shape`, `lead-reconciliation`). The CLI substrate — provenance/`Sources:` rendering, `[conflict]`/`[divergence]` tagging, propose routing, the embedded guidance prompt in the synthesis envelope — carries its deterministic coverage in the crate-level suites (`crates/argv/tests/`, `crates/workflow/tests/`).
2. **Skill-loop orchestration** — the `specify plan execute` stop / resume / drained behavior (e.g. `execute-fail-resume`, `execute-pause-resume`, `workspace-stale-recovery`, `workspace-fail-resume`). The per-step primitives — build-finalize gating, `plan next` advance, `slice merge` stamping `done`, `workspace sync` dirty-slot detection — are covered in `crates/workflow/tests/`; the loop that sequences them is not.
3. **Cross-repo finalize orchestration** — `/spec:finalize` sequencing `specify workspace push` (publishing each routed project's `specify/<change>` branch to its `origin`) and `specify plan archive` in one run across multiple project remotes (e.g. `contract-lifecycle`). Pull requests are opened and merged by the operator outside Specify, so no forge client is exercised; the scenario runs against local bare-repo remotes. (Dual-driving refusal is deterministic — the create-exclusive `.specify/guest.lock` marker — and is covered by a named test in [`crates/workflow/tests/execute.rs`](../../crates/workflow/tests/execute.rs), not a scenario.)

This is the boundary: a behavior whose every assertion falls outside these three categories is **not an eval scenario at all** — it is a named deterministic test in a crate's `tests/`, run under `cargo make test` on every commit, and it never gets a catalog entry here. Re-confirm against this list before adding a scenario to the catalog.

## Running the eval sweep

The sweep needs the binary under test on your PATH. `make install-cli` builds one (from the in-tree workspace) and symlinks it into `~/.local/bin` (overridable with `INSTALL_DIR=`), so the bare `specify` commands in the scenarios resolve to this binary — it warns if that directory is not on your `PATH`. To build without `make`, run `cargo build --release --bin specify` and symlink `target/release/specify` into a PATH directory yourself; confirm it with `specify --version`.

For each scenario:

1. Open the scenario file under [`evals/scenarios/<id>.md`](../../evals/scenarios/) — each is self-contained (intent, setup, invocation, assertions).
2. Bring up a fresh disposable environment per the scenario's **Setup** (common steps factored into [`evals/shared/setup.md`](../../evals/shared/setup.md)).
3. Run the scenario's **Invocation** exactly as written, stamping Gate 1 yourself (`specify plan transition <name> approved`) — the skills never auto-stamp.
4. Grade each **Assertion** through the [assertion taxonomy](../../evals/shared/assertions.md): run the probes, judge the flagged residue with an evidence pointer (durable structure only — never a byte/golden compare).
5. Record the run with [`evals/shared/run-template.md`](../../evals/shared/run-template.md), filed as [`evals/runs/<id>.<result>.md`](../../evals/runs/README.md), and update the scenario's status in the [catalog](../../evals/scenarios/README.md).

Operators who prefer an agent to do the clerical work can paste the reusable prompts in [`evals/shared/prompts.md`](../../evals/shared/prompts.md) into a live `cursor-agent` session.

## Agent runbook — "run specify's evals"

When asked to "run specify's evals and report any issues", an agent should follow this exact sequence. The eval sweep has irreducible human seams, so the agent reports it as a per-scenario table that may include "paused — needs you" rows.

1. **Build the binary under test.** Run `make install-cli` to build + symlink the binary under test. (Specify's own Rust tests are a separate CI gate, run on every commit — not part of running evals.) This step needs no human input. Then make the build under test resolvable in the agent's own shells: run `specify --version` and, if the bare command does not resolve to the freshly built binary, prepend the symlink dir to `PATH` for the rest of the sweep (`export PATH="$HOME/.local/bin:$PATH"`, matching `make install-cli`'s `INSTALL_DIR`) or fall back to the absolute `target/release/specify` path. Re-confirm with `specify --version` before driving any scenario — a Makefile recipe cannot mutate the agent's shell `PATH`, so the agent owns this self-heal.
2. **Eval sweep — per scenario, in group order** (see [catalog](../../evals/scenarios/README.md)):
   - Drive setup with [`shared/prompts.md`](../../evals/shared/prompts.md) Prompt A, then the lifecycle with Prompt B.
   - Grade through the [assertion taxonomy](../../evals/shared/assertions.md): run each assertion's **probe** and record its verdict from the probe output; for **judgment-flagged** assertions, judge with an evidence pointer or mark `needs-human`. Record negative-expectations as held/violated per scenario.
3. **Stop and hand back to the operator** at the irreducible human seams — never fabricate a result for these:
   - A `specify` build that cannot be produced. The agent builds and resolves the binary itself in step 1 (`make install-cli` plus the `PATH` self-heal / absolute-path fallback); it hands back only when the build itself fails — e.g. the in-tree workspace does not compile.
   - Opening and merging pull requests (operator-owned, outside Specify); `/spec:finalize` pushes branches and archives but never touches PRs.
   - Ergonomics / judgment assertions the agent cannot deterministically verify — mark `needs-human`.
   - `deferred` entries and scenario #1 sign-off (release-blocker; see halt rule below).

### Running a single scenario

When asked to run one named scenario (e.g. "run Specify's eval `intent-only`"), the agent follows the same runbook scoped to that id: do step 1 (install + resolve the binary under test), then drive **only** that scenario's id through [`shared/prompts.md`](../../evals/shared/prompts.md) Prompt A → Prompt B, grade via the [assertion taxonomy](../../evals/shared/assertions.md) probes (judging the flagged residue), file the run-summary under [`evals/runs/`](../../evals/runs/README.md), and report. Skip the group ordering — it governs the full-catalog sweep, not a single run. The same human seams (step 3) still apply; in particular `intent-only` is the N=1 release blocker and carries the hard-halt + release-owner sign-off seam.

## Execution order and the halt gate

The catalog is drained in groups. Each run fills a run-summary and flips the scenario's catalog status to `passed` / `failed` / `deferred`.

1. **N=1 hard halt — release blocker.** Scenario `intent-only` (N=1). **Hard halt:** if it fails, record the failure, do not run any other scenario, triage, then resume once green. No later scenario is meaningful while it is red.
2. **Core synthesis + routing.** The happy-path planning, multi-slice, cross-source merge, and cross-repo contract scenarios.
3. **Failure and breakout paths.** The negative, recovery, and breakout scenarios.

Within a group, scenarios are independent and may run in any order; a failure outside the `intent-only` hard halt is recorded and triaged but does not halt sibling runs.

## The gate signal

- Each run commits its filled run-summary under [`evals/runs/`](../../evals/runs/README.md) as the audit trail.
- On failure, preserve the workspace state, `plan.yaml`, `registry.yaml`, push/finalize output, and branch/PR identifiers per the template. The sandbox at `evals/.sandbox/<scenario>/` ([`setup.md`](../../evals/shared/setup.md)) is stable and gitignored, so it survives for inspection; paste trimmed failure output into the run-summary's **Failure detail** section and point **Evidence** at the retained sandbox (see [`inspect.md`](../../evals/shared/inspect.md)). File a follow-up issue in `augentic/specify` linked back to the run-summary.
- The gate is **tiered by the catalog's Gate column**. The **release gate is green** when every `release-blocker` row (`intent-only`, `execute-fail-resume`, `workspace-two-projects`) is `passed` — the `intent-only` hard halt is unchanged. (Specify's deterministic surface, `cargo make test`, is a separate CI gate that runs on every commit.) The **full catalog** drains per minor release or monthly, whichever comes first; a non-blocking `failed` row is triaged via its linked follow-up issue but does not hold a release on its own. A `parked` row (no owner) sits outside the drain expectation until someone claims it and flips it back to `pending`. A `deferred` entry (capability genuinely missing on the binary under test) must carry a linked follow-up issue and explicit release-owner sign-off.

When the whole catalog is `passed` (or `deferred` with sign-off), record the gate as green in the [catalog](../../evals/scenarios/README.md).

## What the scenarios prove

The scenario pack proves the operator-facing `/spec:*` change lifecycle end-to-end across the full difficulty range — N=1 trivial through multi-repo, happy-path through failure and recovery. Highlights:

- **N=1 and single-project planning** (`intent-only`, `documentation-one-slice`, `single-project-plan`): degenerate `intent` / `documentation` survey, Gate-1 ergonomics, `Sources:` provenance, plans that stop at `pending` and print the literal Gate-1 transition command.
- **Synthesis and reconciliation** (`documentation-multi-slice`, `typescript-multi-slice`, `lead-reconciliation`, `target-shape`): multi-slice decomposition, deterministic cross-source reconciliation, guidance-prompt injection, lifecycle reaching `refined` cleanly.
- **Cross-repo routing** (`contract-lifecycle`, `workspace-two-projects`): contract-first plans, registry-driven routing, workspace slot materialisation, durable end-state (archived plan path, one pushed branch per routed project, archived `change.md`).
- **Failure and breakout** (`execute-pause-resume`, `execute-fail-resume`, `workspace-fail-resume`, `workspace-stale-recovery`): build-failure stop/resume, breakout verbs, and stale-slot recovery. Dual-driving refusal is CLI-enforced (the guest execute marker, `.specify/guest.lock`) and proven by the named test in `crates/workflow/tests/execute.rs`, not a scenario.

The deterministic substrate beneath these — reconciliation tagging, authority resolution, plan amendment, extract failure modes, sandbox enforcement — is proven by named tests in the Rust workspace via `cargo make test`, not by scenarios here.

## Fan-in / fan-out proof

The cross-source fan-in / cross-slice fan-out proof on the eval side is the **generated-output-correctness gate**: each exercised target build must pass the target's own replay/golden suite plus `cargo check` / `cargo test` for generated crates (and the equivalent verification for non-Rust targets). A slice whose generated output fails these checks is not done, regardless of build-envelope validity. (The deterministic envelope/ordering proof for the same path lives with the crate-level suites under `cargo make test`; the wasm-only seams stay with the shipped guest and targeted adapter tests, outside the eval sweep.)

## Synthesis byte-replay (deferred)

The framework test harness covers checker regressions and repo consistency, but does **not** assert on the bytes a `/spec:refine` or `/spec:build` skill body emits. A byte-equivalent "synthesis golden" requires either a recorded-transcript layer (capture a `cursor-agent` run via `@cursor/sdk` and replay it) or a structured-trace assertion library (compare the *shape* of synthesised artifacts rather than the bytes). Both are out of scope for now; a follow-up RFC will pick one. Until then, the eval sweep is the source of truth for end-to-end LLM-driven correctness.
