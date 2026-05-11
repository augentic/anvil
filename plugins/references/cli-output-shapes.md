# CLI output shapes

Canonical JSON envelope shapes for `specify *` commands that skills shell out to. Skills should **link** to the relevant section here rather than embedding multi-line JSON examples in their `SKILL.md` body (see [AGENTS.md "Skill body discipline" #2](../../AGENTS.md#skill-body-discipline)).

## Conventions

- Every successful `--format json` response is wrapped in a top-level envelope:
  ```json
  {
    "schema-version": 5,
    "ok": true,
    "data": { /* command-specific body */ }
  }
  ```
- Errors use the same envelope with `"ok": false` and `"error": { "code": "<kebab-discriminant>", "detail": "..." }`. The `code` is grep-stable and forms part of the public contract; see [`AGENTS.md` in `augentic/specify-cli`](https://github.com/augentic/specify-cli/blob/main/AGENTS.md#error-handling-and-exit-codes) for the full discriminant catalogue.
- Paths are emitted as plain strings relative to the repo root unless the field name says otherwise (`absolute-path`, `tempdir-path`).
- All keys are `kebab-case`. The `schema-version` integer bumps on any breaking change to a body shape.

## Shapes

### `specify status`

Dashboard for the whole project. The `slices` array is empty when no slices exist.

```json
{
  "registry": { "version": 1, "projects": [/* RegistryProject[] */] },
  "plan": { "name": "demo", "counts": { "done": 0, "in-progress": 0, "pending": 0, "blocked": 0, "failed": 0, "skipped": 0, "total": 0 } },
  "slices": [
    { "name": "slice-name", "status": "defining", "tasks": { "complete": 0, "total": 0 } }
  ]
}
```

`registry` and `plan` are `null` when the corresponding artifact is absent.

### `specify init`

```json
{
  "project-dir": "/abs/path",
  "config-path": ".specify/project.yaml",
  "capability-name": "omnia",
  "hub": false,
  "created": ["AGENTS.md", ".specify/project.yaml"]
}
```

### `specify registry show` / `validate`

```json
{
  "registry": { "version": 1, "projects": [{ "name": "alpha", "url": ".", "capability": "omnia@v1", "description": null, "contracts": null }] },
  "path": "registry.yaml",
  "ok": true
}
```

`registry` is `null` when no `registry.yaml` exists.

### `specify registry add` / `remove`

Same shape as `validate`, plus `added: RegistryProject` (for `add`) or `removed: "<name>"` and `warnings: string[]` (for `remove`).

### `specify slice create`

```json
{
  "name": "slice-name",
  "path": ".specify/slices/slice-name",
  "metadata": { "capability": "omnia", "status": "defining", "created-at": "..." }
}
```

### `specify slice outcome show`

Returns the `.metadata.yaml.outcome` block verbatim under `outcome`, or `null` when no outcome has been stamped:

```json
{ "name": "slice-name", "phase": "build", "outcome": null }
```

### `specify slice status`

```json
{ "name": "slice-name", "status": "building", "tasks": { "complete": 3, "total": 7 } }
```

### `specify capability resolve`

```json
{
  "capability": "omnia@v1",
  "manifest-path": ".specify/capabilities/omnia.yaml",
  "validations": [{ "rule-id": "rule-001", "rule": "...", "ok": true, "reason": null }]
}
```

### `specify change plan validate`

```json
{
  "plan-path": "plan.yaml",
  "ok": true,
  "warnings": [{ "code": "capability-mismatch-workspace", "entry": "alpha-feature", "detail": "..." }]
}
```

### `specify change plan next` / `transition`

```json
{
  "name": "alpha-feature",
  "project": "alpha",
  "capability": "omnia@v1",
  "status": "in-progress",
  "depends-on": []
}
```

### `specify tool *` (WASI tooling)

WASI-host commands all emit:

```json
{ "tool": "contract-validate", "exit-code": 0, "stdout": "...", "stderr": "" }
```

### `specify compatibility check` / `report`

```json
{ "ok": true, "report": { /* CompatibilityReport from specify_validate */ } }
```

---

When migrating a new dispatcher to `Render`, append its body under a stable H3 heading and link from the corresponding `SKILL.md` instead of inlining the JSON example.
