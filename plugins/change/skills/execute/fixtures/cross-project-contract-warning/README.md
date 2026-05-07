# `/change:execute` — cross-project contract warning fixture

Pins the post-merge cross-project contract compatibility check from
**RFC-9 §3B**. The fixture exercises the wire-level safety net that
catches a producer-side contract change which would silently break a
downstream consumer.

## Scenario

A two-project change on the `platform-v2` plan:

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

When `specify slice merge run` returns success, `/change:execute`:

1. Verifies `.specify/specs/` and `.specify/archive/` are clean after
   the merge-baseline commit.
2. Commits the produced contract as non-baseline residue with
   `specify: residue update-user-api-v2`.
3. Transitions the plan entry to `done` (step 10 in the per-slice
   algorithm).
4. Reads `registry.yaml` and observes that `backend.contracts.produces`
   includes `http/user-api.yaml` (so the post-merge check is
   in-scope).
5. Walks the registry to find consumers — `mobile.contracts.consumes`
   matches.
6. Invokes `/contract:openapi` (verifier intent, `--mode cross-project`)
   with the producer contract path in `.specify/workspace/backend/`
   and the consumer's workspace clone path. (HTTP / resource APIs route
   to OpenAPI; an AsyncAPI contract would route to `/contract:asyncapi`,
   and a shared payload schema to `/contract:json-schema`.)
7. Receives a YAML report with two findings.
8. Records each finding as a `cross-project-warning:` `failure`-kind
   entry on the merged slice's `journal.yaml` via
   `specify slice journal append`.
9. Renders the `⚠ Cross-project contract warnings` block in the merge
   transcript.
10. Continues the loop (the warnings are non-fatal).

## Layout

```text
cross-project-contract-warning/
├── README.md                                   # this file
├── registry.yaml                               # backend (produces) + mobile (consumes)
├── plan.yaml.before                            # update-user-api-v2: in-progress
├── plan.yaml.after                             # update-user-api-v2: done
├── contracts/
│   ├── before/http/user-api.yaml               # baseline (v1) — same as consumer view
│   └── after/http/user-api.yaml                # post-merge (v2) — passed to validator
├── consumer-workspace/                         # snapshot of .specify/workspace/mobile/
│   └── contracts/http/user-api.yaml   # consumer's last-known view (v1)
├── validator-output.yaml                       # /contract:openapi (verifier intent, --mode cross-project) output
├── transcript.md                               # /change:execute success transcript with warning block
└── expected-journal.yaml                       # update-user-api-v2/journal.yaml after recording
```

## Invariants every reviewer checks

1. **Validator output → journal payload is lossless.** Every
   `findings[i]` in `validator-output.yaml` is reflected as exactly one
   entry in `expected-journal.yaml`. The `change-kind`, `locator`, and
   `details` survive the round-trip into `summary` and `context`
   without paraphrase.
2. **`cross-project-warning:` summary prefix.** Each journal entry
   produced by the post-merge check carries this exact prefix in
   `summary`. The prefix is the canonical signal that the entry is a
   §3B finding rather than an in-loop phase failure.
3. **`failure` kind, not a new variant.** No new `EntryKind` variant
   is introduced. The journal entries reuse `type: failure` per the
   existing journal contract; the `cross-project-warning:` summary
   prefix disambiguates.
4. **Plan entry stays `done`.** `plan.yaml.after` shows the merged
   change as `done`. Cross-project warnings do not roll back the
   transition.
5. **Transcript block placement.** The `⚠ Cross-project contract
   warnings` block appears immediately after the `Status: done` line
   in the success transcript, with the per-finding indented list
   format the SKILL specifies.
6. **Validator runs read-only.** Neither `contracts/before/` nor
   `contracts/after/` nor `consumer-workspace/` changes during the
   check. The fixture's three trees are static authoring pins.

## Using this fixture

- Before changing the post-merge cross-project check in
  `../../SKILL.md → §Cross-project contract check`, re-read this
  fixture and confirm the new algorithm still maps each
  `validator-output.yaml` finding to the journal entry shape in
  `expected-journal.yaml`. If it does not, update the fixture in the
  same change as the SKILL.md change.
- Before changing any `/contract:*` skill's verifier intent
  cross-project output shape, re-run the
  [shared cross-project report shape](../../../../../contract/references/report-shape.md#cross-project-mode-output-structured-yaml)
  against `validator-output.yaml` and confirm shape parity.
- The fixture is prose-only (no automated harness). A reviewer reads
  the files in order — `plan.yaml.before` → `contracts/before/...` and
  `consumer-workspace/...` → `contracts/after/...` →
  `validator-output.yaml` → `expected-journal.yaml` →
  `transcript.md` — and confirms the cross-project check's invariants
  hold at each step.
