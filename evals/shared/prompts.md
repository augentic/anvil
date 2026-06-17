# Operator prompts

Reusable prompts an operator pastes into a live `cursor-agent` session to drive a single platform eval scenario. They let the agent do the clerical and deterministically-checkable work — environment setup, driving the slash-command lifecycle, capturing per-stage output, self-grading the structural assertions, and filling the run-summary — while leaving the irreducible human seams to the operator. Every scenario sanctions an agent-as-operator ("a human **or** agent follows this script").

**These prompts are operator aids, not a harness.** They are pasted interactively per run; they add no checked-in runner, no CI target, no fake forge, and no golden-output comparison, so every scenario `negative-expectation` still holds. Multi-step execute scenarios may also be replayed via checked-in helpers under [`evals/drivers/`](../drivers/README.md) — those scripts shell out to the real CLI and are not wired into CI.

The three human seams the prompts always hand back to the operator:

1. **Real forge merges** between the two `/spec:finalize` invocations — never faked.
2. **Ergonomics / judgment assertions** the agent cannot deterministically verify — marked `needs-human` for operator confirmation.
3. **`deferred` and `intent-only` sign-off** — a `deferred` entry needs a linked follow-up issue and release-owner sign-off; `intent-only` is the release blocker.

Drive each scenario with Prompt A first, then Prompt B. Replace `<id>` with the scenario directory id (e.g. `intent-only`).

## Prompt A — setup

```text
You are the eval SETUP operator for Specify scenario <id>.
Goal: bring a fresh, disposable environment to the exact pre-invocation state the
scenario describes, using only real `specify` CLI commands. Do NOT drive any
/spec:* command yet.

Inputs:
- `specify`: the deterministic surface (runbook step 1) already ran `make install-cli`, which
  symlinks the build under test into ~/.local/bin. Confirm with `specify --version` before any
  other call; if the bare command does not resolve to that build, prepend the symlink dir to
  PATH (`export PATH="$HOME/.local/bin:$PATH"`) or call the absolute
  `engine/target/release/specify` path (see evals/shared/setup.md).
- Scenario: evals/scenarios/<id>.md
- Shared setup: evals/shared/setup.md (Prerequisites + the matching
  single-project or cross-repo workspace setup, and the brief the scenario names).

Do, in order, capturing each exact command and its verbatim output:
1. Create the disposable directories the scenario/setup names under the pinned
   sandbox `evals/.sandbox/<id>/` (recreate it clean, per
   evals/shared/setup.md). Never reuse an existing project or a non-empty
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
You are the eval RUN+CONFIRM operator for Specify scenario <id>.
Drive the live agent workflow end to end, capture evidence at every stage,
grade the assertions through their probes, and fill the run-summary.
Pause at the human-only seams; never fabricate a result.

Inputs:
- The environment left by the SETUP prompt (the `evals/.sandbox/<id>/`
  sandbox, the PATH-resolved `specify` build).
- evals/scenarios/<id>.md (Invocation, Assertions, Negative
  Expectations).
- evals/shared/assertions.md (the assertion taxonomy: per-id probe or
  judgment flag).
- evals/shared/run-template.md (simplified pass/fail layout).
- evals/shared/inspect.md (the read-only render verbs for reviewing state).

Drive, in order, recording the exact invocation and verbatim output for each
stage the scenario's `stages` declares:
1. /spec:plan ... exactly as the scenario specifies. Confirm it stops at the
   hand-off / `pending` and prints the literal Gate-1 transition command. Do NOT
   auto-stamp approval.
2. Review seam: run `specify plan validate` and inspect plan.yaml read-only;
   record the slice shape. Use the read-only render verbs in
   evals/shared/inspect.md to review state through the CLI rather than
   hunting raw files.
3. If the scenario executes: stamp Gate 1 only by running the literal
   `specify plan transition <name> approved` the plan printed.
4. If the scenario executes: /spec:execute loop. Answer only genuine
   clarification prompts; never convert prompts into a script. Confirm the loop
   exits `all-done`, not stuck/failed/interrupted.
5. If the scenario finalizes: /spec:finalize <name>. Confirm the branches
   pushed (per-project status `pushed`) and the plan archived in the same run;
   record the pushed branches and the archive path. Specify does not create,
   observe, or merge pull requests — opening PRs is an operator step done by
   hand outside Specify.
6. If the scenario finalizes: re-run /spec:finalize <name>; confirm the
   `no active plan` re-entry exits 0.

Confirm (grade on durable STRUCTURE only — never a byte/golden compare):
- For each assertion id, look it up in evals/shared/assertions.md: run its
  **probe** verbatim and record pass/fail/skipped from the probe output, or —
  for a **judgment flag** — record the named evidence pointer and mark
  needs-human unless the evidence is unambiguous. Fill the **Assertions**
  table; cite probe output in the Evidence column for non-pass rows.
- On a normal pass, record negative expectations as one line: held.
- Point **Evidence** at the retained sandbox and `scripts/snapshot.sh "$SANDBOX"`
  (do not paste full snapshot output on pass).
- Fill the run-summary template and file it under evals/runs/
  <id>.<result>.md; set the title verdict; update the scenario's status
  in the catalog.

Halt and judgment rules:
- intent-only is the release blocker: on any fail, record the failure, STOP, and
  run no other scenario.
- For an ergonomics/judgment assertion you cannot deterministically verify, mark
  it `needs-human` and surface it for operator confirmation instead of guessing.
- On `deferred`, note the missing capability; remind the operator a linked
  follow-up issue and explicit release-owner sign-off are required.
```
