# Provenance projection (`specify slice provenance`)

The audit-only view per slice of every `REQ-*` id and the contributing `(source, id)` pairs synthesis consulted plus the authority outcome. Provenance is carried **inline** on each requirement in the single `model.yaml` artifact; this view is **projected on demand** by `specify slice provenance`, never persisted as a second file. The resolution rules — per-slice override, default ordering — live in [`authority.md`](authority.md); this page covers only the shape of the projected view that records which rule fired.

## How it's produced

During `specify slice synthesize` the kernel writes the load-bearing provenance inline on each `model.yaml` requirement: the contributing `claims[]` (`source` / `id` / `kind`), the per-claim `winner` markers, the rendered `sources` list, and `status`. The skill writes no `provenance.yaml` file. To inspect the audit view, run `specify slice provenance <slice> --format json|text`; the CLI reshapes the inline data into the per-requirement shape below, **recomputing** `resolution` (and the optional `resolution-trace`) from the claim count, `winner` markers, and resolved authority, and **reading** each claim's `value` / `path` from `evidence/<source>.yaml` keyed by `(source, id)` — neither is persisted in `model.yaml`. Because the view is a pure projection of `model.yaml` plus on-disk Evidence, it can never drift from the model and no journal event records a provenance write.

## Block grammar

Every requirements entry shares the same closed top-level shape (`id`, `status`, `sources`, `contributing-claims`, `resolution`, optional `resolution-trace`). `resolution` is the closed enum that names how synthesis arrived at the final value. One worked sub-example per enum value follows.

### `single-source`

One contributing claim only; `status: agreed`.

```yaml
- id: REQ-001
  status: agreed
  sources: [identity-design-notes]
  contributing-claims:
    - source: identity-design-notes
      id: password-reset.request
      kind: requirement
      value: "The system lets a registered user request a password reset link by email."
      path: docs/identity/reset.md#L4
  resolution: single-source
```

### `single-value-agreement`

Multiple contributors; bodies match after whitespace normalisation; `status: agreed`. `winner` is absent on every entry — there is no winner / loser distinction.

```yaml
- id: REQ-002
  status: agreed
  sources: [identity-design-notes, runtime]
  contributing-claims:
    - source: identity-design-notes
      id: users.register.email-validation
      kind: requirement
      value: "The system accepts a registration request when the email field is RFC-5322 valid."
      path: docs/identity/register.md#L12
    - source: runtime
      id: users.register.email-validation
      kind: example
      value: "Registering with a fresh email returns 201 and publishes user.created."
      path: tests/data/replays/users-register/happy.json
  resolution: single-value-agreement
```

### `authority-resolved`

Multiple contributors disagree; the document-level authority ordering broke the tie. `status: divergence`. `resolution-trace.step` is `document-authority-ordering` (the document-level `authority:` ordering won).

```yaml
- id: REQ-007
  status: divergence
  sources: [identity-design-notes, legacy-monolith]
  contributing-claims:
    - source: identity-design-notes
      id: password-reset.expiry
      kind: criterion
      value: "Reset links expire after 30 minutes."
      path: docs/identity/reset.md#L7
      winner: true
    - source: legacy-monolith
      id: password-reset.expiry
      kind: criterion
      value: "expiresAt = createdAt + 24h"
      path: src/users/reset.ts#L42
      winner: false
  resolution: authority-resolved
  resolution-trace:
    step: document-authority-ordering
    winner: identity-design-notes
```

### `per-slice-override`

A per-slice `authority-override.<kind>` on `plan.yaml.slices[]` picked the winner directly. `status: divergence`. `resolution-trace.step` is `per-slice-authority-override` and `override` echoes the slice's map.

```yaml
- id: REQ-007
  status: divergence
  sources: [runtime, identity-design-notes]
  contributing-claims:
    - source: runtime
      id: password-reset.expiry
      kind: example
      value: "Captured handler issues links that expire after 24 hours."
      path: tests/data/replays/password-reset/expiry.json
      winner: true
    - source: identity-design-notes
      id: password-reset.expiry
      kind: criterion
      value: "Reset links expire after 30 minutes."
      path: docs/identity/reset.md#L7
      winner: false
  resolution: per-slice-override
  resolution-trace:
    step: per-slice-authority-override
    override: { criterion: runtime }
    winner: runtime
```

### `unknown-no-evidence`

The proposal called for the requirement; no source supplied a claim for it. `status: unknown`; `sources` is `[]`; `contributing-claims` is `[]`; `resolution-trace` is absent.

```yaml
- id: REQ-008
  status: unknown
  sources: []
  contributing-claims: []
  resolution: unknown-no-evidence
```

### `tied-conflict`

Multiple contributors disagree at the same authority class after every override surface has been walked; no winner exists. `status: conflict`; `winner` is absent on every entry (no winner / loser distinction); `resolution-trace` is absent.

```yaml
- id: REQ-009
  status: conflict
  sources: [product-notes, identity-design-notes]
  contributing-claims:
    - source: product-notes
      id: password-reset.expiry
      kind: criterion
      value: "Reset links expire after 30 minutes."
      path: docs/product/reset.md#L12
    - source: identity-design-notes
      id: password-reset.expiry
      kind: criterion
      value: "Reset links expire after 60 minutes."
      path: docs/identity/reset.md#L4
  resolution: tied-conflict
```

## Inline `value` truncation

`value` is a single-line string. The full per-kind body (an `example` claim's `input` / `output` blocks, a `decision` claim's free-form rationale) stays in the source `evidence/<source>.yaml`, linked by `path`.

- Multi-line claim bodies collapse to the **first non-empty line** with a trailing `…` indicator.
- Over-cap bodies truncate at a **whitespace boundary** and append `…`. The cap is **16 KiB** per `value`, enforced by the writer.
- The trailing `…` is the single-character Unicode horizontal ellipsis (`U+2026`), not three ASCII dots. The on-disk value keeps the full single-line / 16 KiB-capped form.

## `winner`

Boolean, optional:

- **Absent** on every entry of an `agreed` block (`single-source` and `single-value-agreement`) — there is no winner / loser distinction.
- **Absent** on every entry of a `tied-conflict` block — no winner exists.
- **`true`** on the synthesis-selected entry of an `authority-resolved` or `per-slice-override` block.
- **`false`** on every other contributing claim in an `authority-resolved` or `per-slice-override` block — every entry the kernel dropped survives inline in `model.yaml` (and in the projected view) so the operator can audit what was discarded.

## Resolution-trace step names

`resolution-trace` is present **only** when `resolution` is `authority-resolved` or `per-slice-override`. The closed set of `step` names is:

| `step` | When |
| --- | --- |
| `per-slice-authority-override` | The slice's `authority-override.<kind>` named a source key in the reconciled group; that source won. Paired with `resolution: per-slice-override`. |
| `document-authority-ordering` | Fallback to the document-level `authority:` enum (`intent > documentation > behaviour`); highest class won. Paired with `resolution: authority-resolved`. |

> The deferred per-Evidence `authority-overrides` surface (a future RFC — see [`authority.md`](authority.md)) would add a `per-evidence-authority-override` step here. It is out of scope for v1.

The closed set matches the resolution-order taxonomy in [`authority.md` §Resolution order](authority.md#resolution-order) byte-for-byte. The `provenance.schema.json` definition for `resolution-trace.step` accepts any non-empty string today (the taxonomy is enforced by skill discipline, not by the schema, until the step set is judged stable enough to close); writing a value outside the closed set is a skill-body error even though `specify slice validate` will not refuse it.

## Audit posture

The projected provenance view is generated on demand when an operator needs to audit source reconciliation (`specify slice provenance <slice>`). It is **not** an authoritative input to any downstream verb — `/spec:build` reads `spec.md` and `design.md`; `/spec:merge` reads `metadata.yaml` and the baseline. The view is audit-only, the same audit-only posture used by plan summary metadata.

The provenance data lives inline in `model.yaml`, which `/spec:refine` regenerates whole from the current `spec.md` + `evidence/*.yaml`. Operators who want to change a synthesis decision long-term amend `plan.yaml.slices[].authority-override` via `specify plan amend` (or adjust the source set) and re-run `/spec:refine`; the next refine reads back any prose edits outside the kernel-rendered provenance lines, but those lines themselves are never hand-edited.

## No drift surface

Because provenance is carried **inline** in the single `model.yaml` artifact and the audit view is a pure on-demand projection of it, the two can never disagree — there is no separate file to drift. `specify slice validate` checks spec-vs-model staleness and rejects orphan contributing claims (`slice-model-source-orphan`), both cleared by re-running `/spec:refine`.

## Worked example

A slice `identity-password-reset` binds three sources (`identity-design-notes` → `documentation`, `legacy-monolith` → `behaviour`, `runtime` → `behaviour`). The operator pins `runtime` as the `criterion`-class authority for the slice via `specify plan amend identity-password-reset --authority-override identity-password-reset criterion=runtime`. Three requirements illustrate the common shapes:

```yaml
version: 1
slice: identity-password-reset
generated-at: 2026-05-22T13:15:00Z
generator: specify@2.1.0
requirements:
  - id: REQ-001
    status: agreed
    sources: [identity-design-notes, runtime]
    contributing-claims:
      - source: identity-design-notes
        id: password-reset.request
        kind: requirement
        value: "Registered user requests a password reset link by email."
        path: docs/identity/reset.md#L4
      - source: runtime
        id: users.password-reset.request
        kind: example
        value: "POST /password-reset returns 202 and queues an email."
        path: tests/data/replays/password-reset/happy.json
    resolution: single-value-agreement
  - id: REQ-007
    status: divergence
    sources: [runtime, identity-design-notes]
    contributing-claims:
      - source: runtime
        id: password-reset.expiry
        kind: example
        value: "Captured handler issues links that expire after 24 hours."
        path: tests/data/replays/password-reset/expiry.json
        winner: true
      - source: identity-design-notes
        id: password-reset.expiry
        kind: criterion
        value: "Reset links expire after 30 minutes."
        path: docs/identity/reset.md#L7
        winner: false
    resolution: per-slice-override
    resolution-trace:
      step: per-slice-authority-override
      override: { criterion: runtime }
      winner: runtime
  - id: REQ-009
    status: conflict
    sources: [product-notes, identity-design-notes]
    contributing-claims:
      - source: product-notes
        id: password-reset.single-use
        kind: criterion
        value: "Each reset link is consumed on first use."
        path: docs/product/reset.md#L19
      - source: identity-design-notes
        id: password-reset.single-use
        kind: criterion
        value: "Reset links remain valid until expiry, even after a successful reset."
        path: docs/identity/reset.md#L22
    resolution: tied-conflict
```

REQ-001 is the agreed cross-source case (one shared statement; no winner / loser). REQ-007 is the per-slice override case — the operator's `criterion: runtime` line promoted the behaviour-class source to the winner, the documentation-class loser survives as the `winner: false` entry, and the trace records exactly which surface broke the tie. REQ-009 is the `tied-conflict` case the operator must reconcile by recording a per-slice authority override (or amending the source set) and re-running `/spec:refine` before `/spec:build`.

## References

- [`authority.md`](authority.md) — authority hierarchy, override surfaces, and the resolution-order taxonomy the `resolution-trace.step` names mirror.
- [`claim-reconciliation.md`](claim-reconciliation.md) — per-kind landing rules; the `kind` field on each contributing claim copies from the source Evidence claim.
- [`tags.md`](tags.md) — tag / `Status:` coherence on the matching `spec.md` requirement block.
- [`provenance.md`](provenance.md) — normative provenance-index shape and rationale.
