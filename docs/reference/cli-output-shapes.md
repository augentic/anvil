# CLI output shapes

Canonical JSON envelope shapes for `specify *` commands that skills shell out to. Skills should **link** to the relevant section here rather than embedding multi-line JSON examples in their `SKILL.md` body (see [docs/standards/skill-authoring.md "Skill body discipline"](../standards/skill-authoring.md#skill-body-discipline)).

## Conventions

- `--format json` responses are a **flat envelope**: every successful body is a single JSON object whose first key is `envelope-version` and whose remaining keys are the command-specific body fields **at the same level** — there is no `ok` discriminant and no `data` wrapper. Example: `{"envelope-version": 6, "action": "create", "plan": {...}, "entry": {...}}`.
- Failures keep the same flat shape with three extra top-level keys:
  - `error` — a **kebab-case discriminant string** (e.g. `"plan-has-outstanding-work"`). The discriminant is grep-stable and forms part of the public contract; see [`AGENTS.md`](../../AGENTS.md#exit-codes) for the catalogue.
  - `message` — humanised one-liner suitable for direct rendering.
  - `exit-code` — the integer the binary returns.
- Body fields named `ok` / `passed` / `idempotent` are payload fields, not envelope discriminants — they describe the per-command result and do not change the envelope shape.
- Paths are emitted as plain strings relative to the repo root unless the field name says otherwise (`absolute-path`, `tempdir-path`).
- All keys are `kebab-case`. The `envelope-version` integer bumps on any breaking change to a body shape; current version is `6`.

## Shapes

The examples below are hand-curated illustrations of the happy path for each command. For the full variant set — including failure envelopes, edge cases, and idempotent re-runs — browse the canonical fixtures in [`tests/fixtures/plan/`](../../tests/fixtures/plan) and [`tests/fixtures/e2e/goldens/`](../../tests/fixtures/e2e/goldens). When a command grows a new variant, copy the relevant fixture in here (trimmed if necessary) and add a sentence describing when the variant fires.

### `specify plan create`

Scaffolds an empty plan and emits its first entry.

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

### `specify plan amend`

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

### `specify plan next`

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

### Lead-reconciliation request envelope {#plan-reconcile-request}

The reconcile leg inside the guest-routed `specify plan author` assembles the lead-reconciliation **request** envelope for the agent to group: a flat `(source, lead)` lead catalog read 1:1 from `discovery.md`, plus the project topology (always at least one project, each carrying its normalized `target` adapter). Read-only — nothing is written and no journal event fires. `description` is omitted when the project carries none.

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

### `specify plan transition`

Used for both entry transitions (`kind: "entry"`) and the plan-level review stamp (`kind: "plan"`). The `previous` / `current` pair pins the legal transition rung that fired.

```json
{
  "current": "approved",
  "kind": "plan",
  "name": "demo",
  "plan": {
    "name": "demo",
    "path": "<TEMPDIR>/plan.yaml"
  },
  "previous": "pending"
}
```

### `specify plan status`

Read-only projection of the plan's execution state. `next-action` is the dispatch string (`refine|build|merge <slice>` / `stop <reason>` / `drained`) with `action` as its machine discriminant; `stop` is non-null only when `action` is `stop`, carrying the `stop-conditions.md` reason, optional journal detail, and operator hint. The re-entry fields ride the same body: `current-step` / `last-completed` name the slice's position in the `refine → build → merge` loop, and `resume` is the literal command (or skill invocation) that makes progress — `null` when no single command does (e.g. `stuck`, `slice-dropped`). The verb never writes: `plan next` stays the only `in-progress` writer.

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
  "resume": "/spec:refine a",
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
  "resume": "/spec:build a",
  "slice": "a",
  "stop": {
    "detail": "exhausted repair budget",
    "hint": "Fix the failure, then retry /spec:build for the slice. The plan entry stays in-progress.",
    "reason": "build-failed"
  }
}
```

### `specify plan validate`

Runs the plan-shape diagnostics and emits the neutral `DiagnosticReport` envelope (`{ version, summary, findings }`) shared with `specify slice validate` and `specify lint`. A clean plan carries an empty `findings` array and an all-zero `summary`; the exit code (`0`) signals pass, `2` signals a blocking finding.

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

### `specify plan archive`

Sweeps a closed plan into `.specify/archive/plans/`. The `archived` field is the destination path; `archived-plans-dir` is non-null when the plan had a per-plan authoring directory that also got swept. Errors use the standard envelope: `plan-has-outstanding-work` (exit 1) when the plan still has non-terminal entries.

```json
{
  "archived": "<TEMPDIR>/.specify/archive/plans/demo-<YYYYMMDD>.yaml",
  "archived-plans-dir": null,
  "plan": {
    "name": "demo"
  }
}
```

### `specify slice merge run`

Folds the slice's spec deltas into the baseline. `merged-specs[]` carries one entry per spec file touched, each listing the requirement-level operations applied (`added`, `modified`, `removed`).

```json
{
  "merged-specs": [
    {
      "baseline-path": "<TEMPDIR>/.specify/specs/login/spec.md",
      "name": "login",
      "operations": [
        {
          "id": "REQ-001",
          "kind": "added",
          "name": "User can log in"
        }
      ]
    }
  ]
}
```

### `specify slice task mark`

Marks one task complete. `idempotent: true` indicates the task was already complete and the call was a no-op; the `new-content-path` always points at the updated `tasks.md` regardless.

```json
{
  "idempotent": true,
  "marked": "1.1",
  "new-content-path": "<TEMPDIR>/.specify/slices/my-slice/tasks.md"
}
```

### `specify slice task progress`

Reads task counts and per-task state from a slice's `tasks.md`. `complete` / `pending` are the headline counts; `tasks[]` carries each parsed task with its parent `group`, `number` (`X.Y`), free-form `description`, and optional `skill-directive` (the embedded `<!-- skill: plugin:skill-name -->` reference, if any).

```json
{
  "complete": 2,
  "pending": 3,
  "tasks": [
    {
      "complete": true,
      "description": "Wire the crate into the workspace",
      "group": "1. Scaffold",
      "number": "1.2",
      "skill-directive": null
    }
  ],
  "total": 5
}
```

### Synthesis envelopes {#synthesis-envelopes}

The synthesis leg inside the guest-routed `specify slice refine` assembles the agent **inputs** envelope (`kind: inputs`): the slice name, one entry per bound source carrying its inline `lead` and verbatim `claims` (read from `evidence/<source>.yaml`), and the resolved target `shape-brief` body. Authority is deliberately absent — the kernel resolves it after the response. Read-only; emits a `slice.synthesize.agent` journal event.

```json
{
  "version": 1,
  "kind": "inputs",
  "slice": "identity-service",
  "sources": [
    {
      "source": "docs",
      "lead": "password-reset",
      "claims": [
        { "id": "password-reset.request", "kind": "requirement", "statement": "The system lets a registered user request a password reset link by email.", "path": "docs/identity/reset.md#L4" }
      ]
    },
    {
      "source": "legacy",
      "lead": "password-reset",
      "claims": [
        { "id": "password-reset.expiry", "kind": "example", "output": "expiresAt = createdAt + 24h", "path": "src/users/reset.ts#L88" }
      ]
    }
  ],
  "shape-brief": "# Shape brief\n…"
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

### `specify slice build`

Two envelope shapes inside the guest-routed orchestration. The **handoff** envelope is assembled after schema-validating the build request: `request` is the assembled `build/request.yaml` the adapter guest's `build` brief consumes, `report` is where the brief writes its `build/report.yaml`, and `briefs-dir` / `build-brief` locate the brief. The orchestration emits `target.execution.agent` before driving the judgment leg.

```json
{
  "slice": "identity-service",
  "target": "omnia@1.0.0",
  "execution": "agent",
  "request": "<TEMPDIR>/.specify/slices/identity-service/build/request.yaml",
  "report": "<TEMPDIR>/.specify/slices/identity-service/build/report.yaml",
  "briefs-dir": "<TEMPDIR>/adapters/targets/omnia/briefs",
  "build-brief": "<TEMPDIR>/adapters/targets/omnia/prose/briefs/build.md"
}
```

The finalize tail validates the report against `schemas/target/build-report.schema.json`, rejects a `success` report carrying any blocking finding, gates the `built` transition, and emits the **result** envelope (`slice.build.started` then `slice.build.succeeded` / `slice.build.failed`). `findings` is the count of report findings.

```json
{
  "slice": "identity-service",
  "target": "omnia@1.0.0",
  "status": "success",
  "findings": 0
}
```

### `specify slice validate`

Runs the slice-shape brief and cross-check predicates and renders a **`DiagnosticReport`** on stdout — the same neutral finding currency every check surface emits (`specify lint`, `specify lint framework`, `slice validate`). The report shape is identical for clean and failed runs; what changes is the `findings[]` content and the `summary` counts.

Each finding carries a `rule-id` (dotted/kebab invariant id such as `design.references-valid-ids` or `slice-model-source-orphan`), a `severity` (`critical | important | optional | suggestion`), a `source` (`deterministic | model-assisted | hybrid | human | tool`), and a `kind`:

- `kind: "violation"` — a structural defect. Open `critical`/`important` violations block the lifecycle gate (exit 2).
- `kind: "review"` — a deterministically-raised request for agent/human judgment. Surfaced but never blocking; the refine agent reads its worklist as `findings.filter(kind == "review")`.

`summary` carries per-severity counts. A clean run emits no `violation` findings; semantic checks still appear as `review` findings:

```json
{
  "findings": [
    {
      "artifact": "proposal",
      "confidence": "medium",
      "evidence": {
        "kind": "snippet",
        "value": "Semantic check — requires agent judgment"
      },
      "fingerprint": "sha256:…",
      "id": "DIAG-0001",
      "impact": "Semantic check — requires agent judgment",
      "kind": "review",
      "location": { "path": "proposal.md" },
      "remediation": "Uses imperative language for motivation",
      "rule-id": "proposal.uses-imperative-language",
      "severity": "suggestion",
      "source": "model-assisted",
      "title": "Uses imperative language for motivation"
    }
  ],
  "synopsis": { "critical": 0, "important": 0, "optional": 0, "suggestion": 1 },
  "version": 1
}
```

A failed run carries one `kind: "violation"` finding per breached invariant (e.g. `rule-id: "slice-model-source-orphan"`, `severity: "important"`) with `impact`/`remediation` describing the defect, the `summary` counts rise accordingly, and the process exits 2. The exit carries a payload-free error envelope on **stderr** whose `error` is the gate discriminant (e.g. `slice-pre-adapter-gate`); the rich per-finding detail lives only on the stdout report. See [DECISIONS.md §"Drained `Error::Validation` and the `Diagnostic` substrate"](../../DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate).

## Bootstrap verbs

The bootstrap lifecycle verbs (`specify upgrade`, `specify plugins doctor`) emit a self-describing body whose first key is a `version` integer **schema marker** (`1` today), rather than the `envelope-version` stamp the project/slice verbs carry. All keys stay `kebab-case`. The `/spec:init` runbook parses these shapes; skills link here rather than inlining them.

### `specify upgrade --dry-run`

Reports the detected channel and resolved target version without mutating. `commands` lists the channel-native commands that *would* run (empty for the `binary` channel, which instead carries `guidance`). `head-fallback` is `true` when the latest release tag could not be resolved and the `cargo` channel falls back to a HEAD install. On the apply path (`--yes`) `dry-run`/`applied` flip and `journaled` reports whether a `cli.upgraded` event was written.

```json
{
  "version": 1,
  "channel": "cargo",
  "from": "0.3.0",
  "to": "0.43.0",
  "dry-run": true,
  "applied": false,
  "head-fallback": false,
  "journaled": false,
  "commands": [
    { "program": "cargo", "args": ["install", "--git", "https://github.com/augentic/specify", "--tag", "v0.43.0"] }
  ]
}
```

The `binary` channel omits `commands` and carries `guidance` instead, because its in-process self-replace is deferred:

```json
{
  "version": 1,
  "channel": "binary",
  "from": "0.3.0",
  "to": "0.43.0",
  "dry-run": true,
  "applied": false,
  "head-fallback": false,
  "journaled": false,
  "commands": [],
  "guidance": "binary-channel self-replace is deferred; download the latest release manually"
}
```

### `specify plugins doctor`

Read-only Cursor plugin-cache drift report. One `plugins[]` row per declared plugin (then any `extra` cache entries), each with the marketplace-resolved `expected-sha` (`null` when unresolvable), the `cached-sha` (`null` when no cache entry), and a `status` from the closed set `ok | drifted | present | missing | extra`. Drift is a **finding**, never a non-zero exit — `doctor` exits non-zero only on filesystem / marketplace-parse failure.

```json
{
  "version": 1,
  "marketplace": "/abs/path/specify/.cursor-plugin/marketplace.json",
  "cache-root": "/Users/me/.cursor/plugins/cache/augentic",
  "plugins": [
    { "name": "spec", "expected-sha": "f1b21b2…", "cached-sha": "a0c4d1e…", "status": "drifted" },
    { "name": "capture", "expected-sha": "f1b21b2…", "cached-sha": "f1b21b2…", "status": "ok" }
  ],
  "summary": { "ok": 1, "drifted": 1, "present": 0, "missing": 0, "extra": 0 }
}
```

### `specify init --upgrade`

Re-entry version bump. It shares the `specify init` body; the field that distinguishes the re-entry outcome is `specify-version-changed` — `true` when this run rewrote `project.yaml.specify` (a fresh init, or an `--upgrade` that bumped an older pin) and `false` on an `--upgrade` no-op where the pin already matched. The re-entry template reads it to render "upgraded" vs "already current".

```json
{
  "config-path": "/abs/path/.specify/project.yaml",
  "adapter-name": "omnia",
  "specify-version": "2.0.0",
  "specify-version-changed": true,
  "cache-present": true,
  "context-generated": false,
  "context-skipped": true
}
```

---

When migrating a new dispatcher to `Render`, append its body under a stable H3 heading and link from the corresponding `SKILL.md` instead of inlining the JSON example. Trim large example bodies to the smallest shape that illustrates the contract — readers who want byte-for-byte canonical output should follow the fixture link above to the CLI repo.
