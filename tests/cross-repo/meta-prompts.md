# Operator meta prompts

These are reusable prompts an operator pastes into a live `cursor-agent` session to drive a single cross-repo acceptance scenario. They let the agent do the clerical and deterministically-checkable work — environment setup, driving the slash-command lifecycle, capturing per-stage output, self-grading the structural assertions, and filling the run-summary — while leaving the irreducible human seams to the operator. [`scenario.md`](scenario.md) already sanctions an agent-as-operator ("a human **or** agent follows this script").

**These prompts are operator aids, not a harness.** They are pasted interactively per run; they add no checked-in runner, no CI target, no fake forge, and no golden-output comparison, so every negative expectation in [`scenario.md`](scenario.md) still holds. Keep them as documentation next to the queue, never as an unattended job — wiring them into CI or the Cursor SDK would violate `automated-runner-added` / `ci-target-added`.

The three human seams the prompts always hand back to the operator:

1. **Real forge merges** between the two `/spec:finalize` invocations — never faked.
2. **Ergonomics / judgment assertions** the agent cannot deterministically verify — marked `needs-human` for operator confirmation.
3. **`deferred` and scenario-#1 sign-off** — a `deferred` entry needs a linked follow-up issue and release-owner sign-off; scenario #1 is the release blocker.

Drive each scenario with Prompt A first, then Prompt B. Replace `<scenario-id>` with the queue id (e.g. `01-pure-intent`).

## Prompt A — setup

```text
You are the acceptance SETUP operator for Specify scenario <scenario-id>.
Goal: bring a fresh, disposable environment to the exact pre-invocation state the
scenario describes, using only real `specify` CLI commands. Do NOT drive any
/spec:* command yet.

Inputs:
- SPECIFY_BIN: <abs path to the 2.0 binary>. Use it for every specify call; the
  PATH default `specify` is the historical 0.1.0 build and is wrong.
- Scenario stub: tests/cross-repo/runs/2.0.0/<scenario-id>.md
- Shared script: tests/cross-repo/scenario.md (Workspace, Prerequisites, Inputs,
  Invocation §1 "Prepare disposable projects").

Do, in order, capturing each exact command and its verbatim output:
1. Create the disposable directories the scenario names under a fresh temp root.
   Never reuse an existing project or a non-empty Specify state.
2. Run the init / registry-add / brief-file steps verbatim, substituting
   $SPECIFY_BIN for `specify`.
3. Run the scenario's validation step (e.g. `specify registry validate`) and
   confirm it exits 0.
4. STOP before /spec:plan. Report: the temp root, every created path, and the
   captured command log.

Guardrails:
- Real CLI only. Do not add or stub `gh`, a forge, a test runner, or a CI step.
- On any failure, halt and report the failing command + output; do not proceed.
```

## Prompt B — run + confirm

```text
You are the acceptance RUN+CONFIRM operator for Specify scenario <scenario-id>.
Drive the live agent workflow end to end, capture evidence at every stage,
self-verify the structurally-checkable assertions, and fill the run-summary.
Pause at the human-only seams; never fabricate a result.

Inputs:
- The environment left by the SETUP prompt (temp root, SPECIFY_BIN).
- Scenario stub + tests/cross-repo/scenario.md (Invocation, Assertions, Negative
  Expectations).
- tests/cross-repo/run-summary-template.md field-set.

Drive, in order, recording the exact invocation and verbatim output for each:
1. /spec:plan ... exactly as the scenario's draft step specifies. Confirm it
   stops at the hand-off / `pending` and prints the literal Gate-1 transition
   command. Do NOT auto-stamp approval.
2. Review seam: run `specify plan validate` and inspect plan.yaml read-only;
   record the slice shape.
3. Stamp Gate 1 only by running the literal `specify plan transition <name>
   approved` the plan printed.
4. /spec:execute loop. Answer only genuine clarification prompts needed to
   complete slices; never convert prompts into a script. Confirm the loop exits
   `all-done`, not stuck/failed/interrupted.
5. /spec:finalize <name> (first). Confirm push succeeded and it halts with
   `pr-not-merged`; record every PR number + URL.
6. HUMAN SEAM — stop and hand back to the operator: ask them to merge the PRs
   through their real forge, then resume. Do not merge PRs yourself; do not fake
   a forge.
7. /spec:finalize <name> (second). Confirm push is idempotent, PRs report
   MERGED, the plan archives. Record the archive path.
8. /spec:finalize <name> (third). Confirm the `no active plan` re-entry.

Confirm (self-grade on durable STRUCTURE only — never a byte/golden compare):
- For each assertion id, run the matching read-only check (`specify plan
  validate`, inspect plan.yaml, inspect .specify/archive/plans/, `gh pr view`)
  and record pass/fail/skipped with an evidence pointer.
- For each negative expectation, record held/violated/untested.
- Fill every section of the run-summary template into the stub
  <scenario-id>.md and set its Status to passed/failed/deferred.

Halt and judgment rules:
- Scenario #1 is the release blocker: on any fail, write the failure into
  01-pure-intent.md, STOP, and run no other scenario.
- For an ergonomics/judgment assertion you cannot deterministically verify
  (e.g. Gate-1 ergonomics, review-step-no-op), mark it `needs-human` and surface
  it for operator confirmation instead of guessing `pass`.
- On `deferred`, note the missing capability; remind the operator a linked
  follow-up issue and explicit release-owner sign-off are required before the
  gate can count it.
```
