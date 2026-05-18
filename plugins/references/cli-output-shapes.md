# CLI output shapes

Canonical JSON envelope shapes for `specify *` commands that skills shell out to. Skills should **link** to the relevant section here rather than embedding multi-line JSON examples in their `SKILL.md` body (see [docs/standards/skill-authoring.md "Skill body discipline" #2](../../docs/standards/skill-authoring.md#skill-body-discipline)).

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

The per-command sections below are **generated** from the canonical fixtures in `augentic/specify-cli` (`tests/fixtures/plan/*.json` and `tests/fixtures/e2e/goldens/*.json`). To refresh after a fixture change, run `make doc-envelopes` from the repo root. CI runs the same generator with `--check` to ensure the document and fixtures cannot drift.

<!-- generated:begin -->

### `specify plan amend`

Source fixture: `tests/fixtures/plan/amend-replace-depends-on.json`

```json
{
  "action": "amend",
  "entry": {
    "adapter": null,
    "depends-on": [
      "a",
      "b"
    ],
    "description": null,
    "name": "foo",
    "project": "default",
    "sources": [],
    "status": "pending",
    "status-reason": null
  },
  "envelope-version": 6,
  "plan": {
    "name": "demo",
    "path": "<TEMPDIR>/plan.yaml"
  }
}
```

### `specify plan archive`

#### `outstanding-work`

Source fixture: `tests/fixtures/plan/archive-outstanding-work.json`

```json
{
  "envelope-version": 6,
  "error": "plan-has-outstanding-work",
  "exit-code": 1,
  "message": "plan has outstanding non-terminal work: [\"b\"]"
}
```

#### `success`

Source fixture: `tests/fixtures/plan/archive-success.json`

```json
{
  "archived": "<TEMPDIR>/.specify/archive/plans/demo-<YYYYMMDD>.yaml",
  "archived-plans-dir": null,
  "envelope-version": 6,
  "plan": {
    "name": "demo"
  }
}
```

#### `success-with-working-dir`

Source fixture: `tests/fixtures/plan/archive-success-with-working-dir.json`

```json
{
  "archived": "<TEMPDIR>/.specify/archive/plans/demo-<YYYYMMDD>.yaml",
  "archived-plans-dir": "<TEMPDIR>/.specify/archive/plans/demo-<YYYYMMDD>",
  "envelope-version": 6,
  "plan": {
    "name": "demo"
  }
}
```

### `specify plan add`

Source fixture: `tests/fixtures/plan/create-foo.json`

```json
{
  "action": "create",
  "entry": {
    "adapter": "contracts@v1",
    "depends-on": [],
    "description": null,
    "name": "foo",
    "project": null,
    "sources": [],
    "status": "pending",
    "status-reason": null
  },
  "envelope-version": 6,
  "plan": {
    "name": "demo",
    "path": "<TEMPDIR>/plan.yaml"
  }
}
```

### `specify change draft`

Source fixture: `tests/fixtures/plan/init-success.json`

The merged scaffold writes `change.md` and `plan.yaml` together; the
envelope carries one ref per file. The retired `specify plan
create` envelope (which only carried the `plan` ref) has been replaced
by this shape.

```json
{
  "brief": {
    "path": "<TEMPDIR>/change.md"
  },
  "envelope-version": 6,
  "name": "my-change",
  "plan": {
    "path": "<TEMPDIR>/plan.yaml"
  }
}
```

### `specify plan next`

#### `all-done`

Source fixture: `tests/fixtures/plan/next-all-done.json`

```json
{
  "active": null,
  "adapter": null,
  "description": null,
  "envelope-version": 6,
  "next": null,
  "project": null,
  "reason": "all-done",
  "sources": null
}
```

#### `first-pending`

Source fixture: `tests/fixtures/plan/next-first-pending.json`

```json
{
  "active": null,
  "adapter": null,
  "description": null,
  "envelope-version": 6,
  "next": "b",
  "project": "default",
  "reason": null,
  "sources": []
}
```

#### `in-progress`

Source fixture: `tests/fixtures/plan/next-in-progress.json`

```json
{
  "active": "a",
  "adapter": null,
  "description": null,
  "envelope-version": 6,
  "next": null,
  "project": null,
  "reason": "in-progress",
  "sources": null
}
```

#### `stuck`

Source fixture: `tests/fixtures/plan/next-stuck.json`

```json
{
  "active": null,
  "adapter": null,
  "description": null,
  "envelope-version": 6,
  "next": null,
  "project": null,
  "reason": "stuck",
  "sources": null
}
```

### `specify plan status`

Source fixture: `tests/fixtures/plan/status-platform-v2.json`

```json
{
  "blocked": [],
  "counts": {
    "blocked": 0,
    "done": 1,
    "failed": 1,
    "in-progress": 1,
    "pending": 6,
    "skipped": 0,
    "total": 9
  },
  "entries": [
    {
      "depends-on": [],
      "description": null,
      "lifecycle": null,
      "name": "user-registration",
      "sources": [
        "monolith"
      ],
      "status": "done",
      "status-reason": null
    },
    {
      "depends-on": [
        "user-registration"
      ],
      "description": null,
      "lifecycle": null,
      "name": "email-verification",
      "sources": [
        "monolith"
      ],
      "status": "in-progress",
      "status-reason": null
    },
    {
      "depends-on": [],
      "description": "Duplicate email submission returns 500 instead of 409. Discovered during email-verification extraction. Modifies user-registration.\n",
      "lifecycle": null,
      "name": "registration-duplicate-email-crash",
      "sources": [],
      "status": "pending",
      "status-reason": null
    },
    {
      "depends-on": [
        "user-registration"
      ],
      "description": "Greenfield — user-facing notification channel and frequency settings.\n",
      "lifecycle": null,
      "name": "notification-preferences",
      "sources": [],
      "status": "pending",
      "status-reason": null
    },
    {
      "depends-on": [
        "email-verification"
      ],
      "description": "Pull duplicated input validation into a shared validation crate before building checkout-flow. Delta-targets user-registration and email-verification.\n",
      "lifecycle": null,
      "name": "extract-shared-validation",
      "sources": [],
      "status": "pending",
      "status-reason": null
    },
    {
      "depends-on": [
        "extract-shared-validation"
      ],
      "description": null,
      "lifecycle": null,
      "name": "product-catalog",
      "sources": [
        "monolith"
      ],
      "status": "pending",
      "status-reason": null
    },
    {
      "depends-on": [
        "product-catalog",
        "user-registration"
      ],
      "description": null,
      "lifecycle": null,
      "name": "shopping-cart",
      "sources": [
        "orders"
      ],
      "status": "pending",
      "status-reason": null
    },
    {
      "depends-on": [
        "shopping-cart"
      ],
      "description": null,
      "lifecycle": null,
      "name": "checkout-api",
      "sources": [
        "payments"
      ],
      "status": "failed",
      "status-reason": "Type mismatch between cart line-item schema and payment gateway contract. Needs design revision after shopping-cart specs are updated.\n"
    },
    {
      "depends-on": [
        "checkout-api"
      ],
      "description": null,
      "lifecycle": null,
      "name": "checkout-ui",
      "sources": [
        "frontend"
      ],
      "status": "pending",
      "status-reason": null
    }
  ],
  "envelope-version": 6,
  "failed": [
    {
      "name": "checkout-api",
      "reason": "Type mismatch between cart line-item schema and payment gateway contract. Needs design revision after shopping-cart specs are updated.\n"
    }
  ],
  "in-progress": {
    "lifecycle": null,
    "name": "email-verification"
  },
  "next-eligible": null,
  "order": "topological",
  "plan": {
    "name": "platform-v2",
    "path": "<TEMPDIR>/plan.yaml"
  }
}
```

### `specify plan transition`

#### `in-progress-to-done`

Source fixture: `tests/fixtures/plan/transition-in-progress-to-done.json`

```json
{
  "entry": {
    "name": "foo",
    "status": "done",
    "status-reason": null
  },
  "envelope-version": 6,
  "plan": {
    "name": "demo",
    "path": "<TEMPDIR>/plan.yaml"
  }
}
```

#### `in-progress-to-failed-with-reason`

Source fixture: `tests/fixtures/plan/transition-in-progress-to-failed-with-reason.json`

```json
{
  "entry": {
    "name": "foo",
    "status": "failed",
    "status-reason": "boom"
  },
  "envelope-version": 6,
  "plan": {
    "name": "demo",
    "path": "<TEMPDIR>/plan.yaml"
  }
}
```

#### `pending-to-in-progress`

Source fixture: `tests/fixtures/plan/transition-pending-to-in-progress.json`

```json
{
  "entry": {
    "name": "foo",
    "status": "in-progress",
    "status-reason": null
  },
  "envelope-version": 6,
  "plan": {
    "name": "demo",
    "path": "<TEMPDIR>/plan.yaml"
  }
}
```

### `specify plan validate`

#### `clean`

Source fixture: `tests/fixtures/plan/validate-clean.json`

```json
{
  "envelope-version": 6,
  "passed": true,
  "plan": {
    "name": "demo",
    "path": "<TEMPDIR>/plan.yaml"
  },
  "results": []
}
```

#### `duplicate-name`

Source fixture: `tests/fixtures/plan/validate-duplicate-name.json`

```json
{
  "envelope-version": 6,
  "passed": false,
  "plan": {
    "name": "demo",
    "path": "<TEMPDIR>/plan.yaml"
  },
  "results": [
    {
      "code": "duplicate-name",
      "entry": "foo",
      "level": "error",
      "message": "duplicate plan entry name 'foo'"
    }
  ]
}
```

### `specify slice merge run`

Source fixture: `tests/fixtures/e2e/goldens/merge-two-spec.json`

```json
{
  "envelope-version": 6,
  "merged-specs": [
    {
      "name": "login",
      "operations": [
        {
          "id": "REQ-001",
          "kind": "added",
          "name": "User can log in"
        }
      ]
    },
    {
      "name": "oauth",
      "operations": [
        {
          "id": "REQ-001",
          "kind": "added",
          "name": "Handle OAuth callback"
        }
      ]
    }
  ]
}
```

### `specify slice outcome show`

Source fixture: `tests/fixtures/e2e/goldens/slice-outcome.json`

```json
{
  "envelope-version": 6,
  "name": "foo",
  "outcome": {
    "at": "<ISO8601>",
    "context": "trailing newline",
    "outcome": "success",
    "phase": "build",
    "summary": "5/5 tasks"
  }
}
```

### `specify slice task mark`

Source fixture: `tests/fixtures/e2e/goldens/task-mark.json`

```json
{
  "envelope-version": 6,
  "idempotent": true,
  "marked": "1.1",
  "new-content-path": "<TEMPDIR>/.specify/slices/my-slice/tasks.md"
}
```

### `specify slice task progress`

Source fixture: `tests/fixtures/e2e/goldens/task-progress.json`

```json
{
  "complete": 2,
  "envelope-version": 6,
  "pending": 3,
  "tasks": [
    {
      "complete": false,
      "description": "Create the `login` crate skeleton",
      "group": "1. Scaffold",
      "number": "1.1",
      "skill-directive": null
    },
    {
      "complete": true,
      "description": "Wire the crate into the workspace",
      "group": "1. Scaffold",
      "number": "1.2",
      "skill-directive": null
    },
    {
      "complete": true,
      "description": "Implement the session issuer per REQ-001",
      "group": "2. Implement",
      "number": "2.1",
      "skill-directive": null
    },
    {
      "complete": false,
      "description": "Add unit tests covering the happy path",
      "group": "2. Implement",
      "number": "2.2",
      "skill-directive": null
    },
    {
      "complete": false,
      "description": "Document the public API",
      "group": "2. Implement",
      "number": "2.3",
      "skill-directive": null
    }
  ],
  "total": 5
}
```

### `specify slice validate`

#### `clean`

Source fixture: `tests/fixtures/e2e/goldens/validate-good.json`

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
        "rule": "Has a Crates/Features section listing at least one entry",
        "rule-id": "proposal.crates-listed",
        "status": "pass"
      },
      {
        "reason": "Semantic check — requires LLM judgment",
        "rule": "Uses imperative language for motivation",
        "rule-id": "proposal.uses-imperative-language",
        "status": "deferred"
      }
    ],
    "specs": [
      {
        "rule": "Every requirement has at least one scenario",
        "rule-id": "specs.requirements-have-scenarios",
        "status": "pass"
      },
      {
        "rule": "Every requirement has an `ID:` line",
        "rule-id": "specs.requirements-have-ids",
        "status": "pass"
      },
      {
        "rule": "IDs use the `REQ-[0-9]{3}` format",
        "rule-id": "specs.ids-match-pattern",
        "status": "pass"
      },
      {
        "reason": "Semantic check — requires LLM judgment",
        "rule": "Uses SHALL/MUST language for normative requirements",
        "rule-id": "specs.uses-normative-language",
        "status": "deferred"
      }
    ],
    "tasks": [
      {
        "rule": "All tasks use `- [ ] X.Y` checkbox format",
        "rule-id": "tasks.use-checkbox-format",
        "status": "pass"
      },
      {
        "rule": "Tasks grouped under `## ` headings",
        "rule-id": "tasks.grouped-under-headings",
        "status": "pass"
      }
    ]
  },
  "cross-checks": [
    {
      "rule": "Every crate/feature listed in the proposal has a matching spec file",
      "rule-id": "cross.proposal-crates-have-specs",
      "status": "pass"
    },
    {
      "rule": "Every requirement id referenced in design.md exists in specs",
      "rule-id": "cross.design-references-valid",
      "status": "pass"
    },
    {
      "rule": "composition.yaml maps_to values are well-formed",
      "rule-id": "cross.composition-maps-to-consistent",
      "status": "pass"
    }
  ],
  "envelope-version": 6,
  "passed": true
}
```

#### `with-findings`

Source fixture: `tests/fixtures/e2e/goldens/validate-bad.json`

```json
{
  "brief-results": {
    "design": [
      {
        "detail": "design.md references requirement IDs not present in any baseline spec",
        "rule": "References only requirement ids present in specs",
        "rule-id": "design.references-valid-ids",
        "status": "fail"
      }
    ],
    "proposal": [
      {
        "detail": "`## Why` section missing or has no prose",
        "rule": "Has a Why section with at least one sentence",
        "rule-id": "proposal.why-has-content",
        "status": "fail"
      },
      {
        "rule": "Has a Crates/Features section listing at least one entry",
        "rule-id": "proposal.crates-listed",
        "status": "pass"
      },
      {
        "reason": "Semantic check — requires LLM judgment",
        "rule": "Uses imperative language for motivation",
        "rule-id": "proposal.uses-imperative-language",
        "status": "deferred"
      }
    ],
    "specs": [
      {
        "detail": "one or more requirements have no scenarios",
        "rule": "Every requirement has at least one scenario",
        "rule-id": "specs.requirements-have-scenarios",
        "status": "fail"
      },
      {
        "rule": "Every requirement has an `ID:` line",
        "rule-id": "specs.requirements-have-ids",
        "status": "pass"
      },
      {
        "detail": "one or more requirement IDs do not match `^REQ-[0-9]{3}$`",
        "rule": "IDs use the `REQ-[0-9]{3}` format",
        "rule-id": "specs.ids-match-pattern",
        "status": "fail"
      },
      {
        "reason": "Semantic check — requires LLM judgment",
        "rule": "Uses SHALL/MUST language for normative requirements",
        "rule-id": "specs.uses-normative-language",
        "status": "deferred"
      }
    ],
    "tasks": [
      {
        "detail": "found `- …` bullets that do not match the `- [ ] X.Y` checkbox format",
        "rule": "All tasks use `- [ ] X.Y` checkbox format",
        "rule-id": "tasks.use-checkbox-format",
        "status": "fail"
      },
      {
        "rule": "Tasks grouped under `## ` headings",
        "rule-id": "tasks.grouped-under-headings",
        "status": "pass"
      }
    ]
  },
  "cross-checks": [
    {
      "detail": "one or more crates/features listed in the proposal have no matching spec file",
      "rule": "Every crate/feature listed in the proposal has a matching spec file",
      "rule-id": "cross.proposal-crates-have-specs",
      "status": "fail"
    },
    {
      "detail": "design.md references requirement IDs that are not present in the baseline",
      "rule": "Every requirement id referenced in design.md exists in specs",
      "rule-id": "cross.design-references-valid",
      "status": "fail"
    },
    {
      "rule": "composition.yaml maps_to values are well-formed",
      "rule-id": "cross.composition-maps-to-consistent",
      "status": "pass"
    }
  ],
  "envelope-version": 6,
  "passed": false
}
```

<!-- generated:end -->

---

When migrating a new dispatcher to `Render`, append its body under a stable H3 heading and link from the corresponding `SKILL.md` instead of inlining the JSON example. If the new command also lands a fixture under `tests/fixtures/plan/` or `tests/fixtures/e2e/goldens/`, prefer regenerating this file via `make doc-envelopes` so the example stays bit-for-bit identical to the test data.
