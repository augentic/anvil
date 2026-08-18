# CLI output shapes

Canonical JSON envelope shapes for the `emery *` commands that skills shell out to. Skills should **link** to the relevant section here rather than embedding multi-line JSON examples in their `SKILL.md` body. The v1 verb catalogue is archived at git tag `v1`.

## Conventions

- `--format json` responses are a **flat body**: every successful body is a single JSON object carrying the command-specific fields **at the top level** — there is no `ok` discriminant, no `data` wrapper, and no top-level envelope-version stamp.
- Failures keep the same flat shape with three extra top-level keys:
  - `error` — a **kebab-case discriminant string** (e.g. `"init-source-required"`). The discriminant is grep-stable and forms part of the public contract; see [`AGENTS.md`](../../AGENTS.md#exit-codes) for the exit-code table.
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
  "sources": ["documentation", "intent"],
  "emery-version": "0.38.0"
}
```

`emery init` with no source fails with `error: "init-source-required"` (exit 2). A GitHub URL binding fails with `error: "adapter-github-uri-unsupported"` (exit 2).

### `emery specify`

The success body names the committed generation and its reviewable set:

```json
{
  "generation": "9f8e7d6c…",
  "requirements": 3,
  "sources": 3
}
```

Outside an initialised project the verb fails with `error: "not-initialized"` (exit 1). Validation refusals from the extract or synthesis gates (`claim-extras-missing`, `spec-invalid`, `spec-provenance-mismatch`) exit 2.

### `emery completions <shell>`

Emits the shell completion script on stdout (no JSON envelope; the output is the script itself).

## Failure envelope

Every failing verb emits the same flat `ErrorBody` on stderr:

```json
{
  "error": "init-source-required",
  "message": "emery init requires at least one source adapter",
  "exit-code": 2
}
```

An optional `hint` key carries a static recovery hint when the error defines one.
