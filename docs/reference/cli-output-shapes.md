# CLI output shapes

Canonical JSON envelope shapes for `emery *` commands that skills shell out to. Skills should **link** to the relevant section here rather than embedding multi-line JSON examples in their `SKILL.md` body.

## Conventions

- `--format json` responses are a **flat body**: every successful body is a single JSON object carrying the command-specific fields **at the top level** — there is no `ok` discriminant, no `data` wrapper, and no top-level envelope-version stamp. Example: `{"action": "create", "plan": {...}, "entry": {...}}`. (Judgment wire envelopes such as the reconciliation request carry their own in-body `version` field; that is payload, not a transport stamp.)
- Failures keep the same flat shape with three extra top-level keys:
  - `error` — a **kebab-case discriminant string** (e.g. `"plan-has-outstanding-work"`). The discriminant is grep-stable and forms part of the public contract; see [`AGENTS.md`](../../AGENTS.md#exit-codes) for the catalogue.
  - `message` — humanised one-liner suitable for direct rendering.
  - `exit-code` — the integer the binary returns.
- Body fields named `ok` / `passed` / `idempotent` are payload fields, not envelope discriminants — they describe the per-command result and do not change the envelope shape.
- Paths are emitted as plain strings relative to the repo root unless the field name says otherwise (`absolute-path`, `tempdir-path`).
- All keys are `kebab-case`. Body shapes are pinned by the typed `*Body` DTOs in the CLI workspace and change only with the CLI's own versioning.
- Stream roles: the semantic result body (text or JSON) is **stdout**; the failure `ErrorBody` and live host tracing are **stderr**. Tracing verbosity is selected by the reserved host log flags (`--debug` / `--quiet`, peeled before the guest sees argv; see [cli-contract.md](../standards/cli-contract.md)).

## Shapes

The examples below are hand-curated illustrations of the happy path for each command; the accept/reject variant set is exercised by the integration suites under `crates/*/tests/`. When a command grows a new variant, copy the relevant output in here (trimmed if necessary) and add a sentence describing when the variant fires.

### `emery plan add`

Appends one entry to an existing plan.

```json
{
  "action": "create",
  "entry": {
    "depends-on": [],
    "description": null,
    "name": "foo",
    "project": null,
    "sources": [],
    "status": "pending",
    "target": "contracts@1.0.0"
  },
  "plan": {
    "name": "demo",
    "path": "<TEMPDIR>/plan.yaml"
  }
}
```

### `emery plan amend`

Replaces a field on an existing plan entry. The `entry` body mirrors the post-amend state; absent fields surface as `null` or `[]` so consumers can rely on the shape regardless of which field was touched.

```json
{
  "action": "amend",
  "entry": {
    "depends-on": ["a", "b"],
    "description": null,
    "name": "foo",
    "project": "default",
    "sources": [],
    "status": "pending"
  },
  "plan": {
    "name": "demo",
    "path": "<TEMPDIR>/plan.yaml"
  }
}
```

### `emery plan next`

Returns the next entry the executor should pick up, or a `reason` describing why nothing is eligible. Success carries `next: "<entry>"`; drained / blocked / in-progress states carry `next: null` and a populated `reason` (`drained`, `in-progress`, etc.).

```json
{
  "active": null,
  "description": null,
  "next": "b",
  "project": "default",
  "reason": null,
  "sources": [],
  "target": null
}
```

### `emery plan author`

The guest-routed authoring orchestration: survey per bound source, reconcile leads into `slices[]`, persist the Gate 1 prose, validate, exit at `lifecycle: pending`. `hint` is the literal Gate 1 closing line the `/emery:plan` skill relays.

```json
{
  "plan": "identity-revamp",
  "lifecycle": "pending",
  "surveyed": [
    { "source": "docs", "adapter": "documentation", "leads": ["identity-api"] }
  ],
  "slices": ["identity-contracts", "identity-service"],
  "hint": "Plan `identity-revamp` is at `pending`. Review it, then run `emery plan execute` to approve and drive the slices (executing is the Gate 1 stamp)."
}
```

### `emery plan execute`

The drained loop's success body — a stop surfaces on the error envelope instead (`error: "plan-execute-stopped"`, exit 2), so a driver tells a parked loop from a drained one without parsing prose. `gate1-stamped` is `true` only when this invocation performed the Gate 1 stamp (`pending → approved`); re-entry on an already-approved plan reports `false`. `phases[]` lists the phases this run completed, in order. Text mode prints `approved: <plan> (actor: <actor>)` when this run stamped, the phase lines, and closes with the canonical `drained — run /emery:finalize <plan>` line.

```json
{
  "status": "drained",
  "plan": "identity-revamp",
  "gate1-stamped": true,
  "phases": [
    { "slice": "identity-contracts", "step": "refine" },
    { "slice": "identity-contracts", "step": "build" },
    { "slice": "identity-contracts", "step": "merge" }
  ]
}
```

On `plan-execute-stopped`, the `/emery:execute` skill follows up with a quiet `emery plan status` to render the canonical stop card (`stop: <reason>` / `hint:` / `resume:`).

### Lead-reconciliation request envelope {#plan-reconcile-request}

The reconcile leg inside the guest-routed `emery plan author` assembles the lead-reconciliation **request** envelope for the agent to group: a flat `(source, lead)` lead catalog read 1:1 from `discovery.md`, plus the project topology (always at least one project, each carrying its normalized `target` adapter). Read-only — nothing is written and no journal event fires. `description` is omitted when the project carries none.

```json
{
  "version": 1,
  "kind": "request",
  "projects": [
    { "name": "identity-contracts", "target": "contracts@1.0.0", "description": "Versioned API contracts crate for the identity domain." },
    { "name": "identity-service", "target": "omnia@1.0.0", "description": "Omnia identity service implementing auth and password flows." }
  ],
  "leads": [
    { "source": "docs", "lead": "identity-api", "synopsis": "Identity API contract for authentication and account access." },
    { "source": "legacy", "lead": "identity-api", "synopsis": "Legacy identity endpoints." }
  ]
}
```

### Lead-reconciliation write summary {#plan-reconcile-write}

Success summary after the reconcile kernel projects the agent **response** onto `plan.yaml.slices[]`. `slice-names` is the slice set in response order and `slice-count` is its length.

```json
{
  "plan": { "name": "identity-revamp", "path": "/abs/path/to/plan.yaml" },
  "slice-names": ["identity-contracts", "identity-service", "password-reset"],
  "slice-count": 3
}
```

### `emery plan transition`

The one-rung reverse walk (`--undo` is the only mode). The `previous` / `current` pair pins the rung that fired, and the `undo: { from, to }` pair carries the same move as typed discriminants.

```json
{
  "current": "in-progress",
  "name": "identity-service",
  "plan": {
    "name": "demo",
    "path": "<TEMPDIR>/plan.yaml"
  },
  "previous": "done",
  "undo": {
    "from": "done",
    "to": "in-progress"
  }
}
```

### `emery plan status`

Read-only projection of the plan's execution state. `next-action` is the dispatch string (`refine|build|merge <slice>` / `stop <reason>` / `drained`) with `action` as its machine discriminant; `stop` is non-null only when `action` is `stop`, carrying the closed stop-reason discriminant, optional journal detail, and operator hint. The re-entry fields ride the same body: `current-step` / `last-completed` name the slice's position in the `refine → build → merge` loop, and `resume` is the literal command (or skill invocation) that makes progress — `null` when no single command does (e.g. `stuck`, `slice-dropped`). On a `pending` plan the dispatch projections keep their `next-action` but `resume` names `/emery:execute` — the operator path runs through Gate 1, not a phase breakout. The verb never writes: `plan next` stays the only `in-progress` writer.

```json
{
  "action": "refine",
  "active": "a",
  "counts": {
    "done": 0,
    "in-progress": 1,
    "pending": 0
  },
  "current-step": "refine",
  "last-completed": null,
  "lifecycle": "approved",
  "next-action": "refine a",
  "plan": "demo",
  "project": "default",
  "resume": "/emery:refine a",
  "slice": "a"
}
```

A stopped plan carries the classification block:

```json
{
  "action": "stop",
  "active": "a",
  "counts": {
    "done": 0,
    "in-progress": 1,
    "pending": 0
  },
  "current-step": "build",
  "last-completed": "refine",
  "lifecycle": "approved",
  "next-action": "stop build-failed",
  "plan": "demo",
  "project": "default",
  "resume": "/emery:build a",
  "slice": "a",
  "stop": {
    "detail": "exhausted repair budget",
    "hint": "Fix the failure, then retry /emery:build for the slice. The plan entry stays in-progress.",
    "reason": "build-failed"
  }
}
```

### `emery plan validate`

Runs the plan-shape diagnostics and emits the neutral `DiagnosticReport` envelope (`{ version, summary, findings }`) shared with `emery slice validate`. A clean plan carries an empty `findings` array and an all-zero `summary`; the exit code (`0`) signals pass, `2` signals a blocking finding.

```json
{
  "findings": [],
  "summary": {
    "critical": 0,
    "important": 0,
    "optional": 0,
    "suggestion": 0
  },
  "version": 1
}
```

A failed run carries one object per finding in `findings`, each with `rule-id` (kebab-case rule id such as `duplicate-name` or `cycle-in-depends-on`), `severity` (`critical` / `important` / `suggestion` / `optional`), `impact` (the human-readable message), optional `slice` (the entry name), and `evidence`. Health diagnostics (`cycle-in-depends-on`, `orphan-source`, `stale-workspace-clone`) attach their structured payload to `evidence` as `{ "kind": "structured", "data": … }`.

### `emery plan archive`

Sweeps a closed plan into `.emery/archive/plans/`. The `archived` field is the destination path; `archived-plans-dir` is non-null when the plan had a per-plan authoring directory that also got swept. Errors use the standard envelope: `plan-has-outstanding-work` (exit 1) when the plan still has non-terminal entries.

```json
{
  "archived": "<TEMPDIR>/.emery/archive/plans/demo-<YYYYMMDD>.yaml",
  "archived-plans-dir": null,
  "plan": {
    "name": "demo"
  }
}
```

### `emery slice merge run`

Folds the slice's spec deltas into the baseline. The committed-merge body carries the merged baseline spec names, the promoted `DEC-NNNN` Decision Record ids, and the archived slice location; the `--preview` / `--conflict-check` dry-run modes keep their own bodies.

```json
{
  "slice": "login",
  "merged": ["login"],
  "decisions": ["DEC-0001"],
  "archive-path": "<TEMPDIR>/.emery/archive/2026-07-31-login"
}
```

### Synthesis envelopes {#synthesis-envelopes}

The synthesis leg inside the guest-routed `emery slice refine` assembles the agent **inputs** envelope (`kind: inputs`): the slice name, one entry per bound source carrying its `lead` and the project-relative `evidence-path` to its `evidence/<source>.yaml` (the agent reads the claims from the lent tree — they are not inlined on the wire), and the resolved target guidance body (wire field `guidance-brief`). Authority is deliberately absent — the kernel resolves it after the response. Read-only; emits a `slice.synthesize.agent` journal event.

```json
{
  "version": 2,
  "kind": "inputs",
  "slice": "identity-service",
  "sources": [
    {
      "source": "docs",
      "lead": "password-reset",
      "evidence-path": ".emery/slices/identity-service/evidence/docs.yaml"
    },
    {
      "source": "legacy",
      "lead": "password-reset",
      "evidence-path": ".emery/slices/identity-service/evidence/legacy.yaml"
    }
  ],
  "guidance-brief": "# Guidance brief\n…"
}
```

### Synthesis persist summary

Success summary after the projection kernel persisted the artifacts. `artifacts[]` lists the slice-relative paths written, in write order. Emits `slice.synthesize.started` then `slice.synthesize.completed`; on any failure it emits `slice.synthesize.failed`, leaves the prior artifacts intact, and exits non-zero.

```json
{
  "slice": "identity-service",
  "artifacts": [
    "proposal.md",
    "specs/password-reset/spec.md",
    "design.md",
    "tasks.md",
    "model.yaml"
  ]
}
```

### `emery slice build`

One envelope shape inside the guest-routed orchestration: the typed request is assembled, written to `build/request.yaml` for the adapter guest's `build` prompt to consume, and `target.execution.agent` fires before the judgment leg. The finalize tail gates the typed report, rejects a `success` report carrying any blocking finding, gates the `built` transition, and emits the **result** envelope (`slice.build.started` then `slice.build.succeeded` / `slice.build.failed`). `findings` is the count of report findings.

```json
{
  "slice": "identity-service",
  "target": "omnia@1.0.0",
  "status": "success",
  "findings": 0
}
```

### `emery slice validate`

Runs the slice-shape and cross-check predicates and renders a **`DiagnosticReport`** on stdout — the same neutral finding currency every check surface emits (`plan validate`, `slice validate`, build reports). The report shape is identical for clean and failed runs; what changes is the `findings[]` content and the `summary` counts.

Each finding carries a `rule-id` (dotted/kebab invariant id such as `design.references-valid-ids` or `slice-model-source-orphan`), a `severity` (`critical | important | optional | suggestion`), a `source` (`deterministic | model-assisted | hybrid | human | tool`), and a `kind`:

- `kind: "violation"` — a structural defect. Open `critical`/`important` violations block the lifecycle gate (exit 2).
- `kind: "review"` — a deterministically-raised request for agent/human judgment. Surfaced but never blocking; the refine agent reads its worklist as `findings.filter(kind == "review")`.

`summary` carries per-severity counts. A clean run emits an empty `findings[]` and zero counts:

```json
{
  "findings": [],
  "synopsis": { "critical": 0, "important": 0, "optional": 0, "suggestion": 0 },
  "version": 1
}
```

Non-blocking `review` findings can still appear from pre-adapter advisories (e.g. `discovery-lead-synopsis-thin`) when a thin lead synopsis is present — those ride the same report shape but never block the gate.
A failed run carries one `kind: "violation"` finding per breached invariant (e.g. `rule-id: "slice-model-source-orphan"`, `severity: "important"`) with `impact`/`remediation` describing the defect, the `summary` counts rise accordingly, and the process exits 2. The exit carries a payload-free error envelope on **stderr** whose `error` is the gate discriminant (e.g. `slice-pre-adapter-gate`); the rich per-finding detail lives only on the stdout report.

### `emery init --upgrade`

Re-entry version bump. It shares the `emery init` body; the field that distinguishes the re-entry outcome is `emery-version-changed` — `true` when this run rewrote `project.yaml.emery` (a fresh init, or an `--upgrade` that bumped an older pin) and `false` on an `--upgrade` no-op where the pin already matched. The re-entry template reads it to render "upgraded" vs "already current".

```json
{
  "config-path": "/abs/path/.emery/project.yaml",
  "adapter-name": "omnia",
  "emery-version": "2.0.0",
  "emery-version-changed": true,
  "cache-present": true,
  "context-generated": false,
  "context-skipped": true
}
```

---

When migrating a new dispatcher to `Render`, append its body under a stable H3 heading and link from the corresponding `SKILL.md` instead of inlining the JSON example. Trim large example bodies to the smallest shape that illustrates the contract — readers who want byte-for-byte canonical output should follow the mock link above to the CLI repo.
