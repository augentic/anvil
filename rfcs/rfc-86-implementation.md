# RFC-86 Change Facts — Multi-Session Implementation Plan

> **Authoritative living plan** for implementing [RFC-86](rfc-86-change-facts.md).
> Later sessions open this file first, work only the single `next` session, then update status / discovery before finishing.
> Product contract: [`rfc-86-change-facts.md`](rfc-86-change-facts.md). Series sequencing: [`platform.md`](platform.md) § RFC-86 ∥ RFC-87.

## Non-negotiable operating rules

1. **Strict sequence.** Never start session *N+1* until session *N* is `done` and the operator has committed (or explicitly waived a commit for a docs-only/no-op).
2. **One session per agent run.** Load that session’s section, status table, Context pack code files, and every RFC subsection the session cites; do not pre-implement later sessions or substitute the digest for cited RFC text.
3. **No automated git staging/commits/pushes.** End every session with a **Commit handoff** block for the operator. Do not run `git add` / `git commit` / `git push`.
4. **Keep CI green per session.** Prefer `cargo make ci` when affordable; at minimum the session’s named gate must pass.
5. **Hard cut, no shims** (RFC D11). Temporary dual-write is allowed only inside a session (or as an explicit bridge removed by the next session). Do not leave “read old status fields forever” code.
6. **Layout stand-in (D1).** Keep today’s flat `.emery/` + root `plan.yaml` / `discovery.md` / `change.md` until RFC-88’s two-root cut. Per-actor logs live under `.emery/events/<actor>.jsonl` in this cut (not `.emery/change/events/` yet).
7. **Do not delete interim `apply`** (D27 / RFC-88). Phase B retires freeze + `build/patch.yaml` authority only.
8. **Single PR at the end** covering the full RFC. No intermediate PRs.

## Status protocol

Statuses: `pending` → `next` (exactly one) → `in_progress` → `done` | `blocked`.

When discovery forces a plan change: edit the affected future session in place, append a discovery-log entry, and keep earlier `done` sessions untouched unless a fixup session is inserted as `Sxx.1`.

### Status table

| ID | Title | Status | Gate | Commit |
| -- | ----- | ------ | ---- | ------ |
| S0 | Materialize living plan | `done` | file exists; S0=done, S1=next | `docs(rfc-86): add multi-session implementation plan` |
| S1 | Per-actor event log I/O | `done` | `cargo nextest run -p emery-project` (journal focus); lint if cheap | `feat(project): per-actor event logs with union read` |
| S2 | Retarget emit + journal show | `done` | journal/show + emit tests; `rg journal\\.jsonl` | `feat(project): route journal emit and show through per-actor logs` |
| S3 | Claim/release/retract + claim kernel | `done` | claim conflict + concurrent different-slice tests | `feat(project): exclusive per-slice claim facts` |
| S4 | In-scope predicate; retire single-active-entry | `done` | plan validate / advance tests | `refactor(plan): retire single-active-entry; add in-scope predicate` |
| S5 | Fact-based plan status projection | `done` | `crates/change/tests/plan_status.rs` | `feat(plan): compute plan status from artifacts and facts` |
| S6 | Writers stop mutating stored status ladders | `done` | full_loop / refine / build / merge tests | `refactor(workflow): express advance and phase progress as facts` |
| S7 | Delete stored status/lifecycle fields | `done` | `cargo make check` | `refactor!: remove stored plan-entry and slice lifecycle status fields` |
| S8 | Multi-actor claim/union fixtures | `done` | new mock/change integration tests | `test(mock): multi-actor claim and fact-union fixtures for RFC-86` |
| S9 | Source cid pins at plan author | `done` | author/reconciliation pin assertions | `feat(change): record source cid pins at plan author` |
| S10 | Refine writes base.yaml | `done` | refine integration test asserts `base.yaml` | `feat(slice): write refine-time base.yaml pins` |
| S11 | Slice-local REQ ids + MODIFIED digests | `done` | synthesis local-id + modified-digest tests | `feat(slice): mint slice-local requirement ids until wave commit` |
| S12 | One-member wave manifests + target.wave.opened | `done` | wave write/load tests | `feat(project): one-member target wave manifests and open fact` |
| S13 | Build from pins; retire freeze + patch.yaml | `done` | slice build tests + `cargo make check` affected | `feat(slice): build from recorded pins into fact-substrate records` |
| S14 | Wave commit + identity maps; keep apply | `done` | merge identity + postflight tests | `feat(slice): commit one-member waves with requirement identity maps` |
| S15 | Pin drift diagnostics + Phase B fixtures | `done` | new fixtures; no `patch.yaml` authority | `test(slice): pin drift and wave-commit fixtures for RFC-86 Phase B` |
| S16 | Gap inventory + shared-lead rollup | `done` | projection tests incl. multi-homed lead | `feat(plan): typed gap inventory with shared-lead rollup` |
| S17 | Ready/Authorized + status next-actions | `done` | plan_status + Ready/Authorized fixtures | `feat(plan): project Ready and Authorized milestones` |
| S18 | plan.execute.started + --waive CLI | `done` | execute-start fact + waive validation | `feat(change): record plan.execute.started authorization epoch` |
| S19 | Execute gap gate before build | `done` | gap-gate integration tests | `feat(change): enforce gap policy before build under execute epoch` |
| S20 | refine-under-epoch + shift-left fixtures | `done` | both fixtures; CLI absence assertions | `test(change): shift-left and refine-under-epoch execute fixtures` |
| S21 | Remaining Phase C acceptance fixtures | `done` | listed fixtures green | `test(change): RFC-86 Phase C acceptance fixtures` |
| S22 | D20 sibling docs + operator prose | `done` | `cargo make links`; prose checklist | `docs: align operator prose with RFC-86 change facts` |
| S23 | Full cargo make ci + acceptance closeout | `next` | `cargo make ci` | `chore: RFC-86 implementation acceptance closeout` |
| S24 | Open single full-RFC implementation PR | `pending` | PR URL returned | none (operator-directed) |

### Discovery log (append-only)

```text
### 2026-08-06 — S0
- Finding: Materialized this living plan from the Cursor seed plan; no prior `rfcs/rfc-86-implementation.md` existed.
- Plan change: none (initial write). Status: S0=done, S1=next.

### 2026-08-06 — plan wording
- Finding: Session template + risk controls could be read as forbidding RFC reads, leaving only the living-plan digest — too thin for wire-accurate implementation.
- Plan change: Session template now requires opening every cited D# / Acceptance / appendix subsection; digest marked orientation-only; RFC wins on conflict. Risk controls updated to match.

### 2026-08-06 — S1
- Finding: Primary write path is `.emery/events/<actor>.jsonl` with wire `actor` + 1-based `sequence`; default actor id is `local` when `EMERY_ACTOR` is unset. Existing `journal show` / identity readers still consume the legacy single file, so append dual-writes stamped lines to `.emery/journal.jsonl` as an explicit S1→S2 bridge.
- Plan change: none beyond status (S1=done, S2=next). S2 owns deleting the dual-write, retargeting show/emit/read_recent to `read_union`, and updating tests that open `journal.jsonl`.

### 2026-08-06 — S2
- Finding: Dual-write and legacy `path` / single-file `read` / reverse-tail helpers removed. `show`, `read_recent`, and `scan_recent` now load `read_union`. Emit already went through `append_one` — only the bridge write needed deleting. Remaining `journal.jsonl` mentions in Rust are negative assertions (file must not exist); `docs/reference/*` layout trees still name the old file and are deferred to S22 with the broader operator-prose pass.
- Plan change: none beyond status (S2=done, S3=next).

### 2026-08-06 — S3
- Finding: Appendix names `fact.retracted` explicitly and lists claim/release without dotted ids; wired as `slice.claimed` / `slice.released` / `fact.retracted` (payloads `slice-name`; retract targets `{ actor, sequence }`). Pure kernel at `journal::claim` projects live ownership from the union (release by owner; retracted facts omitted via fixed-point), and `claim` / `ensure_claimable` refuse same-slice second actors with `slice-claim-conflict` (exit 2). Not yet wired into plan advance/undo (S6) or refine (later).
- Plan change: none beyond status (S3=done, S4=next).

### 2026-08-06 — S4
- Finding: Removed `single_in_progress` / `multiple-in-progress` validate findings and the `next_eligible` / `advance` “any in-progress blocks” gate. `advance` now starts the next eligible pending even when siblings are already `in-progress`, and resumes an existing in-progress only when nothing pending is eligible. Shared `plan::in_scope` (on plan ∧ not dropped) lands for later gaps/Ready/execute consumers. Execute mid-slice resume now follows `status.active` rather than calling `advance` (so one process still walks one-by-one without reimposing the plan-wide gate). Operator docs that still mention “at most one in-progress” stay for S22.
- Plan change: none beyond status (S4=done, S5=next).

### 2026-08-06 — S5
- Finding: `plan status` / `resolve_entry` now project ladder labels and awaited phase from the fact union + slice artifacts only — stored `Entry.status` / `LifecycleStatus` are not read. Ladder: `plan.entry.advanced` / undo / live claim → in-progress; `slice.archive.created` / `slice.merge.postflight-failed` → done. Phase: `model.yaml`/`spec.md` + refine success facts → refined; `build/patch.yaml` + `slice.build.succeeded` → built; `dropped_at` → slice-dropped. Failure / merge-incomplete overlays stay active-window scoped; pending preview skips the journal overlay so stale same-name events cannot classify. Writers may still stamp old status fields until S6. Event collection unions the plan root with each materialised workspace-slot journal.
- Plan change: none beyond status (S5=done, S6=next).

### 2026-08-06 — S6
- Finding: `plan advance` claims the eligible slice (`slice.claimed` + `plan.entry.advanced`) from fact-projected ladders and no longer rewrites `plan.yaml` `Entry.status`. `plan undo` walks projected rungs via `fact.retracted` (+ label `plan.transition.undone`) without status writes; ladder / active-window projection skip retracted lines. Refine/build stamp phase timestamps + facts only (leave stored `LifecycleStatus` untouched); merge gates on `build/patch.yaml` / projected in-progress, emits `slice.archive.created` for `done`, and dropped the `stamp_plan_entry_done` path. `in_scope` and archive outstanding-work now key off `dropped_at` / projected ladders. Stored status/lifecycle fields remain on disk for S7's hard cut.
- Plan change: none beyond status (S6=done, S7=next).

### 2026-08-06 — S7
- Finding: Hard-cut removed `Entry.status` from `plan.yaml` and `SliceMetadata.status` from `metadata.yaml`. `Status` / `LifecycleStatus` remain as projection labels only (`project_ladders` / `LifecycleStatus::project` from artifacts + timestamps). Deleted `plan/transitions.rs` (stored-status writers). `is_replaceable` / `plan remove` / `propose_from` now gate on projected ladders. Validate dropped `missing-slice-dir-for-in-progress` (it read stored status). Operator docs that still show `status: pending` / `metadata.status` stay for S22.
- Plan change: none beyond status (S7=done, S8=next).

### 2026-08-07 — S8
- Finding: Acceptance #2 fixtures land in `crates/mock/tests/multi_actor.rs` (mock↔change integration via cyclic dev-deps). Two authored copies claim/refine disjoint slices (`login-flow` / `password-reset`) under `alice`/`bob`, then union bob's per-actor log + slice tree; both project `Refined` with live dual ownership. Same-slice second claim → `slice-claim-conflict`. Sibling `plan advance` while a peer claim is live proves no plan-wide single-active-entry. Claims use `append_for` (fixture surface); refine stamps via `EMERY_ACTOR`. Base-drift injection stays S15. Added `login_flow_synthesis` / `password_reset_synthesis` to `mock::answers` (claim ids must match mock evidence, e.g. `login.flow`).
- Plan change: none beyond status (S8=done, S9=next).

### 2026-08-07 — S9
- Finding: Source pins close after survey during `plan author`: `SourceBinding.cid` (`SnapshotId` on wire as `cid`) is stamped on every `plan.yaml.sources.<key>` via `project::plan::close_source_pins`. Value bindings digest a one-file tree entry `content`; path files use basename; path dirs walk with the same `.git`/`.emery` ignore policy as the snapshot store (digest-only — store population deferred to prepare consumers). Path digests match `Store::snapshot` for directory trees. Exact YAML home is the plan source binding (plan-adjacent), not a separate pin file.
- Plan change: none beyond status (S9=done, S10=next).

### 2026-08-07 — S10
- Finding: Refine writes `.emery/slices/<slice>/base.yaml` before extract via `slice::Base::assemble` — copies closed plan `cid`s for every entry binding plus `baseline-spec` (`dir_cid` of the ThreeWayMerge baseline `specs/` tree; missing/empty → `empty_cid`). Shape: `{ sources: { <key>: sha256:… }, baseline-spec: sha256:… }`. Drift diagnostics stay S15.
- Plan change: none beyond status (S10=done, S11=next).

### 2026-08-07 — S11
- Finding: Synthesis `IdAllocator` is slice-local (`REQ-001..N` in declaration order; baseline numbers ignored). `baseline-id` stays on the projected model for MODIFIED rows; kernel stamps `baseline-digest: sha256:…` over the baseline requirement body from `BaselineIndex` (index now stores bodies, not titles). Render classifies MODIFIED via `baseline_id.is_some()` (projected ids no longer match baseline). Wave-commit remapping / drifted-MODIFIED rejection remain S14/S15. Embedded synthesis prompts updated to match.
- Plan change: none beyond status (S11=done, S12=next).

### 2026-08-07 — S12
- Finding: One-member wave types land in `project::wave` at `.emery/targets/<target>/waves/<bare-hex>.yaml` (stand-in under flat `.emery/`). Manifest fields: `target`, `base` (`SnapshotId`), `members[]` (`slice` + `inputs.spec`), `depends-on`, `build-authorization` (`{ actor, sequence }` epoch ref). `Wave::open` write-onces the content-addressed YAML and appends `target.wave.opened` (`target`, `digest`, `slice-name`). Not wired into build orchestration (S13). `cli-contract` event table updated for the new id; broader workflow/AGENTS event prose stays S22.
- Plan change: none beyond status (S12=done, S13=next).

### 2026-08-07 — S13
- Finding: Refine freezes the product tree into `base.yaml` `target-base` (alongside sources + baseline-spec). Build deletes ambient `seam.freeze()`: loads that pin, opens a one-member wave (`inputs.spec` = `dir_cid(specs/)`), `prepare`s from the pin, captures into content-addressed `.emery/slices/<slice>/builds/<digest>.yaml` (`base`/`result`/`touched`/`wave`/`report`), and never writes `build/patch.yaml`. “Built” projects from `BuildRecord::present` (+ facts). Merge loads the newest build record for interim `apply`. Wave `build-authorization` uses `{ actor, sequence: 0 }` until S18’s `plan.execute.started`. Mock fail-build markers read from `project_root` (control-plane), matching merge-gate markers. Also fixed pre-existing slice rustdoc private-link warnings that blocked `cargo make check`.
- Plan change: none beyond status (S13=done, S14=next).

### 2026-08-07 — S14
- Finding: Merge revalidates the build record's one-member wave (`Wave::load_for_merge`), finalizes requirement identity (`merge::identity::finalize` — MODIFIED keeps `baseline-id` after `merge-base-drifted` digest check; ADDED takes next free baseline `REQ-NNN`), rewrites slice specs/`model.yaml`/`tasks.md`, then deterministic commit → strict `target.merge.wave-committed` (identity maps + commit-authorization reusing wave build-authorization until S18) → postflight `target.merge.wave-succeeded` / `target.merge.wave-postflight-failed`. Hard-cut replaced `slice.merge.postflight-failed`. Interim `apply` still runs after successful postflight. Failures before wave-committed leave no merged projection. Ladder/sticky-debt/undo readers updated; `cli-contract` event table lists the new ids.
- Plan change: none beyond status (S14=done, S15=next).

### 2026-08-07 — S15
- Finding: Validate folds non-blocking review signals `slice-base-drifted` (baseline-spec pin ≠ live `.emery/specs/` digest) and `slice-evidence-stale` (bound source pin ≠ live `source_cid`) when `base.yaml` exists — Adapter-path advisories like synopsis-thin, so validate still PASSes. Exported `project::plan::source_cid` for live recompute. Phase B fixtures: `change/tests/pin_build_record.rs` (refine→build→merge asserts base.yaml + BuildRecord + wave facts + interim apply + no `patch.yaml` authority; drift injection for both review codes); `slice/tests/merge_identity.rs` adds two-slice ADDED collision-free wave commit (Acceptance #3). `merge-base-drifted` already landed in S14.
- Plan change: none beyond status (S15=done, S16=next).

### 2026-08-07 — S16
- Finding: Gap inventory lands in `project::plan::gaps` — pure projection of in-scope `(slice, req, status)` for `unknown|conflict|divergence` from `model.yaml` (else `specs/*/spec.md`), joined to contributing `(source, lead)` via plan bindings. Shared-lead presentation rollup annotates multi-homed rows and suggests re-refine selectors (D19); dropped slices excluded via `in_scope` (D24). Surfaced on `plan status` (`StatusBody.gaps`) and a dedicated read-only `emery plan gaps` verb. Execute gap gate / Ready stay later sessions.
- Plan change: none beyond status (S16=done, S17=next).

### 2026-08-07 — S17
- Finding: `StatusBody` now carries `ready` / `authorized` (D22). Ready = all in-scope refined + clean gaps (no conflict/unknown; divergence allowed); Authorized = any `plan.execute.started` in the fact union (coverage wire types landed so fixtures can stamp the fact; execute writer + `--waive` stay S18). Refined-but-not-Ready rewrites Build → `review-gaps`; resume suggests `slice refine` for conflicts or `plan execute --waive <slice>/<req> --reason …` for unknowns; post-author / Ready resume stays `/emery:execute` (D26). Never projects `approved`. Execute maps `ReviewGaps` → build until S19's gap gate.
- Plan change: none beyond status (S17=done, S18=next).

### 2026-08-07 — S18
- Finding: `plan execute` validates `--waive` / `--reason` before `guest.lock`, then appends `plan.execute.started` with `closed-plan` coverage (`plan-digest` = sha256 of `plan.yaml` bytes; per in-scope leaf `existing{dir_cid(specs)}` when model/spec artifacts exist else `refine-under-epoch`; optional `unknown-waivers`). CLI: repeatable `--waive <slice>/<req>` + required `--reason` (one reason applied to every selector). `plan-waiver-invalid` for orphan reason, missing reason, absent gap, or non-unknown (incl. conflict). Wave `build-authorization` now binds the newest covering epoch (sequence `0` only for breakout builds with no epoch). Gap gate before build stays S19 (`plan-epoch-stale` deferred). No `plan approve` / `plan refine`.
- Plan change: none beyond status (S18=done, S19=next).

### 2026-08-07 — S19
- Finding: Execute calls `enforce_before_build` immediately before `slice::orchestrate::build` (hard `Err`, not `plan-execute-stopped`). Policy is per leaf being built: `[conflict]` → `plan-gaps-unresolved` (never waiveable); `[unknown]` blocks unless a matching waiver nests on the newest covering `plan.execute.started`; `[divergence]` listed on failure detail / inventory but allowed. Epoch freshness: `plan-digest` + every in-scope `existing` spec digest must match live artifacts (`plan-epoch-stale`); `refine-under-epoch` leaves skip digest compare. Failure detail renders the full gap inventory. Public `change::orchestrate::enforce_before_build` supports the stale fixture without mid-loop mutation (execute always restamps a fresh epoch at start). ReviewGaps still maps to Build so waived unknowns can proceed.
- Plan change: none beyond status (S19=done, S20=next).

### 2026-08-07 — S20
- Finding: Acceptance #5 fixtures in `crates/change/tests/shift_left.rs`. Preferred path: author is topology-only (no model/base/synthesize facts) → `slice refine` → clean gaps / Ready → execute stamps `existing` coverage and runs Build+Merge only. Under-epoch: execute over unspec'd leaf stamps `refine-under-epoch`, phases Refine→Build→Merge (gap gate before build). CLI absence: `emery plan approve` / `emery plan refine` exit 2 and are absent from `plan --help` (`transport/tests/router.rs`); bumped stale http_parity route count 30→31 (plan gaps).
- Plan change: none beyond status (S20=done, S21=next).

### 2026-08-07 — S21
- Finding: Remaining Acceptance #8–15 fixtures in `crates/change/tests/phase_c_acceptance.rs`: in-scope drop excludes gaps/Ready/gap-gate without `plan remove` (D24; membership via live `dropped_at`, matching prior fixtures); waive skips Ready then clearing unknowns projects Ready (D22); `plan.execute.started` wire shape asserts `kind: closed-plan` + kebab `plan-digest` / Existing specs + stale after spec change; post-author hint + status resume name execute (D26); under-execute wave.opened before build.succeeded + postflight-failed keeps wave-committed (D9). Coverage wire fix: internally-tagged `ClosedPlanCoverage` was emitting snake_case `plan_digest` — explicit `#[serde(rename = "plan-digest")]` / `unknown-waivers` so the journal matches the RFC appendix example.
- Plan change: none beyond status (S21=done, S22=next).

### 2026-08-07 — S22
- Finding: D20 sibling-doc pass. Replaced shipped “nothing is stamped” / “running execute is the approval” / stored `status:` / `journal.jsonl` / single-active-entry / projected `(approved)` prose with `plan.execute.started` authorization epoch, gap gates, per-slice claims, computed ladders, pins/waves, and `.emery/events/<actor>.jsonl`. Touched `AGENTS.md`, `workflow.md`, `cli-contract.md`, CLI help (`routes.rs`), author hint, skills/plugin README, reference (lifecycle, directory-layout, cli/plan, quick-reference, configuration, glossary), tutorials/orientation/how-to/explanation, and RFC-86 status + appendix D20 notes. Series RFCs: `platform.md` already aligned; no leftover `plan approve` / invented `plan refine` in rfc-87..92 beyond decision-trail history. Did not cascade-rewrite RFC-88 design. Gate: `cargo make links` green.
- Plan change: none beyond status (S22=done, S23=next).
```

### Session template (copy into each agent prompt)

```text
You are implementing RFC-86 session <ID> only.
1. Read rfcs/rfc-86-implementation.md — Status table + this session’s section + Discovery log.
2. Open every RFC decision / Acceptance item / appendix subsection this session cites
   (e.g. “RFC D3”, “Acceptance #2”, “appendix wire ids”) in rfcs/rfc-86-change-facts.md.
   Read those subsections for normative detail; do not skim-substitute from the living-plan digest alone.
3. Read the Context pack source files listed for this session (code paths). Do not pre-load
   unrelated crates or later sessions.
4. Implement Deliverables to match the cited RFC wording. If the digest and RFC disagree, the RFC wins.
5. Run the named Gate.
6. Update Status table (this session → done; next → next) and Discovery log if needed.
7. Output a Commit handoff (files + suggested message). Do NOT git add/commit/push.
```

## Architecture target (what “done” means)

```mermaid
flowchart LR
  artifacts[Artifacts plan.yaml specs pins waves]
  facts[PerActor events logs]
  project[Projection kernel]
  status[plan status / gaps]
  execute[plan execute]
  artifacts --> project
  facts --> project
  project --> status
  execute -->|"plan.execute.started"| facts
  execute -->|gap gate before build| project
```

RFC phases map to sessions:

- **Phase A** — facts + computed status + claims (`crates/project`, then wire-through) — S1–S8
- **Phase B** — pins, waves, REQ identity, retire freeze/`patch.yaml` (`crates/slice` + pin writers) — S9–S15
- **Phase C** — epoch, gap gate, waivers, shift-left fixtures, D20 docs (`crates/change` + prose) — S16–S23

This plan is the **session schedule and progress tracker**, not a substitute product contract. Normative decisions, wire ids, coverage shapes, and acceptance criteria live in [`rfc-86-change-facts.md`](rfc-86-change-facts.md) (D1–D27, Acceptance #1–15, Appendix). Each session’s Context line cites the decisions it owns — open those subsections before coding. The condensed digest at the bottom of this file is orientation only; **RFC wording wins on conflict.**

---

## Sessions

### S0 — Materialize living plan

- **Context:** Cursor seed plan; RFC Delivery + Appendix Implementation notes.
- **Deliverables:** Write this file containing the full status table, discovery log, operating rules, and every session below (verbatim enough that later agents need only that file + the RFC).
- **Gate:** file exists; status shows `S0=done`, `S1=next`.
- **Commit handoff:** add `rfcs/rfc-86-implementation.md`. Message: `docs(rfc-86): add multi-session implementation plan`.

---

### Phase A — Fact substrate and computed status

#### S1 — Per-actor event log I/O

- **Context:** [`crates/project/src/journal.rs`](../crates/project/src/journal.rs), [`journal/append.rs`](../crates/project/src/journal/append.rs), [`journal/event.rs`](../crates/project/src/journal/event.rs), RFC D3 / layout stand-in.
- **Deliverables:**
  - Extend wire `Event` with `actor` + `sequence` (monotonic per actor file).
  - Store at `.emery/events/<actor>.jsonl`; actor id = `EMERY_ACTOR` else a stable local default (document the default in module docs).
  - Union reader (all actor files, ordered by `(timestamp, actor, sequence)`); append writes only the calling actor’s file.
  - Crate integration tests for append + union + sequence.
  - Keep old `journal.jsonl` path compiling only if still referenced; prefer introducing the new API as the primary surface this session.
- **Gate:** `cargo nextest run -p emery-project` (or package name in tree) focused on new journal tests; `cargo make lint` if cheap.
- **Commit:** `feat(project): per-actor event logs with union read`.

#### S2 — Retarget emit + `journal show`

- **Context:** [`journal/emit.rs`](../crates/project/src/journal/emit.rs), [`journal/handlers.rs`](../crates/project/src/journal/handlers.rs), all `journal::emit*` / `append` call sites, transport CLI for `journal show`.
- **Deliverables:** Every write goes to per-actor logs; `emery journal show` merges the union; delete single-file `.emery/journal.jsonl` authority; update tests that open `journal.jsonl`.
- **Gate:** journal/show + emit integration tests green; spot-check `rg journal\\.jsonl`.
- **Commit:** `feat(project): route journal emit and show through per-actor logs`.

#### S3 — Claim / release / retract events + claim kernel

- **Context:** [`journal/event.rs`](../crates/project/src/journal/event.rs), RFC D7 / D23; plan advance/undo surfaces.
- **Deliverables:**
  - New events: slice claim, release, `fact.retracted` (exact wire ids per RFC appendix).
  - Pure claim kernel: at most one live claim per slice; `slice-claim-conflict` on same-slice second actor; different slices may be claimed concurrently.
  - Unit-cheap private matrix OK; public behavior via `crates/project/tests/`.
- **Gate:** claim conflict + concurrent different-slice tests.
- **Commit:** `feat(project): exclusive per-slice claim facts`.

#### S4 — In-scope predicate + retire plan-wide single-active-entry

- **Context:** [`plan/validate.rs`](../crates/project/src/plan/validate.rs) `single_in_progress`, [`plan/advance.rs`](../crates/project/src/plan/advance.rs) `next_eligible`, RFC D23 / D24.
- **Deliverables:** Shared `in_scope(plan, slice_meta) = on plan && not dropped`; remove plan-wide single-active gate from validate/advance; exclusivity is claims only. Update validate diagnostics/tests.
- **Gate:** plan validate / advance tests updated and green.
- **Commit:** `refactor(plan): retire single-active-entry; add in-scope predicate`.

#### S5 — Projection kernel reads facts (stop trusting stored status for status CLI)

- **Context:** [`plan/status/project.rs`](../crates/project/src/plan/status/project.rs), [`plan/execution.rs`](../crates/project/src/plan/execution.rs), RFC D2.
- **Deliverables:** Rewrite `plan status` / `resolve_entry` to compute progress from artifacts + fact union (claims, refine/build/merge facts). May still *write* old status fields for one more session, but projection must not *read* them. Project familiar next-actions where possible.
- **Gate:** [`crates/change/tests/plan_status.rs`](../crates/change/tests/plan_status.rs) green under fact-based projection.
- **Commit:** `feat(plan): compute plan status from artifacts and facts`.

#### S6 — Writers stop mutating stored status ladders

- **Context:** advance/undo handlers, refine/build/merge orchestrations, [`slice/lifecycle.rs`](../crates/project/src/slice/lifecycle.rs).
- **Deliverables:** `plan advance` / `undo` become claim/retract facts; refine/build/merge append phase facts only — no `Entry.status` / `LifecycleStatus` transitions as authority. Bridge: if fields still exist on disk, leave them unused or stop serializing writes.
- **Gate:** full_loop / refine / build / merge integration tests adjusted.
- **Commit:** `refactor(workflow): express advance and phase progress as facts`.

#### S7 — Delete stored status fields (hard cut)

- **Context:** [`plan/model/state.rs`](../crates/project/src/plan/model/state.rs) `Entry.status`, slice `metadata.yaml` lifecycle field, all serde/tests/probe assertions.
- **Deliverables:** Remove stored status / lifecycle-as-authority from types and on-disk shapes; ladders remain projection labels only. Fix compile breaks across workspace. Update goldens/fixtures.
- **Gate:** `cargo make check` (fmt/lint/test/doc subset) green.
- **Commit:** `refactor!: remove stored plan-entry and slice lifecycle status fields`.

#### S8 — Phase A multi-actor fixtures

- **Context:** [`crates/mock`](../crates/mock), RFC Acceptance #2; platform.md slack absorber note.
- **Deliverables:** Two-actor fixtures: disjoint slice claims + refine after fact-tree union; same-slice → `slice-claim-conflict`; no Git metadata required; no plan-wide single-active assumption.
- **Gate:** new mock/change integration tests green.
- **Commit:** `test(mock): multi-actor claim and fact-union fixtures for RFC-86`.

---

### Phase B — Pins, waves, identity, stand-in retirement (D27)

#### S9 — Source `cid` pins at plan author (in-place)

- **Context:** author/survey path, `SourceBinding` / discovery, RFC D4 / D25; [`project::snapshot::SnapshotId`](../crates/project/src/snapshot.rs).
- **Deliverables:** When the in-place source set closes during `plan author`, record per-source tree `cid` (`SnapshotId` on wire as `cid`). Exact YAML home is an implementation detail — prefer plan/discovery-adjacent structured pin data, not ambient paths.
- **Gate:** author/reconciliation tests assert pins present after author.
- **Commit:** `feat(change): record source cid pins at plan author`.

#### S10 — Refine writes `base.yaml`

- **Context:** [`crates/slice/src/orchestrate/refine.rs`](../crates/slice/src/orchestrate/refine.rs), RFC D4 / D25.
- **Deliverables:** Before extract, assemble `base.yaml` from plan source pins + baseline-spec digest. Validate later sessions add drift; this session owns writer + shape tests.
- **Gate:** refine integration test asserts `base.yaml`.
- **Commit:** `feat(slice): write refine-time base.yaml pins`.

#### S11 — Slice-local requirement IDs + MODIFIED digests

- **Context:** [`crates/slice/src/synthesis/project.rs`](../crates/slice/src/synthesis/project.rs) `IdAllocator`, RFC D5.
- **Deliverables:** Mint slice-scoped ids at synthesize; each `MODIFIED` records digest of baseline body changed. No baseline `REQ-NNN` assignment yet (wave commit does that).
- **Gate:** synthesis tests for local ids + modified digests.
- **Commit:** `feat(slice): mint slice-local requirement ids until wave commit`.

#### S12 — One-member target wave types + `target.wave.opened`

- **Context:** RFC D9; new module under `crates/project` or `crates/slice` for wave manifests at `targets/<target>/waves/<digest>.yaml` under the change stand-in root (document path: `.emery/targets/...` for this cut).
- **Deliverables:** Wave manifest schema (one member); write-before-build helper; append `target.wave.opened`. Not yet wired into full build orchestration if S13 owns the wire-up — prefer types + persistence + tests here, wire in S13 if smaller.
- **Gate:** wave write/load tests.
- **Commit:** `feat(project): one-member target wave manifests and open fact`.

#### S13 — Build: recorded pin, no freeze, fact-substrate build records (D27)

- **Context:** [`crates/slice/src/orchestrate/target.rs`](../crates/slice/src/orchestrate/target.rs), Workspaces seam, RFC D27.
- **Deliverables:** Delete build-start `seam.freeze()`; `prepare` from recorded base pin; open one-member wave; persist content-addressed build record (retire `build/patch.yaml` as authority); “built” projects from records + wave facts. Update probe/full_loop assertions that require `patch.yaml`.
- **Gate:** slice build tests + `cargo make check` for affected crates.
- **Commit:** `feat(slice): build from recorded pins into fact-substrate records`.

#### S14 — Merge: wave commit, identity maps, keep interim `apply`

- **Context:** [`crates/slice/src/orchestrate/merge.rs`](../crates/slice/src/orchestrate/merge.rs) (+ merge engine), RFC D5 / D9 / D27.
- **Deliverables:** Revalidate wave; assign baseline `REQ-NNN`; record identity maps on `target.merge.wave-committed`; postflight succeeded / postflight-failed facts; still call interim `apply` from recorded result. Reject drifted `MODIFIED`. Failures before commit fact ⇒ not merged.
- **Gate:** merge identity + postflight tests; no `apply` deletion.
- **Commit:** `feat(slice): commit one-member waves with requirement identity maps`.

#### S15 — Pin drift diagnostics + Phase B acceptance fixtures

- **Context:** validate rules; RFC Acceptance #3–4; appendix pin/build-record fixture.
- **Deliverables:** `slice-base-drifted` / `slice-evidence-stale` (and merge block `merge-base-drifted` as needed); end-to-end pin→build-record→wave-commit fixture; “built”/“merged” never from leftover path checks.
- **Gate:** new fixtures green; `rg build/patch.yaml` only in historical comments/RFC if unavoidable — not as authority.
- **Commit:** `test(slice): pin drift and wave-commit fixtures for RFC-86 Phase B`.

---

### Phase C — Authorization epoch, gaps, shift-left, docs

#### S16 — Gap inventory projection (+ shared-lead rollup)

- **Context:** spec/model requirement statuses; RFC Gaps / D18 / D19 / D24.
- **Deliverables:** Pure projection: in-scope rows `(slice, req, status)` for `unknown|conflict|divergence`; presentation rollup by contributing `(source, lead)` with suggested re-refine selectors. Surface via `plan status` and optional read-only `plan gaps` (ship `plan gaps` if cheap — preferred).
- **Gate:** projection tests including multi-homed lead rollup; dropped slices excluded.
- **Commit:** `feat(plan): typed gap inventory with shared-lead rollup`.

#### S17 — Ready vs Authorized + status next-actions (D22 / D26)

- **Context:** status projector; RFC Progress / D22 / D26.
- **Deliverables:** Project Ready (clean gaps only) and Authorized (covering epoch — none yet until S18); post-author resume stays `plan execute` / `/emery:execute`; next-actions may name `slice refine <slice>` / review-gaps / execute `--waive…`. Never project `approved`.
- **Gate:** [`plan_status.rs`](../crates/change/tests/plan_status.rs) + Ready/Authorized fixtures (Authorized may be deferred assertion until S18 if epoch absent).
- **Commit:** `feat(plan): project Ready and Authorized milestones`.

#### S18 — `plan.execute.started` + `--waive` CLI

- **Context:** [`crates/change/src/orchestrate/execute.rs`](../crates/change/src/orchestrate/execute.rs), transport clap for `plan execute`, RFC D6 / D17.
- **Deliverables:** At execute start, append `plan.execute.started` with `closed-plan` coverage (`plan-digest`, per-leaf `existing{digest}|refine-under-epoch`, optional `unknown-waivers`). CLI: repeatable `--waive <slice>/<req>` + required `--reason`. Errors: `plan-waiver-invalid`, later `plan-epoch-stale`. No `plan approve` / `plan refine` verbs.
- **Gate:** execute-start fact fixture; waive validation tests.
- **Commit:** `feat(change): record plan.execute.started authorization epoch`.

#### S19 — Execute gap gate before build

- **Context:** execute loop; gap projection; RFC D13 / D15–D17.
- **Deliverables:** Before build: block on conflict (never waiveable); block on unknown unless matching waiver on covering epoch; list divergence. Refuse build when epoch stale vs covered artifacts (`plan-epoch-stale`). Print inventory on failure.
- **Gate:** gap-gate integration tests (conflict, unknown, waive, stale).
- **Commit:** `feat(change): enforce gap policy before build under execute epoch`.

#### S20 — `refine-under-epoch` path + shift-left happy path fixture

- **Context:** execute orchestration; RFC D12 / D14; Acceptance #5–6.
- **Deliverables:** Coverage may authorize refine-before-build for unspec’d leaves; preferred-path fixture: author → `slice refine` → gaps → execute (build/merge only). Under-epoch fixture: execute → refine → gap gate → build. Confirm no `plan refine` / `plan approve` subcommands.
- **Gate:** both fixtures green; CLI absence assertions.
- **Commit:** `test(change): shift-left and refine-under-epoch execute fixtures`.

#### S21 — Phase C remaining fixtures (in-scope drop, coverage shape, waves under execute)

- **Context:** RFC Acceptance #8–15; appendix test list leftovers.
- **Deliverables:** Dropped-slice in-scope fixture; coverage payload shape + stale-epoch; wave opened before build under execute; Ready skipped when waiving; post-author resume naming.
- **Gate:** listed fixtures green.
- **Commit:** `test(change): RFC-86 Phase C acceptance fixtures`.

#### S22 — D20 sibling consistency + operator prose

- **Context:** `AGENTS.md`, [`docs/standards/workflow.md`](../docs/standards/workflow.md), CLI help strings, skill wrappers if they claim “nothing is stamped”; skim series RFCs for leftover `plan approve` / `approved` / invented `plan refine`.
- **Deliverables:** Update shipped prose for durable `plan.execute.started`, gap gate, claims, computed status, pins/waves at the depth operators need. Do not cascade-rewrite unrelated RFC-88 design. Bump RFC-86 status line when acceptance is met.
- **Gate:** `cargo make links`; prose review checklist in session output.
- **Commit:** `docs: align operator prose with RFC-86 change facts`.

#### S23 — Full CI + acceptance checklist closeout

- **Context:** living plan acceptance checklist vs RFC Acceptance #1–15.
- **Deliverables:** Run `cargo make ci`; fix stragglers only (no new scope). Mark all sessions done; fill acceptance checklist in living plan; note PR-ready.
- **Gate:** `cargo make ci` green.
- **Commit:** `chore: RFC-86 implementation acceptance closeout` (or fold fixes into focused commits if CI failures span areas — still operator-committed).

#### S24 — Open the implementation PR (operator-directed)

- **Context:** full branch diff vs base; living plan; RFC.
- **Deliverables:** Operator (or agent **only when explicitly asked**) pushes branch and opens one PR: full RFC-86 implementation. PR body maps Phases A/B/C + acceptance checklist. **Still no commits invented by the agent unless the operator asks.**
- **Gate:** PR URL returned.
- **Commit:** none expected unless CI fixups requested.

---

## Acceptance checklist (product-level; fill at S23)

Track against RFC Acceptance #1–15. Leave unchecked until S23 closeout proves each item.

- [ ] #1 Progress is always computed from artifacts and facts — never a stored status field.
- [ ] #2 Multi-slice, multi-actor claims: disjoint refine after fact union; same-slice → `slice-claim-conflict`; no Git metadata; no plan-wide single-active-entry.
- [ ] #3 Two slices merge without REQ-id collision; drifted `MODIFIED` rejected; identity maps on `target.merge.wave-committed`.
- [ ] #4 Pins + build records: `base.yaml`; build from recorded pin (no freeze); fact-substrate build records (no `patch.yaml` authority); pin-drift diagnostics; interim `apply` may remain.
- [ ] #5 Shift-left practice: author does not refine; specs via `slice refine`; no `plan refine` CLI; under-epoch still allowed with gap gate on build.
- [ ] #6 Gap gate: unknown blocks (or waive); conflict never waiveable; divergence listed/allowed; no silent waive.
- [ ] #7 No topology-approve verb; refine not gated on topology approval.
- [ ] #8 Authorization epoch: every execute path appends `plan.execute.started` at start; no `plan approve` / `approvals/` / projected `approved`.
- [ ] #9 Same verbs/artifacts for solo laptop and directory-exchange collaboration.
- [ ] #10 Human prose review outside engine; gap gate is typed statuses only.
- [ ] #11 Shared-lead rollup is presentation only; waivers stay per-req.
- [ ] #12 Ready (clean gaps) vs Authorized (covering epoch); waivers skip Ready; no `approved` milestone.
- [ ] #13 In-scope = on plan and not dropped — shared by gaps / Ready / execute / unrefined next-actions.
- [ ] #14 Post-author resume names `plan execute` / `/emery:execute`; status may suggest `slice refine`.
- [ ] #15 One-member target waves: open before build; wave-committed; postflight non-rollback.

---

## Commit handoff format (required session output)

```markdown
## Commit handoff (operator)
**Do not auto-commit.** Suggested:

git add <paths>
git commit -m "$(cat <<'EOF'
<type>(scope): <why>

EOF
)"

### Files
- …

### Gate results
- …

### Living plan
- Status: Sxx done; Sy next
```

## Risk controls for context limits

- Each session lists a **Context pack** ≤ ~8 code files plus the RFC subsections it cites. **Must** open every cited D# / Acceptance / appendix subsection before implementing. **Must not** re-read the entire RFC every session, invent wire shapes from the digest, or treat Deliverables bullets as complete without the cited RFC text.
- Prefer crate-local `nextest` filters before full `cargo make ci` (S23 owns full CI).
- If a session discovers >1 session of work: stop, split into `Sxx` + `Sxx.1` in the living plan, mark current `blocked` or finish a thin vertical slice, and hand off — do not silently expand scope mid-session.
- Merge orchestration is a collision point with future RFC-88 work — finish S12–S14 before any parallel RFC-88 merge edits.

## Explicit exclusions

- No `emery plan approve` / `emery plan refine` verbs.
- No `.emery/change/` two-root migration (RFC-88).
- No deletion of Workspaces `apply`.
- No adapter Evidence schema changes.
- No automated `git add` / `git commit` / `git push` in Sessions S0–S23.
- No intermediate PRs.

## Implementation notes (RFC appendix — digest for implementers)

Orientation only — condensed from RFC-86 Appendix: Implementation notes. **Not normative.** Before coding a session, open the cited D# / Acceptance / appendix text in [`rfc-86-change-facts.md`](rfc-86-change-facts.md); that document wins on any conflict with this digest or with abbreviated Deliverables bullets.

**Already landed (RFC-87) — consume, do not reinvent**

- Value vocabulary: `project::snapshot::{SnapshotId, CodePatch}` (`sha256:…`; wire name `cid`).
- Workspace capability: `project::seam::Workspaces` — `freeze` / `prepare` / `capture` / `discard` / interim `apply`.
- Build today: freeze ambient tree; write `.emery/slices/<slice>/build/{request,report,patch}.yaml`. Phase B (D27): delete freeze; recorded pin; fact-substrate build records; one-member wave before build.
- Merge today: load `build/patch.yaml`, interim `apply`. Phase B: load from build record / wave; still call interim `apply` until RFC-88 deletes it.

**Layout and writers**

- Projection kernel in `crates/project`: facts + artifacts → status, gaps, Ready, Authorized, claims. Shared in-scope filter (D24). Ready is clean-gap only (D22).
- Replace `.emery/journal.jsonl` with `.emery/events/<actor>.jsonl` (stand-in path this cut); `journal show` merges the union.
- Remove stored plan-entry `status` and slice lifecycle fields; retire plan-wide single-active-entry.
- No `approvals/` tree. `plan.execute.started` carries typed `closed-plan` coverage.

**Pins, waves, and identity**

- Source `cid` pins close at plan author (in-place). Refine assembles `base.yaml` before extract. Build reads recorded pin into `prepare`.
- Waves at `.emery/targets/<target>/waves/<digest>.yaml` this cut; events `target.wave.opened`, `target.merge.wave-committed`, postflight succeeded / postflight-failed.
- Synthesis: slice-local ids until wave commit; `MODIFIED` digests; reject drift at commit.

**Author, refine, and execute**

- No `plan refine` / `plan approve`. Execute records epoch at start; gap gate before build; claims never authorize.
- Diagnostics (exit 2): `plan-gaps-unresolved`, `plan-epoch-stale`, `plan-waiver-invalid`, `slice-claim-conflict`, plus pin/merge drift codes. No topology-approval diagnostics; no “another slice already claimed.”

**Hard cut**

- Pre-1.0: re-init over migration; no shims for old status fields, single journal, plan-wide single-active-entry, global synthesize-time ids, plan `approve`/`refine` verbs, projected `approved`, ambient freeze once pins exist, or `patch.yaml` authority once fact-substrate records exist.
