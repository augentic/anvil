---
name: specify-plan
description: Plan a Specify change end-to-end — pre-flight, scaffold `change.md` and `plan.yaml`, survey each bound source, write `discovery.md`, reconcile leads into `slices[]` via the agent-driven `propose` sub-step, and exit at `pending` with the literal Gate-1 transition hint. Use when starting a fresh change from one or more bound sources (or pure intent); not when continuing an already-approved plan into execution (use `/spec:execute`).
argument-hint: <name> [source]...
---

# Plan Skill

`/spec:plan` is the single entry point for every Specify 2.0 change. It scaffolds `change.md` and `plan.yaml`, runs each bound source adapter's `survey` brief into `discovery.md`, reconciles the resulting leads into `slices[]` via the agent-driven `propose` sub-step, and exits at `pending`. The operator stamps Gate 1 by running the literal `specify plan transition <name> approved` command from step 8 — the skill never writes `approved` itself.

N=1 is degenerate, not special. A single intent binding produces one lead; `propose --from` writes one slice — one structured `{ source, lead }` binding under the auto-bound sole project. Multi-source planning differs only in step counts.

## Critical Path

1. **Pre-flight** — validate `<name>` as kebab-case (the CLI rejects malformed names). Read `.specify/project.yaml`. When `workspace: true`, rely on `specify plan create` to refuse if a workspace plan is already active from a project root; surface the structured error verbatim.
2. **Scaffold** — `specify plan create <name> --source <key>=<adapter>:<binding> ...` (repeatable per bound source). The CLI writes `change.md` and `plan.yaml` atomically with an empty `slices:` list — slice rows land later via `propose --from`. When the operator passes no `source` tokens, elicit a one-line intent and scaffold with `--source intent=intent:value:<elicited intent>`.
3. **Workspace sync** (workspace plans only) — `specify workspace sync` before survey. The CLI validates `registry.yaml` first; a malformed registry is a hard failure.
4. **Survey each source** — for every binding under `plan.yaml.sources.<key>`, run the two-phase `specify source survey <source>` handoff: `--phase prepare` resolves the bound source adapter, builds the sandbox (the bound `path` mounts read-only as the `SOURCE_DIR` preopen; `value:` bindings get no `SOURCE_DIR` preopen), emits `source.execution.agent`, and prints the handoff envelope. Execute the adapter's `survey` brief against that prepared sandbox, then `--phase finalize` validates the lead set and merges it under `## Lead inventory` in `discovery.md`; `tool`-execution adapters run the whole operation in a single call with no `--phase`. The merge is CLI-owned, so re-running `/spec:plan` replaces same-source blocks by `(source, lead)` while new sources append fresh ids.
5. **Write `discovery.md`** — the three-section form: `## Summary` (one-line counts), `## Source inventory` (one row per bound source), `## Lead inventory` (one block per lead). N=1 leaves `Summary` and `Source inventory` minimal. Template: [`../../references/discovery.md`](../../references/discovery.md).
6. **Propose** — reconcile leads into `slices[]` rows (see *Propose sub-step* below).
7. **Validate (Gate 1 optional)** — `specify plan validate --format json` before printing the closing hint when multi-slice or workspace plans need doctor output. Surface Error-level findings verbatim; Warnings are advisory.
8. **Exit at `pending`** — print this closing hint exactly. Do not call `specify plan transition`:

   ```text
   Plan `<name>` is at `pending`. Run `specify plan transition <name> approved` to stamp Gate 1, then `/spec:execute` to drive the slices.
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

The skill forwards every binding to `specify plan create --source <key>=<adapter>:<binding>`; the CLI canonicalises it into the structured `plan.yaml.sources.<key>: { adapter, path? | value? }` shape.

## Propose sub-step

Reconcile leads through the D2 envelope (see [`specify plan propose`](../../references/cli/plan-propose.md)):

1. **Dry-run** — `specify plan propose --dry-run --format json` returns the flat lead catalog and `projects[]`. Read-only — nothing is written and no journal event fires.
2. **Agent grouping** — match leads across sources by judgment from `synopsis`, shared slugs, and optional `aliases[]` hints (at most one lead per source per slice — never fuse two leads from the same source), then emit one `slices[]` row per slice, each carrying an explicit kebab-case `name`, its matched `sources[]`, and a bound `project`. There is no `scope` grouping noun — cross-target fan-out is multiple slices that may reference the same lead, joined by `depends-on`. Add `rationale` and `depends-on` as needed. The response shape is pinned by [`proposal.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/discovery/proposal.schema.json). **Split on doubt.** An over-merge is expensive and downstream-poisoning — two unrelated bodies of work land in one slice and one project/target, and `/spec:refine` synthesis inherits the bad match as `[conflict]`/divergence. An over-split is cheap and locally reversible at Gate 1 via `specify plan amend <entry> --sources`. So when a cross-source match is not well-supported by shared slug, alias, or synopsis, keep the leads in **separate** slices and surface the candidate pairing in `## Tentative merges` for the operator to confirm-by-merge — never gamble on an unrecoverable propose-time merge.
3. **Submit** — `specify plan propose --from <response.json> --reconcile-platforms` schema-gates the response against `proposal.schema.json`, validates it against a fresh catalog recomputed from `discovery.md`, replaces all `plan.yaml.slices[]` rows, each binding a `project` (the target adapter is resolved on demand from the project, not stored per slice), and runs the platform reconciliation pass. The `--reconcile-platforms` flag triggers a post-write pass that reads `project.yaml.platforms`, runs the vectis tool in detect mode to find declared-but-absent shells, and deterministically inserts bootstrap slices (e.g. `app-foundation` for greenfield, `bootstrap-<platform>` for incremental) with feature slices depending on them. The command emits a single `plan.reconcile.completed` journal event itself — the skill never runs `specify journal emit` for D2.
4. **Gate 1 review prose** — render cross-source merges into `change.md`. When reconciliation is uncertain, add `## Tentative merges` (never edit `discovery.md`). When merged summaries materially disagree, add `## Likely divergences` and invoke `specify plan amend <entry> --divergence likely` (after `propose --from`, the only slice writer) so the CLI stamps `slices[].divergence`.

Manual fallback: `specify plan add`, `specify plan amend <entry>`, and `specify plan remove` remain available for headless Gate 1 curation; the default flow uses `propose --from`. Use **re-propose** or **remove** for grouping and deferral; reserve **amend** for divergence stamps, authority overrides, and refine-time source binding fixes.

Authority hierarchy does not apply at propose — without `Evidence`, reconciliation runs on headlines alone. Authority activates at slice-time synthesis (`/spec:refine`).

## Guardrails

- **Single-writer for `plan.yaml`.** Slice rows land through `specify plan propose --from` (default) or `specify plan add` / `plan amend` / `plan remove` (manual Gate 1 fallback); `divergence: likely` rides on `plan amend --divergence likely`. The skill never reads-modifies-writes `plan.yaml` directly.
- **Single-driving-mode per project.** In workspace-registered projects, `/spec:plan` from a project root while a workspace plan is active is refused at `specify plan create`. Surface the CLI's structured error to the operator; do not retry from the workspace.
- **Never invent verbs.** Creation/amend paths fold schema validation into `specify plan add` / `plan amend`; validation after writes may call `specify plan validate` for structural/health diagnostics. Confirm the plan parses by re-reading `plan.yaml` after every write.
- **Never bypass the sandbox.** Source adapter `survey` briefs run with the bound `path` mounted read-only as the `SOURCE_DIR` preopen; the CLI denies access outside the granted preopens as `source-survey-path-denied`.

## References

| Reference | Purpose |
|---|---|
| [`../../references/discovery.md`](../../references/discovery.md) | Three-section form for `discovery.md`; minimal lead block; N=1 `intent` minimal form |
| [`fixtures/`](fixtures/) | Scenario goldens: pure-intent N=1, documentation multi-slice, cross-source propose merge, `plan.reconcile.*` and `plan.amend.divergence` journal events |
