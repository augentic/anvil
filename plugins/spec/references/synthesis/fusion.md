# Reconciliation index (`fusion.yaml`)

The audit-only index per slice of every `REQ-*` id and the contributing `(source, claim-id)` pairs synthesis consulted plus the authority outcome. The resolution rules — per-kind precedence, per-slice override, default ordering — live in [`authority.md`](authority.md); this page covers only how to author the index that records which rule fired.

## When the skill writes it

`/spec:refine` step 5 (between the `tasks.md` write and `specify slice validate`). Atomic: write to a sibling temp file, then rename. The file is regenerated whole on each re-refine — operator hand-edits to `fusion.yaml` do not survive, the same posture `spec.md` has against re-refine. The skill body is the writer; there is no `specify slice fusion write` verb. After the atomic rename succeeds, emit the `slice.fusion.written` journal event.

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
      claim-id: password-reset.request
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
      claim-id: users.register.email-validation
      kind: requirement
      value: "The system accepts a registration request when the email field is RFC-5322 valid."
      path: docs/identity/register.md#L12
    - source: runtime
      claim-id: users.register.email-validation
      kind: example
      value: "Registering with a fresh email returns 201 and publishes user.created."
      path: tests/data/replay/users-register/happy.json
  resolution: single-value-agreement
```

### `authority-resolved`

Multiple contributors disagree; the default authority ordering (or a per-Evidence per-kind override) broke the tie. `status: divergence`. `resolution-trace.step` is `document-authority-ordering` (default ordering won) or `per-evidence-authority-override` (a per-Evidence `authority-overrides.<kind>` resolved a stronger class for the loser).

```yaml
- id: REQ-007
  status: divergence
  sources: [identity-design-notes, legacy-monolith]
  contributing-claims:
    - source: identity-design-notes
      claim-id: password-reset.expiry
      kind: criterion
      value: "Reset links expire after 30 minutes."
      path: docs/identity/reset.md#L7
      winner: true
    - source: legacy-monolith
      claim-id: password-reset.expiry
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
      claim-id: password-reset.expiry
      kind: example
      value: "Captured handler issues links that expire after 24 hours."
      path: tests/data/replay/password-reset/expiry.json
      winner: true
    - source: identity-design-notes
      claim-id: password-reset.expiry
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
      claim-id: password-reset.expiry
      kind: criterion
      value: "Reset links expire after 30 minutes."
      path: docs/product/reset.md#L12
    - source: identity-design-notes
      claim-id: password-reset.expiry
      kind: criterion
      value: "Reset links expire after 60 minutes."
      path: docs/identity/reset.md#L4
  resolution: tied-conflict
```

## Inline `value` truncation

`value` is a single-line string. The full per-kind body (an `example` claim's `input` / `output` blocks, a `decision` claim's free-form rationale) stays in the source `evidence/<source-key>.yaml`, linked by `path`.

- Multi-line claim bodies collapse to the **first non-empty line** with a trailing `…` indicator.
- Over-cap bodies truncate at a **whitespace boundary** and append `…`. The cap is **16 KiB** per `value`, enforced by the writer.
- The trailing `…` is the single-character Unicode horizontal ellipsis (`U+2026`), not three ASCII dots. The CLI text renderer (`specify slice fusion show <slice>`) re-truncates to a terminal-friendly column count for display only; the on-disk value keeps the full single-line / 16 KiB-capped form.

## `winner`

Boolean, optional:

- **Absent** on every entry of an `agreed` block (`single-source` and `single-value-agreement`) — there is no winner / loser distinction.
- **Absent** on every entry of a `tied-conflict` block — no winner exists.
- **`true`** on the synthesis-selected entry of an `authority-resolved` or `per-slice-override` block.
- **`false`** on every other contributing claim in an `authority-resolved` or `per-slice-override` block — every entry the index dropped survives in `fusion.yaml` so the operator can audit what was discarded.

## Resolution-trace step names

`resolution-trace` is present **only** when `resolution` is `authority-resolved` or `per-slice-override`. The closed set of `step` names is:

| `step` | When |
| --- | --- |
| `per-slice-authority-override` | The slice's `authority-override.<kind>` named a source key in the fused group; that source won. Paired with `resolution: per-slice-override`. |
| `per-evidence-authority-override` | A contributing Evidence document's `authority-overrides.<kind>` resolved a strictly-greater authority class than the other contributors' effective class for this kind. Paired with `resolution: authority-resolved`. |
| `document-authority-ordering` | Fallback to the document-level `authority:` enum (`intent > documentation > behaviour`); highest class won. Paired with `resolution: authority-resolved`. |

The closed set matches the resolution-order taxonomy in [`authority.md` §Resolution order](authority.md#resolution-order) byte-for-byte. The `fusion.schema.json` definition for `resolution-trace.step` accepts any non-empty string today (the taxonomy is enforced by skill discipline, not by the schema, until the step set is judged stable enough to close); writing a value outside the closed set is a skill-body error even though `specify slice validate` will not refuse it.

## Audit posture

`fusion.yaml` is consumed by `specify slice fusion show <slice>` (text rendering with truncated `value` columns) and `specify slice fusion show <slice> --format json` (verbatim re-serialisation). It is **not** an authoritative input to any downstream verb — `/spec:build` reads `spec.md` and `design.md`; `/spec:merge` reads `.metadata.yaml` and the baseline. The index is audit-only, the same posture RFC-22's `mapping` field and RFC-24's `surfaces[]` follow.

Operator hand-edits to `fusion.yaml` do not survive re-refine: `/spec:refine` re-runs regenerate the file whole from the current `spec.md` + `evidence/*.yaml`. Operators who want to record a synthesis decision long-term hand-edit `spec.md` (which the next refine reads back through provenance) or amend `plan.yaml.slices[].authority-override` via `specify plan amend`.

## Drift detection

`specify slice validate` refuses with structured error `slice-fusion-drift` (exit 2) on either of two drift conditions:

1. **REQ-id parity drift.** The set of `REQ-*` ids in `spec.md` MUST equal the set of `requirements[].id` in `fusion.yaml`, with order preserved.
2. **Contributing-claim → evidence drift.** Every `requirements[].contributing-claims[]` entry's `(source, claim-id)` MUST resolve to a real claim in the per-source `evidence/<source-key>.yaml`. A stale `claim-id` (the source's Evidence rewrote the id) or a stale `source` (the slice's `sources[]` binding was removed) both surface as drift.

Both drift conditions are cleared by re-running `/spec:refine` — synthesis writes a fresh `fusion.yaml` from the current `spec.md` + `evidence/*.yaml`. Operators who hand-edit `spec.md` between refine runs (the common case for reconciling a `[conflict]` tag) MUST re-run `/spec:refine` afterwards so `fusion.yaml` re-aligns; running `specify slice validate` alone will not regenerate the index.

## Worked example

A slice `identity-password-reset` binds three sources (`identity-design-notes` → `documentation`, `legacy-monolith` → `behaviour`, `runtime` → `behaviour`). The operator pins `runtime` as the `criterion`-class authority for the slice via `specify plan amend identity-revamp identity-password-reset --authority-override criterion=runtime`. Three requirements illustrate the common shapes:

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
        claim-id: password-reset.request
        kind: requirement
        value: "Registered user requests a password reset link by email."
        path: docs/identity/reset.md#L4
      - source: runtime
        claim-id: users.password-reset.request
        kind: example
        value: "POST /password-reset returns 202 and queues an email."
        path: tests/data/replay/password-reset/happy.json
    resolution: single-value-agreement
  - id: REQ-007
    status: divergence
    sources: [runtime, identity-design-notes]
    contributing-claims:
      - source: runtime
        claim-id: password-reset.expiry
        kind: example
        value: "Captured handler issues links that expire after 24 hours."
        path: tests/data/replay/password-reset/expiry.json
        winner: true
      - source: identity-design-notes
        claim-id: password-reset.expiry
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
        claim-id: password-reset.single-use
        kind: criterion
        value: "Each reset link is consumed on first use."
        path: docs/product/reset.md#L19
      - source: identity-design-notes
        claim-id: password-reset.single-use
        kind: criterion
        value: "Reset links remain valid until expiry, even after a successful reset."
        path: docs/identity/reset.md#L22
    resolution: tied-conflict
```

REQ-001 is the agreed cross-source case (one shared statement; no winner / loser). REQ-007 is the per-slice override case — the operator's `criterion: runtime` line promoted the behaviour-class source to the winner, the documentation-class loser survives as the `winner: false` entry, and the trace records exactly which surface broke the tie. REQ-009 is the `tied-conflict` case the operator must reconcile by hand-editing `spec.md` (and re-running `/spec:refine`) before `/spec:build`.

## References

- [`authority.md`](authority.md) — authority hierarchy, override surfaces, and the resolution-order taxonomy the `resolution-trace.step` names mirror.
- [`claim-fusion.md`](claim-fusion.md) — per-kind landing rules; the `kind` field on each contributing claim copies from the source Evidence claim.
- [`tags.md`](tags.md) — tag / `Status:` coherence on the matching `spec.md` requirement block.
- [RFC-27](../../../../rfcs/archive/rfc-27-synthesis.md) §Reconciliation index — normative shape and rationale.
