# CLI output shapes

Canonical JSON envelope shapes for the `emery *` commands that skills shell out to. Skills should **link** to the relevant section here rather than embedding multi-line JSON examples in their `SKILL.md` body. The v1 verb catalogue is archived at git tag `v1`.

## Conventions

- `--format json` responses are a **flat body**: every successful body is a single JSON object carrying the command-specific fields **at the top level** — there is no `ok` discriminant, no `data` wrapper, and no top-level envelope-version stamp.
- Failures keep the same flat shape with three extra top-level keys:
  - `error` — a discriminant string: kebab-case for the three recovery codes (`specify-source-required`, `adapter-cli-too-old`, `spec-not-generated`), snake_case for the Omnia defaults (`bad_request`, `not_found`, `server_error`, `bad_gateway`). The discriminant is grep-stable and forms part of the public contract; see [`AGENTS.md`](../../AGENTS.md#exit-codes) for the exit-code table.
  - `message` — humanised one-liner suitable for direct rendering.
  - `exit-code` — the integer the binary returns.
- Paths are emitted as plain strings relative to the repo root unless the field name says otherwise.
- All keys are `kebab-case`. Body shapes are pinned by the typed `*Body` DTOs in the CLI workspace and change only with the CLI's own versioning.
- Stream roles: the semantic result body (text or JSON) is **stdout**; the failure `ErrorBody` and live host tracing are **stderr**. Tracing verbosity is selected by the reserved host log flags (`--debug` / `--quiet`, peeled before the guest sees argv; see [cli-contract.md](../standards/cli-contract.md)).

## Text-mode style

Every `Render` impl follows one convention so operators can scan any command's output the same way:

- **Result line first, lowercase, verb-first**: `committed generation 9f8e7d6c…`.
- **Detail lines are indented `label: value` pairs** with kebab-case labels: `  sources: 3`.
- **Names in backticks**, paths bare.
- **No trailing periods** on result or detail lines.
- **`hint:` is recovery guidance** (what to fix); **`resume:` is the literal next command** (what to run). A line is one or the other, never both.
- **Every empty state prints a lowercase line** — silence is never the empty rendering.

One documented exception: `emery show` renders the document body alone in text mode — no result line — so its stdout pipes and redirects as the document itself. Its generation id rides the JSON envelope.

## Shapes

The examples below are hand-curated illustrations of the happy path; the accept/reject variant set is exercised by the integration suites under `crates/*/tests/`.

### `emery specify`

The success body names the committed generation and its reviewable set:

```json
{
  "generation": "9f8e7d6c…",
  "requirements": 3,
  "sources": 3,
  "diff": {
    "from": "1a2b3c4d…",
    "artifacts": ["spec.md"],
    "added": [],
    "removed": [],
    "changed": ["session.timeout"]
  }
}
```

`diff` is the re-mine diff against the superseded generation: the changed spec-set artifacts plus the requirement subjects added, removed, or changed in `spec.md`. It is absent on a first run and empty (`artifacts: []`) on a byte-stable re-run; nothing is persisted for it.

`emery specify` with no source — and no project-root `emery.toml` to discover — fails with `error: "specify-source-required"` (exit 1); mixing `--config` with positional adapters or `--description`, or naming an absolute or project-escaping local path, fails with `error: "bad_request"` (exit 1). `--config` without a value explicitly selects the project-relative `emery.toml`. A GitHub URL binding fails with `error: "bad_request"`. Validation refusals from the extract or synthesis gates exit 1 with `error: "bad_request"`.

### `emery show <spec|design>`

The success body wraps the document with its generation id; text mode is the document body alone (see the exception above).

```json
{
  "generation": "9f8e7d6c…",
  "document": "spec",
  "body": "# Specification\n…"
}
```

Before any generation is committed the verb fails with `error: "spec-not-generated"` (exit 2); a pointer naming missing documents fails with `error: "server_error"` (exit 3).

### `emery completions <shell>`

Emits the shell completion script on stdout (no JSON envelope; the output is the script itself).

## Failure envelope

Every failing verb emits the same flat `ErrorBody` on stderr:

```json
{
  "error": "specify-source-required",
  "message": "specify-source-required: emery specify requires at least one source: …",
  "exit-code": 1
}
```

An optional `hint` key carries a static recovery hint when the error defines one.
