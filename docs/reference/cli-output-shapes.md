# CLI output shapes

Canonical JSON envelope shapes for the `emery *` commands that skills shell out to. Skills should **link** to the relevant section here rather than embedding multi-line JSON examples in their `SKILL.md` body. The v1 verb catalogue is archived at git tag `v1`.

## Conventions

- `--format json` responses are a **flat body**: every successful body is a single JSON object carrying the command-specific fields **at the top level** — there is no `ok` discriminant, no `data` wrapper, and no top-level envelope-version stamp.
- Failures keep the same flat shape with three extra top-level keys:
  - `error` — a **kebab-case discriminant string** (e.g. `"init-adapter-required"`). The discriminant is grep-stable and forms part of the public contract; see [`AGENTS.md`](../../AGENTS.md#exit-codes) for the exit-code table.
  - `message` — humanised one-liner suitable for direct rendering.
  - `exit-code` — the integer the binary returns.
- Paths are emitted as plain strings relative to the repo root unless the field name says otherwise.
- All keys are `kebab-case`. Body shapes are pinned by the typed `*Body` DTOs in the CLI workspace and change only with the CLI's own versioning.
- Stream roles: the semantic result body (text or JSON) is **stdout**; the failure `ErrorBody` and live host tracing are **stderr**. Tracing verbosity is selected by the reserved host log flags (`--debug` / `--quiet`, peeled before the guest sees argv; see [cli-contract.md](../standards/cli-contract.md)).

## Text-mode style

Every `Render` impl follows one convention so operators can scan any command's output the same way:

- **Result line first, lowercase, verb-first**: `initialized project`.
- **Detail lines are indented `label: value` pairs** with kebab-case labels: `  config: .emery/project.yaml`.
- **Names in backticks**, paths bare.
- **No trailing periods** on result or detail lines.
- **`hint:` is recovery guidance** (what to fix); **`resume:` is the literal next command** (what to run). A line is one or the other, never both.
- **Every empty state prints a lowercase line** — silence is never the empty rendering.

## Shapes

The examples below are hand-curated illustrations of the happy path; the accept/reject variant set is exercised by the integration suites under `crates/*/tests/`.

### `emery init`

The success body's `mode` is the closed run discriminant (`scaffolded` | `already-initialized` | `upgraded`).

```json
{
  "mode": "scaffolded",
  "config-path": "/work/app/.emery/project.yaml",
  "adapter-name": "intent",
  "adapter-binding": "intent",
  "cache-present": false,
  "directories-created": [".emery", ".emery/cache/components"],
  "scaffolded-rule-keys": [],
  "emery-version": "0.38.0",
  "context-generated": true,
  "context-skipped": false
}
```

`emery init` with no adapter fails with `error: "init-adapter-required"` (exit 2). A GitHub URL binding fails with `error: "adapter-github-uri-unsupported"` (exit 2).

### `emery specify`

Reserved for the Phase 3 spec generator; today the verb always fails typed:

```json
{
  "error": "specify-not-implemented",
  "message": "specify-not-implemented: `emery specify` is reserved for the spec generator; it lands with the remediation programme's Phase 3 walking skeleton",
  "exit-code": 1
}
```

### `emery completions <shell>`

Emits the shell completion script on stdout (no JSON envelope; the output is the script itself).

## Failure envelope

Every failing verb emits the same flat `ErrorBody` on stderr:

```json
{
  "error": "init-adapter-required",
  "message": "emery init requires an adapter",
  "exit-code": 2
}
```

An optional `hint` key carries a static recovery hint when the error defines one.
