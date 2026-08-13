# RFC-104 review: implement; contract patches have landed

> Status: Review notes against [RFC-104](rfc-104-system-archaeology.md), [platform.md](platform.md), Propellerhead strategy, and the current engine / adapter codebases.
>
> Date: 13 August 2026
>
> Verdict: **Implement RFC-104.** The product direction is right. Must-fix contract holes, the D11 scope split, and the tighten items below are now in the RFC. Do not reopen D1–D12’s product intent.

Reviewed against:

- [RFC-104](rfc-104-system-archaeology.md)
- [Services Delivery Programme](platform.md)
- [RFC-88](rfc-88-detached-changes.md), [RFC-87](rfc-87-working-trees.md), [RFC-86](rfc-86-change-facts.md)
- Propellerhead strategy (`strategy/brand.md`) and messaging (`modernization.md`, `readiness.md`, `how-we-work.md`, `new-systems.md`)
- Current source WIT (`wit/emery.wit`), survey orchestration, `SourceBinding`, launcher anchoring, first-party adapter survey prompts

---

## Verdict

RFC-104 already matches Propellerhead’s wedge: a paid definition engagement that can finish without slices, with coverage-accounted archaeology, evidence-linked architecture, and a reviewed wave before delivery. Platform.md, the brand strategy, and the readiness copy all ask for that. Keep that shape.

What needed work was the seam between “survey a live estate” and “pin a delivery tree,” plus a live-loop change that does not belong on this cut. Those patches have landed.

Recommended sequence:

1. Implement in three internal cuts inside RFC-104, starting with coverage + Evidence, not diagrams or handoffs.
2. Treat emery-adapters survey/extract retargeting as part of cut 1: stop slice-clustering and emit native surfaces, not a follow-on.
3. Leave live `plan amend` flags alone until RFC-88’s proposal surface exists.
4. Fold RFC-88 consumption through [rfc-104-rfc-88-patches.md](rfc-104-rfc-88-patches.md); do not edit in-flight RFC-88 from this cut.

The remaining judgment (renderer, no `system init`, no Evidence cache, no forge crawler) is already the right v1 posture for a services firm that sells a defensible first wave, not a discovery engine.

---

## What is already right

Do not reopen these:

- Definition home as client architecture, not a workspace registry.
- Completeness as coverage dispositions, not “we know everything.”
- Leads as adapter-native surfaces, not promised slices or model elements.
- Model as authority; diagrams as projections.
- Modernization dispositions so accidental structure does not become requirements.
- Transition architecture and wave richness (state movement, coexistence, cutover, rollback).
- One immutable handoff + `system.wave.reviewed` before RFC-88 import.
- No `system init` / `system amend` / `system archive` as lifecycle verbs.
- Archaeology-only engagements as a legitimate completed outcome.
- Degenerate new-system path through the same three commands, with no flag-only bypass.
- D12 write-back deferred ([RM-23](roadmap.md#rm-23-baseline-write-back) already owns that).

That is the services loop platform.md describes, and it is the readiness deliverable the website copy sells.

---

## Must-fix before coding

### 1. Survey still materializes trees; “no CIDs” is the wrong fence

This is the most important hole.

RFC-104 D2/D3 say this RFC does not acquire CIDs; RFC-88 pins them at bind time. D3 still requires a prepared read-only `source-input` of a recorded URL or path. RFC-87 already says RFC-104 uses its read-only values for location-backed definition sources. Acceptance criterion 2 requires surveying recorded URLs, not only local paths.

Those cannot all be true as written. You cannot survey `https://github.com/acme/orders` without fetching a tree. If you fetch it and do not record what you saw, Evidence path anchors are unanchored, re-survey is not comparable, and the client package is not defensible — which is the commercial claim.

The intended distinction is real and worth keeping:

| Archaeology (RFC-104) | Delivery (RFC-88) |
| --- | --- |
| Mutable origin locator | Exact revision + CID |
| Re-fetch every survey | Later runs use the pin |
| Provenance of what was observed | Identity of what will be built |

Rewrite the fence as: **RFC-104 does not pin delivery CIDs into the handoff.** It should still:

- resolve a locator to a concrete tree (local path, or fetch a Git/HTTPS origin);
- prepare an RFC-87 read-only workspace;
- record an **observed tree digest** (and Git revision when applicable) on the coverage row or Evidence envelope as survey provenance;
- leave RFC-88 to re-resolve and pin the delivery CID at bind time.

Without that, every `system survey` over a Git URL is a live read of a moving origin, and the “evidence-linked” deliverable cannot say which tree produced the claims.

**Resolved in RFC-104.** D2/D3 keep the locator mutable and re-fetch every survey. A successful included survey writes `observed-cid` (RFC-87 tree identity) and, when applicable, `observed-revision` on the coverage row; the handoff copies those fields as provenance. RFC-88 still re-resolves and pins the delivery CID at bind time. The Evidence document schema is unchanged.

### 2. Own the source WIT change; leave `focus` to RFC-88

The RFC says it reuses the existing `survey` / `extract` contract, then specifies:

```text
survey(adapter, source-key, input, focus?) → Lead[]
extract(adapter, source-key, input, lead) → Evidence
```

Today the WIT is `survey(id: adapter-id)` / `extract(id, lead)`. The engine recovers the source from `plan.yaml` and the host lends `$SOURCE_DIR` out of band. That is exactly the recovery path RFC-104 rejects.

RFC-104 should own, in this cut:

- pass **source key + prepared input** (workspace or inline) into both operations;
- stop recovering location from `plan.yaml`.

It should **not** specify `focus?` or child-lead survey. That is RFC-88 D2/D3. Landing it here either ships a no-op parameter or forces a second WIT bump. RFC-88 already requires the focus extension; keep it there.

**Resolved in RFC-104.** D3 owns the WIT change: `survey(adapter, source-key, input)` / `extract(adapter, source-key, input, lead)`. The wire carries the prepared RFC-87 workspace or inline content, not the locator and not `observed-cid`. The live `plan author` / `emery source survey` path migrates onto that WIT in this cut. `focus` and the child-lead response stay on RFC-88, which now consumes the input-passing half rather than landing it.

### 3. First-party adapters still emit slice-sized leads

Engine reuse of `extract` is not enough. Current TypeScript and documentation survey prompts are explicit: “slice-sized units of work” into `discovery.md`. TypeScript then collapses every surface under 1000 LOC into one source-level lead and merges the rest until they look like slices. That is the adapter guessing at an engine noun. Asking it to emit service, store, or journey leads instead would be the same guess aimed at the system model.

RFC-104’s implementation requirements should name **emery-adapters** as in-scope for this cut:

- `survey` emits the adapter’s native surfaces at their smallest stable unit (endpoint, topic, job, document, handler, screen, intent); delete slice-clustering and do not invent model-element leads;
- `extract` of each surface emits existing claim kinds for what that surface actually has, so correlation can evidence D4 relationships;
- keep the Evidence document schema (right rejection);
- same prompt corpus for both loops; RFC-88 `focus` only when a remaining lead is still coarser than a buildable boundary.

Without that, the engine will persist clustered work units and the as-is model will be a renamed lead inventory.

**Resolved in RFC-104.** D3 defines a lead as an adapter-native surface, not a slice and not a model element. Correlation composes many surface Evidence documents into services, stores, and relationships; RFC-88 groups imported surfaces into slices and uses `focus` only for leftover coarse leads. Cut 1 names emery-adapters: TypeScript drops the LOC collapse and merge; extract keeps the Evidence schema and must carry the calls that surface actually has. The handoff example maps two surface leads onto one target.

### 4. Definition-home mounting is a launcher change

`--dir` / CWD is not a new flag on an existing product walk. The launcher currently anchors on `project.yaml`. A detached definition home has none. Auto-discovering `.emery/system/` via that walk is a third resolver without a proven need.

**Resolved in RFC-104.** One selector: `--dir` if present, else CWD. The launcher mounts that directory as `.` and does not walk for `project.yaml` or `.emery/system/`, create the root, or key adapter cache off it. The guest fail-closes without `scope.yaml`. A definition-home `Layout` owns `<system>/events/`. Origin trees enter through RFC-87 workspaces. `.emery/system/` is a place files may live (`--dir` or `cd`), not a resolver rule. `--from` stays on RFC-88 as a read-only extra preopen. Do not reuse `--project-dir`. Platform.md now excepts this mount projection from the launcher-is-orthogonal claim.

### 5. Specify the mixed-file write rules

Coverage rows, `system.yaml`, and `decisions/` are mixed-ownership. D1 said survey does not overwrite declared locators; D3 said a failed source updates its coverage row; `status: decided` named a `decisions/` record with no identity; survey overwrites `as-is` while plan writes `target` / `transition-*` only when absent. Without a persist contract the first cut invents those answers.

**Resolved in RFC-104.** Operator-owned coverage fields are `key`, `location`, `adapter`, `disposition`, and `reason`. Survey writes `observed-cid` / `observed-revision` only when an included source completes, and `survey-error` `{ kind: access | adapter, detail }` on access or adapter failure; it clears `survey-error` on the next success and does not flip disposition, rewrite locators, or clear a prior observed tree. `inaccessible` / `unsupported` remain operator-declared. Definition decisions are operator-authored YAML at `decisions/<id>.yaml` (kebab `id` = filename stem; Nygard `context` / `decision` / `consequences`; optional `applies-to` / `supersedes`); digest is canonical YAML; the engine never writes the directory; absent `decisions/` is valid. `status: decided` carries `decision: <id>`; correlation cannot emit it; survey persist reapplies `applies-to` after writing `as-is`; plan and review validate the overlay and do not stamp it. Handoff `decisions[]` resolves there; current-handoff identity includes `decisions-digest`. Mixed files persist surgically: load → replace only the generated named state or survey-owned coverage fields this run touched → canonical write. Correlation's answer is `as-is` only. Plan writes `target` and proposed `transition-*` only when `target` is absent at load; later plans reproject views and a new handoff and do not add named states.

---

## Split out of this RFC

RFC-104 states the definition-side consumption contract. It does not revise in-flight RFC-88. Successor wording for that RFC lives in [rfc-104-rfc-88-patches.md](rfc-104-rfc-88-patches.md): surface grain (compose imported surfaces into slices; `focus` only for leftover coarse leads), WIT ownership (RFC-104 lands input; RFC-88 adds `focus` over the delivery pin), the observed-vs-delivery CID fence, and declared adapter identity (RFC-88 fills a name at bind time; a pin in the handoff is frozen).

D11’s “no `system amend`” stays. Do not delete field-patch `plan amend` from this RFC; that retirement is recommended, not must-apply, in [rfc-104-rfc-88-patches.md](rfc-104-rfc-88-patches.md).

---

## Tighten, but do not redesign

### 1. Correlation is estate-sized; bound it, do not partition it

One `kind: request | response` judgment with `repaired()` / `MAX_REPAIRS` is the right envelope. Surface-grain survey makes this sharper: the request is many endpoint-sized Evidence documents, not one lead per service. Without a v1 bound the first real multi-repo survey blows the context window and the quote. RFC-92 can measure spend later; the gate has to exist now.

**Resolved in RFC-104.** Two engine constants sit beside `MAX_REPAIRS` (not on `scope.yaml`, not policy-increasable). After included `survey` and before `extract`, lead count over the ceiling is `system-survey-lead-limit`. Before correlation, claim count over the ceiling is `system-correlation-claim-limit`. Either stop leaves `as-is` unreplaced. Recovery is D2: narrow coverage or author another definition home. The engine does not partition the estate or split the correlator.

### 2. Internal cuts, same RFC

Mirror RFC-88’s implementation advice. Acceptance stays “the loop through `system.wave.reviewed`.” Cuts control risk; they are not partial public lifecycles.

**Resolved in RFC-104** (and [platform.md](platform.md)):

1. Definition home + coverage + location materialization + survey/extract into definition-home Evidence, including emery-adapters surface grain and the lead-count gate.
2. Correlation + empty-as-is persist + `system.yaml` as-is + as-is.md / diagrams.
3. `system plan` (dispositions, target/transitions, waves, canonical handoff) + `system review`.

### 3. New-system degenerate must not fail closed

Empty `as-is` and intent-only `as-is` are different. Sending an empty correlation request to the model hallucinates structure or fails closed. Plan must still be able to write `target` plus one wave.

**Resolved in RFC-104.** Zero included Evidence persists empty `as-is` deterministically and skips correlation. Intent-only Evidence is a valid correlation request. Initial `system plan` when `target` is absent is a separate proposal judgment over live `as-is` (possibly empty), `scope.yaml`, and any included intent or constraint Evidence; later plans do not add named states.

### 4. Handoff `adapter` pins

D10 already allows a name or a pin. Projecting a resolved pin would make archaeology look like a delivery freeze.

**Resolved in RFC-104.** The handoff copies what the operator declared; RFC-88 fills a name to a pin at bind time; a pin in the handoff is frozen. The example now shows that mix on purpose: coverage and evidence-scopes carry `typescript`; the target carries `emery:omnia@1.4.0`.

---

## Leave to implementation

These are correctly deferred:

- Named diagram renderer (pick a deterministic textual notation with stable ids and reproducible SVG; D2/Mermaid/Graphviz are an implementation choice).
- Per-kind attribute catalogs.
- RFC-95 write-back DTOs.
- Forge namespace expansion and adapter auto-recognition.
- `system init` as a lifecycle verb. A fail-closed hint that prints the two-file template is enough for v1. A scaffold-only helper can wait until hand-authoring actually burns engagement time.

Do not put commercial recommendation, staffing, or price into the engine. Readiness copy’s “proceed / fix gaps / stop” is practitioner judgment over the archaeology package.

---

## Fit to strategy and codebase

| Strategy need | RFC-104 | Code today |
| --- | --- | --- |
| Paid readiness without building | Yes — loop may end at survey or wave review | Workflow starts at `plan author` |
| Coverage-accounted map | `coverage.yaml` dispositions | Failed/unmatched sources disappear |
| Evidence-linked as-is + diagrams | `system.yaml` + projections | Per-slice `design.md` after decomposition |
| Keep vs change vs retire | Modernization dispositions | Synthesis treats observed behaviour as requirements |
| Bounded first wave | Handoff + review fact | `plan.yaml` is a slice backlog |
| New systems from intent | Degenerate definition, no bypass | Same delivery-shaped survey |
| People / interviews | Implicit via `intent` / docs paths | `intent` adapter exists; no new axis needed |
| Living baseline after delivery | D12 successor + RM-23 | Correctly not v1 |

The current engine can host this: RFC-86 journals, RFC-87 workspaces, Evidence documents, judgment repair loop, canonical YAML digests (`BuildRecord` / wave already do this). Adapter surface grain, definition-home mounting, mixed-file write rules, the D11 scope split, estate-size gates, empty/intent-only `as-is`, internal cuts, and handoff adapter-identity copy are specified. Implementation starts at cut 1.

---

## Codebase anchors

These are the live seams cut 1 has to change. The RFC now specifies the contract; the code still implements the delivery-shaped path:

- Source WIT: `survey(id)` / `extract(id, lead)` in `wit/emery.wit` — location is not on the wire.
- Survey orchestration recovers bindings from `plan.yaml` (`crates/change/src/orchestrate/survey.rs`).
- `SourceBinding` is `adapter` + `path` xor `value` + optional `cid` (`crates/project/src/plan/model/source.rs`).
- Launcher anchors on `project.yaml` (`crates/launcher/src/anchor.rs`); definition homes have none.
- Journal layout is `<project>/.emery/events/<writer>.jsonl` (`crates/project/src/journal.rs`).
- TypeScript / documentation survey prompts still emit slice-sized leads (`emery-adapters` `sources/*/prose/prompts/survey.md`); RFC-104 D3 now requires surface grain in that cut.
- RFC-87 header already states RFC-104 uses D2 read-only values for location-backed definition sources.

---

## Proposed RFC patch checklist

Use this as the review gate before implementation starts:

- [x] Rewrite the CID fence: no delivery pin in the handoff; survey still materializes a tree and records observed digest / revision.
- [x] Specify source-key + prepared input on `survey` / `extract`; drop `focus?` from this RFC.
- [x] Name emery-adapters survey/extract prompt retargeting as in-scope for cut 1 (native surfaces, not slices or model kinds).
- [x] Park RFC-88 consumption patches beside RFC-104; do not edit in-flight RFC-88.
- [x] Require a definition-home `Layout` and launcher mount policy (`--dir` or CWD; no product walk); do not reuse `--project-dir`.
- [x] Close coverage failure writes, `decisions/` identity, and surgical `system.yaml` writes.
- [x] Split field-patch `plan amend` retirement out of D11 (rationale in [rfc-104-rfc-88-patches.md](rfc-104-rfc-88-patches.md)).
- [x] Add estate-size gates: lead count before extract, claim count before correlation; engine constants; typed stops; D2 recovery.
- [x] Record internal implementation cuts (Evidence → model → handoff/review).
- [x] Allow empty/intent-only `as-is` for the degenerate new-system path; skip correlation on empty Evidence.
- [x] One sentence on handoff adapter name vs pin; copy declared identity, do not resolve at projection.
