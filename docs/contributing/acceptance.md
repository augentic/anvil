# Running Acceptance

This is the single entry point for Specify acceptance. It defines the two acceptance surfaces, how an operator (or agent) runs them, the group ordering and halt gate, and the green-gate signal.

The scenario catalog — the canonical list of every scenario, its grouping, release-blocker status, and run status — lives in [`acceptance/scenarios/README.md`](../../acceptance/scenarios/README.md). This document does not duplicate that table.

## The two acceptance surfaces

A release is proven only when **both** surfaces are green:

1. **Deterministic CLI proof — automated.** `cargo make test` in [`augentic/specify-cli`](https://github.com/augentic/specify-cli) (including [`tests/plan/end_to_end.rs`](https://github.com/augentic/specify-cli/blob/main/tests/plan/end_to_end.rs)) asserts the envelope, ordering, and re-projection determinism of the whole CLI path: `source survey` → `plan propose --dry-run | --from` → per-slice `source extract` → `slice synthesize` → `slice build` → `slice merge`, plus `depends-on` ordering and byte-identical kernel re-projection. It does **not** execute real target codegen. Plus the static repository checks: `make lint` runs `specify lint framework --framework-root .` against the live tree (skill frontmatter, adapter manifests, rule shape, links, marketplace consistency, scenario frontmatter).
2. **Operator scenario sweep — manual.** The platform scenarios in [`acceptance/scenarios/`](../../acceptance/scenarios/) exercise the full `/spec:plan` → Gate 1 → `/spec:execute` → `/spec:finalize` rhythm against live `cursor-agent`, plus the per-target generated-output-correctness gate. A schema-valid `build/report.yaml` with `status: success` proves the build envelope held, **not** that the generated code compiles or replays — so each exercised target must also pass `cargo check` / `cargo test` / its replay suite (and the equivalent verification for non-Rust targets). A slice whose generated output fails these checks is not done, regardless of envelope validity.

The scenario sweep is intentionally **not** an automated harness: no runner, fake forge, recorded transcript, CI target, or golden-output comparison. That posture is encoded as the `negative-expectations` frontmatter on every scenario and is the one place this rationale is stated — individual scenarios do not repeat it in prose. It remains manual because it involves LLM-emitted prose; `specify lint framework` does not pin synthesised bytes.

**Fixture-backed scenarios are the exception.** A scenario whose assertions are entirely deterministic CLI/host behavior — no LLM prose to judge — is promoted to `backend: fixture` and proven by a named test in surface 1, not the manual sweep. Its scenario file carries an **Automated coverage** section naming the test (e.g. `source-sandbox-denied` → `tests/source/extract.rs::sandbox_denies_out_of_scope`), and its catalog status is `automated`. The manual sweep skips these; the deterministic surface covers them on every commit. The `negative-expectations` (no runner / no CI target) apply only to the manual-prose scenarios, so a fixture-backed scenario drops them.

**What keeps a scenario manual.** A scenario stays on the manual sweep when at least one assertion is irreducible to deterministic CLI/host behavior. Three categories, each with the deterministic substrate that *is* covered by surface 1 named alongside:

1. **LLM-prose judgment** — whether a synthesized `spec.md` / `design.md` reads correctly, or a plan decomposes a brief sensibly (e.g. `pure-intent`, `documentation-one-slice`, `plan-single-project`, `target-shape-injection`, `cross-source-merge`). The CLI substrate — provenance/`Sources:` rendering, `[conflict]`/`[divergence]` tagging, propose routing, the embedded `shape` brief in the synthesis envelope — is covered by `tests/slice/synthesize.rs`, `tests/workflow/`, and `tests/plan/end_to_end.rs`.
2. **Skill-loop orchestration** — the `/spec:execute` stop / resume / `all-done` behavior emitted by skill markdown, not by any single CLI verb (e.g. `execute-build-failure`, `stepthrough-breakout`, `stale-workspace-recovery`, `workspace-breakout`). The per-step primitives — build-finalize gating (`tests/slice/build.rs`), `plan next` advance, `slice merge` stamping `done`, `workspace sync` dirty-slot detection (`tests/workspace.rs`) — are covered; the loop that sequences them is not.
3. **Live-forge interaction** — real PR pushes/merges between `/spec:finalize` invocations (e.g. `cross-repo-contract-flow`, `workspace-execute-two-projects`), and skill-enforced pre-flight refusals (`dual-driving-refused`). No deterministic fixture stands in for a live forge or a live `cursor-agent` pre-flight.

This is the boundary: a scenario is promotable to `backend: fixture` only when *every* assertion falls outside these three categories. Re-confirm against this list before adding a scenario to the manual sweep — if all its assertions are deterministic, it belongs in surface 1 instead.

## Running the automated surface

```bash
make lint          # static repository checks; no specify-cli checkout required
make install-specify    # resolves a specify binary per SPECIFY_VERSION, runs make lint, then symlinks specify onto your PATH for the sweep
```

Both targets resolve their `specify` binary the same way — through [`scripts/specify.sh`](../../scripts/specify.sh) per `SPECIFY_VERSION` (see [Consistency Checks — binding model](checks.md#binding-to-a-specify-binary)). Neither **requires** a `specify-cli` checkout: under the default `next` channel a sibling/nested checkout is the preferred source (built from source), but absent one both fall back to acquiring the [`.specify-version`](../../.specify-version) pin into `./.bin`. `make install-specify` differs from `make lint` only in that it needs a *concrete binary path* to symlink onto your `PATH`, so it uses the resolver's `bin-path` mode (which builds the checkout once under `next`, or emits the acquired `./.bin` / system binary otherwise) rather than `lint`'s ephemeral `cargo run` prefix.

`make install-specify` **prepares the manual sweep**: it resolves the `specify` binary per `SPECIFY_VERSION` (building from a `specify-cli` checkout when present, else acquiring the published pin), runs `make lint`, and symlinks the resolved binary into `~/.local/bin` (warning if it is not on your `PATH`), then points at the manual sweep below. It does **not** re-run the deterministic acceptance tests — `cargo make test` in `specify-cli` is the single authoritative deterministic surface (including the wasm-tool suites), and it runs there on every commit, so re-running the `plan` / `source` / `slice` / `workspace` test binaries from this repo would only duplicate that work. `make install-specify` does not run, fake, record, or golden-compare the manual scenario pack, and it is deliberately **not** wired into CI, so it is not a required automated acceptance check — every manual scenario's `negative-expectation` stays held.

Set `SPECIFY_ROOT` only when invoking `specify lint framework` directly without `--framework-root`. To run the predicate regression suite, use `cargo make test` from a `specify-cli` checkout.

## Running the manual sweep

The sweep needs the binary under test on your PATH. `make install-specify` resolves one (per `SPECIFY_VERSION`) and symlinks it into `~/.local/bin` (override with `INSTALL_DIR=…`), so the bare `specify` commands in the scenarios resolve to this binary — it warns if `~/.local/bin` is not on your PATH. To build without `make`, run `cargo build --release --manifest-path ../specify-cli/Cargo.toml --bin specify` and symlink `../specify-cli/target/release/specify` into a PATH directory yourself; confirm it with `specify --version`.

For each scenario:

1. Open the scenario file under [`acceptance/scenarios/<id>.md`](../../acceptance/scenarios/) — each is self-contained (intent, setup, invocation, assertions).
2. Bring up a fresh disposable environment per the scenario's **Setup** (common steps factored into [`acceptance/shared/setup.md`](../../acceptance/shared/setup.md)).
3. Run the scenario's **Invocation** exactly as written, stamping Gate 1 yourself (`specify plan transition <name> approved`) — the skills never auto-stamp.
4. Check each **Assertion** on durable structure only (never a byte/golden compare).
5. Record the run with [`acceptance/shared/run-summary-template.md`](../../acceptance/shared/run-summary-template.md), filed under [`acceptance/runs/`](../../acceptance/runs/README.md), and update the scenario's status in the [catalog](../../acceptance/scenarios/README.md).

Operators who prefer an agent to do the clerical work can paste the reusable prompts in [`acceptance/shared/meta-prompts.md`](../../acceptance/shared/meta-prompts.md) into a live `cursor-agent` session.

## Agent runbook — "run specify's acceptance tests"

When asked to "run specify's acceptance tests and report any issues", an agent should follow this exact sequence. The acceptance surface is two-tier, and the manual tier has irreducible human seams, so the agent reports the automated surface as a clean pass/fail and the manual sweep as a per-scenario table that may include "paused — needs you" rows.

1. **Automated surface.** Run `make install-specify` — it runs `make lint` and resolves + symlinks the binary under test. Report `make lint` pass/fail with the failing finding ids. The deterministic acceptance tests (`plan`, `source`, `slice`, `workspace`, and the wasm-tool suites) are owned by `specify-cli` and run there on every commit; run `cargo make test` in the `specify-cli` checkout when you need to prove the full deterministic surface locally. This step needs no human input. Then make the build under test resolvable in the agent's own shells: run `specify --version` and, if the bare command does not resolve to the freshly built binary, prepend the symlink dir to `PATH` for the rest of the sweep (`export PATH="$HOME/.local/bin:$PATH"`, matching `make install-specify`'s `INSTALL_DIR`) or fall back to the absolute `../specify-cli/target/release/specify` path. Re-confirm with `specify --version` before driving any scenario — a Makefile recipe cannot mutate the agent's shell `PATH`, so the agent owns this self-heal.
2. **Manual sweep — per scenario, in group order** (see [catalog](../../acceptance/scenarios/README.md)):
   - Drive setup with [`shared/meta-prompts.md`](../../acceptance/shared/meta-prompts.md) Prompt A, then the lifecycle with Prompt B.
   - Self-grade only the **structurally checkable** assertions and negative-expectations; record pass/fail/skipped with an evidence pointer per scenario.
3. **Stop and hand back to the operator** at the irreducible human seams — never fabricate a result for these:
   - A `specify` build that cannot be produced. The agent builds and resolves the binary itself in step 1 (`make install-specify` plus the `PATH` self-heal / absolute-path fallback); it hands back only when the build itself fails — e.g. the sibling `specify-cli` checkout is missing or does not compile.
   - Real forge PR merges between the two `/spec:finalize` invocations.
   - Ergonomics / judgment assertions the agent cannot deterministically verify — mark `needs-human`.
   - `deferred` entries and scenario #1 sign-off (release-blocker; see halt rule below).

## Execution order and the halt gate

The catalog is drained in groups. Each run fills a run-summary and flips the scenario's catalog status to `passed` / `failed` / `deferred`.

1. **N=1 hard halt — release blocker.** Scenario `pure-intent` (N=1). **Hard halt:** if it fails, record the failure, do not run any other scenario, triage, then resume once green. No later scenario is meaningful while it is red.
2. **Core synthesis + routing.** The happy-path planning, multi-slice, multi-repo routing, authority/conflict tagging, and Gate-1 amend scenarios.
3. **Failure and breakout paths.** The negative, recovery, and breakout scenarios.

Within a group, scenarios are independent and may run in any order; a failure outside the `pure-intent` hard halt is recorded and triaged but does not halt sibling runs.

## The gate signal

- Each run commits its filled run-summary under [`acceptance/runs/`](../../acceptance/runs/README.md) as the audit trail.
- On failure, preserve the workspace state, `plan.yaml`, `registry.yaml`, push/finalize output, and branch/PR identifiers per the template. The sandbox at `acceptance/.sandbox/<scenario>/` ([`setup.md`](../../acceptance/shared/setup.md)) is stable and gitignored, so it survives for inspection; capture an `scripts/snapshot.sh` snapshot into the run-summary's **Artefact snapshot** section so evidence is self-contained rather than tied to a temp root. File a follow-up issue in `augentic/specify` linked back to the run-summary.
- The **release gate is green** when `tests/plan/end_to_end.rs` passes under `cargo make test`, scenario `pure-intent` is `passed`, and every non-deferred catalog entry is `passed`. A `deferred` entry (capability genuinely missing on the binary under test) must carry a linked follow-up issue and explicit release-owner sign-off.

When the whole catalog is `passed` (or `deferred` with sign-off), record the gate as green in the [catalog](../../acceptance/scenarios/README.md) and flip RM-05 from *Partial* to *Done* in [`rfcs/roadmap.md`](../../rfcs/roadmap.md).

## What the scenarios prove

The `lifecycle` pack proves the operator-facing `/spec:*` change lifecycle end-to-end across the full difficulty range — N=1 trivial through multi-repo, happy-path through failure and recovery. Highlights:

- **N=1 and single-project planning** (`pure-intent`, `documentation-one-slice`, `plan-single-project`): degenerate `intent` / `documentation` survey, Gate-1 ergonomics, `Sources:` provenance, plans that stop at `pending` and print the literal Gate-1 transition command.
- **Synthesis and reconciliation** (`combined-evidence`, `divergence-authority`, `same-authority-conflict`, `cross-source-merge`): inline `[conflict]` / `[divergence]` tagging, authority resolution, deterministic cross-source reconciliation, lifecycle reaching `refined` cleanly.
- **Cross-repo routing** (`contract-routing`, `cross-repo-contract-flow`, `multi-repo-workspace`, `workspace-execute-two-projects`): contract-first plans, registry-driven routing, workspace slot materialisation, durable end-state (archived plan path, one merged PR per routed project, archived `change.md`).
- **Failure and breakout** (`extract-failure`, `invalid-evidence`, `target-shape-injection`, `source-sandbox-denied`, `amend-into-two`, `stepthrough-breakout`, `execute-build-failure`, `workspace-breakout`, `dual-driving-refused`, `stale-workspace-recovery`): structured errors that keep the slice in `refining`, build-failure stop/resume, breakout verbs, sandbox enforcement, and stale-slot recovery.

## Fan-in / fan-out acceptance

The cross-source fan-in / cross-slice fan-out acceptance splits across the two surfaces above, and **both** must pass before a release is complete:

1. **Deterministic CLI proof (automated).** [`tests/plan/end_to_end.rs`](https://github.com/augentic/specify-cli/blob/main/tests/plan/end_to_end.rs) in `augentic/specify-cli` runs under `cargo make test` and asserts the envelope, ordering, and determinism of the whole path. It does not execute real target codegen.
2. **Generated-output-correctness release gate (manual / CI).** Each target build must pass the target's own replay/golden suite plus `cargo check` / `cargo test` for generated crates (and the equivalent verification for non-Rust targets). A slice whose generated output fails these checks is not done, regardless of build-envelope validity.

## Synthesis byte-replay (deferred)

The `specify-standards` harness covers checker regressions and repo consistency, but does **not** assert on the bytes a `/spec:refine` or `/spec:build` skill body emits. A byte-equivalent "synthesis golden" requires either a recorded-transcript layer (capture a `cursor-agent` run via `@cursor/sdk` and replay it) or a structured-trace assertion library (compare the *shape* of synthesised artifacts rather than the bytes). Both are out of scope for now; a follow-up RFC will pick one. Until then, the manual sweep is the source of truth for end-to-end LLM-driven correctness.
