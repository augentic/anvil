---
name: specify-plan
description: Plan a Specify change end-to-end — pre-flight, scaffold `change.md` and `plan.yaml`, enumerate each bound source, write `discovery.md`, fuse candidates into `slices[]` via the agent-driven `propose` sub-step, and exit at `pending` with the literal Gate-1 transition hint. Use when starting a fresh change from one or more bound sources (or pure intent); not when continuing an already-reviewed plan into execution (use `/spec:execute`).
argument-hint: <name> [source]...
---

# Plan Skill

`/spec:plan` is the single entry point for every Specify 2.0 change. It scaffolds `change.md` and `plan.yaml`, runs each bound source adapter's `enumerate` brief into `discovery.md`, fuses the resulting candidates into `slices[]` via the agent-driven `propose` sub-step, and exits at `pending`. The operator stamps Gate 1 by running the literal `specify plan transition <name> reviewed` command from the closing hint — the skill never writes `reviewed` itself.

N=1 is degenerate, not special. A single intent binding produces one candidate, one slice with `sources: [intent]` shorthand, and the same Gate-1 hint. Multi-source planning differs only in step counts.

## Critical Path

1. **Pre-flight** — validate `<name>` as kebab-case (the CLI rejects malformed names). Read `.specify/project.yaml`. When `workspace: true`, rely on `specify plan create` to refuse if a workspace plan is already active from a project root; surface the structured error verbatim.
2. **Scaffold** — `specify plan create <name> --source <key>=<value> ...` (repeatable per bound source). The CLI writes `change.md` and `plan.yaml` atomically. When the operator passes no `source` tokens, elicit a one-line intent and scaffold with `--source intent="<elicited intent>"`.
3. **Workspace sync** (workspace plans only) — `specify workspace sync` before enumerate. The CLI validates `registry.yaml` first; a malformed registry is a hard failure.
4. **Enumerate each source** — for every binding under `plan.yaml.sources.<key>`, run `specify source resolve <adapter>` to locate the adapter root and the `briefs/enumerate.md` path, then execute the brief; the CLI exposes the bound `path` as the read-only `SOURCE_DIR` WASI preopen (bindings carrying `value:` get no `SOURCE_DIR` preopen). Append each emitted candidate block under `## Candidate inventory` in `discovery.md`. Re-running `/spec:plan` replaces same-source ids; new sources append fresh ids.
5. **Write `discovery.md`** — the three-section form: `## Summary` (one-line counts), `## Source inventory` (one row per bound source), `## Candidate inventory` (one block per candidate). N=1 leaves `Summary` and `Source inventory` minimal. Template: [`../../references/discovery.md`](../../references/discovery.md).
6. **Propose** — fuse candidates into `slices[]` rows (see *Propose sub-step* below).
7. **Exit at `pending`** — print the closing hint exactly (see *Closing hint* below). Do not call `specify plan transition`.

## Source binding grammar

The operator appends zero or more `source <key>=<value>` positionals after the change name:

```text
/spec:plan <name> source <key>=<value> [source <key>=<value> ...]
```

- `<key>` is a kebab-case identifier used as the slot in `plan.yaml.sources.<key>`.
- `<value>` is a path (e.g. `./design-notes/identity`), a kebab-style adapter binding (e.g. `legacy=./vendor/legacy-monolith`), or a literal string for `intent` (e.g. `"fix typo in user.rs"`).
- When the operator passes no `source` tokens, elicit a one-line intent in step 2 and scaffold with `--source intent="<elicited intent>"`. The scaffolded slice carries `sources: [intent]` (shorthand for `{ key: intent, candidate: <slice.name> }`).

The skill forwards every binding to `specify plan create --source <key>=<value>`; the CLI canonicalises it into `plan.yaml.sources.<key>`.

## Propose sub-step

The agent reads the full `## Candidate inventory` in `discovery.md`, matches candidates across sources by `id`, `summary`, and `sources[]`, and writes one `slices[]` row per unit of work:

1. **Write the slice row** — `specify plan add <slice> --sources <key>=<candidate-id> ...`. Pass one `--sources` argument per contributing source. In workspace plans, also pass `--project <project>` to route the slice to its slot. The CLI writes the structured `{ key, candidate }[]` shape; single-source intent slices may emit the bare `[intent]` shorthand.
2. **Tentative annotations** — when fusion is uncertain (candidates share intent but differ in scope), annotate the contributing blocks in `discovery.md` with a `tentative: true` bullet, and add a `## Tentative merges` block to `change.md` with one paragraph of reasoning per uncertain fusion. The plan still progresses to `pending`; the operator overrides via `specify plan amend` at Gate 1.
3. **`divergence: likely`** — when merged candidates' `summary` strings materially disagree (different numeric values, conflicting verbs, mutually exclusive nouns), invoke `specify plan amend <name> <slice> --divergence likely` for each affected slice. The CLI is the single writer of `plan.yaml.slices[].divergence` (any value) and fires `plan.amend.divergence` once per invocation. Also add a `## Likely divergences` block to `change.md` listing the contributing candidate-pair summaries side by side; that operator-facing prose is still authored by the skill.

Authority hierarchy does not apply at propose — without `Evidence`, fusion runs on headlines alone. Authority activates at slice-time synthesis (`/spec:refine`).

## Closing hint

The skill exits by printing the literal transition command, followed by next-step orientation:

```text
Plan `<name>` is at `pending`. Run `specify plan transition <name> reviewed` to stamp Gate 1, then `/spec:execute` to drive the slices.
```

`/spec:plan` never auto-stamps `reviewed`. Re-running `/spec:plan <name>` re-enumerates every bound source: same-source candidate ids replace in place, new sources append.

## Guardrails

- **Single-writer for `plan.yaml`.** Every value in `plan.yaml` lands through a `specify plan create` / `plan add` / `plan amend` call; `divergence: likely` rides on `plan amend --divergence likely`. The skill never reads-modifies-writes `plan.yaml` directly.
- **Single-driving-mode per project.** In workspace-registered projects, `/spec:plan` from a project root while a workspace plan is active is refused at `specify plan create`. Surface the CLI's structured error to the operator; do not retry from the workspace root.
- **Never auto-stamp `reviewed`.** The closing hint is the only place the operator sees the literal transition command; `/spec:plan` never invokes `specify plan transition`.
- **Never invent verbs.** Validation is folded into `specify plan add` / `plan amend`; there is no `specify plan validate`. Confirm the plan parses by re-reading `plan.yaml` after every write.
- **Never bypass the sandbox.** Source adapter `enumerate` briefs run with the bound `path` mounted read-only as the `SOURCE_DIR` preopen; the CLI denies access outside the granted preopens as `source-enumerate-path-denied`.

## References

| Reference | Purpose |
|---|---|
| [`../../references/discovery.md`](../../references/discovery.md) | Three-section form for `discovery.md`; minimal candidate block; N=1 `intent` minimal form |
| [`fixtures/`](fixtures/) | Scenario goldens: pure-intent N=1, documentation multi-slice, cross-source propose merge, `plan.amend.divergence` journal event |
