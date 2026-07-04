# Shared guardrails

Cross-cutting "do not / never / always" rules that apply across many skills. Skills should **link** to the relevant section here rather than restating these rules verbatim in their `SKILL.md` body. Contributor house style for skill bodies lives in the published [Skill authoring guide](https://specify.augentic.io/standards/skill-authoring.html).

Per-skill guardrails — rules that only make sense for one skill ("never auto-promote a `component:` slug", "never invent cost figures", etc.) — stay in the owning `SKILL.md`. Lift to this file only when 3+ skills repeat the same rule.

## Single-writer for lifecycle state

The CLI is the **only** writer for change and slice lifecycle state. Skills route every write through a CLI verb; they never edit the underlying files by hand.

- **Never hand-edit `plan.yaml`.** Append entries through `specify plan add`; transition entries through `specify plan transition`; close out the plan through `specify plan archive`. The single-writer contract lives in the [`plan` skill](https://specify.augentic.io/reference/change-skills/plan.html).
- **Never hand-edit `.specify/slices/<name>/metadata.yaml`.** Status transitions and timestamp writes go through `specify slice transition`. The CLI enforces the legal lifecycle edges — skills do not need to track them.
- **Never hand-edit `.specify/archive/`.** Archive moves are atomic operations performed by `specify slice merge`, `specify slice transition <name> dropped`, and `specify plan archive`.
- **Never hand-roll `AGENTS.md` during init.** `specify init` generates it when absent, preserves an existing root `AGENTS.md`, and writes `.specify/context.lock` as the generation fingerprint.

## Baseline immutability for contract authoring

Contract authoring skills (OpenAPI, AsyncAPI, JSON Schema) write only inside the active slice directory. The shared baseline is read-only to authoring; merge into the baseline is a separate, explicit step.

- **Do not modify any file outside `$SLICE_DIR/contracts/`** (or `$SLICE_DIR/contracts/schemas/` for the JSON Schema skill).
- **Never modify baseline files in root `contracts/`.** All authored output lands in the slice-local `contracts/` directory; merging into the baseline is `specify slice merge run`'s job.
- **Never silently delete or narrow a baseline schema's fields.** If the spec requires it, surface the slice as a warning and let a human operator decide whether to bump the schema's `$id`.

## Consumer tooling boundary

During slice **execute / build / merge**, agents are **consumers** of Specify and adapters — not maintainers.

- **Do not** edit `specify`, `specify-adapters`, adapter templates, `adapter.wasm`, or `~/.cache/specify/**` to unblock a failing build.
- **Do not** run `specify adapter build`, `sync *-scaffold`, or rebuild/copy WASM to work around verify drift during consumer execute (sync/scaffold remain orchestrator-owned per the Vectis build brief; this rule blocks *agent-initiated* upstream patching).
- On scaffold, verify, finalize, or toolchain failure: **stop**, print CLI `stop:` / `hint:` / `resume:` output, and exit. Tooling fixes belong in specify / specify-adapters in a separate maintainer session.
- Canonical "stop, don't patch" example: Vectis [Template / version-pin drift handling](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/briefs/build.md#template--version-pin-drift-handling) (`adapters/targets/vectis/briefs/build.md` in consumer projects).
