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

### `specrun plan transition`

Used for both entry transitions (`kind: "entry"`) and the plan-level review stamp (`kind: "plan"`). The `previous` / `current` pair pins the legal transition rung that fired.

```json
{
  "current": "reviewed",
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

Runs the slice-shape brief and cross-check predicates. Like `plan validate`, `passed` is a payload indicator and the envelope shape is identical for clean and failed runs.

`brief-results` is keyed by per-brief or per-spec scope (`proposal`, `design`, `tasks`, `specs/<name>/spec.md`); each entry is a list of `{rule, rule-id, status, detail?}` records where `status` is `pass`, `fail`, or `deferred` (semantic checks that need LLM judgement). `cross-checks[]` carries inter-artifact predicates with the same record shape.

```json
{
  "brief-results": {
    "design": [
      {
        "rule": "References only requirement ids present in specs",
        "rule-id": "design.references-valid-ids",
        "status": "pass"
      }
    ],
    "proposal": [
      {
        "rule": "Has a Why section with at least one sentence",
        "rule-id": "proposal.why-has-content",
        "status": "pass"
      },
      {
        "detail": "Semantic check — requires LLM judgment",
        "rule": "Uses imperative language for motivation",
        "rule-id": "proposal.uses-imperative-language",
        "status": "deferred"
      }
    ],
    "specs/login/spec.md": [
      {
        "rule": "Every requirement has at least one scenario",
        "rule-id": "specs.requirements-have-scenarios",
        "status": "pass"
      }
    ],
    "tasks": [
      {
        "rule": "All tasks use `- [ ] X.Y` checkbox format",
        "rule-id": "tasks.use-checkbox-format",
        "status": "pass"
      }
    ]
  },
  "cross-checks": [
    {
      "rule": "Every crate listed in the proposal has a matching spec file",
      "rule-id": "cross.proposal-crates-have-specs",
      "status": "pass"
    }
  ],
  "passed": true
}
```

A failed run keeps the same shape with `status: "fail"` on the offending records and an optional `detail` describing why the predicate fired.

---

When migrating a new dispatcher to `Render`, append its body under a stable H3 heading and link from the corresponding `SKILL.md` instead of inlining the JSON example. Trim large example bodies to the smallest shape that illustrates the contract — readers who want byte-for-byte canonical output should follow the fixture link above to the CLI repo.
