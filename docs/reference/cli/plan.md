# emery plan

Scaffold, populate, validate, transition, and archive change plans. The `plan` verb is the top-level home of every `plan.yaml` operation; each verb on this page is invoked as `emery plan <verb>`.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| `author` | Guest-routed authoring orchestration: scaffold `plan.yaml` (refuses an existing plan unless `--force` recreates it), survey every bound source, reconcile leads into `slices[]`, validate, exit at `pending` with the Gate 1 hint. Invoked by `/emery:plan`. |
| [`execute`](#emery-plan-execute) | Guest-routed driver loop and Gate 1: the first run on a `pending` plan stamps `approved`, then claims → refines → builds → merges per entry until `drained` or a stop (exit 2, `plan-execute-stopped`). Holds the `.emery/guest.lock` marker. |
| [`add`](#emery-plan-add) | Append a new entry to the plan in `pending` state. |
| [`amend`](#emery-plan-amend) | Edit non-status fields (`project`, `description`, `depends-on`, `sources`) on an existing entry. |
| [`remove`](#emery-plan-remove) | Drop a pending entry while the plan is still replaceable (Gate 1 deferral). |
| [reconciliation](#lead-reconciliation-inside-emery-plan-author) | The reconcile leg inside `emery plan author`: validates the agent grouping and replaces `slices[]` on a replaceable plan. |
| [`transition`](#emery-plan-transition) | Walk a plan entry's status one rung backwards (`--undo` is the only mode). Per-entry status is `pending | in-progress | done` only; forward writes belong to `plan next` and `slice merge run`. |
| [`validate`](#emery-plan-validate) | Structural and referential integrity check (cycles, unknown deps, multi-repo invariants) plus three health diagnostics (`cycle-in-depends-on`, `orphan-source`, `stale-workspace-clone`). First triage step when `emery plan execute` reports `stuck`. |
| [`next`](#emery-plan-next) | Report the next eligible entry (used by the execute loop and ad-hoc operators). |
| [`status`](#emery-plan-status) | Read-only projection of the plan's execution state into a deterministic `next-action` (`refine|build|merge <slice>` / `stop <reason>` / `drained`). |
| [`archive`](#emery-plan-archive) | Move a completed `plan.yaml` and `.emery/plans/<name>/` to `.emery/archive/plans/`. Usually invoked by `/emery:finalize` after it observes merged PRs. |

## Subcommands

`emery plan author <name> [--source <key>=<adapter>:<path>...] [--intent "<string>"] [--force]` scaffolds `plan.yaml` at the repo root with the given kebab-case name before its survey and reconcile legs run. Each `--source` carries the structured binding shape: an explicit kebab-case `<adapter>` followed by a colon and either a path (`<adapter>:<path>` — URLs containing `:` like `git@github.com:org/foo.git` round-trip cleanly because only the first colon is significant) or a `value:`-prefixed literal (`<adapter>:value:<literal>` — used by `intent`). Refuses with `already-exists` when `plan.yaml` is already present unless `--force` is set; `--force` recreates the plan unconditionally, whatever its lifecycle or entry statuses (no archive — the previous `plan.yaml` is simply overwritten). `/emery:plan` confirms before passing `--force`.

Author ensures every binding up front, before the scaffold write. Bare adapter names persist bare in `plan.yaml` (no auto version stamp): a cache-seeded binding (`emery adapter add`) resolves the seed, and an unseeded bare name resolves local-first (newest installed store version, else pull-latest provisioning) — the resolved version is logged to stderr, and the survey fan-out plus every later `slice refine` extract dispatch the same local resolution. Explicit `emery:<name>@<semver>` pins stamp `version:` on the binding and install through the standard pull-on-miss path. The `--intent` sugar's implicit `intent` binding rides the same rules. An unresolvable adapter (unpublished name, `emery_floor`) fails fast with nothing on disk.

### emery plan validate

Check structural and referential integrity of the plan, plus the four
health diagnostics below.

```bash
emery plan validate
```

Base shape checks: duplicate entry names, dependency cycles, unknown `depends-on` / `sources` references, duplicate source keys within one entry (`duplicate-source-key` — a slice binds at most one lead per source), at most one `in-progress` entry, and the following cross-registry checks when `registry.yaml` is present:

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

Stamp Gate 1 when needed, then drive the plan through refine → build → merge per entry under the guest lock.

```bash
emery plan execute [--actor operator|agent]
```

Invoking execute on a `pending` plan is the approval act: the loop first stamps `pending → approved` and journals `plan.transition.approved` with the closed `actor` enum (`--actor`, default `operator`); the stamp is idempotent on an already-approved plan (no disk write, no journal event). It then claims the next eligible entry, runs the refine, build, and merge orchestrations, and repeats until `emery plan status` projects `drained` or a stop condition halts it (exit 2, `plan-execute-stopped`). It holds the create-exclusive `.emery/guest.lock` marker for the run's lifetime — a second driver session exits with `guest-marker-held`.

**Workspace routing is unsupported.** The loop runs single-project plans only: a workspace plan root (`workspace: true` in `project.yaml`) or any `project`-scoped plan entry refuses with `plan-execute-workspace-unsupported` (exit 2) before any adapter lookup or plan state is touched. Drive workspace plans hand-driven instead — `emery plan next`, then the `/emery:refine` → `/emery:build` → `/emery:merge` breakouts. The read-only `emery plan status` stays slot-aware and never refuses.

Stops render the `emery plan status` projection verbatim: the closed reason (`refine-failed`, `build-failed`, `merge-conflict`, `merge-postflight-failed`, `slice-dropped`, `merge-incomplete`, `stuck`), the failure detail from the journal, a one-line hint, and the literal resume command. Re-running `emery plan execute` after a refine / build / preflight-merge stop resumes from the same active entry. After `merge-postflight-failed`, the entry is already `done` (non-rollback); re-running execute acknowledges the sticky stop (`plan.merge-postflight.acknowledged`) and continues the queue — or drains when no pending entries remain.

### emery plan next

Report the next eligible plan entry.

```bash
emery plan next
```

Returns the first `pending` entry whose `depends-on` entries are all `done`. Returns an error if no eligible entry exists.

With `--format json`, when an eligible entry is found the response includes `project` (string or null), `description` (string or null), and `sources` (array or null) alongside `next`. These fields are absent when `reason` is non-null (`drained`, `stuck`, `in-progress`). `plan next` is the sole writer of per-entry `in-progress`; use `emery plan status` for a pure read.

### emery plan status

Project the plan's execution state into a deterministic `next-action`. Read-only — `plan next` stays the only writer of per-entry `in-progress`, and `status` emits no journal event.

```bash
emery plan status [--format json]
```

The projection reads three surfaces: `plan.yaml` entries, the candidate slice's `metadata.yaml` lifecycle (resolved into the materialised workspace slot when the entry is project-bound), and the journal tail. The `next-action` field resolves to one of:

| `next-action` | Meaning |
|---------------|---------|
| `refine <slice>` / `build <slice>` / `merge <slice>` | Dispatch the named phase for the candidate entry (the active `in-progress` entry, else the entry `plan next` would claim). |
| `stop <reason>` | Halt the loop; the `stop` sub-body carries the closed reason, optional journal `detail`, and a one-line operator hint. |
| `drained` | No `pending` or `in-progress` entries remain — text mode renders the literal `drained — run /emery:finalize <name>` string. |

Stop reasons are a closed set: `refine-failed` / `build-failed` / `merge-conflict` (the awaited phase's most recent journal terminal — `slice.synthesize.failed` / `slice.build.failed` / `slice.merge.failed` — is a failure, scoped to the active entry's claim window), `merge-postflight-failed` (the target's postflight gate failed after commit — entry is `done` and archived; sticky until `emery plan execute` acknowledges), `slice-dropped`, `merge-incomplete` (the merge landed but the entry's `done` stamp is missing — re-running `/emery:merge` or `emery plan execute` heals the torn stamp), and `stuck` (pending entries blocked on unmet dependencies). A `pending` plan projects the same next action as an approved one — Gate 1 is stamped by execute itself, not gated in status.

With `--format json` the body carries `plan`, `lifecycle`, `counts` (`pending` / `in-progress` / `done`), `active`, `next-action` (the rendered string), `action` (the closed verb), `slice`, `project`, the optional `stop` sub-body, and the re-entry fields: `current-step` / `last-completed` (the candidate slice's position in the `refine → build → merge` loop, `null` outside a dispatchable slice) and `resume` — the literal command or skill invocation that makes progress (`/emery:build a`, `/emery:merge a`, …), `null` when no single command does (`stuck`, `slice-dropped`). Text mode renders `resume:` as the final line.

### emery plan add

Append a new entry to the plan.

```bash
emery plan add <name> [--project <name>] [--description "<text>"] [--depends-on <entry>...] [--sources <key>...]
```

Creates the entry in `pending` state.

### emery plan amend

Edit non-status fields on an existing **entry** (one positional — the slice name; there is a single active `plan.yaml`). Use for divergence stamps, authority overrides, and surgical source/project/depends-on edits. For grouping changes prefer re-running `emery plan author --force` (wholesale replace of a pending plan); for deferral use `emery plan remove`.

```bash
emery plan amend <entry> [--project <name>] [--description "<text>"] [--depends-on <entry>...]
emery plan amend <entry> --add-source <key>=<lead>
emery plan amend <entry> --remove-source <key>
emery plan amend <entry> --divergence likely|accepted|rejected
emery plan amend <entry> --authority-override <entry> <kind>=<source>
```

Per-entry `pending` is written by `emery plan add` / `plan amend`; `in-progress` is written only by `emery plan next`. v1 has no per-entry `failed`, `blocked`, or `skipped` — build failures and merge conflicts leave the active entry `in-progress`.

A slice binds at most one lead per source key (a duplicate would silently overwrite `evidence/<source>.yaml` at refine time). `--add-source` refuses a key the entry already binds with `duplicate-source-key` (exit 2); a duplicate introduced via the wholesale `--sources` replacement rolls back as `plan-amend-validation-failed`. Re-sizing — replacing the lead bound under an existing key via `--sources <key>=<other-lead>` — stays legal.

### emery plan remove

Drop a pending plan entry while the plan is still replaceable (`lifecycle: pending` and every entry `pending`). Gate 1 only — defers the entry's lead(s) without re-surveying `discovery.md`.

```bash
emery plan remove <entry>
```

Refuses with `plan-remove-plan-not-replaceable` when the plan is approved or any entry is non-pending. Refuses with `plan-remove-entry-referenced` when another entry lists `<entry>` in `depends-on`.

### Lead reconciliation (inside `emery plan author`)

The reconcile leg inside the guest-routed `emery plan author` groups the surveyed `discovery.md` leads into the plan's `slices[]` rows.

- The **request** side is a flat catalog of raw `(source, lead)` leads read 1:1 from `discovery.md`, plus the project topology (always at least one project, each carrying its normalized `target` adapter).
- The **write** side is the **only slice writer**. It schema-validates the judgment response (`proposal-schema`), re-reads `discovery.md`, rebuilds the lead catalog, validates the agent's `slices[]` grouping, enforces total lead coverage, validates the explicit slice names, binds projects (auto-binding the sole project and deriving each slice's `target` from the bound project), atomically replaces `plan.yaml.slices[]`, then emits a single `plan.reconcile.completed` journal event. It never trusts a stale snapshot — `discovery.md` and the topology are re-read every invocation.

**Replaceable gate.** The write runs only while the plan is replaceable — `lifecycle: pending` and every entry `pending`; otherwise it fails with `plan-reconcile-plan-not-replaceable`. Re-authoring a still-pending plan wholesale-replaces every slice. Each slice's registry project is bound by the agent inside the response, not by a later assignment pass.

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
| `plan-reconcile-plan-not-replaceable` | The plan is approved or carries a non-pending entry. |

Both envelopes are owned by the typed wire DTOs in [`crates/project/src/plan/propose.rs`](../../../crates/project/src/plan/propose.rs) (closed `kind: request | response`); the response's judgment-answer schema is generated from them by `project::answers::proposal`. See [CLI output shapes](../cli-output-shapes.md) for the envelope bodies.

### emery plan transition

Walk a plan entry's status one rung backwards.

```bash
emery plan transition <entry> --undo
```

`--undo` is the only mode: it walks one rung backwards (`done → in-progress`, then `in-progress → pending` on a second call), firing one `plan.transition.undone` event per rung. Forward writes have no transition surface — `emery slice merge run` stamps `done` (and re-stamps it when healing a torn merge).

Per-entry `pending` is written by `emery plan add` / `plan amend`; `in-progress` is written only by `emery plan next`. v1 has no per-entry `failed`, `blocked`, or `skipped` — build failures and merge conflicts leave the active entry `in-progress`.

At most one entry may be `in-progress` at a time.

### emery plan archive

Archive a completed plan.

```bash
emery plan archive
```

Moves `plan.yaml` and `.emery/plans/<name>/` to `.emery/archive/plans/<YYYYMMDD>-<name>/`.

## See also

- [emery slice](slice.md) -- the per-slice CLI verbs the plan loop drives.
- [Change skills](../change-skills/index.md) -- `/emery:plan` and `/emery:finalize` wrappers
- [`/emery:plan` skill body](../../../plugins/emery/skills/plan/SKILL.md)
- [`/emery:finalize` skill body](../../../plugins/emery/skills/finalize/SKILL.md)
- [Configuration Files](../configuration.md) -- plan.yaml and registry format
