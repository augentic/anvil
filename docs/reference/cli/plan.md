# emery plan

Scaffold, populate, validate, transition, and archive change plans. The `plan` verb is the top-level home of every `plan.yaml` operation; each verb on this page is invoked as `emery plan <verb>`.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`author`](#emery-plan-author) | Guest-routed authoring orchestration: scaffold `plan.yaml` (refuses an existing plan unless `--force` recreates it), survey every bound source, reconcile leads into `slices[]`, validate, exit with the review hint. Invoked by `/emery:plan`. |
| [`execute`](#emery-plan-execute) | Guest-routed driver loop: at start appends `plan.execute.started` (authorization epoch), then advances → refines → builds → merges under gap gates until `drained` or a stop. Holds the `.emery/guest.lock` marker. Optional `--waive` / `--reason` for `[unknown]` only. |
| [`add`](#emery-plan-add) | Append a new entry to the plan (projects `pending` until claimed). |
| [`amend`](#emery-plan-amend) | Edit topology fields (`project`, `description`, `depends-on`, `sources`) on an existing entry. |
| [`remove`](#emery-plan-remove) | Drop an entry while the plan is still replaceable (every entry still projects `pending`). |
| [reconciliation](#lead-reconciliation-inside-emery-plan-author) | The reconcile leg inside `emery plan author`: validates the agent grouping and replaces `slices[]` on a replaceable plan. |
| [`undo`](#emery-plan-undo) | Walk a plan entry's projected ladder backwards via `fact.retracted`, one rung per call by default or several with `--to`. Labels are `pending | in-progress | done` only. |
| [`validate`](#emery-plan-validate) | Structural and referential integrity check (cycles, unknown deps, multi-repo invariants) plus three health diagnostics (`cycle-in-depends-on`, `orphan-source`, `stale-workspace-clone`). First triage step when `emery plan execute` reports `stuck`. |
| [`advance`](#emery-plan-advance) | Claim the next eligible slice so it projects `in-progress`, or return an already-active entry. |
| [`status`](#emery-plan-status) | Read-only projection into a deterministic `next-action` (`refine|build|merge <slice>` / `review-gaps` / `stop <reason>` / `drained`) plus Ready / Authorized. |
| [`gaps`](#emery-plan-gaps) | Read-only typed gap inventory (`unknown` / `conflict` / `divergence`) with shared-lead re-refine suggestions. |
| [`archive`](#emery-plan-archive) | Move a completed `plan.yaml` and `.emery/plans/<name>/` to `.emery/archive/plans/`. Usually invoked by `/emery:finalize` after the operator confirms publication (commits, PRs, review) is complete — publication itself stays operator-owned, outside Emery. |

## Subcommands

### emery plan author

Guest-routed authoring orchestration — scaffold, survey, reconcile, validate, exit for operator review. Invoked by `/emery:plan`.

```bash
emery plan author <name> [--source <key>=<adapter>:<binding>]... [--intent "<string>"] [--force]
```

| Argument | Description |
|----------|-------------|
| `name` | Kebab-case change name. |
| `--source` | Repeatable structured binding `<key>=<adapter>:<binding>`. `<key>` is an operator-chosen label for the binding — it becomes the slot name in `plan.yaml.sources` that plan entries and evidence files reference (e.g. `legacy`, `docs`); pick anything kebab-case and memorable. `<adapter>` is an explicit kebab-case source adapter name. `<binding>` is either a path (`<adapter>:<path>` — URLs containing `:` like `git@github.com:org/foo.git` round-trip cleanly because only the first colon is significant) or a `value:`-prefixed literal (`<adapter>:value:<literal>` — used by `intent`). Example: `--source legacy=typescript:./legacy`. |
| `--intent` | Sugar for a single implicit `intent` value binding. |
| `--force` | Recreate an existing plan unconditionally, whatever its entry statuses (no archive — the previous `plan.yaml` is simply overwritten). `/emery:plan` confirms before passing it. |

Exit codes: `0` success (exits with the review hint); `2` for `plan-already-exists` (present plan without `--force`) and reconcile validation failures (see [Lead reconciliation](#lead-reconciliation-inside-emery-plan-author)); `1` for adapter resolution and I/O failures.

JSON output: the [`emery plan author` envelope](../cli-output-shapes.md#emery-plan-author) — surveyed sources, slice count, and the literal review `hint`.

Behavior notes:

- **Order of operations.** Every binding is resolved up front, then `plan.yaml` is scaffolded at the repo root, then the survey and reconcile legs run. An unresolvable adapter (unpublished name, `emery_floor`) fails fast with nothing on disk.
- **Bare adapter names** persist bare in `plan.yaml` (no auto version stamp). A cache-seeded binding (`emery adapter add`) resolves the seed; an unseeded bare name resolves local-first — newest installed store version, else pull-latest provisioning. The resolved version is logged to stderr; the survey fan-out and every later `slice refine` extract dispatch the same local resolution.
- **Explicit pins** (`emery:<name>@<semver>`) stamp `version:` on the binding and install through the standard pull-on-miss path.
- **`--intent`** creates an implicit `intent` value binding that rides the same resolution rules.

### emery plan validate

Check structural and referential integrity of the plan, plus the four
health diagnostics below.

```bash
emery plan validate
```

Base shape checks: duplicate entry names, dependency cycles, unknown `depends-on` / `sources` references, duplicate source keys within one entry (`duplicate-source-key` — a slice binds at most one lead per source), and the following cross-registry checks when `registry.yaml` is present (there is no plan-wide single-active-entry rule — exclusivity is per-slice claims):

- `project-not-in-registry` (important) -- every `project` value must match a `projects[].name` in the registry.
- `project-missing-multi-repo` (important) -- when the registry has multiple projects, every change must carry a `project` field.
- `topology-cache-stale` (suggestion) -- a workspace slot's `project.yaml` (target adapter, description) or its baseline projection (`surface[]` / `recent[]`) has diverged from the committed `.emery/topology.lock`. The project's `project.yaml` plus its baseline are authoritative; regenerate the lock through the repository's operator-owned topology tooling.

Health diagnostics layered on top — first triage step when `emery plan execute` reports `stuck`:

| Code | Severity | Meaning | Recovery |
|------|----------|---------|----------|
| `cycle-in-depends-on` | important | Dependency cycle in `depends-on`. `next_eligible` silently skips cycles at runtime; validate is the only place where the cycle structure surfaces. Structured evidence carries the cycle path, e.g. `["a", "b", "a"]`. | `emery plan amend <entry> --depends-on …` to break the cycle, then re-run validate. |
| `orphan-source` | suggestion | Top-level `sources:` key declared but no plan entry references it (the inverse of `unknown-source`). | Either reference the key from an entry's `sources:` list or remove the declaration. |
| `stale-workspace-clone` | suggestion | Workspace clone's signature has drifted from the registry, or no signature is readable at all. Reason is one of `signature-changed` (URL or adapter diverged) or `slot-mismatch` (slot materialisation does not match the registry). | Refresh or rematerialize the slot through normal repository tooling. |

JSON output (`--format json`) is the neutral `DiagnosticReport` envelope (`{ version, summary, findings }`) shared with `emery slice validate` — the typed shape lives at [`crates/diagnostics/src/diagnostic.rs`](../../../crates/diagnostics/src/diagnostic.rs). Each finding carries `rule-id` (kebab-case, e.g. `duplicate-name` / `cycle-in-depends-on`), `severity` (`critical` / `important` / `suggestion` / `optional`), `impact` (the human-readable message), optional `slice` (the entry name), and `evidence`. The three health diagnostics attach their machine-readable payload to `evidence` as `{ "kind": "structured", "data": … }`; base validate findings carry a plain `snippet` evidence.

Exit code: `0` when no blocking finding fires (suggestions are non-fatal); `2` when any blocking (`critical` / `important`) finding fires.

### emery plan execute

Drive the plan through refine → build → merge per entry under the guest lock. At start appends `plan.execute.started` with typed `closed-plan` coverage — there is no separate `plan approve` / `plan refine` verb and no projected `approved` rung.

```bash
emery plan execute [--waive <slice>/<req>]... [--reason <text>]
```

| Flag | Description |
|------|-------------|
| `--waive <slice>/<req>` | Repeatable. Waive an open `[unknown]` requirement on the covering epoch. |
| `--reason <text>` | Required when any `--waive` is present; one reason applies to every selector. |

Misuse (`--reason` without waivers, waive of a missing / non-unknown / conflict gap) exits 2 with `plan-waiver-invalid`. Before each build the gap gate refuses `[conflict]` always and `[unknown]` unless a matching waiver nests on the newest covering epoch; stale covered artifacts refuse with `plan-epoch-stale`.

The loop advances the next eligible entry, runs the refine, build, and merge orchestrations, and repeats until `emery plan status` projects `drained` or a stop condition halts it (exit 2, `plan-execute-stopped`). It holds the create-exclusive `.emery/guest.lock` marker for the run's lifetime — a second driver session exits with `guest-marker-held`.

**Workspace routing is unsupported.** The loop runs single-project plans only: a workspace plan root (`workspace: true` in `project.yaml`) or any `project`-scoped plan entry refuses with `plan-execute-workspace-unsupported` (exit 2) before any adapter lookup or plan state is touched. Drive workspace plans hand-driven instead — `emery plan advance`, then the `/emery:refine` → `/emery:build` → `/emery:merge` breakouts. The read-only `emery plan status` stays slot-aware and never refuses.

Stops render the `emery plan status` projection verbatim: the closed reason (`refine-failed`, `build-failed`, `merge-conflict`, `merge-postflight-failed`, `slice-dropped`, `merge-incomplete`, `stuck`), the failure detail from the journal, a one-line hint, and the literal resume command. Re-running `emery plan execute` after a refine / build / preflight-merge stop resumes from the same active entry. After `merge-postflight-failed`, the entry already projects `done` (non-rollback); re-running execute acknowledges the sticky stop (`plan.merge-postflight.acknowledged`) and continues the queue — or drains when no pending entries remain.

Exit codes: `0` when the loop drains; `2` for a stop (`plan-execute-stopped`), gap/epoch/waiver refusal, a workspace plan (`plan-execute-workspace-unsupported`), or a held marker (`guest-marker-held`).

JSON output: the [`emery plan execute` envelope](../cli-output-shapes.md#emery-plan-execute) — the completed `phases[]` and the drained line; a stop surfaces on the error envelope instead.

### emery plan advance

Claim the next eligible slice so it projects `in-progress`, or return an already-active entry.

```bash
emery plan advance
```

Returns an active `in-progress` entry, or claims the first `pending` entry whose `depends-on` entries all project `done` (`slice.claimed` + `plan.entry.advanced`). Sibling entries may already project `in-progress` — there is no plan-wide single-active gate. When nothing is eligible, the body carries `advanced: null` and a populated `reason` (`drained`, `stuck`, `in-progress`).

Exit codes: `0` (including the `reason` outcomes); `1` when no `plan.yaml` exists; `2` for `slice-claim-conflict`.

JSON output: the [`emery plan advance` envelope](../cli-output-shapes.md#emery-plan-advance) — when an eligible entry is found the response includes `project`, `description`, and `sources` alongside `advanced`; these fields are absent when `reason` is non-null. Use `emery plan status` for a pure read.

### emery plan status

Project the plan's execution state into a deterministic `next-action`. Read-only — computed from artifacts and the fact union; emits no journal event.

```bash
emery plan status [--format json]
```

The projection reads plan topology, slice artifacts / phase timestamps (resolved into the materialised workspace slot when the entry is project-bound), and the per-writer fact union. The `next-action` field resolves to one of:

| `next-action` | Meaning |
|---------------|---------|
| `refine <slice>` / `build <slice>` / `merge <slice>` | Dispatch the named phase for the candidate entry (an active `in-progress` entry, else the entry `plan advance` would take). |
| `review-gaps` | In-scope slices are refined but open conflicts / unknowns block a clean Ready path. |
| `stop <reason>` | Halt the loop; the `stop` sub-body carries the closed reason, optional journal `detail`, and a one-line operator hint. |
| `drained` | No `pending` or `in-progress` entries remain — text mode renders the literal `drained — run /emery:finalize <name>` string. |

Text mode also prints `ready:` / `authorized:` milestones (RFC-86 D22) — never an `approved` label. Stop reasons are a closed set: `refine-failed` / `build-failed` / `merge-conflict` (the awaited phase's most recent journal terminal — `slice.synthesize.failed` / `slice.build.failed` / `slice.merge.failed` — is a failure, scoped to the active entry's active window), `merge-postflight-failed` (the target's postflight gate failed after wave commit — entry projects `done` and is archived; sticky until `emery plan execute` acknowledges), `slice-dropped`, `merge-incomplete`, and `stuck` (pending entries blocked on unmet dependencies).

With `--format json` the body carries `plan`, `counts` (`pending` / `in-progress` / `done`), `active`, `next-action` (the rendered string), `action` (the closed verb), `slice`, `project`, `ready`, `authorized`, `gaps`, the optional `stop` sub-body, and the re-entry fields: `current-step` / `last-completed` (the candidate slice's position in the `refine → build → merge` loop, `null` outside a dispatchable slice) and `resume` — the literal command or skill invocation that makes progress (`/emery:build a`, `/emery:merge a`, `emery plan execute --waive…`, …), `null` when no single command does (`stuck`, `slice-dropped`). A fresh plan's `resume` (nothing done, nothing in progress) is `/emery:execute`.

### emery plan gaps

Read-only typed gap inventory across in-scope slices.

```bash
emery plan gaps [--format json]
```

Lists open `(slice, req, status)` rows for `unknown` / `conflict` / `divergence` from `model.yaml` (else `specs/*/spec.md`). Dropped slices are excluded. When findings share a contributing `(source, lead)`, the projection annotates the group and suggests re-refine selectors — presentation only; waivers stay `--waive <slice>/<req>`.

### emery plan add

Append a new entry to the plan.

```bash
emery plan add <name> [--project <name>] [--description "<text>"] [--depends-on <entry>...] [--sources <key>...]
```

Creates the entry; it projects `pending` until claimed.

Exit codes: `0` success; `2` for validation refusals (duplicate entry name, unknown `depends-on` or source references).

JSON output: the [`emery plan add` envelope](../cli-output-shapes.md#emery-plan-add) — the created `entry` body plus the plan identity.

### emery plan amend

Edit topology fields on an existing **entry** (one positional — the slice name; there is a single active `plan.yaml`). Use for divergence stamps, authority overrides, and surgical source/project/depends-on edits. For grouping changes prefer re-running `emery plan author --force` (wholesale replace of a still-replaceable plan); for deferral use `emery plan remove`.

```bash
emery plan amend <entry> [--project <name>] [--description "<text>"] [--depends-on <entry>...]
emery plan amend <entry> --add-source <key>=<lead>
emery plan amend <entry> --remove-source <key>
emery plan amend <entry> --divergence likely|accepted|rejected
emery plan amend <entry> --authority-override <kind>=<source>
```

Ladder labels project from facts; amend does not write status fields. v1 has no per-entry `failed`, `blocked`, or `skipped` — build failures and merge conflicts leave the active entry projecting `in-progress`.

A slice binds at most one lead per source key (a duplicate would silently overwrite `evidence/<source>.yaml` at refine time). `--add-source` refuses a key the entry already binds with `duplicate-source-key` (exit 2); a duplicate introduced via the wholesale `--sources` replacement rolls back as `plan-amend-validation-failed`. Re-sizing — replacing the lead bound under an existing key via `--sources <key>=<other-lead>` — stays legal.

JSON output: the [`emery plan amend` envelope](../cli-output-shapes.md#emery-plan-amend) — the post-amend `entry` body; absent fields surface as `null` or `[]`.

### emery plan remove

Drop a plan entry while the plan is still replaceable (every entry still projects `pending`). Pre-execution only — defers the entry's lead(s) without re-surveying `discovery.md`.

```bash
emery plan remove <entry>
```

Refuses with `plan-remove-plan-not-replaceable` when any entry no longer projects `pending`. Refuses with `plan-remove-entry-referenced` when another entry lists `<entry>` in `depends-on`.

### Lead reconciliation (inside `emery plan author`)

The reconcile leg inside the guest-routed `emery plan author` groups the surveyed `discovery.md` leads into the plan's `slices[]` rows.

- The **request** side is a flat catalog of raw `(source, lead)` leads read 1:1 from `discovery.md`, plus the project topology (always at least one project, each carrying its normalized `target` adapter).
- The **write** side is the **only slice writer**. It schema-validates the judgment response (`proposal-schema`), re-reads `discovery.md`, rebuilds the lead catalog, validates the agent's `slices[]` grouping, enforces total lead coverage, validates the explicit slice names, binds projects (auto-binding the sole project and deriving each slice's `target` from the bound project), atomically replaces `plan.yaml.slices[]`, then emits a single `plan.reconcile.completed` journal event. It never trusts a stale snapshot — `discovery.md` and the topology are re-read every invocation.

**Replaceable gate.** The write runs only while the plan is replaceable — every entry `pending`; otherwise it fails with `plan-reconcile-plan-not-replaceable`. Re-authoring a still-replaceable plan wholesale-replaces every slice. Each slice's registry project is bound by the agent inside the response, not by a later assignment pass.

Validation codes (all exit 2):

| Code | Meaning |
|------|---------|
| `proposal-schema` | The judgment response failed JSON-Schema validation. |
| `plan-reconcile-empty-catalog` | `discovery.md` surfaced no leads to reconcile. |
| `plan-reconcile-lead-orphan` | A cited `(source, lead)` is not in the surveyed catalog. |
| `lead-coverage-orphan` | The grouped leads do not achieve total coverage — a surveyed lead is referenced by no slice. (A lead referenced by more than one slice is legal fan-out.) |
| `plan-reconcile-slice-source-collision` | A slice names more than one lead from the same source. |
| `plan-reconcile-slice-name-invalid` | A slice `name` is not kebab-case. |
| `plan-reconcile-slice-name-collision` | Two slices resolve to the same plan slice name. |
| `plan-reconcile-depends-on-cycle` | The projected `depends-on` edges form a cycle. |
| `plan-reconcile-project-binding-required` | A slice omits `project` when more than one project exists. |
| `plan-reconcile-project-orphan` | A slice binds a `project` absent from the request topology. |
| `plan-reconcile-plan-not-replaceable` | The plan carries a non-pending entry. |

Both envelopes are owned by the typed wire DTOs in [`crates/project/src/plan/propose.rs`](../../../crates/project/src/plan/propose.rs) (closed `kind: request | response`); the response's judgment-answer schema is generated from them by `project::answers::proposal`. See [CLI output shapes](../cli-output-shapes.md) for the envelope bodies.

### emery plan undo

Walk a plan entry's status backwards.

```bash
emery plan undo <entry> [--to pending|in-progress]
```

Without `--to`, the verb walks one rung backwards (`done → in-progress`, then `in-progress → pending` on a second call). With `--to <status>` it walks rung by rung until the entry reaches the named status (`done → pending` is two rungs in one call). Either way it fires one `plan.transition.undone` event per rung. Forward writes have no undo surface — `emery slice merge` stamps `done` (and re-stamps it when healing a torn merge).

Exit codes: `0` success; `2` when the entry is unknown, already `pending` (nothing to undo), or already at (or below) the `--to` status.

JSON output: the [`emery plan undo` envelope](../cli-output-shapes.md#emery-plan-undo) — the entry with the full `from → to` walk, one pair per rung.

Per-entry `pending` is written by `emery plan add` / `plan amend`; `in-progress` is written only by `emery plan advance`. v1 has no per-entry `failed`, `blocked`, or `skipped` — build failures and merge conflicts leave the active entry `in-progress`.

At most one entry may be `in-progress` at a time.

### emery plan archive

Archive a completed plan.

```bash
emery plan archive
```

Moves `plan.yaml` and `.emery/plans/<name>/` to `.emery/archive/plans/<YYYYMMDD>-<name>/`.

Exit codes: `0` success; `1` for `plan-has-outstanding-work` when the plan still has non-terminal entries.

JSON output: the [`emery plan archive` envelope](../cli-output-shapes.md#emery-plan-archive) — the `archived` destination path plus `archived-plans-dir` when a per-plan authoring directory was swept.

## See also

- [emery slice](slice.md) -- the per-slice CLI verbs the plan loop drives.
- [Change skills](../change-skills/index.md) -- `/emery:plan` and `/emery:finalize` wrappers
- [`/emery:plan` skill body](../../../plugins/emery/skills/plan/SKILL.md)
- [`/emery:finalize` skill body](../../../plugins/emery/skills/finalize/SKILL.md)
- [Configuration Files](../configuration.md) -- plan.yaml and registry format
