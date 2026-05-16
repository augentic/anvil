# Cross-project compatibility report fixture

Pins the RM-04 `specify compatibility check --change <name> --report-only` shape for a producer-side contract change that would break a downstream consumer.

## Scenario

A two-project registry on the `platform-v2` change:

- **`backend`** (producer) — owns `contracts/http/user-api.yaml`.
- **`mobile`** (consumer) — calls `GET /users/{id}` and `POST /users`.

The producer ships `update-user-api-v2`: the merge updates
`contracts/http/user-api.yaml` to (a) drop the `email` field from the
`GET /users/{id}` response and (b) add `phone-number` to the required
list of `POST /users`. Both changes are wire-level breaking from the
consumer's perspective, but the producer's specs / tests pass and the
merge itself succeeds.

The consumer's tier-2 workspace clone (under
`.specify/workspace/mobile/`) was last sync'd against the **previous**
contract — the v1 shape with `email` in the response and a smaller
required list on the request. That clone's
`contracts/http/user-api.yaml` is the consumer's "view" of
the contract, and is what the cross-project check compares against.

When the operator runs `specify compatibility check --change platform-v2 --report-only`, the CLI:

1. Reads `registry.yaml` and observes that `backend.contracts.produces` includes `http/user-api.yaml`.
2. Finds matching consumers — `mobile.contracts.consumes` contains the same path.
3. Compares root `contracts/after/http/user-api.yaml` with the consumer workspace's last-known view.
4. Emits two `breaking` findings using the shared `change-kind` vocabulary.
5. Leaves plan state, journals, and workspace clones untouched.

## Layout

```text
cross-project-contract-warning/
├── README.md                                   # this file
├── registry.yaml                               # backend (produces) + mobile (consumes)
├── plan.yaml.before                            # update-user-api-v2: in-progress
├── plan.yaml.after                             # update-user-api-v2: done
├── contracts/
│   ├── before/http/user-api.yaml               # baseline (v1) — same as consumer view
│   └── after/http/user-api.yaml                # producer current view (v2)
├── consumer-workspace/                         # snapshot of .specify/workspace/mobile/
│   └── contracts/http/user-api.yaml   # consumer's last-known view (v1)
└── compatibility-report.json                   # specify compatibility check --report-only output
```

## Invariants every reviewer checks

1. **Registry routing.** `registry.yaml` has exactly one producer and one matching consumer for `http/user-api.yaml`.
2. **Consumer view is stale.** `consumer-workspace/contracts/http/user-api.yaml` matches `contracts/before/http/user-api.yaml`.
3. **Producer update is breaking.** `contracts/after/http/user-api.yaml` removes the `email` field and adds required `phone-number`.
4. **Report shape is RM-04.** `compatibility-report.json` uses `classification`, optional `change-kind`, producer/consumer project names, contract paths, locators, details, and summary counts.
5. **Read-only reporting.** No journal, transcript, plan transition, or workspace mutation is part of this fixture.

## Using this fixture

- Before changing the compatibility classifier, re-read this fixture and confirm the report still maps the two breaking deltas to `removed-field` and `required-field-added`.
- The fixture is prose-only (no automated harness). A reviewer reads the files in order — `registry.yaml` → `contracts/before/...` and `consumer-workspace/...` → `contracts/after/...` → `compatibility-report.json` — and confirms the compatibility report's invariants hold at each step.
