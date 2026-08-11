# emery debt

Read-only baseline debt projection (RFC-86a D9) — the carried backlog looking ahead.

## Synopsis

```bash
emery debt [--format json]
```

## Description

Walks the baseline specs under `.emery/specs/` and lists every requirement whose status is `unknown` or `conflict` — the debt earlier changes deferred into the baseline through the merge fold. Each row carries the reason, originating change, deferral date, and age in days, parsed from the self-describing `Note: deferred — …` line the merge appended. Conflicts render separately from unknowns — a shipped-around contradiction is louder news than a shipped-around absence.

The projection reads the baseline alone — never archived fact logs — and writes nothing (no journal event). A missing or debt-free baseline projects cleanly (`baseline debt: none`). A carried gap row without a well-formed deferral note (merged outside the deferral surface, or hand-mangled prose) still lists as debt, just without its provenance detail.

`emery plan author` renders the same inventory as the `## Carried debt` section of the review prose it authors, so a corrective change is scoped with the backlog in view. Resolution flows through the ordinary path: new evidence in the corrective change's sources resolves carried rows at refine, and they disappear from the baseline at the next merge.

## Text output

```text
baseline debt (2 carried rows):
  unknown:
    orders/REQ-007 Password reset flow — deferred at the build gate under epoch 2026-08-01T02:11:04Z (change auth-login, 12 days)
  conflict:
    orders/REQ-011 Session timeout — deferred at the build gate under epoch 2026-08-01T02:11:04Z (change auth-login, 12 days)
```

## JSON output

With `--format json`, the body carries `rows[]` — each row `{ domain, req, status, summary, deferral? }`, where `deferral` is `{ reason, change, deferred-on, age-days }` and is omitted when the row carries no well-formed note.

## See also

- [`emery plan gaps`](plan.md#emery-plan-gaps) — the live change's gap inventory (dispositions included)
- [`emery plan archive`](plan.md#emery-plan-archive) — the carried-debt summary looking back
