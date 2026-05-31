# CLI output shapes

Canonical JSON envelope shapes for `specify *` commands that skills shell out to. Skills should **link** to the relevant section here rather than embedding multi-line JSON examples in their `SKILL.md` body (see [docs/standards/skill-authoring.md "Skill body discipline"](../standards/skill-authoring.md#skill-body-discipline)).

## Conventions

- `--format json` responses are a **flat envelope**: every successful body is a single JSON object whose first key is `envelope-version` and whose remaining keys are the command-specific body fields **at the same level** — there is no `ok` discriminant and no `data` wrapper. Example: `{"envelope-version": 6, "action": "create", "plan": {...}, "entry": {...}}`.
- Failures keep the same flat shape with three extra top-level keys:
  - `error` — a **kebab-case discriminant string** (e.g. `"plan-has-outstanding-work"`). The discriminant is grep-stable and forms part of the public contract; see [`AGENTS.md` in `augentic/specify-cli`](https://github.com/augentic/specify-cli/blob/main/AGENTS.md#error-handling-and-exit-codes) for the catalogue.
  - `message` — humanised one-liner suitable for direct rendering.
  - `exit-code` — the integer the binary returns.
- Body fields named `ok` / `passed` / `idempotent` are payload fields, not envelope discriminants — they describe the per-command result and do not change the envelope shape.
- Paths are emitted as plain strings relative to the repo root unless the field name says otherwise (`absolute-path`, `tempdir-path`).
- All keys are `kebab-case`. The `envelope-version` integer bumps on any breaking change to a body shape; current version is `6`.

## Shapes

The examples below are hand-curated illustrations of the happy path for each command. For the full variant set — including failure envelopes, edge cases, and idempotent re-runs — browse the canonical fixtures in [`augentic/specify-cli/tests/fixtures/plan/`](https://github.com/augentic/specify-cli/tree/main/tests/fixtures/plan) and [`augentic/specify-cli/tests/fixtures/e2e/goldens/`](https://github.com/augentic/specify-cli/tree/main/tests/fixtures/e2e/goldens). When a command grows a new variant, copy the relevant fixture in here (trimmed if necessary) and add a sentence describing when the variant fires.

### `specrun plan create`

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
    "target": "contracts@v1"
  },
  "plan": {
    "name": "demo",
    "path": "<TEMPDIR>/plan.yaml"
  }
}
```

### `specrun plan amend`

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

### `specrun plan next`

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

### `specrun plan propose --dry-run`

Emits the lead-reconciliation **request** envelope for the agent to group: a flat `(source, lead)` lead catalog read 1:1 from `discovery.md`, plus the project topology (always at least one project, each carrying its normalized `target` adapter). Read-only — nothing is written and no journal event fires. `description` is omitted when the project carries none; per-lead `aliases` appears only when non-empty.

```json
{
  "version": 1,
  "kind": "request",
  "projects": [
    { "name": "identity-contracts", "target": "contracts@v1", "description": "Versioned API contracts crate for the identity domain." },
    { "name": "identity-service", "target": "omnia@v1", "description": "Omnia identity service implementing auth and password flows." }
  ],
  "leads": [
    { "source": "docs", "lead": "identity-api", "synopsis": "Identity API contract for authentication and account access." },
    { "source": "legacy", "lead": "identity-api", "synopsis": "Legacy identity endpoints." }
  ]
}
```

### `specrun plan propose --from`

Success summary after projecting the agent **response** onto `plan.yaml.slices[]`. `slice-names` is the derived slice set in response order; `slice-count` is its length and `scope-count` is the number of distinct reconciled scopes.

```json
{
  "plan": { "name": "identity-revamp", "path": "/abs/path/to/plan.yaml" },
  "slice-names": ["identity-contracts", "identity-service", "password-reset"],
  "slice-count": 3,
  "scope-count": 2
}
```

### `specrun plan transition`

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

### `specrun plan status`

Dashboard view over a plan. `counts` summarises per-status totals; `entries` carries the full topologically-sorted entry list with per-entry status and depends-on edges; `in-progress` and `next-eligible` are convenience pointers into `entries`.

```json
{
  "counts": {
    "done": 1,
    "in-progress": 1,
    "pending": 7,
    "total": 9
  },
  "drained": false,
  "entries": [
    {
      "depends-on": [],
      "description": null,
      "lifecycle": null,
      "name": "user-registration",
      "sources": ["monolith"],
      "status": "done"
    }
  ],
  "in-progress": {
    "lifecycle": null,
    "name": "email-verification"
  },
  "lifecycle": "pending",
  "next-eligible": null,
  "order": "topological",
  "plan": {
    "name": "platform-v2",
    "path": "<TEMPDIR>/plan.yaml"
  }
}
```

### `specrun plan validate`

Runs the plan-shape diagnostics. The `passed` payload field is a result indicator, not an envelope discriminant — both the clean and failed bodies have the same top-level shape, with `results` either empty or populated.

```json
{
  "passed": true,
  "plan": {
    "name": "demo",
    "path": "<TEMPDIR>/plan.yaml"
  },
  "results": []
}
```

A failed run carries one entry per finding in `results`, each with `code` (kebab-case rule id such as `duplicate-name` or `cycle-in-depends-on`), `entry` (the entry name or `null` for plan-wide findings), `message`, and `severity` (`error` or `warning`).

### `specrun plan archive`

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

### `specrun slice merge run`

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

### `specrun slice task mark`

Marks one task complete. `idempotent: true` indicates the task was already complete and the call was a no-op; the `new-content-path` always points at the updated `tasks.md` regardless.

```json
{
  "idempotent": true,
  "marked": "1.1",
  "new-content-path": "<TEMPDIR>/.specify/slices/my-slice/tasks.md"
}
```

### `specrun slice task progress`

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

### `specrun slice validate`

Runs the slice-shape brief and cross-check predicates and renders a **`DiagnosticReport`** on stdout — the same neutral finding currency every check surface emits (`specrun lint`, `specdev lint`, `slice validate`). The report shape is identical for clean and failed runs; what changes is the `findings[]` content and the `summary` counts.

Each finding carries a `rule-id` (dotted/kebab invariant id such as `design.references-valid-ids` or `slice-provenance-drift`), a `severity` (`critical | important | optional | suggestion`), a `source` (`deterministic | model-assisted | hybrid | human | tool`), and a `kind`:

- `kind: "violation"` — a structural defect. Open `critical`/`important` violations block the lifecycle gate (exit 2).
- `kind: "review"` — a deterministically-raised request for agent/human judgment (the former `deferred` semantic checks). Surfaced but never blocking; the refine agent reads its worklist as `findings.filter(kind == "review")`.

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

A failed run carries one `kind: "violation"` finding per breached invariant (e.g. `rule-id: "slice-provenance-drift"`, `severity: "important"`) with `impact`/`remediation` describing the defect, the `summary` counts rise accordingly, and the process exits 2. The exit carries a payload-free error envelope on **stderr** whose `error` is the gate discriminant (e.g. `slice-pre-adapter-gate`); the rich per-finding detail lives only on the stdout report. See the CLI repo's [DECISIONS.md §"Drained `Error::Validation` and the `Diagnostic` substrate"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate).

---

When migrating a new dispatcher to `Render`, append its body under a stable H3 heading and link from the corresponding `SKILL.md` instead of inlining the JSON example. Trim large example bodies to the smallest shape that illustrates the contract — readers who want byte-for-byte canonical output should follow the fixture link above to the CLI repo.
