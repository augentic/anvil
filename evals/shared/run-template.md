# Run: `<id>` — **`<pass | fail | deferred>`**

> Copy into `evals/runs/<id>.<result>.md` (e.g. `intent-only.pass.md`), fill against the live run, and update the scenario's status in the [catalog](../scenarios/README.md). Assertion ids and negative-expectations come from the scenario file — do not duplicate them in prose. On `fail` or `deferred`, file a follow-up issue in `augentic/specify` and link it from **Notes**.

## Context

- **Scenario:** `<id>`
- **Operator:** `<name or model identifier>`
- **CLI:** `<command -v specify>` — `<specify --version>`
- **Sandbox:** `<evals/.sandbox/<id>/>` (or list multiple roots for multi-fixture runs)

## Assertions

Grade each assertion id against its entry in the [assertion taxonomy](assertions.md): run the **probe** (verdict from probe output) or judge the **judgment flag** (verdict requires an evidence pointer). Probe output is the evidence — cite it (or its absence) in the **Evidence** column for any verdict other than `pass`; on `pass` the probe command itself suffices.

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `<assertion>` | `<pass \| fail \| skipped \| needs-human>` | `<probe output / evidence pointer — non-pass only>` |

**Negative expectations:** `<held | violated — see Notes>` (manual-by-design posture unchanged on a normal pass; expand only when violated).

## Deviations

List only where the run diverged from the scenario's **Setup** or **Invocation**. Write `none` when the script was followed verbatim.

- `<deviation or none>`

## Notes

Caveats, `needs-human` follow-ups, and links to follow-up issues (`fail` / `deferred`).

- `<notes>`

## Evidence

- **Reproduce:** `scripts/snapshot.sh "$SANDBOX"` (re-run against the retained sandbox; do not paste full output on `pass`)
- **Retained at:** `<sandbox path(s)>`
- **Key paths:** `<plan.yaml`, slice dirs, journal, PR URLs — as relevant>`

---

### Fail / deferred — add these sections

Skip on `pass`.

#### Fault

- **Fault domain:** `<plan | review | execute | finalize-push | finalize-pr-observation | finalize-archive | synthesis | operator-error | unknown>`
- **Follow-up issue:** `<URL or none>`

#### Failure detail

Paste verbatim output for the failing step and a trimmed snapshot excerpt if it helps reproduction.

```text
<failing command + output>
```

#### Plan structure

Use for multi-slice or workspace scenarios; one line is enough for N=1 (e.g. `1 slice, status done, drained`).

| Slice | Project | Sources | Status |
| --- | --- | --- | --- |
| `<slice>` | `<project>` | `<sources>` | `<status>` |
