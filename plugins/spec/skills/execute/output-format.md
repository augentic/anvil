# Output format

This file pins the rendered output for each mode. Behavioural fixtures pinning each shape live in [fixtures.md](fixtures.md).

## `--dry-run`

Emits a compact snapshot of the plan's current state and the change the driver *would* run next. The progress line has exactly six status counters in a fixed order so downstream parsers see a stable shape.

```text
[dry-run] /spec:execute — <plan-name>

Progress: done <N>, in-progress <N>, pending <N>, blocked <N>, failed <N>, skipped <N> (total <N>)

Next: <name> (sources: [<sources>])

No changes written.
```

Variants the skill picks based on `specify plan next`:

- `reason: "in-progress"` — replace the `Next:` line with: `Active: <name> (driver would resume/adopt this entry)`
- `reason: "all-done"` — replace with: `Initiative complete — no eligible changes remain.`
- `reason: "stuck"` — replace with: `Stuck — no eligible changes. Pending: [<names>]. Blocked: [<names>]. Failed: [<names>].`

The canonical shape is pinned by `fixtures/dry-run/`.

Under multi-repo routing, the `--dry-run` output includes a `Routing:` diagnostic line:

```text
[dry-run] Routing: <name> → <project> (<resolved-path>)
```

## Supervised / per-change transcript

Three variants (success / failure / deferred), each pinned by a behavioural fixture under `fixtures/single-change/`.

### Success

```text
## /spec:execute — <plan-name>

### Initiative: <plan-name>
Progress: done <N>, in-progress <N>, pending <N>, blocked <N>, failed <N>, skipped <N> (total <N>)

---

### Processing: <name> (sources: [<sources>])

Step 1/3: define
  - extract sub-step (via /spec:extract)
      Source: <path>
      Artifacts: specs/<name>/spec.md, design.md ✓
  Artifacts: proposal.md, specs, design.md, tasks.md ✓

Step 2/3: build
  Tasks: N/M complete ✓

Step 3/3: merge
  Baseline updated: .specify/specs/<name>/spec.md ✓
  Status: done

⚠ Cross-project contract warnings           # OPTIONAL — see multi-repo.md
  Contract: <produced-contract-path>
  Consumers checked: <N> (<consumer-name>[, ...])

  <consumer-name> (.specify/workspace/<consumer-name>/):
    - <change-kind> at <locator>
    - <change-kind> at <locator>

  Recorded <M> finding(s) to .specify/changes/<name>/journal.yaml.
  Action needed: review the warning(s); the consumer change(s) may need a follow-up.
```

The `(sources: [...])` suffix is rendered only when the plan entry has `sources`; greenfield entries become `### Processing: <name> (greenfield)`. The extract sub-step block inside `Step 1/3: define` is elided when the entry has no `sources`.

The `⚠ Cross-project contract warnings` block is rendered only when (a) the merged change touches a contract listed in the producer's `registry.yaml:contracts.produces` list AND (b) at least one consumer's verifier invocation (the format-appropriate `/contract:*` skill in its verifier intent, `--mode cross-project`) reports `summary.total-findings > 0`. See [multi-repo.md](multi-repo.md) for the full algorithm and the per-finding journal payload schema.

### Failure

```text
## /spec:execute — <plan-name>

### Initiative: <plan-name>
Progress: done <N>, in-progress <N>, pending <N>, blocked <N>, failed <N>, skipped <N> (total <N>)

---

### Processing: <name> (sources: [<sources>])

Step 1/3: define
  Artifacts: proposal.md, specs, design.md, tasks.md ✓

Step 2/3: build
  ✗ Build failed — change dropped, plan entry transitioned to failed

  Summary: <outcome.summary verbatim>
  Journal: .specify/changes/<name>/journal.yaml
  Action needed: Fix the underlying error, then retry via
    specify plan transition <name> pending
  Status: failed
```

The phase that fails is whichever returned `outcome: failure`; the step header names that phase. The `Summary:` line is the phase's `outcome.summary` copied byte-for-byte.

### Deferred

```text
## /spec:execute — <plan-name>

### Initiative: <plan-name>
Progress: done <N>, in-progress <N>, pending <N>, blocked <N>, failed <N>, skipped <N> (total <N>)

---

### Processing: <name> (greenfield)

Step 1/3: define
  ⚠ Question recorded — change deferred to blocked

  Question: <outcome.summary verbatim>
  Journal: .specify/changes/<name>/journal.yaml
  Action needed: Enrich the plan description (specify plan amend …) with the missing
    detail, then unflag (blocked → pending) to retry.
  Status: blocked
```

The `⚠ Question recorded — change deferred to blocked` line is the canonical deferred banner; do not reword. `Question:` carries the phase's `outcome.summary` verbatim.

When the deferral classification was `registry-amendment-required` (RFC-9 §2B), an additional `Proposed registry amendment` block is appended immediately after the `Status: blocked` line. The block layout, field names, and "Action needed" line are pinned by [per-change-algorithm.md](per-change-algorithm.md) → §"Surface the proposal in the per-change transcript". The block is omitted entirely on plain `deferred` outcomes.

## Terminal summary (`--loop` exit)

At the end of every `--loop` run — success, interruption, or halt — `/spec:execute` emits a single terminal summary block. Fixtures under `fixtures/loop/` pin one example per `Completion:` value.

```text
## /spec:execute — <plan-name> — terminated

### Final state
Progress: done <N>, in-progress <N>, pending <N>, blocked <N>, failed <N>, skipped <N> (total <N>)

Completion: <all-done | stuck | halted | driver-interrupted>

Blocked:
  - <name> (status-reason: "<verbatim status-reason>")
  - ...

Failed:
  - <name> (status-reason: "<verbatim status-reason>")
  - ...

Pending (dependencies not satisfied):
  - <name> (waits on: <unmet-dep-name>[, <unmet-dep-name> ...])
  - ...

Next action: <context-sensitive instruction>
```

Section rules:

- `Progress:` always enumerates all six statuses in the fixed order `done, in-progress, pending, blocked, failed, skipped`, followed by `(total <N>)`. Zeros are rendered explicitly.
- `Blocked:` / `Failed:` / `Pending (dependencies not satisfied):` sections are omitted entirely (including the heading) when their bucket is empty. An `all-done` run therefore renders only the `Final state` / `Completion` / `Next action` lines.
- `Blocked` and `Failed` entries quote `status-reason` byte-for-byte from the plan entry. Paraphrasing is forbidden.
- `Pending (…)` lists each pending entry alongside the `depends-on` entries whose status is not `done`.
- `in-progress` count is zero on all classifications except `driver-interrupted`, where the active change is preserved as `in-progress` for self-heal to reclaim on the next run.

### `Completion:` classification

| Classification | Condition | Next action template |
|---|---|---|
| `all-done` | Every entry's status is in `{done, skipped}`. | `Initiative complete. Land remote PRs (specify workspace merge or merge them by hand on the forge), then close out via specify plan finalize — see [specify plan](../../../../docs/reference/cli/plan.md#specify-plan-finalize) for the closure verb.` |
| `stuck` | Some entries remain in `{pending, blocked, failed}` but none are eligible (pending entries have unmet deps; no eligible sibling exists). | `Resolve blocked/failed entries (specify plan amend + specify plan transition <name> blocked → pending / failed → pending) or accept the partial initiative and run specify plan archive --force.` |
| `halted` | Self-heal detected an ambiguous on-disk state on startup and refused to speculate. Individual mid-loop failures or deferrals do NOT reach `halted`. | `Manually triage the halted change: inspect .specify/changes/<name>/.metadata.yaml against plan.yaml, repair the contradiction, then re-run /spec:execute --loop.` |
| `driver-interrupted` | SIGINT or SIGTERM arrived mid-run. The current phase finished (or no phase was in flight), subsequent phases were skipped, the active plan entry is still `in-progress`, the lock was released. | `Re-run /spec:execute --loop — self-heal will reclaim the interrupted change on the next startup.` |

The distinction between `stuck` and `halted` matters for operator routing: `stuck` means the plan is well-formed but needs human-level priority decisions; `halted` means the on-disk state itself is inconsistent and needs forensic triage before the loop can run safely again.

### Exit codes

| Classification | Exit code |
|---|---|
| `all-done` | 0 |
| `stuck` | 0 (driver did nothing wrong; partial completion is observable via the plan) |
| `halted` | `2` (`EXIT_VALIDATION_FAILED` — see the exit-code table in specify-cli `src/main.rs`) |
| `driver-interrupted` | non-zero (typically 130 for SIGINT, 143 for SIGTERM, inherited from the host shell's signal conventions) |
