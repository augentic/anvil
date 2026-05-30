---
name: specify-plan
description: Plan a Specify change end-to-end — pre-flight, scaffold `change.md` and `plan.yaml`, survey each bound source, write `discovery.md`, reconcile leads into `slices[]` via the agent-driven `propose` sub-step, and exit at `pending` with the literal Gate-1 transition hint. Use when starting a fresh change from one or more bound sources (or pure intent); not when continuing an already-approved plan into execution (use `/spec:execute`).
argument-hint: <name> [source]...
---

# Plan Skill

`/spec:plan` is the single entry point for every Specify 2.0 change. It scaffolds `change.md` and `plan.yaml`, runs each bound source adapter's `survey` brief into `discovery.md`, reconciles the resulting leads into `slices[]` via the agent-driven `propose` sub-step, and exits at `pending`. The operator stamps Gate 1 by running the literal `specrun plan transition <name> approved` command from step 8 — the skill never writes `approved` itself.

N=1 is degenerate, not special. A single intent binding produces one lead, one slice with `sources: [intent]` shorthand, and the same Gate-1 hint. Multi-source planning differs only in step counts.

## Critical Path

1. **Pre-flight** — validate `<name>` as kebab-case (the CLI rejects malformed names). Read `.specify/project.yaml`. When `workspace: true`, rely on `specrun plan create` to refuse if a workspace plan is already active from a project root; surface the structured error verbatim.
2. **Scaffold** — `specrun plan create <name> --source <key>=<adapter>:<binding> ...` (repeatable per bound source). The CLI writes `change.md` and `plan.yaml` atomically. When the operator passes no `source` tokens, elicit a one-line intent and scaffold with `--source intent=intent:value:<elicited intent>`.
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
- When the operator passes no `source` tokens, elicit a one-line intent in step 2 and scaffold with `--source intent=intent:value:<elicited intent>`. The scaffolded slice carries `sources: [intent]` (shorthand for `{ key: intent, lead: <slice.name> }`).

The skill forwards every binding to `specrun plan create --source <key>=<adapter>:<binding>`; the CLI canonicalises it into the structured `plan.yaml.sources.<key>: { adapter, path? | value? }` shape.

## Propose sub-step

The agent reads the full `## Lead inventory` in `discovery.md`, matches leads across sources by `id`, `summary`, and `sources[]`, and writes one `slices[]` row per unit of work:

1. **Write the slice row** — `specrun plan add <slice> --sources <key>=<lead-id> ...`. Pass one `--sources` argument per contributing source. In workspace plans, also pass `--project <project>` to route the slice to its slot. The CLI writes the structured `{ key, lead }[]` shape; single-source intent slices may emit the bare `[intent]` shorthand.
2. **Tentative annotations** — when reconciliation is uncertain (leads share intent but differ in scope), annotate the contributing blocks in `discovery.md` with a `tentative: true` bullet, and add a `## Tentative merges` block to `change.md` with one paragraph of reasoning per uncertain reconciliation. The plan still progresses to `pending`; the operator overrides via `specrun plan amend` at Gate 1.
3. **`divergence: likely`** — when merged leads' `summary` strings materially disagree (different numeric values, conflicting verbs, mutually exclusive nouns), invoke `specrun plan amend <name> <slice> --divergence likely` for each affected slice. The CLI is the single writer of `plan.yaml.slices[].divergence` (any value) and fires `plan.amend.divergence` once per invocation. Also add a `## Likely divergences` block to `change.md` listing the contributing lead-pair summaries side by side; that operator-facing prose is still authored by the skill.

Authority hierarchy does not apply at propose — without `Evidence`, reconciliation runs on headlines alone. Authority activates at slice-time synthesis (`/spec:refine`).

## Guardrails

- **Single-writer for `plan.yaml`.** Every value in `plan.yaml` lands through a `specrun plan create` / `plan add` / `plan amend` call; `divergence: likely` rides on `plan amend --divergence likely`. The skill never reads-modifies-writes `plan.yaml` directly.
- **Single-driving-mode per project.** In workspace-registered projects, `/spec:plan` from a project root while a workspace plan is active is refused at `specrun plan create`. Surface the CLI's structured error to the operator; do not retry from the workspace root.
- **Never invent verbs.** Creation/amend paths fold schema validation into `specrun plan add` / `plan amend`; validation after writes may call `specrun plan validate` for structural/health diagnostics. Confirm the plan parses by re-reading `plan.yaml` after every write.
- **Never bypass the sandbox.** Source adapter `survey` briefs run with the bound `path` mounted read-only as the `SOURCE_DIR` preopen; the CLI denies access outside the granted preopens as `source-survey-path-denied`.

## References

| Reference | Purpose |
|---|---|
| [`../../references/discovery.md`](../../references/discovery.md) | Three-section form for `discovery.md`; minimal lead block; N=1 `intent` minimal form |
| [`fixtures/`](fixtures/) | Scenario goldens: pure-intent N=1, documentation multi-slice, cross-source propose merge, `plan.amend.divergence` journal event |
