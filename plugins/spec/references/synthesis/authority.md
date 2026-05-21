# Authority hierarchy

Top-level `authority:` on every `Evidence` document is a closed enum. Highest wins:

1. **`intent`** — operator override at slice time. Emitted by the `intent` source adapter.
2. **`documentation`** — operator-provided written product / technical intent (internal docs, RFCs, product notes). Emitted by the `documentation` and `screenshots` source adapters. Distinct from the synthesised `design.md` artifact and from the refine substep named `design`.
3. **`behaviour`** — what legacy code actually does. Emitted by code source adapters (`code-typescript`, future code adapters).

Authority is a property of the **Evidence document**, not of individual claims. Every claim inside an Evidence document inherits the document's authority.

## Agreement → `Status` decision table

Apply this table per requirement after [claim fusion](claim-fusion.md) has grouped contributing claims by `claim-id`:

| Agreement                                  | `Status:`     | Tag in headline | Body shape                                                                            |
| ------------------------------------------ | ------------- | --------------- | ------------------------------------------------------------------------------------- |
| Single contributing source                 | `agreed`      | (none)          | One paragraph stating the requirement.                                                |
| Multiple sources, all agree                | `agreed`      | (none)          | One paragraph; `Sources:` lists every contributing key, highest authority first.      |
| Multiple sources disagree, one wins authority | `divergence` | `[divergence]`  | Winning value as the requirement; loser preserved as `Note: <source-key> observed …`. |
| Multiple sources disagree, tied top authority | `conflict`   | `[conflict]`    | Both values preserved inline as `Note: <source-key> says …` lines; no winner.         |
| No contributing Evidence at all            | `unknown`     | `[unknown]`     | One-line placeholder noting that no source supplied a claim for this requirement.    |

The tag in the headline MUST match `Status:` per the coherence rule in [`tags.md`](tags.md). The provenance parser (consumed by `specify slice validate`) refuses output where a `[…]` headline tag and `Status:` disagree.

## Worked applications

### Single source

One `documentation` Evidence contributing one `requirement` claim with `claim-id: password-reset.request`:

```markdown
### Requirement: Password reset request

ID: REQ-001
Sources: [product-notes]
Status: agreed

The system lets a registered user request a password reset link by email.
```

### Multiple sources agree

`documentation` and `code-typescript` Evidence both surface the same email-validation behaviour. Both keys appear, highest authority first (`documentation` before `behaviour`):

```markdown
### Requirement: User registration accepts valid email

ID: REQ-001
Sources: [identity-design-notes, legacy-monolith]
Status: agreed

The system accepts a registration request when the email field is RFC-5322 valid.
```

### Disagree, one wins authority (`[divergence]`)

`documentation` says expiry is 30 minutes; `code-typescript` observed 24 hours. `documentation > behaviour` resolves the contradiction:

```markdown
### Requirement: Reset link expiry [divergence]

ID: REQ-007
Sources: [identity-design-notes, legacy-monolith]
Status: divergence

The system expires password reset links after 30 minutes. (from identity-design-notes; documentation)

Note: legacy-monolith observed 24-hour expiry; the documentation authority overrides. Operator review recommended.
```

### Disagree, tied top authority (`[conflict]`)

Two `documentation` Evidence contribute claims with the same `claim-id` but contradictory values. No winner exists at the authority level:

```markdown
### Requirement: Reset link expiry [conflict]

ID: REQ-007
Sources: [product-notes, identity-design-notes]
Status: conflict

Note: product-notes says reset links expire after 30 minutes.
Note: identity-design-notes says reset links expire after 60 minutes.

Operator reconciliation required before /spec:build.
```

### No contributing Evidence (`[unknown]`)

A requirement the slice's proposal calls for (e.g. covered by the candidate `summary`) that no source supplied a claim for. Synthesis still authors the block so the operator sees the gap:

```markdown
### Requirement: Reset link single-use [unknown]

ID: REQ-008
Sources: []
Status: unknown

No contributing source supplied a claim for this requirement. Operator review required.
```

## Notes

- Authority does **not** apply at plan-time `propose` (no `Evidence` yet); it activates here at slice-time synthesis.
- Per-claim or per-slice authority overrides are deferred — there is no `authority-override` field on slice entries or claims in v1. The override seam is hand-editing `spec.md` after `/spec:refine` transitions the slice to `refined`.
- The `Sources:` list MUST list every contributing source key, highest authority first. The provenance parser cross-resolves every key against the slice's `plan.yaml.slices[].sources[]` bindings; a stale or missing key fails validation.
