# Self-heal on startup

Self-heal is the driver's reconciliation pass. It runs **once per `/spec:execute` invocation**, under the driver lock, immediately after the lock is acquired and before `specify plan next`. Its job is to make the plan agree with what actually finished on disk before the previous driver crashed — not to do any new work.

Two invariants frame the whole step. First, `.metadata.yaml:outcome` is the single authoritative signal: the driver never consults `journal.yaml`, tempfiles, or stderr transcripts to decide what happened. Second, nothing in self-heal speculates. Every ambiguity (missing outcome with no change dir, outcome field that contradicts `LifecycleStatus`, two archives with equal timestamps, …) halts the driver with a non-zero exit so a human can triage. A speculative transition could silently mark a failed change as `done` and that failure mode is strictly worse than "one extra triage per N runs".

## Algorithm

For every plan entry `E` whose `status` is `in-progress`, in the order they appear in `plan.yaml` (the loop tolerates more than one even though `specify plan next` would flag that as a validation warning):

1. **Locate the latest `.metadata.yaml` for `E.name`.**
   - First check `.specify/changes/<name>/.metadata.yaml` — the active change directory. If present, that is the file to read.
   - If absent, inspect `.specify/archive/`. Archive directory names end with `-<name>` (the `YYYY-MM-DD-<name>` convention from `specify change archive`). Multiple matches are legal: a change that failed and was later retried leaves one archive per attempt. Pick the most recent by `created-at` inside its `.metadata.yaml` (falling back to `defined-at`, then the directory's `YYYY-MM-DD` prefix); the outcome itself is surfaced by `specify change outcome show <name> --format json` once the active directory is back in place.
   - If nothing is found anywhere, the plan entry is `in-progress` but no change has ever been created — typically because the prior driver crashed between `specify plan transition pending → in-progress` (step 5) and `/spec:define` (step 6). Treat this as mid-change resume with `LifecycleStatus = None` — jump to step 3 below.

**Multi-repo self-heal.** For each `in-progress` entry `E` in `plan.yaml`, self-heal reads `E.project` from the plan entry. If non-null:
1. Resolve the target project directory from `registry.yaml` (same resolution as step 5a of the per-change algorithm).
2. Check workspace freshness for that slot. If `missing`, halt — same semantics as the main loop.
3. Look for `.specify/changes/<E.name>/.metadata.yaml` under the resolved project root instead of the initiating repo root. The classification logic (step 2 of self-heal) and recovery journal append (step 4) are unchanged.
4. Restore CWD to the initiating repo root after each entry's reconciliation.

For entries without a `project` field, self-heal follows the standard single-repo path.

2. **Read `.metadata.yaml.outcome` and act on it.**
   - `outcome.outcome == success` and `outcome.phase == merge` → the merge finished but the driver crashed before the terminal plan transition. Run:
     ```bash
     specify plan transition <name> done
     ``` No `status-reason` on success.
   - `outcome.outcome == success` and `outcome.phase ∈ {define, build}` → the phase finished but the driver crashed before launching the next phase. This is **not** a terminal state. Do NOT transition the plan. Fall through to step 3 with `LifecycleStatus ∈ {defined, complete}` to resume the next phase.
   - `outcome.outcome == failure` → run `/spec:drop <name> --reason "<outcome.summary>"` (same drop skill steps 11a/b invoke for a live failure — it is idempotent against an already-dropped change). Then:
     ```bash
     specify plan transition <name> failed --reason "<outcome.summary>"
     ``` `--reason` copied byte-for-byte from `outcome.summary`; never paraphrase or truncate.
   - `outcome.outcome == deferred` → same shape with `blocked`:
     ```bash
     /spec:drop <name> --reason "<outcome.summary>"
     specify plan transition <name> blocked --reason "<outcome.summary>"
     ```
   - The `outcome` field is **absent** and `LifecycleStatus` is non-terminal (`defining`, `defined`, `building`, `complete`) → no terminal outcome was ever stamped; the prior phase was in-flight at crash time. Fall through to step 3 with the on-disk `LifecycleStatus`.
   - The `outcome` field is malformed (unknown enum variant, missing `phase`, missing `summary`, …) or its `phase` contradicts `LifecycleStatus` (for example `phase: merge` with `status: defining`, or `outcome: success` with `status: dropped`) → **halt**. Emit the diagnostic line below with exit code `2` (`EXIT_VALIDATION_FAILED` — see the exit-code table in specify-cli `src/main.rs`). Leave the plan entry as `in-progress`. Do not append a recovery journal entry. Do not drop the change. Humans triage.

3. **Mid-change resume** (no terminal outcome yet). Reached when step 1 found no metadata, when step 2 saw success on `define`/`build`, or when step 2 saw no `outcome` field with a non-terminal `LifecycleStatus`. Read `LifecycleStatus` from `.metadata.yaml.status` (missing metadata counts as "None") and apply the resumption table:
   - `None` → invoke `/spec:define <name>` from scratch (step 6 of the per-change algorithm, skipping step 5 because `plan.yaml` already has the entry `in-progress`).
   - `defining` → resume / restart `/spec:define <name>`.
   - `defined` → invoke `/spec:build <name>` (step 7).
   - `building` → resume `/spec:build <name>`.
   - `complete` → invoke `/spec:merge <name>` (step 8).
   - `merged` or `dropped` → contradiction. **Halt** — same semantics as the step-2 ambiguity branch. Resumption does NOT write a plan transition — the entry stays `in-progress` while the resumed phase sequence completes; the normal step-9 phase-outcome read wraps up the run.

4. **Append one `type: recovery` entry to `journal.yaml`** for every entry self-heal actually resolved or resumed (not for halts):
   ```bash
   specify change journal append <name> <phase> recovery \
       --summary "Self-heal on startup: <action>" \
       --context "before=<plan-status>/<lifecycle-status>, after=<resolved-status-or-phase>"
   ``` where `<phase>` is the phase the recovery relates to (the `outcome.phase` for terminal cases; the phase about to run for mid-change resume). Example `<action>` strings:
   - `"applied terminal transition done after finding success outcome on merge"`
   - `"applied terminal transition failed after finding failure outcome on build"`
   - `"applied terminal transition blocked after finding deferred outcome on define"`
   - `"resumed mid-change build phase (LifecycleStatus=defined)"`

5. **Lock scope.** Self-heal runs **inside** the driver lock already acquired at step 2 of the outer run. There is no second acquire and no inner release: the whole reconciliation pass is part of the same critical section that wraps the per-change loop. Two `/spec:execute` invocations started at the same time cannot both enter self-heal — the second one fails at `specify plan lock acquire` with `Error::DriverBusy`.

## Diagnostic output

Every self-heal action emits exactly one line to stdout:

```text
Self-heal: no in-progress entries found.
Self-heal: <name> → done (merge success from prior run)
Self-heal: <name> → failed (build failure: "<outcome.summary verbatim>")
Self-heal: <name> → blocked (define deferred: "<outcome.summary verbatim>")
Self-heal: <name> — resuming <phase> (LifecycleStatus=<lifecycle>)
Self-heal halted: <name> has outcome=<outcome> phase=<phase> but LifecycleStatus=<lifecycle>. Manual triage required.
```

The halt variant is followed by `Exit 2` (`EXIT_VALIDATION_FAILED`; see the exit-code table in specify-cli `src/main.rs`); the other variants fall through to step 4 of the per-change algorithm. Fixture-pinned examples live under `fixtures/self-heal/`.

## Dry-run variant (report-only)

Under `--dry-run`, self-heal runs the same classification scan but performs **no writes**: no `specify plan transition`, no `specify change journal append`, no `/spec:drop`. Instead it prints what the writing path *would* do:

```text
Self-heal (dry-run): no in-progress entries found.
Self-heal (dry-run): <name> → done (if executed) — merge success from prior run
Self-heal (dry-run): <name> → failed (if executed) — build failure: "<outcome.summary verbatim>"
Self-heal (dry-run): <name> → blocked (if executed) — define deferred: "<outcome.summary verbatim>"
Self-heal (dry-run): <name> — would resume <phase> (LifecycleStatus=<lifecycle>)
Self-heal (dry-run): <name> — would halt (ambiguous outcome=<outcome> phase=<phase>, LifecycleStatus=<lifecycle>)
```

Terminal-resolution lines use "(if executed)" to make clear that no transition was written. The halt line still ends the run: dry-run self-heal emits it, releases the lock, and exits non-zero. It is the one place `--dry-run` exits non-zero on the happy startup path — the whole point of the halt is to surface ambiguity. Mid-change resume does nothing in dry-run anyway (the writing path also emits no plan transition for this case). The report-only path never appends `type: recovery` entries — recovery entries are the writing path's observable side-effect; dry-run has no side-effects by contract.
