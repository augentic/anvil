# RFC-88 Implementation Plan

> Status: **Draft for iteration** — not an RFC. This is the working implementation plan for [RFC-88 Detached Changes](rfc-88-detached-changes.md), structured as sequential agent-session-sized steps. The operator runs one agent session per step, strictly in sequence, committing each to the `rfc-88-impl` branch on `augentic/emery` and `augentic/emery-adapters`. Every step completes on those branches before any pull request is raised; then one PR per repository is raised in concert and the two land together (closed Open Question 13). Intermediate commits are branch-local. Git management is entirely operator-owned and appears nowhere in the steps.
>
> Companion inputs: [RFC-86](rfc-86-change-facts.md) (facts, waves, epochs — implemented), [RFC-86a](rfc-86a-gap-deferral.md) (gap deferral — implemented), [RFC-87](rfc-87-working-trees.md) (private workspaces — implemented; RFC-88 deletes its interim `apply`), [RFC-90](rfc-90-build-verification.md) / [RFC-91](rfc-91-refinement-stage.md) (implemented), [RFC-104](rfc-104-system-archaeology.md) (definition predecessor — **imminent**, landing just before this plan or very shortly after it; sequencing resolved in closed Open Question 1), [RFC-95](rfc-95-publication-sets.md) / [RFC-96](rfc-96-concurrent-execution.md) (successors whose seams RFC-88 must preserve), and the [platform programme](platform.md) (§ "RFC-88 scope discipline" and the four internal cuts).

## How to run this plan

**Session protocol.** Every step below is designed to fit one fresh agent session without exhausting context. Each session must:

1. Read `rfcs/rfc-88-detached-changes.md`, this plan's step section (plus its listed RFC anchors), and the step's listed code areas. Do not re-read the whole RFC corpus.
2. Implement exactly the step's scope. Out-of-scope items are listed per step and belong to later steps — do not pull them forward.
3. Run `cargo make ci` in the repo(s) the step touches (plus the wasm32 compile check `cargo check --lib -p emery --examples --target wasm32-wasip2` when the engine guest or WIT is touched) before finishing.
4. Update this file: tick the step's checkbox, and record any deviation, discovered detail, or new open question in the step's **Notes** block (add one if needed). The next session inherits that state.
5. Never hand-edit engine-owned artifacts in test fixtures where a builder exists; extend the builder.

**Cross-repo choreography.** `emery-adapters` consumes the engine SDK (`emery-adapter`, `emery-native`, `emery-probe`) as git dependencies pinned by engine release tag. For the duration of this branch, adapter-repo steps use the committed `[patch."https://github.com/augentic/emery.git"]` block in the adapters root `Cargo.toml` (uncommented) to resolve against the sibling `../emery` checkout on `rfc-88-impl`. No tag-pinned intermediate engine release is cut (closed Open Question 13). After every step is done, the operator re-points the pin and re-comments the patch when raising the two concert PRs; the PRs land together, so a published engine and adapters never diverge on a WIT or SDK break. Steps must not re-point the pin themselves. Engine steps that change the WIT package or SDK seam still note it so the next session runs the matching adapters step — that window is intra-branch only.

**Ordering rationale.** The steps follow the platform programme's four internal cuts, reordered only where the codebase makes a different order cheaper:

- **Cut A (steps 1–3; step 4 closed with no work per Open Question 2):** change-home relocation, target-tree boundary, accepted-CID merge, deletion of interim `apply` — all still in-place, so the substrate change is isolated from the new authoring surface.
- **Cut B (steps 5–9):** RFC-104 handoff import DTOs, two-root plumbing, detached change home, bounded location binding, adapter catalog, `discovery.yaml`.
- **Cut C (steps 10–17):** canonical lead catalog, source WIT extension, model-capability profiles, conflict-domain decomposition (deterministic kernel first, judgment second), refinement boundary escalation, amendment proposals.
- **Cut D (steps 18–20):** execution digest chain and detached accepted-CID execution, adapter-repo fixtures and eval cases, documentation closure.

## Current state (verified against the codebase, 2026-08-13)

What exists and is load-bearing for this plan:

| Area | State |
| --- | --- |
| RFC-104 (`emery system *`, `system.wave.reviewed`, handoffs, definition home) | **Read surface only** — `project::definition` owns the closed `Handoff` DTO, canonical digest, and fail-closed `resolve` (step 5). No `emery system *` write path. `EventKind::SystemWaveReviewed` parses from a definition-home event root and is refused by change-journal append. `EventKind::PlanExecuteStarted.discovery_digest` exists but is always `None`. |
| In-place `plan author` | Wave-bind (step 9) plus imported catalog (step 10): reviewed handoff → `discovery.yaml` + skeleton `plan.yaml` + catalog-only `leads.md` (retained at `leads/<digest>.md`, stamped on `plan.leads_digest`). Decomposition still pending (`slices[]` empty). |
| Plan model | `crates/project/src/plan/model/state.rs`: `Plan { name, sources, entries }`; `Entry.project: Option<String>` is the current target hook (removed in step 9 per closed Open Question 11); no `targets:` map, no digests. |
| RFC-91 refinement | Implemented: `refinement.yaml` with `inputs.planning.{entry, leads, decomposition}` (decomposition currently the canonical single-node projection), `inputs.profile` = canonical empty digest placeholder. |
| RFC-86 waves / build records | Implemented in-place: `crates/project/src/wave.rs` (`.emery/change/targets/<target>/waves/<digest>.yaml`; `Wave.members: Vec<Member>` gated by `enforce_one_member` — preserved, closed Open Question 12), `crates/project/src/build_record.rs` (`builds/<digest>.yaml`). Wave base comes from `freeze()` at build open. |
| RFC-86a gap gate | Implemented (`gap.deferred`, debt projection). No RFC-88 work needed beyond coverage wiring. |
| Workspace kernel | Implemented: `crates/project/src/workspace/` + `snapshot.rs`. `Store::apply` and `Workspaces::apply` are **deleted**. `Workspaces` carries `freeze/prepare/capture/discard/sweep`; snapshot ignore policy excludes `.git` **and a nested change home** (`.emery/change/`). Durable `.emery/` state (`project.yaml`, `specs/`, `decisions/`) folds into every snapshot. |
| Merge | `crates/slice/src/orchestrate/merge.rs`: per-member preflight → fold delta specs/identity maps **inside a writable workspace** prepared from the composed member-result → capture → `target.merge.wave-committed` (`members`, `base`, `result`) → archive → per-member postflight. Checkout is never written. Accepted CID: `project::wave::accepted_cid`. |
| Registry / workspace slots | Already removed (router rejects `emery registry *` / `emery init --workspace`). D4 is mostly a no-op; the remaining `Entry.project` vocabulary is deleted in step 9 (closed Open Question 11). |
| Source WIT | `survey(id) -> list<lead>` — no focus parameter, no child leads. Source adapters read `plan.yaml` from the lent project preopen to find their binding. |
| Adapter catalog / fingerprinting | Implemented: `project::adapter::catalog` (pins, recognition profiles, `select` / `assign`). Hosts supply the table through `adapter::Inventory` (`native::Provider`, guest `Provider`, `launcher::catalog()`). `intent` is explicit-only. |
| Model-capability profiles | Absent (only the manifest placeholder digest). |
| Adapters repo | Five sources (`intent`, `documentation`, `typescript`, `screenshots`, `captures`), three targets (`omnia`, `vectis`, `contracts`). Targets already receive RFC-87 workspaces + artifact stage; low churn expected. Eval workflow cases (`orders-contracts`, `omnia-r9k`) drive `init` + `plan author` from `case.toml` intent/source strings — no definition home. |

## Step overview

| # | Cut | Repo | Title |
| --- | --- | --- | --- |
| 1 | A | emery | Relocate the in-place change home to `.emery/change/` |
| 2 | A | emery | Target-tree boundary: `.git` + change home only |
| 3 | A | emery | Accepted-CID merge; delete interim `apply` |
| 4 | A | — | ~~Interim accepted-result access and eval survival~~ (closed by Open Question 2 — no interim surface; tombstone) |
| 5 | B | emery | Handoff + review-envelope DTOs, digests, fixture builder |
| 6 | B | emery | Two-root plumbing, detached change home, `plan author --from --wave` grammar |
| 7 | B | emery | Locator resolution, exact-revision ingestion, bounded-read policy |
| 8 | B | emery | Adapter catalog, recognition profiles, source keys, exact pins |
| 9 | B | emery | Plan model extension, `discovery.yaml`, wave-binding phase |
| 10 | C | emery | Canonical `leads.md` catalog + revision retention |
| 11 | C | emery | Source WIT extension: value-in, focused survey, child leads |
| 12 | C | adapters | Update the five source adapters to the new seam |
| 13 | C | emery | Model-capability profiles |
| 14 | C | emery | Decomposition substrate: DTOs, validators, compiler, projection |
| 15 | C | emery | Decomposition judgment legs + full detached `plan author` |
| 16 | C | emery | Refinement boundary escalation |
| 17 | C | emery | Amendment proposals + `plan amend --proposal` |
| 18 | D | emery | Execute digest chain + detached accepted-CID execution |
| 19 | D | adapters | Definition-home fixtures, eval and wasm cases |
| 20 | D | both | Documentation closure and final gates |

Steps 1–3 and 5–18 are strictly ordered. Step 4 is closed with no work (Open Question 2 — no interim materialization surface). Step 12 must follow 11; step 19 must follow 18. The 11 → 12 gap is intra-branch only (closed Open Question 13): no intermediate PR or engine tag is cut, and the two repository PRs land together.

---

## Cut A — accepted-CID merge and deletion of interim `apply`

The first cut lands the execution-substrate change while everything is still in-place and single-target, so behaviour changes are testable against the existing suite before any new authoring surface exists.

### Step 1 — Relocate the in-place change home to `.emery/change/` [x]

**RFC anchors:** D1 (change-home tree, in-place mode paragraph), implementation requirement "Operations take explicit target (product) and change roots… in-place changes use `<product>/.emery/change/`".

**Scope (emery):**

- Move every change-scoped artifact under `.emery/change/`: `plan.yaml`, `change.md`, `discovery.md` (renamed later, step 10), `slices/`, `events/`, `targets/` (wave manifests), and archive staging. Durable product state stays where it is: `.emery/project.yaml`, `.emery/specs/`, `.emery/decisions/`, `.emery/design-system/`.
- Rework `Layout<'a>` (`crates/project/src/config.rs`) to expose a `change_root()` beneath the project root and re-anchor every change-scoped path helper on it. This step deliberately keeps *one* root parameter; the two-root split is step 6. The goal here is that "change home" becomes a single directory the later steps can point elsewhere.
- Update `emery init` scaffolding, `plan archive` (archive move target), `archive prune`, journal writer paths, and every prompt/orchestration string that names a path (e.g. synthesis prompts referencing `.emery/slices/...`).
- Hard cut: no fallback read of the old flat layout (pre-1.0 posture confirmed by closed Open Question 3; an existing project re-inits). `emery init --upgrade` re-scaffolds the new layout but does not migrate live changes.
- Sweep `rg`-able path references in `AGENTS.md` and `docs/` in the same change (per the AGENTS.md removal rule); full doc prose rewrite waits for step 20.

**Key code areas:** `crates/project/src/config.rs`, `crates/project/src/journal.rs`, `crates/project/src/wave.rs`, `crates/change/src/plan/handlers/`, `crates/slice/src/`, `crates/transport/tests/`, integration suites across `crates/{project,slice,change,transport}/tests/`.

**Tests / gates:** existing integration suites updated to the new layout; a dedicated layout test asserting durable state and change state land on the correct sides of the boundary; `cargo make ci`.

**Out of scope:** detached roots, `--from`/`--wave`, tree-boundary changes, any new artifact shape.

**Why first:** every later step (tree boundary, two-root dispatch, detached homes) needs "the change home" to be one directory, not a scatter of root-level files plus most-of-`.emery/`.

**Notes (2026-08-13):**

- `Layout::change_root()` is `<project>/.emery/change/`. Durable helpers (`config_path`, `specs_dir`, `decisions_dir`, `guest_lock_path`) stay on `.emery/`; change-scoped helpers (`plan_path`, `change_brief_path`, `discovery_path`, `slices_dir`, `events_dir`, `targets_dir`, `archive_dir`) re-anchor on `change_root()`. `emery init --upgrade` now ensures those directories exist and does not migrate live artifacts from the old flat layout.
- Parent-walk helpers that assumed `slices/` sat directly under `.emery/` are updated: `Layout::project_dir_from_slice` walks `slices → change → .emery → project`. `Plan::archive` takes a `Layout` instead of inferring the project root from `plan.yaml`'s parent. The unused `.emery/plans/<name>/` co-move (nothing writes that trail today) now looks under `change_root()/plans/`.
- `guest.lock` and `scratch/` stay under `.emery/`, not the change home — they were not in this step's change-scoped list. Step 6's two-root plumbing should decide whether the guest marker moves onto the change root once a detached home has no `.emery/`.
- First-party source adapters still tell the model to parse `plan.yaml` at the lent workspace root (`BINDING_NOTE` in `emery-adapters`). Engine mock tests do not hit that path. Live eval/wasm against real adapters will look in the wrong place until step 12 deletes those notes. No adapters-repo change in this step.
- Answer-schema goldens (`report` / `phase-report`) carry the slice Decision Record path in a description string; the engine copies under `crates/adapter/schemas/` were updated with it. Not a wire-shape break.
- Between this step and step 2, `plan.yaml` / `change.md` / `discovery.md` drop out of snapshots (they moved under `.emery/`, which is still fully excluded). That is the desired end state for those files — step 2 keeps the change home excluded. Step 2's "root-plan-artifact exclusions" item is now a no-op: there are no plan artifacts at the repo root to exclude. The snapshot walker still carries `IGNORED_ROOT` (`change.md` / `discovery.md` / `plan.yaml` at the walk root) in `crates/project/src/workspace/store.rs`; after this move those names never appear at the product root, so step 2 should delete the constant when it retargets the ignore policy from `.emery/` to `.emery/change/`.

### Step 2 — Target-tree boundary: `.git` + change home only [x]

**RFC anchors:** D1 (last paragraph), D5 ("the identified tree includes durable Emery state"), implementation requirement "Target trees ignore only `.git` and a nested change home. `.emery/` is otherwise included." Amends RFC-87 D4 / acceptance criterion 4.

**Scope (emery):**

- Change the snapshot ignore policy (`project::snapshot` ignore rules and the native `Store::snapshot` walker) from "exclude `.git` + `.emery`" to "exclude `.git` + the nested change home (`.emery/change/`)". `project.yaml`, `specs/`, `decisions/`, and the rest of `.emery/` fold into every snapshot.
- Update `freeze()` implementations (native provider, guest provider). Delete `IGNORED_ROOT` in `crates/project/src/workspace/store.rs` (`change.md` / `discovery.md` / `plan.yaml` at the walk root): step 1 moved those files into the change home, so there is nothing at the repo root for freeze to special-case.
- Verify digest identity stays deployment-independent (native vs guest kernels produce byte-identical manifests for the new boundary).
- Kernel round-trip tests: baseline (`.emery/specs/`), `project.yaml`, and `decisions/` survive `freeze → prepare → capture`; the change home never appears in a snapshot; a workspace prepared from such a snapshot exposes the baseline to the build agent.

**Key code areas:** `crates/project/src/snapshot.rs`, `crates/project/src/workspace/`, `crates/native/src/provider.rs`, `crates/guest/src/provider.rs`, workspace kernel tests.

**Tests / gates:** kernel tests above; existing build/merge integration suites still pass (they will see baselines inside workspaces from here on); `cargo make ci` + wasm32 compile check.

**Out of scope:** merge behaviour changes (step 3). Note: between steps 2 and 3, merge still writes the checkout baseline; that overlap is fine because the snapshot simply carries a copy.

**Notes (2026-08-13):**

- Ignore policy lives on `project::snapshot::ignored` (the vocabulary AGENTS.md already named). `Store::snapshot`, the native `FsExecBits` walk, and `plan::pins` (source/`dir_cid` trees) all call it, so a source tree and a freeze of the same directory stay byte-identical — step 7's both-roles CID reuse depends on that. `IGNORED_ROOT` is deleted. Nested `.emery/change` at any depth is excluded (vendored product trees), matching `.git`.
- `freeze()` on the native and guest providers is still `store.snapshot(project_root)`; only the kernel boundary and the impl/trait docs changed. Digest identity stays the shared canonical manifest (existing `golden::pinned_snapshot_id`).
- **Ephemeral `.emery/` tenants now fold into snapshots.** `guest.lock` and `scratch/` were previously excluded only because all of `.emery/` was. Execute holds `.emery/guest.lock` (pid/hostname/timestamp body) across wave-open `freeze()`, so every in-place freeze until step 6 re-homes the marker will hash that lock into the CID. `scratch/` is gitignored but freeze walks the filesystem. This step does not add those exclusions — the RFC requirement is "only `.git` and a nested change home". Step 6's re-home onto the change root also *fixes* in-place: `.emery/change/guest.lock` is already ignored here. Step 3's accepted-CID work should not golden a freeze digest and should treat lock-file presence as known noise until step 6; if a test needs a product-only CID before then, exclude the two tenants in the ignore policy as a recorded deviation rather than teaching freeze to run before the marker.

### Step 3 — Accepted-CID merge; delete interim `apply` [x]

**RFC anchors:** D7 (whole), D8 acceptance-criterion 7 semantics, implementation requirements "Route every detached target through RFC-86's immutable one-member wave…", "`Workspaces::apply` and its write-back machinery are removed", "merges update the baseline inside the workspace", RFC-87 D8 (interim `apply` deletion), RFC-91 D6 (base selected at wave open).

**Scope (emery):**

- **Accepted-CID projection.** Add a per-target accepted-CID projection computed from `target.merge.wave-committed` facts: initial CID → chain of `{base, result}` transitions; reject a broken chain (a committed fact whose `base` is not the prior accepted CID) with a typed error. In-place mode's initial CID is the freeze taken when the target's *first* wave opens (RFC-91 D6 semantics preserved); the projection lives beside the wave/fact machinery in `crates/project/`.
- **Wave open selects the accepted CID.** `open_wave` (`crates/slice/src/orchestrate/target.rs`) uses the projection: first wave freezes ambient (in-place) and records it as the initial CID; every later wave opens against the current accepted CID, not a fresh freeze. Record base CID on the wave manifest as today. Keep `Wave::one_member` as the serial constructor and `enforce_one_member` as the write/open gate (closed Open Question 12).
- **Member model (closed Open Question 12).** `Wave.members` stays `Vec<Member>`. Do not collapse it to `member: Member` or `slice: SliceName`. RFC-96 retires `enforce_one_member` for the concurrent executor only; this step does not. Merge iterates `wave.members` in stable order — N=1 is a one-element loop, not a type-level singleton. The merge WIT stays per-slice (`merge(id, slice, phase)`); the engine owns the member loop so RFC-96 can widen membership without changing that operation.
- **Merge inside the workspace.** Rework `crates/slice/src/orchestrate/merge.rs`: resolve the named slice's frozen wave and refuse until every member result is present (D7 — in this cut that is one member, still checked over `wave.members`); prepare a writable workspace from the composed member-result (for this cut, the sole `BuildRecord.result` CID, loaded through the member list); run every member's slice-scoped preflight in stable member order; fold every member's delta spec and identity map in that order *inside that workspace* (baseline now lives in the tree per step 2); capture the final candidate CID; append `target.merge.wave-committed` carrying the frozen member set, base and final-result CIDs, identity maps, baseline digest, deferred (debt) members; then every member's postflight in the same order with persisted reports and crash-resume at the first missing report. A crash before the commit fact leaves the prior accepted CID authoritative — assert this with a crash-boundary test. Replace the sole-member `slice_name` stand-in on `target.merge.wave-committed` with the frozen member set (D7 — RFC-96 keeps that fact shape); `target.merge.wave-postflight-failed` names every failed member (D7 aggregate). `target.wave.opened` may keep `slice_name` as the serial-executor's named leaf (always `members[0].slice` under the one-member gate); the manifest remains membership authority. `merged` projects for every named member of the committed set.
- **Delete `apply`.** Remove `Store::apply`, the two `seam::Workspaces::apply` legs (native, guest), merge's `apply_result`, and the `slice.code.applied` event kind. The operator checkout is no longer written by merge — in-place included, no migration (closed Open Question 3). `merged` continues to project only from the committed fact.
- Update `plan archive` completion conditions if they referenced applied trees; update debt/baseline conservation tests (baseline debt now folds inside the workspace tree during merge).

**Key code areas:** `crates/slice/src/orchestrate/merge.rs` + `merge/`, `crates/slice/src/merge/` (engine + commit), `crates/project/src/wave.rs`, `crates/project/src/journal/event.rs`, `crates/project/src/workspace/store.rs`, `crates/{native,guest}/src/provider.rs`, `crates/slice/src/orchestrate/target.rs`.

**Tests / gates:** integration coverage for: one-member wave over accepted CID; a two-member write still fails `target-wave-member-count` (the vec is not collapsed); commit fact names the frozen member set (length 1 in this cut) and `merged` projects from that set; dependent second wave opening against the first's accepted result; commit/postflight crash boundaries (fail before fact → no merge projected; fail after fact → accepted CID stands, resumable stop); broken-chain rejection; checkout untouched by merge. `cargo make ci` + wasm32 check.

**Out of scope:** detached targets, multi-target execution (step 18), multi-member waves (RFC-96 — keep the `Vec` and the one-member gate; closed Open Question 12). There is no materialization surface to build — per closed Open Question 2, merged results stay store-only until RFC-95; from this step until step 19 repoints the graders, the eval and wasm rungs cannot inspect produced trees.

**Note:** this is the largest Cut A step. If a session runs hot, the accepted-CID projection + wave-open change can be split from the merge rework at the recorded seam (projection lands first, merge rework consumes it). Step 2's Notes: in-place `freeze()` currently hashes `.emery/guest.lock` (held for the execute run) into the CID; do not golden freeze digests, and do not treat lock-file presence as product drift.

**Notes (2026-08-13):**

- Accepted-CID projection is `project::wave::accepted_cid`. No commits → first `target.wave.opened` wave's `base`; no waves → `None` (caller freezes). Broken `{base, result}` chain is `target-accepted-cid-broken-chain`. `target.merge.wave-committed` carries `members` (replacing sole-member `slice_name`), `base`, and `result`. `target.merge.wave-postflight-failed` names every failed member. `slice.code.applied` is deleted.
- Wave open uses the projection: first wave freezes ambient; later waves open against the current accepted CID. `Wave::one_member` + `enforce_one_member` stay (vec not collapsed). Merge iterates `wave.members`; this cut's composed result is still the sole `BuildRecord.result` via that list.
- Merge prepares a **writable** workspace from the composed member-result, folds inside it, captures, appends the commit fact, **then** archives, then postflight (resume at first missing `postflight.yaml`). `slice::merge::slice::commit` no longer archives — the orchestrator archives after the fact. Resume-after-commit archives leftover slice dirs with an empty `MergeCommit` (archive summary may say "no baseline specs touched").
- Checkout is never written. Tests inspect folded baselines and build outputs by `Store::materialize` of the accepted CID (`Session::materialize_accepted`). Do not golden freeze digests (`guest.lock` hashes into in-place freeze until step 6).
- `plan archive`'s change-scoped sweep now keeps every target's accepted CID as a **live** GC root. Without that, archive would collect the merged product (checkout is no longer a copy). Intermediate wave/build pins still go. Step 18's archive audit should keep this invariant when coverage/plan CIDs join the root set.
- Mock merge-gate markers still live on checkout `ctx.project_root` (`lending` only overrides `lend`, not `project_root`).
- **`emery debt` and `plan author`'s `## Carried debt` still read checkout `.emery/specs/`**, which merge no longer writes — those operator surfaces are dark until step 18 re-homes them onto the accepted CID. `plan archive`'s carried-debt summary still joins facts and stays live. Eval/wasm graders stay checkout-based until step 19.
- `EventKind::TargetMergeWaveCommitted` grew (`members` + `base`/`result` CIDs) and pushed `orchestrate::execute`'s future past clippy's 16KiB `large_futures` cap. Callers stay small by boxing at `Execute::call` and the probe client's dispatch; later EventKind growth should box the same bodies rather than every test call site.

### Step 4 — Interim accepted-result access and eval survival [closed — no work]

**Closed by Open Question 2 (2026-08-13): no interim surface is built.** The operator accepts that merged results exist only as store snapshots until RFC-95 lands. No session runs for this step — everything it would have delivered either already exists or belongs elsewhere: the accepted-CID projection is step 3's scope, `project::workspace::Store::materialize` is already public kernel API (and `probe`'s case runner already uses it in-process), and repointing the eval graders is step 19's scope. Between steps 3 and 19 the eval and wasm rungs cannot grade against produced trees; the wasm rungs' observable outcome is exit codes, journal facts, and `build/report.yaml` until RFC-95. The step number is retained as a tombstone so later cross-references stay valid.

---

## Cut B — detached change home, wave import, delivery binding

### Step 5 — Handoff + review-envelope DTOs, digests, fixture builder [x]

**RFC anchors:** D1 (`imports/` retention), D3 (digest coverage paragraph), D8 (verification list), implementation requirements "Handoff and binding DTOs reject unknown fields and use typed canonical digests… Integration tests use reviewed definition fixtures"; RFC-104 D10 (handoff shape, `system.wave.reviewed` fact, resolution rules).

**Scope (emery):**

- New module `crates/project/src/definition/` owning the RFC-104 contract surface RFC-88 consumes (closed Open Question 8 — a module in `project`, not a new workspace crate). Per closed Open Question 1, this is **coordinated RFC-104 code delivered early**, not an RFC-88-private read-side approximation: exactly one implementation of the `Handoff` DTO and its canonical digester exists, agreed with the imminent RFC-104 implementation (which adds the write side to the same module). Coordinate the shape before this step runs; any handoff-shape change during RFC-104's development is a breaking change to this shared module that both efforts absorb together — never RFC-104's private business. A drifted digester would fail every retained import's verification chain the day RFC-104 lands.
  - `Handoff` DTO matching RFC-104 D10's YAML (version, definition, scope/coverage/sources/system-model/migration-plan digests, `wave` block with targets, evidence-scopes, delivery-mappings, element lists, and every `{id, digest}` reference list). `deny_unknown_fields`, typed digests, canonical digest function (schema-validated content, format-independent — reuse the canonicalization approach of `plan::projection`). Per closed Open Question 9, each evidence-scope row closes `value` xor `source-cid`: location-backed sources carry `source-cid`; `intent` carries the inline `value` (no locator, no CID).
  - `system.wave.reviewed` enters the engine's closed `EventKind` as a definition-scoped variant with a typed payload and **no writer**: RFC-88 *parses and verifies* the event from a definition home's `events/<writer>.jsonl` (identity `(writer, sequence, event-digest)`) but never appends it — the write path arrives with RFC-104's `emery system review`. RFC-104's implementation requirements extend the closed taxonomy with this kind anyway, so no parallel foreign-envelope parsing machinery is built. Definition and change event roots stay separate (RFC-104 requirement).
  - Definition-home read surface: given a definition root and wave id, resolve the single current handoff projection under `handoffs/<digest>.yaml`, fail closed on missing or ambiguous projections, and verify the matching review fact.
- **Fixture builder** in `crates/mock` (host-only test support): mint a valid definition home — handoff with correct canonical digests, review event, degenerate and multi-target variants — so every downstream step and the adapters-repo eval cases can author against fixtures. The degenerate variant always includes an `intent` evidence scope with inline `value` (reserved key `intent`, no locator, no CID) so step 9's in-place `--from .emery/system/` path and the adapters-repo eval cases bind intent 1:1 from the handoff (closed Open Question 9). This builder is permanent test substrate, not a stopgap: `emery system survey` / `system plan` are judgment-bearing live-model orchestrations, so CI can never mint definition homes through them even after RFC-104 lands.
- Typed diagnostics: missing review fact, ambiguous/missing current projection, digest mismatch, unknown fields, malformed envelope.
- **Rebase rule (closed Open Question 1):** if RFC-104 has landed before this step runs, the step shrinks to the read surface + fixture builder over RFC-104's already-landed types; verify canonical-digest parity against a definition home authored by the real `emery system` verbs instead of authoring the DTOs here.

**Key code areas:** new `crates/project/src/definition/`, `crates/project/src/journal.rs` (the writer-less `system.wave.reviewed` taxonomy variant + read-side envelope verification), `crates/mock/src/` (fixture builder), `crates/project/tests/`.

**Tests / gates:** round-trip + canonical-digest stability tests (YAML reformatting does not change the digest); fail-closed resolution matrix; fixture-builder self-checks including the degenerate `intent` evidence-scope form (`value` present, `source-cid` absent). `cargo make ci`.

**Out of scope:** any CLI surface, binding, or import-into-change-home behaviour (steps 6/9).

**Notes (2026-08-13):**

- RFC-104 has not landed; this step authored the shared `Handoff` DTO, canonical digester, and read surface in `crates/project/src/definition/` (closed Open Question 1 / 8). `system.wave.reviewed` is `EventKind::SystemWaveReviewed { handoff_digest }` — change-journal `append_for` refuses it (`journal-event-read-only`). Fixture homes write definition-root `events/<writer>.jsonl` directly.
- **Current projection** is the unique `handoffs/<hex>.yaml` whose `wave.id` matches. Zero files → `definition-handoff-missing`; two or more → `definition-handoff-ambiguous` (no timestamp pick). Filename stem must equal the canonical digest hex (`definition-handoff-mismatch`). RFC-104's write side retains historical handoffs in the same directory: either keep at most one file per wave id as "current", or add input-digest matching against live `scope.yaml` / `coverage.yaml` / … before this read can disambiguate a real definition home. Any unreadable YAML in `handoffs/` fails the whole resolve (`definition-handoff-malformed`) — retained historical files must stay parseable by this DTO.
- Definition-home event read **fails closed** on unparseable lines (`definition-event-malformed`), unlike change-journal `read_union` which skips them. Multiple review facts for the same handoff digest take the first in union order.
- Evidence-scope xor is enforced at parse: `intent` → `value` and no `source-cid` (`definition-intent-form`); every other source → `source-cid` and no `value`. Canonical digest is `serialise_yaml` of the parsed DTO (empty lists omitted); YAML comments/whitespace/key order do not move it. Review `event-digest` is SHA-256 of the envelope's canonical JSON.
- Fixture builder: `mock::definition::{Spec, mint}`. `Spec::degenerate(intent)` always includes the reserved `intent` evidence scope with inline `value`. `Spec::multi_target()` is two location-backed scopes. Adapters-repo eval does not depend on `emery-mock` today — step 19 should re-export `mint` through `probe` / the lab shim rather than adding a mock dependency to first-party eval. Step 9 copies byte-identical envelopes by re-reading `Home::handoff_path` and `events_dir`; `Reviewed` does not carry raw bytes.

### Step 6 — Two-root plumbing, detached change home, author grammar [x]

**RFC anchors:** D1 (detached tree, "Operations therefore receive separate target (product) and change roots"), acceptance criterion 1; implementation requirements "Operations take explicit target (product) and change roots… Detached homes have no synthetic `project.yaml`".

**Scope (emery):**

- Extend `ExecutionPaths` / anchoring (`crates/project/src/handler/paths.rs`, `crates/launcher/`) with an explicit change root distinct from any product root. In-place mode: change root = `<product>/.emery/change/` (step 1's layout), product root = the repo. Detached mode: change root is an operator-selected directory with **no** `.emery/project.yaml` and no Git requirement; there is no ambient product root at all (product trees only ever appear as store-materialized workspaces, per D7).
- Change-root anchoring (closed Open Question 6): no marker file, no ancestor walk for a change home. Detection is `--change-dir` (that path is the detached change root) else nearest ancestor carrying `.emery/project.yaml` (in-place, as today) else cwd is the detached change root. Detached `plan author <name>` writes into that resolved directory — `<name>` is the change identity, not a subdirectory. `--change-dir` is the `--project-dir`-shaped optional override on change-scoped verbs; it is never required. The operator is expected to run from the change directory; a subdirectory, a parent, or a detached home nested inside a product checkout is operator error (the last is detected as in-place unless `--change-dir` is passed).
- Transport grammar: `emery plan author <name> --from <definition-root> --wave <id> [--change-dir <dir>] [--force]` (detached), and in-place `--from .emery/system/` binding for a colocated degenerate definition. The old author grammar (bindings from `case.toml`-style init) is removed in step 9 when binding fully replaces survey-driven authoring; this step lands the flags, root resolution, and change-home scaffolding, with the new path returning a typed "not yet implemented" until step 9 completes the phases. (If the operator prefers no dead grammar on the branch, fold this step's CLI surface into step 9 and keep only the root plumbing here.)
- Launcher: mount policy for the detached change root (writable) and the definition root (read-only) as preopens; keep the store host-owned. Guest `Layout` consumes the change root.
- `emery init` is not required for a detached change (acceptance criterion 1: empty non-Git directory works); make init-dependence explicit and remove it from the detached path.
- Step 1 left `guest.lock` at `.emery/guest.lock`. A detached change home has no `.emery/`; this step should re-home the marker onto the resolved change root (or replace it) so detached `plan refine` / `plan execute` still have a create-exclusive fence. Re-homing onto the change root also stops in-place wave-open `freeze()` from hashing the lock into the CID (step 2's boundary already excludes `.emery/change/`; see that step's Notes). `scratch/` stays under `.emery/` and will still enter snapshots unless this step moves it too or the ignore policy grows that tenant.

**Key code areas:** `crates/project/src/handler/paths.rs`, `crates/project/src/config.rs`, `crates/launcher/src/`, `crates/transport/src/command/`, `crates/guest/`, `src/main.rs` mount expressions.

**Tests / gates:** router/grammar tests; detached-root resolution tests (cwd is the home — no `project.yaml`, no git, no subdirectory created, no marker); `--change-dir` override; in-place still wins when an ancestor has `project.yaml`; in-place `--from .emery/system/` resolution; `cargo make ci` + wasm32 check.

**Out of scope:** actual binding (step 9), locator resolution (step 7).

**Notes (2026-08-13):**

- Detection is `project::config::Roots::resolve(invoked_dir, change_dir)`: `--change-dir` is always detached (even when that directory contains `project.yaml`); else nearest ancestor with `.emery/project.yaml` is in-place; else cwd is the detached change home. No marker file, no change-home ancestor walk. `<name>` is the change identity, not a subdirectory.
- `ExecutionPaths` carries `project_root` (guest `.` mount), `change_root`, and `detached`. `new` is in-place; `detached` sets both roots to the change directory; `from_roots` is the launcher/native assembly. Guest `ExecutionPaths::guest` trusts `EMERY_DETACHED` (set `"1"` when detached, **unset** when in-place) and does not probe cwd. `EMERY_CHANGE_ROOT` is the host-absolute change home; `EMERY_PROJECT_ROOT` remains the `.` mount (product in-place, change when detached).
- `Layout::new` stays in-place-only. `Layout::detached` is deferred until `Ctx` can load without init (steps 9/18). `guest.lock` is re-homed to `<change-root>/guest.lock` (in-place: `.emery/change/guest.lock`, already snapshot-ignored). Detached `plan status` / `refine` / `execute` still fail `NotInitialized` until later steps teach those verbs to skip `Ctx::load`.
- `--change-dir` is a clap flatten on change-scoped verbs, consumed by the launcher (and native `command::execute` when the flag is explicit). It is **not** on `Input`. Relative `--change-dir` joins the invocation directory (launcher) or the caller's `project_root` (native — tests pass a tempdir while cwd is the crate). `adapter add --project-dir` still wins for that verb and stays in-place.
- `plan author --from` / `--wave` require each other at clap; mixing with `--source` / `--intent` is `Error::Argument`. The new path skips `Ctx::load` and returns `plan-author-unimplemented` (exit 2) until step 9 binding. Old `--source` / `--intent` is kept. `--force` is accepted, unused until step 9. No `name/` subdirectory, no marker, no synthetic `project.yaml`.
- Definition preopen: when `--from` resolves to an existing directory, a self-named read-only mount (seed pattern) so the guest opens argv unchanged; otherwise a `/emery-definition` fallback onto the `.` mount so the runtime always boots. Relative `--from .emery/system/` also opens via `.` in-place.
- `scratch/` still lives under `.emery/` and still hashes into in-place freeze (gitignore only). Not moved. Guest freeze still snapshots `.` (product in-place; would snapshot the change home if execute ran detached — step 18 must freeze the accepted CID, not `.`).
- `--change-dir` pointing at a product tree is detached by spec; `EMERY_DETACHED` tells the guest not to treat `.emery/project.yaml` on that mount as in-place.

### Step 7 — Locator resolution, exact-revision ingestion, bounded-read policy [x]

**RFC anchors:** D2 (whole), D5 (locator + `cid` for targets), D9 (whole); implementation requirements "Repository ingestion accepts an exact revision; workspace preparation accepts an explicit target and CID", "Plan CIDs remain GC roots for the change lifetime"; acceptance criteria 2 and 10.

**Scope (emery):**

- Locator grammar and typed parse: Git reference (`url@revision`), change-relative path, external local path, bounded HTTPS URL; optional `path` selector (default `.`); `value` inline alternative. One row shape shared by sources and targets.
- Resolution kernel: resolve each locator once → stage temporarily → store as a tree under its CID (a file becomes a one-file tree); mutable Git refs resolve to exact revisions at the host; later runs use the recorded CID and never reread the origin. Both-roles reuse (same repo as target and source → one CID).
- Git ingestion at an exact revision (host-side, launcher-adjacent like `launcher::install`): no hooks, no submodules, no LFS filters, no escaping symlinks; `.git` and nested change home excluded from the ingested tree (step 2's boundary). Moved-branch = freshness warning; unavailable recorded commit = error.
- HTTPS reads: HTTPS-only, credential-free URLs, no private-network targets, redirect and body budgets; GitHub document pages resolve to raw content.
- Bounded-read policy as a closed Rust struct with compiled defaults (closed Open Question 10 — not a `discovery.yaml` field, not digest-covered, no host override): limits on bindings, API requests, concurrency, time, inspected bytes, imported trees, redirects, HTTPS bodies. Starting values: concurrency 4 (independent bind reads only; focused-survey fan-out stays RFC-96), bindings/imported trees 32, HTTPS redirects 5, HTTPS body 32 MiB, inspected-bytes / tree cap 512 MiB. Documented in-code as declared starting values. Budget exhaustion fails the wave for upstream narrowing — no namespace fallback, no per-project raise.
- Read-only source views: prepare-with-empty-writable-scope over the recorded CID (RFC-87 D2 path); `capture` already refuses read-only workspaces.
- GC roots: plan/discovery CIDs registered as store GC roots for the change lifetime; `plan archive`'s existing `Store::sweep` picks them up when the change ends.

**Key code areas:** new `crates/project/src/binding/` (or similar) for the locator/resolution kernel, `crates/launcher/src/` (network + git side), `crates/project/src/workspace/`, `crates/project/src/seam.rs` if the host capability surface grows, fixtures with a local git repository and local HTTP server.

**Tests / gates:** integration tests over local git fixtures (exact revision, moved branch, missing commit), local HTTPS fixtures (budgets, redirect caps, private-network refusal), file-vs-tree CID shape, both-roles CID reuse, GC-root retention. `cargo make ci`.

**Out of scope:** adapter selection (step 8), writing `discovery.yaml` (step 9).

**Notes (2026-08-13):**

- Kernel is `project::binding` (wasm-clean): `Locator` / `Location` / `Origin`, `Policy::standard()`, `Meter`, `Session::ingest`, intern `Cache`, `roots()`, `view()` (prepare `writable: false`). Git clone and HTTPS fetch are `launcher::ingest` (host-side, like `launcher::install`). No WIT / seam trait this step — "no traits for testability alone"; both production hosts will need a capability in step 9 (see that step).
- Git form is `url@revision` (HTTPS git, `git@` SSH, `file://`, absolute/`./`/`../` local path). `https://` without `@revision` is a document URL. Credentials and `http://` are typed refusals. Path selector defaults to `.` and is refused on HTTPS locators.
- `Policy` starting values: concurrency 4, bindings/imported trees 32, HTTPS redirects 5, HTTPS body 32 MiB, inspected-bytes 512 MiB. OQ10 left API-requests and wall-clock unspecified; declared starting values are `api_requests` 128 and `time_ms` 120_000. Production always uses `Policy::standard()`; tests may construct tighter copies. Resolve is serial (1 ≤ concurrency 4); step 9 may fan out under the cap.
- Recorded CID in the store skips origin I/O. Both-roles reuse is the intern cache (canonical locator+path → CID), including the pre-pin `url@branch` key after Git ingest. Moved-branch warning fires when a named ref's tip differs from `prior`; ingest still uses `prior`. Unavailable SHA is `git-revision-unavailable`.
- HTTPS transport tests (redirect/body caps) use loopback HTTP with the URL gate skipped — production `fetch` always applies the gate, which refuses loopback/private literals before connect. Private-network refusal is covered by `check_https` / `fetch("https://127.0.0.1/…")`.
- `binding::roots` is the GC-root helper. `plan archive` is not wired this step (`discovery.yaml` does not exist yet). Step 9 must pass discovery/plan CIDs into archive's live set (and into `dead` after archive, except accepted CIDs).

### Step 8 — Adapter catalog, recognition profiles, source keys, exact pins [x]

**RFC anchors:** D6 (whole); acceptance criterion 3.

**Scope (emery):**

- Host-supplied bounded, versioned adapter catalog: source adapters with recognition profiles (deterministic fingerprint rules over a staged source value — e.g. extension/manifest probes for `typescript`, `documentation`, `screenshots`, `captures`), target adapters with platform constraints. The engine consumes the catalog through a seam capability; the shipped deployment compiles the first-party catalog into the launcher/native host (mirroring the existing first-party GHCR mapping); `intent` is explicit-only and never fingerprinted.
- Deterministic matching: reuse pins already carried by the handoff verbatim; fingerprint only a newly focused source the handoff left open; exactly one match pins, zero/many block that wave member with `source-adapter-no-match` / `source-adapter-ambiguous`. No ranking, no model fallback.
- Exact package pins everywhere in detached topology (`emery:<name>@<semver>`); unversioned local components are refused for detached binding with a typed diagnostic.
- Source key generation: locator → normalized basename; inline value → adapter name; `intent` reserved as the sole key for that adapter (closed Open Question 9 — 1:1, never a second intent binding, digest suffixes do not apply to it). Unchanged bindings keep keys; collisions on other adapters receive stable digest suffixes; duplicate bindings rejected. Persisted key is authoritative downstream.
- **No `intent` locator form** (closed Open Question 9): a locator (Git, path, HTTPS) on the `intent` adapter is a typed refusal. `intent` is always the reserved key with inline `value`; D2's locator arm does not apply to it.

**Key code areas:** `crates/project/src/adapter/` (catalog seam + matching kernel), `crates/native/src/` and `crates/launcher/src/` (catalog supply), `crates/mock` (catalog fixtures for tests).

**Tests / gates:** key determinism + collision-stability matrix; no-match/ambiguous blocking; pin reuse from handoff fixtures; duplicate rejection; `intent` locator form refused; reserved-key uniqueness (a second intent binding is a duplicate, not a suffixed key). `cargo make ci`.

**Notes (2026-08-13):**

- Kernel is `project::adapter::catalog` (wasm-clean): `Catalog` / `Pin` / `Profile` / `Recognition`, `select` (handoff pin reuse vs fingerprint), `assign` (source keys). `intent` is `Recognition::Explicit` and never probed. Exact pins only (`Pin::parse`); bare names and local components fail `adapter-unversioned`.
- Host supply: `adapter::Inventory::inventory` (method is not `catalog` — native `Provider::catalog` is the dispatch vtable). Default table is `Catalog::first_party()`; native `with_inventory` substitutes. Guest compiles the same table until a WIT override lands. `launcher::catalog()` / `mock::inventory()` re-export it. No WIT this step — same posture as step 7 ingest.
- Compiled first-party version is `0.12.0` (declared starting value, adapters workspace today). Pin reuse copies the handoff string verbatim (fixtures stay `@1.0.0`). Step 9 should overlay the host's resolvable version when fingerprinting, or the recorded pin will drift from GHCR.
- Recognition starting probes: `typescript` (`package.json` / `tsconfig.json` / `jsconfig.json` + `ts`/`js` family), `documentation` (`md`/`mdx`/`rst`/`adoc`), `screenshots` (raster extensions), `captures` (`tests/data/replays`). Walk skips `.git` / `.emery` / `node_modules` / `target` / `dist` / `build` and does not follow symlinks. A tree with both `package.json` and `README.md` is `source-adapter-ambiguous` (no ranking). Fingerprint does not meter D9 `inspected-bytes` yet — step 9 should thread `Policy`.
- Keys: locator basename (path selector when not `.`, else URL/path stem); inline value uses the adapter name; `intent` is reserved. Unchanged rows keep `prior` keys; new collisions suffix 8 hex of the identity digest (lex-least identity keeps the bare basename). Duplicate identity (same locator key, or a second `intent`) is `source-adapter-duplicate`. A locator whose stem is `intent` cannot steal the reserved key (`source-<hex>`).
- **Step 9 leftover — open adapter on the handoff.** `Scope.adapter` is still a required pin string. Fingerprint (`Hint::Open`) is the kernel path for "the handoff left its adapter open", but the DTO cannot represent that yet. Relax `Scope.adapter` to optional/empty in step 9 (or RFC-104 always pins and Open is only for newly focused sources added after the handoff — record the choice). `catalog::select` / `assign` are the binding-phase call sites; pass `prior` from an existing `discovery.yaml` on `--force`.

### Step 9 — Plan model extension, `discovery.yaml`, wave-binding phase [x]

**RFC anchors:** D1 (`discovery.yaml`, `imports/`), D3 (phase 1 + verification paragraph), D5 (whole), D6 (binding outputs); implementation requirement "`Plan` gains…"; acceptance criteria 1 and 4.

**Scope (emery):**

- **Plan DTO extension** (`crates/project/src/plan/model/`): `Plan` gains `discovery-digest`, `leads-digest`, `decomposition-digest`, the `definition` identity block (handoff digest, review identity + event digest, system/system-model/migration-plan/wave ids), a `targets:` map (adapter pin, locator, cid, model-capability-profile id + digest — profile fields filled from step 13 onward), source rows in the shared shape (adapter pin, locator xor value, cid), and required singular `slices[].target` replacing `Entry.project` (closed Open Question 11). Per closed Open Question 9 the `intent` row is the `value` arm only — reserved key `intent`, no `locator`, no `cid` (inline values stay under the plan digest per D2). Hard cut on the wire shape; update every consumer and golden.
- **`Entry.project` → `slices[].target` (closed Open Question 11).** One cut with the rest of the Plan DTO: the field is a required string naming a key in `plan.yaml.targets` — never omitted, never an alias for `project`. Delete `topology.rs`'s sole-project fallback and `resolve_project_binding`'s omit-means-sole-project branch; in-place N=1 still writes `target: <the-one-key>`. Sweep consumers in the same change: propose-kernel `ResponseSlice.project` / `ProjectRef` comments, `model.yaml.project` → `model.yaml.target` plus `slice-model-target-drift`, and status/advance rendering that prints `project:`. `project.yaml`, the `project` crate, and `project_root` are unchanged — **project** remains the product-tree noun.
- **`discovery.yaml`** DTO + writer: reviewed-handoff identities, pinned targets and sources with CIDs and adapter pins. The `intent` source copies 1:1 from the handoff evidence scope as `{ adapter, value }` with no CID. Unknown fields rejected; canonical digest recorded into `plan.yaml.discovery-digest`. No read-policy body (closed Open Question 10 — D9 limits are engine constants, not planning inputs).
- **Wave-binding phase** of detached `plan author` (D3 phase 1): resolve the single current reviewed handoff (step 5), copy byte-identical handoff + review envelopes to `imports/handoffs/<digest>.yaml` / `imports/reviews/<digest>.json`, resolve target locators to exact revisions + CIDs (step 7), validate each target's pinned `.emery/project.yaml` (product identity, `platforms:`, target-axis adapter present — mismatch blocks authoring), pin sources (step 7 + 8; an absent/empty `Scope.adapter` is `Hint::Open` — the field is required today, see step 8 Notes), generate keys (`catalog::assign`, pass `prior` on `--force`), write `discovery.yaml`. The handoff's `intent` evidence scope binds 1:1 onto `discovery.yaml` / `plan.yaml.sources.intent` (closed Open Question 9); a degenerate `--from .emery/system/` handoff always carries that scope, so in-place N=1 still gets intent without a CLI flag. Independent binding reads may run concurrently under the D9 policy's concurrency bound; results merge into one validated document.
- **Host ingest capability (step 7 leftover).** Git/HTTPS I/O lives in `launcher::ingest` and cannot run in the engine guest. This step's in-guest wave-binding must call it through a host capability: add a small `seam` trait (native in-process, guest via a new WIT import implemented by the launcher) rather than pre-resolving locators in the launcher before the guest boots. Always pass `Policy::standard()`. Feed `binding::roots` into `plan archive`'s live set while the change is open. `adapter::Inventory` already exists on both providers (step 8) — do not add a second catalog trait; call `inventory()` then `catalog::select` / `assign`. Thread D9 `inspected-bytes` through fingerprint walks. Overlay the host-resolved adapter version onto fingerprint-selected pins so compiled `0.12.0` is not recorded when the store/registry has a newer identity.
- Retire the old survey-driven author entry path: in-place authoring now also binds a (degenerate, colocated) definition via `--from .emery/system/`. The old `plan author` bindings-from-init flow is deleted (no flag-only bypass) — `--source` bindings and `--intent` sugar go with it; intent arrives only through the handoff. **Step 6 landed the `--from`/`--wave` grammar with `plan-author-unimplemented`; this step replaces that refusal with the binding phase** (and then stops at typed "decomposition pending" until step 15). Note: from this step until step 15, `plan author` produces `discovery.yaml` + imports and stops with a typed "decomposition pending" outcome — the plan cannot be fully authored until the decomposition phases land. The operator accepts a red-ish window on the branch here; existing end-to-end suites that exercised the old author path are quarantined to fixture-driven equivalents progressively through Cut C (record status in Notes).
- `--force` semantics: rebind the same reviewed handoff and redo binding; selecting a changed wave requires a new handoff + review fact.

**Key code areas:** `crates/project/src/plan/model/`, new `discovery` module beside it, `crates/project/src/plan/propose/` (`topology.rs`, `kernel.rs`, `wire.rs`), `crates/slice/src/model.rs` + `validate/model_drift.rs`, `crates/project/src/plan/{status,advance,execution,projection}.rs`, `crates/change/src/orchestrate/author.rs`, `crates/change/src/plan/handlers/author.rs`, answer-schema goldens (`crates/project/answers/`), `crates/transport/` projections.

**Tests / gates:** acceptance-criterion-4 matrix (missing/ambiguous handoff, missing review fact, edits, unknown fields, stale revisions, changed pins all block); byte-identical import retention; empty-non-git-dir authoring reaches `discovery.yaml` (criterion 1, binding half); degenerate-handoff `intent` evidence scope binds 1:1 as `{ adapter, value }` with reserved key and no CID; an `intent` locator on the handoff or plan row is refused; `slices[].target` is required (omitted or unknown key is a typed refusal; sole-target in-place still serializes the key; a leftover `project:` field is unknown-field rejection). If RFC-104 has landed by this step, run one manual in-place `--from .emery/system/` smoke over a real degenerate definition home authored via the `emery system` verbs (the binding phase's best real-seam check); CI coverage stays fixture-driven. `cargo make ci`.

**Notes (2026-08-13):**

- Plan DTO hard-cut: `deny_unknown_fields` (leftover `project:` is unknown-field); required `slices[].target`; `SourceBinding` is adapter pin + locator xor value + optional cid; `intent` is value-only (no locator, no cid). `Discovery` writes `discovery.yaml`; its canonical digest is `plan.yaml.discovery-digest`. `Layout::with_change_root` / `ExecutionPaths::layout()` serve detached homes; `discovery.md` stays until step 10.
- Wave-binding replaces `plan-author-unimplemented`. `--from`/`--wave` required; `--source`/`--intent` deleted. Success is `AuthorBody.pending: "decomposition"` (not a halt error) with a skeleton `plan.yaml` (`slices[]` empty). Detached author skips `Ctx::load`. `--force` rebinds the same `definition.handoff_digest`; a changed wave is `plan-author-handoff-changed`.
- Bind is sequential (1), not concurrent under D9 concurrency 4. Ingest-host and fingerprint meters are split budgets (`select_metered` takes `budget.as_mut()`). Always `Policy::standard()`. `Scope.locator` is optional; a location-backed scope without one is `source-locator-missing`. Absent/empty `Scope.adapter` is `Hint::Open`. `catalog::overlay` applies only on Open (pin reuse keeps the handoff pin); overlay uses the host-resolved `manifest.version`.
- Host ingest: `project::seam::Ingest` (native in-process; guest via WIT crate `crates/wasi-ingest`, lib `omnia_wasi_ingest`; launcher backend). Intent locator is refused at mint (`definition-intent-form`) and again on a plan row (`source-intent-locator`). `plan archive` feeds binding CIDs into live/dead (`binding_cids`); intent rows have no cid.
- Native ensure matches exact package pins including compiled `0.0.0`. `adapter upgrade --all` walks only `project.yaml`'s bare target — plan rows are all pins. `target_policy::fresh` resolves `plan.yaml.targets[entry.target]`; `best_effort_advance` delegates to it.
- Propose kernel: `ResponseSlice.target` required. **`resolve_topology` still synthesises one `ProjectRef` from live `project.yaml` (`name` = product name).** After binding, `plan.yaml.targets` keys are handoff target ids. Step 15 must retarget topology to those keys or `propose_from` / `plan-reconcile-target-unknown` will disagree. Judgment `propose` is `allow(dead_code)` until then.
- Fixtures: `write_plan_fixture` / `write_greeting_plan` / `write_adversarial_plan` pin the target from `project.yaml.adapter` @ `0.0.0` so fail-adapter sessions match `metadata.yaml.target`; `discovery.md` lead blocks stay `### <source>:<lead>` until step 10. Locator fixtures are a local path plus a real `.emery/project.yaml`. Inline `value` sources record no cid (`inputs.sources` is locator-only); value drift stales the planning `entry` projection, not a source-tree pin.
- Quarantine: e2e author suites now fixture-driven. `crates/probe` seeds a fixture plan from case source bindings (`seed_authored_plan`) instead of invoking retired `--intent`/`--source`; live eval until Execute stays red until step 15 retargets `--from`/`--wave`. README / tutorials / `docs/reference/quick-reference.md` still describe the old author path — steps 15/19/20. First-party adapters still tell the model to parse `plan.yaml` from the lent workspace (`BINDING_NOTE`) until step 12. RFC-104 write surface has not landed; no in-place `--from .emery/system/` smoke. Adding `Ingest` to the native provider grew eval futures past clippy's 16KiB `large_futures` cap; `probe::case::run` and the composition client `Box::pin` them.

---

## Cut C — leads, decomposition, profiles, proposals

### Step 10 — Canonical `leads.md` catalog + revision retention [x]

**RFC anchors:** D1 (`leads.md` / `leads/<digest>.md` paragraphs), D3 (focused-scope import), acceptance criterion 6.

**Scope (emery):**

- Reshape the lead inventory into the authoritative parsed catalog `leads.md`: per source key, every lead id, synopsis, topic, parent relation, and source-local focus. Extend the `artifacts::discovery` parser (rename module to `artifacts::leads`) with parent/focus fields; canonical `leads-digest` over the parsed content.
- Wave import populates the initial catalog from the handoff's evidence scopes (D3 phase 2's import half); focused-survey appends land in later steps but the catalog shape and append semantics (new revision, never mutate a referenced meaning) land here.
- Revision retention: before any decomposition revision or build fact references a `leads-digest`, copy the exact document to `leads/<digest>.md`; immutable thereafter.
- Wire `refinement.yaml.inputs.planning.leads` to the real contributing-lead-closure digest over the new catalog (the projection machinery exists; it now reads `leads.md`).
- `discovery.md` disappears as an artifact name (closed Open Question 5): `leads.md` is catalog-only — no prefix/suffix, and `Discovery::set_preamble` is deleted. Status / freshness / the propose kernel / `plan add` / survey-merge retarget `layout.leads_path()` (they already consume only `Discovery::leads()`). Delete `gate.discovery-summary` and `gate.discovery-source-inventory` from the proposal answer schema, `propose.md`, and the mock/scripted answers; `gate.change` remains. Do not add new `change.md` sections here — step 15's author frame consumes the engine-authored orientation (counts + binding table) recorded in the closed question. Adapter-prompt and docs name-sweeps wait for steps 12 and 20.

**Key code areas:** `crates/artifacts/src/discovery/` → `leads/`, `crates/change/src/orchestrate/{author,survey}.rs`, `crates/project/src/{config,plan/projection,plan/propose/wire,plan/status}.rs`, `crates/project/answers/proposal.schema.json`, `crates/change/prompts/propose.md`, retention plumbing in the change-home layout.

**Tests / gates:** catalog parse/serialize round-trip (no preamble); digest stability; retention-on-reference; criterion-6 edit-invalidates-digest cases; proposal-schema golden regen. `cargo make ci`.

**Notes (2026-08-13):**

- Markdown document type is `artifacts::leads::Leads` (not `Discovery`) so it does not collide with `project::plan::Discovery` (`discovery.yaml`). Error codes are `leads-parse-failed` / `leads-lead-schema` / `leads-lead-unknown`.
- Catalog-only: `## Lead inventory` plus `### <source>:<lead>` blocks; preamble/suffix refused; `Discovery::set_preamble` deleted. Canonical digest is YAML of `{ version: 1, leads: [...] }` sorted by `(source, lead)`, hashed as bare hex. Empty `parent` / `focus` / `topics` skip serialization so existing refinement lead-projection digests stay stable for rows without those fields.
- Wave import: one catalog row per assigned plan source key, zipped with handoff evidence scopes in order. Synopsis is the inline `value` for intent (and any value-backed scope), otherwise the lead id. Parent/focus are `None`. Author writes `leads.md`, calls `retain_leads`, stamps `plan.leads_digest` — that is the first reference. Retention filename is `leads/<64-hex>.md` via `SnapshotId::digest()` (no `sha256:` colon).
- Survey-merge (`orchestrate::survey`) rewrites the current `leads.md` only and does **not** call `retain_leads` or bump `plan.leads_digest` — it remains the debug breakout. WIT/seam `Lead` is unchanged (`{lead, source, synopsis, topics}`); survey maps `parent: None, focus: None`.
- `gate.discovery-summary` / `gate.discovery-source-inventory` deleted; `gate.change` remains. `proposal.schema.json` regenerated. `report.schema.json` / `phase-report.schema.json` also regenerated because `Artifact::Plan` rustdoc now names `leads.md`; the adapter `phase-report` pin is a symlink again (it had become a regular file). Archive co-moves `leads.md` + `leads/` + `discovery.yaml` (no `discovery.md`).
- Docs/glossary/skills still say `discovery.md` (step 20). The rustdoc path links in `docs/standards/workflow.md` were retargeted to `crates/artifacts/src/leads/{lead,document}.rs` so the links gate does not 404. AGENTS.md vocabulary and the never-hand-edit list name `leads.md`.
- `resolve_topology` still synthesises one `ProjectRef` from live `project.yaml` (step 9 leftover; step 15).

### Step 11 — Source WIT extension: value-in, focused survey, child leads [ ]

**RFC anchors:** D2 (source key + workspace-or-value; they never parse `plan.yaml`; focused-survey paragraphs), implementation requirement "The source WIT receives either a read-only workspace or inline value. Extend `survey` with an optional parent-lead focus and stable child-lead response…". D2's "read-only change artifacts" for sources is typed catalog context on that input, not a filesystem grant (closed Open Question 7).

**Scope (emery — WIT + SDK + engine dispatch; adapters follow in step 12):**

- WIT `source` interface: `survey` gains a typed input carrying the source key, either a read-only workspace handle or the inline value, and an **optional parent-lead focus** (full `Lead`: id, synopsis, topics, parent, focus). The response distinguishes top-level leads from stable child leads under the named parent. `extract` gains the same value-in shape; the terminal `(source, lead)` is unchanged as identity, but the `Lead` record carries parent/focus so child extraction inherits catalog context on the wire. No third source operation. No change-home path, catalog file, or slice-artifact root on the record.
- SDK (`crates/adapter`): `Source` trait and seam DTOs updated (`Lead` gains parent/focus fields aligned with step 10's catalog); `LEADS_ANSWER_SCHEMA` regenerated; `source!` macro re-wired. This is a WIT-breaking change; step 12 consumes it on the same branch (closed Open Question 13 — no intermediate release; the two PRs land together). Note the break so the next session runs step 12.
- Engine dispatch (`crates/change/src/orchestrate/survey.rs`, `crates/slice/src/source.rs`): prepare the read-only source view from the recorded CID (step 7) and pass it (or the inline value); look up the focused parent or terminal lead in the catalog and pass the record — do not mount `leads.md`, `plan.yaml`, or `slices/`. The source guest's `"."` is not the change root; the agent lend is the source CID view (RFC-87 D2 empty-writable-scope prepare) or nothing for inline `value`. Sources never receive the project or change-home preopen. The engine owns which lead is focused and merges child leads into the catalog as a new revision (stable merge order). Unfocused survey always returns the complete current set; the adapter does not read the catalog to decide it is a re-survey.
- Mock adapter (`crates/mock`) updated, including scripted focused-survey answers, so engine integration tests cover the new seam without the adapters repo.

**Key code areas:** `wit/emery.wit` (via `crates/adapter`), `crates/adapter/src/{operations,seam,source,answers}.rs`, `crates/change/src/orchestrate/survey.rs`, `crates/slice/src/`, `crates/mock/`, `crates/{native,guest}` providers.

**Tests / gates:** engine-side focused-survey integration over the mock catalog; extract-over-view tests (terminal `Lead` carries parent/focus; no change-home / `leads.md` / `slices/` preopen on the source dispatch); schema goldens regenerated; `cargo make ci` + wasm32 check.

**Notes (from step 10):** Engine-owned focused survey must write a **new** `leads.md` revision and `retain_leads` when the new digest is published onto the plan; do not overwrite `leads/<old-digest>.md`. Today's survey-merge does not update `plan.leads_digest`. Extend WIT/seam `Lead` with parent/focus to match the catalog type (`artifacts::leads::Lead`); the catalog already stores those fields. Import synopses are value/lead-id placeholders until focused survey fills them.

### Step 12 — Update the five source adapters to the new seam (adapters repo) [ ]

**RFC anchors:** as step 11; adapters-repo AGENTS.md contract.

**Scope (emery-adapters, path-patched to sibling engine):**

- The adapters repo is expected not to build against the sibling engine between steps 11 and 12 — that window is intra-branch only (closed Open Question 13). The adapters PR lands with the engine PR.
- Rework `sources/{intent,documentation,typescript,screenshots,captures}/src/operations.rs` to the new trait: consume the passed source key, workspace-or-value, and optional parent/terminal `Lead` (parent/focus on the wire). Delete every `BINDING_NOTE` that tells the model to parse `plan.yaml` from a lent project or change-home tree (as of step 1 those notes still say "workspace root", which is already wrong for in-place `.emery/change/plan.yaml`). Implement focused survey (return stable child leads under the requested parent, inheriting parent context from the passed `Lead`, not from `leads.md` or slice files).
- Update survey/extract prompts (`prose/prompts/`) for the new input model and focused re-survey semantics: `$SOURCE_DIR` is the CID view (absent for inline `value`); the change home and `$PROJECT_DIR` are unreachable; do not look leads up in `leads.md` / `discovery.md` or read `slices/<slice>/`.
- Target adapters: audit for assumptions broken by steps 2–3 (baseline now inside the workspace tree; merge runs against a workspace, not the checkout) — expected small; contracts preflight/postflight baseline reads are the main suspects.
- Update adapter native tests (`tests/operations.rs`) to the new DTOs.

**Tests / gates:** `cargo make check` + `cargo make ci` in emery-adapters; focused-survey unit coverage per source adapter. Live eval rungs stay operator-invoked and are not gating here.

### Step 13 — Model-capability profiles [ ]

**RFC anchors:** D3 (profile paragraphs: closed assessment dimensions, engine-computed weighted sum, operation thresholds), acceptance criterion 5 (profile digests as planning inputs); RFC-92 note (it patches the profile shape later — keep the DTO closed but versioned).

**Scope (emery):**

- Closed `ModelCapabilityProfile` DTO: id, version, weights over the five dimensions (behavioural breadth, coupling, uncertainty, context volume, verification surface — integers 0–10 come from judgment; weights and thresholds from the profile), thresholds (`slice-split` consumed here; `task` recorded for RFC-96), canonical digest.
- Engine resolution of one profile per target from the configured model class, per closed Open Question 4: compiled-in versioned defaults keyed by model class (initial table is the single `frontier-large-v1` entry with the RFC worked-example weights/thresholds, documented as declared starting values), overridable only by composition-time whole-table host supply mirroring step 8's catalog seam — never a model answer, never project state, and no `model-class` field on plan/project/target rows.
- Persistence: full closed bodies + ids + digests into `decomposition.yaml` (step 14 writes it; this step supplies the type and resolution), ids + digests copied into `plan.yaml.targets`.
- `refinement.yaml.inputs.profile` becomes the real bound-target profile digest; freshness recomputes it; changing a profile stales manifests and (with step 18) invalidates the epoch.

**Key code areas:** new `crates/project/src/profile.rs` (or under `plan/`), `crates/project/src/refinement.rs` + `crates/slice/src/refinement/`, provider/composition plumbing for the host override hook (closed Open Question 4).

**Tests / gates:** digest stability; scoring kernel (weighted sum + threshold application) dense matrix; host-supplied table replaces the compiled default and produces a distinct digest; manifest staleness on profile change. `cargo make ci`.

### Step 14 — Decomposition substrate: DTOs, validators, compiler, projection [ ]

**RFC anchors:** D1 (`decomposition.yaml`, `decompositions/<digest>.yaml`), D3 (split/leaf validation rules, containment-vs-dependency, budgets, deterministic engine ownership), implementation requirement "Add the closed `decomposition.yaml` shape… RFC-95 target-contraction cycle check"; acceptance criteria 5 and 6; RFC-95 D4/implementation note (shared contraction kernel).

**Scope (emery — deterministic only, no judgment):**

- `decomposition.yaml` DTO: version, `leads-digest`, model-capability-profiles (full bodies from step 13), root, nodes (children, parent, contributing `(source, lead)` scopes, target bindings, ownership envelopes, dependencies, local gate kinds, terminal `slice:` mapping). Unknown fields rejected; canonical digest; revision retention at `decompositions/<digest>.yaml` on first reference. Depth/node/judgment/repair budgets are compiled engine constants, not fields on this DTO (closed Open Question 10).
- Deterministic validators, each a typed diagnostic: at-least-once lead coverage; cross-cutting lead retention on every informed child; child-target-set containment; strict normalized-scope-measure reduction per split; at-most-one-terminal-lead-per-source per leaf; sibling ownership-overlap requires explicit order or fan-in child (ambiguity blocks); leaf completeness (one target, ownership manifest, acceptance boundary); depth/node budget enforcement against the compiled caps (`MAX_DECOMPOSITION_DEPTH` 8, `MAX_DECOMPOSITION_NODES` 64 — closed Open Question 10); acyclic leaf graph; **target-contraction cycle check** (`publication-target-cycle`) as a pure kernel shared with future RFC-95 validation.
- Domain-dependency compiler: expand domain→domain dependencies into exit-leaf → entry-leaf `depends-on` edges, deterministically.
- Deterministic projector: terminal domains → `plan.yaml.slices` (byte-stable), copying bound topology and digests including required `slices[].target` (closed Open Question 11 — never omit, never write `project`); exact-projection validator (plan ↔ decomposition drift is a typed failure; hand-edited drift never executes).
- `refinement.yaml.inputs.planning.decomposition` switches from the single-node placeholder projection to the real leaf-scoped projection (retained ancestry, dependency closure, terminal mapping) — RFC-91 D4 declared the fields absent-as-canonical-empty precisely so this lands without staling unrelated digests; verify that property with a test.
- One-slice degenerate tree (root → leaf) covered.

**Key code areas:** new `crates/project/src/plan/decomposition/` (DTOs + validators + compiler + projector), `crates/project/src/plan/projection.rs`, retention plumbing, extensive fixture-based tests.

**Tests / gates:** validator matrix over hand-built fixtures (every blocking rule); byte-stable projection; contraction-cycle fixtures (leaf-acyclic but target-cyclic); ≥3-level multi-target fixture; revision retention. `cargo make ci`.

### Step 15 — Decomposition judgment legs + full detached `plan author` [ ]

**RFC anchors:** D3 (phases 2–3, judgment `split | leaf` responses, provisional complexity, bounded boundary review, focused-survey requeue, budget exhaustion), platform § scope discipline ("the engine owns recursion, budgets, validation, and projection"); acceptance criterion 5.

**Scope (emery):**

- Judgment answer schemas + prompts (in `crates/change/prompts/`): typed `split | leaf` partition response (children with scopes, targets, ownership, dependencies, rationale) and the closed five-dimension complexity assessment; schema goldens under `crates/project/answers/` conventions.
- Engine-owned recursion: deterministic queue over open domains, one bounded judgment per dispatch, validation via step 14's kernels, per-leg `MAX_REPAIRS` (2) on invalid responses (no second global repair budget), `MAX_DECOMPOSITION_JUDGMENTS` 128 covering split/leaf dispatches including focused-survey requeue (closed Open Question 10 — compiled constants, not persisted), requeue-after-focused-survey (step 11's engine-controlled focus), leaf-readiness gate (provisional score vs `slice-split` threshold → one bounded boundary review → close-with-rationale / focused-split / unready-blocks-authoring). Exhaustion parks.
- Wire `plan author` end to end: bind (step 9) → focus delivery scopes (import + focused survey where a broad scope needs child detail) → decompose → validate complete tree → publish `decomposition.yaml` + `plan.yaml` together (complete-tree policy) → author `change.md` review prose. Per closed Open Question 5, the deterministic frame carries engine-authored survey orientation (Sources/Leads counts plus the binding table restated from `discovery.yaml`); `gate.change` remains the sole model-authored body (Intent/Scope, plus uncertainties, tentative merges, and likely divergences preserved for the operator). This closes step 9's `pending: "decomposition"` window (empty `slices[]`, judgment `propose` currently `allow(dead_code)`).
- **Retarget `resolve_topology`.** Step 9 left `propose/topology.rs` synthesising one `ProjectRef` from live `project.yaml` (`name` = product name). Binding writes `plan.yaml.targets` keyed by handoff target ids, and `ResponseSlice.target` / `plan-reconcile-target-unknown` match against topology names. Feed `plan.yaml.targets` keys (and adapter pins) into the request topology — do not keep the product-name fallback.
- **Retarget probe argv.** `crates/probe/src/case.rs` currently seeds a fixture plan (`seed_authored_plan`) from case.toml source strings — wave-binding `plan author` still stops at decomposition pending. Drive `--from`/`--wave` over fixture definition homes here and delete the seed helper. Step 19 still reworks adapters-repo `case.toml` shape.
- Journal: reuse `plan.reconcile.completed` for the successful author write (or extend its payload with the new digests — decide in-session and record); no new authorization semantics.
- Uncertain boundaries and estimate caveats render into `change.md`; a source→target contradiction with the reviewed wave escalates as the D8 definition-revision request (inert stub here; full proposal machinery is step 17 — emit the typed stop now, attach the proposal document shape in 17).

**Key code areas:** `crates/change/src/orchestrate/author.rs` (+ new `decompose.rs`), `crates/change/src/judgment/`, `crates/change/prompts/`, `crates/project/src/plan/propose/topology.rs`, `crates/probe/src/case.rs`, `crates/mock` scripted answers for the new judgment legs.

**Tests / gates:** end-to-end detached author over mock catalog + fixture definition (single-leaf degenerate; multi-target ≥3 levels; budget exhaustion parks; invalid-split repair path; overlap ambiguity blocks). This closes step 9's "decomposition pending" window — the full author path is green again. `cargo make ci`.

**Notes (from step 10):** `gate.change` remains the sole model-authored body; do not revive `gate.discovery-summary` / `gate.discovery-source-inventory`. Imported catalog synopses are value/lead-id placeholders until focused survey (step 11) fills them — the engine-authored orientation in `change.md` (counts + binding table) should not treat those placeholders as survey-grade headlines. `resolve_topology` leftover from step 9 still applies.

### Step 16 — Refinement boundary escalation [ ]

**RFC anchors:** D3 (refinement paragraphs: typed `proceed | boundary-escalation`, Evidence-informed reassessment, focused resurvey + nearest-domain re-decomposition into an **inert** proposal, no artifact promotion, budgets/parking), implementation requirement "The refinement judgment adds the typed `proceed | boundary-escalation` outcome"; acceptance criterion 9 (refinement half).

**Scope (emery):**

- Extend the synthesis/refinement judgment with the typed outcome before bundle promotion: `proceed` continues today's path; `boundary-escalation` names affected terminal `(source, lead)` pairs + typed rationale, validated against the pinned profile and the same closed dimensions.
- On escalation: run focused survey for the named parents (engine-controlled), build the candidate lead-catalog revision and candidate nearest-domain decomposition revision, and persist them **inert** inside one amendment-proposal document (shape from step 17 — coordinate: if 16 runs before 17, land the proposal DTO skeleton here and let 17 own application; record in Notes). Current `leads.md` / `decomposition.yaml` / `plan.yaml` unchanged; no `refined` transition; no build work; child slices later re-extract their own child leads (parent Evidence not reused).
- Budget exhaustion parks the leaf with a typed stop; `plan refine` re-entry behaviour and `plan status` next-actions updated.

**Key code areas:** `crates/slice/src/orchestrate/` (synthesis judgment), `crates/slice/src/answers.rs` (+ goldens), `crates/change/src/orchestrate/refine.rs`, prompts.

**Tests / gates:** scripted-escalation integration (inert proposal produced, planning artifacts untouched, no transition); proceed-path regression; parking. `cargo make ci`.

### Step 17 — Amendment proposals + `plan amend --proposal` [ ]

**RFC anchors:** D1 (`planning/proposals/`), D8 (ownership proposals, boundary proposals, envelope escalation shape for RFC-96, definition-revision requests, compare-and-set application, preservation rules, lowering of direct plan mutations); acceptance criterion 9 (application half); implementation requirement "Add closed ownership and refinement-boundary proposal DTOs plus `emery plan amend --proposal <digest>`…".

**Scope (emery):**

- Closed proposal DTOs under `planning/proposals/<digest>.yaml`: **ownership** (nearest domain, new dependency or fan-in leaf, expected planning digests, expected accepted CID per target, committed leaf→wave set, affected open-wave/claim frontier), **boundary** (failed leaf, assessment + profile digest, candidate lead catalog, candidate nearest-domain decomposition, same expected frontiers), the **envelope** escalation record shape (authored by RFC-96 later — DTO only, no producer), and the inert **definition-revision request** (conflicting handoff reference, stops the affected scope; not an amendment).
- `emery plan amend --proposal <digest>`: validate → compare-and-set every expected revision and accepted frontier → refuse live affected claims/waves (operator quiesces first) → for boundary proposals atomically activate candidate lead + decomposition revisions → reproject `plan.yaml` via step 14's projector → retain new revisions → invalidate the old closed-plan epoch. Preservation checks: committed leaves keep identity, binding, target, dependencies, terminal mapping; accepted leaves may gain dependants but cannot be removed/rebound/reordered-behind/depublished. Stale, malformed, cyclic, accepted-history-changing, or ambiguous proposals change nothing (typed refusals).
- Lower existing `plan add` / `amend` / `remove` to the same domain-mutation + reprojection path; refuse when no unambiguous hierarchy edit exists. `plan drop` (in-scope exclusion) is audited against the new model but keeps its semantics.
- Runtime ownership-overlap detection (build/merge surfaces) authors ownership proposals but never applies them — wire the authoring hook where overlap is detected today (merge staleness/validate diagnostics), keeping it inert.

**Key code areas:** new `crates/project/src/plan/proposal/`, `crates/change/src/plan/handlers/amend.rs` (+ new proposal leg), `crates/change/src/orchestrate/`, transport grammar.

**Tests / gates:** compare-and-set matrix (each expected-revision mismatch refuses); preservation-violation refusals; boundary activation + reprojection round-trip; add/amend/remove lowering including refusal; epoch invalidation observed by step 18's checks (assert the fact-side effect now, full chain in 18). `cargo make ci`.

---

## Cut D — execution, fixtures, closure

### Step 18 — Execute digest chain + detached accepted-CID execution [ ]

**RFC anchors:** D7 (execution over accepted CIDs, workspaces from accepted CID, read-only change mounts), D8 (verification list, coverage contents), D9 ("execution never repeats target binding"); implementation requirements on coverage; acceptance criteria 7, 8, 11.

**Scope (emery):**

- **Verification chain at execute start** (before any epoch/workspace/wave): plan ↔ `discovery.yaml` match; imported handoff + review-envelope bytes match digests and identities; the definition's *current* handoff still has its review fact and names the selected wave; `leads-digest` identifies the retained revision; plan is the exact projection of `decomposition.yaml`; recorded source/target/CID/adapter pins remain valid; profile digests match. Each failure a typed diagnostic; authoring-time validation (steps 9/14/15) shares these kernels — extract them accordingly.
- **Coverage:** `plan.execute.started` carries `plan-digest`, **required** `discovery-digest` (populate the existing optional field and tighten it), and sorted per-leaf refinement digests. The plan digest transitively binds definition/system-model/migration-plan/wave/lead/decomposition/profile digests — verify transitivity with a test that mutates each underlying revision and observes epoch invalidation. D9 read-policy and D3 depth/node/judgment/repair constants are not on that chain (closed Open Question 10); a binary bump of a starting value does not invalidate an in-flight epoch.
- **Detached execution loop:** serial scheduler selects leaves whose dependencies are accepted (per target); wave open uses `Wave::one_member` + `enforce_one_member` against the bound target's accepted-CID projection (step 3) seeded from `plan.yaml.targets[].cid` — still a one-element `members` vec, not a scalar `Wave` (closed Open Question 12); workspaces prepare from the accepted CID with change artifacts mounted read-only (two-root dispatch from step 6); build/verify/repair/review machine unchanged (RFC-90); merge (step 3) iterates the member list and advances the per-target accepted CID; dependent leaves across targets open later waves against accepted results; multi-target drain to one accepted CID per touched target and **no commit or branch** (RFC-95's job). Multi-member waves remain RFC-96; this step does not retire `enforce_one_member`.
- In-place execution runs the identical path (the change home is just colocated); the checkout is never a write target.
- **Detached `Ctx` / `Layout`.** Step 6 left `Layout::new` in-place-only and `Ctx::load` requiring `project.yaml`. This step must teach execute (and `plan status` / `gaps` / `emery debt` in a detached home) to load without init: a `Layout::detached` (or equivalent) over `ExecutionPaths.change_root()`, skip `ProjectConfig::load`, and never treat the `.` mount as a product tree. Guest freeze must use the accepted CID, not `snapshot(.)` — a detached `.` is the change home.
- `plan status` / `plan gaps` / `emery debt` / `plan archive` audited against the new chain (archive's carried-debt summary, sweep of change-scoped GC roots including plan CIDs, RFC-95 gate note). **After step 3, `emery debt` and `plan author`'s `## Carried debt` still read checkout `.emery/specs/`**, which merge no longer writes — this step must re-home those projections onto the accepted CID (prepare a read-only view of it) so the operator backlog survives in-place. Do not leave them dark until RFC-95: they are fact-adjacent inventory, not merged product code. `scratch/` remains under `.emery/` and still hashes into an in-place freeze of `.`; do not golden freeze digests against that tenant.

**Key code areas:** `crates/change/src/orchestrate/{execute,epoch}.rs`, `crates/change/src/orchestrate/` scheduler, `crates/slice/src/orchestrate/target.rs`, `crates/project/src/journal/event.rs` (coverage payload), status/gaps handlers.

**Tests / gates:** acceptance-criteria 7/8/11 integration fixtures over the mock catalog (two-target plan with a cross-target dependency; postflight-failure resumable stop; live-claim-without-epoch refusal; stale-definition refusal mid-plan); `cargo make ci` + wasm32 check.

### Step 19 — Definition-home fixtures, eval and wasm cases (adapters repo) [ ]

**RFC anchors:** implementation requirement "Integration tests use reviewed definition fixtures, local repository-host, HTTP, content-addressed store, and component fixtures"; adapters-repo testing map.

**Scope (emery-adapters):**

- Rework the eval runner (`examples/eval/`) case shape: `case.toml` supplies a definition home (fixture or generated via the engine fixture builder exposed through `probe`/lab shim) + wave id instead of intent/source strings; runner drives `plan author --from --wave → plan refine → plan execute` and grades against accepted CIDs materialized in-process via `project::workspace::Store::materialize` through `probe` (per closed Open Question 2 — no CLI surface; the runner already materializes result snapshots this way) instead of checkout trees. Per closed Open Question 1, the case shape treats the definition home as an opaque supplied input — fixture-built or real — never a builder-only artifact; if RFC-104 has landed by this step, add one live case driving `emery system survey → plan → review → plan author --from` end to end (the only rung that exercises the real seam between the two RFCs), without making it a gating requirement of this plan.
- Update `orders-contracts` and `omnia-r9k` cases and the wasm example scripts (`examples/wasm/`) to the new choreography; add one multi-target workflow case (even a small two-target contracts+omnia fixture) to exercise cross-target dependencies live — this is the only rung that exercises D7 across targets with real adapters. Per closed Open Question 2 the wasm rungs assert on exit codes, journal facts, and `build/report.yaml` only — merged code stays store-only until RFC-95. If output visibility proves necessary during this step, the only permitted home for a materialize convenience is the dev-only lab shim (`cargo make lab`), never the shipped binary; decide in-session and record it in Notes.
- Confirm the five updated source adapters (step 12) behave over pinned read-only source views in the wasm rung. The cases ship in the adapters PR that lands with the engine PR (closed Open Question 13); they do not need a mid-plan engine tag.

**Tests / gates:** `cargo make ci` in emery-adapters; eval/wasm rungs remain operator-invoked (run at least `wasm-contracts` once before closing the step; record the outcome in Notes).

### Step 20 — Documentation closure and final gates (both repos) [ ]

**Scope:**

- **emery:** rewrite affected prose — `AGENTS.md` (vocabulary: change home paths, `discovery.yaml` / `leads.md` / `decomposition.yaml`, targets map, accepted CID, removed `apply`/`slice.code.applied`, detached mode, cwd-as-change-root, source seam is value-in + typed catalog context with no change-home grant), `docs/standards/workflow.md` (the workflow contract spans both repos), `docs/reference/` and `docs/explanation/` pages touching artifacts/adapter-contract/provenance (adapter-contract sandbox: sources do not get `$PROJECT_DIR` or the change home), skills wrapper text (`plugins/emery/` — `/emery:plan` elicits `--from`/`--wave`; later skills inherit workspace cwd as the change root and may elicit `--change-dir`), RFC status headers (RFC-87: `apply` deleted; RFC-86: stand-ins resolved; RFC-88: status → implemented contract wording per house style; platform.md "Where we are"). Run the AGENTS.md rule: `rg` every removed/renamed symbol across Rust *and* prose in both repos.
- **emery-adapters:** `AGENTS.md` (source contract now value-in + focused survey; no change-home filesystem grant — catalog context is typed on the call), `docs/authoring.md`, `docs/testing.md`, eval README.
- Final gates: `cargo make ci` in both repos, wasm32 compile check, `cargo make links`.

**Notes (from step 10):** Rustdoc path links in `docs/standards/workflow.md` already point at `crates/artifacts/src/leads/{lead,document}.rs` (required for the links gate after the module rename). Remaining `discovery.md` prose in docs, glossary, skills, and CLI reference is this step's name sweep. AGENTS.md vocabulary and the never-hand-edit list already name `leads.md`.

---

## Open questions

All thirteen questions are closed (2026-08-13). Resolutions are recorded below; affected steps are amended in place. New questions discovered during a session go in that step's **Notes** block.

1. **RFC-104 sequencing. — CLOSED (2026-08-13).** RFC-88's public surface (`plan author --from <definition-home> --wave <id>`, no bypass) consumes RFC-104 outputs that nothing can produce yet. RFC-104 is now imminent — landing just before this plan or very shortly after it — which keeps the original recommendation's decision but changes two of its details. **Resolution:**
   - **Proceed without sequencing behind RFC-104.** Cut A is RFC-104-independent; the decision only becomes load-bearing at step 5.
   - **The step-5 `Handoff` DTO + canonical digest kernel in `crates/project/src/definition/` is coordinated RFC-104 code delivered early** — exactly one shared implementation, agreed with the RFC-104 implementers before step 5 runs. Any handoff-shape change during RFC-104's development is a breaking change both efforts absorb together; a drifted digester would fail every retained import's verification chain when RFC-104 lands.
   - **`system.wave.reviewed` joins the closed `EventKind` as a writer-less definition-scoped variant**, replacing the previously planned foreign-envelope parsing machinery — RFC-104's implementation requirements extend the taxonomy with this kind anyway; RFC-88 still never writes it.
   - **The fixture builder is permanent test substrate either way**: the `emery system` orchestrations are judgment-bearing and live-model, so CI can never mint definition homes through them even post-RFC-104.
   - **If RFC-104 lands before Cut B begins, step 5 rebases** to consume its types (see step 5's rebase rule) and the "operator workflow is fixture-only" caveat is struck; step 9 gains a real-home smoke path and step 19's case shape accepts real and fixture-built homes alike.

   Steps 5, 9, and 19 are amended accordingly. Closed Open Question 8 independently locks the shared home as a module in `project`.
2. **Interim access to accepted results (pre-RFC-95). — CLOSED (2026-08-13).** After step 3, merged code exists only as store snapshots; nothing materializes it for the operator, eval graders, or manual inspection until RFC-95. **Resolution: no interim CLI surface — the alternative is adopted.** The operator accepts the blind spot until RFC-95 lands:
   - **No new verb, even temporarily.** The shipped `emery` grammar gains no materialization surface; `emery target materialize` is not built.
   - **Eval grading needs no verb.** `project::workspace::Store::materialize` is already public kernel API and `probe`'s case runner already materializes result snapshots in-process (see `crates/probe/src/case.rs`); step 19 repoints the graders from checkout trees to the accepted CID via the same path.
   - **The wasm rungs go dark on output until RFC-95.** `cargo make wasm-contracts` / `wasm-omnia-r9k` can no longer show merged code (the sandbox project tree is never written after step 3); their observable outcome degrades to exit codes, journal facts, and `build/report.yaml`. This is consciously accepted.
   - **Escape hatch, if the darkness bites:** any future materialize convenience must live in the dev-only lab shim (the adapters-repo `eval` binary behind `cargo make lab`), never the shipped binary. Whether to add it is a step-19 in-session call; record the decision in that step's Notes.
   - **Step 4 dissolves** (the projection lands in step 3, the kernel exists, the grader repointing is step 19's scope); steps 3 and 19 are amended accordingly.
3. **In-place hard-cut confirmation. — CLOSED (2026-08-13).** Step 1 relocates the change home to `.emery/change/` and step 3 stops writing merge results to the checkout, both without migration. **Resolution: pre-1.0 hard-cut posture confirmed — the operator does not care about migration.** No fallback layout reads, no compatibility shims, no migration framework; existing projects re-init and live changes are recreated. The output-visibility consequence of the checkout-write removal was independently accepted in closed Open Question 2. Steps 1 and 3 proceed exactly as written — they already assumed this answer; only the confirmation gate is removed.
4. **Model-capability profile provenance. — CLOSED (2026-08-13).** D3 says the profile is "an engine input, not a model answer" resolved "for the configured model class", but not where it lives. **Resolution: the recommendation is adopted — compiled-in versioned defaults keyed by model class, with a host-composition override hook; a profile is never a model answer and never a project artifact.**
   - **Profiles are engine constants first.** The closed `ModelCapabilityProfile` DTO and the default profile table live in `crates/project/` beside the existing engine budget constants (`MAX_REPAIRS` precedent): versioned Rust constants keyed by model class, not on-disk configuration the engine must discover or validate at runtime.
   - **The initial table has exactly one entry.** `frontier-large-v1` with the RFC's worked-example values — weights `behavioural-breadth 3, coupling 4, uncertainty 2, context-volume 1, verification-surface 3`, thresholds `slice-split 80, task 35` — documented in-code as declared starting values per the platform's honesty posture, not calibrated measurements. Until a second model class exists, every target resolves to this profile; RFC-88 adds no `model-class` field on `plan.yaml`, `project.yaml`, or a target row — the model-class keying is table shape only, exercised when a real second class arrives.
   - **The override hook is deployment composition, not project state.** Mirroring step 8's adapter-catalog seam, the host (launcher / native composition) may supply a replacement profile table through the provider. The replacement is the whole table, not a per-weight patch, so RFC-92's later `routes` addition stays one digestable body. Nothing in the project tree, `plan.yaml`, an environment variable, or a model answer can substitute a profile — the "deployment-config" override of the original recommendation is realized as composition-time supply, keeping the engine free of a config-file discovery surface.
   - **Provenance needs no extra recording.** Step 13 already persists each profile's full closed body plus digest into `decomposition.yaml` and copies id + digest into `plan.yaml.targets`, so the digest chain covers whichever profile was in force: an overridden profile is distinguishable by digest, and changing one stales refinement manifests and (with step 18) invalidates the epoch exactly as D3 requires. RFC-92 later patches the DTO shape — the DTO stays closed but versioned.

   Step 13 is unblocked and amended accordingly.
5. **Fate of `discovery.md`. — CLOSED (2026-08-13).** RFC-88's change home has `leads.md` + `discovery.yaml` and no `discovery.md`. **Resolution: the recommendation is adopted — `leads.md` replaces `discovery.md` wholesale; the authored summary preamble moves into `change.md`; nothing downstream needs a separate survey narrative document.**
   - **Machine consumers read only the lead inventory.** `plan status` (`load_inventory`), refinement freshness, the propose kernel, `plan add` lead resolution, and survey merge all call `Discovery::leads()` and ignore the prefix/suffix. They retarget `leads.md`. An absent catalog still degrades to an empty inventory.
   - **`leads.md` is catalog-only.** RFC-88 D1: "`leads.md` is an authoritative parsed catalog, not unbound review prose"; `leads-digest` covers source key, lead id, synopsis, topic, parent, and focus. The current `## Summary` / `## Source inventory` prefix cannot ride along: excluded from the digest it would drift without invalidating references; included it would make review-prose formatting churn the digest. `Discovery::prefix` / `suffix` / `set_preamble` go away with the rename.
   - **The preamble is operator orientation, not a second authority.** `## Source inventory` (key / adapter / path-or-value) is a human restatement of binding that `discovery.yaml` already owns (D1/D5). `## Summary` (`Sources: N. Leads: M.`) is deterministic from the catalog. Both move into `change.md` as **engine-authored** sections of the existing `# Change — <name>` frame — not model answers. The proposal `gate` object shrinks to `change` only; `discovery-summary` and `discovery-source-inventory` are deleted from the answer schema and `propose.md`.
   - **Prompts and adapters do not need the document.** Synthesis already only says "never edit `discovery.md`". Source survey/extract prompts name it as the persist target they already do not write; D2 plus steps 11–12 remove that assumption. The name sweep is step 12 (adapter prompts) and step 20 (docs, glossary, skills).
   - **Archive** moves `leads.md` + retained `leads/<digest>.md` (and `discovery.yaml`) instead of `discovery.md`.

   Step 10 is unblocked and amended accordingly; step 15's `change.md` frame consumes the engine-authored orientation.
6. **Detached change-root anchoring. — CLOSED (2026-08-13).** How do post-author verbs find a detached change home, and does `plan author <name>` create a subdirectory? **Resolution: cwd is the change home; `--change-dir` is an optional override; no marker file.** The operator runs from the change directory and accepts the fragility of no ancestor walk.
   - **No marker, no walk.** A change-home marker plus ancestor walk was the original recommendation (the analog of today's `.emery/project.yaml` walk). It is not built. Detached homes have no synthetic `project.yaml` (D1) and need no substitute sentinel.
   - **Detection.** `--change-dir <dir>` selects that path as the detached change root. Else the nearest ancestor carrying `.emery/project.yaml` is in-place (change root `<product>/.emery/change/`, as today). Else cwd is the detached change root. `--change-dir` is the `--project-dir`-shaped optional override on change-scoped verbs — never required, never an always-explicit flag.
   - **Author writes into the resolved directory.** Detached `plan author <name>` uses cwd (or `--change-dir`) itself. `<name>` is the change identity in `plan.yaml`, not a subdirectory. This matches acceptance criterion 1: an empty non-Git directory *is* the change home (`mkdir the-change && cd the-change && emery plan author <name> --from … --wave …`).
   - **Accepted fragility.** Running from a subdirectory or parent of a detached home is operator error (the verb sees that directory, not a walk). A detached home nested inside a product checkout is detected as in-place unless `--change-dir` is passed. Skills inherit the Cursor workspace cwd; the operator opens the change directory as the workspace or passes `--change-dir`.

   Step 6 is unblocked and amended accordingly.
7. **Source-seam change-artifact grant. — CLOSED (2026-08-13).** D2 says source operations receive "read-only change artifacts" without enumerating them. **Resolution: the original recommendation is not adopted — sources receive no change-home filesystem grant; catalog context is typed on the WIT input.**
   - **D2's phrase is the typed lead/catalog fields, not a mount.** The implementation requirement already describes this: workspace-or-value plus optional parent-lead focus. "Never parse `plan.yaml`" is a sandbox invariant, not a prompt convention. Target operations keep RFC-87 D4's `workspace.artifacts` grant; that pattern is not copied onto the source seam.
   - **Survey/extract input.** Source key, workspace-or-value, and (when focused or extracting) the relevant `Lead` record with parent/focus filled from the catalog. Unfocused survey walks the source and returns the complete current set; the engine merges. The adapter does not read `leads.md` to decide it is a re-survey — the WIT focus parameter tells it.
   - **No change-home preopen.** The source guest's `"."` is not the change root. The agent lend is the source CID view (RFC-87 D2 empty-writable-scope prepare) or nothing for inline `value`. `plan.yaml`, `leads.md`, `decomposition.yaml`, `slices/`, events, and imports stay off the source seam.
   - **Slice artifacts are not an extract input.** Extract runs before synthesis; the engine persists `evidence/<source>.yaml`. Granting `slices/<slice>/` would lend an empty tree or previous-refine synthesis output extract must not consume, and would couple the extract WIT to slice identity it does not have today.
   - **Escape hatch, not built.** If a later adapter cannot fit parent `focus` on the record, a prepared catalog-row view (this source's parent chain from the retained revision) may be added then — not the slice tree and not the change home.

   Steps 11 and 12 are unblocked and amended accordingly. Step 20's adapter-contract sweep records the sandbox.
8. **Handoff DTO ownership. — CLOSED (2026-08-13).** Confirm `crates/project/src/definition/` as the shared home (RFC-104 will add the write side there), versus a new `definition` crate. **Resolution: the recommendation is adopted — module in `project`; RFC-88 does not add a workspace crate.** RFC-104 may promote later if its remaining DTOs need a lifecycle-free leaf; that is not this plan's call.
   - **New crates are the exception.** [`docs/standards/architecture.md`](../docs/standards/architecture.md) keeps the leaf → root graph (`error` → `artifacts`/`diagnostics` → `project` → `slice` → `change`) and treats a new workspace crate as exceptional. The handoff surface does not meet that bar: it is one closed DTO, a canonical digester, a definition-home read, and a writer-less `EventKind` variant.
   - **The types are not a lifecycle-free leaf.** `artifacts` exists as a crate so parsers cannot reach plan/slice transitions. The handoff kernel *does* sit on engine types: it extends the closed `EventKind` taxonomy, reuses `plan::projection`'s canonicalization, and resolves a definition-home layout. Extracting it would either depend on `project` (then it is not a leaf) or duplicate those seams.
   - **One module, two writers over time.** Closed Open Question 1 already made this coordinated RFC-104 code from step 5, not an RFC-88-private approximation. RFC-88 lands the DTO, digester, and read surface; RFC-104 adds the write side (`emery system *` handlers, remaining definition DTOs) to the same module. A crate split now would package a shared module both efforts already co-own, before the write side exists to justify the split.
   - **Promotion stays available, unused here.** If RFC-104's remaining DTOs (scope, coverage, system model, migration plan) grow a second domain that `project` should not own, RFC-104 promotes the module then. RFC-88 does not pre-create the crate.

   Step 5 is unblocked and amended accordingly.
9. **`intent` as an inline-value source. — CLOSED (2026-08-13).** Current intent bindings carry `value` in `plan.yaml.sources`. Under D2/D6 `intent` is the reserved explicit key with inline value protected by the plan digest. **Resolution: the mapping is 1:1 — `intent` is always the reserved key with inline `value`; there is no locator form; the degenerate definition's handoff carries intent as an evidence scope.**
   - **One reserved key, one row.** Today's `plan.yaml.sources.intent` with `value:` is the same row tomorrow. The key is always `intent`; there is never a second intent binding and digest suffixes do not apply to it. D6's "inline value uses its adapter; `intent` is reserved" is that lock, not a naming hint.
   - **No locator form.** D2's `locator` xor `value` is the shared source-row shape, but the `intent` adapter takes only the `value` arm. A Git, path, or HTTPS locator on `intent` is a typed refusal. Fingerprinting never applies (`intent` is explicit-only). There is no CID: inline values stay under the plan digest (D2).
   - **Degenerate handoff carries the evidence scope.** RFC-104 D10's degenerate definition is "explicit intent, constraints, one target-architecture view, and one wave". Its handoff `evidence-scopes[]` includes the `intent` source with the inline `value` (no `source-cid`). Binding copies that scope 1:1 onto `discovery.yaml` / `plan.yaml.sources.intent`. In-place N=1 (`--from .emery/system/`) still gets intent; it does not come from a CLI flag.
   - **Handoff DTO closes `value` xor `source-cid`.** Location-backed evidence scopes carry `source-cid`; `intent` carries `value`. This is the shared RFC-104/RFC-88 DTO from closed Open Question 1 — RFC-104's write side authors the same xor.
   - **`--intent` sugar and `--source` bindings die with the old author path** in step 9. Intent arrives only through the reviewed handoff.

   Steps 5, 8, and 9 are unblocked and amended accordingly.
10. **Budget and policy numbers. — CLOSED (2026-08-13).** Initial values for D3 decomposition depth/node/judgment/repair budgets and D9's versioned read policy, and whether those values are digest-covered. **Resolution: compile conservative constants; do not persist them; do not digest-cover them.** The compile-constants half of the original recommendation is adopted (`MAX_REPAIRS` / RFC-90 D1 precedent); the persist-into-`discovery.yaml`-or-decomposition half is not.
   - **These are process bounds, not planning inputs.** Profiles earned digest coverage in closed Open Question 4 because changing a slice-split threshold changes which leaves exist. D3 depth/node/judgment/repair caps decide whether authoring *completes*; a successful `decomposition.yaml` already is the tree. D9 read limits decide whether binding *completes*; execution never repeats target binding (D9), so a later byte-cap bump must not stale an already-pinned change. D3 already keeps the path to a tree ephemeral ("Raw judgment requests, responses, and repair attempts remain ephemeral"). The RFC's worked `discovery.yaml` / `decomposition.yaml` examples carry no policy or budget fields.
   - **No artifact copy, no host override.** A versioned policy DTO in `discovery.yaml` would make every binary bump of a starting value invalidate in-flight epochs. A composition-time raise would be a hidden scope expansion (platform.md: do not silently widen budgets). D9 exhaustion fails the wave for upstream narrowing in RFC-104; it is not a per-project dial. Version the Rust type if a field is added later — that is D9's "versioned policy." Acceptance criterion 10's "recorded limits" means the engine constants *are* the recorded policy, asserted by fail-closed tests, not a second copy in the change home.
   - **Two constant clusters.** D9: a closed Rust struct beside the resolution kernel (step 7) — concurrency 4 (independent bind reads only), bindings/imported trees 32, HTTPS redirects 5, HTTPS body 32 MiB, inspected-bytes / tree cap 512 MiB. D3: compiled caps beside the decomposition kernel — `MAX_DECOMPOSITION_DEPTH` 8, `MAX_DECOMPOSITION_NODES` 64, `MAX_DECOMPOSITION_JUDGMENTS` 128 (split/leaf dispatches including focused-survey requeue); invalid-split repair reuses `MAX_REPAIRS` (2) per leg, no second global repair budget. All documented in-code as declared starting values, not calibrated measurements. A real wave that hits a cap is evidence for a later constant bump or for splitting the wave upstream — not a reason to put the cap in `discovery.yaml`.

   Steps 7, 9, 14, 15, and 18 are unblocked and amended accordingly.
11. **`Entry.project` → `slices[].target`. — CLOSED (2026-08-13).** Confirm the rename and the removal of the registry-flavoured `project` vocabulary from the plan wire shape in one cut (step 9), including the `topology.rs` sole-project fallback. **Resolution: the rename is confirmed — one hard cut in step 9; slices always carry `target:`; omit-and-auto-bind is deleted.**
   - **Required singular `slices[].target`.** The field is a required string naming a key in `plan.yaml.targets`. RFC-88's worked example and RFC-95 publication membership both assume that shape. There is no `project` alias, no dual read, and no N=1 omission. In-place with one target still writes `target: <the-one-key>`.
   - **The sole-project fallback dies with the field.** Today's `Entry.project: Option<String>` is omitted on N=1 because an explicit `project` used to mean workspace routing (`build_entries` / `resolve_project_binding` in `crates/project/src/plan/propose/`). Registry slots are already gone; that reason is gone. `topology.rs` stops synthesizing a one-element `projects[]` from live `project.yaml` as the plan topology — `plan.yaml.targets` is the stored topology (D4); `project.yaml` remains the per-target identity/platforms check. Delete the omit-means-sole-project branch.
   - **Same cut as the rest of the Plan DTO.** Step 9 already hard-cuts the wire (`targets:`, digests, definition block). Keeping `Entry.project` through the decomposition projector (step 14) would be a hybrid wire. The old survey-driven author path and its propose-kernel project-binding retire in the same step; from step 14 the projector writes `slices[].target`.
   - **Sweep the coupled leftovers, not the product-tree noun.** `model.yaml.project` → `model.yaml.target` and `slice-model-target-drift` (the check currently special-cases omitted plan `project`); status/advance rendering that prints `project:`; propose-kernel `ResponseSlice.project` / `ProjectRef` comments. `project.yaml`, the `project` crate, and `project_root` are unchanged — **project** remains the product-tree noun. Pre-1.0 hard-cut (closed Open Question 3): existing `plan.yaml` files with `project:` do not parse; they re-init.

   Step 9 is unblocked and amended accordingly; step 14's projector writes the required field.
12. **Multi-member-wave seam checks. — CLOSED (2026-08-13).** RFC-88 must preserve RFC-96's ability to widen wave membership "without changing the merge WIT operation". **Resolution: confirmed — no step narrows the member model to a scalar.** The landed RFC-86 shape is already the RFC-96-preserving one; this plan keeps it.
   - **`Wave.members` stays `Vec<Member>`.** `Wave::one_member` is the serial constructor; `enforce_one_member` (`target-wave-member-count`) is the write/open gate. No step replaces the vec with `member: Member` or `slice: SliceName`. RFC-96 retires that gate for the concurrent executor only and keeps the same manifest schema.
   - **Merge iterates members.** Step 3's workspace merge resolves the named slice's frozen wave and refuses until every member result is present (D7). Preflight, delta-spec fold, and postflight walk `wave.members` in stable order; N=1 is a one-element loop, not `members[0]` as a type-level singleton. The merge WIT stays per-slice (`merge(id, slice, phase)`); the engine owns the loop.
   - **The commit fact carries the frozen member set.** D7 already requires `target.merge.wave-committed` to name the frozen member set plus base/result CIDs and to project every named member `merged`. Step 3 replaces the sole-member `slice_name` stand-in on that fact so RFC-96 can widen membership without changing the fact shape. `target.merge.wave-postflight-failed` names every failed member (D7 aggregate). `target.wave.opened` may keep `slice_name` as the serial-executor's named leaf (always `members[0].slice` under the one-member gate); the manifest remains membership authority.
   - **Step 18 stays a one-member executor, not a scalar type.** The serial scheduler still opens waves via `Wave::one_member` + `enforce_one_member`. Multi-member waves, antichain selection, and retiring the one-member gate remain RFC-96 (already in Deliberately out of scope).

   Steps 3 and 18 are amended accordingly.
13. **WIT release choreography. — CLOSED (2026-08-13).** Step 11 breaks the WIT package; until step 12 lands, the adapters repo does not build against the sibling engine. **Resolution: no intermediate PRs or tag-pinned engine releases — the path-patch on the branch is confirmed, and the two repository PRs land together.**
   - **All steps complete before any PR.** Sessions commit to `rfc-88-impl` on both repos; nothing is published until the plan is done. Intermediate commits are branch-local. There is no concern that a published engine would leave adapters unbuildable, or that adapters would pin a tag that does not yet exist.
   - **PRs raise in concert and land together.** One PR on `augentic/emery` and one on `augentic/emery-adapters` close the whole plan. They are treated as simultaneous: the WIT break in step 11 and the adapter update in step 12 never appear as a published mismatch.
   - **Path-patch on the branch, pin at PR time.** Adapter-repo steps keep the committed `[patch."https://github.com/augentic/emery.git"]` block uncommented against the sibling `../emery` checkout. No tag-pinned intermediate engine release is cut. The operator re-points the git-dependency pin and re-comments the patch when raising the concert PRs; steps do not do this themselves.
   - **11 → 12 remains session order, not a release gate.** Between those sessions the adapters repo is expected not to build against the sibling engine. That window is intra-branch only. Step 11 still notes the WIT break so the next session runs step 12; it is not a choreography signal for a release.

   Steps 11, 12, and 19 are unblocked and amended accordingly. Cross-repo choreography is updated to match.

## Deliberately out of scope (guard rails for every session)

- No RFC-104 write surface (`emery system *`), no `system.wave.reviewed` writer, no coverage dispositions in `discovery.yaml`.
- No RFC-95 publication: no push/branch/PR/seal, no forge provider work beyond what D9's bounded reads need.
- No RFC-96: no concurrent scheduler, task decomposition, multi-member waves, or envelope-escalation *producer* (the DTO ships in step 17).
- No streaming/partial publication (complete-tree policy only), no `--create`/provisioning surface, no `adapter.component-digest`, no second source-digest scheme, no undo/preview verbs.
- Removed concepts stay removed (acceptance criterion 12) — do not reintroduce registry vocabulary, `snapshot` as a wire field name for tree identity, or ambient product roots.
