# Operator meta prompts

Reusable prompts an operator pastes into a live `cursor-agent` session to drive a single `lifecycle` acceptance scenario. They let the agent do the clerical and deterministically-checkable work — environment setup, driving the slash-command lifecycle, capturing per-stage output, self-grading the structural assertions, and filling the run-summary — while leaving the irreducible human seams to the operator. Every scenario sanctions an agent-as-operator (`backend: manual`, "a human **or** agent follows this script").

**These prompts are operator aids, not a harness.** They are pasted interactively per run; they add no checked-in runner, no CI target, no fake forge, and no golden-output comparison, so every scenario `negative-expectation` still holds. Keep them as documentation next to the scenarios, never as an unattended job.

The three human seams the prompts always hand back to the operator:

1. **Real forge merges** between the two `/spec:finalize` invocations — never faked.
2. **Ergonomics / judgment assertions** the agent cannot deterministically verify — marked `needs-human` for operator confirmation.
3. **`deferred` and `pure-intent` sign-off** — a `deferred` entry needs a linked follow-up issue and release-owner sign-off; `pure-intent` is the release blocker.

Drive each scenario with Prompt A first, then Prompt B. Replace `<id>` with the scenario directory id (e.g. `01-pure-intent`).

## Prompt A — setup

```text
You are the acceptance SETUP operator for Specify scenario <id>.
Goal: bring a fresh, disposable environment to the exact pre-invocation state the
scenario describes, using only real `specify` CLI commands. Do NOT drive any
/spec:* command yet.

Inputs:
- `specify`: the automated surface (runbook step 1) already ran `make acceptance`, which
  symlinks the build under test into ~/.local/bin. Confirm with `specify --version` before any
  other call; if the bare command does not resolve to that build, prepend the symlink dir to
  PATH (`export PATH="$HOME/.local/bin:$PATH"`) or call the absolute
  `../specify-cli/target/release/specify` path (see acceptance/shared/setup.md).
- Scenario: acceptance/lifecycle/<id>.md
- Shared setup: acceptance/shared/setup.md (Prerequisites + the matching
  single-project or cross-repo workspace setup, and the brief the scenario names).

Do, in order, capturing each exact command and its verbatim output:
1. Create the disposable directories the scenario/setup names under the pinned
   sandbox `acceptance/.sandbox/<id>/` (recreate it clean, per
   acceptance/shared/setup.md). Never reuse an existing project or a non-empty
   Specify state.
2. Run the init / registry-add / brief-file steps verbatim using the bare
   `specify` command (which the PATH export resolves to the build under test).
3. Run the scenario's validation step (e.g. `specify registry validate`) and
   confirm it exits 0.
4. STOP before /spec:plan. Report: the sandbox path, every created path, and the
   captured command log.

Guardrails:
- Real CLI only. Do not add or stub `gh`, a forge, a test runner, or a CI step.
- On any failure, halt and report the failing command + output; do not proceed.
```

## Prompt B — run + confirm

```text
You are the acceptance RUN+CONFIRM operator for Specify scenario <id>.
Drive the live agent workflow end to end, capture evidence at every stage,
self-verify the structurally-checkable assertions, and fill the run-summary.
Pause at the human-only seams; never fabricate a result.

Inputs:
- The environment left by the SETUP prompt (the `acceptance/.sandbox/<id>/`
  sandbox, the PATH-resolved `specify` build).
- acceptance/lifecycle/<id>.md (Invocation, Assertions, Negative
  Expectations).
- acceptance/shared/run-summary-template.md field-set.
- acceptance/shared/inspect.md (the read-only render verbs for reviewing state).

Drive, in order, recording the exact invocation and verbatim output for each
stage the scenario's `stages` declares:
1. /spec:plan ... exactly as the scenario specifies. Confirm it stops at the
   hand-off / `pending` and prints the literal Gate-1 transition command. Do NOT
   auto-stamp approval.
2. Review seam: run `specify plan validate` and inspect plan.yaml read-only;
   record the slice shape. Use the read-only render verbs in
   acceptance/shared/inspect.md to review state through the CLI rather than
   hunting raw files.
3. If the scenario executes: stamp Gate 1 only by running the literal
   `specify plan transition <name> approved` the plan printed.
4. If the scenario executes: /spec:execute loop. Answer only genuine
   clarification prompts; never convert prompts into a script. Confirm the loop
   exits `all-done`, not stuck/failed/interrupted.
5. If the scenario finalizes: /spec:finalize <name> (first). Confirm push
   succeeded and it halts with `pr-not-merged`; record every PR number + URL.
6. HUMAN SEAM — stop and hand back: ask the operator to merge the PRs through
   their real forge, then resume. Do not merge PRs yourself; do not fake a forge.
7. If the scenario finalizes: /spec:finalize <name> (second). Confirm push is
   idempotent, PRs report MERGED, the plan archives. Record the archive path.
   Then /spec:finalize <name> (third); confirm the `no active plan` re-entry.

Confirm (self-grade on durable STRUCTURE only — never a byte/golden compare):
- For each assertion id, run the matching read-only check (`specify plan
  validate`, inspect plan.yaml, inspect .specify/archive/plans/, `gh pr view`)
  and record pass/fail/skipped with an evidence pointer.
- For each negative expectation, record held/violated/untested.
- Capture an artefact snapshot of the sandbox
  (`scripts/acceptance-snapshot.sh "$SANDBOX"`) and paste it into the
  run-summary's **Artefact snapshot** section, so the record is self-contained.
- Fill the run-summary template and file it under acceptance/runs/
  <id>-<date>.md; update the scenario's status in the catalog.

Halt and judgment rules:
- pure-intent is the release blocker: on any fail, record the failure, STOP, and
  run no other scenario.
- For an ergonomics/judgment assertion you cannot deterministically verify, mark
  it `needs-human` and surface it for operator confirmation instead of guessing.
- On `deferred`, note the missing capability; remind the operator a linked
  follow-up issue and explicit release-owner sign-off are required.
```
