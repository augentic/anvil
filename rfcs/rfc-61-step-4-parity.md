# RFC-61 Step 4 — verb parity and skill-orchestration coverage

> Status: Step-boundary audit (Step 4, Milestone G) for [RFC-61](rfc-61-omnia-migration.md) · Audited surfaces: the shared clap grammar (`engine/crates/dispatch/src/cli.rs`), the guest route table (`engine/crates/dispatch/src/guest.rs`), the guest orchestrators (`engine/crates/workflow/src/orchestrate/`), and all eight `plugins/spec/skills/*/SKILL.md` bodies on the `specify-wasm` branch.

This document is the Step 4 → Step 5 gate artifact: it classifies every CLI verb by where it runs after the workflow-guest port, and maps every orchestration line in the eight phase skills to its guest home, per the RFC's invoke-and-relay test ("a skill that cannot be reduced to 'invoke and relay' indicates workflow logic that belongs in the guest and must be ported before the skill is thinned"). Standing decisions cited here live in the engine workspace's [`DECISIONS.md`](../DECISIONS.md).

## Dispositions

Every verb lands in exactly one of three buckets:

- **guest-owned** — runs in the workflow guest today, either as an in-process handler (the shared `specify-dispatch` handler the native binary also uses) or as a **collapsed orchestration** (the guest shim drives a `specify_workflow::orchestrate` entry point against the WIT seam, folding the native two-phase agent handoff into one call).
- **native residue until Step 5** — the guest refuses the verb on the standard argument-error surface (wire code `argument`, exit 2); the native binary keeps the handler. Residue verbs need subprocesses, the network, Wasmtime, git, or host-machine state the guest deliberately does not model (decision D4).
- **deleted at Step 5** — the verb (or flag) exists only to serve the native agent-handoff envelope machinery; its behaviour is already collapsed into a guest orchestrator or in-guest adapter library code, and the cutover deletes it rather than porting it.

## Verb parity matrix

Verified verb-by-verb against `guest::route()`; the routing column states what the guest does with the parsed invocation today.

| Verb | Guest routing today | Disposition | Notes |
| --- | --- | --- | --- |
| `init` | refused | native residue | Bootstrap verb: adapter fetch (network), scaffold, `AGENTS.md` generation. |
| `source resolve` | in-process handler | guest-owned | Project-context-free manifest resolve. |
| `source preview` | in-process handler | guest-owned (review at Step 5) | Workflow-free sandbox prep for the standalone agent preview loop — handoff machinery by purpose; its Step 5 disposition (retire vs keep as a debugging surface) is an open cutover call. |
| `source survey <source>` | collapsed orchestration (`orchestrate::survey`) | guest-owned | `--phase` accepted-but-ignored in-guest; the flag is deleted at Step 5. |
| `source extract <source> <lead> --slice` | collapsed orchestration (`orchestrate::extract`) | guest-owned | Same `--phase` note. |
| `target resolve` | in-process handler | guest-owned | |
| `adapter build` / `adapter publish` | refused | native residue | RFC-48 packaging + OCI push (network). |
| `rules export` / `rules sync` | refused | native residue | Codex overlay resolution for consumer projects. |
| `extension run` / `fetch` / `gc` / `schema` | refused | deleted at Step 5 | The vectis and contracts tools are already in-guest adapter library code (Step 3); the engine's registry-hosted Wasmtime runner and its cache verbs go with `extension run` (acceptance criterion 4). |
| `lint project` / `lint framework` | refused | native residue | Stays native Rust permanently per RFC §Step 5 — the lint residue is host-side code, never a guest. |
| `slice create` | in-process handler | guest-owned | |
| `slice validate` | in-process handler | guest-owned | |
| `slice provenance` | in-process handler | guest-owned | |
| `slice model show` | in-process handler | guest-owned | |
| `slice synthesize --dry-run` / `--from` | in-process handler | deleted at Step 5 | The envelope pair is the native agent handoff; the guest home is the collapsed judgment leg inside `orchestrate::refine` / `orchestrate::synthesize`, which reuses the same persist tail (`slice::synthesis::persist`). |
| `slice build <name>` | collapsed orchestration (`orchestrate::build`) | guest-owned | The native two-phase handler (`--phase prepare|finalize`, manifest `prepare.argv` hook dispatch) stays binary-side as residue; the `--phase` flag is deleted at Step 5. |
| `slice merge run` | collapsed orchestration (`orchestrate::merge`) | guest-owned | Deterministic-only per D2: no target merge brief dispatch, git commit leg skipped with `slice.merge.commit-skipped`. |
| `slice merge preview` / `conflict-check` | in-process handler | guest-owned | |
| `slice task progress` / `mark` | in-process handler | guest-owned | |
| `slice transition` | in-process handler | guest-owned | |
| `slice touched-specs` | in-process handler | guest-owned | |
| `slice overlap` | in-process handler | guest-owned | |
| `slice drop` | in-process handler | guest-owned | |
| `catalog infer --phase report|bind` | refused | native residue | Dispatches the vectis WASI tool through the extension runner; Step 5 must re-home the dispatch onto the in-guest vectis library before `extension run` is deleted. |
| `archive prune` | refused | native residue | Pure-filesystem retention GC; kept native for now (no skill orchestration line needs it in-loop). |
| `plan create` (incl. `--intent`) | in-process handler | guest-owned | `--intent` desugars to the `intent=intent:value:…` binding (Milestone E). |
| `plan validate` | in-process handler | guest-owned | |
| `plan next` | in-process handler | guest-owned | The `require_held` flock gate degrades permissive off Unix (Milestone A `plan_lock` imp), so the standalone guest verb is not bricked; the execute loop claims through the lock-free core instead. |
| `plan status` | in-process handler | guest-owned | |
| `plan add` / `amend` / `remove` | in-process handler | guest-owned | |
| `plan propose --dry-run` / `--from` | in-process handler | deleted at Step 5 | The envelope pair is the native agent handoff; the guest home for the judgment is `judgment::propose::reconcile` (Milestone B) over the same kernel (`Plan::propose_from`) — **not yet wired to a dispatch verb** (see gaps). |
| `plan transition` (Gate 1 / per-entry / `--undo`) | in-process handler | guest-owned | |
| `plan execute` | collapsed orchestration (`orchestrate::execute`) | guest-owned (guest-only) | Native refuses the verb; the D1 marker replaces the flock in-guest. |
| `plan archive` | in-process handler | guest-owned | |
| `plan lock -- <cmd>` | refused | native residue | Subprocess wrapper fencing separate OS processes; the guest collapses breakouts in-process. Its post-cutover fate is tied to the native skill loop it fences — a Step 5 decision. |
| `journal emit` / `show` | in-process handler | guest-owned | Guest anchors `Ctx` at the `"."` preopen. |
| `registry validate` / `add` / `remove` | refused | native residue | Workspace-registry surface (D3: workspace plans stay native until Step 5). |
| `workspace sync` / `prepare` / `push` | refused | native residue | Git and network; D3. |
| `completions` | refused | native residue | Host-shell integration. |
| `contract dump` | refused | native residue | Feeds the native lint cross-check (`cli-contract`). |
| `upgrade` | refused | native residue | Self-update (network). |
| `plugins doctor` / `refresh` | refused | native residue | Cursor plugin-cache maintenance on the host machine. |

Global flags: `--format` is shared and honoured on both sides. `--plan-dir` is **native-only on guest-routed verbs**: the guest anchors plan artifacts at the `"."` preopen (the working directory), so a plan-root override has no in-guest home — the S3 triage layer refuses `--plan-dir` (or `SPECIFY_PLAN_DIR`) with any value other than the working directory on the standard argument-error surface (wire code `argument`, exit 2) rather than silently ignoring it. `--phase` on `source survey` / `source extract` / `slice build` is the other documented argv divergence — parsed everywhere, ignored in-guest, deleted at Step 5.

## Skill-orchestration coverage map

Every CLI invocation or control-flow step each skill body performs, mapped to its guest home. "Relay" marks lines that survive skill thinning by construction (eliciting arguments, printing CLI output verbatim) — the ultrathin-wrapper residue RFC line 95 permits.

### `/spec:init`

| Orchestration line | Guest home |
| --- | --- |
| `specify --version` probe, `cargo install` bootstrap | native residue (D4) — bootstrap surface stays native by design |
| `specify upgrade --dry-run` / `--yes` | native residue (`upgrade`) |
| `specify plugins doctor` / `refresh --yes` | native residue (`plugins`) |
| topology choice, metadata/platform elicitation | relay (argument elicitation) |
| `specify init <adapter>` / `--workspace` / `--upgrade` | native residue (`init`) |
| baseline-extraction offer → `specify slice create` | guest verb `slice create` (the offer itself is elicitation) |

Verdict: reducible to invoke-and-relay over native-residue verbs; no guest port required before Step 5.

### `/spec:plan`

| Orchestration line | Guest home |
| --- | --- |
| pre-flight `project.yaml` read | every guest verb's `Ctx` load |
| `specify plan create --source …` / intent elicitation | guest verb `plan create` (+ `--intent` sugar) |
| `specify workspace sync` (workspace plans) | native residue (D3 — workspace plans refused in-guest) |
| per-source `specify source survey` two-phase handoff | collapsed `orchestrate::survey`; the all-bound-sources fan-out is `orchestrate::survey_all` (core fn, **no dispatch verb yet** — see gaps) |
| `discovery.md` lead-inventory merge | `Discovery::merge_survey` inside the survey orchestrator |
| `discovery.md` `## Summary` / `## Source inventory` prose | **no guest home** — part of the plan-authoring judgment leg (see gaps) |
| propose dry-run → agent grouping → `propose --from` | judgment home `judgment::propose::reconcile` + `Plan::propose_from` kernel (Milestone B) — **not wired to a verb** (see gaps); the envelope pair is deleted at Step 5 |
| `change.md` Gate 1 review prose (tentative merges, divergences) | **no guest home** — the propose answer schema carries `slices[]` only (see gaps) |
| `specify plan amend --divergence likely` (post-propose stamps) | guest verb `plan amend` |
| `specify plan validate` | guest verb `plan validate` |
| exit-at-`pending` closing hint | relay |

Verdict: the slice-loop half is covered; the plan-authoring half (survey fan-out verb, propose wiring, Gate 1 prose) is the one open porting item — documented below as the Step 5 precondition.

### `/spec:refine`

| Orchestration line | Guest home |
| --- | --- |
| `specify plan lock` wrapper / `SPECIFY_PLAN_LOCK_HELD` | native residue; in-guest the execute loop holds the D1 marker and claims through the lock-free core |
| slice resolution via `specify plan next`, binding cross-resolution | `plan next` guest verb; inside the loop, `plan_next_body` + `orchestrate::refine`'s `load_entry` |
| workspace routing (sync, chdir, `SPECIFY_PLAN_DIR`) | D3 refusal — workspace plans stay native until Step 5 |
| `specify slice create --if-exists continue` | inside `orchestrate::refine` (also standalone guest verb) |
| per-binding `specify source extract` two-phase fan-out, serial order | inside `orchestrate::refine` (serial, binding order) via `orchestrate::extract` |
| `slice synthesize --dry-run` envelope + response authoring + `--from` | collapsed judgment leg `orchestrate::synthesize` + shared persist tail inside `orchestrate::refine`; envelope pair deleted at Step 5 |
| Decision Record authoring (optional prose) | stays judgment-side — carried by the synthesis prompt corpus at Step 5 (opt-in artifact; validated by the same `decision-record-*` gates in-guest) |
| `specify slice validate` sweep | inside `orchestrate::refine` (`pre_adapter_gates` + adapter rules + tag journal); also standalone guest verb |
| `specify slice transition refined` | inside `orchestrate::refine`; also standalone guest verb |
| closing hints (refined / extract-failure / validation-failure) | typed errors + `RefineOutcome`; the shim renders the native envelope shapes |

Verdict: fully covered inside the loop. There is **no standalone `refine` guest verb** — a breakout `/spec:refine` has no single in-guest invocation (see gaps).

### `/spec:execute`

| Orchestration line | Guest home |
| --- | --- |
| `specify plan status` projection + refusal on `plan-not-approved` | `plan_status_body` inside `orchestrate::execute` (Gate 1 enforced by the first projection); also standalone guest verb |
| plan lock acquisition / `plan-lock-busy` exit | D1 guest marker (`.specify/guest.lock`, `guest-marker-held`) |
| loop: `plan next` claim → phase dispatch → repeat | `orchestrate::execute` (claim via `plan_next_body`, dispatch to `refine` / `build` / `merge`) |
| workspace routing per entry | D3 refusal (`plan-execute-workspace-unsupported`) |
| stop rendering (verbatim CLI block + hint) | typed `ExecuteOutcome::Stopped` → `plan-execute-stopped` envelope carrying the closed `StopReason` + hint |
| re-entry semantics | re-running `plan execute` re-projects from on-disk state; phase failures leave the entry `in-progress` |

Verdict: fully covered — this is the Milestone E flagship; the skill retires outright at Step 5 per RFC §Step 4.

### `/spec:build`

| Orchestration line | Guest home |
| --- | --- |
| plan lock / `plan next` resolution / `[slice-name]` match guard | loop-owned in-guest; the standalone guest `slice build <name>` takes the name directly and does **not** enforce the active-entry match (minor divergence, noted below) |
| workspace routing | D3 refusal |
| lifecycle refusal (only `refined` proceeds) | the `Refined → Built` gate inside `orchestrate::build`'s finalize tail |
| `slice build --phase prepare` (request assembly, schema gate, handoff) | collapsed into `orchestrate::build` (assembly + schema gate, no envelope stop) |
| run the target build brief (agent codegen + validation) | `TargetSeam::build` — the adapter guest's judgment leg (brief compiled in, `create` + MCP shelf) |
| `slice build --phase finalize` (report gates, `slice.build.*`, `built`) | the finalize tail inside `orchestrate::build` (report schema gate, `enforce_report_*`, events, transition) |
| stop-hint rendering | typed errors through the shim's native failure envelope |

Verdict: fully covered by `orchestrate::build`; the native two-phase verb plus its `prepare.argv` hook dispatch stays binary-side residue until Step 5.

### `/spec:merge`

| Orchestration line | Guest home |
| --- | --- |
| plan lock / `plan next` resolution / match guard | as `/spec:build` above |
| workspace routing | D3 refusal |
| lifecycle refusal (only `built` proceeds) | the `lifecycle` gate inside `slice_merge::commit` |
| target merge brief pre-merge gate (cargo/clippy/test, cap-matrix, contract tool) | **dropped in-guest per D2** — deterministic-only merge; verification moves agent-side into target prompts at Step 5 (watch-item below) |
| `specify slice merge run` (delta merge, DEC promotion, archive, ledger, `done`) | `orchestrate::merge` — including `slice.merge.commit-skipped` for the git leg and the ledger entry with no `merge-sha` |
| `merge preview` / `conflict-check` probes | guest verbs (in-process handlers) |
| post-merge hook (`specify extension run contract …`) | the contracts validator is in-guest adapter library code (Step 3); its post-merge dispatch has no guest home under D2 (watch-item below) |
| AskQuestion confirmation (interactive breakout) | relay (elicitation); in-loop the guest is its own confirmation seam, matching the skill's non-interactive rule |
| replay-summary rendering from `metadata.yaml` `replay:` | native residue (build-time replay target hook); rendering is relay once the block exists |

Verdict: the deterministic core is covered; the two brief-driven verification legs are deliberate D2 drops the Step 5 review must confirm as agent-side moves.

### `/spec:finalize`

| Orchestration line | Guest home |
| --- | --- |
| pre-flight name/plan validation | relay + `Ctx` load |
| drained check `specify plan status` | guest verb `plan status` |
| `specify workspace push` | native residue (git/network, D4) |
| `specify plan archive` | guest verb `plan archive` |
| closing message | relay |

Verdict: reducible to invoke-and-relay today (two guest verbs, one residue verb).

### `/spec:drop`

| Orchestration line | Guest home |
| --- | --- |
| slice enumeration + AskQuestion selection/confirmation | relay (elicitation) |
| lifecycle pre-read + warnings | relay (the CLI re-enforces terminal-status refusal) |
| `specify slice drop --reason` | guest verb `slice drop` |

Verdict: reducible to invoke-and-relay today.

## Gaps found by this audit

1. **The plan-authoring collapse is unwired (documented, not built — Step 5 precondition).** `/spec:plan`'s propose sub-step has its judgment leg (`judgment::propose::reconcile`, Milestone B, natively tested) and its kernel (`Plan::propose_from`, shared with the envelope verb), but no dispatch verb drives them, `orchestrate::survey_all` has no verb for the all-bound-sources fan-out, and the Gate 1 prose artifacts (`change.md`, `discovery.md`'s `## Summary` / `## Source inventory` sections) are outside the propose answer schema. Wiring this is a plan-authoring orchestrator (scaffold → survey fan-out → propose judgment → validate → `pending` exit) plus an answer-schema widening for the prose — real Rust surface, not a milestone-G patch. Until it lands, `/spec:plan` cannot be thinned; the RFC's Step 4 invoke-and-relay test fails for exactly this skill and passes for the other seven.
2. **No standalone `refine` breakout verb.** `orchestrate::refine` runs only inside `plan execute`. A breakout `/spec:refine <slice>` has no single guest invocation — the Step 5 cutover must either add a `slice refine`-shaped verb or retire the breakout in favour of the loop (the loop's re-entry semantics already resume a parked slice at its next phase). Decision deferred to the same plan-authoring wiring pass.
3. **Standalone `slice build` / `slice merge run` skip the active-entry match guard.** Natively the skills refuse `[slice-name]` ≠ the active `in-progress` entry; the guest verbs act on the named slice directly. The lifecycle gates still hold (only `refined` builds, only `built` merges, `done` stamps only through merge), so this is a documented divergence, not a correctness hole — the guard's home at Step 5 is the loop plus the D1 marker.
4. **D2 verification drops need their Step 5 confirmation.** The pre-merge target gate (cargo/clippy/test, cap-matrix, contracts tool) and the post-merge contracts validator do not run under the guest merge. The RFC's posture (verification moves agent-side into target prompts; `verify` grants stay stubbed pending the deferred verify-profiles RFC) covers this, but the cutover review should confirm each first-party target's prompts actually carry the checks before the native path is deleted.
5. **`catalog infer` re-homing.** The verb dispatches the vectis tool through `extension run` machinery that Step 5 deletes; its dispatch must move onto the in-guest vectis library (or the verb moves in-guest) in the same cut.

No misrouted verbs and no missing refusals were found: every arm of `guest::route()` matches its disposition above, the native binary refuses `plan execute` (mirror-image of the guest's refusals), and the off-Unix `plan_lock` imp degrades permissive exactly where the guest needs it.

## Step 5 Milestone S4 outcome (residue-matrix update)

The S4 cut (see the engine workspace's [`DECISIONS.md` §"Old-stack deletion"](../DECISIONS.md#old-stack-deletion-milestone-s4)) resolved every open call above. The matrix's "guest routing today" column now describes the shipped binary:

- **Triage set widened.** `source survey`, `source extract`, `slice build`, and `slice merge run` route to the composed-deployment guest leg alongside `plan execute`, `plan author`, and `slice refine`; their native handlers and the whole envelope machinery (`--phase`, `slice synthesize --dry-run/--from`, `plan propose --dry-run/--from`, the two-phase `slice build` handler with `prepare.argv` dispatch) are deleted. Guest-owned verbs marked "in-process handler" above stay native-dispatched shared handlers.
- **Open calls closed.** `source preview` — retired (D-preview; last consumer of the shared prep seam). `plan lock -- <cmd>` — retired with the `require_held` gates (D-planlock; the D1 guest marker is the only fence, D3 refusal stays). `catalog infer` — retired outright: report judgment and bind bookkeeping are in-guest vectis-core code driven by the build's `${SLICE_DIR}/build/component-bindings.yaml`, with no engine-side catalog capability (D-catalog-infer).
- **`extension run/fetch/gc/schema`** — deleted as scheduled; `specify-registry` shrank to `pack`/`oci`/`store` plus the `lint project` WASI path.
- **Gap resolutions.** Gap 1 closed at S1 (`plan author`); gap 2 closed at S1 (`slice refine`); gap 3 stands as documented divergence; gap 4's confirmation happened in the adapters-repo cutover review; gap 5 resolved by retirement rather than re-homing.
- **Workspace-mode regression (D-workspace).** With `--plan-dir` native-only and the phase verbs guest-routed, workspace-routed plans cannot drive the breakout phase verbs from slots; workspace plans stay on the native pure verbs until slot routing has an in-guest counterpart.
