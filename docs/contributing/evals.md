# Running Evals

This is the single entry point for Specify evals. It defines the two proof surfaces, how an operator (or agent) runs the eval sweep, the group ordering and halt gate, and the green-gate signal.

The scenario catalog — the canonical list of every scenario, its grouping, release-blocker status, and run status — lives in [`evals/scenarios/README.md`](../../evals/scenarios/README.md). This document does not duplicate that table.

## The two proof surfaces

A release is proven only when **both** surfaces are green:

1. **Deterministic proof.** `cargo make test` in [`augentic/specify-cli`](https://github.com/augentic/specify-cli) (including [`tests/plan/end_to_end.rs`](https://github.com/augentic/specify-cli/blob/main/tests/plan/end_to_end.rs)) asserts the envelope, ordering, and re-projection determinism of the whole CLI path: `source survey` → `plan propose --dry-run | --from` → per-slice `source extract` → `slice synthesize` → `slice build` → `slice merge`, plus `depends-on` ordering and byte-identical kernel re-projection. It does **not** execute real target codegen. Plus the static repository checks: `make lint` runs `specify lint framework` against the live tree (skill frontmatter, adapter manifests, rule shape, links, marketplace consistency, scenario frontmatter).
2. **Eval sweep.** The operator-driven scenarios in [`evals/scenarios/`](../../evals/scenarios/) exercise the full `/spec:plan` → Gate 1 → `/spec:execute` → `/spec:finalize` rhythm against live `cursor-agent`, plus the per-target generated-output-correctness gate. A schema-valid `build/report.yaml` with `status: success` proves the build envelope held, **not** that the generated code compiles or replays — so each exercised target must also pass `cargo check` / `cargo test` / its replay suite (and the equivalent verification for non-Rust targets). A slice whose generated output fails these checks is not done, regardless of envelope validity.

The eval sweep is intentionally **not** an automated harness: no runner, fake forge, recorded transcript, CI target, or golden-output comparison. That posture is encoded as the `negative-expectations` frontmatter on every scenario and is the one place this rationale is stated — individual scenarios do not repeat it in prose. It is operator-driven because it judges LLM-emitted output; `specify lint framework` does not pin synthesised bytes. The negative expectations constrain *driving*; **grading** is mechanical where it can be — the post-run probes in the [assertion taxonomy](../../evals/shared/assertions.md) violate none of them.

**What makes something an eval scenario.** A behavior earns a catalog entry only when at least one assertion is irreducible to deterministic CLI/host behavior. Three admission categories, each with the deterministic substrate that *is* covered by surface 1 named alongside:

1. **LLM-prose judgment** — whether a synthesized `spec.md` / `design.md` reads correctly, or a plan decomposes a brief sensibly (e.g. `intent-only`, `documentation-one-slice`, `single-project-plan`, `target-shape`, `lead-reconciliation`). The CLI substrate — provenance/`Sources:` rendering, `[conflict]`/`[divergence]` tagging, propose routing, the embedded `shape` brief in the synthesis envelope — is covered by `tests/slice/synthesize.rs`, `tests/workflow/`, and `tests/plan/end_to_end.rs`.
2. **Skill-loop orchestration** — the `/spec:execute` stop / resume / `all-done` behavior emitted by skill markdown, not by any single CLI verb (e.g. `execute-fail-resume`, `execute-pause-resume`, `workspace-stale-recovery`, `workspace-fail-resume`). The per-step primitives — build-finalize gating (`tests/slice/build.rs`), `plan next` advance, `slice merge` stamping `done`, `workspace sync` dirty-slot detection (`tests/workspace.rs`) — are covered; the loop that sequences them is not.
3. **Cross-repo finalize orchestration** — `/spec:finalize` sequencing `specify workspace push` (publishing each routed project's `specify/<change>` branch to its `origin`) and `specify plan archive` in one run across multiple project remotes (e.g. `contract-lifecycle`). Pull requests are opened and merged by the operator outside Specify, so no forge client is exercised; the scenario runs against local bare-repo remotes. (Dual-driving refusal used to sit here as a skill-enforced pre-flight; the CLI's plan-lock probe made it deterministic, so it is now the named test `tests/workflow/plan_lock.rs` in `specify-cli` with no catalog entry.)

This is the boundary: a behavior whose every assertion falls outside these three categories is **not an eval scenario at all** — it is a named deterministic test in `augentic/specify-cli`, run under `cargo make test` on every commit, and it never gets a catalog entry here. Re-confirm against this list before adding a scenario to the catalog.

## Running the deterministic surface

```bash
make lint          # static repository checks; builds the pinned specify-cli source
make install-cli    # builds the resolved cli source, runs nothing else, then symlinks specify onto your PATH for the sweep
```

Both targets resolve their `specify` source the same way — through [`scripts/specify.rs`](../../scripts/specify.rs), which reads the `cli` source spec from [`Specify.toml`](../../Specify.toml) (or a gitignored `Specify.local.toml` overlay) and **builds** it (see [Consistency Checks — binding model](checks.md#binding-to-a-specify-source)). Both forms build from source; no published binary is downloaded. `make install-cli` runs the resolver with `--install` (materializing `.cli/bin/specify`), then symlinks that onto your PATH.

`make install-cli` **prepares the eval sweep**: it builds the resolved `cli` source, materializes `.cli/bin/specify`, and symlinks it into `~/.local/bin` (overridable with `INSTALL_DIR=`, warning if it is not on your `PATH`), then points at the eval sweep below. It does **not** re-run the deterministic tests — `cargo make test` in `specify-cli` is the single authoritative deterministic surface (including the wasm-tool suites), and it runs there on every commit, so re-running the `plan` / `source` / `slice` / `workspace` test binaries from this repo would only duplicate that work. `make install-cli` does not run, fake, record, or golden-compare the scenario pack, and it is deliberately **not** wired into CI, so it is not a required CI check — every scenario's `negative-expectation` stays held.

Run `specify lint framework` from the repo root, or pass `--framework-root` / set `SPECIFY_ROOT` when invoking directly from another cwd or checkout. To run the predicate regression suite, use `cargo make test` from a `specify-cli` checkout.

## Running the eval sweep

The sweep needs the binary under test on your PATH. `make install-cli` materializes one (from the resolved `cli` source) and symlinks it into `~/.local/bin` (overridable with `INSTALL_DIR=`), so the bare `specify` commands in the scenarios resolve to this binary — it warns if that directory is not on your `PATH`. To build without `make`, run `cargo build --release --manifest-path ../specify-cli/Cargo.toml --bin specify` and symlink `../specify-cli/target/release/specify` into a PATH directory yourself; confirm it with `specify --version`.

For each scenario:

1. Open the scenario file under [`evals/scenarios/<id>.md`](../../evals/scenarios/) — each is self-contained (intent, setup, invocation, assertions).
2. Bring up a fresh disposable environment per the scenario's **Setup** (common steps factored into [`evals/shared/setup.md`](../../evals/shared/setup.md)).
3. Run the scenario's **Invocation** exactly as written, stamping Gate 1 yourself (`specify plan transition <name> approved`) — the skills never auto-stamp.
4. Grade each **Assertion** through the [assertion taxonomy](../../evals/shared/assertions.md): run the probes, judge the flagged residue with an evidence pointer (durable structure only — never a byte/golden compare).
5. Record the run with [`evals/shared/run-template.md`](../../evals/shared/run-template.md), filed as [`evals/runs/<id>.<result>.md`](../../evals/runs/README.md), and update the scenario's status in the [catalog](../../evals/scenarios/README.md).

Operators who prefer an agent to do the clerical work can paste the reusable prompts in [`evals/shared/prompts.md`](../../evals/shared/prompts.md) into a live `cursor-agent` session.

## Agent runbook — "run specify's evals"

When asked to "run specify's evals and report any issues", an agent should follow this exact sequence. The proof surface is two-tier, and the eval tier has irreducible human seams, so the agent reports the deterministic surface as a clean pass/fail and the eval sweep as a per-scenario table that may include "paused — needs you" rows.

1. **Deterministic surface.** Run `make lint` (builds the pinned `cli` source and runs the framework checks) and report pass/fail with the failing finding ids, then run `make install-cli` to build + symlink the binary under test. The deterministic tests (`plan`, `source`, `slice`, `workspace`, and the wasm-tool suites) are owned by `specify-cli` and run there on every commit; run `cargo make test` in the `specify-cli` checkout when you need to prove the full deterministic surface locally. This step needs no human input. Then make the build under test resolvable in the agent's own shells: run `specify --version` and, if the bare command does not resolve to the freshly built binary, prepend the symlink dir to `PATH` for the rest of the sweep (`export PATH="$HOME/.local/bin:$PATH"`, matching `make install-cli`'s `INSTALL_DIR`) or fall back to the absolute `../specify-cli/target/release/specify` path. Re-confirm with `specify --version` before driving any scenario — a Makefile recipe cannot mutate the agent's shell `PATH`, so the agent owns this self-heal.
2. **Eval sweep — per scenario, in group order** (see [catalog](../../evals/scenarios/README.md)):
   - Drive setup with [`shared/prompts.md`](../../evals/shared/prompts.md) Prompt A, then the lifecycle with Prompt B.
   - Grade through the [assertion taxonomy](../../evals/shared/assertions.md): run each assertion's **probe** and record its verdict from the probe output; for **judgment-flagged** assertions, judge with an evidence pointer or mark `needs-human`. Record negative-expectations as held/violated per scenario.
3. **Stop and hand back to the operator** at the irreducible human seams — never fabricate a result for these:
   - A `specify` build that cannot be produced. The agent builds and resolves the binary itself in step 1 (`make install-cli` plus the `PATH` self-heal / absolute-path fallback); it hands back only when the build itself fails — e.g. the sibling `specify-cli` checkout is missing or does not compile.
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
- On failure, preserve the workspace state, `plan.yaml`, `registry.yaml`, push/finalize output, and branch/PR identifiers per the template. The sandbox at `evals/.sandbox/<scenario>/` ([`setup.md`](../../evals/shared/setup.md)) is stable and gitignored, so it survives for inspection; paste trimmed failure output into the run-summary's **Failure detail** section and point **Evidence** at `scripts/snapshot.sh "$SANDBOX"`. File a follow-up issue in `augentic/specify` linked back to the run-summary.
- The gate is **tiered by the catalog's Gate column**. The **release gate is green** when `cargo make test` is green in `specify-cli` (it runs there on every commit) and every `release-blocker` row (`intent-only`, `execute-fail-resume`, `workspace-two-projects`) is `passed` — the `intent-only` hard halt is unchanged. The **full catalog** drains per minor release or monthly, whichever comes first; a non-blocking `failed` row is triaged via its linked follow-up issue but does not hold a release on its own. A `parked` row (no owner) sits outside the drain expectation until someone claims it and flips it back to `pending`. A `deferred` entry (capability genuinely missing on the binary under test) must carry a linked follow-up issue and explicit release-owner sign-off.

When the whole catalog is `passed` (or `deferred` with sign-off), record the gate as green in the [catalog](../../evals/scenarios/README.md).

## What the scenarios prove

The scenario pack proves the operator-facing `/spec:*` change lifecycle end-to-end across the full difficulty range — N=1 trivial through multi-repo, happy-path through failure and recovery. Highlights:

- **N=1 and single-project planning** (`intent-only`, `documentation-one-slice`, `single-project-plan`): degenerate `intent` / `documentation` survey, Gate-1 ergonomics, `Sources:` provenance, plans that stop at `pending` and print the literal Gate-1 transition command.
- **Synthesis and reconciliation** (`documentation-multi-slice`, `typescript-multi-slice`, `lead-reconciliation`, `target-shape`): multi-slice decomposition, deterministic cross-source reconciliation, `shape`-brief injection, lifecycle reaching `refined` cleanly.
- **Cross-repo routing** (`contract-lifecycle`, `workspace-two-projects`): contract-first plans, registry-driven routing, workspace slot materialisation, durable end-state (archived plan path, one pushed branch per routed project, archived `change.md`).
- **Failure and breakout** (`execute-pause-resume`, `execute-fail-resume`, `workspace-fail-resume`, `workspace-stale-recovery`): build-failure stop/resume, breakout verbs, and stale-slot recovery. Dual-driving refusal is CLI-enforced (the plan-lock probe) and proven by the named test `tests/workflow/plan_lock.rs`, not a scenario.

The deterministic substrate beneath these — reconciliation tagging, authority resolution, plan amendment, extract failure modes, sandbox enforcement — is proven by named tests in `augentic/specify-cli` under `cargo make test`, not by scenarios here.

## Fan-in / fan-out proof

The cross-source fan-in / cross-slice fan-out proof splits across the two surfaces above, and **both** must pass before a release is complete:

1. **Deterministic CLI proof.** [`tests/plan/end_to_end.rs`](https://github.com/augentic/specify-cli/blob/main/tests/plan/end_to_end.rs) in `augentic/specify-cli` runs under `cargo make test` and asserts the envelope, ordering, and determinism of the whole path. It does not execute real target codegen.
2. **Generated-output-correctness release gate (manual / CI).** Each target build must pass the target's own replay/golden suite plus `cargo check` / `cargo test` for generated crates (and the equivalent verification for non-Rust targets). A slice whose generated output fails these checks is not done, regardless of build-envelope validity.

## Synthesis byte-replay (deferred)

The `specify-standards` harness covers checker regressions and repo consistency, but does **not** assert on the bytes a `/spec:refine` or `/spec:build` skill body emits. A byte-equivalent "synthesis golden" requires either a recorded-transcript layer (capture a `cursor-agent` run via `@cursor/sdk` and replay it) or a structured-trace assertion library (compare the *shape* of synthesised artifacts rather than the bytes). Both are out of scope for now; a follow-up RFC will pick one. Until then, the eval sweep is the source of truth for end-to-end LLM-driven correctness.
