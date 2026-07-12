# Authority hierarchy

Top-level `authority:` on every `Evidence` document is a closed enum. Highest wins:

1. **`intent`** — operator override at slice time. Emitted by the `intent` source adapter.
2. **`documentation`** — operator-provided written product / technical intent (internal docs, RFCs, product notes). Emitted by the `documentation` and `screenshots` source adapters. Distinct from the synthesised `design.md` artifact and from the refine substep named `design`.
3. **`behaviour`** — what legacy code actually does. Emitted by behaviour sources such as `typescript`, `captures`, and future code or observation adapters.

Authority is a property of the **Evidence document** by default. v1 sharpens that default with a single opt-in override surface (see [§Authority overrides](#authority-overrides) below): a per-slice override on `plan.yaml`. A slice without `authority-override` behaves exactly as the document-level rule above. (A per-Evidence per-kind `authority-overrides` surface is deferred to a future RFC.)

The **agent** never resolves authority or marks winners. It records the contributing `(source, id, kind)` claims and an `agreement` verdict (`agreed` / `disagreed`) per requirement. The **kernel** resolves authority from the on-disk Evidence and any per-slice override *after* the response returns, then derives `status`, winner markers, and the rendered `Sources:` list.

## `agreement` verdict → kernel `status` derivation

The agent supplies the `agreement` verdict; the kernel derives `status` from the claim count, that verdict, and the resolved authority:

| `claims` | `agreement`                       | Kernel `status` | Tag           | Winner markers                |
| -------- | --------------------------------- | --------------- | ------------- | ----------------------------- |
| 0        | *(omitted)*                       | `unknown`       | `[unknown]`   | none                          |
| 1        | *(omitted)*                       | `agreed`        | (none)        | none                          |
| ≥2       | `agreed`                          | `agreed`        | (none)        | none                          |
| ≥2       | `disagreed`, unique top authority | `divergence`    | `[divergence]`| winner `true`, losers `false` |
| ≥2       | `disagreed`, top authority ties   | `conflict`      | `[conflict]`  | none                          |

The kernel renders the headline tag to match `status` per the coherence rule in [`tags.md`](./tags.md); the provenance parser (consumed by `specify slice validate`) refuses any hand-edit where a `[…]` headline tag and `Status:` disagree.

## Worked applications

The rendered blocks below are the kernel's **output** — the agent authors only the heading and body prose plus the `(source, id, kind)` claims and `agreement` verdict; the kernel injects `ID:` / `Sources:` / `Status:` and the headline tag.

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

`documentation` and `typescript` Evidence both surface the same email-validation behaviour. Both keys appear, highest authority first (`documentation` before `behaviour`):

```markdown
### Requirement: User registration accepts valid email

ID: REQ-001
Sources: [identity-design-notes, legacy-monolith]
Status: agreed

The system accepts a registration request when the email field is RFC-5322 valid.
```

### Disagree, one wins authority (`[divergence]`)

`documentation` says expiry is 30 minutes; `typescript` observed 24 hours. `documentation > behaviour` resolves the contradiction:

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

A requirement the slice's proposal calls for (e.g. covered by the lead `synopsis`) that no source supplied a claim for. The agent still records the requirement (with an empty claims list) so the operator sees the gap; the kernel derives `Status: unknown` and the `[unknown]` tag:

```markdown
### Requirement: Reset link single-use [unknown]

ID: REQ-008
Sources: []
Status: unknown

No contributing source supplied a claim for this requirement. Operator review required.
```

## Authority overrides

The document-level `authority:` rule is the default. One opt-in override surface sharpens it for the cases the rule gets wrong — most often legacy migrations where production behaviour is the truth and the `documentation > behaviour` default would otherwise drop the operative value into a `Note:` line.

> **Deferred (future RFC).** A per-Evidence per-kind `authority-overrides: { <claim-kind>: <authority-class> }` map on each Evidence document — which would let one document lift a kind's authority class — is out of scope for v1. v1 resolves authority at document level and offers only the per-slice override below.

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
- Orphan source keys (a value that is not in the slice's own `sources[]`) are rejected by `specify slice validate` with the structured error `slice-authority-override-orphan-source` before `/spec:refine` runs.
- Operators author the map via the CLI; the synthesis playbook never asks an agent to hand-edit `plan.yaml`:

```bash
specify plan amend <entry> --authority-override <entry> <claim-kind>=<source>
specify plan amend <entry> --clear-authority-override <entry> <claim-kind>
specify plan amend <entry> --clear-authority-overrides
specify plan add   <entry> --authority-override <claim-kind>=<source>   # repeatable on create
```

### Resolution order

When a requirement's `agreement` verdict is `disagreed`, the kernel walks the following ordered steps over the contributing claims. The first step that yields a winner stops the walk; the chosen step name is recorded inline in `model.yaml` at `requirements[].resolution-trace.step` (and surfaced by `specify slice provenance`) so the operator can audit which surface broke the tie.

1. **`per-slice-authority-override`** — the slice's `authority-override.<kind>` names a source key that appears in the reconciled group's contributing sources. That source wins; the kernel derives `status: divergence` (or `agreed` when the override aligns with a shared value), and the runner-up survives with `winner: false`.
2. **`document-authority-ordering`** — fall back to the document-level `authority:` enum (`intent > documentation > behaviour`). Highest class wins; ties at the top class continue to step 3.
3. **`tied-conflict`** — still tied. The kernel derives `status: conflict` with the `[conflict]` tag; no winner markers. The operator reconciles by re-running `/spec:refine` (after amending the override or the source set) before `/spec:build`.

Steps 1–2 yield `status: divergence` when the chosen source disagrees with at least one other contributor and `status: agreed` when every contributor's value matches the winner's. Step 3 yields `status: conflict`. Step names are byte-stable across runs and match the projected `requirements[].resolution-trace.step` exactly — see [`provenance.md`](./provenance.md) for the audit shape and [`claim-reconciliation.md`](./claim-reconciliation.md) for the per-kind body landing rules. (The deferred per-Evidence surface would insert a `per-evidence-authority-override` step between 1 and 2.)

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

The agent records both contributing claims with `agreement: disagreed`; the kernel walks the resolution order. Step 1 (`per-slice-authority-override`) matches: `runtime` is in the reconciled group's contributing sources. The walk stops; `runtime` wins. The kernel renders:

```markdown
### Requirement: Reset link expiry [divergence]

ID: REQ-007
Sources: [runtime, identity-design-notes]
Status: divergence

The system expires password reset links after 24 hours. (from runtime; behaviour, per-slice authority-override)

Note: identity-design-notes (documentation) says reset links expire after 30 minutes; the per-slice authority-override pins behaviour-class as the winner. Operator review recommended.
```

The runner-up (`identity-design-notes`) is preserved verbatim as a `Note:` line. The `Sources:` list lists `runtime` first because the per-slice override promoted it to the operative source for this block — the audit trail (inline in `model.yaml`, surfaced by `specify slice provenance`) reads `resolution-trace.step: per-slice-authority-override` with `override: { criterion: runtime }` and `winner: runtime`.

## Notes

- Authority does **not** apply at plan-time `propose` (no `Evidence` yet); it activates here at slice-time synthesis.
- Per-kind and per-claim overrides remain out of scope for v1 (the per-Evidence `authority-overrides` surface is deferred to a future RFC). The override seam below per-slice granularity is re-running `/spec:refine` with a different `agreement` verdict or amended `plan.yaml.slices[].authority-override`, never a hand-edit of the kernel-rendered `spec.md` provenance lines.
- The kernel renders the `Sources:` list with every contributing source key, highest authority first **after override resolution** — a per-slice override that promotes a `behaviour`-class source to the operative winner promotes that key to the front of the list for the affected block.
- The provenance parser cross-resolves every `Sources:` key against the slice's `plan.yaml.slices[].sources[]` bindings; a stale or missing key fails validation. Per-slice `authority-override` source keys are checked by the same parser before `/spec:refine` runs.
- Every override resolution — including step 2 fallbacks where no override fired — lands inline in `model.yaml` at `requirements[].resolution-trace.step` and is surfaced by `specify slice provenance`. The projected provenance view is the audit surface; `spec.md` carries operator-facing prose only.
