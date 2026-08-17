# emery plan

Scaffold, populate, refine, validate, execute, and archive change plans. The `plan` verb is the top-level home of every `plan.yaml` operation; each verb on this page is invoked as `emery plan <verb>`. `author → refine → execute → archive` is the whole workflow spine; the rest of the group is curation (`add` / `amend` / `remove` / `drop`) and read-only projection (`status` / `gaps` / `validate`).

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`author`](#emery-plan-author) | Bind a reviewed handoff from `emery system review` (`--from` / `--wave`), import its surface leads, decompose it, and publish `decomposition.yaml` + `plan.yaml` together. Persists the tree incrementally and parks failed cuts instead of aborting; re-entry resumes open and parked domains, a reconciled plan is a read-only no-op, and `--force` is the wholesale replace. Invoked by `/emery:plan`. |
| [`correct`](#emery-plan-correct) | Record a durable operator correction for a decomposition domain: fact-only on a parked author (honored at `plan author` re-entry), fact + inert boundary proposal on an authored plan (`plan amend --proposal` applies it). Invoked by `/emery:correct`. |
| [`refine`](#emery-plan-refine) | Guest-routed serial refinement drain: per in-scope leaf in dependency order, extract every bound source, synthesize + validate the slice artifacts, atomically write `refinement.yaml`. Fresh manifests are skipped; no code work. Invoked by `/emery:refine`. Optional repeated `--slice` selectors. |
| [`execute`](#emery-plan-execute) | Guest-routed driver loop: requires a fresh refinement manifest per in-scope leaf, at start appends `plan.execute.started` (authorization epoch covering the exact refinement digests), then claims → builds → merges per entry under gap gates until `drained` or a stop. Holds the `.emery/change/guest.lock` marker. |
| [`add`](#emery-plan-add) | Append a new entry to the plan (projects `pending` until claimed). |
| [`amend`](#emery-plan-amend) | Edit topology fields (`description`, `depends-on`, `sources`), divergence stamps, authority overrides, the `allow-composition-replace` merge authorization, or apply a retained amendment (`--proposal`). |
| [`remove`](#emery-plan-remove) | Drop an entry while the plan is still replaceable (every entry still projects `pending`). |
| [`drop`](#emery-plan-drop) | Abandon one entry's already-refined slice without merging — stamps `dropped` and archives the slice tree. |
| [decomposition](#decomposition-inside-emery-plan-author) | The decompose + propose legs inside `emery plan author`: survey the bound catalog, write `decomposition.yaml`, and project `slices[]`. |
| [`validate`](#emery-plan-validate) | Structural and referential integrity check (cycles, unknown deps) plus health diagnostics (`cycle-in-depends-on`, `orphan-source`). First triage step when `emery plan execute` reports `stuck`. |
| [`status`](#emery-plan-status) | Read-only projection into a deterministic `next-action` (`refine|build|merge <slice>` / `stop <reason>` / `drained`) plus Ready / Authorized. |
| [`gaps`](#emery-plan-gaps) | Read-only typed gap inventory (`unknown` / `conflict` / `divergence`) with shared-lead grouping annotations. |
| [`archive`](#emery-plan-archive) | Move a completed `plan.yaml` and `.emery/change/plans/<name>/` to `.emery/change/archive/plans/`. Usually invoked by `/emery:finalize` after the operator confirms publication (commits, PRs, review) is complete — publication itself stays operator-owned, outside Emery. |

## Subcommands

### emery plan author

Bind a reviewed handoff, decompose the bound catalog, and publish `decomposition.yaml` + `plan.yaml` together. Invoked by `/emery:plan`.

```bash
emery plan author <name> --from <definition-home> --wave <id> [--force] [--change-dir <dir>]
```

| Argument | Description |
|----------|-------------|
| `name` | Kebab-case change name. |
| `--from` | Reviewed definition home. Relative values join the product root in-place (`.emery/system/` for a colocated degenerate definition) or the change home when detached. |
| `--wave` | Wave id inside the definition named by `--from`. |
| `--force` | Wholesale replace: rebind the same reviewed handoff and redo the decomposition. A changed wave needs a new handoff and review fact (`plan-author-handoff-changed`). `--force` is never the recover path — without it, re-entry on a bound-not-authored plan resumes the open and parked domains, and re-entry on a reconciled plan is a read-only no-op; only a *different plan name* over an existing plan refuses with `plan-already-exists`. `/emery:plan` confirms before passing it. |
| `--change-dir <dir>` | Optional detached change root. Omitted, the nearest ancestor with `.emery/project.yaml` is in-place (`<product>/.emery/change/`); else cwd is the change home. No marker, no walk. |

Exit codes: `0` success (including the read-only no-op re-entry); `2` for `plan-already-exists` (name mismatch), missing/ambiguous handoff, binding validation failures, a parked stop (`plan-author-stopped` — one or more domains parked after failed cuts; the stop card names them), and the fatal decomposition stops (`plan-author-budget-exhausted`, `plan-author-definition-revision`); `1` for ingest and I/O failures.

JSON output: the [`emery plan author` envelope](../cli-output-shapes.md#emery-plan-author) — bound targets and sources, the discovery / leads / decomposition digests, and the projected `slices[]`.

Behavior notes:

- **Order of operations.** Resolve the current reviewed handoff (`system::review::current_handoff`, verify-on-read), copy byte-identical envelopes under `imports/`, re-resolve each coverage locator and pin the delivery CID (a handoff `observed-cid` is imported provenance and does not authorize the pin), fill a declared adapter name and keep a handoff pin, write `discovery.yaml`, import the wave's surface leads (focused child survey only when an imported lead is still coarser than a buildable boundary), decompose the catalog, and publish `decomposition.yaml` + `plan.yaml` together.
- **Dispositions, not aborts.** The tree persists after every applied cut. After the repair budget, a failed cut (non-reducing, uncovered/dropped lead, unordered overlap) is a disposition: the engine closes the domain as a leaf through the same profile gate a model-emitted leaf gets (journaling `domain.partition.closed` with the findings and a `change.md` caveat), else parks that domain (`plan.author.parked`) and keeps draining independent domains. A park projects already-closed leaves into `plan.entries` without a `plan.reconcile.completed` fact and exits 2 with `plan-author-stopped`; `plan status` shows `stop partition-parked`, and the resume pair is `emery plan correct` (optional) + `emery plan author`. Topology verbs (`add` / `amend` / `remove` / `gaps`) refuse `plan-author-incomplete` until authoring completes.
- **Intent** arrives only through the handoff (reserved key `intent`, value-only, no locator, no CID). There is no `--intent` or `--source` authoring flag.
- **Exact pins.** Every bound source and target records `emery:<name>@<semver>`. A handoff pin is frozen; a declared name is filled at bind time. Bare names are refused on the persisted plan topology.

### emery plan correct

Record one durable operator correction for a decomposition domain. Invoked by `/emery:correct`.

```bash
emery plan correct [--domain <id>] [--constraint <close-as-leaf|split>] [--child <id>]... --intent "…" [--change-dir <dir>]
```

| Flag | Description |
|------|-------------|
| `--domain <id>` | Decomposition node id (or a leaf slice name on an authored plan, resolving to its nearest domain). Omitted, the correction targets the sole parked domain — refusing with `plan-correct-domain-required` / `plan-correct-domain-ambiguous` when none or several are parked. |
| `--constraint` | Closed structural constraint the deterministic tail enforces: `close-as-leaf` refuses a split answer; `split` refuses a leaf answer and requires any `--child` ids to appear. Free-text intent alone is model guidance. |
| `--child <id>` | Repeatable. Child domain ids a `split` constraint requires. Needs `--constraint split`. |
| `--intent "…"` | Required. Operator intent, verbatim — the partition judgment treats it as a hard constraint. |

Phase-split semantics:

- **Bound-not-authored (parked author):** record the `plan.correction.recorded` fact only — no model call, no proposal, no artifact write. Re-running `emery plan author` carries every active correction into the parked domain's partition request. Always works regardless of how many domains are open, and spends no judgment budget.
- **Authored plan:** the escalate-shaped path — re-decompose the named domain with the correction in the request into an inert boundary proposal at `planning/proposals/<digest>.yaml`, journal the fact with the digest, and leave live planning artifacts unchanged until `emery plan amend --proposal <digest>` applies it. A re-cut that would uncover a lead or fail to reduce refuses with `plan-correction-non-reducing` without mutating the live tree.

Corrections are durable, append-only facts; activity is projected (a bound-path fact stays active until the next `plan.reconcile.completed`; an authored-path fact rides its proposal through `plan amend`).

Exit codes: `0` success (the body carries `status: recorded | proposed`, the domain, and the proposal digest on the authored path); `2` for domain-resolution refusals (`plan-correct-domain-required`, `plan-correct-domain-ambiguous`, `decomposition-node-unknown`), incoherent constraints, and `plan-correction-non-reducing`.

### emery plan refine

Guest-routed serial refinement drain — the specification stage between `plan author` and `plan execute` (RFC-91). Invoked by `/emery:refine`.

```bash
emery plan refine [--slice <slice>]... [--change-dir <dir>]
```

| Flag | Description |
|------|-------------|
| `--slice <slice>` | Repeatable. Target specific in-scope leaves; selectors also include the stale-or-missing predecessor closure the selected work needs. Omitted, every in-scope leaf is targeted. |

The drain walks in-scope plan entries in topological `depends-on` order and, for every targeted leaf whose refinement manifest is missing or stale, extracts each bound source, synthesizes and validates the slice artifacts (`proposal.md`, `specs/<domain>/spec.md`, `design.md`, `tasks.md`, `model.yaml`), and atomically writes `.emery/change/slices/<slice>/refinement.yaml` — the canonical record of the refinement's exact inputs and complete output bundle, identified by its content digest. Dependent refinement requires every direct predecessor's manifest currently fresh; the fresh digests are recorded as the dependent's ordered `dependencies[]` pins. Fresh leaves are skipped (`fresh <slice> (skipped)`), so re-running resumes missing or stale work. The drain holds the create-exclusive `.emery/change/guest.lock` marker for the run's lifetime.

No target build operation, product workspace, target wave, `BuildRecord`, merge gate, or authorization epoch is created — `plan refine` writes planning artifacts only. Successful refinement may carry `[unknown]` / `[conflict]` / `[divergence]` review outputs; those are persisted, not failures, and the output points at `emery plan gaps`. Bundle artifacts are engine-owned between refine and execute: a direct edit is detected as staleness and re-refinement replaces it — durable corrections travel through inputs (source material, `emery plan amend`, authority overrides).

The drain stops on the first failed refinement (exit 2, `plan-refine-stopped`, with the canonical plan-status stop card on stdout); prior successful manifests stay, and re-running `emery plan refine` resumes at the parked slice.

Exit codes: `0` when every targeted leaf is fresh; `2` for a stop (`plan-refine-stopped`), a selector naming no in-scope entry, or a held marker (`guest-marker-held`).

JSON output: the [`emery plan refine` envelope](../cli-output-shapes.md#emery-plan-refine) — the `refined` / `skipped` slice lists and the `gaps` flag.

### emery plan validate

Check structural and referential integrity of the plan, plus the health diagnostics below.

```bash
emery plan validate
```

Base shape checks: duplicate entry names, dependency cycles, unknown `depends-on` / `sources` references, duplicate source keys within one entry (`duplicate-source-key` — a slice binds at most one lead per source), orphan authority-override sources (`slice-authority-override-orphan-source`), divergence stamp coherence (`slice-divergence-unrecorded` / `slice-divergence-orphan-values`), and orphan slice directories (`orphan-slice-dir`). There is no plan-wide single-active-entry rule — exclusivity is per-slice claims.

Health diagnostics layered on top — first triage step when `emery plan execute` reports `stuck`:

| Code | Severity | Meaning | Recovery |
|------|----------|---------|----------|
| `cycle-in-depends-on` | important | Dependency cycle in `depends-on`. `next_eligible` silently skips cycles at runtime; validate is the only place where the cycle structure surfaces. Structured evidence carries the cycle path, e.g. `["a", "b", "a"]`. | `emery plan amend <entry> --depends-on …` to break the cycle, then re-run validate. |
| `orphan-source` | suggestion | Top-level `sources:` key declared but no plan entry references it (the inverse of `unknown-source`). | Either reference the key from an entry's `sources:` list or remove the declaration. |

JSON output (`--format json`) is the neutral `DiagnosticReport` envelope (`{ version, summary, findings }`) shared with `emery slice validate` — the typed shape lives at [`crates/diagnostics/src/diagnostic.rs`](../../../crates/diagnostics/src/diagnostic.rs). Each finding carries `rule-id` (kebab-case, e.g. `duplicate-name` / `cycle-in-depends-on`), `severity` (`critical` / `important` / `suggestion` / `optional`), `impact` (the human-readable message), optional `slice` (the entry name), and `evidence`. The health diagnostics attach their machine-readable payload to `evidence` as `{ "kind": "structured", "data": … }`; base validate findings carry a plain `snippet` evidence.

Exit code: `0` when no blocking finding fires (suggestions are non-fatal); `2` when any blocking (`critical` / `important`) finding fires.

### emery plan execute

Drive the plan through build → merge per entry under the guest lock, consuming the exact refinement manifests `emery plan refine` wrote. At start appends `plan.execute.started` with typed `closed-plan` coverage — exact per-leaf refinement digests — there is no separate `plan approve` verb and no projected `approved` rung. Re-entry on an already-drained plan is a read-only no-op (no new epoch); on any other resume the fresh epoch replaces the previous one. Deferrals are durable facts, not epoch payload — nothing needs re-supplying on a resume.

```bash
emery plan execute [--change-dir <dir>]
```

Before each build the gap gate joins durable dispositions from the deferral fact union: deferred rows (`[unknown]` and `[conflict]` alike) leave build scope and proceed. Every remaining open row is dispositioned at build time by minting one `gap.deferred` fact per requirement (synthesized reason) and the build proceeds — nothing blocks. `[divergence]` rows are listed and allowed. Stale covered artifacts refuse with `plan-epoch-stale`. Execute never parks on gaps — review them ahead of the run with [`emery plan gaps`](#emery-plan-gaps), and after it through the fact log, the archive summary, and [`emery debt`](debt.md).

**Coverage requires fresh refinement.** The start-of-run coverage assembly requires a fresh refinement manifest for every in-scope leaf and covers its exact refinement digest — `{plan-digest, refinements: {<slice>: <digest>}}`. A missing or stale manifest fails typed (`plan-refinement-required`) **before** any epoch, workspace, or wave; execute never refines. The iteration loop after any input change is simply: fix inputs → `emery plan refine` → review gaps → `emery plan execute`.

The loop dispatches the ready set — cross-target builds overlap on the bounded pool, a same-target ready group freezes one multi-member wave before claims and builds (RFC-96), and merges commit serially at the canonical head (the build phase freezes the current product tree as the wave base when it opens a wave) — and repeats until `emery plan status` projects `drained` or a stop condition halts it (exit 2, `plan-execute-stopped`). When a `decomposition.yaml` binds the plan, the loop also records durable domain-convergence rounds (RFC-96 D8): a frontier round verifies a multi-member wave's composed candidate before it commits, and a complete round verifies each domain's accepted tree once every child and dependency lands — content-addressed `DomainRound` records under `targets/<target>/domains/`, journaled as `domain.convergence.recorded` and reused by identity on restart; a target drains only when its root domain holds a passing complete round for the current revision and accepted CID. The merge phase reads the entry's [`allow-composition-replace`](#emery-plan-amend) field to decide whether a whole-document composition may overwrite a non-empty baseline. The loop holds the create-exclusive `.emery/change/guest.lock` marker for the run's lifetime — a second driver session exits with `guest-marker-held`.

Stops render the `emery plan status` projection verbatim: the closed reason (`refinement-required`, `refine-failed`, `build-failed`, `merge-conflict`, `merge-postflight-failed`, `slice-dropped`, `merge-incomplete`, `stuck`, `boundary-escalation`, `refine-budget-exhausted`, `domain-frontier-failed`, `domain-complete-failed`), the failure detail from the journal, a one-line hint, and the literal resume command. Re-running `emery plan execute` after a build / preflight-merge stop resumes from the same active entry; a `refinement-required` stop resumes through `emery plan refine`. A `boundary-escalation` stop resumes through `emery plan amend --proposal <digest>` (then `emery plan refine` on the new children); `refine-budget-exhausted` resumes through `emery plan refine`. After `merge-postflight-failed`, the entry already projects `done` (non-rollback); re-running execute acknowledges the sticky stop (`plan.merge-postflight.acknowledged`) and continues the queue — or drains when no pending entries remain.

Exit codes: `0` when the loop drains; `2` for a stop (`plan-execute-stopped`), a missing/stale refinement (`plan-refinement-required`), an epoch refusal (`plan-epoch-stale`), or a held marker (`guest-marker-held`).

JSON output: the [`emery plan execute` envelope](../cli-output-shapes.md#emery-plan-execute) — the completed `phases[]` and the drained line; a stop surfaces on the error envelope instead.

### emery plan status

Project the plan's execution state into a deterministic `next-action`. Read-only — computed from artifacts and the fact union; emits no journal event.

```bash
emery plan status [--format json]
```

The projection reads plan topology, slice artifacts / phase timestamps, and the per-writer fact union. The `next-action` field resolves to one of:

| `next-action` | Meaning |
|---------------|---------|
| `refine <slice>` / `build <slice>` / `merge <slice>` | The step the workflow would run next for the candidate entry (an active `in-progress` entry, else the entry the loop would claim). `refine <slice>` resolves through `emery plan refine`; `build` / `merge` are execute-loop phases. |
| `stop <reason>` | Halt the loop; the `stop` sub-body carries the closed reason, optional journal `detail`, and a one-line operator hint. |
| `drained` | No `pending` or `in-progress` entries remain — text mode renders the literal `drained — run /emery:finalize <name>` string. |

Text mode also prints `ready:` / `authorized:` milestones (RFC-86 D22) — never an `approved` label — and a debt line counting deferred gaps with conflicts broken out (e.g. `3 deferred gaps (2 unknown, 1 conflict)`). Ready stays clean-only: zero open **and** zero deferred findings; a debt-carrying plan reaches build via Authorized. Stop reasons are a closed set: `refine-failed` / `build-failed` / `merge-conflict` (the awaited phase's most recent journal terminal — `slice.synthesize.failed` / `slice.build.failed` / `slice.merge.failed` — is a failure, scoped to the active entry's active window), `merge-postflight-failed` (the target's postflight gate failed after wave commit — entry projects `done` and is archived; sticky until `emery plan execute` acknowledges), `slice-dropped`, `merge-incomplete`, `stuck` (pending entries blocked on unmet dependencies), `boundary-escalation` (inert proposal at `planning/proposals/<digest>.yaml`; resume is `emery plan amend --proposal <digest>`), `refine-budget-exhausted` (resume is `emery plan refine`), and the RFC-96 D8 domain pair — `domain-frontier-failed` (a failed domain verification over a composed wave candidate parks the wave; repairing the members retracts it) and `domain-complete-failed` (the accepted tree's domain verification failed or has not passed; drain and publication stay blocked until an authorized repair lands).

With `--format json` the body carries `plan`, `counts` (`pending` / `in-progress` / `done`), `active`, `next-action` (the rendered string), `action` (the closed verb), `slice`, `project`, `ready`, `authorized`, `gaps`, the optional `stop` sub-body, and the re-entry fields: `current-step` / `last-completed` (the candidate slice's position in the `refine → build → merge` loop, `null` outside a dispatchable slice) and `resume` — the literal command or skill invocation that makes progress (`emery plan refine`, `emery plan execute`, `/emery:finalize <name>`, …), `null` when no single command does (`stuck`, `slice-dropped`). A fresh plan's `resume` (nothing done, nothing in progress) is `/emery:refine` while refinement is outstanding, then `/emery:execute`; refinement resumes through `emery plan refine`, build and merge through the execute loop.

### emery plan gaps

Read-only typed gap inventory across in-scope slices.

```bash
emery plan gaps [--format json]
```

Lists `(slice, req, status)` rows for `unknown` / `conflict` / `divergence` from `model.yaml` (else `specs/*/spec.md`), each `unknown` / `conflict` row with its computed **disposition** (`open | deferred`, joined from the deferral fact union; `[divergence]` rows take no disposition). Deferred rows render their reason, with deferred conflicts listed separately from deferred unknowns. Dropped slices are excluded. When findings share a contributing `(source, lead)`, the projection annotates the group — presentation only; dispositions and the gap gate stay per-requirement.

### emery plan add

Append a new entry to the plan.

```bash
emery plan add <name> --target <key> [--description "<text>"] [--depends-on <entry>...] [--source <key>=<lead>...]
```

Creates the entry; it projects `pending` until claimed. `--target` is required and must name a key in `plan.yaml.targets`. When `decomposition.yaml` exists, add/amend/remove reproject through it; otherwise a hierarchy-shaped edit refuses `plan-mutation-ambiguous`.

Exit codes: `0` success; `2` for validation refusals (duplicate entry name, unknown `depends-on` or source references).

JSON output: the [`emery plan add` envelope](../cli-output-shapes.md#emery-plan-add) — the created `entry` body plus the plan identity.

### emery plan amend

Edit topology fields on an existing **entry** (one positional — the slice name; there is a single active `plan.yaml`), or apply a retained amendment with `--proposal`. Use for divergence stamps, authority overrides, the composition-replace merge authorization, and surgical source/depends-on edits. Topology edits reproject through `decomposition.yaml` when it exists. For grouping changes prefer re-running `emery plan author --force` (wholesale replace of a still-replaceable plan); for deferral use `emery plan remove`.

```bash
emery plan amend <entry> [--description "<text>"] [--depends-on <entry>...]
emery plan amend <entry> --add-source <key>=<lead>
emery plan amend <entry> --remove-source <key>
emery plan amend <entry> --divergence likely|accepted|rejected
emery plan amend <entry> --authority-override <kind>=<source>
emery plan amend <entry> --allow-composition-replace true|false
emery plan amend --proposal <digest>
```

`--proposal <digest>` applies a retained document at `planning/proposals/<digest>.yaml` (`Boundary` / `Ownership`). Envelope and definition-revision documents refuse `plan-proposal-kind` — they are not amendments. A successful apply journals `plan.amend.applied` and invalidates the closed-plan epoch. Compare-and-set refusals: `plan-proposal-stale`, `plan-proposal-live`, `plan-proposal-preserve`, `plan-proposal-cycle`, `plan-proposal-not-found`, `plan-proposal-malformed`. Combining `--proposal` with entry-edit flags is `Error::Argument`.

`--allow-composition-replace` sets the entry's `allow-composition-replace` field: it authorizes a whole-document (`screens:`) slice composition to overwrite a non-empty baseline when the execute loop merges this slice. Reserved for intentional full-baseline rewrites; routine per-screen edits flow through `delta:` and never need it. Omit the flag to leave the field unchanged.

Ladder labels project from facts; amend does not write status fields. v1 has no per-entry `failed`, `blocked`, or `skipped` — build failures and merge conflicts leave the active entry projecting `in-progress`.

A slice binds at most one lead per source key (a duplicate would silently overwrite `evidence/<source>.yaml` at refine time). `--add-source` refuses a key the entry already binds with `duplicate-source-key` (exit 2); a duplicate introduced via the wholesale `--sources` replacement rolls back as `plan-amend-validation-failed`. Re-sizing — replacing the lead bound under an existing key via `--sources <key>=<other-lead>` — stays legal.

JSON output: the [`emery plan amend` envelope](../cli-output-shapes.md#emery-plan-amend) — the post-amend `entry` body; absent fields surface as `null` or `[]`.

### emery plan remove

Drop a plan entry while the plan is still replaceable (every entry still projects `pending`). Pre-execution only — defers the entry's lead(s) without re-surveying `leads.md`.

```bash
emery plan remove <entry>
```

Refuses with `plan-remove-plan-not-replaceable` when any entry no longer projects `pending`. Refuses with `plan-remove-entry-referenced` when another entry lists `<entry>` in `depends-on`.

### emery plan drop

Abandon one plan entry's slice without merging.

```bash
emery plan drop <entry> [--reason "<rationale>"]
```

Stamps the slice `dropped` (persisting the reason in `metadata.yaml.drop_reason`) and moves the slice tree to `.emery/change/archive/`. The entry stays on the plan and projects the `slice-dropped` stop — a dropped slice remains in-scope for gap accounting (RFC-86 D24).

Exit codes: `0` success (the body carries the archive path); `1` for an unknown entry (`plan-entry-not-found`) or a never-refined entry with no slice tree (`plan-drop-no-slice` — curate that entry with `emery plan remove` instead).

### Decomposition (inside `emery plan author`)

Authoring binds the reviewed handoff, surveys every source into `leads.md`, decomposes the catalog into `decomposition.yaml`, and projects `plan.yaml.slices[]` from that tree. `slices[].target` is required and names a key in `plan.yaml.targets`. `plan add` / topology `amend` / `remove` reproject through `decomposition.yaml` when it exists.

The propose gate authors `change.md` orientation (counts + binding table) through the typed proposal DTOs (kebab wire fields, closed `kind: request | response`). Cross-source matching is agent judgment inside decomposition; the operator curates during plan review.

**Replaceable gate.** Re-authoring with `--force` rebinds the same reviewed handoff. A still-replaceable plan wholesale-replaces every slice; a non-pending plan refuses `plan-reconcile-plan-not-replaceable`.

Validation codes (all exit 2):

| Code | Meaning |
|------|---------|
| `proposal-schema` | The judgment response failed JSON-Schema validation. |
| `plan-reconcile-empty-catalog` | `leads.md` surfaced no leads to decompose. |
| `plan-reconcile-lead-orphan` | A cited `(source, lead)` is not in the surveyed catalog. |
| `lead-coverage-orphan` | The grouped leads do not achieve total coverage — a surveyed lead is referenced by no slice. (A lead referenced by more than one slice is legal fan-out.) |
| `plan-reconcile-slice-source-collision` | A slice names more than one lead from the same source. |
| `plan-reconcile-slice-name-invalid` | A slice `name` is not kebab-case. |
| `plan-reconcile-slice-name-collision` | Two slices resolve to the same plan slice name. |
| `plan-reconcile-depends-on-cycle` | The projected `depends-on` edges form a cycle. |
| `plan-reconcile-target-unknown` | A slice names a `target` absent from `plan.yaml.targets`. |
| `plan-reconcile-plan-not-replaceable` | The plan carries a non-pending entry. |
| `plan-mutation-ambiguous` | A direct `plan add` / `amend` / `remove` cannot uniquely reproject through `decomposition.yaml`. |

The propose envelopes are owned by the typed wire DTOs in [`crates/project/src/plan/propose.rs`](../../../crates/project/src/plan/propose.rs) (closed `kind: request | response`); the response's judgment-answer schema is generated from them by `project::answers::proposal`. See [CLI output shapes](../cli-output-shapes.md) for the envelope bodies.

### emery plan archive

Archive a completed plan.

```bash
emery plan archive
```

Moves `plan.yaml` and `.emery/change/plans/<name>/` to `.emery/change/archive/plans/<YYYYMMDD>-<name>/`, then runs the change-scoped snapshot collection: the archived change's pins (wave bases, `builds/<digest>.yaml`) stop being GC roots, so snapshot-store objects reachable only from them are deleted (RFC-88 D2). Objects still reachable from a live slice tree survive.

When the change carried deferred debt into the baseline, the archive prints the carried-debt summary (slice, requirement, reason, age) — advisory only; archiving never blocks on debt. The rows stay in the baseline, projected by [`emery debt`](debt.md).

Exit codes: `0` success; `1` for `plan-has-outstanding-work` when the plan still has non-terminal entries, or `snapshot-sweep-failed` when the plan archived but the collection could not complete.

JSON output: the [`emery plan archive` envelope](../cli-output-shapes.md#emery-plan-archive) — the `archived` destination path, `archived-plans-dir` when a per-plan authoring directory was swept, and `swept-objects` (snapshot objects deleted by the collection).

## See also

- [emery slice](slice.md) -- the read-only per-slice projections.
- [Skills](../skills/index.md) -- the `/emery:*` wrappers
- [`/emery:plan` skill body](../../../plugins/emery/skills/plan/SKILL.md)
- [`/emery:refine` skill body](../../../plugins/emery/skills/refine/SKILL.md)
- [`/emery:finalize` skill body](../../../plugins/emery/skills/finalize/SKILL.md)
- [Configuration Files](../configuration.md) -- the plan.yaml format
