# RFC-104 → RFC-88 patches

> Status: Must-apply items are folded into [RFC-88](rfc-88-detached-changes.md). Keep this file as the consumption-patch record; do not re-apply the replacements. Recommended field-patch `plan amend` retirement stays out of scope.
>
> [RFC-104](rfc-104-system-archaeology.md) states the definition-side consumption contract. This file is the RFC-88-side wording so the two documents agree. It is not a new lifecycle RFC. [platform.md](../platform.md) already tells RFC-88 to use internal cuts rather than extra RFCs.

## Why this file exists

RFC-104's header, and RFC-99's predecessor-patch rule, leave in-flight RFC-88 unchanged. The definition review still produces consumption patches: adapter-native surface grain, WIT ownership, the observed-vs-delivery CID fence, and declared adapter identity on the handoff. It also parks field-patch `plan amend` retirement here as recommended, not as a definition-home change. Editing RFC-88 in the RFC-104 branch would collide with the in-progress RFC-88 cut.

Fold the must-apply items in the next RFC-88 edit. Until then, RFC-104 D3 / D10 are authoritative for what import must do; this file is how RFC-88's text catches up. RFC-104 D11 no longer deletes field-patch `plan amend`; that retirement is the recommended item below.

## Must apply (decided in RFC-104)

### 1. Surface grain — compose upward; `focus` is the exception

Estate `survey` emits the smallest surface the adapter can name (one endpoint, topic, job, document, handler, screen, or intent string). It does not emit slices and does not emit system-model elements. RFC-88 groups imported surface leads into slices. Focused child survey runs only when an imported lead is still coarser than a buildable boundary — a monolithic document, a generated mega-handler — not to recover endpoints that estate survey already emitted.

Replace D1 `leads.md` sentence:

```text
`leads.md` contains the selected wave's delivery evidence scopes and any focused child leads with their parent lead; it is not the system inventory or architecture model.
```

with:

```text
`leads.md` contains the selected wave's imported surface leads and any focused child leads with their parent lead; it is not the system inventory or architecture model.
```

Replace the D2 paragraph that begins `RFC-104's initial survey`:

```text
RFC-104's initial `survey` emits source-local evidence scopes and extracts them into the reviewed system model. RFC-88 imports only the scopes attached to the selected wave. To split a broad scope without guessing from its synopsis, the source contract lets the engine survey that lead as a focus and append stable child leads under it. The engine controls recursion and budgets; an adapter handles only one requested source scope.
```

with:

```text
RFC-104's estate `survey` emits adapter-native surfaces at their smallest stable unit and extracts them into the reviewed system model. RFC-88 imports only the surface leads attached to the selected wave and groups them into slices. Focused child survey runs only when an imported lead is still coarser than a buildable boundary — a monolithic document, a generated mega-handler — not to recover endpoints that estate survey already emitted. The engine controls recursion and budgets; an adapter handles only one requested source scope.
```

Replace D3 phase 2:

```text
2. **Focus delivery scopes** — import the wave's Evidence scopes and survey only where a broad scope needs source-local child detail for a buildable boundary; RFC-96 may fan these independent calls out without changing their stable merge order.
```

with:

```text
2. **Focus delivery scopes** — import the wave's surface leads and survey only where a remaining lead is still coarser than a buildable boundary; RFC-96 may fan these independent calls out without changing their stable merge order.
```

In Flow step 3, “obtains any focused delivery leads the wave still needs” means the exception path above, not a second estate survey. Example `lead: orders-api` ids in RFC-88 become surface ids (`post-orders`, `orders-created`) when those examples are next touched. RFC-104's handoff example already maps two surface leads onto one target.

Leaf binding is unchanged: at most one terminal lead per source per leaf; `extract` still takes one terminal `(source, lead)` pair.

### 2. WIT ownership — RFC-104 lands input; RFC-88 adds `focus` over the delivery pin

RFC-104 owns `survey(adapter, source-key, input)` / `extract(adapter, source-key, input, lead)`. The wire carries the prepared RFC-87 workspace or inline content, not the locator and not `observed-cid`. RFC-88 does not re-land that half. It extends `survey` with optional parent-lead `focus` and a stable child-lead response **over the delivery pin**. Estate survey has no parent; focused survey is a delivery-time call, not a filter on the observed archaeology tree.

After the D2 sentence `They never parse plan.yaml, assume that a source is a target, or capture a source workspace.`, append:

```text
 RFC-104 lands that explicit-input WIT; this RFC extends `survey` with optional parent-lead focus and a stable child-lead response over the delivery pin.
```

Replace the implementation-requirements WIT bullet:

```text
- The source WIT receives either a read-only workspace or inline value. Extend `survey` with an optional parent-lead focus and stable child-lead response so the engine can request source-local detail without adding a third source operation. `extract` continues to consume a terminal lead and return Evidence. The refinement judgment adds the typed `proceed | boundary-escalation` outcome. Target-axis adapters continue to receive a prepared workspace and read-only change artifacts.
```

with:

```text
- RFC-104 lands source key and prepared input (read-only workspace or inline value) on `survey` / `extract`. This RFC extends `survey` with an optional parent-lead focus and stable child-lead response so the engine can request source-local detail over the delivery pin without adding a third source operation. `extract` continues to consume a terminal lead and return Evidence. The refinement judgment adds the typed `proceed | boundary-escalation` outcome. Target-axis adapters continue to receive a prepared workspace and read-only change artifacts.
```

### 3. Observed CID is archaeology provenance, not the delivery pin

A coverage `location` stays a mutable origin locator. RFC-104 re-fetches every survey, prepares an RFC-87 read-only workspace, and writes `observed-cid` (and `observed-revision` when Git reports one) on the coverage row. The handoff copies those fields onto `evidence-scopes[]` as provenance of what was read.

RFC-88 re-resolves the same locator at bind time and pins the **delivery** CID. Later delivery runs use that pin. Do not treat a handoff `observed-cid` as the delivery source pin. The two CIDs may differ if the origin moved between archaeology and binding. Snapshot objects from a survey fetch are not delivery GC roots.

`evidence-scopes[]` is a selected subset of estate-survey surface leads, typically several from one source mapped onto one target. They are not broad parents waiting to be split, and they are not system-model elements.

When RFC-88 describes binding, say that it re-resolves the locator and pins the delivery CID; mention `observed-cid` only as imported provenance that does not authorize the pin.

### 4. Handoff adapter identity is declared, not resolved

RFC-104 copies `adapter` onto `targets[]` and `evidence-scopes[]` exactly as the operator declared it: a bare name or an exact package pin. The handoff does not resolve a name to a pin.

RFC-88 fills a declared name to an exact package pin at bind time. A pin already in the handoff is frozen — do not re-resolve it, and do not treat a coverage-row name as if archaeology had pinned a version.

When RFC-88 describes adapter binding, say that it fills a handoff name and keeps a handoff pin.

### 5. Import wire facts (as implemented)

Two wire facts import depends on, recorded here so the next RFC-88 cut cites them instead of re-deriving them from the code. Neither changes the contract above.

- **The review-event envelope.** The fact `plan author --from` must verify is `system.wave.reviewed`, carrying `{ wave, handoff-digest }`, appended to the definition home's own per-writer log at `<system>/events/<writer>.jsonl` (union-read by `(timestamp, writer, sequence)`; definition-home writers only, never `.emery/events/`). The fact grants architectural authority only — it does not replace `plan.execute.started` and carries no product mutation authority.
- **Handoff verify-on-read.** Handoffs are content-addressed at `handoffs/<digest>.yaml` (bare 64-hex filename, `sha256:`-prefixed digests inside). The RFC-104 loader recomputes the content address on read and rejects drift as `system-handoff-corrupt`. Import must load through the same verification rather than trusting the filename, and must match the reviewed fact's `handoff-digest` against the verified address.

### Field-patch `plan amend` retirement

D11's “no `system amend`” decision is correct. Folding in deletion of `--description`, `--depends-on`, `--add-source`, `--remove-source`, `--sources`, `--divergence`, `--authority-override`, `--allow-composition-replace`, and the `plan.amend.divergence` / `plan.amend.authority-override` facts is a live delivery-loop rewrite.

That change:

- is unnecessary for a definition engagement that never touches `plan.yaml`;
- collides with RFC-88, which still requires leaf `plan add` / `amend` / `remove` to lower onto domain mutation, and keeps `plan amend --proposal`;
- rewrites the current “CLI is the single writer of Divergence” contract in `workflow.md`, skills, how-tos, and journal taxonomy;
- delays the paid archaeology package for a consistency aesthetic.

Keep on RFC-104: declared definition inputs are hand-edited; the next stage validates. There is still no `system amend`.

If the next RFC-88 cut takes the retirement: delete those field-patch flags and amendment facts there, or in a one-page follow-on after the definition loop exists. RFC-104 D11 no longer deletes them. RFC-88's `plan amend --proposal` compare-and-set is a different operation and stays.

[platform.md](../platform.md) already tells RFC-88 to use internal cuts rather than extra lifecycle RFCs; parking this here is the same discipline in the other direction.

## Do not take from RFC-104

- Definition-home `Layout`, `--dir` (else CWD), and launcher mounts. Those are RFC-104. `--from` stays a read-only extra preopen on this RFC.
- Adapter survey/extract prompt retargeting. That is RFC-104 cut 1 in `augentic/emery-adapters`.
- Correlation, `system.yaml`, diagrams, waves, `system.wave.reviewed`. RFC-88 imports one reviewed handoff; it does not correlate the estate.
- A second Evidence schema, an Evidence cache, or `focus` on estate survey.
