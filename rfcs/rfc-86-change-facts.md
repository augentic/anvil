# RFC-86: Change Facts

> **Status:** Draft — incomplete. Open product questions remain at the end.
>
> **Series:** Step 1 of the [platform-migration series](platform.md). Later RFCs (working trees, detached changes, publication, concurrency, node sync) all build on this.
>
> **Audience:** Operators and contributors who know Emery’s workflow vocabulary (plan, slice, refine, build, merge, sources, specs). Implementation detail is deferred to an appendix.

## Synopsis

This RFC does two related things:

1. **Shift specs left.** Creating per-slice specs (`refine`) moves out of `plan execute` and into the **plan** phase. Execute becomes **build → merge** only, after the operator has reviewed and approved those specs.
2. **Make change state shareable.** Workflow progress stops living in mutable status fields. A change becomes a set of durable files and an append-only history. Status is *computed* from that history. The same model works on one laptop or across several people and machines.

Together: better specs before generation, and a state model that can travel.

---

## Why

Emery already treats specs as the contract that drives build. Real projects show that **build quality tracks spec quality**. Today that contract is created too late, and known holes do not stop generation.

### What happens today

```text
/emery:plan            survey sources → propose slices → review the plan
emery plan execute     for each slice: refine → build → merge
```

Problems:

| Problem | In plain terms |
| ------- | -------------- |
| Specs appear mid-execute | Plan review sees leads and `plan.yaml`, not `spec.md`. By the time specs exist, the loop is already heading into build. |
| Gaps do not stop build | Refine already tags `[unknown]`, `[conflict]`, and `[divergence]` on requirements. Those tags are informational only — execute builds anyway. |
| Approval is invisible | Running `plan execute` *is* the approval. Nothing records what was approved. |
| Status is hard to share | Progress is stored as fields in YAML files. Two people cannot safely collaborate on one change, and the same pattern will not scale to multiple machines. |
| Requirement IDs collide | Two slices refined against the same baseline can mint the same `REQ-NNN` numbers. Merge can silently overwrite. |

### What we want instead

```text
Plan phase     author slices → plan refine → review gaps → approve
Execute phase  build → merge   (only after approval)
```

- Operators review **topology** after author (before synthesis spend), then read **real specs** after `plan refine` and check the **gap inventory** before any code generation. Prose quality is human-owned — the engine does not record how spec reading happens (see D17).
- Known gaps are listed and, by default, **block approval to build**.
- Approval is a recorded artifact: who approved what, against which specs.
- One person on a laptop and a multi-person (later multi-node) change use the **same** rules.

Unchanged: source and target adapters, artifact shapes (`spec.md`, Evidence, etc.), and the meaning of refine / build / merge as verbs. What changes is **when** refine runs (via `emery plan refine` in the plan phase, not inside execute), **whether** gaps may enter build, and **how** progress is stored.

---

## Operator flow

### Plan phase — intent through specs

1. **Author** — `emery plan author` (via `/emery:plan`) surveys bound sources and reconciles leads into slices. Produces `discovery.md`, `change.md`, and `plan.yaml` (which work exists). **Stops here** — no extract or synthesis yet, so the operator can review topology before paying for refine.
2. **Refine** — `emery plan refine` (default: every unrefined in-scope slice; optional slice selectors) claims slices and runs the same per-slice refine implementation as today (`emery slice refine`): extract Evidence, synthesize `proposal.md`, `spec.md`, `design.md`, `tasks.md`, and `model.yaml`, and record input pins. Prints or leaves the gap inventory ready for review.
3. **Review** — Read `spec.md` and related slice artifacts, and the **gap inventory** (see below). Prose quality and acceptance-criteria depth are operator-owned (see D17) — the engine does not score or record how that reading happened. Optional **topology-only** approval may sit between author and refine when handing off the slice list before specs exist.
4. **Iterate** — Fix inputs (richer intent/docs, authority overrides, corrected sources), then re-refine only the affected slices with `emery slice refine <slice>` (or `emery plan refine` over a subset).
5. **Approve** — Record a **build approval** once the change is ready (all in-scope slices refined; no open conflicts; unknowns cleared or waived per requirement on approve — see D16).

### Execute phase — code through merge

6. **Execute** — `emery plan execute` runs **build → merge** per slice. It does **not** extract or synthesize again.
7. **Finalize** — Operator publishes; archive as today.

`/emery:plan` remains an ultrathin wrapper over `plan author` only. Hand-driven breakouts (`emery slice refine` / `build` / `merge`) still work; `plan refine` is the batch fan-out over that same refine implementation. The drained execute loop simply no longer contains refine.

One-slice changes stay the same shape, only shorter: author → `plan refine` → review → approve → build → merge. `plan status`’s resume points at `emery plan refine` when any in-scope slice is still unrefined.

### Who owns what

| Work | Plan phase | Execute phase |
| ---- | ---------- | ------------- |
| Survey and propose slices (`plan author`) | yes | no |
| Extract and synthesize specs (`plan refine` / `slice refine`) | yes | **no** |
| Review specs (prose) and close typed gaps | yes | no |
| Approve for build | yes | required before starting |
| Build and merge code | no | yes |

---

## Gaps before build

### What refine already tells us

When sources are combined into a requirement, Emery assigns a status:

| Status | Tag | Meaning |
| ------ | --- | ------- |
| `agreed` | — | Sources agree (or there is only one claim) |
| `unknown` | `[unknown]` | **Gap** — not enough evidence; the requirement is incomplete |
| `divergence` | `[divergence]` | Sources disagree; a higher-authority source won; the loser is kept as a note |
| `conflict` | `[conflict]` | Sources disagree and no automatic winner — **unresolved contradiction** |

Plan authoring can also flag fuzzy *slice* matching (tentative merges, `divergence: likely`). That is about how work is grouped. Requirement gaps appear only after refine — another reason specs must exist before build approval.

### Gap inventory

After refine, the operator sees a single list of open findings across slices, for example:

```text
slice          req        status       summary                         shared-lead
auth-login     REQ-003    unknown      password-reset path not evidenced  docs:conventions
payments       REQ-008    unknown      reset copy not evidenced           docs:conventions
auth-login     REQ-007    conflict     session TTL: docs vs intent (tied) —
payments       REQ-012    divergence   retry budget: docs beat behaviour  —

# shared lead docs:conventions → re-refine selectors: auth-login payments
```

This list is **derived** from the specs/`model.yaml` already on disk. It is not a second file to keep in sync. It includes only typed requirement statuses (`unknown`, `conflict`, `divergence`) — not prose-quality advisories (see D17). `plan status` (and a dedicated gaps command — name TBD) surfaces it.

**Gate authority stays per requirement** — each row is still `(slice, req, status)`. When the same lead is multi-homed across slices (coverage is at-least-once; cross-cutting leads in `change.md`), one thin or contradictory lead can surface as several inventory rows. `plan gaps` therefore also offers a **presentation rollup**: annotate or group open findings that share a contributing `(source, lead)`, and suggest the slice-selector set for a follow-up `plan refine` / `slice refine` after the shared input is fixed (see D18). The rollup is navigation only — it does not merge findings, change the approve gate, or introduce lead-wide waivers.

### How to close gaps

Do not hand-edit the machine-rendered `ID:` / `Sources:` / `Status:` lines or the `[…]` tags. Change the inputs, then re-refine (same rule as today’s conflict how-to):

| Finding | Typical fix | Next step |
| ------- | ----------- | --------- |
| `[unknown]` | Operator supplies the missing information: enrich a source (intent, docs, captures), or re-scope the lead so the requirement is not invented. Build stays blocked until the tag clears or that requirement is explicitly waived on approve (see D16) | Re-refine that slice (and any siblings the rollup lists for the same lead) |
| `[conflict]` | Set an authority override, or remove/correct a misleading source. **Not waiveable** — resolve inputs, then re-refine | Re-refine (per slice that still shows the conflict; overrides remain per-slice) |
| `[divergence]` | Informational — authority already chose; no decision required. Override only if the wrong source won | Re-refine only if inputs changed |
| Stale inputs | Sources or baseline moved since refine pinned them | Re-pin and re-refine |

Prefer fixing one shared input, then re-refining the affected slice set (the rollup’s suggested selectors when findings share a lead). Full-plan re-synthesis is expensive. Waivers stay per-requirement (`--waive <slice>/<req>`) even when rows share a lead.

### Human-only ambiguity (outside the engine)

Vague prose in an otherwise `agreed` requirement (weak scenarios, missing acceptance criteria) stays a **human** review concern. The operator reads `spec.md` / `design.md` by whatever process fits — IDE, PR, checklist on paper, pair review — and decides whether to approve. The agentic and programmed workflow does not model that step: no checklist artifact, no review attestation on `plan approve`, and no spec-quality findings in the gap inventory (see D17). This RFC only machine-gates the typed statuses above.

### When build is allowed (gap policy)

Refine may finish with tags still present — the slice is refined, but not necessarily ready to build.

**Build approval** checks a gap policy:

| Finding | Policy |
| ------- | ------ |
| `[conflict]` | **Block, not waiveable** — do not generate code over an unresolved contradiction. Resolve via authority override or source correction, then re-refine (see D16) |
| `[unknown]` | **Block** — insufficient information reached the agent; the operator must supply it (richer sources, re-scope) or **explicitly waive that requirement** on approve before build. Uniform for every change shape — including intent-only / N=1. Desk-testing shows warn-only yields unpredictable generation that compounds through build and merge (see D14, D16) |
| `[divergence]` | **Allow, but list** — informational only; authority already chose (`intent` > `documentation` > `behaviour`, plus any per-slice override). No acknowledgment or decision required for approval. Override and re-refine only when the wrong source won (see D15) |

If the policy fails, approve refuses and prints the inventory. The operator may:

- close the findings and approve normally, or
- **explicitly waive** individual `[unknown]` requirements on the approve command (recorded on the approval, each with a reason) — never silently, never as a plan-wide or slice-wide off-switch, and never for `[conflict]` (see D16).

Running execute interactively must **not** auto-waive gaps. It may auto-record approval only when the change is already ready (clean inventory, or a prior approval that already carries any needed unknown-waivers).

Two approval scopes:

| Scope | Covers | Unlocks execute? |
| ----- | ------ | ---------------- |
| Topology only | The plan’s slice list (handoff before specs exist) | **No** |
| Build | Plan + current spec digests + gap outcome (including any unknown-waivers) | **Yes** |

If someone re-refines after a build approval, the approval goes stale — including every waiver it carried — and must be done again (re-list any remaining unknown-waivers).

---

## How state works (the substrate)

Today, “where is this change?” answers are scattered: status fields in YAML, one shared journal file, approval only as a gesture. That works for one operator on one machine. It does not travel.

### Three kinds of thing

| Kind | Role | Examples |
| ---- | ---- | -------- |
| **Artifacts** | Durable files that describe the change | `plan.yaml`, per-slice `spec.md` / `design.md` / Evidence, approval records |
| **Facts** | Append-only history of what happened | “slice claimed”, “plan approved”, “build succeeded”, “merge completed” |
| **Values** | Large payloads referred to by digest (later RFCs) | Code tree revisions and changesets |

Nothing else is workflow authority. In particular, **status is never stored as a field to edit**. `plan status` *computes* progress from artifacts + facts.

### Rough layout of a change

```text
<change>/
  change.md
  plan.yaml                 # what slices exist — not their status
  approvals/…               # recorded approvals
  events/<person-or-node>.jsonl   # each actor appends only their own log
  slices/<slice>/
    base.yaml               # what baseline and sources this refine used
    evidence/…
    spec.md, design.md, tasks.md, model.yaml
    builds/…                # build outcomes tied to a specific spec
```

Sharing a change is ordinary git (push / pull / PR). Two people’s event logs merge without fighting over one journal file.

### Progress, computed

| Milestone | Meaning |
| --------- | ------- |
| Authored | Plan has slices |
| Refined (per slice) | Validated specs exist for that slice, pinned to known inputs (`plan refine` or `slice refine`) |
| Ready | Every in-scope slice is refined, and the gap policy passes (no conflicts; unknowns cleared or waived per D16) |
| Approved | A build approval covers the current plan and specs |
| Built / merged | Build and merge facts exist for that slice |

`plan status` next actions follow the phase split: `plan refine` / `slice refine` / review gaps / approve in plan phase; build / merge after approval.

### Pins and requirement IDs (why implementers care)

Two rules make multi-slice and multi-person planning safe:

1. **Pins** — Refine records which baseline and source snapshots it used. If those move later, validate reports staleness instead of silently building on drift.
2. **Local then global IDs** — Each slice uses its own requirement ids while planning. Merge assigns final baseline `REQ-NNN` numbers and records the mapping. Two slices can no longer collide by minting the same id at refine time.

### Desktop = simplest deployment

One operator, one machine, no remote: same artifacts, same facts, same commands. Multi-person and later multi-node add transport, not a second lifecycle.

---

## Decisions (summary)

| # | Decision | Operator-visible effect |
| - | -------- | ----------------------- |
| D1 | Change is a git-backed file tree | Planning can last days; others can clone and review specs |
| D2 | Status is computed, not stored | `plan status` is the only progress view; no hand-edited status fields |
| D3 | Per-person (or per-node) event logs | Collaboration and later multi-machine sync without one contested journal file |
| D4 | Every refine/build pins its inputs | A reviewed spec is tied to what it was made from; drift is detected |
| D5 | Requirement ids finalized at merge | Parallel refine of different slices against one baseline is safe |
| D6 | Approval is a recorded artifact with scope | Auditable “who approved what”; topology handoff ≠ permission to build |
| D7 | Work is claimed in the log | Two people do not unknowingly refine the same slice |
| D8 | Phases consume pinned inputs only | Retry after failure loses no completed work |
| D9 | One lifecycle everywhere | Laptop and fleet differ only by transport config |
| D10 | Hard cut (pre-1.0) | No compatibility shims for old status fields or execute-bundled refine |
| D11 | **Plan owns refine; execute owns build/merge** | Specs are reviewed before generation spend |
| D12 | **Gaps gate build approval, not refine success** | Incomplete Evidence can still refine; it cannot silently enter build |
| D13 | **`emery plan refine` is the plan-phase batch; `/emery:plan` stops after author** | Topology review before synthesis cost; named batch for N-many; `slice refine` stays the per-slice implementation and gap-closure breakout |
| D14 | **`[unknown]` always blocks build approval** | Thin intent is not an exception — close the gap or waive it explicitly; generation must not invent missing information |
| D15 | **`[divergence]` is informational; listed but allowed** | Authority hierarchy already picked a winner; approve does not require per-divergence acknowledgment. Wrong winner → override / amend sources and re-refine |
| D16 | **Waiver UX: per-`[unknown]` on approve; `[conflict]` never waiveable; no multi-operator gate** | Repeatable `--waive <slice>/<req>` + required `--reason` on build-scoped `plan approve`; one operator’s approval is enough; re-refine clears approval and waivers |
| D17 | **Human prose review stays outside the engine** | Operators own spec quality; approve gates only on typed gap policy. No checklist artifact, review attestation, or spec-quality rollup in `plan gaps` / `plan approve` |
| D18 | **Shared-lead gap rollup is presentation only** | `plan gaps` annotates/groups open findings that share a contributing `(source, lead)` and suggests re-refine selectors; approve/waive stay per-requirement. No lead-wide waive, no shared-Evidence extract, no lead-level gate |

---

## Commands that change

| Command | Change |
| ------- | ------ |
| `emery plan approve` | **New.** Records approval (topology or build scope); build scope enforces the gap policy. For `[unknown]` leftovers only: repeatable `--waive <slice>/<req>` with required `--reason` (see D16). No `--force`, no bulk/all-gaps waive, no separate `plan waive` verb |
| `emery plan refine` | **New.** Plan-phase batch: claims and refines every unrefined in-scope slice by default; optional slice selectors for a subset. Fans out to the same orchestration as `emery slice refine` (pins, extract, synthesize). Does not approve and does not build |
| `emery plan gaps` (name TBD) | **New.** Shows the typed-status gap inventory (not spec-quality advisories — see D17). When open findings share a contributing `(source, lead)`, annotates or groups those rows and suggests the slice-selector set for re-refine — presentation only; gate and waivers stay per-requirement (see D18) |
| `emery plan execute` | Requires build approval; runs **build → merge only**; never refines |
| `emery slice refine` | Still the refine implementation and per-slice breakout (gap closure, single-slice re-refine); records input pins; used in plan phase |
| `/emery:plan` | Unchanged contract: elicit → `emery plan author` → relay; stops after topology. Does **not** run refine |
| `emery plan status` | Next actions include `plan refine` / `slice refine` / review-gaps / approve, then build / merge; resume points at `emery plan refine` while any in-scope slice is unrefined |
| `emery plan advance` / `undo` | Expressed as claim / retraction facts instead of rewriting status fields |

Exact error codes and event names belong in the [implementation notes](#appendix-implementation-notes); product behavior is above.

---

## Delivery

| Phase | Delivers | Operator should notice |
| ----- | -------- | ---------------------- |
| **A** | Fact logs + computed status | Same day-to-day flow; status still looks familiar |
| **B** | Pins + merge-time requirement ids | Safer parallel refine; drift diagnostics |
| **C** | `plan refine`, gap inventory (+ shared-lead presentation rollup), approval gate, execute without refine | The new rhythm: author → `plan refine` → gaps → approve → build/merge-only execute; multi-homed leads correlate in `plan gaps` without changing the per-req gate |

Do not implement Phase C until the [open questions](#open-questions) are closed.

---

## Acceptance (product-level)

1. Progress reported by the CLI is always computed from artifacts and facts — never read from a stored status field.
2. Two people can refine different slices on copies of one change, merge via git, and both slices show as refined without journal conflicts.
3. Two slices refined against the same baseline merge without requirement-id collision; a drifted modification is rejected instead of overwritten.
4. **Shift-left:** after authoring, every slice is refined via `emery plan refine` (or per-slice `emery slice refine`) before any build; execute performs build and merge only. `/emery:plan` does not run refine.
5. **Gap gate:** `[unknown]` prevents build approval until fixed or explicitly waived per requirement on approve (including intent-only / N=1); `[conflict]` prevents approval until resolved via override/sources and re-refine — **not** waiveable; `[divergence]` is listed but does not block (authority already chose); execute never silent-waives.
6. Topology-only approval does not unlock execute; re-refine after build approval forces re-approval (waivers on the stale approval do not carry forward).
7. The same verbs and artifacts work with no remote (solo laptop) and with the change shared over git (two people).
8. **Human review boundary:** build approval enforces typed gap policy only; prose review of `spec.md` is operator-owned and leaves no engine artifact (see D17).
9. **Shared-lead rollup:** when open findings share a contributing `(source, lead)`, `plan gaps` annotates or groups those rows and suggests re-refine selectors; approve still fails or succeeds per requirement, and waivers remain `--waive <slice>/<req>` only (see D18).

---

## Open questions

Close these before Phase C implementation.

1. ~~**How does plan-phase refine start?**~~ **Closed — D13.** `/emery:plan` / `emery plan author` stop after topology. Specs are minted by the new `emery plan refine` (batch over unrefined in-scope slices; optional selectors). Per-slice gap closure and re-refine use `emery slice refine`. Rejected: folding refine into `plan author` (pays synthesis before topology review; blurs the two review seams; makes topology-only approval awkward); status-driven `slice refine` only (no named batch — poor N-many ergonomics; agents invent ad-hoc fan-out outside the CLI contract).
2. ~~**Should `[unknown]` block by default?**~~ **Closed — D14.** Always block. `[unknown]` means insufficient information was available to the agent; the operator must provide it (or explicitly waive) before build. Rejected: warn-only for intent-only / N=1 (desk-testing — unpredictable generation that compounds through later phases); context-sensitive defaults keyed on source count or change shape (two policies to teach; under-protects thin multi-slice intent).
3. ~~**Must each `[divergence]` be acknowledged?**~~ **Closed — D15.** Listed but allowed; informational only — no acknowledgment or decision required. The kernel already applied the authority hierarchy (`intent` > `documentation` > `behaviour`, plus any per-slice override) and wrote the winner as the operative body. Rejected: require per-divergence ack before approve (ceremony over a resolved disagreement; rubber-stamp risk; conflates divergence with conflict/unknown). Authority rules may be tightened later if wrong winners prove costly in practice; that is a hierarchy/override change, not a gate change.
4. ~~**Waiver UX**~~ **Closed — D16.** Build-scoped `emery plan approve` accepts repeatable `--waive <slice>/<req>` with required `--reason`. Only `[unknown]` may be waived; `[conflict]` is not waiveable (authority override or source fix, then re-refine). Waivers nest on the approval artifact only — no separate `plan waive` verb, no plan-/slice-wide or inventory-digest waive, no `--force` / `--allow-gaps`. One operator’s recorded approval is enough; this RFC does **not** require a second-person countersign (multi-person four-eyes is a non-goal). Re-refine invalidates the approval and every waiver it carried; remaining unknowns must be re-listed on the new approve. Rejected: bulk/all-gaps waive (rubber-stamp / agent `--force` path); waiving `[conflict]` (unresolved contradiction must be decided in inputs, not papered over); separate `plan waive` then approve (extra verb and limbo without a countersign need); multi-operator waiver gating (second lifecycle / mode bit; solo laptop is the primary deployment).
5. ~~**Human-only ambiguity**~~ **Closed — D17.** Prose review of `spec.md` (and related artifacts) alone — human operators own spec quality and how they choose to review; the engine does not record or gate on that process. Rejected: optional operator checklist artifact (extra artifact, stale-on-re-refine binding, rubber-stamp risk, overlaps git/PR review without improving gap policy); rolling advisory `kind: review` spec-quality heuristics into `plan gaps` or approve (blurs the typed-status boundary; waiver creep); `--reviewed` / review attestation on approve (ceremony without substance); model-assisted spec-quality gate at approve time (non-goal for this RFC — eval / later concurrency work).
6. ~~**Shared leads across slices**~~ **Closed — D18.** Flat per-requirement inventory remains the gate authority. `plan gaps` adds a **presentation rollup**: when open findings share a contributing `(source, lead)` (multi-homed / cross-cutting leads), annotate or group those rows and suggest the slice-selector set for re-refine after the shared input is fixed. Rejected: flat list only with no correlation aid (operators and agents invent sibling fan-out outside the CLI; N-row noise from one lead); lead-wide or `--waive-lead` sugar (same rubber-stamp risk as bulk waive in D16; same lead ≠ same gap after per-slice extract); first-class lead-level gate or status noun (second checklist-like surface; derived “lead status” can lie when sibling Evidence diverges); shared extract / shared Evidence for multi-homed leads (changes the per-slice extract contract; deferred — not a gap-inventory decision).
7. **Sibling docs** — `platform.md` and later RFCs still say execute runs `refine → build → merge`; update them when Phase C decisions freeze.

---

## Non-goals

- Implementing this RFC while the open questions remain open.
- Changing Evidence schemas or the authority ranking (`intent` > `documentation` > `behaviour`).
- Automatically judging whether an `agreed` requirement is *good* (scenario depth, usefulness) — that stays human review and eval; no checklist artifact or approve-time attestation for prose review (see D17).
- Parallel swarm refine (later concurrency RFC) — this RFC only makes multi-slice refine safe and claimable.
- Multi-operator waiver / approval countersign — one actor’s build approval (with any unknown-waivers) is sufficient; shared-git collaboration stays social review of the approval artifact, not an engine four-eyes gate.
- Lead-wide waive, lead-level approve gate, or shared Evidence for multi-homed leads — correlation of shared-lead gaps is presentation-only in `plan gaps` (see D18); extract stays per-slice.

---

## Appendix: Prior art (short)

Settled patterns this RFC borrows, without adopting their full machinery:

- **Append-only operations, derived snapshots** (git-bug / Radicle COBs) — progress is replayed, not edited in place. We detect conflicting claims rather than CRDT-merging one slice.
- **Stable identity vs content identity** (Jujutsu) — slice-local requirement ids vs baseline numbers at merge.
- **Content-addressed work** (Bazel Remote Execution) — phases named by input digests; we record judgment outcomes instead of caching non-deterministic generations.
- **Approval as a statement over digests** (in-toto / SLSA) — without cryptographic envelopes in this cut.
- **Spec review before implementation** — open issues tracked against the baseline, not discovered only while coding.

---

## Appendix: Rejected alternatives

- Hosted database as status authority — forces a server and a second mode for the laptop.
- Keep mutable status and synchronize it — harder than computing status from facts; creates two lifecycles.
- Keep refine inside execute with an “optional” pre-pass — optional review is what busy runs already skip.
- Fail refine on any gap tag — blocks useful incomplete Evidence; approval is the right gate.
- Auto-waive gaps when execute is invoked interactively — recreates invisible approval for the failures we care about.
- Global requirement numbering at synthesize time — couples slices exactly when independence matters.
- Custom git merge driver for one journal file — brittle vs per-actor logs that union naturally.
- Fold refine into `plan author` / `/emery:plan` — spends extract/synthesis before the operator can re-cut the slice list; collapses topology review and spec review; leaves topology-only approval with nowhere natural to sit (see D13).
- Status-driven `emery slice refine` only (no `plan refine`) — preserves a thin CLI but forces N-many operators and agents to invent batching; the drained refine fan-out belongs in one named plan verb (see D13).
- Warn-only `[unknown]` for intent-only / N=1 (or any context-sensitive soften) — desk-testing shows generation invents missing detail and the error compounds through build and merge; thin intent closes gaps by enriching sources or waiving, not by skipping the gate (see D14).
- Require per-`[divergence]` acknowledgment before build approval — taxes a disagreement the authority hierarchy already resolved; invites rubber-stamping; blurs divergence (winner chosen) with conflict/unknown (no winner / incomplete). List in the gap inventory; fix wrong winners via override and re-refine (see D15).
- Plan-/slice-wide, inventory-digest, or `--force` / `--allow-gaps` waive — recreates invisible approval as a one-flag off-switch; agents will prefer it over closing gaps (see D16).
- Waive `[conflict]` on approve — papers over an unresolved contradiction; the operator must pick a winner via authority override or correct sources, then re-refine (see D16).
- Separate `emery plan waive` verb before approve — extra noun and waived-but-not-approved limbo once countersign is a non-goal; nest waivers on `plan approve` instead (see D16).
- Multi-operator countersign on waivers or build approval — second lifecycle / mode bit; solo laptop is the primary deployment; collaboration remains git + review of the approval artifact (see D16).
- Operator checklist artifact for spec review — extra durable file, digest/stale rules, and checkbox theater; prose review stays outside the engine (see D17).
- Review attestation flags on `plan approve` (`--reviewed`, reviewer id) — records ceremony, not quality; same boundary as D17.
- Spec-quality advisories in `plan gaps` or approve-time blocking on `kind: review` findings — conflates human judgment with typed gap policy; heuristics belong in eval, not Phase C gates (see D17).
- Flat gap list with no shared-lead correlation — forces operators and agents to rediscover multi-home fan-out from `change.md` alone; presentation rollup is cheap and keeps the CLI contract complete (see D18).
- Lead-wide / `--waive-lead` waive, or a lead-level gap gate — papers over per-requirement decisions and over-groups when sibling extracts diverge; gate and waivers stay `<slice>/<req>` (see D16, D18).
- Shared extract or shared Evidence for multi-homed leads — changes the per-slice extract contract and couples claimable slices; not a Phase C gap-inventory decision (see D18).

---

## Appendix: Implementation notes

For engine contributors. Not required to evaluate the product intent.

**Layout and writers**

- Projection kernel in `crates/project`: facts + artifact index → status, gap inventory (typed statuses only — no spec-quality rollup; D17), `ready`. Gap inventory rows stay `(slice, req, status)`; when projecting `plan gaps`, join open findings to contributing `(source, lead)` via plan bindings + Evidence/provenance and, when the same lead appears in more than one open finding, attach a presentation group plus suggested re-refine selectors (D18) — never a lead-level status field or waive expansion. Property-test: any interleaving of per-actor logs, same projection.
- Replace `.emery/journal.jsonl` with `events/<actor>.jsonl`; `emery journal show` merges the union.
- Remove stored plan-entry `status` and slice lifecycle fields; ladders survive only as projection labels.
- Approval files under `approvals/` plus matching events; build-scoped approvals embed gap counts and per-requirement `[unknown]` waiver lists (`slice` + req id + reason). Approving with a `--waive` for a non-unknown or absent gap is `plan-gaps-unresolved` / a typed waive error (name TBD).

**Pins and identity**

- Refine writes `base.yaml` (baseline specs digest + per-source snapshot revisions) before extract.
- Synthesis mints slice-scoped requirement ids; each `MODIFIED` records a digest of the baseline requirement body it changed.
- Merge assigns baseline `REQ-NNN`, records the id map as a merge fact, rejects drifted `MODIFIED` bases.
- Validate gains `slice-base-drifted` / `slice-evidence-stale` (review signals); merge blocks on `merge-base-drifted` where needed.

**Plan refine and execute**

- Guest `plan refine` orchestration claims each selected unrefined in-scope slice and dispatches the existing `slice refine` orchestration (pins, extract, synthesize); default selection is every unrefined in-scope slice.
- Guest `plan execute` drops the refine leg.
- Diagnostics (exit 2): `plan-approval-missing`, `plan-approval-stale`, `plan-gaps-unresolved`, `plan-approval-topology-only`, `plan-waiver-invalid` (waive of non-unknown / unknown id / missing reason), `slice-claim-conflict`, plus staleness / merge-drift codes above.
- New events: `plan.refined` (or per-slice claim + existing refine events), `plan.approved` (unknown-waivers nested on the approval), claim/release, `fact.retracted`, identity-mapped merge.

**Tests**

- Multi-actor fixtures in `crates/mock`: disjoint refine, claim conflict, base drift (shared-git collaboration; not a waiver countersign gate).
- Shift-left fixture: author → `plan refine` → gaps → fix conflicts / waive unknowns → approve → build/merge-only execute; refuse conflict-waive and bulk-waive shapes.
- Multi-homed lead fixture: one `(source, lead)` bound into two refined slices with open unknowns; `plan gaps` groups/annotates both rows and suggests both slice selectors; approve still requires per-req waive or clearance (no lead-wide waive).
- `cargo make ci` green; projection determinism and gap/approval paths covered as crate integration tests.

**Hard cut**

- Pre-1.0: re-init over migration; no shims for old status fields, single journal, global synthesize-time ids, or execute-bundled refine.
