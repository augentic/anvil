---
name: specify-plan
description: Plan a Specify change end-to-end — pre-flight, scaffold `change.md` and `plan.yaml`, survey each bound source, write `discovery.md`, reconcile leads into `slices[]` via the agent-driven `propose` sub-step, and exit at `pending` with the literal Gate-1 transition hint. Use when starting a fresh change from one or more bound sources (or pure intent); not when continuing an already-approved plan into execution (use `/spec:execute`).
argument-hint: <name> [source]...
---

# Plan Skill

`/spec:plan` is the single entry point for every Specify 2.0 change. It scaffolds `change.md` and `plan.yaml`, runs each bound source adapter's `survey` brief into `discovery.md`, reconciles the resulting leads into `slices[]` via the agent-driven `propose` sub-step, and exits at `pending`. The operator stamps Gate 1 by running the literal `specrun plan transition <name> approved` command from step 8 — the skill never writes `approved` itself.

N=1 is degenerate, not special. A single intent binding produces one lead; `propose --from` writes one slice — one structured `{ source-key, lead-id }` binding under the auto-bound sole project. Multi-source planning differs only in step counts.

## Critical Path

1. **Pre-flight** — validate `<name>` as kebab-case (the CLI rejects malformed names). Read `.specify/project.yaml`. When `workspace: true`, rely on `specrun plan create` to refuse if a workspace plan is already active from a project root; surface the structured error verbatim.
2. **Scaffold** — `specrun plan create <name> --source <key>=<adapter>:<binding> ...` (repeatable per bound source). The CLI writes `change.md` and `plan.yaml` atomically with an empty `slices:` list — slice rows land later via `propose --from`. When the operator passes no `source` tokens, elicit a one-line intent and scaffold with `--source intent=intent:value:<elicited intent>`.
3. **Workspace sync** (workspace plans only) — `specrun workspace sync` before survey. The CLI validates `registry.yaml` first; a malformed registry is a hard failure.
4. **Survey each source** — for every binding under `plan.yaml.sources.<key>`, run the two-phase `specrun source survey <source-key>` handoff: `--phase prepare` resolves the bound source adapter, builds the sandbox (the bound `path` mounts read-only as the `SOURCE_DIR` preopen; `value:` bindings get no `SOURCE_DIR` preopen), emits `source.execution.agent`, and prints the handoff envelope. Execute the adapter's `survey` brief against that prepared sandbox, then `--phase finalize` validates the lead set and merges it under `## Lead inventory` in `discovery.md`; `tool`-execution adapters run the whole operation in a single call with no `--phase`. The merge is CLI-owned, so re-running `/spec:plan` replaces same-source ids and preserves operator aliases while new sources append fresh ids.
5. **Write `discovery.md`** — the three-section form: `## Summary` (one-line counts), `## Source inventory` (one row per bound source), `## Lead inventory` (one block per lead). N=1 leaves `Summary` and `Source inventory` minimal. Template: [`../../references/discovery.md`](../../references/discovery.md).
6. **Propose** — reconcile leads into `slices[]` rows (see *Propose sub-step* below).
7. **Validate (Gate 1 optional)** — `specrun plan validate --format json` before printing the closing hint when multi-slice or workspace plans need doctor output. Surface Error-level findings verbatim; Warnings are advisory.
8. **Exit at `pending`** — print this closing hint exactly. Do not call `specrun plan transition`:

   ```text
   Plan `<name>` is at `pending`. Run `specrun plan transition <name> approved` to stamp Gate 1, then `/spec:execute` to drive the slices.
   ```

## Source binding grammar

The operator appends zero or more `source <key>=<adapter>:<binding>` positionals after the change name:

```text
/spec:plan <name> source <key>=<adapter>:<path> [source <key>=<adapter>:value:<literal> ...]
```

- `<key>` is a kebab-case identifier used as the slot in `plan.yaml.sources.<key>`.
- `<adapter>` is the kebab-case name of the bound source adapter (e.g. `intent`, `documentation`, `code-typescript`, `screenshots`).
- `<binding>` is either a path (e.g. `documentation:./design-notes/identity`, `code-typescript:./vendor/legacy-monolith`) or the literal form `value:<literal>` (e.g. `intent:value:fix typo in user.rs`). Exactly one of `path` or `value` is required per binding.
- When the operator passes no `source` tokens, elicit a one-line intent in step 2 and scaffold with `--source intent=intent:value:<elicited intent>`. Slice rows are written by `propose --from`, not at create time.

The skill forwards every binding to `specrun plan create --source <key>=<adapter>:<binding>`; the CLI canonicalises it into the structured `plan.yaml.sources.<key>: { adapter, path? | value? }` shape.

## Propose sub-step

Reconcile leads through the D2 envelope (see [`specrun plan propose`](../../../../docs/reference/cli/plan.md#specrun-plan-propose)):

1. **Dry-run** — `specrun plan propose --dry-run --format json` returns the flat lead catalog and `projects[]`. Read-only — nothing is written and no journal event fires.
2. **Agent grouping** — match leads across sources by judgment from `summary`, shared slugs, and optional `aliases[]` hints (at most one lead per source per scope — never fuse two leads from the same source), then emit one `slices[]` row per `(scope, project)` pair carrying a `scope` id, its matched `sources[]`, and a bound `project`. Fan-out repeats the `scope` id and identical `sources[]`. Add `rationale`, `depends-on`, and optional `name` as needed. The response shape is pinned by [`proposal.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/discovery/proposal.schema.json). **Split on doubt.** An over-merge is expensive and downstream-poisoning — two unrelated bodies of work land in one slice and one project/target, and `/spec:refine` synthesis inherits the bad match as `[conflict]`/divergence. An over-split is cheap and locally reversible at Gate 1 via `specrun plan amend <entry> --sources`. So when a cross-source match is not well-supported by shared slug, alias, or summary, keep the leads in **separate** scopes and surface the candidate pairing in `## Tentative merges` for the operator to confirm-by-merge — never gamble on an unrecoverable propose-time merge.
3. **Submit** — `specrun plan propose --from <response.json>` schema-gates the response against `proposal.schema.json`, validates it against a fresh catalog recomputed from `discovery.md`, replaces all `plan.yaml.slices[]` rows, and derives each slice's `target` from the bound project. The command emits the paired `plan.reconcile.agent` + `plan.reconcile.completed` journal events itself, in one atomic batch — the skill never runs `specrun journal emit` for D2.
4. **Gate 1 review prose** — render cross-source merges into `change.md`. When reconciliation is uncertain, add `## Tentative merges` (never edit `discovery.md`). When merged summaries materially disagree, add `## Likely divergences` and invoke `specrun plan amend <entry> --divergence likely` (after `propose --from`, the only slice writer) so the CLI stamps `slices[].divergence`.

Manual fallback: `specrun plan add`, `specrun plan amend <entry>`, and `specrun plan remove` remain available for headless Gate 1 curation; the default flow uses `propose --from`. Use **re-propose** or **remove** for grouping and deferral; reserve **amend** for divergence stamps, authority overrides, and refine-time source binding fixes.

Authority hierarchy does not apply at propose — without `Evidence`, reconciliation runs on headlines alone. Authority activates at slice-time synthesis (`/spec:refine`).

## Guardrails

- **Single-writer for `plan.yaml`.** Slice rows land through `specrun plan propose --from` (default) or `specrun plan add` / `plan amend` / `plan remove` (manual Gate 1 fallback); `divergence: likely` rides on `plan amend --divergence likely`. The skill never reads-modifies-writes `plan.yaml` directly.
- **Single-driving-mode per project.** In workspace-registered projects, `/spec:plan` from a project root while a workspace plan is active is refused at `specrun plan create`. Surface the CLI's structured error to the operator; do not retry from the workspace root.
- **Never invent verbs.** Creation/amend paths fold schema validation into `specrun plan add` / `plan amend`; validation after writes may call `specrun plan validate` for structural/health diagnostics. Confirm the plan parses by re-reading `plan.yaml` after every write.
- **Never bypass the sandbox.** Source adapter `survey` briefs run with the bound `path` mounted read-only as the `SOURCE_DIR` preopen; the CLI denies access outside the granted preopens as `source-survey-path-denied`.

## References

| Reference | Purpose |
|---|---|
| [`../../references/discovery.md`](../../references/discovery.md) | Three-section form for `discovery.md`; minimal lead block; N=1 `intent` minimal form |
| [`fixtures/`](fixtures/) | Scenario goldens: pure-intent N=1, documentation multi-slice, cross-source propose merge, `plan.reconcile.*` and `plan.amend.divergence` journal events |
