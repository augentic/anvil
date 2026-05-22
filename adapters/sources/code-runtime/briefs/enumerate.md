---
id: enumerate
description: Walk the read-only fixture tree at $SOURCE_DIR and emit one `## Candidate inventory` block per observed handler entry point (HTTP route, message handler, scheduled job, WebSocket handler).
authority: behaviour
---

# Runtime fixture enumeration

`/spec:plan` invokes this brief once per binding under `plan.yaml.sources.<key>` whose adapter is `code-runtime`. Your job: walk the read-only fixture tree at `$SOURCE_DIR`, identify one handler-grain candidate per `tests/data/replay/<handler>/` directory the wiretapper captured, and return one candidate block per handler. The CLI appends your blocks under `## Candidate inventory` in `discovery.md`; you never write `discovery.md` directly.

## Binding

Operators bind a captured-fixture directory under `plan.yaml.sources.<key>`:

```yaml
sources:
  runtime:
    adapter: code-runtime
    path: ./fixtures/replay
```

The bound `path:` becomes `$SOURCE_DIR` at invocation time. The expected layout matches the format the RT wiretapper writes — see [fixture-format reference](../../../plugins/rt/skills/replay-writer/references/fixture-format.md):

```text
$SOURCE_DIR/
├── tests/data/replay/
│   ├── <handler>/                # one subdirectory per captured handler
│   │   ├── <scenario>.json       # TestDef-style fixture (one scenario per file)
│   │   └── INSTRUCTIONS.md       # optional per-handler hint material
│   └── samples/                  # optional shared payloads (not fixtures)
└── ...
```

Operators with a non-conforming layout adapt the directory or write a thin wrapper adapter; v1 does not invent a new fixture format.

## Inputs

- **`$SOURCE_DIR`** — read-only preopen of the operator-bound fixture root. Walk this tree; never write into it.
- **Source key** — kebab-case identifier passed in via the runner (the `<key>` from `plan.yaml.sources.<key>`). Echoed into every candidate's `sources:` list.

The bound directory is the only filesystem grant — `$PROJECT_DIR` is unreachable, host env is unreadable, the network is denied. Use `$SCRATCH_DIR` for unavoidable intermediate state.

## Candidate grain

One candidate per observed handler — that is, one per `tests/data/replay/<handler>/` directory. Each directory groups every captured scenario for one HTTP route, message handler, scheduled job, or WebSocket handler. The slice grain operators reason about is the handler, not the individual fixture; per-scenario detail lives in `extract`-time claims (one `kind: example` claim per scenario file).

The directory name is the kebab-case handler identifier — keep it verbatim as the candidate `id`. When two sources surface the same handler under different names (e.g. `password-reset` here, `account-pwd-reset` in the legacy code source), the operator adds an `aliases:` row at plan time through `specify plan amend --add-alias`; do not invent aliases here.

## Output: candidate blocks

Emit one fenced block per identified handler, in the shape the CLI appends under `## Candidate inventory`:

```markdown
### <handler-id>

- id: <handler-id>
- sources: [<source-key>]
- summary: <one-line description>
```

Field order is fixed (`id`, `sources`, `summary`). `id` is kebab-case and matches the `<handler>/` directory name verbatim. `summary` is a single line that names the surface (HTTP route + method, queue + job name, cron expression, WebSocket topic) and the captured-scenario count. Quote concrete counts the fixtures themselves verify; do not infer from `INSTRUCTIONS.md` prose alone. The block validates against `schemas/discovery/candidate.schema.json`.

Emit blocks sorted alphabetically by `id` so re-enumeration produces byte-stable diffs.

## Algorithm

1. **Walk `tests/data/replay/`.** Enumerate immediate subdirectories. Skip `samples/` (shared payloads, not handlers) and any directory whose name begins with `.` or `_`.
2. **Per handler, inventory scenarios.** List `<handler>/*.json`. Skip the optional per-handler `INSTRUCTIONS.md` — the brief is not authoritative for surface naming. Zero-scenario handler directories are skipped silently (the operator drops them upstream).
3. **Identify the surface.** Inspect one or two scenario files to derive the route / topic / job identifier and method (e.g. `POST /users`, queue `user.created`, cron `0 */5 * * *`). When scenarios disagree, prefer the most common surface and note the spread in `summary`.
4. **Emit one candidate block per handler.** Sort by `id`. Each block carries the handler id, the supplied source key, and a one-line summary.

## Path rules

Every internal reference to a fixture path is relative under `$SOURCE_DIR`:

- No leading `/`, no Windows drive letter, no `..` segments.
- Resolves to a file under `$SOURCE_DIR`.
- Never walks outside `tests/data/replay/` for candidate identification — sibling source trees are not the adapter's concern.

A symlink inside `$SOURCE_DIR` pointing outside the bound root is denied at canonicalization; the host runner returns `source-enumerate-path-denied` and the slice stays `refining`.

## Worked example

Bound directory (relative to `$SOURCE_DIR`):

```text
tests/data/replay/
├── password-reset/
│   ├── happy-path.json
│   ├── unknown-email.json
│   └── INSTRUCTIONS.md
├── user-registration/
│   ├── duplicate-email.json
│   ├── happy.json
│   └── invalid-password.json
└── samples/
    └── argon2-hashes.json
```

Expected output (alphabetically by `id`, source key `runtime`):

```markdown
### password-reset

- id: password-reset
- sources: [runtime]
- summary: POST /accounts/reset observed in 2 fixtures; both return 202 with no body.

### user-registration

- id: user-registration
- sources: [runtime]
- summary: POST /users observed in 3 fixtures; happy path publishes `user.created`, error paths return 400 and 409.
```

## Determinism

- Emit candidates sorted alphabetically by `id`.
- Field order inside each block is fixed: `id`, `sources`, `summary`.
- Quote concrete scenario counts and surface identifiers the fixtures verify; do not embed timestamps, host paths, or other run-state.
- Re-running against an unchanged fixture tree produces byte-identical blocks.

## Anti-patterns

- **Inventing handlers from `INSTRUCTIONS.md`.** The prose is operator hint material; the directory listing is the candidate source of truth. If a handler is named in `INSTRUCTIONS.md` but has no scenario JSON files, emit nothing for it.
- **Per-scenario candidates.** One block per `<handler>/` directory, never one per `<scenario>.json`. Scenario-level detail belongs in `extract`'s `kind: example` claims.
- **Cross-source aliasing here.** Aliases are added at plan time via `specify plan amend --add-alias`; this brief sees one source's tree.
- **Writing `discovery.md` or `plan.yaml`.** Only candidate blocks. The CLI owns every lifecycle file.

## Failure modes

| Condition | Action |
| --- | --- |
| `$SOURCE_DIR` empty or missing `tests/data/replay/` | Return zero candidates. Operator reviews in `discovery.md`. |
| `tests/data/replay/<handler>/` contains no `*.json` files | Skip the handler silently. |
| Read denied outside `$SOURCE_DIR` | Host runner returns `source-enumerate-path-denied`; slice stays `refining`. |
| Fixture JSON unparseable during surface identification | Continue with the remaining scenarios; surface ambiguity surfaces in the `summary` line. |

## References

- [RFC-27 §Runtime source adapter (D1)](../../../rfcs/rfc-27-synthesis.md#runtime-source-adapter-d1)
- [Fixture format reference](../../../plugins/rt/skills/replay-writer/references/fixture-format.md)
