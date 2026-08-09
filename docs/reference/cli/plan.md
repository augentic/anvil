# emery plan

Scaffold, populate, validate, execute, and archive change plans. The `plan` verb is the top-level home of every `plan.yaml` operation; each verb on this page is invoked as `emery plan <verb>`. `author → execute → archive` is the whole workflow spine; the rest of the group is curation (`add` / `amend` / `remove` / `drop`) and read-only projection (`status` / `gaps` / `validate`).

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`author`](#emery-plan-author) | Guest-routed authoring orchestration: scaffold `plan.yaml` (refuses an existing plan unless `--force` recreates it), survey every bound source, reconcile leads into `slices[]`, validate, exit with the review hint. Invoked by `/emery:plan`. |
| [`execute`](#emery-plan-execute) | Guest-routed driver loop: at start appends `plan.execute.started` (authorization epoch), then claims → refines → builds → merges per entry under gap gates until `drained` or a stop. Holds the `.emery/guest.lock` marker. Optional `--waive` / `--reason` for `[unknown]` only. |
| [`add`](#emery-plan-add) | Append a new entry to the plan (projects `pending` until claimed). |
| [`amend`](#emery-plan-amend) | Edit topology fields (`description`, `depends-on`, `sources`), divergence stamps, authority overrides, and the `allow-composition-replace` merge authorization on an existing entry. |
| [`remove`](#emery-plan-remove) | Drop an entry while the plan is still replaceable (every entry still projects `pending`). |
| [`drop`](#emery-plan-drop) | Abandon one entry's already-refined slice without merging — stamps `dropped` and archives the slice tree. |
| [reconciliation](#lead-reconciliation-inside-emery-plan-author) | The reconcile leg inside `emery plan author`: validates the agent grouping and replaces `slices[]` on a replaceable plan. |
| [`validate`](#emery-plan-validate) | Structural and referential integrity check (cycles, unknown deps) plus health diagnostics (`cycle-in-depends-on`, `orphan-source`). First triage step when `emery plan execute` reports `stuck`. |
| [`status`](#emery-plan-status) | Read-only projection into a deterministic `next-action` (`refine|build|merge <slice>` / `review-gaps` / `stop <reason>` / `drained`) plus Ready / Authorized. |
| [`gaps`](#emery-plan-gaps) | Read-only typed gap inventory (`unknown` / `conflict` / `divergence`) with shared-lead grouping annotations. |
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
- **Bare adapter names** persist bare in `plan.yaml` (no auto version stamp). A cache-seeded binding (`emery adapter add`) resolves the seed; an unseeded bare name resolves local-first — newest installed store version, else pull-latest provisioning. The resolved version is logged to stderr; the survey fan-out and every later refine-phase extract dispatch the same local resolution.
- **Explicit pins** (`emery:<name>@<semver>`) stamp `version:` on the binding and install through the standard pull-on-miss path.
- **`--intent`** creates an implicit `intent` value binding that rides the same resolution rules.

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

Drive the plan through refine → build → merge per entry under the guest lock. At start appends `plan.execute.started` with typed `closed-plan` coverage — there is no separate `plan approve` / `plan refine` verb and no projected `approved` rung. Re-entry on an already-drained plan is a read-only no-op (no new epoch); on any other resume the fresh epoch replaces the previous one, so unknown-waivers must be re-supplied with `--waive` on every run.

```bash
emery plan execute [--waive <slice>/<req>]... [--reason <text>]
```

| Flag | Description |
|------|-------------|
| `--waive <slice>/<req>` | Repeatable. Waive an open `[unknown]` requirement on the covering epoch. |
| `--reason <text>` | Required when any `--waive` is present; one reason applies to every selector. |

Misuse (`--reason` without waivers, waive of a missing / non-unknown / conflict gap) exits 2 with `plan-waiver-invalid`. Before each build the gap gate refuses `[conflict]` always and `[unknown]` unless a matching waiver nests on the newest covering epoch; stale covered artifacts refuse with `plan-epoch-stale`.

**Coverage owns refine staleness.** The start-of-run coverage assembly classifies every entry: an unrefined entry is `refine-under-epoch`, an already-`refined` entry whose recorded `base.yaml` pins still match its inputs is `existing`, and a `refined` entry whose pins drifted (`slice-base-drifted` / `slice-evidence-stale`) is treated as `refine-under-epoch` — the loop re-refines only the affected slices under the new epoch, then stops at the gap gate for review. The iteration loop after any input change is simply: fix inputs → `emery plan execute` → review gaps → `emery plan execute`.

The loop claims the next eligible entry, runs the refine, build, and merge phases, and repeats until `emery plan status` projects `drained` or a stop condition halts it (exit 2, `plan-execute-stopped`). The merge phase reads the entry's [`allow-composition-replace`](#emery-plan-amend) field to decide whether a whole-document composition may overwrite a non-empty baseline. The loop holds the create-exclusive `.emery/guest.lock` marker for the run's lifetime — a second driver session exits with `guest-marker-held`.

Stops render the `emery plan status` projection verbatim: the closed reason (`refine-failed`, `build-failed`, `merge-conflict`, `merge-postflight-failed`, `slice-dropped`, `merge-incomplete`, `stuck`), the failure detail from the journal, a one-line hint, and the literal resume command. Re-running `emery plan execute` after a refine / build / preflight-merge stop resumes from the same active entry. After `merge-postflight-failed`, the entry already projects `done` (non-rollback); re-running execute acknowledges the sticky stop (`plan.merge-postflight.acknowledged`) and continues the queue — or drains when no pending entries remain.

Exit codes: `0` when the loop drains; `2` for a stop (`plan-execute-stopped`), gap/epoch/waiver refusal, or a held marker (`guest-marker-held`).

JSON output: the [`emery plan execute` envelope](../cli-output-shapes.md#emery-plan-execute) — the completed `phases[]` and the drained line; a stop surfaces on the error envelope instead.

### emery plan status

Project the plan's execution state into a deterministic `next-action`. Read-only — computed from artifacts and the fact union; emits no journal event.

```bash
emery plan status [--format json]
```

The projection reads plan topology, slice artifacts / phase timestamps, and the per-writer fact union. The `next-action` field resolves to one of:

| `next-action` | Meaning |
|---------------|---------|
| `refine <slice>` / `build <slice>` / `merge <slice>` | The phase the execute loop would dispatch next for the candidate entry (an active `in-progress` entry, else the entry the loop would claim). |
| `review-gaps` | In-scope slices are refined but open conflicts / unknowns block a clean Ready path. |
| `stop <reason>` | Halt the loop; the `stop` sub-body carries the closed reason, optional journal `detail`, and a one-line operator hint. |
| `drained` | No `pending` or `in-progress` entries remain — text mode renders the literal `drained — run /emery:finalize <name>` string. |

Text mode also prints `ready:` / `authorized:` milestones (RFC-86 D22) — never an `approved` label. Stop reasons are a closed set: `refine-failed` / `build-failed` / `merge-conflict` (the awaited phase's most recent journal terminal — `slice.synthesize.failed` / `slice.build.failed` / `slice.merge.failed` — is a failure, scoped to the active entry's active window), `merge-postflight-failed` (the target's postflight gate failed after wave commit — entry projects `done` and is archived; sticky until `emery plan execute` acknowledges), `slice-dropped`, `merge-incomplete`, and `stuck` (pending entries blocked on unmet dependencies).

With `--format json` the body carries `plan`, `counts` (`pending` / `in-progress` / `done`), `active`, `next-action` (the rendered string), `action` (the closed verb), `slice`, `project`, `ready`, `authorized`, `gaps`, the optional `stop` sub-body, and the re-entry fields: `current-step` / `last-completed` (the candidate slice's position in the `refine → build → merge` loop, `null` outside a dispatchable slice) and `resume` — the literal command or skill invocation that makes progress (`emery plan execute`, `emery plan execute --waive…`, `/emery:finalize <name>`, …), `null` when no single command does (`stuck`, `slice-dropped`). A fresh plan's `resume` (nothing done, nothing in progress) is `/emery:execute`; every phase resumes through the execute loop — there are no phase-breakout verbs.

### emery plan gaps

Read-only typed gap inventory across in-scope slices.

```bash
emery plan gaps [--format json]
```

Lists open `(slice, req, status)` rows for `unknown` / `conflict` / `divergence` from `model.yaml` (else `specs/*/spec.md`). Dropped slices are excluded. When findings share a contributing `(source, lead)`, the projection annotates the group — presentation only; waivers stay `--waive <slice>/<req>` on `emery plan execute`.

### emery plan add

Append a new entry to the plan.

```bash
emery plan add <name> [--description "<text>"] [--depends-on <entry>...] [--sources <key>...]
```

Creates the entry; it projects `pending` until claimed.

Exit codes: `0` success; `2` for validation refusals (duplicate entry name, unknown `depends-on` or source references).

JSON output: the [`emery plan add` envelope](../cli-output-shapes.md#emery-plan-add) — the created `entry` body plus the plan identity.

### emery plan amend

Edit topology fields on an existing **entry** (one positional — the slice name; there is a single active `plan.yaml`). Use for divergence stamps, authority overrides, the composition-replace merge authorization, and surgical source/depends-on edits. For grouping changes prefer re-running `emery plan author --force` (wholesale replace of a still-replaceable plan); for deferral use `emery plan remove`.

```bash
emery plan amend <entry> [--description "<text>"] [--depends-on <entry>...]
emery plan amend <entry> --add-source <key>=<lead>
emery plan amend <entry> --remove-source <key>
emery plan amend <entry> --divergence likely|accepted|rejected
emery plan amend <entry> --authority-override <kind>=<source>
emery plan amend <entry> --allow-composition-replace true|false
```

`--allow-composition-replace` sets the entry's `allow-composition-replace` field: it authorizes a whole-document (`screens:`) slice composition to overwrite a non-empty baseline when the execute loop merges this slice. Reserved for intentional full-baseline rewrites; routine per-screen edits flow through `delta:` and never need it. Omit the flag to leave the field unchanged.

Ladder labels project from facts; amend does not write status fields. v1 has no per-entry `failed`, `blocked`, or `skipped` — build failures and merge conflicts leave the active entry projecting `in-progress`.

A slice binds at most one lead per source key (a duplicate would silently overwrite `evidence/<source>.yaml` at refine time). `--add-source` refuses a key the entry already binds with `duplicate-source-key` (exit 2); a duplicate introduced via the wholesale `--sources` replacement rolls back as `plan-amend-validation-failed`. Re-sizing — replacing the lead bound under an existing key via `--sources <key>=<other-lead>` — stays legal.

JSON output: the [`emery plan amend` envelope](../cli-output-shapes.md#emery-plan-amend) — the post-amend `entry` body; absent fields surface as `null` or `[]`.

### emery plan remove

Drop a plan entry while the plan is still replaceable (every entry still projects `pending`). Pre-execution only — defers the entry's lead(s) without re-surveying `discovery.md`.

```bash
emery plan remove <entry>
```

Refuses with `plan-remove-plan-not-replaceable` when any entry no longer projects `pending`. Refuses with `plan-remove-entry-referenced` when another entry lists `<entry>` in `depends-on`.

### emery plan drop

Abandon one plan entry's slice without merging.

```bash
emery plan drop <entry> [--reason "<rationale>"]
```

Stamps the slice `dropped` (persisting the reason in `metadata.yaml.drop_reason`) and moves the slice tree to `.emery/archive/`. The entry stays on the plan and projects the `slice-dropped` stop — a dropped slice remains in-scope for gap accounting (RFC-86 D24).

Exit codes: `0` success (the body carries the archive path); `1` for an unknown entry (`plan-entry-not-found`) or a never-refined entry with no slice tree (`plan-drop-no-slice` — curate that entry with `emery plan remove` instead).

### Lead reconciliation (inside `emery plan author`)

The reconcile leg inside the guest-routed `emery plan author` groups the surveyed `discovery.md` leads into the plan's `slices[]` rows.

- The **request** side is a flat catalog of raw `(source, lead)` leads read 1:1 from `discovery.md`, plus the project topology (always at least one project, each carrying its normalized `target` adapter).
- The **write** side is the **only slice writer**. It schema-validates the judgment response (`proposal-schema`), re-reads `discovery.md`, rebuilds the lead catalog, validates the agent's `slices[]` grouping, enforces total lead coverage, validates the explicit slice names, binds projects (auto-binding the sole project and deriving each slice's `target` from the bound project), atomically replaces `plan.yaml.slices[]`, then emits a single `plan.reconcile.completed` journal event. It never trusts a stale snapshot — `discovery.md` and the topology are re-read every invocation.

**Replaceable gate.** The write runs only while the plan is replaceable — every entry `pending`; otherwise it fails with `plan-reconcile-plan-not-replaceable`. Re-authoring a still-replaceable plan wholesale-replaces every slice.

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

### emery plan archive

Archive a completed plan.

```bash
emery plan archive
```

Moves `plan.yaml` and `.emery/plans/<name>/` to `.emery/archive/plans/<YYYYMMDD>-<name>/`, then runs the change-scoped snapshot collection: the archived change's pins (`base.yaml`, `builds/<digest>.yaml`) stop being GC roots, so snapshot-store objects reachable only from them are deleted (RFC-88 D2). Objects still reachable from a live slice tree survive.

Exit codes: `0` success; `1` for `plan-has-outstanding-work` when the plan still has non-terminal entries, or `snapshot-sweep-failed` when the plan archived but the collection could not complete.

JSON output: the [`emery plan archive` envelope](../cli-output-shapes.md#emery-plan-archive) — the `archived` destination path, `archived-plans-dir` when a per-plan authoring directory was swept, and `swept-objects` (snapshot objects deleted by the collection).

## See also

- [emery slice](slice.md) -- the read-only per-slice projections.
- [Skills](../skills/index.md) -- the `/emery:*` wrappers
- [`/emery:plan` skill body](../../../plugins/emery/skills/plan/SKILL.md)
- [`/emery:finalize` skill body](../../../plugins/emery/skills/finalize/SKILL.md)
- [Configuration Files](../configuration.md) -- the plan.yaml format
