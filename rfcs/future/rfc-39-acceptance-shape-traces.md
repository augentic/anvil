# RFC-39: Acceptance Shape Assertions and Orchestration Traces

> Status: Deferred · Depends: acceptance run-record typing + status reconciler (Phases 0–1 of the acceptance remediation effort), [RM-05](../roadmap.md#rm-05-multi-repo-acceptance-suite) · Supersedes: [`docs/contributing/acceptance.md` §"Synthesis byte-replay (deferred)"](../../docs/contributing/acceptance.md#synthesis-byte-replay-deferred)

## Abstract

Today acceptance has exactly two tiers: a **deterministic surface** (`cargo make test` in `augentic/specify-cli`, fed by fixtures) and a **manual operator sweep** (the [`acceptance/lifecycle/`](../../acceptance/lifecycle/README.md) scenarios, judged by a human or `cursor-agent`). A scenario is either fully automatable (`backend: fixture`) or fully manual (`backend: manual`); there is nothing in between. As a result the manual sweep is the *only* end-to-end proof of LLM-driven correctness, it is labour-intensive and non-reproducible, and it conflates two unlike concerns under one verdict: **was the workflow orchestrated correctly** (largely deterministic) versus **is the synthesized prose good** (irreducibly subjective).

This RFC introduces two intermediate mechanisms and the scenario tier that carries them:

1. **Shape assertions** — deterministic structural predicates over synthesized artifacts (requirement count, `Sources:` presence, `[conflict]` / `[divergence]` tags, `#### Scenario:` headings, lifecycle state) that check the *shape* of an artifact without pinning its bytes.
2. **Orchestration traces** — recorded `cursor-agent` transcripts (captured via `@cursor/sdk`) replayed against the real CLI so that skill-loop control flow (`stop` / `resume` / `all-done`, build-failure parking) is deterministically assertable without a live agent.

Together they define a new `backend: shape` tier between `manual` and `fixture`, and a promotion path **manual → shape → fixture**. The goal is to shrink the manual sweep down to genuine prose-quality judgment while keeping the deliberate `negative-expectations` posture intact: shape assertions are *not* golden bytes, and orchestration replay is *not* a fake forge.

This RFC adds no lifecycle authority. Shape checks and trace replays are acceptance evidence only; they never transition a slice, stamp a plan, or merge a change.

## Motivation

[`docs/contributing/acceptance.md` §"What keeps a scenario manual"](../../docs/contributing/acceptance.md) names three categories that keep a scenario off the deterministic surface: **LLM-prose judgment**, **skill-loop orchestration**, and **live-forge interaction**. The current binary tiering forces all three into the same fully-manual bucket, which over-charges the sweep:

- **The manual sweep is the only E2E LLM-correctness proof.** The deferred "Synthesis byte-replay" note (`acceptance.md` §"Synthesis byte-replay (deferred)") already flags that nothing asserts on the bytes a `/spec:refine` or `/spec:build` body emits, and explicitly leaves the choice between a recorded-transcript layer and a structured-trace assertion library to a follow-up RFC. **This is that RFC**, and it picks *both*, scoped to different concerns.
- **Orchestration is deterministic but trapped in the manual tier.** Whether `/spec:execute` parks on a build failure, resumes, and reaches `all-done` is control flow, not prose. The `acceptance/examples/**/expected-trace.md` files (e.g. [`build/success/expected-trace.md`](../../acceptance/examples/skills/build/success/expected-trace.md)) already describe these visible side effects in prose — they are one capture format away from being machine-checkable.
- **Operator self-grading is error-prone.** During the sweep the agent eyeballs structure ("does the `Sources:` line carry both keys?"). A shape-check verb lets the operator self-grade the *structural* assertions deterministically and reserve human attention for the prose.

### Trigger conditions

This RFC stays deferred until both prerequisites hold, then any one trigger promotes it to active:

Prerequisites (the acceptance plumbing this layer reads):

- **Typed run records** — run-summaries carry machine-readable frontmatter (verdict, binary, issues) so shape/trace verdicts can be filed and reconciled.
- **Status reconciler** — the catalog ↔ runs ↔ issues consistency check exists, so a new `backend: shape` status has somewhere to be validated.

Triggers:

1. **Sweep cost.** The manual sweep's per-release run time becomes the dominant cost of cutting a release.
2. **Reproducibility pressure.** A scenario regresses between releases and the prose run records are too unstructured to bisect what changed.
3. **`@cursor/sdk` availability.** A stable recording/replay surface in `@cursor/sdk` makes trace capture cheap enough to maintain.

## Principles

- **Shape is not bytes.** A shape assertion checks structural facts (counts, presence of named sections, tag coherence, lifecycle state). It never compares synthesized prose byte-for-byte. `golden-output-required` stays a forbidden negative-expectation.
- **Replay is not a fake forge.** An orchestration trace replays a *recorded agent's own outputs* against the *real* CLI verbs; it never stubs `gh`, fakes a remote, or simulates a merge. Live-forge scenarios stay manual.
- **The CLI is authoritative.** Shape predicates and the trace-replay harness are deterministic primitives owned by `augentic/specify-cli`; scenario files and recorded traces are data in `augentic/specify`. Skills and agents consume them; they do not reimplement them.
- **No lifecycle authority.** Acceptance evidence — manual, shape, or fixture — never transitions a slice or stamps a plan. (Roadmap principle "Keep enforcement surfaces distinct" carries forward.)
- **One verdict per concern.** A scenario's orchestration verdict (trace) and prose verdict (human) are recorded separately so a green orchestration result is not diluted by a subjective prose call, and vice versa.

## Design

### Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 `shape` backend tier** | Add `shape` to the scenario `backend` enum, between `manual` and `fixture`. A `backend: shape` scenario carries machine-checkable shape assertions and/or an orchestration trace, plus (optionally) residual prose assertions that stay human-judged. | Widen `schemas/scenario.schema.json` `backend` enum additively; `specify lint framework` scenario-frontmatter check accepts the new value. |
| **D2 Shape-assertion DTO** | A closed set of structural predicates over a slice/plan artifact tree: `artifact-present`, `requirement-count`, `sources-line-contains`, `tag-present` (`conflict` / `divergence` / `unknown`), `scenario-heading-present`, `lifecycle-state`. Each predicate names its target path and expected value. | New `schemas/acceptance/shape.schema.json`; new `ShapeAssertion` DTO + evaluator in `specify-cli` over the `specify-model` parsers (reusing the `spec.md` provenance parser). |
| **D3 Orchestration trace format** | A recorded `cursor-agent` session: ordered agent turns + the CLI verbs they invoked, captured via `@cursor/sdk`. Replay runs the recorded agent outputs against the *real* CLI in a disposable project and asserts the **control-flow trace** (verbs called, in order; stop hints emitted; terminal `all-done` / parked state) — never the prose bytes. | New `schemas/acceptance/trace.schema.json`; a replay harness in `specify-cli` that drives the recorded turns and diffs the observed control-flow trace against the recorded one. |
| **D4 Two consumption modes** | Shape assertions are authored once and consumed two ways: **(a) CI mode** over recorded artifact fixtures under `acceptance/examples/` (deterministic, runs under `cargo make test`); **(b) live mode** during the manual sweep, against the just-produced live output, so the operator self-grades structure deterministically. | One evaluator, two entry points: a test harness (CI) and a `specify acceptance shape-check <id>` verb (live). |
| **D5 Promotion path** | `manual → shape → fixture`. A scenario graduates to `shape` when its orchestration and structural assertions are captured as a trace + shape set; it graduates to `fixture` only when *zero* residual prose assertions remain. Demotion is allowed and recorded when a shape assertion can no longer hold. | The status reconciler (prerequisite) validates that a `shape` scenario names a trace and/or shape set; a `fixture` scenario names a deterministic test (existing rule, RFC-bound here). |
| **D6 Negative-expectation evolution** | Add `shape-assertion-added` and `orchestration-trace-added` as **sanctioned** mechanisms (they drop from a scenario's forbidden list when it reaches `shape`). `fake-forge-added` and `golden-output-required` stay forbidden on every tier. `automated-runner-added` stays forbidden for the *prose-grading* loop but does not forbid trace replay of *orchestration*. | Scenario schema: `negative-expectations` enum gains the two sanctioned ids; `docs/contributing/acceptance.md` documents the per-tier forbidden set in one place. |
| **D7 Repo split** | Deterministic primitives (shape evaluator, trace replay harness, schemas) live in `augentic/specify-cli`. Scenario files, recorded traces, and artifact fixtures live in `augentic/specify`. | New schemas in `crates/schema/`; evaluator/harness in a workflow- or acceptance-scoped module; recorded traces under `acceptance/examples/traces/`. |

### The `shape` tier in a scenario file

```yaml
---
id: divergence-authority
backend: shape            # was: manual
entrypoint: /spec:plan
stages: [plan, refine]
shape:
  - artifact-present: .specify/slices/password-reset/spec.md
  - tag-present: { artifact: spec.md, tag: divergence }
  - sources-line-contains: { requirement: PWD-RESET-EXPIRY, keys: [docs, legacy] }
  - lifecycle-state: refined
trace: traces/divergence-authority.trace.json   # optional orchestration replay
assertions:                                       # residual human-judged prose, if any
  - divergence-reads-coherently
negative-expectations:
  - fake-forge-added
  - golden-output-required
  # shape-assertion-added / orchestration-trace-added are now sanctioned, so absent here
---
```

A scenario with **no** residual `assertions` block and a passing shape set + trace is eligible for promotion to `fixture` (D5).

### Shape-assertion evaluation

The evaluator is a pure function over a slice/plan artifact tree and a `ShapeAssertion[]`. It reuses the existing `specify-model` parsers — notably the `spec.md` requirement-block parser (`crates/model/src/spec/provenance.rs`) — so `requirement-count`, `sources-line-contains`, and `tag-present` are read from the same structures the product itself uses, not a bespoke re-parse.

- **CI mode** runs the evaluator over a recorded artifact fixture checked into `acceptance/examples/`. Deterministic; runs under `cargo make test`; the scenario's **Automated coverage** section names the test (mirroring the existing `fixture` convention).
- **Live mode** runs `specify acceptance shape-check <id>` against the project the operator just drove. It emits a per-predicate pass/fail table the operator pastes into the run record. This replaces eyeballing for the structural assertions; the human still judges the residual prose.

### Orchestration trace capture and replay

Capture records a real `cursor-agent` run of a scenario via `@cursor/sdk`: each agent turn and the CLI verbs it invoked, plus the observed control-flow markers (stop hints, terminal state). The recording is stored as `acceptance/examples/traces/<id>.trace.json`.

Replay drives the recorded agent turns against the **real** CLI in a fresh disposable project and asserts the **control-flow trace** matches: the same verbs in the same order, the same stop hints at the same points, the same terminal `all-done` / parked outcome. The `expected-trace.md` files already capture exactly this kind of side-effect sequence in prose (see [`build/success/expected-trace.md`](../../acceptance/examples/skills/build/success/expected-trace.md)) — this decision makes that sequence executable.

What replay deliberately does **not** assert: the prose content of the agent's turns, the bytes of synthesized artifacts (that is D2's shape job), or anything involving a live forge (those scenarios stay manual).

### CLI surface

```bash
specify acceptance shape-check <id>          # live mode: evaluate the scenario's shape set against the current project
specify acceptance trace record <id>         # capture a cursor-agent run into acceptance/examples/traces/<id>.trace.json
specify acceptance trace replay <id>         # replay a recorded trace against the real CLI; assert the control-flow trace
specify acceptance status                    # (from the prerequisite reconciler) now also reports shape/trace coverage
```

`shape-check` and `trace replay` are read-only with respect to project lifecycle; `trace record` writes only under `acceptance/examples/traces/`.

### Relationship to the existing surfaces

| Concern | Deterministic fixture (`backend: fixture`) | This RFC (`backend: shape`) | Manual (`backend: manual`) |
| --- | --- | --- | --- |
| Synthesized-artifact bytes | not asserted | not asserted | human-judged |
| Synthesized-artifact **structure** | asserted (where deterministic) | **asserted (shape, D2)** | human-judged |
| Skill-loop orchestration | partial (per-verb primitives) | **asserted (trace replay, D3)** | human-judged |
| Prose quality | n/a | residual `assertions` only | human-judged |
| Live forge | excluded | excluded | human-driven |
| Runs in CI | yes | yes (CI mode, D4a) | no |

## Prerequisite plumbing (deferred)

The two prerequisites named in §"Trigger conditions" — **typed run records** and a **status reconciler** — plus two supporting helpers were scoped out of the active acceptance remediation effort and parked here. The active effort delivered only the genuinely forward-moving items (the `specify init` first-party shorthand + `$SPECIFY_FRAMEWORK_ROOT` fallback, the `make acceptance-scenario` scaffolder, and the Makefile simplification); this plumbing is captured concretely so it is ready to build when the trigger fires.

**Why deferred, not dropped.** A reconciler polices drift between run records, the catalog, and issues — but there is nothing to reconcile until the manual sweep actually runs. There are currently zero run records, so building the policing layer (especially as a native `specify lint framework` check) before the artifacts it polices exist is premature. Fix what blocks the sweep, run it, then let real run records justify the reconciler.

**Promotion trigger.** Promote this plumbing from the RFC back into an active plan once the first real sweep has produced run records *and* the catalog has drifted (or is at concrete risk of drifting) in practice.

### P0 — Typed run records

Give run-summaries machine-readable frontmatter so verdicts can be filed and reconciled.

- New `schemas/acceptance/run.schema.json` in `augentic/specify-cli`; embed it as `RUN_JSON_SCHEMA` (constant in `crates/schema/src/constants.rs`, export in `crates/schema/src/lib.rs`, parity + compile entries in `crates/schema/tests/schemas.rs` — mirroring every other embedded schema constant).
- Add the frontmatter block to [`acceptance/shared/run-summary-template.md`](../../acceptance/shared/run-summary-template.md) and reverse the prose-only note in [`acceptance/runs/README.md`](../../acceptance/runs/README.md) (and the matching note in `docs/contributing/checks.md`).
- Fields: `scenario`, `date`, `verdict` (`pass | fail | deferred`), `wave`, `binary { version, path }`, `issues[]`, `operator`.

### P1 — Status reconciler

A native `specify lint framework` check (no Python), modelled on the existing cross-file `scenarios.duplicate-id` check.

- Implement `crates/standards/src/framework/check/acceptance.rs` in `augentic/specify-cli`, bridged by a `CORE-053` `kind: authoring-predicate` rule file in `augentic/specify`.
- Validates: catalog status vs. the typed run records (P0) vs. each scenario's `## Automated coverage` section.
- Blocking findings *are* the gate signal — no separate `specify acceptance status` command is required for the gate (the `status` verb in the CLI surface above stays an optional reporter).

### P2 — Evidence capture helper

A small bash `tee` / `script(1)` helper that writes each command's output under the run's evidence directory, referenced from [`acceptance/shared/meta-prompts.md`](../../acceptance/shared/meta-prompts.md) and the run-summary template. Pure setup/capture aid — it drives no `/spec:*` command and grades nothing, so the `automated-runner-added` negative-expectation stays held.

### P3 — Cross-repo coverage map

Verify each `automated` (`backend: fixture`) scenario's named test actually exists in `augentic/specify-cli`. This is a poor fit for `specify lint framework` (the framework repo cannot see the sibling repo's test tree), hence RFC-only for now. Two candidate designs to decide at promotion time:

- **Test-name manifest** — `specify-cli` exports a manifest of test names that the reconciler (P1) cross-references. Keeps the framework repo self-contained.
- **`SPECIFY_CLI_ROOT`-gated probe** — the reconciler probes the sibling repo when an env var points at it, and gracefully skips when absent.

## Alternatives considered

**Recorded-transcript replay only (no shape layer).** Rejected as insufficient. Replaying a transcript proves the *orchestration* held but says nothing about whether the synthesized `spec.md` has the right structure for a *different* (live) run. Shape assertions are needed to check live output during the sweep, which transcript replay cannot do.

**Shape assertions only (no trace replay).** Rejected as insufficient. Shape checks validate artifact structure but not the stop/resume/`all-done` control flow that keeps several Wave-2 scenarios manual. The two mechanisms cover disjoint concerns; both are needed to drain the manual tier.

**Golden-byte comparison of synthesized artifacts.** Rejected, and explicitly forbidden by D6. LLM output is non-deterministic; byte goldens would be perpetually red or force temperature-zero contortions that do not reflect real usage. This is the long-standing `golden-output-required` negative-expectation and it stays.

**Fold orchestration replay into the existing `fixture` tier.** Rejected. `fixture` means "zero residual human judgment". A `shape` scenario may still carry residual prose assertions; collapsing the tiers would either over-promote scenarios (hiding real prose debt) or block the orchestration win behind full automation. The middle tier is the point.

**Put the evaluator/harness in `augentic/specify` (skills/scripts).** Rejected by the "CLI is authoritative" principle. Deterministic acceptance primitives belong in the binary so they are versioned, tested, and reused; scenario data stays in the framework repo.

## Non-Goals

- Faking, recording, or replaying a **forge**. Live-forge scenarios (`cross-repo-contract-flow`, `workspace-execute-two-projects`) stay fully manual.
- Grading **prose quality** by machine. Residual prose assertions remain human-judged; this RFC only removes the *structural* and *orchestration* assertions from human hands.
- Byte-level golden comparison of any synthesized artifact.
- A hosted or unattended acceptance runner. Shape-check and trace-replay are operator/CI primitives, not a background service.
- Lifecycle authority of any kind (no slice/plan transitions from acceptance evidence).

## Open Questions

1. **Trace stability.** How tolerant should trace replay be to benign verb-ordering differences (e.g. two independent `specify ... --format json` reads)? Current preference: assert a partial order derived from declared `stages`, not a strict total order.
2. **Trace capture cost.** Is `@cursor/sdk` recording cheap and stable enough to keep traces current as skills evolve, or do traces become stale fixtures that mask drift? Revisit once the SDK surface settles (trigger 3).
3. **Where shape fixtures live vs. the existing corpus.** Should recorded artifact fixtures reuse `acceptance/examples/skills/**` or get a dedicated `acceptance/examples/shape/**` tree? Current preference: reuse the existing corpus and add a shape set beside the existing `expected/` outputs.
4. **Promotion ratchet enforcement.** Should the reconciler *block* a scenario that has zero residual prose assertions from staying `shape` (forcing promotion to `fixture`), or only warn? Current preference: warn first, enforce once the tier is established.

## References

- [`docs/contributing/acceptance.md`](../../docs/contributing/acceptance.md) — the two-surface model, the "what keeps a scenario manual" categories, and the superseded "Synthesis byte-replay (deferred)" note.
- [`acceptance/lifecycle/README.md`](../../acceptance/lifecycle/README.md) — the scenario catalog and status legend (`automated` / `manual`).
- [`acceptance/lifecycle/01-pure-intent.md`](../../acceptance/lifecycle/01-pure-intent.md) and [`05a-combined-evidence.md`](../../acceptance/lifecycle/05a-combined-evidence.md) — `manual` and `fixture` exemplars.
- [`acceptance/examples/skills/build/success/expected-trace.md`](../../acceptance/examples/skills/build/success/expected-trace.md) — prose precedent for the executable orchestration trace (D3).
- [`acceptance/shared/run-summary-template.md`](../../acceptance/shared/run-summary-template.md) — where shape/trace verdicts are filed.
- [Specify Roadmap — RM-05](../roadmap.md#rm-05-multi-repo-acceptance-suite) — the acceptance-proof track this RFC serves.
