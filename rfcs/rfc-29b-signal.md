# RFC-29b-signal: Strengthening Plan-Time Reconciliation Signal

> Status: Draft — Amends [RFC-29b](rfc-29b-reconciliation.md) (D2) — Depends: RFC-29b shipped — Touches: `lead.schema.json` + the discovery model/parser (retires `tentative`), the source `survey` briefs, and the `/spec:plan` propose brief. No change to the partition kernel, the `proposal.schema.json` shape, the response shape, or any `plan-reconcile-*` code.

[RFC-29b](rfc-29b-reconciliation.md) made plan-time lead reconciliation an agent-judgment-under-CLI-kernel step: the agent groups raw `(source-key, lead-id)` leads into `slices[]`, the kernel validates the partition and writes the plan, and the operator curates at Gate 1. The grouping is — correctly — a **reversible hypothesis**, not an authoritative merge.

This amendment does not reopen that design. It tightens the *inputs and guidance* the agent uses to form that hypothesis, because today the entire cross-source match rides on a single one-line `summary` and a slug, with no contract on summary quality and no stated bias for the asymmetric cost of a wrong merge. It also retires a dead, self-contradictory field — the survey-time `tentative` flag — and records where the *tentative* concept actually belongs (the reconciliation layer). Each item below is independently adoptable; none requires touching the projection kernel.

## Background

At propose time the agent sees only the `kind: request` envelope: per lead a `source-key`, `lead-id`, optional `aliases[]`, and a one-line `summary`, plus `projects[]` for binding. Deep `Evidence` does not exist yet — `extract` is slice-time — so reconciliation runs **on headlines alone** (the `/spec:plan` propose brief states this explicitly). The kernel never auto-merges; safety comes from the total-partition invariant, the at-most-one-lead-per-source rule, Gate 1 review, and cheap re-propose.

Three weak spots in that input/guidance surface motivate this amendment:

1. **Summary quality is an uncontracted single point of failure.** `lead.schema.json` enforces only `minLength: 1` on `summary`. Two genuinely different leads sharing a slug, or two same-thing leads with terse summaries under different slugs, are only as separable as the upstream source adapter's survey brief happened to make them. There is no kernel guardrail — only Gate 1.
2. **The source's own `tentative` flag is withheld from the grouping agent.** A source flagging its own lead as shaky is real, available context for a match decision, but RFC-29b keeps it off the request wire (to avoid conflating source-side and grouping-side uncertainty) and routes it only through `change.md` prose, so the agent making the match never sees it.
3. **The error-cost asymmetry is unstated.** Over-merging two unrelated leads forces unrelated work into one slice bound to one project/target and pushes the reconciliation cost downstream into D3 synthesis (which *will* have full Evidence and is likely to emit `[conflict]`/divergence). Over-splitting is cheap — just an extra slice. The propose guidance gives the agent no "prefer split on doubt" bias.

## Decision

| ID | Decision |
| -- | -------- |
| **D2.1 Summary content floor** | The reconciliation `summary` carries a *contentfulness* expectation, not just `minLength: 1`. The lead `summary` SHOULD identify the lead's behaviour distinctly enough to be matched or distinguished from a same-slug lead in another source, and MAY span more than one line to do so (relax the "one-line" description; no second field). Encode this as survey-brief guidance plus a non-blocking advisory finding; do not hard-gate (a thin summary must never block planning). |
| **D2.2 Surface `tentative` to propose** | The request catalog row carries the source's own `tentative` flag (when set) as an advisory input signal, distinct from the agent's own grouping uncertainty. The agent MAY weight it when matching and SHOULD surface tentative-on-either-side merges into `## Tentative merges`. It remains advisory — the kernel never reads or locks on it. |
| **D2.3 Split-on-doubt heuristic** | The propose brief states the error-cost asymmetry and instructs the agent to **prefer keeping leads in separate scopes when a cross-source match is uncertain**, leaning on Gate 1 merge (`plan amend --sources`) rather than an unrecoverable propose-time over-merge. Guidance only; no schema or kernel change. |

## D2.1 — Summary content floor

The problem is garbage-in / garbage-out: the match is only as good as the discriminating power of the `summary`. No new field is needed — `summary` is already a free-form `string` (`minLength: 1`), and nothing in the schema forbids a multi-line value; only its *description* says "one-line." Two complementary, independently adoptable moves, both on the existing field:

- **Survey-brief guidance (cheapest).** Each source adapter's `survey` brief instructs that a `summary` distinguish the lead's behaviour — name the operation/surface and its salient constraint — so a same-slug lead from another source can be matched or split on its content, not just its slug. Relax the schema's "one-line" wording so a source MAY use a few lines when one is too thin. This is documentation, not a structural schema change, and it stays plan-time-only headline material — **not** a back-door for slice-time `Evidence`, which remains the job of `extract`/D3.
- **Advisory contentfulness finding (optional).** A non-blocking `Diagnostic` (e.g. `discovery-lead-summary-thin`) at `suggestion` severity from `specrun slice validate` (or surfaced at survey finalize) flags summaries below a contentfulness heuristic. **Non-blocking by design** — a thin summary must never park planning; it is a nudge to improve the source adapter.

A thin summary the agent cannot match on, contrasted with content-bearing ones (request `leadCatalogEntry` rows; the second content-bearing summary spans two lines):

```yaml
# Thin — same slug under two sources, nothing to match or split on but the slug:
leads:
  - source-key: docs
    lead-id: identity-api
    summary: "Identity API."          # too thin: which surface? what behaviour?
  - source-key: legacy
    lead-id: identity-api
    summary: "Identity stuff."         # advisory: discovery-lead-summary-thin

# Content-bearing — the agent can now match or split on behaviour, not just the slug:
leads:
  - source-key: docs
    lead-id: identity-api
    summary: "Authentication + account-access API: login, token refresh, profile read."
  - source-key: legacy
    lead-id: identity-api
    summary: |
      Admin user-management endpoints: create/suspend/delete accounts.
      Internal /admin/users CRUD behind the operator role — distinct from
      the public auth surface despite the shared slug.
```

The shared slug alone would tempt a merge; the content-bearing summaries let the agent correctly keep these as two scopes.

Recommendation: ship the survey-brief guidance (including the multi-line relaxation) first; treat the advisory finding as a follow-on gated on whether thin summaries actually hurt grouping in practice.

## D2.2 — Surface `tentative` to propose

`lead.schema.json` already carries an optional `tentative` boolean a source sets on its own lead. RFC-29b deliberately keeps it off the request wire. This amendment reverses that **for the request only**, as an explicitly *advisory* signal:

- Add `tentative` (optional boolean) to the request `leadCatalogEntry` in `proposal.schema.json`, populated from the lead's flag at request-build time.
- The agent MAY weight a source-tentative lead toward caution when matching, and SHOULD route a merge that involves a tentative lead on either side into `## Tentative merges` in `change.md`.
- The kernel still never reads, locks, or auto-merges on `tentative`; it stays out of every `plan-reconcile-*` invariant. The response shape is unchanged.

Request `leadCatalogEntry` carrying the source's own flag:

```yaml
leads:
  - source-key: docs
    lead-id: password-reset
    summary: "Registered users request a password-reset link by email; links expire after 30 minutes."
  - source-key: legacy
    lead-id: account-pwd-reset
    summary: "Account password-reset handler; emits a 24-hour reset token."
    tentative: true                    # legacy survey flagged its own lead as shaky
```

The agent sees the `tentative: true` and routes the candidate cross-source match into `change.md`:

```markdown
## Tentative merges

`password-reset` — docs `password-reset` and legacy `account-pwd-reset` look like the
same flow, but the legacy lead is source-flagged `tentative` and its token lifetime
(24h) disagrees with the docs lead (30m). Left as separate scopes; operator to confirm
the merge at Gate 1 via `specrun plan amend <keep> --sources ...`.
```

This keeps source-side uncertainty (the adapter's signal) and grouping-side uncertainty (the agent's prose) distinct — they now both reach Gate 1, but the agent forming the match can finally see the former. If the original conflation concern resurfaces, the field can carry a name that marks it source-authored (e.g. `source-tentative`).

## D2.3 — Split-on-doubt heuristic

Purely a propose-brief addition (`plugins/spec/skills/plan/SKILL.md`, propose sub-step). State plainly:

- An over-**merge** is expensive and downstream-poisoning: two unrelated bodies of work land in one slice, one project/target binding, and synthesis (D3) inherits the reconciliation as `[conflict]`/divergence.
- An over-**split** is cheap and locally reversible at Gate 1 via `plan amend --sources` (the operator owns the same-source/cross-source sizing risk there anyway).
- Therefore: **when a cross-source match is not well-supported by shared slug, alias, or summary, keep the leads in separate scopes and note the candidate pairing in `## Tentative merges`** for the operator to confirm-by-merge at Gate 1.

The heuristic shapes the *response*: an uncertain pair stays as two scopes rather than one merged scope.

```yaml
# Discouraged — over-merge on a weak match (unrecoverable except by re-propose):
slices:
  - scope: password-reset
    sources:
      - { source-key: docs, lead-id: password-reset }
      - { source-key: legacy, lead-id: account-pwd-reset }   # weak match → expensive if wrong

# Preferred — keep separate, surface the candidate in `## Tentative merges`:
slices:
  - scope: password-reset
    sources:
      - { source-key: docs, lead-id: password-reset }
  - scope: account-pwd-reset
    sources:
      - { source-key: legacy, lead-id: account-pwd-reset }
```

The operator merges the preferred form in one Gate 1 step if the pairing is real; reversing the discouraged form needs a whole re-propose.

This biases the recoverable failure mode and aligns the brief with the existing Gate 1 merge recipe.

## Wire-contract impact

Minimal and additive:

- `proposal.schema.json` (request `leadCatalogEntry`) — add optional `tentative` (D2.2); D2.1 is a `summary` *description* relaxation (allow multi-line), not a new property. Response schema, `scope`/`sources[]` shape, and partition invariants are **unchanged**.
- `lead.schema.json` — `tentative` already exists; D2.1 only relaxes the `summary` "one-line" description. No new field.
- New optional advisory finding `discovery-lead-summary-thin` (D2.1) — non-blocking `suggestion` severity; no new exit code, no lifecycle authority.
- No new `Error::Validation` code, no `plan-reconcile-*` change, no journal-event change, no kernel change.

## Out of scope

- **Richer matching facets** (`blocking-keys[]`, facet edges, lexical clustering) remain deferred per [RFC-29b Appendix item 2](rfc-29b-reconciliation.md#appendix-deferred-work).
- **Any plan-time `Evidence`.** Deep extraction stays slice-time (`extract`/D3). A multi-line `summary` is still headline material only — not a back-door for claims.
- **A second summary field.** `synopsis` / `detail` is explicitly rejected as a near-synonym of `summary`; the one field carries the content.
- **Hard-gating on summary quality or `tentative`.** Both stay advisory; neither parks planning or transitions a plan.
- **An agent-side defer bucket.** Total partition at propose time is unchanged; deferral remains a Gate 1 `plan remove`.

## Acceptance

- A `survey` brief fixture whose summaries name behaviour distinctly; a same-slug cross-source pair the agent splits or merges on summary content (not slug alone).
- If D2.2 ships: a request-envelope golden carrying `tentative: true` on a catalog row, and a `change.md` golden routing that lead's candidate merge into `## Tentative merges`.
- If the advisory finding ships: a `specrun slice validate` case emitting `discovery-lead-summary-thin` at `suggestion` without gating the transition.
