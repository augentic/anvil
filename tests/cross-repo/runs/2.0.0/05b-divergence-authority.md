# Scenario #5b -- `[divergence]` from authority resolution

## Source

[`rfcs/archive/rfc-25-workflow.md` §Acceptance scenarios](../../../../rfcs/archive/rfc-25-workflow.md#acceptance-scenarios), row #5b.

> Combined-evidence slice where docs and legacy code disagree at different authority classes, for example docs say "30 minutes" expiry while code observed 24 hours.

**Stress-tests:** `Status: divergence` written; documentation authority wins as the operative requirement; behaviour preserved as inline commentary; lifecycle transitions to `refined`; operator may hand-edit before build.

## Run-summary

Status: **pending**

Operator: copy the field-set from [`tests/cross-repo/run-summary-template.md`](../../run-summary-template.md) into this file, fill every section against the live run, and update the **Status:** line above.
