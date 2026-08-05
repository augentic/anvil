# RFC-86: Change Facts

> **Status:** Draft — gap/shift-left, multi-slice claim, in-scope membership, pin/value vocabulary, no-`approve`-verb, and change-home layout decisions closed (D11–D27); [open review questions](#open-questions) below must close before an implementation plan is cut. Sibling-series consistency remains an implementation task (D19), not a product blocker. Change-home layout and the absence of RFC-88 `change approve` follow [RFC-88](rfc-88-detached-changes.md).
>
> **Series:** Step 1 of the [platform-migration series](platform.md) by product ownership — the fact substrate every later step consumes. [RFC-87](rfc-87-working-trees.md) (Private Workspaces) has **already landed** with interim stand-ins for this RFC’s recorded pins and build records; this document is the contract those stand-ins await. Remaining series RFCs (88–92) still depend on this substrate completing.
>
> **Owns:** the fact-based change substrate — projected status, per-actor event logs, claims, execute-implied approval facts, pinned judgment inputs (using the landed RFC-87 snapshot identity), merge-finalized requirement identity, shift-left refine / gap gate on execute (no separate `approve` verb), and the single-actor desktop as the degenerate case of the same substrate.
>
> **Audience:** Operators and contributors who know Emery’s workflow vocabulary (plan, slice, refine, build, merge, sources, specs). Implementation detail is deferred; this RFC must be complete enough to plan from, not to implement from.

## Synopsis

This RFC does two related things:

1. **Shift specs left.** Creating per-slice specs (`refine`) moves out of `plan execute` and into the **plan** phase. Execute becomes **build → merge** only, after the operator has reviewed those specs and dealt with typed gaps. Starting execute is the approval gesture — there is no separate `approve` CLI verb.
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

Build and merge already run through [RFC-87](rfc-87-working-trees.md)’s private workspaces (`prepare` / `capture` / `discard` over content-addressed `SnapshotId`s). What they do **not** yet have is this RFC’s recorded pin and fact substrate — so the landed path uses interim stand-ins: build **self-freezes** the product tree at start instead of reading a recorded base pin, persists the code patch at `.emery/slices/<slice>/build/patch.yaml`, and merge applies that result via an interim `apply` ([RFC-88](rfc-88-detached-changes.md) deletes `apply` in its cut; older prose assigned that to [RFC-89](rfc-89-publication-sets.md)). Coordination state is still the pre–RFC-86 shape: one `.emery/journal.jsonl`, stored plan-entry / slice lifecycle fields, and “running execute is approval” with no durable digest binding.

Problems:

| Problem | In plain terms |
| ------- | -------------- |
| Specs appear mid-execute | Plan review sees leads and `plan.yaml`, not `spec.md`. By the time specs exist, the loop is already heading into build. |
| Gaps do not stop build | Refine already tags `[unknown]`, `[conflict]`, and `[divergence]` on requirements. Those tags are informational only — execute builds anyway. |
| Approval has no durable binding | Running `plan execute` *is* the approval gesture (and stays that way — see D20 / D26), but nothing records *which* specs and gap outcome that gesture covered. |
| Status is hard to share | Progress is stored as fields in YAML files. Two people cannot safely collaborate on one change, and the same pattern will not scale to multiple machines. |
| Requirement IDs collide | Two slices refined against the same baseline can mint the same `REQ-NNN` numbers. Merge can silently overwrite. |
| Pins are not recorded | Refine does not write `base.yaml`. Build freezes an ambient tree at call time. Drift between refine and build is not a typed gate — only an interim freeze that this RFC retires. |

### What we want instead

```text
Plan phase     author slices → plan refine → review gaps (close or waive)
Execute phase  build → merge   (starting execute implies plan approval; gap/status gates enforce readiness)
```

- Operators review **topology** after author (before synthesis spend), then read **real specs** after `plan refine` and check the **gap inventory** before any code generation. Prose quality is human-owned — the engine does not record how spec reading happens (see D17).
- Known gaps are listed and, by default, **block execute**. Slice lifecycle / computed status gates already refuse work that has not been dealt with; this RFC adds the typed gap policy on the same seam.
- There is **no `emery plan approve` (or other `approve`) CLI verb**. Starting `emery plan execute` is enough to imply the plan is approved. The fact substrate records that gesture against current plan/spec digests and any unknown-waivers when execute begins — audit without a second ceremony (see D20 / D26).
- One person on a laptop and a multi-person (later multi-node) change use the **same** rules.

Unchanged: source and target adapters, artifact shapes (`spec.md`, Evidence, etc.), the meaning of refine / build / merge as verbs, and the landed RFC-87 workspace contract (`prepare` / `capture` / `discard`, `SnapshotId`, code patch as `{ base, result, touched paths }`). What changes is **when** refine runs (via `emery plan refine` in the plan phase, not inside execute), **whether** gaps may enter build, **how** progress is stored, and **how** pins / build outcomes are recorded so those workspaces consume durable facts instead of interim self-freeze.

---

## Operator flow

### Plan phase — intent through specs

1. **Author** — `emery plan author` (via `/emery:plan`) surveys bound sources and reconciles leads into slices. Produces `discovery.md`, `change.md`, and `plan.yaml` (which work exists). **Stops here** — no extract or synthesis yet, so the operator can review topology before paying for refine.
2. **Refine** — `emery plan refine` (default: every unrefined in-scope slice — on the plan and not dropped, see D24; optional slice selectors) claims slices and runs the same per-slice refine implementation as today (`emery slice refine`): extract Evidence, synthesize `proposal.md`, `spec.md`, `design.md`, `tasks.md`, and `model.yaml`, and record input pins. Prints or leaves the gap inventory ready for review.
3. **Review** — Read `spec.md` and related slice artifacts, and the **gap inventory** (see below). Deal with typed issues before generation: close conflicts, clear or plan to waive unknowns. Prose quality and acceptance-criteria depth are operator-owned (see D17) — the engine does not score or record how that reading happened. After author, operators **pause** to catch slicing issues before expensive refine; that human review needs no engine gate and no topology-approve verb (see D21).
4. **Iterate** — Fix inputs (richer intent/docs, authority overrides, corrected sources), then re-refine only the affected slices with `emery slice refine <slice>` (or `emery plan refine` over a subset).

### Execute phase — code through merge

5. **Execute** — `emery plan execute` (via `/emery:execute`) **is** the plan-approval gesture. Once every in-scope slice is refined (D24), conflicts are gone, and remaining `[unknown]`s are either cleared (Ready — D22) or explicitly waived per requirement on this command (D16), starting execute implies the plan is approved, records that binding in the fact substrate, and runs **build → merge** per slice. It does **not** extract or synthesize again. There is no prior `approve` verb to call (see D20 / D26). Gap and slice-status gates refuse execute when issues have not been dealt with.
6. **Finalize** — Operator publishes; archive as today.

`/emery:plan` remains an ultrathin wrapper over `plan author` only. Hand-driven breakouts (`emery slice refine` / `build` / `merge`) still work; `plan refine` is the batch fan-out over that same refine implementation. The drained execute loop simply no longer contains refine. The CLI has no `approve` verb on any surface this RFC owns.

One-slice changes stay the same shape, only shorter: author → `plan refine` → review → `plan execute` (build → merge). `plan status`’s resume points at `emery plan refine` when any in-scope slice is still unrefined; at close-gaps or `plan execute --waive…` when refined with open unknowns (not Ready — D22); and at `emery plan execute` once Ready (see D20 / D26).

### Who owns what

| Work | Plan phase | Execute phase |
| ---- | ---------- | ------------- |
| Survey and propose slices (`plan author`) | yes | no |
| Extract and synthesize specs (`plan refine` / `slice refine`) | yes | **no** |
| Review specs (prose) and close typed gaps | yes | no |
| Imply plan approval by starting execute | human review precedes it | yes — `plan execute` is the gesture; gap/status gates enforce readiness (D20 / D26) |
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

Plan authoring can also flag fuzzy *slice* matching (tentative merges, `divergence: likely`). That is about how work is grouped. Requirement gaps appear only after refine — another reason specs must exist before execute.

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

**Gate authority stays per requirement** — each row is still `(slice, req, status)`. When the same lead is multi-homed across slices (coverage is at-least-once; cross-cutting leads in `change.md`), one thin or contradictory lead can surface as several inventory rows. `plan gaps` therefore also offers a **presentation rollup**: annotate or group open findings that share a contributing `(source, lead)`, and suggest the slice-selector set for a follow-up `plan refine` / `slice refine` after the shared input is fixed (see D18). The rollup is navigation only — it does not merge findings, change the execute gap gate, or introduce lead-wide waivers.

### How to close gaps

Do not hand-edit the machine-rendered `ID:` / `Sources:` / `Status:` lines or the `[…]` tags. Change the inputs, then re-refine (same rule as today’s conflict how-to):

| Finding | Typical fix | Next step |
| ------- | ----------- | --------- |
| `[unknown]` | Operator supplies the missing information: enrich a source (intent, docs, captures), or re-scope the lead so the requirement is not invented. Execute stays blocked until the tag clears or that requirement is explicitly waived on `plan execute` (see D16) | Re-refine that slice (and any siblings the rollup lists for the same lead) |
| `[conflict]` | Set an authority override, or remove/correct a misleading source. **Not waiveable** — resolve inputs, then re-refine | Re-refine (per slice that still shows the conflict; overrides remain per-slice) |
| `[divergence]` | Informational — authority already chose; no decision required. Override only if the wrong source won | Re-refine only if inputs changed |
| Stale inputs | Sources or baseline moved since refine pinned them | Re-pin and re-refine |

Prefer fixing one shared input, then re-refining the affected slice set (the rollup’s suggested selectors when findings share a lead). Full-plan re-synthesis is expensive. Waivers stay per-requirement (`--waive <slice>/<req>`) even when rows share a lead.

### Human-only ambiguity (outside the engine)

Vague prose in an otherwise `agreed` requirement (weak scenarios, missing acceptance criteria) stays a **human** review concern. The operator reads `spec.md` / `design.md` by whatever process fits — IDE, PR, checklist on paper, pair review — and decides whether to start execute. The agentic and programmed workflow does not model that step: no checklist artifact, no review attestation on execute, and no spec-quality findings in the gap inventory (see D17). This RFC only machine-gates the typed statuses above.

### When build is allowed (gap policy)

Refine may finish with tags still present — the slice is refined, but not necessarily ready to build.

**`plan execute`** checks a gap policy (alongside existing slice-status / lifecycle gates that refuse undealt work):

| Finding | Policy |
| ------- | ------ |
| `[conflict]` | **Block, not waiveable** — do not generate code over an unresolved contradiction. Resolve via authority override or source correction, then re-refine (see D16) |
| `[unknown]` | **Block** — insufficient information reached the agent; the operator must supply it (richer sources, re-scope) or **explicitly waive that requirement** on `plan execute` before build proceeds. Uniform for every change shape — including intent-only / N=1. Desk-testing shows warn-only yields unpredictable generation that compounds through build and merge (see D14, D16) |
| `[divergence]` | **Allow, but list** — informational only; authority already chose (`intent` > `documentation` > `behaviour`, plus any per-slice override). No acknowledgment or decision required to start execute. Override and re-refine only when the wrong source won (see D15) |

If the policy fails, execute refuses and prints the inventory. The operator may:

- close the findings and run execute normally, or
- **explicitly waive** individual `[unknown]` requirements on the execute command (recorded on the execute-implied approval fact, each with a reason) — never silently, never as a plan-wide or slice-wide off-switch, and never for `[conflict]` (see D16).

There is **no separate `approve` verb**. Starting execute *is* the approval gesture; when the gap/status gates pass, execute records a covering approval-implied fact against current plan/spec digests (and any unknown-waivers) so the binding is auditable and goes stale on re-refine (see D20 / D26). Execute must **not** auto-waive gaps — listing `--waive` is explicit.

Topology handoff after author is human-owned (pause and re-cut the slice list if needed). This RFC adds **no** topology-approve / `--slices` CLI surface (see D21). If someone re-refines after execute has recorded a covering binding, that binding goes stale — including every waiver it carried — and execute must be started again (re-list any remaining unknown-waivers) before further build/merge may proceed under a fresh covering fact.

---

## How state works (the substrate)

Today, “where is this change?” answers are scattered: status fields in YAML, one shared journal file, approval only as an unrecorded gesture. That works for one operator on one machine. It does not travel.

### Three kinds of thing

| Kind | Role | Examples |
| ---- | ---- | -------- |
| **Artifacts** | Durable files that describe the change | `plan.yaml`, per-slice `spec.md` / `design.md` / Evidence, `base.yaml` pins |
| **Facts** | Append-only history of what happened | “slice claimed”, “execute started / plan approved (implied)”, “build succeeded”, “merge completed” |
| **Values** | Content-addressed product-code trees (landed in RFC-87) | `SnapshotId` (`sha256:…` tree digest); a **code patch** is the relation `{ base snapshot, result snapshot, touched paths }` — no separate patch blob |

Nothing else is workflow authority. In particular, **status is never stored as a field to edit**. `plan status` *computes* progress from artifacts + facts. Facts and build records **reference** snapshot identities; they never store workspace paths. Private workspaces remain disposable execution machinery under host-owned storage (`$EMERY_HOME/snapshots/`, `$EMERY_HOME/workspaces/`) — outside the change tree. Approval is **not** a separate on-disk artifact family authored by an `approve` verb — it is the execute-implied fact (digests + gap outcome + waivers) written when execute begins past the gates (see D20 / D26).

### Rough layout of a change

Illustrative coordination layout (D27 — layout-neutral; concrete homes from [RFC-88](rfc-88-detached-changes.md)):

```text
<change>/                   # in-place: .emery/change/ ; detached: change-dir root
  change.md
  plan.yaml                 # what slices exist — not their status
  events/<person-or-node>.jsonl   # each actor appends only their own log
                              # (includes execute-implied approval facts)
  slices/<slice>/
    base.yaml               # refine-time pin assembly (sources + baseline)
    evidence/…
    spec.md, design.md, tasks.md, model.yaml
    build/                  # today (RFC-87 interim): patch.yaml + report.yaml
                            # this RFC re-homes into fact-substrate build records
```

Durable project state (`project.yaml`, baseline `specs/`, `decisions/`) stays outside the change home. Sharing a change is ordinary file/git exchange of that home (push / pull / PR when the operator versions it). Two people’s event logs merge without fighting over one journal file.

### Progress, computed

| Milestone | Meaning |
| --------- | ------- |
| Authored | Plan has slices |
| Refined (per slice) | Validated specs exist for that slice, pinned to known inputs (`plan refine` or `slice refine`) |
| Ready | Every in-scope slice is refined, and the **clean** gap policy passes: no conflicts, **zero** open unknowns. Artifact-derived only — waivers are not part of Ready (see D22). In-scope is on the plan and not dropped (see D24) |
| Approved | A covering execute-implied approval fact exists for the current plan and specs (may carry unknown-waivers). Distinct from Ready even when the waive list is empty — Ready means “may start execute cleanly”; Approved means “execute has begun under a covering binding” (D20 / D26) |
| Built / merged | Build and merge facts exist for that slice (referencing RFC-87 base/result snapshot ids; today the interim signal is `build/patch.yaml`) |

`plan status` next actions follow the phase split: `plan refine` / `slice refine` / review gaps in plan phase; then `plan execute` (build → merge) once gates pass. Open unknowns mean the change is **not** Ready; resume points at closing gaps *or* `plan execute` with per-requirement `--waive` (skipping Ready). Clean Ready resumes at `emery plan execute`. There is no resume point at a separate `approve` verb.

### Pins and requirement IDs (why implementers care)

Two rules make multi-slice and multi-person planning safe:

1. **Pins** — Judgment legs run against recorded [RFC-87](rfc-87-working-trees.md) `SnapshotId`s (and baseline-spec digests), not ambient trees. Pin *authorship* closes when inputs are knowable (see D4 / D25): source snapshot ids at plan authoring (in-place) or at detached discovery / plan-author intake ([RFC-88](rfc-88-detached-changes.md) — no `change approve` verb); refine assembles each slice’s `base.yaml` from those source pins plus the baseline digest before extract; build reads a recorded target-base pin before `prepare` (retiring today’s self-freeze). If pins move later, validate reports staleness instead of silently building on drift.
2. **Local then global IDs** — Each slice uses its own requirement ids while planning. Merge assigns final baseline `REQ-NNN` numbers and records the mapping. Two slices can no longer collide by minting the same id at refine time. (Today synthesis still allocates global `REQ-NNN` against the baseline via `IdAllocator` — Phase B replaces that.)

### Desktop = simplest deployment

One operator, one machine, no remote: same artifacts, same facts, same commands. Multi-person and later multi-node add transport, not a second lifecycle. When two people share a change over git, each may claim a **different** slice and refine at the same time; a slice still has only one owner (D23).

---

## Decisions (summary)

| # | Decision | Operator-visible effect |
| - | -------- | ----------------------- |
| D1 | Change is a self-contained fact tree (layout-neutral home; D27) | Planning can last days; coordination artifacts travel as ordinary files. Versioning / clone / PR of the change home is operator policy, not a workflow prerequisite ([RFC-88](rfc-88-detached-changes.md) amends any prior “must be a git repo” reading) |
| D2 | Status is computed, not stored | `plan status` is the only progress view; no hand-edited status fields |
| D3 | Per-person (or per-node) event logs | Collaboration and later multi-machine sync without one contested journal file |
| D4 | Every refine/build pins its inputs (RFC-87 `SnapshotId` vocabulary) | A reviewed spec is tied to what it was made from; drift is detected. Source pins close when the source set closes; refine assembles `base.yaml`; build reads a recorded base — never ambient self-freeze (see D25) |
| D5 | Requirement ids finalized at merge | Parallel refine of different slices against one baseline is safe |
| D6 | Approval is implied by starting execute; recorded as a fact | Auditable “who started execute against which digests”; no separate `approve` artifact tree or verb |
| D7 | Work is claimed in the log | A slice has at most one owner at a time; two people do not unknowingly work the same slice (see D23) |
| D8 | Phases consume pinned inputs only | Retry after failure loses no completed work |
| D9 | One lifecycle everywhere | Laptop and fleet differ only by transport config |
| D10 | Hard cut (pre-1.0) | No compatibility shims for old status fields or execute-bundled refine |
| D11 | **Plan owns refine; execute owns build/merge** | Specs are reviewed before generation spend |
| D12 | **Gaps gate execute, not refine success** | Incomplete Evidence can still refine; it cannot silently enter build |
| D13 | **`emery plan refine` is the plan-phase batch; `/emery:plan` stops after author** | Topology review before synthesis cost; named batch for N-many; `slice refine` stays the per-slice implementation and gap-closure breakout |
| D14 | **`[unknown]` always blocks execute** | Thin intent is not an exception — close the gap or waive it explicitly on execute; generation must not invent missing information |
| D15 | **`[divergence]` is informational; listed but allowed** | Authority hierarchy already picked a winner; execute does not require per-divergence acknowledgment. Wrong winner → override / amend sources and re-refine |
| D16 | **Waiver UX: per-`[unknown]` on execute; `[conflict]` never waiveable; no multi-operator gate** | Repeatable `--waive <slice>/<req>` + required `--reason` on `plan execute`; one operator’s execute start is enough; re-refine clears the execute-implied approval fact and its waivers |
| D17 | **Human prose review stays outside the engine** | Operators own spec quality; execute’s gap gate covers typed statuses only. No checklist artifact, review attestation, or spec-quality rollup in `plan gaps` / execute |
| D18 | **Shared-lead gap rollup is presentation only** | `plan gaps` annotates/groups open findings that share a contributing `(source, lead)` and suggests re-refine selectors; execute/waive stay per-requirement. No lead-wide waive, no shared-Evidence extract, no lead-level gate |
| D19 | **Sibling-series consistency is an implementation task** | When implementing this RFC (especially Phase C), review `platform.md` and remaining unimplemented series RFCs (RFC-88…RFC-92; RFC-87 is already Implemented) for prose that still assumes execute-bundled refine, a mandatory `plan approve` verb, or interactive execute auto-*waive*. Do not litigate or cascade-rewrite those docs as part of freezing this RFC; later RFCs may further review this work and roll back or refactor anything that becomes obsolete. Known drift is listed under [open questions](#open-questions) |
| D20 | **No `approve` CLI verb; starting execute implies plan approval** | `emery plan execute` / `/emery:execute` is the approval gesture after plan→refine review. Gap and slice-status gates refuse undealt issues. When gates pass, execute records a covering approval-implied fact (digests + gap outcome + any unknown-waivers). Rejected: a mandatory `emery plan approve` (or any other `approve`) verb before execute — extra ceremony with no job once human review and status/gap gates exist |
| D21 | **No topology-approve CLI surface** | Human pause after author is the topology review seam. No `plan approve --slices` (or other topology-approve) verb — handoff is social/git. Nothing machine-gates refine on a topology approval |
| D22 | **Ready is clean-gap only; waivers live only under Approved** | Ready = all in-scope refined + no conflicts + zero open unknowns (no waiver contribution). Approved = covering execute-implied approval fact (possibly with unknown-waivers). Waiver path skips Ready; never make Ready depend on an approval fact that does not exist yet |
| D23 | **Many slices in flight; one actor per slice** | Different slices may be claimed and progressed by separate actors at the same time. A slice is an exclusive unit of work — never two actors on one slice. Plan-wide “at most one `in-progress` entry” is retired. Swarming *inside* one slice stays a non-goal |
| D24 | **In-scope = on the plan and not dropped** | One shared membership predicate for default `plan refine`, `plan gaps`, Ready, and the execute gap gate. `plan remove` deletes the entry; `slice drop` abandons it and excludes it from in-scope. Optional refine selectors narrow the batch only — they do not redefine membership |
| D25 | **Pins use landed RFC-87 snapshot identity; this RFC authors them** | Pin *wire identity* is RFC-87’s `SnapshotId` (`sha256:…`). This RFC owns *when pins are written and what consumes them*: source snapshot ids close when the source set closes — at plan authoring (in-place) or at detached discovery / plan-author intake ([RFC-88](rfc-88-detached-changes.md); there is no `change approve` pin-close site); refine copies those pins into `base.yaml` and adds the baseline-spec digest before extract; build reads the recorded target-base pin before `prepare` (replacing the interim freeze-at-build stand-in). Exact on-disk `base.yaml` shape stays an implementation detail. Phase B may ship against today’s trees using that vocabulary — it does **not** wait on further RFC-87 work |
| D26 | **Remove / never ship an Emery CLI `approve` verb for plan build** | Operators review after refine and deal with issues; simply starting execute implies approval. Internal slice-status and gap gates prevent undealt work from entering build. Any prior draft of this RFC that introduced `emery plan approve` is superseded |
| D27 | **Layout-neutral change home; concrete trees from RFC-88** | Coordination artifacts (plan, events, slice specs / pins / build records, execute-implied approval facts) live with the **change**, not mixed into durable project state. Same logical tree in both modes: in-place home is `.emery/change/` beside durable `.emery/` project state (`project.yaml`, `specs/`, `decisions/`); detached home is the change-directory root ([RFC-88](rfc-88-detached-changes.md) D1). Private workspaces / snapshot objects stay under host-owned `$EMERY_HOME` (RFC-87). Phase A scopes against this contract; today’s flat `.emery/` + root `plan.yaml` layout is a pre-cut stand-in until the two-root cut lands |

---

## Commands that change

| Command | Change |
| ------- | ------ |
| `emery plan approve` (any `approve` plan verb) | **Removed / never shipped.** Not part of the CLI surface. Starting `plan execute` implies plan approval; gap and slice-status gates enforce readiness (see D20 / D26). No `--slices` topology-approve substitute (D21) |
| `emery plan refine` | **New.** Plan-phase batch: claims and refines every unrefined in-scope slice by default (on the plan and not dropped — D24); optional slice selectors for a subset. Fans out to the same orchestration as `emery slice refine` (pins, extract, synthesize). Claims are exclusive per slice; other slices may already be claimed by other actors (D23). Does not build |
| `emery plan gaps` (name TBD) | **New.** Shows the typed-status gap inventory (not spec-quality advisories — see D17). When open findings share a contributing `(source, lead)`, annotates or groups those rows and suggests the slice-selector set for re-refine — presentation only; gate and waivers stay per-requirement (see D18) |
| `emery plan execute` | **Is** the plan-approval gesture (D20 / D26). Enforces the gap policy and slice-status gates; on success records a covering approval-implied fact (digests + gap outcome + any unknown-waivers); runs **build → merge only**; never refines. For `[unknown]` leftovers only: repeatable `--waive <slice>/<req>` with required `--reason` (see D16). No `--force`, no bulk/all-gaps waive, no separate `plan waive` / `approve` verb |
| `emery slice refine` | Still the refine implementation and per-slice breakout (gap closure, single-slice re-refine); records input pins; used in plan phase |
| `/emery:plan` | Unchanged contract: elicit → `emery plan author` → relay; stops after topology. Does **not** run refine |
| `emery plan status` | Next actions include `plan refine` / `slice refine` / review-gaps, then `plan execute` (build / merge); resume points at `emery plan refine` while any in-scope slice is unrefined (D24), at close-gaps *or* `plan execute --waive…` when refined with open unknowns (not Ready — D22), and at `emery plan execute` when Ready (D20 / D26) |
| `emery plan advance` / `undo` | Expressed as claim / retraction facts instead of rewriting status fields; no plan-wide single-active-entry (D23) |

Exact error codes and event names belong in the [implementation notes](#appendix-implementation-notes); product behavior is above.

---

## Delivery

| Phase | Delivers | Operator should notice |
| ----- | -------- | ---------------------- |
| **A** | Fact logs + computed status + exclusive per-slice claims (D23) | Same day-to-day flow for one operator; status still looks familiar; two actors may refine different slices without waiting on each other |
| **B** | Recorded pins (D4 / D25) + merge-time requirement ids; replace interim freeze-at-build with a recorded pin read (depth of `build/patch.yaml` re-home — [open question 2](#open-questions)) | Safer parallel refine; drift diagnostics; build prepares from a recorded base pin instead of freezing the ambient tree |
| **C** | `plan refine`, gap inventory (+ shared-lead presentation rollup), execute gap gate (no `approve` verb), execute without refine; sibling-series consistency pass (D19) | The new rhythm: author → `plan refine` → gaps → `plan execute` (build/merge only; starting execute implies approval); multi-homed leads correlate in `plan gaps` without changing the per-req gate. Implementation also reviews `platform.md` / RFC-88…RFC-92 for stale execute-bundled-refine or mandatory-`plan-approve` assumptions — without blocking on a full cascade rewrite here |

Close the [open review questions](#open-questions) before cutting an implementation plan. Phase delivery above is a suggested split, not a locked schedule. Include the D19 sibling-review task in Phase C plan work.

---

## Acceptance (product-level)

1. Progress reported by the CLI is always computed from artifacts and facts — never read from a stored status field.
2. **Multi-slice, multi-actor (D23):** two people can claim and refine *different* slices on copies of one change *at the same time*, merge via git, and both slices show as refined without journal conflicts. The same slice cannot be claimed by two actors (`slice-claim-conflict`). Plan-wide single-active-entry is gone — work does not wait for one slice to finish before another may start.
3. Two slices refined against the same baseline merge without requirement-id collision; a drifted modification is rejected instead of overwritten.
4. **Pins (D25):** refine records `base.yaml` against RFC-87 `SnapshotId`s (and baseline-spec digest); build prepares from a recorded base pin rather than freezing the ambient product tree; validate detects pin drift. (Depth of `build/patch.yaml` re-home — open question 2.)
5. **Shift-left:** after authoring, every slice is refined via `emery plan refine` (or per-slice `emery slice refine`) before any build; execute performs build and merge only. `/emery:plan` does not run refine.
6. **Gap gate:** `[unknown]` prevents execute until fixed or explicitly waived per requirement on `plan execute` (including intent-only / N=1); `[conflict]` prevents execute until resolved via override/sources and re-refine — **not** waiveable; `[divergence]` is listed but does not block (authority already chose); execute never silent-waives. Slice-status gates continue to refuse undealt work.
7. **No topology-approve verb:** refine is not gated on any slice-list / topology approval; post-author topology review is human-owned only (see D21).
8. **No `approve` CLI verb (D20 / D26):** every execute path (CLI and `/emery:execute`) *is* the plan-approval gesture. When gap/status gates pass, execute records a covering approval-implied fact (digests + gap outcome + any unknown-waivers). Re-refine after that fact forces a fresh execute start (waivers on the stale fact do not carry forward). The CLI ships no `emery plan approve` (or other plan `approve`) verb.
9. The same verbs and artifacts work with no remote (solo laptop) and with the change shared over git (two people).
10. **Human review boundary:** execute’s gap gate enforces typed statuses only; prose review of `spec.md` is operator-owned and leaves no engine artifact (see D17).
11. **Shared-lead rollup:** when open findings share a contributing `(source, lead)`, `plan gaps` annotates or groups those rows and suggests re-refine selectors; execute still fails or succeeds per requirement, and waivers remain `--waive <slice>/<req>` only (see D18).
12. **Ready vs Approved:** `plan status` projects Ready only when every in-scope slice is refined and the clean gap policy passes (no conflicts; zero open unknowns). Open unknowns keep the change out of Ready; starting execute with unknown-waivers reaches Approved without ever being Ready. Waivers never contribute to the Ready projection (see D22).
13. **In-scope membership (D24):** default `plan refine`, `plan gaps`, Ready, and the execute gap gate all use the same filter — every entry currently on the plan whose slice is not dropped. Dropping a slice excludes it from that set without a second `plan remove`; removed entries are simply absent from the plan. Optional refine selectors do not redefine membership.

---

## Open questions

Close the **open** items before cutting an implementation plan. They are product / contract questions — not engine design. Closed items below are the decision trail for D13–D27. Layout and `change approve` closures follow [RFC-88](rfc-88-detached-changes.md).

### Open (close before planning)

1. **Post-author resume.** `/emery:plan` stays an author-only wrapper (D13), but today’s author/status hints point operators at `emery plan execute` next. Decide the operator-visible next step after a successful author: always `emery plan refine` (then gaps → execute), and whether `plan status` / author epilogues must say so in this RFC’s acceptance surface. Skill-body wording can follow; the contract question is the resume point. (Starting execute remains the plan-approval gesture after refine — see D20 / D26. No topology-approve verb after author — see D21. RFC-88 folds discovery into detached `plan author` and still has no post-author topology-approve verb — it does not settle this shift-left resume point.)

2. **How completely must Phase B retire RFC-87 interim stand-ins?** Landed code freezes the product tree at build start and writes `.emery/slices/<slice>/build/patch.yaml`. D25 says build reads a recorded pin instead of freezing. Decide the acceptance bar for this RFC: (a) Phase B **must** replace freeze-at-build with a recorded pin read **and** re-home `build/patch.yaml` into fact-substrate build records, or (b) pin authorship + identity finalization can land while leaving the patch.yaml → build-record re-home as follow-on work tracked under D19. Affects Phase B scope and whether “built” is projected from facts or from the interim file. **Out of this question:** interim `apply` retirement — [RFC-88](rfc-88-detached-changes.md) deletes `apply` in its own cut (and amends the older “RFC-89 deletes apply” story); do not scope `apply` deletion into this RFC’s Phase B.

3. **Sibling prose on execute / approval (series / platform docs).** `[platform.md](platform.md)` still sequences “complete RFC-86 first” then RFC-87, and its operator loop still says execute is “approval-gated (auto-approves interactively).” This RFC’s D20 / D26 keep “starting execute implies approval” but **forbid** a separate `approve` verb and **forbid** silent auto-waive of gaps. Per D19 this document does **not** rewrite those siblings — but the implementation plan needs an explicit answer: treat D20 / D26 as the in-force product rule and file sibling edits as ordinary delivery (preferred), clarifying that “auto-approve” means the execute gesture itself (with gap gates), not auto-waive or a hidden second mint path. Also note RFC-87 landed out of the spine’s “86 then 87” completion order with stand-ins; `platform.md` / roadmap sequencing prose should be refreshed in a later pass so contributors do not re-litigate whether private workspaces “depend on completed 86.” **Settled against [RFC-88](rfc-88-detached-changes.md):** that RFC’s review (R5) and rewrite make `plan execute` the only authorization surface, drop separate discovery/topology-approve verbs, and align with execute-implied approval facts — no further naming collision with a plan `approve` verb. Remaining drift to file under D19 is spine/platform, not RFC-88 itself.

### Closed — decision trail

1. ~~**What “in-scope” means for default `plan refine` / Ready / execute gap gate**~~ **Closed — D24.** **In-scope** means every entry currently on the plan whose slice is **not dropped**. That single predicate feeds default `plan refine`, `plan gaps`, Ready, and the execute gap gate — status, gaps, and execute must not invent divergent filters. `plan remove` deletes the entry (absent from the plan, so not in-scope). `slice drop` abandons the slice and excludes it from in-scope even if the plan row remains for audit — drop alone can shrink the change toward Ready/execute without a second remove. Optional refine selectors only narrow the batch; they do not redefine membership. Merged is not a separate in-scope exclusion for these plan-phase gates (execute’s covering fact binds digests at execute-start time). Rejected: every plan entry including dropped (drop cannot unblock Ready/execute without a second `plan remove` / amend); excluding already-merged from the plan-phase membership noun (smuggles execute into Ready); a stored scope / include-exclude list on the plan (editable membership next to computed status — fights D2); claim- or last-selector-derived scope (unclaimed unrefined slices would fall out of Ready and create false Ready / execute-over-incomplete-plan); phase-divergent filters (the failure mode this question exists to prevent); a soft “deferred / parked” exit ramp that is neither drop nor remove (third lifecycle noun without a clear job).
2. ~~**Concurrent refine claims / single-active-entry**~~ **Closed — D23.** A slice is a coherent, independently workable scope and is owned by **at most one actor** at a time. **Different** slices may be claimed and progressed by **separate actors at the same time** — refine does not wait for one slice to finish before another may start. Plan-wide “at most one `in-progress` entry” is **retired**; exclusivity is per slice via claim facts (D7), not per plan. Same-slice overlap fails closed (`slice-claim-conflict`). Parallel *swarm* work *inside* one slice remains a non-goal (later concurrency RFCs). This RFC’s claim model is the product rule for per-slice work generally; a single `plan execute` process may still walk entries one-by-one, but that must not reimpose a plan-wide single-active gate that blocks another actor on a different slice. Rejected: keep single-active-entry for the whole plan (denies slice independence; forces multi-person work to serialize); multi-person refine only via social convention / non-overlapping selectors without engine claims (Acceptance #2 becomes unenforceable; agents invent ad-hoc fan-out); defer multi-slice multi-actor to RFC-91/92 (leaves D7 and per-actor logs without the cardinality rule they exist for).
3. ~~**How does plan-phase refine start?**~~ **Closed — D13.** `/emery:plan` / `emery plan author` stop after topology. Specs are minted by the new `emery plan refine` (batch over unrefined in-scope slices; optional selectors). Per-slice gap closure and re-refine use `emery slice refine`. Rejected: folding refine into `plan author` (pays synthesis before topology review; blurs the two review seams); status-driven `slice refine` only (no named batch — poor N-many ergonomics; agents invent ad-hoc fan-out outside the CLI contract).
4. ~~**Should `[unknown]` block by default?**~~ **Closed — D14.** Always block. `[unknown]` means insufficient information was available to the agent; the operator must provide it (or explicitly waive on execute) before build. Rejected: warn-only for intent-only / N=1 (desk-testing — unpredictable generation that compounds through later phases); context-sensitive defaults keyed on source count or change shape (two policies to teach; under-protects thin multi-slice intent).
5. ~~**Must each `[divergence]` be acknowledged?**~~ **Closed — D15.** Listed but allowed; informational only — no acknowledgment or decision required. The kernel already applied the authority hierarchy (`intent` > `documentation` > `behaviour`, plus any per-slice override) and wrote the winner as the operative body. Rejected: require per-divergence ack before execute (ceremony over a resolved disagreement; rubber-stamp risk; conflates divergence with conflict/unknown). Authority rules may be tightened later if wrong winners prove costly in practice; that is a hierarchy/override change, not a gate change.
6. ~~**Waiver UX**~~ **Closed — D16.** `emery plan execute` accepts repeatable `--waive <slice>/<req>` with required `--reason`. Only `[unknown]` may be waived; `[conflict]` is not waiveable (authority override or source fix, then re-refine). Waivers nest on the execute-implied approval fact only — no separate `plan waive` or `plan approve` verb, no plan-/slice-wide or inventory-digest waive, no `--force` / `--allow-gaps`. One operator’s execute start is enough; this RFC does **not** require a second-person countersign (multi-person four-eyes is a non-goal). Re-refine invalidates the covering fact and every waiver it carried; remaining unknowns must be re-listed on the next execute. Rejected: bulk/all-gaps waive (rubber-stamp / agent `--force` path); waiving `[conflict]` (unresolved contradiction must be decided in inputs, not papered over); separate `plan waive` then execute (extra verb and limbo without a countersign need); multi-operator waiver gating (second lifecycle / mode bit; solo laptop is the primary deployment).
7. ~~**Human-only ambiguity**~~ **Closed — D17.** Prose review of `spec.md` (and related artifacts) alone — human operators own spec quality and how they choose to review; the engine does not record or gate on that process. Rejected: optional operator checklist artifact (extra artifact, stale-on-re-refine binding, rubber-stamp risk, overlaps git/PR review without improving gap policy); rolling advisory `kind: review` spec-quality heuristics into `plan gaps` or execute (blurs the typed-status boundary; waiver creep); `--reviewed` / review attestation on execute (ceremony without substance); model-assisted spec-quality gate at execute time (non-goal for this RFC — eval / later concurrency work).
8. ~~**Shared leads across slices**~~ **Closed — D18.** Flat per-requirement inventory remains the gate authority. `plan gaps` adds a **presentation rollup**: when open findings share a contributing `(source, lead)` (multi-homed / cross-cutting leads), annotate or group those rows and suggest the slice-selector set for re-refine after the shared input is fixed. Rejected: flat list only with no correlation aid (operators and agents invent sibling fan-out outside the CLI; N-row noise from one lead); lead-wide or `--waive-lead` sugar (same rubber-stamp risk as bulk waive in D16; same lead ≠ same gap after per-slice extract); first-class lead-level gate or status noun (second checklist-like surface; derived “lead status” can lie when sibling Evidence diverges); shared extract / shared Evidence for multi-homed leads (changes the per-slice extract contract; deferred — not a gap-inventory decision).
9. ~~**Sibling docs**~~ **Closed — D19.** Consistency with `platform.md` and later series RFCs is an **implementation task** for this RFC (especially Phase C): review them for stale “execute runs `refine → build → merge`” / mandatory `plan approve` / interactive auto-*waive* assumptions. RFC-87 is already Implemented — the remaining unimplemented consumers are RFC-88…RFC-92 plus the spine docs. This RFC does **not** cascade-rewrite those documents at decision-freeze time; known contradictions are tracked as open question 3 (spine/platform; RFC-88 itself is aligned — see closed items 15–16). Later RFCs remain free to review the landed shape and roll back or refactor anything that becomes obsolete. Rejected: freezing Phase C product design on a full sibling rewrite now; treating all series-doc drift as a blocker to freezing this RFC’s product decisions.
10. ~~**Is a separate `plan approve` required in the happy path?**~~ **Closed — D20 / D26.** **No.** Starting `plan execute` / `/emery:execute` implies plan approval after the operator has reviewed plan→refine and dealt with issues. Gap and slice-status gates refuse undealt work; when they pass, execute records a covering approval-implied fact (digests + gap outcome + any unknown-waivers) for audit and stale-on-re-refine. Resume after refine/gaps is `plan execute` (with `--waive…` when needed). Rejected: a mandatory `emery plan approve` (or any other plan `approve`) verb before execute — extra ceremony with no job once human review and status/gap gates exist; agents and operators would rubber-stamp it. Auto-waive on execute remains rejected (D16). Prior draft text that required `plan approve` and forbade execute minting approval is superseded.
11. ~~**Topology-only approval: command shape and consumers**~~ **Closed — D21.** **No topology-approve CLI surface.** Human pause after author is the primary topology review seam — catch slicing issues before expensive refine — and needs no engine gate and no recorded `--slices` / topology-approve verb. Handoff is social/git. Rejected: hard-gating `plan refine` on topology approval (second mandatory ceremony; fights solo-laptop / N=1); soft warn-only refine gate (noise agents ignore); optional `plan approve --slices` as a plan-phase recording surface (extra verb next to a CLI that deliberately has no `approve`); a separate topology verb for the same reason.
12. ~~**Ready vs Approved when waivers exist**~~ **Closed — D22.** Ready means every in-scope slice is refined and the **clean** gap policy passes (no conflicts; zero open unknowns) — computed from artifacts only. Approved means a covering execute-implied approval fact exists and may carry unknown-waivers. The waiver path therefore **skips** Ready (Refined + open unknowns → `plan execute --waive…` → Approved); it never backfills Ready from waivers on that fact. Clean Ready resumes at `plan execute` with an empty waive list (D20 / D26). Rejected: Ready includes “cleared or waived” (circular — waivers nest only on the execute-implied fact, so Ready would depend on Approved); Ready = “executable” with unknowns still open (weakens the noun; needs a second clean-gap signal anyway); drop Ready as a milestone (throws away the clean-path execute resume); dual Ready labels / Ready depending on a prior approval fact (extra vocabulary or collapsed milestones).
13. ~~**Pins before RFC-87 values**~~ **Closed — D25.** RFC-87 has landed: `SnapshotId`, `CodePatch`, and `prepare` / `capture` / `discard` are the value vocabulary. Pin *semantics* for this RFC are recorded snapshot (and baseline-spec digest) identities with typed drift detection — not a second value format. Phase B authors and consumes those pins; it does not wait on further RFC-87 design. The remaining stand-in-retirement depth (how far Phase B must go beyond “read recorded pin” / whether `build/patch.yaml` must re-home in the same phase) is open question 2. Rejected: inventing a pre-snapshot pin representation now that RFC-87 shipped; blocking Phase B on a detached `change approve` as the only pin-close site (in-place plan authoring still closes source pins; detached closes them at discovery / plan-author intake per [RFC-88](rfc-88-detached-changes.md)); treating the interim freeze-at-build as permanent.
14. ~~**Keep or remove an Emery CLI `approve` verb for plan build?**~~ **Closed — D26.** **Remove / never ship.** The operator reviews the plan→refine step and deals with issues; simply starting execute is enough to imply the plan is approved. Internal slice-status and gap gates already prevent undealt issues from entering build. Rejected: retaining `emery plan approve` as a mandatory or optional happy-path verb (ceremony without substance once gates and human review exist).
15. ~~**Change home for this RFC vs RFC-88**~~ **Closed — D27.** Layout-neutral contract: coordination artifacts live with the change; durable project state does not. Concrete homes from [RFC-88](rfc-88-detached-changes.md): in-place `.emery/change/`; detached change-directory root; two roots (project + change) rather than one layout with two mappings. Private workspaces / snapshots stay under `$EMERY_HOME` (RFC-87). Phase A scopes against this contract; today’s flat `.emery/` + root `plan.yaml` is a pre-cut stand-in. Rejected: adapting facts only into today’s flat in-place layout as the enduring home (fights the durable-vs-change lifetime split RFC-88 needs); requiring the detached repository layout before Phase A can start (blocks the substrate behind a later series step); leaving the concrete tree entirely undecided (Phase A cannot pick writers).
16. ~~**RFC-88 `change approve` naming vs this RFC’s no-`approve` plan surface**~~ **Closed — superseded by [RFC-88](rfc-88-detached-changes.md).** That RFC removes `emery change approve` (and `emery change open` / standalone discover): discovery and topology recording are the first internal phase of detached `plan author`; `plan execute` is the only authorization surface and records the RFC-86 execute-implied approval fact (extended with the candidate subject when detached). There is no second `approve` verb to confuse with this RFC’s deliberate absence of plan `approve` (D20 / D26). Pin close for detached sources is discovery / plan-author intake, not a change-approve gate (D25). Rejected: keeping a plan `approve` verb here to “match” a change-approve surface that RFC-88 no longer has; inventing a rename/cross-link exercise for a verb the sibling deletes.

---

## Non-goals

- Changing Evidence schemas or the authority ranking (`intent` > `documentation` > `behaviour`).
- Automatically judging whether an `agreed` requirement is *good* (scenario depth, usefulness) — that stays human review and eval; no checklist artifact or execute-time attestation for prose review (see D17).
- Parallel swarm *inside* one slice (later concurrency RFC) — this RFC makes multi-slice, multi-actor work safe via exclusive per-slice claims (D23); it does not fan out multiple writers within a single slice.
- Multi-operator waiver / approval countersign — one actor’s execute start (with any unknown-waivers) is sufficient; shared-git collaboration stays social review of the fact log, not an engine four-eyes gate.
- Lead-wide waive, lead-level execute gate, or shared Evidence for multi-homed leads — correlation of shared-lead gaps is presentation-only in `plan gaps` (see D18); extract stays per-slice.
- Cascade-rewriting `platform.md` and later series RFCs as part of freezing this RFC — sibling consistency is a Phase C implementation review task; later RFCs may further adjust obsolete assumptions (see D19). Known spine/platform contradictions (interactive auto-waive wording; “86 then 87” completion order after RFC-87 landed; leftover `plan approve` prose) remain open question 3, not silent product flips. RFC-88 is already aligned on execute-only authorization.
- A separate Emery CLI `approve` verb for plan/build (mandatory or optional) — starting execute implies approval; gap/status gates enforce readiness (see D20 / D26).
- Silent auto-waive of gaps when execute starts — waivers stay explicit `--waive` on execute (see D16).
- Redefining the RFC-87 workspace contract, inventing a second snapshot identity, or owning project seal — those stay RFC-87 / RFC-89. Interim `apply` retirement is claimed by [RFC-88](rfc-88-detached-changes.md), not by this RFC. This RFC records pins and result facts those seams consume.
- Introducing a plan topology-approve verb, or reintroducing an RFC-88 `change approve` / open / discover command group — rejected by D21 and by [RFC-88](rfc-88-detached-changes.md) (closed item 16); this RFC does not mint a substitute.
- Machine-gating refine on slice-list / topology approval — the post-author pause is human-owned (see D21).
- Ready that includes waived unknowns, or Ready that depends on a covering approval fact — Ready stays clean-gap / artifact-only; waivers nest only under Approved (see D22).
- Counting dropped slices as in-scope, or inventing a stored scope / park flag / claim-derived membership set — in-scope stays on the plan and not dropped, shared by refine / gaps / Ready / execute (see D24).
- Mixing durable project state into the change home, or treating today’s flat `.emery/` + root `plan.yaml` layout as the enduring coordination home — fights D27 / RFC-88’s two-root split.

---

## Appendix: Prior art (short)

Settled patterns this RFC borrows, without adopting their full machinery:

- **Append-only operations, derived snapshots** (git-bug / Radicle COBs) — progress is replayed, not edited in place. We detect conflicting claims rather than CRDT-merging one slice.
- **Stable identity vs content identity** (Jujutsu) — slice-local requirement ids vs baseline numbers at merge.
- **Content-addressed work** (Bazel Remote Execution) — phases named by input digests (here: RFC-87 `SnapshotId`s); we record judgment outcomes instead of caching non-deterministic generations.
- **Approval as a statement over digests** (in-toto / SLSA) — without cryptographic envelopes in this cut.
- **Spec review before implementation** — open issues tracked against the baseline, not discovered only while coding.

---

## Appendix: Rejected alternatives

- Hosted database as status authority — forces a server and a second mode for the laptop.
- Keep mutable status and synchronize it — harder than computing status from facts; creates two lifecycles.
- Keep refine inside execute with an “optional” pre-pass — optional review is what busy runs already skip.
- Fail refine on any gap tag — blocks useful incomplete Evidence; the execute gap gate is the right seam.
- Auto-waive gaps when execute is invoked interactively — recreates invisible skip of the failures we care about (see D16).
- A mandatory or optional `emery plan approve` (or other plan `approve`) verb before execute — extra ceremony; operator review after refine plus status/gap gates already cover the job; starting execute is the approval gesture (see D20 / D26).
- Global requirement numbering at synthesize time — couples slices exactly when independence matters.
- Custom git merge driver for one journal file — brittle vs per-actor logs that union naturally.
- Fold refine into `plan author` / `/emery:plan` — spends extract/synthesis before the operator can re-cut the slice list; collapses topology review and spec review (see D13).
- Status-driven `emery slice refine` only (no `plan refine`) — preserves a thin CLI but forces N-many operators and agents to invent batching; the drained refine fan-out belongs in one named plan verb (see D13).
- Warn-only `[unknown]` for intent-only / N=1 (or any context-sensitive soften) — desk-testing shows generation invents missing detail and the error compounds through build and merge; thin intent closes gaps by enriching sources or waiving, not by skipping the gate (see D14).
- Require per-`[divergence]` acknowledgment before execute — taxes a disagreement the authority hierarchy already resolved; invites rubber-stamping; blurs divergence (winner chosen) with conflict/unknown (no winner / incomplete). List in the gap inventory; fix wrong winners via override and re-refine (see D15).
- Plan-/slice-wide, inventory-digest, or `--force` / `--allow-gaps` waive — recreates invisible skip as a one-flag off-switch; agents will prefer it over closing gaps (see D16).
- Waive `[conflict]` on execute — papers over an unresolved contradiction; the operator must pick a winner via authority override or correct sources, then re-refine (see D16).
- Separate `emery plan waive` verb before execute — extra noun and waived-but-not-executed limbo once countersign is a non-goal; nest waivers on `plan execute` instead (see D16).
- Multi-operator countersign on waivers or execute start — second lifecycle / mode bit; solo laptop is the primary deployment; collaboration remains git + review of the fact log (see D16).
- Operator checklist artifact for spec review — extra durable file, digest/stale rules, and checkbox theater; prose review stays outside the engine (see D17).
- Review attestation flags on execute (`--reviewed`, reviewer id) — records ceremony, not quality; same boundary as D17.
- Spec-quality advisories in `plan gaps` or execute-time blocking on `kind: review` findings — conflates human judgment with typed gap policy; heuristics belong in eval, not Phase C gates (see D17).
- Flat gap list with no shared-lead correlation — forces operators and agents to rediscover multi-home fan-out from `change.md` alone; presentation rollup is cheap and keeps the CLI contract complete (see D18).
- Lead-wide / `--waive-lead` waive, or a lead-level gap gate — papers over per-requirement decisions and over-groups when sibling extracts diverge; gate and waivers stay `<slice>/<req>` (see D16, D18).
- Shared extract or shared Evidence for multi-homed leads — changes the per-slice extract contract and couples claimable slices; not a Phase C gap-inventory decision (see D18).
- Hard- or soft-gating `plan refine` on topology / slice-list approval — second ceremony (or ignorable warning) without improving the execute gate; human pause after author is enough (see D21).
- Topology approve verb or `plan approve --slices` — extra plan-phase surface this RFC deliberately omits; handoff is social/git (see D21).
- Ready includes “cleared or waived” unknowns — circular with D16 (waivers nest only on the execute-implied fact); Ready would depend on Approved (see D22).
- Ready = “executable” while unknowns remain open — weakens the milestone; operators still need a clean-gap signal for the clean execute resume (see D22).
- Drop Ready as a computed milestone — throws away the clean-path `plan execute` resume D20 / D26 rely on; keep Ready as clean-gap only (see D22).
- Keep plan-wide “at most one `in-progress` entry” — denies that a slice is an independently workable scope; forces every actor to wait on every other slice (see D23).
- Multi-person refine only via social convention / non-overlapping selectors, without engine claims — Acceptance #2 becomes unenforceable; same-slice collisions stay silent until git fight (see D23).
- Defer multi-slice multi-actor claims to RFC-91/92 — leaves D7 and per-actor logs without the cardinality rule they exist for; solo and shared-git deployments must share one lifecycle now (see D9, D23).
- Count every plan entry including dropped as in-scope — drop cannot unblock Ready/execute without a second `plan remove` / amend (see D24).
- Exclude already-merged from the plan-phase in-scope noun — smuggles execute into Ready; covering fact binds digests at execute-start time (see D24).
- Stored scope / include-exclude list, claim- or last-selector-derived membership, phase-divergent filters, or a soft “deferred / parked” exit ramp — reintroduce editable or divergent membership; in-scope stays on the plan and not dropped (see D2, D24).
- Invent a second pin / revision wire format alongside RFC-87 `SnapshotId`, or keep build-time ambient freeze once recorded pins exist — fights D4 / D25 and the landed workspace seam.
- Treat interim `apply` retirement or project seal as this RFC’s job — `apply` deletion is claimed by [RFC-88](rfc-88-detached-changes.md); project seal stays RFC-89; this RFC records the facts and pins those steps consume.
- Keep a detached `change approve` (or other topology-approve) verb so plan can stay approve-less — RFC-88 has neither; execute is the sole authorization gesture in both RFCs.

---

## Appendix: Implementation notes

For engine contributors. Not required to evaluate the product intent.

**Already landed (RFC-87) — consume, do not reinvent**

- Value vocabulary: `project::snapshot::{SnapshotId, CodePatch}` (`sha256:…` tree digest; code patch = `{ base, result, touched paths }`).
- Workspace capability: `project::seam::Workspaces` — `freeze` / `prepare` / `capture` / `discard` / interim `apply`; host backend in `project::workspace` + `launcher::Workspaces` + `wasi-workspaces`.
- Build orchestration (`crates/slice/src/orchestrate/target.rs`) brackets finalize with prepare → capture → discard and writes `.emery/slices/<slice>/build/{request,report,patch}.yaml`. Comment at the freeze call site: *when RFC-86 records base pins, this call site reads the recorded pin instead.*
- Merge orchestration loads `build/patch.yaml`, prepares a read-only view of `patch.result`, and applies touched paths via interim `apply` (deleted by [RFC-88](rfc-88-detached-changes.md), not by this RFC). Phase B identity finalization lands on this merge path — do not reintroduce ambient checkout writes.

**Layout and writers**

- Projection kernel in `crates/project`: facts + artifact index → status, gap inventory (typed statuses only — no spec-quality rollup; D17), `ready`, and per-slice claim ownership. Shared **in-scope** filter (D24): plan entries whose slice is not dropped — used by default `plan refine` selection, `plan gaps`, `ready`, and the execute gap gate. `ready` is clean-gap only (D22): all in-scope refined + no conflicts + zero open unknowns — never consult execute-waiver lists. Gap inventory rows stay `(slice, req, status)` and omit dropped slices; when projecting `plan gaps`, join open findings to contributing `(source, lead)` via plan bindings + Evidence/provenance and, when the same lead appears in more than one open finding, attach a presentation group plus suggested re-refine selectors (D18) — never a lead-level status field or waive expansion. Property-test: any interleaving of per-actor logs, same projection; fixture with unknown-waivers on an execute-implied fact stays not-`ready` if unknowns remain on disk; many slices may show concurrent claims by different actors (D23); a dropped in-plan entry is excluded from in-scope so Ready/execute can proceed over remaining siblings without `plan remove`.
- Replace `.emery/journal.jsonl` with `events/<actor>.jsonl`; `emery journal show` merges the union.
- Remove stored plan-entry `status` and slice lifecycle fields; ladders survive only as projection labels. Retire plan-wide single-active-entry (`single_in_progress` / `next_eligible` blocking on any in-progress entry) — exclusivity is per-slice claim only (D23).
- No `approvals/` tree and no `plan approve` writer. Execute-implied approval facts live in the per-actor event log and embed gap counts plus per-requirement `[unknown]` waiver lists (`slice` + req id + reason) when execute starts past the gates. `--waive` for a non-unknown or absent gap is `plan-gaps-unresolved` / a typed waive error (name TBD).
- Concrete on-disk home follows D27: logical change tree; in-place writers target `.emery/change/` ([RFC-88](rfc-88-detached-changes.md)); detached writers target the change-directory root. Until the two-root cut lands, Phase A may keep today’s flat `.emery/` + root `plan.yaml` paths as an explicit pre-cut stand-in — do not invent a third enduring layout.

**Pins and identity**

- Plan authoring (in-place) or detached discovery / plan-author intake closes per-source `SnapshotId` pins at plan scope when the source set is known (D4 / D25). There is no `change approve` pin-close site.
- Refine writes `base.yaml` by copying those source pins and adding the baseline-spec digest **before** extract — assembly, not the first writer of source snapshot ids.
- Build reads the recorded target-base pin and passes it to `prepare` — delete the interim `seam.freeze()` at build start once pins exist (open question 2 for how far the `build/patch.yaml` re-home must go in the same phase). Do not scope interim `apply` deletion into this RFC — [RFC-88](rfc-88-detached-changes.md) owns that cut.
- Synthesis today: `IdAllocator` in `crates/slice/src/synthesis/project.rs` mints global `REQ-NNN` against the baseline. Phase B: mint slice-scoped ids; each `MODIFIED` records a digest of the baseline requirement body it changed; merge assigns baseline `REQ-NNN`, records the id map as a merge fact, rejects drifted `MODIFIED` bases.
- Validate gains `slice-base-drifted` / `slice-evidence-stale` (review signals); merge blocks on `merge-base-drifted` where needed.

**Plan refine and execute**

- Guest `plan refine` orchestration claims each selected unrefined in-scope slice (exclusive per slice — D23; in-scope = on the plan and not dropped — D24) and dispatches the existing `slice refine` orchestration (pins, extract, synthesize); default selection is every unrefined in-scope slice. Claiming slice B must not require slice A to be unclaimed or refined.
- Guest `plan execute` drops the refine leg; enforces gap + slice-status gates; on success records a covering execute-implied approval fact (digests + gap outcome + any unknown-waivers) and runs build → merge (D20 / D26). There is no `plan approve` operation to call or wire. A single execute process may walk entries sequentially, but must not reimpose a plan-wide single-active gate that prevents another actor from claiming a different slice (D23). Build/merge continue to use the landed Workspaces capability.
- Diagnostics (exit 2): `plan-gaps-unresolved`, `plan-approval-stale` (re-refine invalidated a prior covering fact — re-run execute), `plan-waiver-invalid` (waive of non-unknown / unknown id / missing reason), `slice-claim-conflict` (same slice, two actors — D23), plus staleness / merge-drift codes above. There is **no** `plan-approval-missing` (no prior approve verb), **no** `plan-approval-topology-only`, and refine must **not** emit a missing-topology-approval diagnostic. There is **no** diagnostic for “another slice is already claimed.”
- New events: `plan.refined` (or per-slice claim + existing refine events), `plan.execute.started` / approval-implied fact (unknown-waivers nested here), claim/release, `fact.retracted`, identity-mapped merge. Do not invent a `plan.approved` event that implies a separate approve verb.

**Tests**

- Multi-actor fixtures in `crates/mock`: two actors claim different slices concurrently and both refine succeed after fact union; same-slice double-claim → `slice-claim-conflict`; base drift (shared-git collaboration; not a waiver countersign gate). No fixture may require plan-wide single-active-entry.
- Shift-left fixture: author → `plan refine` → gaps → fix conflicts / waive unknowns on `plan execute` → build/merge-only execute; refuse conflict-waive and bulk-waive shapes; refuse execute while gaps remain without matching `--waive` (D20 / D26); refine succeeds with no topology-approve ceremony (D21). Ready/Approved fixture: refined + open unknowns projects not-Ready; `plan execute --waive…` reaches Approved without Ready; clearing unknowns then projects Ready before a clean execute (D22). In-scope fixture: drop one of two refined slices; default `plan refine` / `plan gaps` / Ready / execute gap gate ignore the dropped entry while the sibling remains on the plan (D24). CLI surface fixture: `emery plan approve` is absent (unknown subcommand / not wired).
- Pin fixture (Phase B): refine writes `base.yaml`; build prepares from the recorded pin (no freeze); validate reports drift when the baseline or a bound source moves; merge identity map + drifted-`MODIFIED` rejection. Workspace round-trip coverage stays in `crates/project/tests/workspace.rs` (RFC-87) — do not duplicate it here.
- Multi-homed lead fixture: one `(source, lead)` bound into two refined slices with open unknowns; `plan gaps` groups/annotates both rows and suggests both slice selectors; execute still requires per-req waive or clearance (no lead-wide waive).
- `cargo make ci` green; projection determinism and gap/execute paths covered as crate integration tests.

**Sibling series (D19)**

- During Phase C implementation, review `platform.md`, roadmap, and series RFCs for prose that still assumes execute-bundled refine, a mandatory `plan approve` verb, or interactive auto-waive (open question 3). Refresh “depends on completed 86” / completion-order language where RFC-87’s stand-in landing made it stale. Record or fix drift as ordinary delivery work; do not block this RFC’s product freeze on a cascade rewrite. Follow [RFC-88](rfc-88-detached-changes.md) when reconciling siblings — it has no `change approve` and uses execute as the sole authorization surface (closed items 15–16).

**Hard cut**

- Pre-1.0: re-init over migration; no shims for old status fields, single journal, plan-wide single-active-entry, global synthesize-time ids, execute-bundled refine, a plan `approve` verb, or build-time ambient freeze once recorded pins exist.
