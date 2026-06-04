# Running Acceptance

This is the single entry point for Specify acceptance. It defines the two acceptance surfaces, how an operator (or agent) runs them, the wave ordering and halt gate, and the green-gate signal.

The scenario catalog — the canonical list of every scenario, its wave, release-blocker status, and run status — lives in [`acceptance/lifecycle/README.md`](../../acceptance/lifecycle/README.md). This document does not duplicate that table.

## The two acceptance surfaces

A release is proven only when **both** surfaces are green:

1. **Deterministic CLI proof — automated.** `cargo make test` in [`augentic/specify-cli`](https://github.com/augentic/specify-cli) (including [`tests/fan_in_fan_out.rs`](https://github.com/augentic/specify-cli/blob/main/tests/fan_in_fan_out.rs)) asserts the envelope, ordering, and re-projection determinism of the whole CLI path: `source survey` → `plan propose --dry-run | --from` → per-slice `source extract` → `slice synthesize` → `slice build` → `slice merge`, plus `depends-on` ordering and byte-identical kernel re-projection. It does **not** execute real target codegen. Plus the static repository checks: `make lint` runs `specify lint framework --framework-root .` against the live tree (skill frontmatter, adapter manifests, rule shape, links, marketplace consistency, scenario frontmatter).
2. **Operator scenario sweep — manual.** The `lifecycle` scenarios in [`acceptance/lifecycle/`](../../acceptance/lifecycle/) exercise the full `/spec:plan` → Gate 1 → `/spec:execute` → `/spec:finalize` rhythm against live `cursor-agent`, plus the per-target generated-output-correctness gate. A schema-valid `build/report.yaml` with `status: success` proves the build envelope held, **not** that the generated code compiles or replays — so each exercised target must also pass `cargo check` / `cargo test` / its replay suite (and the equivalent verification for non-Rust targets). A slice whose generated output fails these checks is not done, regardless of envelope validity.

The scenario sweep is intentionally **not** an automated harness: no runner, fake forge, recorded transcript, CI target, or golden-output comparison. That posture is encoded as the `negative-expectations` frontmatter on every scenario and is the one place this rationale is stated — individual scenarios do not repeat it in prose. It remains manual because it involves LLM-emitted prose; `specify lint framework` does not pin synthesised bytes.

**Fixture-backed scenarios are the exception.** A scenario whose assertions are entirely deterministic CLI/host behavior — no LLM prose to judge — is promoted to `backend: fixture` and proven by a named test in surface 1, not the manual sweep. Its scenario file carries an **Automated coverage** section naming the test (e.g. `source-sandbox-denied` → `tests/source_extract.rs::sandbox_denies_out_of_scope`), and its catalog status is `automated`. The manual sweep skips these; the deterministic surface covers them on every commit. The `negative-expectations` (no runner / no CI target) apply only to the manual-prose scenarios, so a fixture-backed scenario drops them.

**What keeps a scenario manual.** A scenario stays on the manual sweep when at least one assertion is irreducible to deterministic CLI/host behavior. Three categories, each with the deterministic substrate that *is* covered by surface 1 named alongside:

1. **LLM-prose judgment** — whether a synthesized `spec.md` / `design.md` reads correctly, or a plan decomposes a brief sensibly (e.g. `pure-intent`, `documentation-one-slice`, `plan-single-project`, `target-shape-injection`, `cross-source-merge`). The CLI substrate — provenance/`Sources:` rendering, `[conflict]`/`[divergence]` tagging, propose routing, the embedded `shape` brief in the synthesis envelope — is covered by `tests/slice/synthesize.rs`, `tests/plan_orchestrate/`, and `tests/fan_in_fan_out.rs`.
2. **Skill-loop orchestration** — the `/spec:execute` stop / resume / `all-done` behavior emitted by skill markdown, not by any single CLI verb (e.g. `execute-build-failure`, `stepthrough-breakout`, `stale-workspace-recovery`, `workspace-breakout`). The per-step primitives — build-finalize gating (`tests/slice_build.rs`), `plan next` advance, `slice merge` stamping `done`, `workspace sync` dirty-slot detection (`tests/workspace.rs`) — are covered; the loop that sequences them is not.
3. **Live-forge interaction** — real PR pushes/merges between `/spec:finalize` invocations (e.g. `cross-repo-contract-flow`, `workspace-execute-two-projects`), and skill-enforced pre-flight refusals (`dual-driving-refused`). No deterministic fixture stands in for a live forge or a live `cursor-agent` pre-flight.

This is the boundary: a scenario is promotable to `backend: fixture` only when *every* assertion falls outside these three categories. Re-confirm against this list before adding a scenario to the manual sweep — if all its assertions are deterministic, it belongs in surface 1 instead.

## Running the automated surface

```bash
make lint          # static repository checks (links, scenario frontmatter, skill/adapter/rule shape)
make acceptance    # builds the release binary, runs make lint + the fixture-backed acceptance tests, then prints the PATH export line for the sweep
```

`make acceptance` covers the **deterministic surface only**. It builds the release binary, runs `make lint` plus the fixture-backed acceptance tests against the sibling `specify-cli` checkout — `fan_in_fan_out`, `source_extract`, `slice`, `plan_orchestrate`, and `workspace`, which carry the named tests behind every `automated` catalog entry — prints an `export PATH="…:$PATH"` line, then points at the manual sweep below. (`cargo make test` in `specify-cli` remains the full deterministic surface, including the wasm-tool suites `make acceptance` skips for portability.) It does not run, fake, record, or golden-compare the manual scenario pack, and it is deliberately **not** wired into `make ci`, so it is not a required automated acceptance check — every manual scenario's `negative-expectation` stays held.

`make ci` runs `make lint`. Set `SPECIFY_FRAMEWORK_ROOT` only when invoking `specify lint framework` directly without `--framework-root`. To run the predicate regression suite, use `cargo make test` from a `specify-cli` checkout.

## Running the manual sweep

The sweep needs the build under test on your PATH. `make acceptance` builds one and prints an `export PATH="…:$PATH"` line — copy-paste it before running the sweep so the bare `specify` commands in the scenarios resolve to this build. To build without `make`, run `cargo build --release --manifest-path ../specify-cli/Cargo.toml --bin specify` and prepend `../specify-cli/target/release` to your PATH yourself; confirm it with `specify --version`.

For each scenario:

1. Open the scenario file under [`acceptance/lifecycle/<id>.md`](../../acceptance/lifecycle/) — each is self-contained (intent, setup, invocation, assertions).
2. Bring up a fresh disposable environment per the scenario's **Setup** (common steps factored into [`acceptance/shared/setup.md`](../../acceptance/shared/setup.md)).
3. Run the scenario's **Invocation** exactly as written, stamping Gate 1 yourself (`specify plan transition <name> approved`) — the skills never auto-stamp.
4. Check each **Assertion** on durable structure only (never a byte/golden compare).
5. Record the run with [`acceptance/shared/run-summary-template.md`](../../acceptance/shared/run-summary-template.md), filed under [`acceptance/runs/`](../../acceptance/runs/README.md), and update the scenario's status in the [catalog](../../acceptance/lifecycle/README.md).

Operators who prefer an agent to do the clerical work can paste the reusable prompts in [`acceptance/shared/meta-prompts.md`](../../acceptance/shared/meta-prompts.md) into a live `cursor-agent` session.

## Agent runbook — "run specify's acceptance tests"

When asked to "run specify's acceptance tests and report any issues", an agent should follow this exact sequence. The acceptance surface is two-tier, and the manual tier has irreducible human seams, so the agent reports the automated surface as a clean pass/fail and the manual sweep as a per-scenario table that may include "paused — needs you" rows.

1. **Automated surface.** Run `make acceptance` — it runs `make lint` plus the fixture-backed acceptance tests (`fan_in_fan_out`, `source_extract`, `slice`, `plan_orchestrate`, `workspace`), which prove every `automated` catalog entry. Report pass/fail with the failing finding/test ids. For the full deterministic surface (including the wasm-tool suites), run `cargo make test` in the `specify-cli` checkout. This step needs no human input.
2. **Manual sweep — per scenario, in wave order** (see [catalog](../../acceptance/lifecycle/README.md)):
   - Drive setup with [`shared/meta-prompts.md`](../../acceptance/shared/meta-prompts.md) Prompt A, then the lifecycle with Prompt B.
   - Self-grade only the **structurally checkable** assertions and negative-expectations; record pass/fail/skipped with an evidence pointer per scenario.
3. **Stop and hand back to the operator** at the irreducible human seams — never fabricate a result for these:
   - A missing `specify` binary. `make acceptance` builds one and prints the `export PATH="…:$PATH"` line to copy-paste, or prepend the sibling `specify-cli/target/release` dir to PATH yourself; the agent hands back when no built binary exists.
   - Real forge PR merges between the two `/spec:finalize` invocations.
   - Ergonomics / judgment assertions the agent cannot deterministically verify — mark `needs-human`.
   - `deferred` entries and scenario #1 sign-off (release-blocker; see halt rule below).

## Execution order and the halt gate

The catalog is drained in three waves. Each run fills a run-summary and flips the scenario's catalog status to `passed` / `failed` / `deferred`.

1. **Wave 0 — release blocker.** Scenario `pure-intent` (N=1). **Hard halt:** if it fails, record the failure, do not run any other scenario, triage, then resume once green. No later scenario is meaningful while it is red.
2. **Wave 1 — core synthesis + routing.** The happy-path planning, multi-slice, multi-repo routing, authority/conflict tagging, and Gate-1 amend scenarios.
3. **Wave 2 — failure and breakout paths.** The negative, recovery, and breakout scenarios.

Within a wave, scenarios are independent and may run in any order; a failure outside Wave 0 is recorded and triaged but does not halt sibling runs.

## The gate signal

- Each run commits its filled run-summary under [`acceptance/runs/`](../../acceptance/runs/README.md) as the audit trail.
- On failure, preserve the workspace state, `plan.yaml`, `registry.yaml`, push/finalize output, and branch/PR identifiers per the template, and file a follow-up issue in `augentic/specify` linked back to the run-summary.
- The **release gate is green** when `tests/fan_in_fan_out.rs` passes under `cargo make test`, scenario `pure-intent` is `passed`, and every non-deferred catalog entry is `passed`. A `deferred` entry (capability genuinely missing on the binary under test) must carry a linked follow-up issue and explicit release-owner sign-off.

When the whole catalog is `passed` (or `deferred` with sign-off), record the gate as green in the [catalog](../../acceptance/lifecycle/README.md) and flip RM-05 from *Partial* to *Done* in [`rfcs/roadmap.md`](../../rfcs/roadmap.md).

## What the scenarios prove

The `lifecycle` pack proves the operator-facing `/spec:*` change lifecycle end-to-end across the full difficulty range — N=1 trivial through multi-repo, happy-path through failure and recovery. Highlights:

- **N=1 and single-project planning** (`pure-intent`, `documentation-one-slice`, `plan-single-project`): degenerate `intent` / `documentation` survey, Gate-1 ergonomics, `Sources:` provenance, plans that stop at `pending` and print the literal Gate-1 transition command.
- **Synthesis and reconciliation** (`combined-evidence`, `divergence-authority`, `same-authority-conflict`, `cross-source-merge`): inline `[conflict]` / `[divergence]` tagging, authority resolution, deterministic cross-source reconciliation, lifecycle reaching `refined` cleanly.
- **Cross-repo routing** (`contract-routing`, `cross-repo-contract-flow`, `multi-repo-workspace`, `workspace-execute-two-projects`): contract-first plans, registry-driven routing, workspace slot materialisation, durable end-state (archived plan path, one merged PR per routed project, archived `change.md`).
- **Failure and breakout** (`extract-failure`, `invalid-evidence`, `target-shape-injection`, `source-sandbox-denied`, `amend-into-two`, `stepthrough-breakout`, `execute-build-failure`, `workspace-breakout`, `dual-driving-refused`, `stale-workspace-recovery`): structured errors that keep the slice in `refining`, build-failure stop/resume, breakout verbs, sandbox enforcement, and stale-slot recovery.

## Fan-in / fan-out acceptance

The cross-source fan-in / cross-slice fan-out acceptance splits across the two surfaces above, and **both** must pass before a release is complete:

1. **Deterministic CLI proof (automated).** [`tests/fan_in_fan_out.rs`](https://github.com/augentic/specify-cli/blob/main/tests/fan_in_fan_out.rs) in `augentic/specify-cli` runs under `cargo make test` and asserts the envelope, ordering, and determinism of the whole path. It does not execute real target codegen.
2. **Generated-output-correctness release gate (manual / CI).** Each target build must pass the target's own replay/golden suite plus `cargo check` / `cargo test` for generated crates (and the equivalent verification for non-Rust targets). A slice whose generated output fails these checks is not done, regardless of build-envelope validity.

## Synthesis byte-replay (deferred)

The `specify-standards` harness covers checker regressions and repo consistency, but does **not** assert on the bytes a `/spec:refine` or `/spec:build` skill body emits. A byte-equivalent "synthesis golden" requires either a recorded-transcript layer (capture a `cursor-agent` run via `@cursor/sdk` and replay it) or a structured-trace assertion library (compare the *shape* of synthesised artifacts rather than the bytes). Both are out of scope for now; a follow-up RFC will pick one. Until then, the manual sweep is the source of truth for end-to-end LLM-driven correctness.
