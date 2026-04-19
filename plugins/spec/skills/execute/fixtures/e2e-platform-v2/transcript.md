# e2e-platform-v2 — `/spec:execute --loop` drains RFC-2 §"The Plan"

This is the Layer 2 exit-gate meta-fixture. The seed is the full
`platform-v2` plan from [RFC-2 §"The Plan"](../../../docs/links.md#rfc-2-the-plan)
verbatim — nine entries spanning every shape the driver must handle:
greenfield, `sources`-only, `affects`-only, combined, pre-failed,
mid-run-crashed. `/spec:execute --loop` starts against this seed and
drives the plan until no eligible change remains.

There is no automated harness; this is an authoring pin covering the
argument-resolution plumbing introduced in L2.I end-to-end.

## Seeded state

- `plan.yaml.before` is the RFC-2 §"The Plan" YAML unchanged:
  - `user-registration`: **done** — specs merged in a prior run.
  - `email-verification`: **in-progress** — the prior driver's
    `/spec:merge` stamped `outcome: success` then crashed before
    running `specify initiative transition done`.
    `metadata-email-verification-crashed.yaml` is the illustrative
    `.metadata.yaml` snapshot that self-heal reads.
  - `registration-duplicate-email-crash`: pending,
    `affects: [user-registration]`, no `sources`.
  - `notification-preferences`: pending, greenfield (neither
    `sources` nor `affects`), description-only.
  - `extract-shared-validation`: pending,
    `affects: [user-registration, email-verification]`, no `sources`,
    depends on `email-verification`.
  - `product-catalog`: pending, `sources: [monolith]`,
    depends on `extract-shared-validation`.
  - `shopping-cart`: pending, `sources: [orders]` (git URL),
    depends on `product-catalog` and `user-registration`.
  - `checkout-api`: **failed**, `sources: [payments]` (git URL),
    `status-reason` already stamped — the prior run gave up on this
    one after a type mismatch against the payment gateway contract.
    Intentionally preserved as `failed`; the operator has chosen
    not to retry.
  - `checkout-ui`: pending, `sources: [frontend]` (git URL),
    depends on `checkout-api` (which is `failed`, so this entry is
    permanently ineligible until the operator triages upstream).

Top-level `sources` map:
```yaml
monolith: /path/to/legacy-codebase           # local path
orders:   git@github.com:org/orders-service.git    # git URL
payments: git@github.com:org/payments-service.git  # git URL
frontend: git@github.com:org/web-app.git           # git URL
```

Every `sources` key referenced by an entry resolves; no
`unknown-source` validation errors exist in the seed.

## Driver timeline

```text
$ /spec:execute --loop

# step 1: project resolution — silent on success.
# step 2: acquire the driver lock — single acquire for the whole run.

# step 3: self-heal (writing path).
#   Entry email-verification: status=in-progress.
#   .specify/changes/email-verification/.metadata.yaml: outcome.outcome=success,
#   outcome.phase=merge.
#   Self-heal applies the terminal transition.
Self-heal: email-verification → done (merge success from prior run)
#   specify initiative transition email-verification done
#   specify change journal-append email-verification merge recovery
#       --summary "Self-heal on startup: applied terminal transition done after finding success outcome on merge"
#       --context "before=in-progress/merged, after=done"

# Post-self-heal plan state:
#   user-registration: done, email-verification: done,
#   everything else pending except checkout-api: failed.

# ───────────────────────────────────────────────────────────
# Iteration 1 — registration-duplicate-email-crash
# ───────────────────────────────────────────────────────────
# specify initiative next → { "next": "registration-duplicate-email-crash" }
# Tie-break: plan list order. Both registration-duplicate-email-crash
# and notification-preferences are eligible (user-registration is done);
# the former appears first in plan list order.

# Argument resolution:
#   sources: []                 → no --source flags
#   affects: [user-registration] → --affects user-registration
# Pinned invocation (see fixtures/field-wiring/affects-only/):
#   /spec:define registration-duplicate-email-crash --affects user-registration

# specify initiative transition registration-duplicate-email-crash in-progress
# /spec:define registration-duplicate-email-crash --affects user-registration → success
# /spec:build  registration-duplicate-email-crash                             → success
# /spec:merge  registration-duplicate-email-crash                             → success
# specify initiative transition registration-duplicate-email-crash done

### Processing: registration-duplicate-email-crash (affects: [user-registration])

Step 1/3: define
  Artifacts: proposal.md, specs, design.md, tasks.md ✓
Step 2/3: build
  Tasks: 3/3 complete ✓
Step 3/3: merge
  Baseline updated: .specify/specs/user-registration/spec.md ✓
  Status: done

# ───────────────────────────────────────────────────────────
# Iteration 2 — notification-preferences (greenfield)
# ───────────────────────────────────────────────────────────
# specify initiative next → { "next": "notification-preferences" }

# Argument resolution:
#   sources: []  →  no --source flags
#   affects: []  →  no --affects flags
# Invocation: /spec:define notification-preferences

# specify initiative transition notification-preferences in-progress
# /spec:define notification-preferences → success
# /spec:build  notification-preferences → success
# /spec:merge  notification-preferences → success
# specify initiative transition notification-preferences done

### Processing: notification-preferences (greenfield)

Step 1/3: define
  Artifacts: proposal.md, specs, design.md, tasks.md ✓
Step 2/3: build
  Tasks: 4/4 complete ✓
Step 3/3: merge
  Baseline updated: .specify/specs/notification-preferences/spec.md ✓
  Status: done

# ───────────────────────────────────────────────────────────
# Iteration 3 — extract-shared-validation
# ───────────────────────────────────────────────────────────
# specify initiative next → { "next": "extract-shared-validation" }
# (email-verification is done now, so this entry's dep is satisfied.)

# Argument resolution:
#   sources: []                               → no --source flags
#   affects: [user-registration, email-verification]
#                                             → --affects user-registration --affects email-verification
# Invocation:
#   /spec:define extract-shared-validation --affects user-registration --affects email-verification

# specify initiative transition extract-shared-validation in-progress
# /spec:define … → success
# /spec:build  … → success
# /spec:merge  … → success
# specify initiative transition extract-shared-validation done

### Processing: extract-shared-validation (affects: [user-registration, email-verification])

Step 1/3: define
  Artifacts: proposal.md, specs, design.md, tasks.md ✓
Step 2/3: build
  Tasks: 6/6 complete ✓
Step 3/3: merge
  Baseline updated: .specify/specs/user-registration/spec.md,
                    .specify/specs/email-verification/spec.md ✓
  Status: done

# ───────────────────────────────────────────────────────────
# Iteration 4 — product-catalog (local-path source)
# ───────────────────────────────────────────────────────────
# specify initiative next → { "next": "product-catalog" }

# Argument resolution:
#   sources: [monolith] — resolve "monolith" against top-level map:
#                         /path/to/legacy-codebase (local filesystem path)
#                         → --source monolith=/path/to/legacy-codebase
#   affects: [] → no --affects flags
# Invocation:
#   /spec:define product-catalog --source monolith=/path/to/legacy-codebase

# specify initiative transition product-catalog in-progress
# /spec:define product-catalog --source monolith=/path/to/legacy-codebase → success
#   (define's brief pipeline invokes /spec:extract with the resolved
#    path; no clone — path is local.)
# /spec:build  product-catalog → success
# /spec:merge  product-catalog → success
# specify initiative transition product-catalog done

### Processing: product-catalog (sources: [monolith])

Step 1/3: define
  - extract sub-step (via /spec:extract)
      Source: /path/to/legacy-codebase
      Artifacts: specs/product-catalog/spec.md, design.md ✓
  Artifacts: proposal.md, specs, design.md, tasks.md ✓
Step 2/3: build
  Tasks: 5/5 complete ✓
Step 3/3: merge
  Baseline updated: .specify/specs/product-catalog/spec.md ✓
  Status: done

# ───────────────────────────────────────────────────────────
# Iteration 5 — shopping-cart (git-URL source)
# ───────────────────────────────────────────────────────────
# specify initiative next → { "next": "shopping-cart" }

# Argument resolution:
#   sources: [orders] — resolve "orders" against top-level map:
#                        git@github.com:org/orders-service.git (git URL)
#                        → --source orders=git@github.com:org/orders-service.git
#   affects: [] → no --affects flags
# Invocation:
#   /spec:define shopping-cart --source orders=git@github.com:org/orders-service.git

# The driver does NOT clone here. /spec:define's brief pipeline
# invokes /spec:extract, which in turn consults git-cloner to
# materialize the tree into its clone cache. The --source value
# travels through as a URL string.

# specify initiative transition shopping-cart in-progress
# /spec:define shopping-cart --source orders=… → success
# /spec:build  shopping-cart → success
# /spec:merge  shopping-cart → success
# specify initiative transition shopping-cart done

### Processing: shopping-cart (sources: [orders])

Step 1/3: define
  - extract sub-step (via /spec:extract)
      Source: git@github.com:org/orders-service.git
      Artifacts: specs/shopping-cart/spec.md, design.md ✓
  Artifacts: proposal.md, specs, design.md, tasks.md ✓
Step 2/3: build
  Tasks: 7/7 complete ✓
Step 3/3: merge
  Baseline updated: .specify/specs/shopping-cart/spec.md ✓
  Status: done

# ───────────────────────────────────────────────────────────
# Iteration 6 (terminating) — no eligible change remains
# ───────────────────────────────────────────────────────────
# specify initiative next →
#   { "next": null, "reason": "stuck",
#     "pending": ["checkout-ui"],
#     "failed":  ["checkout-api"] }
#
# checkout-api is failed; checkout-ui's only dep is checkout-api.
# No pending entry has all deps done; specify initiative next classifies
# this as "stuck". The loop breaks.

# step 5: emit terminal summary.
# step 6: release the driver lock.
```

## Terminal summary (as rendered by `/spec:execute`)

```text
## /spec:execute — platform-v2 — terminated

### Final state
Progress: done 7, in-progress 0, pending 1, blocked 0, failed 1, skipped 0 (total 9)

Completion: stuck

Failed:
  - checkout-api (status-reason: "Type mismatch between cart line-item schema and payment gateway contract. Needs design revision after shopping-cart specs are updated.")

Pending (dependencies not satisfied):
  - checkout-ui (waits on: checkout-api)

Next action: Resolve blocked/failed entries (specify initiative amend + specify initiative transition <name> blocked → pending / failed → pending) or accept the partial initiative and run specify initiative archive --force.
```

Exit code: 0 (stuck is a partial-success terminal state; the driver
did nothing wrong).

## Tension with the RFC-2 §"The Plan" acceptance wording

The RFC-2 acceptance language reads "drives… to `all-done`", but
the seeded plan contains a pre-`failed` `checkout-api` whose
`status-reason` says "Needs design revision after shopping-cart
specs are updated" — human triage, not an automatic retry. The
loop cannot reach `Completion: all-done` without the operator
either (a) transitioning `checkout-api` back to `pending` and
retrying, or (b) flipping `checkout-ui` to `skipped` to break the
dangling dependency.

This fixture pins the faithful read: the loop drains every entry
it *can* (seven `done` plus the already-failed and its dependent),
then exits with `Completion: stuck`. That is the Layer 2 exit-gate
guarantee — the driver drives an initiative as far as the plan's
dependency graph and per-entry `status` allow, then surfaces the
remaining triage cleanly. The `all-done` path is exercised by the
existing `fixtures/loop/all-done/` fixture on a three-entry plan
with no pre-failed entries.

## Invariants pinned by this fixture

1. **Self-heal runs once, before the iteration loop.** The
   `email-verification` in-progress entry is reconciled to `done`
   inside the single pre-iteration self-heal pass.
2. **Every plan-entry shape is exercised.** Greenfield
   (`notification-preferences`), `affects`-only
   (`registration-duplicate-email-crash`), `affects` with multiple
   targets (`extract-shared-validation`), local-path `sources`
   (`product-catalog`), git-URL `sources` (`shopping-cart`). The
   `combined/` shape is pinned by the dedicated field-wiring
   fixture; no entry in RFC-2 §"The Plan" as written declares both
   `sources` and `affects` on the same entry.
3. **Lock held once, across the whole run.** Five successful
   iterations share the single `specify initiative lock acquire` from
   step 2 of the `--loop` algorithm; the release happens once at
   step 6, after the terminal summary.
4. **`checkout-api` stays `failed`** — the driver never retries a
   `failed` entry unconditionally. Retries go through a human-
   initiated `specify initiative transition failed → pending`.
5. **`checkout-ui` stays `pending`** — its only dep is `failed`.
   `specify initiative next` does not return a pending entry whose deps
   are unmet; the loop treats this as `stuck` rather than halting.
6. **Verbatim `status-reason` in the terminal summary.** The
   multi-line `status-reason` on `checkout-api` is rendered into
   the `Failed:` section byte-for-byte (subject to the one-line
   rendering convention the terminal-summary section describes —
   the newline-folded YAML form is flattened for display but the
   string content is unchanged).
