# RFC-29b-signal: Strengthening Plan-Time Reconciliation Signal

> Status: Draft — Amends [RFC-29b](rfc-29b-reconciliation.md) (D2) — Depends: RFC-29b shipped — Touches: `lead.schema.json` + the discovery model/parser (retires `tentative`), the source `survey` briefs, and the `/spec:plan` propose brief. No change to the partition kernel, the `proposal.schema.json` shape, the response shape, or any `plan-reconcile-*` code.

[RFC-29b](rfc-29b-reconciliation.md) made plan-time lead reconciliation an agent-judgment-under-CLI-kernel step: the agent groups raw `(source-key, lead-id)` leads into `slices[]`, the kernel validates the partition and writes the plan, and the operator curates at Gate 1. The grouping is — correctly — a **reversible hypothesis**, not an authoritative merge.

This amendment does not reopen that design. It tightens the *inputs and guidance* the agent uses to form that hypothesis, because today the entire cross-source match rides on a single one-line `summary` and a slug, with no contract on summary quality and no stated bias for the asymmetric cost of a wrong merge. It also retires a dead, self-contradictory field — the survey-time `tentative` flag — and records where the *tentative* concept actually belongs (the reconciliation layer). Each item below is independently adoptable; none requires touching the projection kernel.

## Background

At propose time the agent sees only the `kind: request` envelope: per lead a `source-key`, `lead-id`, optional `aliases[]`, and a one-line `summary`, plus `projects[]` for binding. Deep `Evidence` does not exist yet — `extract` is slice-time — so reconciliation runs **on headlines alone** (the `/spec:plan` propose brief states this explicitly). The kernel never auto-merges; safety comes from the total-partition invariant, the at-most-one-lead-per-source rule, Gate 1 review, and cheap re-propose.

Three things motivate this amendment:

1. **Summary quality is an uncontracted single point of failure.** `lead.schema.json` enforces only `minLength: 1` on `summary`. Two genuinely different leads sharing a slug, or two same-thing leads with terse summaries under different slugs, are only as separable as the upstream source adapter's survey brief happened to make them. There is no kernel guardrail — only Gate 1.
2. **The error-cost asymmetry is unstated.** Over-merging two unrelated leads forces unrelated work into one slice bound to one project/target and pushes the reconciliation cost downstream into D3 synthesis (which *will* have full Evidence and is likely to emit `[conflict]`/divergence). Over-splitting is cheap — just an extra slice. The propose guidance gives the agent no "prefer split on doubt" bias.
3. **The survey-time `tentative` flag is a dead, self-contradictory field.** `lead.schema.json` and RFC-29b describe `tentative` as a survey-time flag a source sets on its own lead, but every source `survey` brief tells the adapter *not* to set it ([`intent`](../adapters/sources/intent/briefs/survey.md), [`screenshots`](../adapters/sources/screenshots/briefs/survey.md), [`code-typescript`](../adapters/sources/code-typescript/briefs/survey.md)), and one even says it is "set later by `/spec:plan`'s `propose` sub-step". No adapter emits it; the model, parser, and schema carry plumbing nothing fills. A lead is always a lead — a right-sized fragment of a decomposed surface — so there is no survey-time "tentative lead". The only legitimate *tentative* signal is the agent's confidence in a cross-source **merge**, which already lives as `## Tentative merges` prose in `change.md`.

## Decision

| ID | Decision |
| -- | -------- |
| **D2.1 Summary content floor** | The reconciliation `summary` carries a *contentfulness* expectation, not just `minLength: 1`. The lead `summary` SHOULD identify the lead's behaviour distinctly enough to be matched or distinguished from a same-slug lead in another source, and MAY span more than one line to do so (relax the "one-line" description; no second field). Encode this as survey-brief guidance plus a non-blocking advisory finding; do not hard-gate (a thin summary must never block planning). |
| **D2.2 Split-on-doubt heuristic** | The propose brief states the error-cost asymmetry and instructs the agent to **prefer keeping leads in separate scopes when a cross-source match is uncertain**, leaning on Gate 1 merge (`plan amend --sources`) rather than an unrecoverable propose-time over-merge. Guidance only; no schema or kernel change. |
| **D2.3 Retire survey-time `tentative`** | Remove the unused `tentative` field from `lead.schema.json`, the discovery model/parser, its tests, and the "do not set `tentative`" lines in the `survey` briefs and reference. `tentative` is a **reconciliation** concept, not a survey one: a source never marks its own lead tentative, and the agent's cross-source merge confidence already lives as `## Tentative merges` in `change.md`. A future RFC may add a *structured* per-scope confidence hint to the propose **response**; that is reserved, not specified here. |

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

## D2.2 — Split-on-doubt heuristic

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

## D2.3 — Retire survey-time `tentative`

`tentative` was specified as a survey-time, source-set scope-uncertainty flag, but the codebase never wired a producer and actively contradicts the schema. The honest reconciliation is to **delete the survey-side field** and keep *tentative* purely as a reconciliation-layer concept.

**The contradiction.** The schema and RFC-29b say the source sets it:

```23:26:schemas/discovery/lead.schema.json
    "tentative": {
      "type": "boolean",
      "description": "Optional survey-time uncertainty flag a source adapter may set on its own lead; the operator reconciles at Gate 1. `/spec:plan`'s `propose` sub-step reads these leads but never edits `discovery.md`."
    },
```

…while every `survey` brief says the opposite — e.g. `intent`:

```26:26:adapters/sources/intent/briefs/survey.md
- Do not set `tentative`. The intent source surfaces exactly one lead per slice; cross-source merge ambiguity is a `/spec:plan` propose-time concern, not a survey concern.
```

`screenshots` repeats "Do not set `tentative`", and `code-typescript` says "no `tentative:` field at survey time (set later by `/spec:plan`'s `propose` sub-step)" — a fourth, propose-side meaning. The result is dead plumbing in the model, parser, and schema that nothing fills.

**The fix (removal).** Delete the survey-side field end to end:

- `schemas/discovery/lead.schema.json` — drop the `tentative` property and its mention in the top-level `description` (the object is `additionalProperties: false`, so a `discovery.md` lead carrying `tentative:` becomes a validation error — intended, since no adapter emits one).
- `crates/model/src/discovery/lead.rs` — remove the `pub tentative: Option<bool>` field and its doc comment; update the round-trip tests.
- `crates/model/src/discovery/document.rs` — remove the `tentative` parse arm and the render branch.
- `crates/workflow/tests/schemas.rs` — drop `LEAD_VALID_TENTATIVE` / `LEAD_INVALID_TENTATIVE_WRONG_TYPE` and their assertions.
- `crates/workflow/src/change/plan/core/propose.rs` — drop the "`tentative` is deliberately not surfaced" comments (the field no longer exists to surface).
- Briefs + reference — remove the now-moot "do not set `tentative`" lines from `adapters/sources/{intent,screenshots,code-typescript}/briefs/survey.md` and the `Survey adapters MAY set tentative` sentence in `plugins/spec/references/discovery.md`.
- RFC-29b — delete the §"Cross-Source Matching" paragraph that explains why the survey `tentative` flag is kept off the wire (it no longer exists); leave `## Tentative merges` untouched.

Per the cross-repo discipline ([specify-cli `AGENTS.md` rule 5](https://github.com/augentic/specify-cli/blob/main/AGENTS.md)), `tentative` touches `crates/model/src/discovery/` and a schema, so the removal must be grepped and applied across **both** repos in one change.

**Where *tentative* lives instead.** The useful idea — "I think this cross-source merge is right, but I'm not sure" — is a **reconciliation** signal, not a survey one. Today it is the agent's `## Tentative merges` prose in `change.md`, reviewed at Gate 1. If a structured form is ever wanted, the natural home is an optional per-scope confidence/`tentative` hint on the propose **response** slice (the agent's own output), never an input flag a source stamps on a lead. That is explicitly **reserved for a future RFC**, not specified here.

## Wire-contract impact

Small and mostly subtractive:

- `proposal.schema.json` — **unchanged**. No new request or response property; the response shape, `scope`/`sources[]` shape, and partition invariants are untouched.
- `lead.schema.json` — D2.1 relaxes the `summary` "one-line" description (no new field); D2.3 **removes** the `tentative` property and its `description` mention.
- `crates/model/src/discovery/{lead.rs,document.rs}` + `crates/workflow/tests/schemas.rs` — D2.3 removes the `tentative` field, its parse/render arms, and its tests.
- New optional advisory finding `discovery-lead-summary-thin` (D2.1) — non-blocking `suggestion` severity; no new exit code, no lifecycle authority.
- No new `Error::Validation` code, no `plan-reconcile-*` change, no journal-event change, no kernel change.

D2.3 is a **breaking parse change** in the narrow sense that a `discovery.md` lead carrying `tentative:` would now fail validation — acceptable because no shipped adapter emits one and the briefs forbid it.

## Out of scope

- **Richer matching facets** (`blocking-keys[]`, facet edges, lexical clustering) remain deferred per [RFC-29b Appendix item 2](rfc-29b-reconciliation.md#appendix-deferred-work).
- **Any plan-time `Evidence`.** Deep extraction stays slice-time (`extract`/D3). A multi-line `summary` is still headline material only — not a back-door for claims.
- **A second summary field.** `synopsis` / `detail` is explicitly rejected as a near-synonym of `summary`; the one field carries the content.
- **Hard-gating on summary quality.** The advisory finding stays non-blocking; it never parks planning or transitions a plan.
- **A structured `tentative` reconciliation hint.** Reserved (D2.3): if the agent's merge-confidence is ever structured, it belongs on the propose *response*, not as a survey input — deferred to a future RFC.
- **An agent-side defer bucket.** Total partition at propose time is unchanged; deferral remains a Gate 1 `plan remove`.

## Acceptance

- A `survey` brief fixture whose summaries name behaviour distinctly; a same-slug cross-source pair the agent splits or merges on summary content (not slug alone).
- If the advisory finding ships: a `specrun slice validate` case emitting `discovery-lead-summary-thin` at `suggestion` without gating the transition.
- D2.3: `cargo make check` green after the `tentative` removal; a `discovery.md` lead carrying `tentative:` is rejected by `lead.schema.json`; `rg tentative` across both repos returns only `## Tentative merges` reconciliation prose (no survey-side field, no "do not set `tentative`" lines).
