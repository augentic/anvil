# specify plan

Scaffold, populate, validate, transition, and archive change plans. The `plan` verb is the top-level home of every `plan.yaml` operation; each verb on this page is invoked as `specify plan <verb>`.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`create`](#specify-plan-create) | Scaffold an empty `plan.yaml` at the repo root. Refuses to overwrite an existing plan. |
| `author` | Guest-routed authoring orchestration: scaffold, survey every bound source, reconcile leads into `slices[]`, validate, exit at `pending` with the Gate 1 hint. Invoked by `/spec:plan`. |
| `execute` | Guest-routed driver loop over an approved plan: claim → refine → build → merge per entry until `drained` or a stop (exit 2, `plan-execute-stopped`). Holds the `.specify/guest.lock` marker. |
| [`add`](#specify-plan-add) | Append a new entry to the plan in `pending` state. |
| [`amend`](#specify-plan-amend) | Edit non-status fields (`project`, `description`, `depends-on`, `sources`) on an existing entry. |
| [`remove`](#specify-plan-remove) | Drop a pending entry while the plan is still replaceable (Gate 1 deferral). |
| [reconciliation](#lead-reconciliation-inside-specify-plan-author) | The reconcile leg inside `specify plan author`: validates the agent grouping and replaces `slices[]` on a replaceable plan. |
| [`transition`](#specify-plan-transition) | Stamp Gate 1 (`specify plan transition <plan-name> approved`) or close a merged entry (`specify plan transition <entry-name> done`). Per-entry status is `pending | in-progress | done` only. |
| [`validate`](#specify-plan-validate) | Structural and referential integrity check (cycles, unknown deps, multi-repo invariants) plus three health diagnostics (`cycle-in-depends-on`, `orphan-source`, `stale-workspace-clone`). First triage step when `specify plan execute` reports `stuck`. |
| [`next`](#specify-plan-next) | Report the next eligible entry (used by the execute loop and ad-hoc operators). |
| [`status`](#specify-plan-status) | Read-only projection of the plan's execution state into a deterministic `next-action` (`refine|build|merge <slice>` / `stop <reason>` / `drained`). |
| [`archive`](#specify-plan-archive) | Move a completed `plan.yaml` and `.specify/plans/<name>/` to `.specify/archive/plans/`. Usually invoked by `/spec:finalize` after it observes merged PRs. |

## Subcommands

### specify plan create

Scaffold an empty plan.

```bash
specify plan create <name> [--source <key>=<adapter>:<path>...] [--source <key>=<adapter>:value:<literal>...]
```

Writes `plan.yaml` at the repo root with the given kebab-case name and an empty `slices:` list. Each optional `--source` carries the structured binding shape: an explicit kebab-case `<adapter>` followed by a colon and either a path (`<adapter>:<path>` — URLs containing `:` like `git@github.com:org/foo.git` round-trip cleanly because only the first colon is significant) or a `value:`-prefixed literal (`<adapter>:value:<literal>` — used by `intent`). Refuses with `already-exists` when `plan.yaml` is already present.

### specify plan validate

Check structural and referential integrity of the plan, plus the four
health diagnostics below.

```bash
specify plan validate
```

Base shape checks: duplicate entry names, dependency cycles, unknown `depends-on` / `sources` references, duplicate source keys within one entry (`duplicate-source-key` — a slice binds at most one lead per source), at most one `in-progress` entry, and the following cross-registry checks when `registry.yaml` is present:

- `project-not-in-registry` (important) -- every `project` value must match a `projects[].name` in the registry.
- `project-missing-multi-repo` (important) -- when the registry has multiple projects, every change must carry a `project` field.
- `topology-cache-stale` (suggestion) -- a workspace slot's `project.yaml` (target adapter, description) or its baseline projection (`surface[]` / `recent[]`) has diverged from the committed `.specify/topology.lock`. The project's `project.yaml` plus its baseline are authoritative; the fix is `specify workspace sync` to regenerate the cache.

Health diagnostics layered on top — first triage step when `specify plan execute` reports `stuck`:

| Code | Severity | Meaning | Recovery |
|------|----------|---------|----------|
| `cycle-in-depends-on` | important | Dependency cycle in `depends-on`. `next_eligible` silently skips cycles at runtime; validate is the only place where the cycle structure surfaces. Structured evidence carries the cycle path, e.g. `["a", "b", "a"]`. | `specify plan amend <entry> --depends-on …` to break the cycle, then re-run validate. |
| `orphan-source` | suggestion | Top-level `sources:` key declared but no plan entry references it (the inverse of `unknown-source`). | Either reference the key from an entry's `sources:` list or remove the declaration. |
| `stale-workspace-clone` | suggestion | Workspace clone's signature has drifted from the registry, or no signature is readable at all. Reason is one of `signature-changed` (URL or adapter diverged) or `slot-mismatch` (slot materialisation does not match the registry). | `specify workspace sync` to refresh the clone. |

JSON output (`--format json`) is the neutral `DiagnosticReport` envelope (`{ version, summary, findings }`) shared with `specify slice validate` and `specify lint` — see [`schemas/diagnostics/diagnostic-report.schema.json`](../../../schemas/diagnostics/diagnostic-report.schema.json). Each finding carries `rule-id` (kebab-case, e.g. `duplicate-name` / `cycle-in-depends-on`), `severity` (`critical` / `important` / `suggestion` / `optional`), `impact` (the human-readable message), optional `slice` (the entry name), and `evidence`. The three health diagnostics attach their machine-readable payload to `evidence` as `{ "kind": "structured", "data": … }`; base validate findings carry a plain `snippet` evidence.

Exit code: `0` when no blocking finding fires (suggestions are non-fatal); `2` when any blocking (`critical` / `important`) finding fires.

### specify plan next

Report the next eligible plan entry.

```bash
specify plan next
```

Returns the first `pending` entry whose `depends-on` entries are all `done`. Returns an error if no eligible entry exists.

With `--format json`, when an eligible entry is found the response includes `project` (string or null), `description` (string or null), and `sources` (array or null) alongside `next`. These fields are absent when `reason` is non-null (`all-done`, `stuck`, `in-progress`). `plan next` is the sole writer of per-entry `in-progress`; use `specify plan status` for a pure read.

### specify plan status

Project the plan's execution state into a deterministic `next-action`. Read-only — `plan next` stays the only writer of per-entry `in-progress`, and `status` emits no journal event.

```bash
specify plan status [--format json]
```

The projection reads three surfaces: `plan.yaml` entries, the candidate slice's `metadata.yaml` lifecycle (resolved into the materialised workspace slot when the entry is project-bound), and the journal tail. The `next-action` field resolves to one of:

| `next-action` | Meaning |
|---------------|---------|
| `refine <slice>` / `build <slice>` / `merge <slice>` | Dispatch the named phase for the candidate entry (the active `in-progress` entry, else the entry `plan next` would claim). |
| `stop <reason>` | Halt the loop; the `stop` sub-body carries the closed reason, optional journal `detail`, and a one-line operator hint. |
| `drained` | No `pending` or `in-progress` entries remain — text mode renders the literal `drained — run /spec:finalize <name>` string. |

Stop reasons are a closed set: `plan-not-approved` (Gate 1 not stamped), `refine-failed` / `build-failed` / `merge-conflict` (the awaited phase's most recent journal terminal — `slice.synthesize.failed` / `slice.build.failed` / `slice.merge.failed` — is a failure, scoped to the active entry's claim window), `slice-dropped`, `merge-incomplete` (the merge landed but the entry's `done` stamp is missing), and `stuck` (pending entries blocked on unmet dependencies).

With `--format json` the body carries `plan`, `lifecycle`, `counts` (`pending` / `in-progress` / `done`), `active`, `next-action` (the rendered string), `action` (the closed verb), `slice`, `project`, the optional `stop` sub-body, and the re-entry fields: `current-step` / `last-completed` (the candidate slice's position in the `refine → build → merge` loop, `null` outside a dispatchable slice) and `resume` — the literal command or skill invocation that makes progress (`/spec:build a`, `specify plan transition demo approved`, …), `null` when no single command does (`stuck`, `slice-dropped`). Text mode renders `resume:` as the final line.

### specify plan add

Append a new entry to the plan.

```bash
specify plan add <name> [--project <name>] [--description "<text>"] [--depends-on <entry>...] [--sources <key>...]
```

Creates the entry in `pending` state.

### specify plan amend

Edit non-status fields on an existing **entry** (one positional — the slice name; there is a single active `plan.yaml`). Use for divergence stamps, authority overrides, and surgical source/project/depends-on edits. For grouping changes prefer re-running `specify plan author` (wholesale re-reconcile); for deferral use `specify plan remove`.

```bash
specify plan amend <entry> [--project <name>] [--description "<text>"] [--depends-on <entry>...]
specify plan amend <entry> --add-source <key>=<lead>
specify plan amend <entry> --remove-source <key>
specify plan amend <entry> --divergence likely|accepted|rejected
specify plan amend <entry> --authority-override <entry> <kind>=<source>
```

Per-entry `pending` is written by `specify plan add` / `plan amend`; `in-progress` is written only by `specify plan next`. v1 has no per-entry `failed`, `blocked`, or `skipped` — build failures and merge conflicts leave the active entry `in-progress`.

A slice binds at most one lead per source key (a duplicate would silently overwrite `evidence/<source>.yaml` at refine time). `--add-source` refuses a key the entry already binds with `duplicate-source-key` (exit 2); a duplicate introduced via the wholesale `--sources` replacement rolls back as `plan-amend-validation-failed`. Re-sizing — replacing the lead bound under an existing key via `--sources <key>=<other-lead>` — stays legal.

### specify plan remove

Drop a pending plan entry while the plan is still replaceable (`lifecycle: pending` and every entry `pending`). Gate 1 only — defers the entry's lead(s) without re-surveying `discovery.md`.

```bash
specify plan remove <entry>
```

Refuses with `plan-remove-plan-not-replaceable` when the plan is approved or any entry is non-pending. Refuses with `plan-remove-entry-referenced` when another entry lists `<entry>` in `depends-on`.

### Lead reconciliation (inside `specify plan author`)

The reconcile leg inside the guest-routed `specify plan author` groups the surveyed `discovery.md` leads into the plan's `slices[]` rows (the standalone two-phase envelope verb retired at the Omnia-migration cutover).

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

Both envelopes validate against [`schemas/discovery/proposal.schema.json`](../../../schemas/discovery/proposal.schema.json) (closed `kind: request | response`). See [CLI output shapes](../cli-output-shapes.md) for the envelope bodies.

### specify plan transition

Stamp Gate 1 or close a merged plan entry.

```bash
specify plan transition <name> <target> [--reason "<text>"]
```

| Target | Applies to | Meaning |
|--------|------------|---------|
| `approved` | `<plan-name>` (matches `plan.yaml` `name`) | Gate 1 — operator-only stamp after `/spec:plan`. |
| `done` | `<entry-name>` (a `slices[]` row) | Close the entry after `/spec:merge` folded the slice. |

Per-entry `pending` is written by `specify plan add` / `plan amend`; `in-progress` is written only by `specify plan next`. v1 has no per-entry `failed`, `blocked`, or `skipped` — build failures and merge conflicts leave the active entry `in-progress`.

At most one entry may be `in-progress` at a time.

### specify plan archive

Archive a completed plan.

```bash
specify plan archive
```

Moves `plan.yaml` and `.specify/plans/<name>/` to `.specify/archive/plans/<YYYYMMDD>-<name>/`.

## See also

- [specify slice](slice.md) -- the per-slice CLI verbs the plan loop drives.
- [/spec:plan](../change-skills/plan.md) -- skill that authors plans
- [specify plan execute](../change-skills/execute.md) -- the guest-routed driver loop
- [/spec:finalize](../change-skills/finalize.md) -- skill that closes out a completed change
- [Configuration Files](../configuration.md) -- plan.yaml and registry format
