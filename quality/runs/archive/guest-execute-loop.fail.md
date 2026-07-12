# Run: `guest-execute-loop` — **fail**

## Context

- **Scenario:** `guest-execute-loop`
- **Operator:** Cursor agent (Fable 5), driving `quality/profiles/workflow/guest-execute-loop.sh` on the RFC-61 `specify-wasm` branch
- **CLI:** `engine/target/debug/specify` — `specify 0.27.2` (in-tree build under test; `runtime` from the same workspace)
- **Sandbox:** `quality/.sandbox/guest-execute-loop/`

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `guest-loop-drained` | fail | `plan execute` exited 2 with the typed stop `plan-execute-stopped: … stop refine-failed (greeting-service): discovery.md not found` — the upstream survey never wrote `discovery.md` (see Fault); `specify plan status --format json` reports `"action":"refine"`, entry `in-progress` |
| `guest-journal-cadence` | fail | journal carries only `plan.transition.approved`, `source.execution.agent`, `plan.entry.advanced` — the cadence stops where the model backend failed |
| `guest-generated-crate-verifies` | skipped | no build ran; no `crates/` output to verify |
| `guest-marker-released` | pass | `test ! -f .specify/guest.lock` → released (released on the typed-stop return, not only on drain) |
| `guest-spec-sensible` | skipped | no synthesis ran |

**Negative expectations:** held.

## Deviations

- No follow-up issue filed, deviating from the run template's on-`fail` step: the fault domain is `operator-error` (missing CLI credentials in the run environment), not a framework defect, and this record on the `specify-wasm` branch is the tracking artifact until the post-login re-run.
- Otherwise none — the driver followed the scenario's Setup and Invocation verbatim.

## Notes

- Everything up to the model boundary worked as designed: the nine-guest deployment composed, `source survey intent` dispatched through `augentic:specify/source` to the committed intent guest, the guest's judgment leg reached `omnia:model/completion`, and the cursor backend's spawn failure came back across both guest boundaries as the typed `seam-dispatch-failed` envelope (exit 1) — no trap. `plan execute` then claimed the entry and parked with the typed `refine-failed` stop, leaving a consistent, resumable state.
- The fault is environment authentication, not the framework: `cursor-agent status` reports an IDE login, but `--print` mode (what `omnia-cursor` spawns) requires CLI credentials — `cursor-agent login` or `CURSOR_API_KEY` — and neither is present in the run environment. A bare `cursor-agent --print "say OK"` outside the deployment fails identically.
- Re-run is one operator action away: complete `cursor-agent login` (or export `CURSOR_API_KEY`), then re-run `bash quality/profiles/workflow/guest-execute-loop.sh`.
- Follow-up: re-run under the parent RFC-61 session once CLI auth is in place; no separate issue filed — this record is the branch's tracking artifact until then.

## Evidence

- **Reproduce:** `bash quality/profiles/workflow/guest-execute-loop.sh` (recreates the sandbox; requires authenticated `cursor-agent`)
- **Retained at:** `quality/.sandbox/guest-execute-loop/`
- **Key paths:** `plan.yaml` (entry `in-progress`), `.specify/journal.jsonl` (three events, above), `guest-execute-loop.log`

---

#### Fault

- **Fault domain:** `operator-error`
- **Follow-up issue:** none — tracked by this record on the `specify-wasm` branch; unblocked by `cursor-agent login` in the run environment.

#### Failure detail

```text
==> runtime -- source survey intent
error: seam-dispatch-failed: seam `survey` dispatch to `source:intent` failed: internal: backend failure: cursor-agent exited with exit status: 1: Error: Authentication required. Please run 'agent login' first, or set CURSOR_API_KEY environment variable.
==> runtime -- plan execute
error: plan-execute-stopped: the execute loop drains the plan: stop refine-failed (greeting-service): discovery.md not found at ./discovery.md — Fix the failure, then retry /spec:refine for the slice. The plan entry stays in-progress.
```

#### Plan structure

| Slice | Project | Sources | Status |
| --- | --- | --- | --- |
| `greeting-service` | — | `intent` | in-progress |
