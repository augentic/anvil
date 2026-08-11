# RFC-86: Change Facts

> **Status:** Implemented — substrate D1–D11 and product D12–D27 are closed and landed through Phase C; the D20 sibling-doc pass and product acceptance checklist are complete.
>
> **Amendment (plan-centric surface cut):** the breakout-verb *surface* referenced by D12 / D14 / D26 — `emery slice refine` (and the sibling `slice build` / `slice merge` / `slice drop` operator verbs) — is **superseded** by the three-verb public surface: `plan author → plan execute → plan archive` (plus `plan drop` for in-scope exclusion). Refine, build, and merge are now phases only the execute loop drives; execute covers a Refined slice whose recorded pins drifted as `refine-under-epoch`, so re-running execute re-refines exactly the affected slices. The *semantics* these decisions closed — the gap gate before build, typed `closed-plan` coverage, unknown waivers, the authorization epoch, no `plan refine` verb — are unchanged. Mentions of `emery slice refine` in the closed-question trail below are historical; the Acceptance section and operator surface describe today’s execute phases.
>
> **Amendment (implemented):** [RFC-91 Refinement Stage](rfc-91-refinement-stage.md) amends D6 / D8 / D12 / D14 / D22 / D25 / D26 after removal of the per-slice breakouts exposed the missing specs-only stage. RFC-91 is implemented: `emery plan refine` is the standalone refinement stage, execute never refines, and `plan.execute.started` covers per-leaf refinement digests. Mentions of the execute-owned refine path and `refine-under-epoch` coverage below read historically.
>
> **Amendment (gap deferral — [RFC-86a](rfc-86a-gap-deferral.md)):** the per-epoch waiver surface this RFC shipped is **deleted** in favour of durable, digest-bound deferral facts and a declared gap policy. **D15** is amended (an *open* `[unknown]` blocks build; a *deferred* one leaves build scope and build proceeds); **D17** is amended (`--waive` / `--reason` argv and the coverage `unknown-waivers` field are gone — the disposition surface is `emery plan defer` + `--gap-policy <strict|defer>`, and the coverage payload carries the effective `gap-policy`; conflict **build-over** stays forbidden while conflict **deferral** is RFC-86a D6); **D22** is preserved with deferrals in the waiver seat (Ready counts open *and* deferred findings). Waiver prose below — including the `unknown-waivers` payload example — is historical.
>
> **Proposed amendment:** [RFC-91 Refinement Stage](rfc-91-refinement-stage.md) revisits D6 / D8 / D12 / D14 / D22 / D25 / D26 after removal of the per-slice breakouts exposed the missing specs-only stage. Until RFC-91 is accepted and implemented, this RFC's execute-owned refine path remains the in-force contract.
>
> **Series:** Step 1 of the [platform-migration series](platform.md) by product ownership — the fact substrate every later step consumes. Phases A–C are landed against [RFC-87](rfc-87-working-trees.md)’s workspace contract: recorded `base.yaml` pins, content-addressed `builds/<digest>.yaml` records, and one-member waves. The remaining series stand-ins live in later RFCs — merge-time `apply` and the flat `.emery/` change home ([RFC-88](rfc-88-detached-changes.md) deletes `apply` and relocates the change home). Later series RFCs assume this substrate’s authorization epoch, per-leaf coverage, and one-member target waves.
>
> **Owns:** the fact-based change substrate — projected status, per-writer event logs, claims (ownership, not authority), `plan.execute.started` as an explicit authorization epoch with typed `closed-plan` coverage, one-member target-wave merge facts, pinned judgment inputs (landed RFC-87 `SnapshotId` / wire `cid`), merge-finalized requirement identity, shift-left refine practice / gap gate before build (no separate `approve` or `plan refine` verb), and the single-writer desktop as the degenerate case of the same substrate.
>
> **Audience:** Operators and contributors who know Emery’s workflow vocabulary (plan, slice, refine, build, merge, sources, specs). This RFC is the in-force product contract for the change-facts substrate; the appendix records contributor implementation notes.



## Synopsis

This RFC does two related things:

1. **Shift specs left (as practice, not a new plan verb).** Creating per-slice specs (`refine`) should happen — and be reviewed, with typed gaps dealt with — before generation spend. The public plan surface stays [RFC-88](rfc-88-detached-changes.md)’s three verbs: `plan author → plan execute → plan archive`. Refine is an execute-loop phase (not a plan or slice CLI breakout). Starting execute opens the authorization epoch — there is no separate `approve` CLI verb. Execute may also authorize [RFC-88](rfc-88-detached-changes.md) `refine-under-epoch` for leaves that lack specs yet; the gap policy still blocks **build** over undealt conflicts / unknowns.
2. **Make change state shareable.** Workflow progress stops living in mutable status fields. A change becomes a set of durable files and an append-only history. Status is *computed* from that history. The same model works on one laptop or across several people and machines.

Together: better specs before generation, and a state model that can travel — without adding a fourth public plan verb that fights the sibling series surface.

---



## Why

Emery already treats specs as the contract that drives build. Real projects show that **build quality tracks spec quality**. Today that contract is created too late, and known holes do not stop generation.

### Problems this RFC closed (pre-landing baseline)

Before Phases A–C landed, the operator loop looked like:

```text
/emery:plan            survey sources → propose slices → review the plan
emery plan execute     for each slice: refine → build → merge
```

Build and merge already used [RFC-87](rfc-87-working-trees.md) private workspaces, but coordination and pins did not yet follow this RFC. The pre-landing path self-froze at build, persisted `build/patch.yaml`, used one `.emery/journal.jsonl`, stored plan-entry / slice lifecycle fields, and treated “running execute” as approval with no digest-bound epoch.

| Problem                              | In plain terms                                                                                                                                                                                                                    |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Specs appear mid-execute             | Plan review sees leads and `plan.yaml`, not `spec.md`. By the time specs exist, the loop is already heading into build.                                                                                                           |
| Gaps do not stop build               | Refine already tags `[unknown]`, `[conflict]`, and `[divergence]` on requirements. Those tags are informational only — execute builds anyway.                                                                                     |
| Execute leaves no durable start fact | Running `plan execute` *is* the operator gesture that starts privileged work (and stays that way — see D6), but nothing records *which* digests / per-leaf coverage / gap outcome that gesture authorized.                        |
| Status is hard to share              | Progress is stored as fields in YAML files. Two people cannot safely collaborate on one change, and the same pattern will not scale to multiple machines.                                                                         |
| Requirement IDs collide              | Two slices refined against the same baseline can mint the same `REQ-NNN` numbers. Merge can silently overwrite.                                                                                                                   |
| Pins are not recorded                | Refine does not write `base.yaml`. Build freezes an ambient tree at call time. Drift between refine and build is not a typed gate.                                                                                                 |
| Merge has no wave shape              | Merge is a single-slice write-back. Series RFCs need an immutable one-member target wave (build authorization vs commit authorization) so RFC-88 can attach accepted-CID semantics and RFC-92 can widen membership later.         |

### Landed substrate (Phases A–C)

Those problems are closed in code. The in-force path is:

- Per-writer logs at `.emery/events/<writer>.jsonl` (no authoritative `journal.jsonl`)
- Computed plan / slice status (no stored status fields to edit)
- `plan.execute.started` with typed `closed-plan` coverage at execute start
- Refine-authored `base.yaml`; build prepares from the recorded `target_base` (no ambient freeze-at-build)
- Content-addressed `builds/<digest>.yaml` (`BuildRecord`) as build-outcome authority (no `build/patch.yaml`)
- One-member waves at `.emery/targets/<target>/waves/<digest>.yaml` with `target.wave.opened` / `target.merge.wave-committed`

Still deferred to later series RFCs: merge-time `apply` and the flat `.emery/` change home ([RFC-88](rfc-88-detached-changes.md)); publication seals ([RFC-89](rfc-89-publication-sets.md)); engine-owned verify/repair phases ([RFC-90](rfc-90-build-verification.md)).




### What we want instead

```text
Public surface   emery plan author → emery plan execute → emery plan archive
Preferred path   author → review topology → execute (refine → review gaps / waive → build → merge)
Also allowed     execute with refine-under-epoch (RFC-88) for leaves still missing specs; gap policy still blocks build
```

- Operators review **topology** after author, then run `emery plan execute` / `/emery:execute`. The loop’s refine phase mints specs; operators read those artifacts and the **gap inventory** before generation spend continues into build. Prose quality is human-owned — the engine does not record how spec reading happens (see D18).
- Known gaps are listed and, by default, **block build**. Slice lifecycle / computed status gates already refuse work that has not been dealt with; this RFC adds the typed gap policy on the same seam (enforced when execute would enter build — including after under-epoch refine).
- There is **no** `emery plan approve` **(or other** `approve`**) CLI verb** and **no** `emery plan refine` **verb**. The public plan surface stays three verbs with RFC-88. Starting `emery plan execute` appends `plan.execute.started` **at execute start** with typed `closed-plan` coverage (existing spec digests and/or `refine-under-epoch`, plus any unknown-waivers) — durable authorization without a second ceremony (see D6). There is no projected `approved` rung and no `approvals/` artifact tree.
- One person on a laptop and a multi-person (later multi-node) change use the **same** rules.

Unchanged: source and target adapters, artifact shapes (`spec.md`, Evidence, etc.), the meaning of refine / build / merge as verbs, and the landed RFC-87 workspace contract (`prepare` / `capture` / `discard`, `SnapshotId`, code patch as `{ base, result, touched paths }`). Wire documents name that tree identity `cid` ([RFC-88](rfc-88-detached-changes.md)); Rust may keep the `SnapshotId` type. What this RFC changed is **when** refine is preferred (before build, as an execute phase — not folded into author, and not a new plan verb), **whether** gaps may enter build, **how** progress is stored, **how** pins / build outcomes are recorded so workspaces consume durable facts, and **how** merge projects through an immutable one-member target wave. Alignment with RFC-88’s `refine-under-epoch` means execute is not forbidden from authorizing refine for unspec’d leaves; it is forbidden from **building** over undealt typed gaps.

---



## Operator flow



### Author — topology only

1. **Author** — `emery plan author` (via `/emery:plan`) surveys bound sources and reconciles leads into slices (detached mode also runs RFC-88’s internal discovery / decomposition phases). Produces the authored plan artifacts (`discovery.md` / RFC-88 `leads.md` + `decomposition.yaml` as applicable, `change.md`, `plan.yaml`). **Stops here** — no extract or synthesis. The operator can review topology before paying for refine. Folding refine into author is rejected (see D14).



### Preferred path — specs before generation

1. **Review topology** — Human pause: re-cut slices / decomposition if needed. No topology-approve verb (see D21). Author epilogues and fresh-plan `plan status` still resume at `emery plan execute` / `/emery:execute` (see D26) — that is the next *plan* verb; refine is not a separate CLI verb.
2. **Execute (refine phase)** — `emery plan execute` claims each eligible in-scope slice (on the plan and not dropped, see D24), extracts Evidence, synthesizes `proposal.md`, `spec.md`, `design.md`, `tasks.md`, and `model.yaml`, and records input pins. Claims are exclusive per slice (D23). Claims never create authorization for build/merge beyond the covering epoch (D6 / D7).
3. **Review gaps** — Read `spec.md` and related slice artifacts, and the **gap inventory** (see below). Deal with typed issues before generation: close conflicts, clear or plan to waive unknowns. Prose quality and acceptance-criteria depth are operator-owned (see D18). Re-running execute resumes at the parked phase.
4. **Iterate** — Fix inputs (richer intent/docs, authority overrides, corrected sources), then re-run `emery plan execute` so only the affected slices re-refine under a fresh or matching epoch.



### Execute — authorization epoch, then code

1. **Execute** — `emery plan execute` (via `/emery:execute`) is the sole privileged-start surface (RFC-88 D8). **At execute start** it appends `plan.execute.started` with typed `closed-plan` coverage (D6): for each in-scope leaf, either an **existing spec digest** (preferred shift-left path) or RFC-88 `refine-under-epoch`. Under that epoch, the refine phase may still run before build. Gap and slice-status gates refuse **build** when issues have not been dealt with. Once every leaf that will build is refined, conflicts are gone, and remaining `[unknown]`s are either cleared (Ready — D22) or explicitly waived per requirement on this command (D17), execute runs **build → merge** under one-member target waves (D9). There is no prior `approve` verb and no projected `approved` milestone.
2. **Finalize** — Operator publishes; archive as today (`plan archive` / `/emery:finalize`).

`/emery:plan` remains an ultrathin wrapper over `plan author` only. There are no hand-driven `slice refine` / `build` / `merge` breakout verbs — those phases run only inside execute. This RFC adds **no** `emery plan refine` batch verb and **no** `approve` verb. The public workflow stays `plan author → plan execute → plan archive`.

One-slice changes stay the same shape: author → (optional topology pause) → `plan execute` (refine → review/waive → build → merge), including `refine-under-epoch` when specs are absent at start. `plan status` resume after a fresh author is `emery plan execute` / `/emery:execute` (D26); next-actions may project `refine <slice>` while in-scope slices are unrefined; close-gaps or `plan execute --waive…` when refined with open unknowns (not Ready — D22); and `emery plan execute` when Ready (see D6 / D22).

### Who owns what


| Work                                          | Outside execute                        | Under execute                                                                                                      |
| --------------------------------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Survey and propose slices (`plan author`)     | yes                                    | no                                                                                                                 |
| Extract and synthesize specs (refine phase)   | no — not a CLI breakout                | yes — under `plan.execute.started` (`existing` digests or `refine-under-epoch`)                                    |
| Review specs (prose) and close typed gaps     | yes — after refine parks / between runs | yes — gap gate before build                                                                                        |
| Open authorization epoch by starting execute  | human topology review precedes it when possible | yes — `plan execute` appends `plan.execute.started` at start; gap/status gates enforce readiness before build (D6) |
| Build and merge code                          | no                                     | yes — under one-member target waves (D9)                                                                           |


---



## Gaps before build



### What refine already tells us

When sources are combined into a requirement, Emery assigns a status:


| Status       | Tag            | Meaning                                                                      |
| ------------ | -------------- | ---------------------------------------------------------------------------- |
| `agreed`     | —              | Sources agree (or there is only one claim)                                   |
| `unknown`    | `[unknown]`    | **Gap** — not enough evidence; the requirement is incomplete                 |
| `divergence` | `[divergence]` | Sources disagree; a higher-authority source won; the loser is kept as a note |
| `conflict`   | `[conflict]`   | Sources disagree and no automatic winner — **unresolved contradiction**      |


Plan authoring can also flag fuzzy *slice* matching (tentative merges, `divergence: likely`). That is about how work is grouped. Requirement gaps appear only after refine — another reason specs should exist (and be reviewed) before build.

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

This list is **derived** from the specs/`model.yaml` already on disk. It is not a second file to keep in sync. It includes only typed requirement statuses (`unknown`, `conflict`, `divergence`) — not prose-quality advisories (see D18). `plan status` surfaces it; a dedicated read-only projection (`plan gaps`, name TBD) may expose the same inventory without adding an orchestration verb (see Commands).

**Gate authority stays per requirement** — each row is still `(slice, req, status)`. When the same lead is multi-homed across slices (coverage is at-least-once; cross-cutting leads in `change.md`), one thin or contradictory lead can surface as several inventory rows. The gaps projection therefore also offers a **presentation rollup**: annotate or group open findings that share a contributing `(source, lead)`, and suggest the slice-selector set for a follow-up re-refine under execute after the shared input is fixed (see D19). The rollup is navigation only — it does not merge findings, change the execute gap gate, or introduce lead-wide waivers.

### How to close gaps

Do not hand-edit the machine-rendered `ID:` / `Sources:` / `Status:` lines or the `[…]` tags. Change the inputs, then re-refine (same rule as today’s conflict how-to):


| Finding        | Typical fix                                                                                                                                                                                                                                                 | Next step                                                                             |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `[unknown]`    | Operator supplies the missing information: enrich a source (intent, docs, captures), or re-scope the lead so the requirement is not invented. Build stays blocked until the tag clears or that requirement is explicitly waived on `plan execute` (see D17) | Re-run `emery plan execute` so that slice re-refines (and any siblings the rollup lists for the same lead) |
| `[conflict]`   | Set an authority override, or remove/correct a misleading source. **Not waiveable** — resolve inputs, then re-refine                                                                                                                                        | Re-refine (per slice that still shows the conflict; overrides remain per-slice)       |
| `[divergence]` | Informational — authority already chose; no decision required. Override only if the wrong source won                                                                                                                                                        | Re-refine only if inputs changed                                                      |
| Stale inputs   | Sources or baseline moved since refine pinned them                                                                                                                                                                                                          | Re-pin and re-refine                                                                  |


Prefer fixing one shared input, then re-refining the affected slice set (the rollup’s suggested selectors when findings share a lead). Full-plan re-synthesis is expensive. Waivers stay per-requirement (`--waive <slice>/<req>`) even when rows share a lead.

### Human-only ambiguity (outside the engine)

Vague prose in an otherwise `agreed` requirement (weak scenarios, missing acceptance criteria) stays a **human** review concern. The operator reads `spec.md` / `design.md` by whatever process fits — IDE, PR, checklist on paper, pair review — and decides whether to start execute (or whether to build under an already-started execute epoch). The agentic and programmed workflow does not model that step: no checklist artifact, no review attestation on execute, and no spec-quality findings in the gap inventory (see D18). This RFC only machine-gates the typed statuses above.

### When build is allowed (gap policy)

Refine may finish with tags still present — the slice is refined, but not necessarily ready to build.

`plan execute` opens the authorization epoch at start (D6). Before entering **build** for a leaf — whether specs were pre-refined or produced under `refine-under-epoch` — it checks a gap policy alongside existing slice-status / lifecycle gates:


| Finding        | Policy                                                                                                                                                                                                                                                                                                                                                                            |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `[conflict]`   | **Block, not waiveable** — do not generate code over an unresolved contradiction. Resolve via authority override or source correction, then re-refine (see D17)                                                                                                                                                                                                                   |
| `[unknown]`    | **Block** — insufficient information reached the agent; the operator must supply it (richer sources, re-scope) or **explicitly waive that requirement** on `plan execute` before build proceeds. Uniform for every change shape — including intent-only / N=1. Desk-testing shows warn-only yields unpredictable generation that compounds through build and merge (see D15, D17) |
| `[divergence]` | **Allow, but list** — informational only; authority already chose (`intent` > `documentation` > `behaviour`, plus any per-slice override). No acknowledgment or decision required to start execute or enter build. Override and re-refine only when the wrong source won (see D16)                                                                                                |


If the policy fails, execute refuses to enter build (or stops before build) and prints the inventory. The operator may:

- close the findings and continue under the same epoch once gaps clear, or
- **explicitly waive** individual `[unknown]` requirements on the execute command (recorded on the `plan.execute.started` coverage payload, each with a reason) — never silently, never as a plan-wide or slice-wide off-switch, and never for `[conflict]` (see D17).

There is **no separate** `approve` **verb** and **no projected** `approved` **milestone**. Starting execute opens the authorization epoch; when the gap/status gates pass for build, privileged work proceeds under that covering `plan.execute.started` fact (digests + per-leaf coverage + any unknown-waivers). The binding goes stale when covered artifacts change — including every waiver it carried — and execute must be started again (re-list any remaining unknown-waivers) before further build/merge may proceed under a fresh epoch (see D6 / D22). Execute must **not** auto-waive gaps — listing `--waive` is explicit.

Topology handoff after author is human-owned (pause and re-cut the slice list if needed). This RFC adds **no** topology-approve / `--slices` CLI surface (see D21).

---



## How state works (the substrate)

Today, “where is this change?” answers are scattered: status fields in YAML, one shared journal file, execute as an unrecorded gesture. That works for one operator on one machine. It does not travel.

### Three kinds of thing


| Kind          | Role                                                    | Examples                                                                                                                                                                                                             |
| ------------- | ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Artifacts** | Durable files that describe the change                  | `plan.yaml`, per-slice `spec.md` / `design.md` / Evidence, `base.yaml` pins, build records, closed target-wave manifests under `targets/<target>/waves/<digest>.yaml`, RFC-88 lead-catalog / decomposition revisions |
| **Facts**     | Append-only history of what happened                    | “slice claimed”, `plan.execute.started` (authorization epoch), “build succeeded”, `target.merge.wave-committed`                                                                                                      |
| **Values**    | Content-addressed product-code trees (landed in RFC-87) | `SnapshotId` / wire `cid` (`sha256:…` tree digest); a **code patch** is the relation `{ base, result, touched paths }` — no separate patch blob                                                                      |


Nothing else is workflow authority. In particular, **status is never stored as a field to edit**. `plan status` *computes* progress from artifacts + facts. Facts and build records **reference** snapshot / `cid` identities; they never store workspace paths. Private workspaces remain disposable execution machinery under host-owned storage (`$EMERY_HOME/snapshots/`, `$EMERY_HOME/workspaces/`) — outside the change tree. Authorization, ownership, and input identity are orthogonal: the execute-start fact grants work, a claim assigns one leaf to one journal writer, and each build or merge fact pins the exact inputs it consumed. There is **no** separate on-disk `approvals/` family and **no** projected `approved` rung (see D6).

### Rough layout of a change

Illustrative coordination layout (D1 — layout-neutral; concrete homes from [RFC-88](rfc-88-detached-changes.md)):

```text
<change>/                   # in-place: .emery/change/ ; detached: ordinary directory root
  change.md
  plan.yaml                 # topology + executable leaf projection — not status
  decomposition.yaml        # RFC-88 conflict-domain hierarchy (detached); no lifecycle
  events/<writer>.jsonl          # each journal writer appends only its own log
                              # (includes plan.execute.started authorization epochs)
  targets/<target>/waves/<digest>.yaml   # immutable one-member wave manifests (D9)
  slices/<slice>/
    base.yaml               # refine-time pin assembly (sources + baseline)
    evidence/…
    spec.md, design.md, tasks.md, model.yaml
    builds/<digest>.yaml    # BuildRecord: base/result + touched + wave digest + report (D27)
                            # (retired build/patch.yaml as outcome authority)
```

Durable project state (`project.yaml`, baseline `specs/`, `decisions/`) stays outside the change home. Sharing a change is ordinary file exchange of that home (push / pull / PR / copy when the operator versions it). The change-tree contract requires no Git metadata — two people’s event logs union without fighting over one journal file. Versioning and transport are deployment or operator policy, not workflow prerequisites ([RFC-88](rfc-88-detached-changes.md) amends any prior “must be a git repo” reading).

### Progress, computed


| Milestone           | Meaning                                                                                                                                                                                                                                                                                                                                          |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Authored            | Plan has slices                                                                                                                                                                                                                                                                                                                                  |
| Refined (per slice) | Validated specs exist for that slice, pinned to known inputs (refine phase)                                                                                                                                                                                                                                                                      |
| Ready               | Every in-scope slice is refined, and the **clean** gap policy passes: no conflicts, **zero** open unknowns. Artifact-derived only — waivers are not part of Ready (see D22). In-scope is on the plan and not dropped (see D24)                                                                                                                   |
| Authorized          | A covering `plan.execute.started` epoch exists for the current plan with typed per-leaf coverage (may carry unknown-waivers and/or `refine-under-epoch`). Distinct from Ready even when the waive list is empty — Ready means “clean path to build”; Authorized means “execute has opened a covering epoch” (D6 / D22). **Not** named `approved` |
| Built / merged      | A content-addressed `builds/<digest>.yaml` `BuildRecord` and a `target.merge.wave-committed` fact exist for that slice (referencing RFC-87 base/result ids). D27 retired `build/patch.yaml` as outcome authority                                                                                                                                  |


`plan status` next actions: after a fresh author, **resume** is `emery plan execute` / `/emery:execute` (D26); while in-scope slices are unrefined, next-actions may project `refine <slice>` (phase label — resume remains execute); then review-gaps; then `plan execute` (build → merge) once gates pass — or execute with refine-under-epoch when following the RFC-88 authorization shape. Open unknowns mean the change is **not** Ready; resume points at closing gaps *or* `plan execute` with per-requirement `--waive` (skipping Ready). Clean Ready resumes at `emery plan execute`. There is no resume point at a separate `approve` or `plan refine` verb, and no projected `approved` rung.

### Authorization epoch and coverage

Privileged work (build, merge) may proceed only under a live `plan.execute.started` authorization epoch. The first implementation accepts only `closed-plan` coverage over one reviewed plan digest. Its sorted per-leaf spec coverage is either `existing { digest }` or `refine-under-epoch`: the latter authorizes only the spec produced by that leaf's refinement fact under this epoch, preserving today's refine→build loop without pretending an unknown future digest was reviewed. Optional `unknown-waivers` nest on the same payload (D17). A changed or externally replaced covered artifact requires a new epoch. A claim or projected `in-progress` status never implies authorization.

Every `slice.build.started` and merge-wave member binds its authorization epoch plus the exact leaf, spec, dependency frontier, and pinned base it consumed. RFC-88 extends that fence with lead-catalog, decomposition-revision, and target-`cid` digests. Any changed input makes the result stale independently of whether the epoch remains open. [RFC-94](future/rfc-94-streaming-execution.md) owns the future authority mode that may build ready leaves while survey continues but cannot commit a target wave; accepted-`cid` mutation still requires a later reviewed closed-plan gesture. There is no `approved` rung.

Example detached execute start (one line in `events/<writer>.jsonl`):

```json
{"timestamp":"2026-08-05T04:30:00Z","writer":"operator-a","sequence":7,"event":"plan.execute.started","payload":{"coverage":{"kind":"closed-plan","plan-digest":"sha256:…","specs":{"orders-api":{"kind":"existing","digest":"sha256:…"},"orders-ui":{"kind":"refine-under-epoch"}},"unknown-waivers":[{"slice":"orders-api","req":"REQ-003","reason":"reset path deferred"}]},"discovery-digest":"sha256:…"}}
```



### Pins and requirement IDs (why implementers care)

Two rules make multi-slice and multi-person planning safe:

1. **Pins** — Judgment legs run against recorded [RFC-87](rfc-87-working-trees.md) `SnapshotId`s / wire `cid`s (and baseline-spec digests), not ambient trees. Pin *authorship* closes when inputs are knowable (see D4 / D25): source ids at plan authoring (in-place) or at detached discovery / plan-author intake ([RFC-88](rfc-88-detached-changes.md) — no `change approve` verb); refine assembles each slice’s `base.yaml` from those source pins plus the baseline digest before extract; build reads a recorded target-base pin before `prepare` (retiring today’s self-freeze). If pins move later, validate reports staleness instead of silently building on drift.
2. **Local then global IDs** — Each slice uses its own requirement ids while planning. Target-wave commit (D9) assigns final baseline `REQ-NNN` numbers and records the mapping. Two slices can no longer collide by minting the same id at refine time. (Phase B landed: synthesis mints slice-scoped ids; `MODIFIED` rows record baseline body digests; wave commit assigns baseline `REQ-NNN` and rejects drifted bases.)



### Desktop = simplest deployment

One operator, one machine, no remote: same artifacts, same facts, same commands. Multi-person and later multi-node add transport, not a second lifecycle. When two people share a change over ordinary file exchange, each may claim a **different** slice and refine at the same time; a slice still has only one owner (D23).

---



## Decisions (summary)

Substrate decisions D1–D11 match the series-facing spine assumed by [RFC-88](rfc-88-detached-changes.md) (which amends D1’s concrete homes) and [RFC-89](rfc-89-publication-sets.md). Product decisions D12–D27 are this branch’s closed gap / shift-left / membership / Phase B stand-in layer.


| #   | Decision                                                                                                | Operator-visible effect                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --- | ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | **Change is a self-contained fact tree with no version-control requirement**                            | In-place home is `.emery/change/` beside durable `.emery/` project state; detached home is an ordinary directory root ([RFC-88](rfc-88-detached-changes.md)). Planning can last days; coordination artifacts travel as ordinary files. Versioning / clone / PR of the change home is operator policy, not a workflow prerequisite                                                                                                                                                                                                                                                                                                                                                                                    |
| D2  | Status is computed, not stored                                                                          | `plan status` is the only progress view; no hand-edited status fields                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| D3  | Per-person (or per-node) event logs                                                                     | Collaboration and later multi-machine sync without one contested journal file                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| D4  | Every refine/build pins its inputs (RFC-87 `SnapshotId` / wire `cid`)                                   | A reviewed spec is tied to what it was made from; drift is detected. Source pins close when the source set closes; refine assembles `base.yaml`; build reads a recorded base — never ambient self-freeze (see D25)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| D5  | Requirement ids finalized at target-wave commit                                                         | Parallel refine of different slices against one baseline is safe                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| D6  | `plan execute` **opens an explicit authorization epoch**                                                | Before privileged work, appends `plan.execute.started` with typed `closed-plan` coverage (`existing { digest }` or `refine-under-epoch` per leaf; optional `unknown-waivers`). Durable operator authority, not a worker claim. No plan-approval verb, status, file, or projected `approved` rung. Starting execute is the operator gesture; the fact is an orchestration event like `slice.build.started`                                                                                                                                                                                                                                                                                                            |
| D7  | Work is claimed in the log                                                                              | A slice has at most one owner at a time; claims never create authorization (see D23). Internal conflict domains carry no claim — only terminal slices do                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| D8  | Phases consume pinned inputs only                                                                       | Retry after failure loses no completed work. Build bindings include build authorization; commit adds its closed-plan authorization                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| D9  | **Merge transitions use one immutable target wave**                                                     | Before build, write `targets/<target>/waves/<digest>.yaml` (one member in this cut) with build-authorization epoch; merge appends `target.merge.wave-committed` with a (possibly distinct) closed-plan commit-authorization, then postflight succeeded / postflight-failed. [RFC-88](rfc-88-detached-changes.md) attaches base/result `cid`s and accepted-`cid` semantics to the same shape; [RFC-92](rfc-92-concurrent-execution.md) may widen membership                                                                                                                                                                                                                                                           |
| D10 | One lifecycle everywhere                                                                                | Laptop and fleet differ only by transport config                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| D11 | Hard cut (pre-1.0)                                                                                      | No compatibility shims for old status fields, single journal, unrecorded execute starts, ambient freeze once recorded pins exist, or `build/patch.yaml` as build-outcome authority once fact-substrate records exist (D27)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| D12 | **Shift-left refine is preferred practice; execute owns authorization + build/merge**                   | Specs are preferred before generation; public surface stays author → execute → archive. Execute may authorize RFC-88 `refine-under-epoch`; gap policy still blocks build                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| D13 | **Gaps gate build, not refine success**                                                                 | Incomplete Evidence can still refine; it cannot silently enter build                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| D14 | **No** `plan refine` **verb; author stays topology-only; refine is** `slice refine`                     | Aligns with RFC-88’s three-verb public surface. Author / `/emery:plan` do not extract or synthesize. Rejected: fold refine into author; invent `emery plan refine` as a fourth plan verb                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| D15 | `[unknown]` **always blocks build** *(amended by [RFC-86a](rfc-86a-gap-deferral.md))*                   | Thin intent is not an exception — close the gap or waive it explicitly on execute; generation must not invent missing information. *RFC-86a: an **open** `[unknown]` blocks; a **deferred** one leaves build scope*                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| D16 | `[divergence]` **is informational; listed but allowed**                                                 | Authority hierarchy already picked a winner; execute does not require per-divergence acknowledgment. Wrong winner → override / amend sources and re-refine                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| D17 | **Waiver UX: per-**`[unknown]` **on execute;** `[conflict]` **never waiveable; no multi-operator gate** *(amended by [RFC-86a](rfc-86a-gap-deferral.md))* | Repeatable `--waive <slice>/<req>` + required `--reason` on `plan execute`; waivers nest on the `plan.execute.started` coverage payload; one operator’s execute start is enough; re-refine / covered-artifact change clears the epoch and its waivers. *RFC-86a: the waiver surface is deleted — dispositions are durable `gap.deferred` facts via `plan defer` / `--gap-policy`; conflict build-over stays forbidden, conflict deferral is D6 there*                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| D18 | **Human prose review stays outside the engine**                                                         | Operators own spec quality; execute’s gap gate covers typed statuses only. No checklist artifact, review attestation, or spec-quality rollup in the gaps projection / execute                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| D19 | **Shared-lead gap rollup is presentation only**                                                         | Gaps projection annotates/groups open findings that share a contributing `(source, lead)` and suggests re-refine selectors; execute/waive stay per-requirement. No lead-wide waive, no shared-Evidence extract, no lead-level gate                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| D20 | **Sibling-series consistency is an implementation task**                                                | D6 / D17 are the in-force series authority rule; [platform.md](platform.md) chronology and authorization prose are refreshed to match (ownership vs landing; execute opens the epoch; no auto-waive). When implementing this RFC (especially Phase C), still review remaining series RFCs and shipped operator docs (`AGENTS.md`, `workflow.md`, CLI help) for leftover `plan approve` / projected `approved` / invented `plan refine` / “nothing is stamped” wording — update those when `plan.execute.started` lands. Do not cascade-rewrite them as a freeze prerequisite                                                                                                                                         |
| D21 | **No topology-approve CLI surface**                                                                     | Human pause after author is the topology review seam. No `plan approve --slices` (or other topology-approve) verb — handoff is social / ordinary file exchange. Nothing machine-gates refine on a topology approval                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| D22 | **Ready is clean-gap only; waivers live only under an Authorized epoch** *(preserved by [RFC-86a](rfc-86a-gap-deferral.md) with deferrals substituted)* | Ready = all in-scope refined + no conflicts + zero open unknowns (no waiver contribution). Authorized = covering `plan.execute.started` (possibly with unknown-waivers). Waiver path skips Ready; never make Ready depend on an epoch that does not exist yet; never project an `approved` rung. *RFC-86a: Ready counts open **and** deferred findings; debt-carrying plans build via Authorized*                                                                                                                                                                                                                                                                                                                                                                                                                      |
| D23 | **Many slices in flight; one writer per slice**                                                         | Different slices may be claimed and progressed by separate journal writers at the same time. A slice is an exclusive unit of work — never two writers on one slice. Plan-wide “at most one `in-progress` entry” is retired. Swarming *inside* one slice stays a non-goal                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| D24 | **In-scope = on the plan and not dropped**                                                              | One shared membership predicate for the gaps projection, Ready, the execute gap gate, and status’s unrefined next-actions. `plan remove` deletes the entry; `slice drop` abandons it and excludes it from in-scope. Optional refine selectors narrow breakout work only — they do not redefine membership                                                                                                                                                                                                                                                                                                                                                                                                            |
| D25 | **Pins use landed RFC-87 snapshot identity; wire name is** `cid`                                        | Pin *wire identity* in plan/discovery artifacts is `cid` (`sha256:…`); Rust type remains RFC-87’s `SnapshotId`. This RFC owns *when pins are written and what consumes them*: source ids close when the source set closes — at plan authoring (in-place) or at detached discovery / plan-author intake ([RFC-88](rfc-88-detached-changes.md); there is no `change approve` pin-close site); refine copies those pins into `base.yaml` and adds the baseline-spec digest before extract; build reads the recorded target-base pin before `prepare` (replacing the interim freeze-at-build stand-in). Exact on-disk `base.yaml` shape stays an implementation detail. Phase B does **not** wait on further RFC-87 work |
| D26 | **Post-author resume stays** `plan execute`                                                             | After successful author, author epilogues and `plan status` resume at `emery plan execute` / `/emery:execute` — matching today and RFC-88’s public surface. Topology/decomposition review is human prose in the hint, not a verb. Status may still suggest `slice refine <slice>` as a next-action while slices are unrefined. Rejected: invent `plan refine` as the resume; fold refine into author; require a topology-approve pause command                                                                                                                                                                                                                                                                       |
| D27 | **Phase B retires freeze +** `patch.yaml`**; leaves only interim** `apply` **for RFC-88**               | Phase B acceptance bar: (1) build never ambient-`freeze`s — always prepares from a recorded pin; (2) re-home `build/patch.yaml` into content-addressed fact-substrate build records referenced by one-member waves; (3) “built” / “merged” project only from those records and `target.merge.wave-committed` — never from a leftover path check; (4) merge may still call interim `apply` from the recorded result CID / `CodePatch`. Rejected: land pins + waves while leaving `patch.yaml` as the build-outcome authority (dual status; forces RFC-88 to re-home again). Interim `apply` deletion stays [RFC-88](rfc-88-detached-changes.md)’s cut                                                                 |




---



## Commands that change

This RFC adds **no orchestration / lifecycle subcommand** beyond re-expressing existing verbs over facts. Read-only projections (`plan status`, optional `plan gaps`) are not orchestration verbs.


| Command                                        | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `emery plan approve` (any `approve` plan verb) | **Removed / never shipped.** Not part of the CLI surface. Starting `plan execute` opens the authorization epoch; gap and slice-status gates enforce readiness before build (see D6). No `--slices` topology-approve substitute (D21). No `approvals/` tree; no projected `approved` rung                                                                                                                                                                                                                                                                                      |
| `emery plan refine`                            | **Not shipped.** This RFC adds no plan-phase refine batch verb (D14). N-many refine is the execute loop’s refine phase over in-scope entries. Aligns with RFC-88’s three-verb public surface                                                                                                                                                                                                                                                                                                                                                                                   |
| `emery plan gaps`                              | **Read-only projection** of the typed-status gap inventory (not an orchestration verb; same data `plan status` can surface — see D18). When open findings share a contributing `(source, lead)`, annotates or groups those rows and suggests the slice-selector set for re-refine — presentation only; gate and waivers stay per-requirement (see D19). *(RFC-86a: waivers are deleted — the gate and durable deferrals stay per-requirement, and the projection gains a disposition column, `open \| deferred`)*                                                                                                                                                                                                                       |
| `emery plan execute`                           | Sole privileged-start surface (D6; RFC-88 D8). **At start** appends `plan.execute.started` with typed `closed-plan` coverage (per-leaf `existing` / `refine-under-epoch`, optional `unknown-waivers`, detached digests as required by RFC-88). Drives refine → build → merge; enforces the gap policy and slice-status gates before **build**; runs build → merge under one-member target waves (D9). For `[unknown]` leftovers only: repeatable `--waive <slice>/<req>` with required `--reason` (see D17). No `--force`, no bulk/all-gaps waive, no separate `plan waive` / `approve` / `plan refine` verb. *(superseded by [RFC-86a](rfc-86a-gap-deferral.md): `--waive` / `--reason` argv and the coverage `unknown-waivers` field are deleted — coverage carries the effective `gap-policy`, and the disposition surface is `emery plan defer` + `--gap-policy <strict\|defer>`)* |
| `emery slice refine` / `build` / `merge`       | **Removed as operator verbs** (plan-centric surface cut). Refine / build / merge run only as execute-loop phases; `emery slice *` is read-only inspection (`list`, `validate`, `provenance`, `model show`). Abandon uses `emery plan drop`                                                                                                                                                                                                                                                                                                                                   |
| `/emery:plan`                                  | Unchanged contract: elicit → `emery plan author` → relay; stops after topology. Does **not** run refine                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `emery plan status`                            | After a fresh author, **resume** is `emery plan execute` / `/emery:execute` (D26). Next-actions may project `refine <slice>` / review-gaps while unrefined or gappy; then `plan execute` (build / merge) when Ready, or `plan execute --waive…` when skipping Ready (D22). Projects Authorized when a covering epoch exists — never an `approved` rung. *(RFC-86a: `plan execute --waive…` reads `plan defer` / `plan execute --gap-policy defer`; status also counts deferred debt)*                                                                                                                                                                                                                        |
| `emery plan advance` / `undo`                  | Expressed as claim / retraction facts instead of rewriting status fields; no plan-wide single-active-entry (D23). Claims never create authorization                                                                                                                                                                                                                                                                                                                                                                                                                           |


Exact error codes and event names belong in the [implementation notes](#appendix-implementation-notes); product behavior is above.

---



## Delivery


| Phase | Delivers                                                                                                                                                                                                                                                                                    | Operator should notice                                                                                                                                                                                                                                                                                                                                                             |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A** | Fact logs + computed status + exclusive per-slice claims (D23)                                                                                                                                                                                                                              | Same day-to-day flow for one operator; status still looks familiar; two journal writers may refine different slices without waiting on each other                                                                                                                                                                                                                                  |
| **B** | Recorded pins (D4 / D25) + one-member target waves (D9) + merge-time requirement ids; retire freeze-at-build **and** re-home `build/patch.yaml` into fact-substrate build records (D27); keep interim `apply` for [RFC-88](rfc-88-detached-changes.md)                                      | Safer parallel refine; drift diagnostics; build prepares from a recorded base pin; “built” / “merged” project from facts; merge still write-backs via interim `apply` until RFC-88                                                                                                                                                                                                 |
| **C** | Gap inventory (+ shared-lead presentation rollup), execute gap gate (no `approve` / no `plan refine` verb), typed `closed-plan` coverage with optional waivers *(RFC-86a: coverage now carries the effective `gap-policy`; the waiver field is deleted)*, shift-left practice via execute’s refine phase, RFC-88 `refine-under-epoch` alignment; sibling-series consistency pass (D20) | Public rhythm stays author → execute → archive; preferred path reviews topology then lets execute refine and park on gaps before build; starting execute opens the authorization epoch; multi-homed leads correlate in the gaps projection without changing the per-req gate. Phase C also updated shipped “nothing is stamped” / leftover approve-vocabulary prose when the durable epoch landed |


All [open review questions](#open-questions) are closed. Phase delivery above is a suggested split, not a locked schedule. Include the D20 sibling-review task (shipped docs + any leftover series RFCs) in Phase C plan work; the spine/`platform.md` authority and chronology refresh is already done.

---



## Acceptance (product-level)

1. Progress reported by the CLI is always computed from artifacts and facts — never read from a stored status field.
2. **Multi-slice, multi-writer (D23):** two people operating under distinct writer IDs can claim and refine *different* slices on copies of one change *at the same time*, combine via lossless fact-tree union, and both slices show as refined without journal conflicts. The same slice cannot be claimed by two writers (`slice-claim-conflict`). Plan-wide single-active-entry is gone — work does not wait for one slice to finish before another may start. Neither copy needs Git metadata.
3. Two slices refined against the same baseline merge without requirement-id collision; a drifted modification is rejected instead of overwritten. Identity maps live on `target.merge.wave-committed` (D9).
4. **Pins + build records (D25 / D27):** refine records `base.yaml` against RFC-87 `SnapshotId`s / wire `cid`s (and baseline-spec digest); build prepares from a recorded base pin rather than freezing the ambient product tree; build outcomes live in fact-substrate build records referenced by one-member waves (no `build/patch.yaml` authority); validate detects pin drift. Merge may still call interim `apply` until [RFC-88](rfc-88-detached-changes.md) deletes it.
5. **Shift-left practice (D12 / D14):** after authoring, `/emery:plan` / `plan author` do not run refine. Specs are minted by execute’s refine phase. The happy path reviews topology, runs execute so in-scope slices refine, deals with gaps before build spend continues, then re-runs execute with existing spec digests for build and merge only. RFC-88 `refine-under-epoch` remains an allowed authorization coverage for leaves that lack specs at execute start — gap policy still blocks build. The CLI ships no `emery plan refine` and no `emery slice refine` breakout.
6. **Gap gate:** `[unknown]` prevents build until fixed or explicitly waived per requirement on `plan execute` (including intent-only / N=1); `[conflict]` prevents build until resolved via override/sources and re-refine — **not** waiveable; `[divergence]` is listed but does not block (authority already chose); execute never silent-waives. Slice-status gates continue to refuse undealt work. *(superseded by [RFC-86a](rfc-86a-gap-deferral.md): the waive surface is deleted — an **open** gap blocks build, a **deferred** one leaves build scope; conflicts defer under the same exclusion while build-over stays forbidden)*
7. **No topology-approve verb:** refine is not gated on any slice-list / topology approval; post-author topology review is human-owned only (see D21).
8. **Authorization epoch (D6):** every execute path (CLI and `/emery:execute`) appends `plan.execute.started` **at execute start** with typed `closed-plan` coverage. When gap/status gates pass for build, privileged work proceeds under that epoch. Covered-artifact change (including re-refine) forces a fresh execute start (waivers on the stale epoch do not carry forward). The CLI ships no `emery plan approve` (or other plan `approve`) verb, no `approvals/` tree, and no projected `approved` rung. *(RFC-86a: deferrals are durable facts, not epoch payload — a fresh epoch re-supplies nothing)*
9. The same verbs and artifacts work with no remote (solo laptop) and with the change shared by lossless directory exchange (two people).
10. **Human review boundary:** execute’s gap gate enforces typed statuses only; prose review of `spec.md` is operator-owned and leaves no engine artifact (see D18).
11. **Shared-lead rollup:** when open findings share a contributing `(source, lead)`, the gaps projection annotates or groups those rows and suggests re-refine selectors; execute still fails or succeeds per requirement, and waivers remain `--waive <slice>/<req>` only (see D19). *(RFC-86a: waivers read deferrals — `plan defer <slice>/<req>`, still per-requirement)*
12. **Ready vs Authorized:** `plan status` projects Ready only when every in-scope slice is refined and the clean gap policy passes (no conflicts; zero open unknowns). Open unknowns keep the change out of Ready; starting execute with unknown-waivers reaches Authorized without ever being Ready. Waivers never contribute to the Ready projection (see D22). There is no `approved` milestone. *(RFC-86a: deferrals sit in the waiver seat — Ready counts open **and** deferred findings; debt-carrying plans reach build via Authorized)*
13. **In-scope membership (D24):** gaps projection, Ready, the execute gap gate, and unrefined next-actions all use the same filter — every entry currently on the plan whose slice is not dropped. Dropping a slice excludes it from that set without a second `plan remove`; removed entries are simply absent from the plan. Optional refine selectors do not redefine membership.
14. **Post-author resume (D26):** after a successful author (no refine yet), `plan author`’s hint and `plan status`’s resume name `emery plan execute` / `/emery:execute` — never a `plan refine` verb. Status may still project `refine <slice>` as a next-action phase label while resume stays execute.
15. **Target waves (D9):** serial execution opens one immutable one-member target wave before build. Merge names both build and closed-plan commit authorization, projects the member merged with one `target.merge.wave-committed` fact, and survives postflight failure without rollback; failures before that fact leave no merged projection.

---



## Open questions

All product / contract questions are **closed**. Closed items below are the decision trail for D12–D27 (and the substrate D1–D11 alignment), plus the sibling-prose / spine-chronology closure. Layout, `change approve`, public three-verb surface, `refine-under-epoch`, and wave ownership follow [RFC-88](rfc-88-detached-changes.md).

### Closed — decision trail

1. ~~**Sibling prose on execute / authorization (series / platform docs)**~~ **Closed — D6 / D17 / D20.** D6 and D17 are the in-force series authority rule: starting `plan execute` opens `plan.execute.started` with typed coverage; no separate `approve` verb; no projected `approved` rung; no silent auto-waive. Historical “running execute is the approval” / “auto-approve” wording means that execute gesture (with gap gates), not a second mint path or interactive waive. [platform.md](platform.md) is refreshed to separate **product-ownership order** (86 is step 1) from **landing chronology** (87 landed with stand-ins; 86 Phase B retires freeze / `patch.yaml`; workspaces do not wait on “completed 86”). Shipped-doc drift (`AGENTS.md`, `workflow.md`, CLI help) is ordinary Phase C delivery under D20 once the durable epoch lands — not a freeze blocker (completed in living-plan S22). **Settled against [RFC-88](rfc-88-detached-changes.md):** execute-only authorization, three-verb public surface, one-member waves against D9, `plan.execute.started` alignment. Rejected: cascade-rewriting every sibling before freezing; treating “auto-approve” as auto-waive; re-litigating whether private workspaces depend on finished 86.
2. ~~**What “in-scope” means for Ready / execute gap gate / gaps / unrefined next-actions**~~ **Closed — D24.** **In-scope** means every entry currently on the plan whose slice is **not dropped**. That single predicate feeds the gaps projection, Ready, the execute gap gate, and status’s unrefined next-actions — status, gaps, and execute must not invent divergent filters. `plan remove` deletes the entry (absent from the plan, so not in-scope). `slice drop` abandons the slice and excludes it from in-scope even if the plan row remains for audit — drop alone can shrink the change toward Ready/execute without a second remove. Optional refine selectors only narrow breakout work; they do not redefine membership. Merged is not a separate in-scope exclusion for these gates (execute’s covering epoch binds digests at execute-start time). Rejected: every plan entry including dropped; excluding already-merged from the membership noun; a stored scope / include-exclude list on the plan; claim- or last-selector-derived scope; phase-divergent filters; a soft “deferred / parked” exit ramp that is neither drop nor remove.
3. ~~**Concurrent refine claims / single-active-entry**~~ **Closed — D23.** A slice is a coherent, independently workable scope and is owned by **at most one journal writer** at a time. **Different** slices may be claimed and progressed by **separate writers at the same time**. Plan-wide “at most one `in-progress` entry” is **retired**; exclusivity is per slice via claim facts (D7), not per plan. Same-slice overlap fails closed (`slice-claim-conflict`). Parallel *swarm* work *inside* one slice remains a non-goal. A single `plan execute` process may still walk entries one-by-one, but that must not reimpose a plan-wide single-active gate that blocks another writer on a different slice. Rejected: keep single-active-entry for the whole plan; multi-person refine only via social convention without engine claims; defer multi-slice multi-writer execution to RFC-92/93.
4. ~~**How does plan-phase / shift-left refine start?**~~ **Closed — D14 (revisited against RFC-88).** `/emery:plan` / `emery plan author` stop after topology — they do **not** extract or synthesize. Specs are minted by the existing `emery slice refine` breakout. This RFC **adds no** `emery plan refine` verb — the public plan surface stays RFC-88’s `author → execute → archive`. Shift-left remains the preferred Phase C practice; RFC-88 `refine-under-epoch` remains allowed authorization coverage when specs are absent at execute start. Rejected: folding refine into `plan author`; inventing `emery plan refine` as a fourth public plan verb.
5. ~~**Should**~~ `[unknown]` ~~**block by default?**~~ **Closed — D15.** Always block **build**. `[unknown]` means insufficient information was available to the agent; the operator must provide it (or explicitly waive on execute) before build. Rejected: warn-only for intent-only / N=1; context-sensitive defaults keyed on source count or change shape.
6. ~~**Must each**~~ `[divergence]` ~~**be acknowledged?**~~ **Closed — D16.** Listed but allowed; informational only — no acknowledgment or decision required. Rejected: require per-divergence ack before execute.
7. ~~**Waiver UX**~~ **Closed — D17.** `emery plan execute` accepts repeatable `--waive <slice>/<req>` with required `--reason`. Only `[unknown]` may be waived; `[conflict]` is not waiveable. Waivers nest on the `plan.execute.started` coverage payload only — no separate `plan waive` or `plan approve` verb, no plan-/slice-wide or inventory-digest waive, no `--force` / `--allow-gaps`. One operator’s execute start is enough. Covered-artifact change invalidates the epoch and every waiver it carried. Rejected: bulk/all-gaps waive; waiving `[conflict]`; separate `plan waive` then execute; multi-operator waiver gating.
8. ~~**Human-only ambiguity**~~ **Closed — D18.** Prose review of `spec.md` alone — human operators own spec quality; the engine does not record or gate on that process. Rejected: optional operator checklist artifact; rolling advisory `kind: review` into gaps/execute; `--reviewed` / review attestation on execute; model-assisted spec-quality gate at execute time.
9. ~~**Shared leads across slices**~~ **Closed — D19.** Flat per-requirement inventory remains the gate authority. Gaps projection adds a **presentation rollup** for shared `(source, lead)`. Rejected: flat list only with no correlation aid; lead-wide or `--waive-lead` sugar; first-class lead-level gate; shared extract / shared Evidence for multi-homed leads.
10. ~~**Sibling docs**~~ **Closed — D20.** Consistency with later series RFCs and shipped operator docs is an **implementation task** (especially Phase C). Spine/`platform.md` authority and chronology are already refreshed (closed item 1). This RFC did **not** cascade-rewrite `AGENTS.md` / `workflow.md` / CLI help at decision-freeze time — those still said “nothing is stamped” until Phase C journaled the epoch; living-plan S22 completed that prose pass. RFC-88 itself is aligned on execute-only authorization, three-verb public surface, and waves owned here (D9). Rejected: freezing Phase C product design on a full sibling rewrite now.
11. ~~**Is a separate**~~ `plan approve` ~~**/ projected**~~ `approved` ~~**required?**~~ **Closed — D6.** **No.** Starting `plan execute` / `/emery:execute` opens `plan.execute.started` with typed coverage. Gap and slice-status gates refuse undealt work before build. Rejected: a mandatory `emery plan approve` verb; an `approvals/` artifact tree; naming the fact `plan.approved` or projecting an `approved` plan status; inferring authorization from the first `in-progress` claim. Auto-waive on execute remains rejected (D17). Prior draft text that required `plan approve`, forbade execute minting the covering fact, or projected an `Approved` milestone is superseded — the milestone is **Authorized**.
12. ~~**Topology-only approval: command shape and consumers**~~ **Closed — D21.** **No topology-approve CLI surface.** Human pause after author is the primary topology review seam. Rejected: hard-gating refine on topology approval; soft warn-only refine gate; optional `plan approve --slices`; a separate topology verb.
13. ~~**Ready vs Authorized when waivers exist**~~ **Closed — D22.** Ready means every in-scope slice is refined and the **clean** gap policy passes — computed from artifacts only. Authorized means a covering `plan.execute.started` epoch exists and may carry unknown-waivers. The waiver path therefore **skips** Ready; it never backfills Ready from waivers on that epoch. Clean Ready resumes at `plan execute` with an empty waive list (D6). Rejected: Ready includes “cleared or waived”; Ready = “executable” with unknowns still open; drop Ready as a milestone; dual Ready labels; projecting an `approved` rung instead of Authorized.
14. ~~**Pins before / after RFC-87 values**~~ **Closed — D25.** RFC-87 has landed: `SnapshotId`, `CodePatch`, and `prepare` / `capture` / `discard` are the value vocabulary; wire documents say `cid`. Pin *semantics* for this RFC are recorded snapshot (and baseline-spec digest) identities with typed drift detection. Phase B authors and consumes those pins; it does not wait on further RFC-87 design. Stand-in-retirement depth is D27. Rejected: inventing a pre-snapshot pin representation; blocking Phase B on a detached `change approve` as the only pin-close site; treating the interim freeze-at-build as permanent.
15. ~~**Keep or remove an Emery CLI**~~ `approve` ~~**verb for plan build?**~~ **Closed — D6.** **Remove / never ship.** Same closure as closed item 11.
16. ~~**Change home for this RFC vs RFC-88**~~ **Closed — D1.** Layout-neutral contract with concrete homes from [RFC-88](rfc-88-detached-changes.md): in-place `.emery/change/`; detached ordinary directory root; two roots (project + change). Private workspaces / snapshots stay under `$EMERY_HOME` (RFC-87). Phase A scopes against this contract; today’s flat `.emery/` + root `plan.yaml` is a pre-cut stand-in. Rejected: adapting facts only into today’s flat in-place layout as the enduring home; requiring a Git-backed detached repository before Phase A can start; leaving the concrete tree entirely undecided.
17. ~~**RFC-88**~~ `change approve` ~~**naming vs this RFC’s no-**~~`approve` ~~**plan surface**~~ **Closed — superseded by [RFC-88](rfc-88-detached-changes.md).** That RFC removes `emery change approve` (and `emery change open` / standalone discover): discovery and topology recording are the first internal phase of detached `plan author`; `plan execute` is the only authorization surface and records this RFC’s `plan.execute.started` (extended with detached digests). Pin close for detached sources is discovery / plan-author intake, not a change-approve gate (D25).
18. ~~**Post-author resume**~~ **Closed — D26.** After successful author, the operator-visible **resume** remains `emery plan execute` / `/emery:execute`. Topology/decomposition review stays human-owned prose in the hint. Status may still name `emery slice refine <slice>` as a *next-action* while in-scope slices are unrefined. Rejected: invent `emery plan refine` as the post-author resume; fold refine into author; require a topology-approve pause command.
19. ~~**Target waves and build vs commit authorization**~~ **Closed — D9.** This RFC owns the independently deployable one-member wave, `target.wave.opened`, atomic `target.merge.wave-committed` (identity maps included), and separate build-authorization vs closed-plan commit-authorization anchors. [RFC-88](rfc-88-detached-changes.md) attaches `cid`s and accepted-`cid` semantics; [RFC-92](rfc-92-concurrent-execution.md) may widen membership. Rejected: keeping only a per-slice merge fact with no wave shape; letting a claim authorize merge; requiring RFC-87 snapshot provider before the wave shape can land.
20. ~~**How completely must Phase B retire RFC-87 interim stand-ins?**~~ **Closed — D27.** Phase B **must** replace freeze-at-build with a recorded pin read **and** re-home `build/patch.yaml` into fact-substrate build records / wave manifests so “built” / “merged” project from facts only. Merge may still call interim `apply` from the recorded result CID / `CodePatch` — that deletion is [RFC-88](rfc-88-detached-changes.md)’s cut (and amends the older “RFC-89 deletes apply” story). Rejected: land pins + identity finalization + one-member waves while leaving `patch.yaml` as follow-on / dual build-outcome authority (forces a second re-home in RFC-88 and fights D2).



---



## Non-goals

- Changing Evidence schemas or the authority ranking (`intent` > `documentation` > `behaviour`).
- Automatically judging whether an `agreed` requirement is *good* (scenario depth, usefulness) — that stays human review and eval; no checklist artifact or execute-time attestation for prose review (see D18).
- Parallel swarm *inside* one slice (later concurrency RFC) — this RFC makes multi-slice, multi-journal-writer work safe via exclusive per-slice claims (D23); it does not fan out multiple code writers within a single slice.
- Multi-operator waiver / authorization countersign — one writer’s execute start (with any unknown-waivers) is sufficient; shared-directory collaboration stays social review of the fact log, not an engine four-eyes gate.
- Lead-wide waive, lead-level execute gate, or shared Evidence for multi-homed leads — correlation of shared-lead gaps is presentation-only (see D19); extract stays per-slice.
- Cascade-rewriting shipped operator docs and every later series RFC as part of freezing this RFC — sibling consistency beyond the spine/`platform.md` refresh remains a Phase C implementation review task (see D20 / closed item 1). RFC-88 is already aligned on execute-only authorization, the three-verb public surface, and waves owned here.
- A separate Emery CLI `approve` verb for plan/build, an `approvals/` artifact tree, or a projected `approved` rung — starting execute opens the authorization epoch; gap/status gates enforce readiness (see D6).
- An `emery plan refine` (or other plan-phase refine batch) verb — refine stays `slice refine`; public surface stays author → execute → archive (see D14 / D26).
- Folding extract / synthesize into `plan author` / `/emery:plan` — author stays topology-only; RFC-88 folds discovery into author, not refine (see D14).
- Silent auto-waive of gaps when execute starts — waivers stay explicit `--waive` on execute (see D17).
- Redefining the RFC-87 workspace contract, inventing a second snapshot identity, or owning project seal — those stay RFC-87 / RFC-89. Interim `apply` retirement is claimed by [RFC-88](rfc-88-detached-changes.md), not by this RFC. This RFC records pins, authorization epochs, and wave/result facts those seams consume. Wire name for tree identity is `cid`; Rust type may remain `SnapshotId`.
- Introducing a plan topology-approve verb, or reintroducing an RFC-88 `change approve` / open / discover command group — rejected by D21 and by [RFC-88](rfc-88-detached-changes.md); this RFC does not mint a substitute.
- Machine-gating refine on slice-list / topology approval — the post-author pause is human-owned (see D21).
- Ready that includes waived unknowns, or Ready that depends on a covering epoch — Ready stays clean-gap / artifact-only; waivers nest only under Authorized (see D22).
- Counting dropped slices as in-scope, or inventing a stored scope / park flag / claim-derived membership set — in-scope stays on the plan and not dropped, shared by gaps / Ready / execute (see D24).
- Mixing durable project state into the change home, or treating today’s flat `.emery/` + root `plan.yaml` layout as the enduring coordination home — fights D1 / RFC-88’s two-root split.
- Requiring Git metadata on the change home — fights D1’s version-control-neutral contract.
- Amending RFC-88 to forbid `refine-under-epoch` — this RFC adopts that authorization coverage; the gap policy still blocks build.
- Inferring authorization from a claim or first `in-progress` entry — confuses worker ownership with operator grant (see D6 / D7).

---



## Appendix: Prior art (short)

Settled patterns this RFC borrows, without adopting their full machinery:

- **Append-only operations, derived snapshots** (git-bug / Radicle COBs) — progress is replayed, not edited in place. Fact trees are version-control-neutral; we detect conflicting claims rather than CRDT-merging one slice.
- **Stable identity vs content identity** (Jujutsu) — slice-local requirement ids vs baseline numbers at merge.
- **Content-addressed work** (Bazel Remote Execution) — phases named by input digests (here: RFC-87 `SnapshotId` / `cid`); we record judgment outcomes instead of caching non-deterministic generations.
- **Authorization as a statement over digests** (in-toto / SLSA) — without cryptographic envelopes in this cut; the statement is `plan.execute.started`, not a separate approval artifact.
- **Spec review before implementation** — open issues tracked against the baseline, not discovered only while coding.

---



## Appendix: Rejected alternatives

- Hosted database as status authority — forces a server and a second mode for the laptop.
- Keep mutable status and synchronize it — harder than computing status from facts; creates two lifecycles.
- Treat shift-left as optional with no gap gate — optional review is what busy runs already skip; the gap policy is the machine seam.
- Fail refine on any gap tag — blocks useful incomplete Evidence; the build gap gate is the right seam.
- Auto-waive gaps when execute is invoked interactively — recreates invisible skip of the failures we care about (see D17).
- A mandatory or optional `emery plan approve` (or other plan `approve`) verb, an `approvals/` tree, or a projected `approved` rung — extra ceremony; operator review plus status/gap gates already cover the job; starting execute opens the authorization epoch (see D6).
- A separate authorization artifact beside `plan.execute.started` — duplicates one statement across two authorities without adding information.
- Inferring authorization from the first `in-progress` slice / claim — confuses a recoverable worker claim with an operator grant (see D6 / D7).
- Naming the fact `plan.approved` — reintroduces plan-approval vocabulary for an orchestration start event.
- Global requirement numbering at synthesize time — couples slices exactly when independence matters.
- A single journal file with transport-specific merge logic — brittle vs per-writer logs that union naturally.
- Fold refine into `plan author` / `/emery:plan` — spends extract/synthesis before the operator can re-cut the slice list; collapses topology review and spec review; fights RFC-88, which folds discovery into author, not refine (see D14).
- Invent `emery plan refine` as a fourth public plan verb — fights RFC-88’s `author → execute → archive` surface; N-many refine uses `slice refine` + status (see D14 / D26).
- Point post-author resume at a new `plan refine` verb — same fourth-verb problem; keep resume at `plan execute` and let status suggest `slice refine` as a next-action (see D26).
- Forbid RFC-88 `refine-under-epoch` so execute can never authorize refine — over-constrains the sibling authorization shape; adopt under-epoch coverage and keep the gap policy on **build** (see D12).
- Warn-only `[unknown]` for intent-only / N=1 (or any context-sensitive soften) — desk-testing shows generation invents missing detail and the error compounds through build and merge (see D15).
- Require per-`[divergence]` acknowledgment before execute — taxes a disagreement the authority hierarchy already resolved (see D16).
- Plan-/slice-wide, inventory-digest, or `--force` / `--allow-gaps` waive — recreates invisible skip as a one-flag off-switch (see D17).
- Waive `[conflict]` on execute — papers over an unresolved contradiction (see D17).
- Separate `emery plan waive` verb before execute — extra noun and waived-but-not-executed limbo once countersign is a non-goal; nest waivers on `plan.execute.started` instead (see D17).
- Multi-operator countersign on waivers or execute start — second lifecycle / mode bit; solo laptop is the primary deployment (see D17).
- Operator checklist artifact for spec review — extra durable file, digest/stale rules, and checkbox theater (see D18).
- Review attestation flags on execute (`--reviewed`, reviewer id) — records ceremony, not quality (see D18).
- Spec-quality advisories in gaps or execute-time blocking on `kind: review` findings — conflates human judgment with typed gap policy (see D18).
- Flat gap list with no shared-lead correlation — forces operators and agents to rediscover multi-home fan-out from `change.md` alone (see D19).
- Lead-wide / `--waive-lead` waive, or a lead-level gap gate — papers over per-requirement decisions (see D17, D19).
- Shared extract or shared Evidence for multi-homed leads — changes the per-slice extract contract (see D19).
- Hard- or soft-gating refine on topology / slice-list approval — second ceremony without improving the execute gate (see D21).
- Topology approve verb or `plan approve --slices` — extra plan-phase surface this RFC deliberately omits (see D21).
- Ready includes “cleared or waived” unknowns — circular with D17; Ready would depend on Authorized (see D22).
- Ready = “executable” while unknowns remain open — weakens the milestone (see D22).
- Drop Ready as a computed milestone — throws away the clean-path `plan execute` resume D6 relies on (see D22).
- Keep plan-wide “at most one `in-progress` entry” — denies that a slice is an independently workable scope (see D23).
- Multi-person refine only via social convention / non-overlapping selectors, without engine claims — Acceptance #2 becomes unenforceable (see D23).
- Defer multi-slice multi-writer claims to RFC-92/93 — leaves D7 and per-writer logs without the cardinality rule they exist for (see D10, D23).
- Count every plan entry including dropped as in-scope — drop cannot unblock Ready/execute without a second `plan remove` / amend (see D24).
- Exclude already-merged from the membership noun — smuggles execute into Ready (see D24).
- Stored scope / include-exclude list, claim- or last-selector-derived membership, phase-divergent filters, or a soft “deferred / parked” exit ramp — reintroduce editable or divergent membership (see D2, D24).
- Invent a second pin / revision wire format alongside RFC-87 `SnapshotId` / `cid`, or keep build-time ambient freeze / `build/patch.yaml` authority once recorded pins and fact-substrate build records exist — fights D4 / D25 / D27 and the landed workspace seam.
- Treat interim `apply` retirement or project seal as this RFC’s job — `apply` deletion is claimed by [RFC-88](rfc-88-detached-changes.md); project seal stays RFC-89; this RFC records the facts and pins those steps consume (D27 retires freeze + `patch.yaml` only).
- Leave `build/patch.yaml` as the build-outcome authority after Phase B pins and waves land — dual status (file vs facts); forces RFC-88 to re-home again (see D27).
- Keep a detached `change approve` (or other topology-approve) verb so plan can stay approve-less — RFC-88 has neither; execute is the sole authorization gesture in both RFCs.
- Per-slice merge facts with no wave shape — fights D9 and leaves RFC-88 without the accepted-`cid` transition it assumes.
- Requiring Git metadata on the change home — fights D1.

---



## Appendix: Implementation notes

For engine contributors. Not required to evaluate the product intent.

**Already landed (RFC-87 + Phases A–C) — consume, do not reinvent**

- Value vocabulary: `project::snapshot::{SnapshotId, CodePatch}` (`sha256:…` tree digest; code patch = `{ base, result, touched paths }`). Wire documents say `cid` for the same identity ([RFC-88](rfc-88-detached-changes.md)).
- Workspace capability: `project::seam::Workspaces` — `freeze` (refine pin authorship) / `prepare` / `capture` / `discard` / interim `apply`; the wasm-clean kernel in `project::workspace` runs in-guest over `wasi:blobstore` + `emery:exec-bits` (and in-process in the native provider) — see the deployment note in [RFC-87](rfc-87-working-trees.md).
- Build orchestration (`crates/slice/src/orchestrate/target.rs`) opens a one-member wave, prepares from the recorded `base.yaml` `target_base` pin, dispatches `target.build`, captures, and persists `builds/<digest>.yaml` (`BuildRecord`). No ambient freeze-at-build; no `build/patch.yaml` authority.
- Merge orchestration loads the `BuildRecord`, revalidates the wave, commits `target.merge.wave-committed`, and still calls interim `apply` for write-back (deleted by [RFC-88](rfc-88-detached-changes.md), not by this RFC). Do not reintroduce ambient checkout writes.

**Layout and writers**

- Projection kernel in `crates/project`: facts + artifact index → status, gap inventory (typed statuses only — no spec-quality rollup; D18), `ready`, Authorized (covering epoch present — never an `approved` label), and per-slice claim ownership. Shared **in-scope** filter (D24): plan entries whose slice is not dropped — used by gaps projection, `ready`, the execute gap gate, and unrefined next-actions. `ready` is clean-gap only (D22): all in-scope refined + no conflicts + zero open unknowns — never consult execute-waiver lists. Gap inventory rows stay `(slice, req, status)` and omit dropped slices; when projecting gaps, join open findings to contributing `(source, lead)` via plan bindings + Evidence/provenance and, when the same lead appears in more than one open finding, attach a presentation group plus suggested re-refine selectors (D19) — never a lead-level status field or waive expansion. Property-test: any interleaving of per-writer logs, same projection; fixture with unknown-waivers on a `plan.execute.started` epoch stays not-`ready` if unknowns remain on disk; many slices may show concurrent claims by different writers (D23); a dropped in-plan entry is excluded from in-scope so Ready/execute can proceed over remaining siblings without `plan remove`.
- Replace `.emery/journal.jsonl` with `events/<writer>.jsonl`; `emery journal show` merges the union.
- Remove stored plan-entry `status` and slice lifecycle fields; ladders survive only as projection labels. Retire plan-wide single-active-entry (`single_in_progress` / `next_eligible` blocking on any in-progress entry) — exclusivity is per-slice claim only (D23).
- No `approvals/` tree and no `plan approve` writer. `plan.execute.started` lives in the per-writer event log with typed `closed-plan` coverage (`plan-digest`, sorted per-leaf `existing` / `refine-under-epoch`, optional `unknown-waivers`, optional/required detached digests per RFC-88). `--waive` for a non-unknown or absent gap is `plan-gaps-unresolved` / a typed waive error (name TBD). *(RFC-86a: coverage carries `gap-policy` instead of `unknown-waivers`; `--waive` is deleted — invalid `plan defer` input is `plan-deferral-invalid`)*
- Concrete on-disk home follows D1: logical change tree; in-place writers target `.emery/change/` ([RFC-88](rfc-88-detached-changes.md)); detached writers target the change-directory root. Until RFC-88 lands that cut, the shipped stand-in remains today’s flat `.emery/` + root `plan.yaml` paths — do not invent a third enduring layout.

**Pins, waves, and identity**

- Plan authoring (in-place) or detached discovery / plan-author intake closes per-source `SnapshotId` / `cid` pins at plan scope when the source set is known (D4 / D25). There is no `change approve` pin-close site.
- Refine writes `base.yaml` by copying those source pins and adding the baseline-spec digest **before** extract — assembly, not the first writer of source snapshot ids.
- Build reads the recorded target-base pin and passes it to `prepare` (D27 landed). “Built” projects from `BuildRecord`s + wave facts only. Do not scope interim `apply` deletion into this RFC — [RFC-88](rfc-88-detached-changes.md) owns that cut.
- Before build, write `targets/<target>/waves/<digest>.yaml` naming the target (current project in the in-place cut), pinned base, ordered member set (one member), exact member inputs, dependency frontier, and build-authorization epoch; append `target.wave.opened`. Merge revalidates, names a `closed-plan` commit-authorization epoch (which may differ from build authorization; serial execution normally uses the same epoch), performs the deterministic merge, and appends one `target.merge.wave-committed` fact carrying every identity map. Postflight then appends succeeded or postflight-failed; failure is non-rollback and uses the existing acknowledgement stop. [RFC-88](rfc-88-detached-changes.md) attaches base/result `cid`s and accepted-`cid` semantics to this fact.
- Synthesis (Phase B landed): `IdAllocator` in `crates/slice/src/synthesis/project.rs` mints slice-scoped ids; each `MODIFIED` records a digest of the baseline requirement body it changed; wave commit assigns baseline `REQ-NNN`, records the id map on the committed fact, rejects drifted `MODIFIED` bases.
- Validate gains `slice-base-drifted` / `slice-evidence-stale` (review signals); merge blocks on `merge-base-drifted` where needed.

**Author, refine, and execute**

- No guest `plan refine` orchestration and no `plan refine` / `slice refine` CLI verbs (D14 + plan-centric surface cut). The refine orchestration (`slice::orchestrate`) runs only as an execute-loop phase; claims each slice exclusively (D23; in-scope = on the plan and not dropped — D24). Claiming slice B must not require slice A to be unclaimed or refined. Claims never create build/merge authority.
- Guest `plan execute` is the sole privileged-start surface: **at start** records `plan.execute.started` with typed `closed-plan` coverage (D6); accepts per-leaf existing spec digests **or** RFC-88 `refine-under-epoch` coverage; dispatches the refine phase under that epoch when needed; enforces gap + slice-status gates before **build**; then runs build → merge under one-member waves (D9). There is no `plan approve` or `plan refine` operation to call or wire. A single execute process may walk entries sequentially, but must not reimpose a plan-wide single-active gate that prevents another writer from claiming a different slice (D23). Build/merge continue to use the landed Workspaces capability.
- Author epilogue / fresh-plan `plan status` resume stays `emery plan execute` / `/emery:execute` (D26). Unrefined next-actions may project `refine <slice>` as a phase label; resume remains execute.
- Diagnostics (exit 2): `plan-gaps-unresolved`, `plan-epoch-stale` (covered-artifact change invalidated a prior epoch — re-run execute; replaces any draft `plan-approval-stale` naming), `plan-waiver-invalid` (waive of non-unknown / unknown id / missing reason), `slice-claim-conflict` (same slice, two writers — D23), plus staleness / merge-drift codes above. There is **no** `plan-approval-missing`, **no** `plan-approval-topology-only`, and refine must **not** emit a missing-topology-approval diagnostic. There is **no** diagnostic for “another slice is already claimed.” *(superseded by [RFC-86a](rfc-86a-gap-deferral.md): `plan-waiver-invalid` is deleted with the waiver surface; its deferral analog is `plan-deferral-invalid`)*
- New events: per-slice claim + existing refine events, `plan.execute.started` (unknown-waivers nested in coverage; refine-under-epoch as needed), claim/release, `fact.retracted`, `target.wave.opened`, `target.merge.wave-committed`, `target.merge.wave-succeeded`, `target.merge.wave-postflight-failed`. Do not invent a `plan.approved` event. Do not invent a `plan.refined` batch event that implies a `plan refine` verb. The committed wave fact carries identity maps — a separate `slice.merge.identity-mapped` event is unnecessary. *(RFC-86a: coverage nests `gap-policy`, not `unknown-waivers`; the deferral facts are `gap.deferred` / `gap.deferral-retracted`, and `target.merge.wave-committed` gains the deferred member-set snapshot)*



**Tests**

- Multi-writer fixtures in `crates/mock`: two writers claim different slices concurrently and both refine succeed after fact union; same-slice double-claim → `slice-claim-conflict`; base drift (shared-directory collaboration; not a waiver countersign gate). No fixture may require plan-wide single-active-entry. Neither directory needs Git metadata.
- Shift-left fixture: author → execute refine phase (per in-scope slice) → gaps → fix conflicts / waive unknowns on `plan execute` → build/merge under one-member waves; also cover execute with `refine-under-epoch` then refine-before-build; refuse conflict-waive and bulk-waive shapes; refuse build while gaps remain without matching `--waive` (D6); refine succeeds with no topology-approve ceremony (D21). Ready/Authorized fixture: refined + open unknowns projects not-Ready; `plan execute --waive…` reaches Authorized without Ready; clearing unknowns then projects Ready before a clean execute (D22). In-scope fixture: drop one of two refined slices; gaps / Ready / execute gap gate ignore the dropped entry while the sibling remains on the plan (D24). CLI surface fixtures: `emery plan approve`, `emery plan refine`, and `emery slice refine` / `build` / `merge` are absent (unknown subcommand / not wired). Post-author resume fixture: author hint and fresh-plan status resume name `emery plan execute` / `/emery:execute` (D26); next-action may project `refine <slice>`. Wave fixture: one-member wave opened before build; commit projects merged; postflight failure leaves merge accepted (D9).
- Pin / build-record fixture (Phase B, D27): refine writes `base.yaml`; build prepares from the recorded pin (no freeze); outcomes land in fact-substrate build records (no `build/patch.yaml` authority); “built” projects from those records + wave facts; validate reports drift when the baseline or a bound source moves; wave-commit identity map + drifted-`MODIFIED` rejection; merge may still call interim `apply` from the recorded result. Workspace round-trip coverage stays in `crates/project/tests/workspace.rs` (RFC-87) — do not duplicate it here.
- Multi-homed lead fixture: one `(source, lead)` bound into two refined slices with open unknowns; gaps projection groups/annotates both rows and suggests both slice selectors; execute still requires per-req waive or clearance (no lead-wide waive).
- Coverage fixture: `plan.execute.started` payload matches the closed `closed-plan` shape (existing / refine-under-epoch / optional waivers); changed spec requires a fresh epoch (`plan-epoch-stale`).
- Acceptance closeout (S23): focused crate integration coverage for the acceptance criteria is green (`emery-change`, `emery-project`, `emery-slice`, `emery-transport`, `emery-mock`); projection determinism and gap/execute/wave paths covered as crate integration tests.

**Sibling series (D20)**

- Spine/`platform.md` authority and chronology are already refreshed (closed item 1): D6 / D17 win; 87 landed; Phase B retired freeze / `patch.yaml`; remaining stand-in is merge-time `apply` (plus flat change home until RFC-88); ownership order ≠ landing chronology. Phase C / living-plan S22 reviewed remaining series RFCs and shipped operator docs (`AGENTS.md`, `workflow.md`, CLI help, skills, reference/tutorial prose) for leftover `plan approve` / projected `approved` / invented `plan refine` / “nothing is stamped” wording and aligned them with `plan.execute.started`. Living-plan S23 (product acceptance closeout) is complete — this RFC is Implemented. Follow [RFC-88](rfc-88-detached-changes.md) when reconciling siblings — it has no `change approve`, uses execute as the sole authorization surface, keeps the three-verb public workflow, defines `refine-under-epoch`, and routes accepted-`cid` through this RFC’s D9 waves (closed items 16–19).

**Hard cut**

- Pre-1.0: re-init over migration; no shims for old status fields, single journal, plan-wide single-active-entry, global synthesize-time ids, a plan `approve` verb, a plan `refine` verb, a projected `approved` rung, build-time ambient freeze once recorded pins exist, or `build/patch.yaml` as build-outcome authority once fact-substrate records exist (D27).

