# Authority hierarchy

Top-level `authority:` on every `Evidence` document is a closed enum. Highest wins:

1. **`intent`** — operator override at slice time. Emitted by the `intent` source adapter.
2. **`documentation`** — operator-provided written product / technical intent (internal docs, RFCs, product notes). Emitted by the `documentation` and `screenshots` source adapters. Distinct from the synthesised `design.md` artifact and from the refine substep named `design`.
3. **`behaviour`** — what legacy code actually does. Emitted by behaviour sources such as `code-typescript`, `captures`, and future code or observation adapters.

Authority is a property of the **Evidence document** by default. Two narrow override surfaces sharpen that default without widening the closed enum (see [§Authority overrides](#authority-overrides) below): a per-kind override on each Evidence document, and a per-slice override on `plan.yaml`. Both are opt-in; an Evidence file without `authority-overrides` and a slice without `authority-override` behave exactly as the document-level rule above.

## Agreement → `Status` decision table

Apply this table per requirement after [claim reconciliation](claim-reconciliation.md) has grouped contributing claims by `id`:

| Agreement                                  | `Status:`     | Tag in headline | Body shape                                                                            |
| ------------------------------------------ | ------------- | --------------- | ------------------------------------------------------------------------------------- |
| Single contributing source                 | `agreed`      | (none)          | One paragraph stating the requirement.                                                |
| Multiple sources, all agree                | `agreed`      | (none)          | One paragraph; `Sources:` lists every contributing key, highest authority first.      |
| Multiple sources disagree, one wins authority | `divergence` | `[divergence]`  | Winning value as the requirement; loser preserved as `Note: <source> observed …`. |
| Multiple sources disagree, tied top authority | `conflict`   | `[conflict]`    | Both values preserved inline as `Note: <source> says …` lines; no winner.         |
| No contributing Evidence at all            | `unknown`     | `[unknown]`     | One-line placeholder noting that no source supplied a claim for this requirement.    |

The tag in the headline MUST match `Status:` per the coherence rule in [`tags.md`](tags.md). The provenance parser (consumed by `specrun slice validate`) refuses output where a `[…]` headline tag and `Status:` disagree.

## Worked applications

### Single source

One `documentation` Evidence contributing one `requirement` claim with `id: password-reset.request`:

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

Two `documentation` Evidence contribute claims with the same `id` but contradictory values. No winner exists at the authority level:

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

A requirement the slice's proposal calls for (e.g. covered by the lead `synopsis`) that no source supplied a claim for. Synthesis still authors the block so the operator sees the gap:

```markdown
### Requirement: Reset link single-use [unknown]

ID: REQ-008
Sources: []
Status: unknown

No contributing source supplied a claim for this requirement. Operator review required.
```

## Authority overrides

The document-level `authority:` rule is the default. Two override surfaces sharpen it for the cases the rule gets wrong — most often legacy migrations where production behaviour is the truth and the `documentation > behaviour` default would otherwise drop the operative value into a `Note:` line. Both surfaces are additive and opt-in.

### Per-Evidence per-kind overrides

Each Evidence document MAY carry an optional `authority-overrides: { <claim-kind>: <authority-class> }` map. Keys are the closed claim-kind enum from `schemas/evidence.schema.json` (`requirement`, `criterion`, `decision`, `section`, `excerpt`, `type`, `call`, `region`, `container`, `leaf`, `intent`, `diagram`, `contract`, `example`); values are the closed `authority:` enum (`intent`, `documentation`, `behaviour`).

```yaml
source: legacy-monolith
adapter: code-typescript
authority: behaviour            # document-level default (applied when no per-kind override matches)
authority-overrides:
  decision: documentation       # decisions extracted from comments outrank decision claims from elsewhere
  criterion: behaviour          # explicit: pin behaviour-class precedence for criterion claims
lead: user-registration
```

Rules:

- `authority` (document-level) stays required. `authority-overrides` is purely additive — an Evidence file without the field behaves exactly as today.
- The override applies to **every** claim of the named kind in that Evidence document; per-claim overrides remain out of scope (see the non-goals in this synthesis reference set).
- Both keys and values are closed enums. New kinds or new classes still require an RFC update.
- The map is informational on the Evidence document alone; it gains force only when synthesis runs the [§Resolution order](#resolution-order) below.

### Per-slice overrides on `plan.yaml`

Each `plan.yaml.slices[]` entry MAY carry an optional `authority-override: { <claim-kind>: <source> }` map. Keys are the closed claim-kind enum; values are source keys that MUST already appear in the slice's own `sources[]` list.

```yaml
slices:
  - name: identity-user-registration
    project: identity-svc
    sources:
      - key: identity-design-notes
        lead: user-registration
      - key: legacy-monolith
        lead: user-registration
      - key: runtime
        lead: user-registration
    authority-override:
      requirement: runtime         # runtime captures dictate requirement-class disagreements on this slice
      criterion: legacy-monolith   # legacy code dictates criterion-class disagreements on this slice
    status: pending
```

Rules:

- Plan-wide and project-wide overrides are out of scope; the map is scoped to a single slice.
- Orphan source keys (a value that is not in the slice's own `sources[]`) are rejected by `specrun slice validate` with the structured error `slice-authority-override-orphan-source` before `/spec:refine` runs.
- Operators author the map via the CLI; the synthesis playbook never asks an agent to hand-edit `plan.yaml`:

```bash
specrun plan amend <entry> --authority-override <entry> <claim-kind>=<source>
specrun plan amend <entry> --clear-authority-override <entry> <claim-kind>
specrun plan amend <entry> --clear-authority-overrides
specrun plan add   <entry> --authority-override <claim-kind>=<source>   # repeatable on create
```

### Resolution order

When synthesis reconciles claims for a single `id` group and the contributing claims disagree, it walks the following ordered steps. The first step that yields a winner stops the walk; the chosen step name is recorded in `provenance.yaml` at `requirements[].resolution-trace.step` so the operator can audit which surface broke the tie.

1. **`per-slice-authority-override`** — the slice's `authority-override.<kind>` names a source key that appears in the reconciled group's contributing sources. That source wins; the requirement block carries `Status: divergence` (or `agreed` when the override happens to align with a shared value), and the runner-up survives as a `Note:` line.
2. **`per-evidence-authority-override`** — at least one contributing Evidence carries `authority-overrides.<kind>` that resolves to a strictly-greater authority class than the other contributors' effective class for this kind. That class wins.
3. **`document-authority-ordering`** — fall back to the document-level `authority:` enum (`intent > documentation > behaviour`). Highest class wins; ties at the top class continue to step 4.
4. **`tied-conflict`** — still tied. Emit `Status: conflict` with `[conflict]` tag; preserve every contributing value as `Note:` lines. The operator reconciles by hand-editing `spec.md` before `/spec:build`.

Steps 1–3 produce `Status: divergence` when the chosen source disagrees with at least one other contributor and `Status: agreed` when every contributor's value matches the winner's. Step 4 produces `Status: conflict`. Step names are byte-stable across runs and match `provenance.yaml.requirements[].resolution-trace.step` exactly — see [`provenance.md`](provenance.md) for the audit shape and [`claim-reconciliation.md`](claim-reconciliation.md) for the per-kind body landing rules.

### Worked example — both overrides at play

Slice `identity-password-reset` binds three sources. `identity-design-notes` (authority `documentation`) and `runtime` (authority `behaviour`) both contribute a `criterion` claim with `id: password-reset.expiry`. The documentation says expiry is 30 minutes; the runtime captures show the production handler issuing links that expire after 24 hours. The operator wants the production observation to win on this slice and pins `runtime` via per-slice override:

```yaml
# plan.yaml fragment
slices:
  - name: identity-password-reset
    project: identity-svc
    sources:
      - key: identity-design-notes
        lead: password-reset
      - key: runtime
        lead: password-reset
    authority-override:
      criterion: runtime
    status: pending
```

Synthesis walks the resolution order. Step 1 (`per-slice-authority-override`) matches: `runtime` is in the reconciled group's contributing sources. The walk stops; `runtime` wins.

```markdown
### Requirement: Reset link expiry [divergence]

ID: REQ-007
Sources: [runtime, identity-design-notes]
Status: divergence

The system expires password reset links after 24 hours. (from runtime; behaviour, per-slice authority-override)

Note: identity-design-notes (documentation) says reset links expire after 30 minutes; the per-slice authority-override pins behaviour-class as the winner. Operator review recommended.
```

The runner-up (`identity-design-notes`) is preserved verbatim as a `Note:` line. The `Sources:` list lists `runtime` first because the per-slice override promoted it to the operative source for this block — the audit trail in `provenance.yaml.requirements[].resolution-trace.step` reads `per-slice-authority-override` with `override: { criterion: runtime }` and `winner: runtime`.

Had the operator instead omitted the per-slice map and added `authority-overrides: { criterion: behaviour }` to the `runtime` Evidence document, the walk would skip step 1, match step 2 (`per-evidence-authority-override`), and reach the same winner with a different audit trace.

## Notes

- Authority does **not** apply at plan-time `propose` (no `Evidence` yet); it activates here at slice-time synthesis.
- Per-claim overrides remain out of scope (see the non-goals in this synthesis reference set). The override seam below per-kind granularity stays as today: hand-edit `spec.md` after `/spec:refine` transitions the slice to `refined`.
- The `Sources:` list MUST list every contributing source key, highest authority first **after override resolution** — a per-slice override that promotes a `behaviour`-class source to the operative winner promotes that key to the front of the list for the affected block.
- The provenance parser cross-resolves every `Sources:` key against the slice's `plan.yaml.slices[].sources[]` bindings; a stale or missing key fails validation. Per-slice `authority-override` source keys are checked by the same parser before `/spec:refine` runs.
- Every override resolution — including step 3 fallbacks where neither override map fired — lands in `provenance.yaml.requirements[].resolution-trace.step`. The provenance index is the audit surface; `spec.md` carries operator-facing prose only.
