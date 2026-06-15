# Runtime capture survey

`/spec:plan` invokes this brief once per binding under `plan.yaml.sources.<key>` whose adapter is `captures`. Your job: walk the read-only capture tree at `$SOURCE_DIR`, identify one handler-grain lead per `tests/data/replays/<handler>/` directory the wiretapper captured, and return one lead block per handler. The CLI appends your blocks under `## Lead inventory` in `discovery.md`; you never write `discovery.md` directly.

## Binding

Operators bind a captured runtime-data directory under `plan.yaml.sources.<key>`:

```yaml
sources:
  runtime:
    adapter: captures
    path: ./captures/replays
```

The bound `path:` becomes `$SOURCE_DIR` at invocation time. The expected layout matches the format `/capture:wiretapper` writes — see [capture-format reference](../references/capture-format.md):

```text
$SOURCE_DIR/
├── tests/data/replays/
│   ├── <handler>/                # one subdirectory per captured handler
│   │   ├── <scenario>.json       # TestDef-style capture (one scenario per file)
│   │   └── INSTRUCTIONS.md       # optional per-handler hint material
│   └── samples/                  # optional shared payloads (not captures)
└── ...
```

Operators with a non-conforming layout adapt the directory or write a thin wrapper adapter; v1 does not invent a new capture format.

## Inputs

- **`$SOURCE_DIR`** — read-only preopen of the operator-bound capture root. Walk this tree; never write into it.
- **Source key** — kebab-case identifier passed in via the runner (the `<key>` from `plan.yaml.sources.<key>`). The CLI stamps each lead's `source` from it; this brief does not emit it.

The bound directory is the only filesystem grant — `$PROJECT_DIR` is unreachable, host env is unreadable, the network is denied. Use `$SCRATCH_DIR` for unavoidable intermediate state.

## Lead grain

One lead per observed handler — that is, one per `tests/data/replays/<handler>/` directory. Each directory groups every captured scenario for one HTTP route, message handler, scheduled job, or WebSocket handler. The slice grain operators reason about is the handler, not the individual capture; per-scenario detail lives in `extract`-time claims (one `kind: example` claim per scenario file).

The directory name is the kebab-case handler identifier — keep it verbatim as the lead `lead`. When two sources surface the same handler under different slugs (e.g. `password-reset` here, `account-pwd-reset` in the legacy code source), cross-source reconciliation is agent judgment at propose time and Gate 1 curation via `specify plan amend <entry> --sources`; do not invent alternate names here.

## Output: lead blocks

Emit one fenced block per identified handler, in the shape the CLI appends under `## Lead inventory`:

```markdown
### <handler-id>

- lead: <handler-id>
- synopsis: <reconciliation-grade headline>
- topics: [<optional-kebab-slugs>]
```

Field order is fixed (`lead`, `synopsis`, then optional `topics`). `lead` is kebab-case and matches the `<handler>/` directory name verbatim. Do not emit `source`; the CLI stamps it from the survey binding. `topics` (optional) is an inline list of kebab-case domain slugs drawn from the handler's surface; author them only when the captures clearly support the classification, omit the bullet otherwise. They are extra grouping context for `propose` and the join key for the decision-contradiction warning — never a grouping the CLI computes. `synopsis` names the surface (HTTP route + method, queue + job name, cron expression, WebSocket topic) and the captured-scenario count — content-bearing enough that a same-slug lead from another source can be matched or distinguished on content, not just the shared slug. Prefer one line; it MAY run to a few lines when one is too thin. Quote concrete counts the captures themselves verify; do not infer from `INSTRUCTIONS.md` prose alone. After the CLI stamps `source`, the block validates against `schemas/discovery/lead.schema.json`.

Emit blocks sorted alphabetically by `lead` so re-survey produces byte-stable diffs.

## Algorithm

1. **Walk `tests/data/replays/`.** Survey immediate subdirectories. Skip `samples/` (shared payloads, not handlers) and any directory whose name begins with `.` or `_`.
2. **Per handler, inventory scenarios.** List `<handler>/*.json`. Skip the optional per-handler `INSTRUCTIONS.md` — the brief is not authoritative for surface naming. Zero-scenario handler directories are skipped silently (the operator drops them upstream).
3. **Identify the surface.** Inspect one or two scenario files to derive the route / topic / job identifier and method (e.g. `POST /users`, queue `user.created`, cron `0 */5 * * *`). When scenarios disagree, prefer the most common surface and note the spread in `synopsis`.
4. **Emit one lead block per handler.** Sort by `lead`. Each block carries the handler `lead` and a reconciliation-grade synopsis; the CLI stamps `source` from the survey binding.

## Path rules

Every internal reference to a capture path is relative under `$SOURCE_DIR`:

- No leading `/`, no Windows drive letter, no `..` segments.
- Resolves to a file under `$SOURCE_DIR`.
- Never walks outside `tests/data/replays/` for lead identification — sibling source trees are not the adapter's concern.

A symlink inside `$SOURCE_DIR` pointing outside the bound root is denied at canonicalization; the host runner returns `source-survey-path-denied` and the slice stays `refining`.

## Worked example

Bound directory (relative to `$SOURCE_DIR`):

```text
tests/data/replays/
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

Expected output (alphabetically by `lead`; the CLI stamps `source: runtime`):

```markdown
### password-reset

- lead: password-reset
- synopsis: POST /accounts/reset observed in 2 captures; both return 202 with no body.

### user-registration

- lead: user-registration
- synopsis: POST /users observed in 3 captures; happy path publishes `user.created`, error paths return 400 and 409.
```

## Determinism

- Emit leads sorted alphabetically by `lead`.
- Field order inside each block is fixed: `lead`, `synopsis`, then optional `topics` (slugs ordered deterministically).
- Quote concrete scenario counts and surface identifiers the captures verify; do not embed timestamps, host paths, or other run-state.
- Re-running against an unchanged capture tree produces byte-identical blocks.

## Anti-patterns

- **Inventing handlers from `INSTRUCTIONS.md`.** The prose is operator hint material; the directory listing is the lead source of truth. If a handler is named in `INSTRUCTIONS.md` but has no scenario JSON files, emit nothing for it.
- **Per-scenario leads.** One block per `<handler>/` directory, never one per `<scenario>.json`. Scenario-level detail belongs in `extract`'s `kind: example` claims.
- **Cross-source slug mismatches here.** When another source surfaces the same handler under a different slug, reconciliation is propose-time agent judgment and Gate 1 `--sources` edits; this brief sees one source's tree. See [From sources to slices](../references/spec-runtime/reconciliation.md#plan-time-leads-become-slices) for how leads reconcile into slices.
- **Writing `discovery.md` or `plan.yaml`.** Only lead blocks. The CLI owns every lifecycle file.

## Failure modes

| Condition | Action |
| --- | --- |
| `$SOURCE_DIR` empty or missing `tests/data/replays/` | Return zero leads. Operator reviews in `discovery.md`. |
| `tests/data/replays/<handler>/` contains no `*.json` files | Skip the handler silently. |
| Read denied outside `$SOURCE_DIR` | Host runner returns `source-survey-path-denied`; slice stays `refining`. |
| Capture JSON unparseable during surface identification | Continue with the remaining scenarios; surface ambiguity surfaces in the `synopsis` line. |

## References

- [Capture format reference](../references/capture-format.md) and [`extract.md`](extract.md) for `kind: example` Evidence claims
- [Capture format reference](../references/capture-format.md)
