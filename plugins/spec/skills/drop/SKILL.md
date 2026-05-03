---
name: specify-drop
description: Drop a change without merging specs into the baseline. Use when the user wants to discard a change that should not be merged normally.
argument-hint: "[change-name]"
---

# Drop

Drop a change without merging its specs into the baseline.

Deterministic bookkeeping — change selection, lifecycle transition, archive move — is delegated to the `specify` CLI. This skill drives the confirmation flow and the summary.

## Non-interactive mode

When invoked with `--reason`, skip the confirmation `AskQuestion` calls in steps 1–3; proceed directly to step 4 with the supplied reason. The change name must be provided explicitly as the positional argument. Exit code is 0 on a clean drop, non-zero only on CLI failure.

Non-interactive mode is how `/spec:execute` invokes this skill during `--loop`, supervised single-change runs, and self-heal reclaim of a `failure` / `deferred` outcome (see [`../execute/SKILL.md`](../execute/SKILL.md) steps 11b, 12b, and §"Self-heal on startup" step 2). The driver supplies a `--reason` string assembled from the upstream phase's outcome — see the verbatim-`summary` rule in [`../../references/phase-outcome-contract.md`](../../references/phase-outcome-contract.md). This skill forwards that string to `specify change drop` verbatim, without prompting.

When working plan-driven (a `plan.yaml` exists), after `specify change drop` succeeds the plan entry should transition to `failed` or `blocked` — `failed` for a build/test failure the human does not intend to retry automatically, `blocked` when a design question needs resolving before the entry is re-entered as `pending`:

```bash
specify plan transition <name> failed  --reason "<short rationale>"
specify plan transition <name> blocked --reason "<short rationale>"
```

This is an advisory note — this skill does not run the command itself. `/spec:execute` will run it automatically; in Layer 1 the human closes the loop.

## Phase outcome contract

This skill is the **drop** phase of the `/spec:execute` driver loop.
The shared phase contract — outcome values, journal kinds, plan-mutation rules,
the verbatim-`summary` rule, and the success/failure/deferred semantics — is
authored once at [`../../references/phase-outcome-contract.md`](../../references/phase-outcome-contract.md).

This phase's outcome-specific deltas:

- `success` — `specify change drop` exited zero: the change is archived with status `dropped` and the supplied `--reason` recorded in `.metadata.yaml`. The lifecycle stamp itself is the success signal — no separate `outcome set` call.
- `failure` — `specify change drop` returned a lifecycle violation (the change is already `merged`/`dropped`, the directory is malformed); record skill-side via `outcome set ... drop failure ...`.
- `deferred` — rare; an interactive cancel mid-flow or a precondition that needs human resolution before the drop is safe. Non-interactive runs from `/spec:execute` do not reach this path.

## Input

Optionally specify a change name. If omitted, check whether it can be inferred from conversation context. If vague or ambiguous, you MUST prompt for available changes.

## Steps

1. **Select the change**

   If a name is provided, use it. Otherwise run `specify status --format json` to enumerate active changes from the dashboard:

   - If only one entry exists, use it but confirm with the user.
   - If multiple, use the **AskQuestion tool** to let the user select.

   **IMPORTANT**: Always confirm the change name before dropping it.

   If `--reason` was supplied (non-interactive mode — see above), the change name must be the positional argument; skip the prompting fallback and the confirmation.

2. **Check lifecycle status**

   Run `specify change status <name> --format json` and inspect `status`:

   - `complete`: warn that the change appears ready to merge normally — `/spec:merge` may be the intended action.
   - `merged` or `dropped`: stop and tell the user the change is already finalized (the CLI would error with `lifecycle`, but surface it clearly before attempting).
   - Any other status: explain that dropping will discard the working change without promoting its specs.

   If `--reason` was NOT supplied, use the **AskQuestion tool** to confirm the user wants to drop the change. In non-interactive mode skip the prompt and proceed (the CLI still enforces the terminal-status check in step 4 — a `merged` / `dropped` change surfaces `Error::Lifecycle` there).

3. **Summarize what will happen**

   Before invoking the CLI, display a short summary:

   ```text
   ## Drop Preview: <change-name>

   - Change status will be set to `dropped`
   - The change directory will move under `.specify/archive/YYYY-MM-DD-<change-name>/`
   - No specs will be merged into `.specify/specs/`
   - Existing baseline specs remain unchanged
   ```

   If `--reason` was NOT supplied, use the **AskQuestion tool** to confirm:

   - **Proceed**: drop the change
   - **Cancel**: keep the change as-is

   In non-interactive mode skip this confirmation too; the preview may still be printed as an informational line but the skill does not wait for input.

4. **Drop and archive**

   Run:

   ```bash
   specify change drop <name> --reason "<user-supplied rationale>" --format json
   ```

   The CLI performs the lifecycle transition (enforcing the legal non-terminal → `dropped` edge), stamps `dropped-at`, records the optional reason in `.metadata.yaml.drop-reason`, and moves the directory under `.specify/archive/YYYY-MM-DD-<name>/`. The `archive-path` field in the JSON response names the final location.

5. **Display summary**

## Output On Success

```text
## Change Dropped

**Change:** <change-name>
**Archived to:** .specify/archive/YYYY-MM-DD-<change-name>/
**Reason:** <drop-reason>

No specs were merged into `.specify/specs/`.
The baseline remains unchanged.
```

## Guardrails

- Always confirm the change before dropping it.
- Do not merge or rewrite any files under `.specify/specs/`.
- Warn if the change is already `complete`, since `/spec:merge` may be the intended action.
- Stop if the change is already finalized as `merged` or `dropped`.
- Never hand-edit `.metadata.yaml` or the archive directory. `specify change drop` is the sole supported code path.
