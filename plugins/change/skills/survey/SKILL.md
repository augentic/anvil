---
name: change-survey
description: "Decompose legacy-code sources into slice-sized candidates by driving per-language enumeration briefs and validating the result against the closed `surfaces.json` schema, then sizing candidates and applying minimal same-source clustering. Use when `/change:draft` has recorded `legacy-code` sources and the pipeline needs a candidate inventory before `propose`, or when refreshing survey artifacts for an existing change."
argument-hint: <change-name>
---

# Survey skill

> **Decompose legacy-code sources into slice-sized candidates for `propose`.** `/change:survey` sits between workspace sync and propose in the `/change:draft` pipeline. For each `legacy-code` source it resolves the per-language enumeration brief at [`briefs/enumerate/<language>.md`](briefs/enumerate/), drives an LLM to produce a candidate `surfaces.json`, hands it to `specify change survey` for validation and canonical write, repairs on bounded validator failure, then sizes candidates and emits the inventory consumed by `propose`. The per-capability clustering briefs at [`briefs/omnia/cluster.md`](briefs/omnia/cluster.md) and [`briefs/vectis/cluster.md`](briefs/vectis/cluster.md) live on a separate axis (capability-axis clustering, not language-axis enumeration) and keep their current shape.

## Critical Path

1. **Enumerate sources** — read the change's recorded `legacy-code` sources from `change.md` / `plan.yaml`.
2. **Per source: drive enumeration brief** — detect language, resolve [`briefs/enumerate/<language>.md`](#brief-resolution), drive the LLM with that brief over the source root, write the candidate to `<staged-dir>/<source-key>.json`. The stage directory lives at `.specify/plans/<change>/survey/staged/`.
3. **Write sources file** — write `.specify/plans/<change>/survey/sources.yaml` (`version: 1`, one row per source with `key` + `path`). See [references/sources-yaml-shape.md](references/sources-yaml-shape.md).
4. **Invoke CLI (batch form)** — run `specify change survey --sources <file> --staged <staged-dir> --out .specify/plans/<change>/survey/`. The verb validates, canonicalizes, captures metadata, and writes `<source-key>/surfaces.json` + `<source-key>/metadata.json` per row.
5. **Bounded repair loop** — on `surfaces-validation-failed`, `surfaces-id-collision`, or `surfaces-touches-out-of-tree`, re-prompt the LLM with the structured validator output (budget 3 retries per source, each retry run with `--validate-only`). On exhaustion exit `surveyor-exhausted` and persist the last failing candidate plus the validator output under `.specify/plans/<change>/survey/staged/<source-key>.last-failure.json`. See [references/repair-loop.md](references/repair-loop.md).
6. **Read canonicalized sidecars** — load every `surfaces.json` and `metadata.json` written by the verb.
7. **Run candidate algorithm** — per source, apply sizing + surface candidates + minimal clustering. See [references/candidate-algorithm.md](references/candidate-algorithm.md).
8. **Write `survey.md`** — render `.specify/plans/<change>/survey.md` with the three required sections (`Summary`, `Source inventory`, `Candidate inventory`). Byte-stable.
9. **Append to discovery** — append candidate blocks under `## Candidate inventory` in `discovery.md`. Skip blocks already present (idempotent re-runs).

## Inputs

The change's recorded `legacy-code` sources — each a local path or materialized clone. The discovery brief's `## Candidate inventory` heading must already exist in `discovery.md`; this skill appends under it but never writes it.

## Outputs

- `.specify/plans/<change>/survey/staged/<source-key>.json` — staged candidate produced by the LLM.
- `.specify/plans/<change>/survey/sources.yaml` — batch input file for the CLI.
- `.specify/plans/<change>/survey/<source-key>/surfaces.json` — per-source surfaces (written by CLI).
- `.specify/plans/<change>/survey/<source-key>/metadata.json` — per-source metadata (written by CLI).
- `.specify/plans/<change>/survey.md` — combined candidate inventory.
- Appended candidate blocks under `## Candidate inventory` in `discovery.md`.

All outputs are byte-stable: fixed field order, sorted lists, no timestamps, no absolute paths.

## Brief resolution

For each `legacy-code` source the skill detects the source's `language` and resolves `plugins/change/skills/survey/briefs/enumerate/<language>.md`. JavaScript sources (`.js`, `.mjs`, `.cjs`, `.jsx`) resolve to `typescript.md` — the TypeScript brief covers both. Supported languages in v1, alphabetical: `cobol`, `csharp`, `javascript`, `rust`, `typescript`. When no brief matches the detected language, fail closed with `no enumeration brief for <language>; supported languages: cobol, csharp, javascript, rust, typescript` rather than degrading to a no-op.

## Candidate algorithm

See [references/candidate-algorithm.md](references/candidate-algorithm.md) for the full algorithm. Summary:

1. **Size check** — if a source's union-of-`touches` LOC < 1000, emit one source-level candidate covering every surface and stop for that source.
2. **Surface candidates** — otherwise, treat each surface as the default candidate; size it by its `touches` LOC.
3. **Minimal clustering** — merge same-source surfaces only when (a) shared `touches` overlap >= 50%, (b) documentation in `discovery.md` explicitly groups them, or (c) they share a handler / call site. Combined size must remain `acceptable` (< 1000 LOC).
4. **`too-large` post-cluster** — any candidate whose LOC >= 1000 after clustering is emitted with `unresolved: true`. Survey exits 0; `propose` is the gate.

## Sizing rules

Production LOC excludes tests, generated code, vendored deps, blank lines, and comment-only lines. v1 sizing uses a simple line count over touched files applying the same skip patterns as the CLI (`node_modules`, `vendor`, `target`, `.venv`, `*.gen.*`, `*.d.ts`, test directories). Per-file LOC from `metadata.json` is a deferred refinement.

| Size | Production LOC | Planning meaning |
|---|---|---|
| `acceptable` | `< 1000` | Slice-sized; emit as candidate. |
| `too-large` | `>= 1000` | Split or mark `unresolved: true`. |

## `declared-at` rendering

Single renderer: list sorted `<source-key>:<path>` or `<source-key>:<path>:<line>` entries from the underlying `surfaces[].declared-at` arrays. Entries are sorted alphabetically.

## `survey.md` shape

See [references/survey-md-shape.md](references/survey-md-shape.md) for the canonical example. Required sections in order:

1. `# <change> survey`
2. `## Summary` — one-line counts: source / surface / candidate / unresolved.
3. `## Source inventory` — one row per input source: source-key, path, language, LOC, surface count.
4. `## Candidate inventory` — one fenced-YAML block per candidate using the unified grammar.

## Discovery handshake

Append candidate blocks to the discovery-owned `## Candidate inventory` heading in `discovery.md` using the same fenced-YAML grammar as [`/change:analyze`](../analyze/SKILL.md). Survey-derived blocks always include `kind: candidate`. Skip blocks whose `### <name>` heading already exists in `discovery.md` (idempotent re-runs). The heading is written by the discovery brief; survey never re-emits it.

## Routing and brownfield

Survey-derived candidates do not carry `target-project`; assignment continues to use today's signals. Survey does not read `.specify/specs/` baselines — baseline projects are opaque routing targets.

## Determinism policy

The agent producer of `surfaces.json` is non-deterministic by construction; the artifact contract stays reproducible. Summary:

- **Schema-stable.** `surfaces.json` is validated against the closed schema before write; unknown `kind`, missing required fields, paths outside the source root, absolute paths, or duplicate `id` values fail the run.
- **Sort-stable.** The CLI sorts `surfaces[]` by `id` and sorts `touches[]` / `declared-at[]` alphabetically before write; the agent's output order does not affect the canonical form.
- **Idempotent on unchanged inputs.** Equivalent agent runs produce byte-identical canonical files even when the LLM phrasing differs in transit.
- **Pinned per-language brief.** Each enumeration brief lives at `plugins/change/skills/survey/briefs/enumerate/<language>.md`; brief changes are reviewable diffs.
- **Bounded repair loop.** v1 retry budget is 3 per source; on exhaustion the skill exits `surveyor-exhausted` (see [references/repair-loop.md](references/repair-loop.md)).

## Failure modes

On any non-zero exit from `specify change survey` other than the three repair-eligible codes, the skill does not write `survey.md` or append to `discovery.md`. The CLI error message is surfaced verbatim. `surveyor-exhausted` is emitted by the skill itself when the repair loop budget runs out.

| Discriminant | Skill action |
|---|---|
| `staged-input-missing` | Refuse to compose `survey.md`; surface CLI error verbatim. (skill / staging bug) |
| `staged-input-malformed` | Refuse to compose `survey.md`; surface CLI error verbatim. (skill bug) |
| `surfaces-validation-failed` | Enter repair loop (up to 3 retries); on exhaustion exit `surveyor-exhausted`. |
| `surfaces-id-collision` | Enter repair loop (up to 3 retries); on exhaustion exit `surveyor-exhausted`. |
| `surfaces-touches-out-of-tree` | Enter repair loop (up to 3 retries); on exhaustion exit `surveyor-exhausted`. |
| `source-path-missing` | Refuse to compose `survey.md`; surface CLI error verbatim. (operator bug) |
| `source-path-not-readable` | Refuse to compose `survey.md`; surface CLI error verbatim. (operator bug) |
| `source-key-mismatch` | Refuse to compose `survey.md`; surface CLI error verbatim. (skill bug) |
| `sources-file-missing` | Refuse to compose `survey.md`; surface CLI error verbatim. (skill bug) |
| `sources-file-malformed` | Refuse to compose `survey.md`; surface CLI error verbatim. (skill bug) |
| `surveyor-exhausted` | Skill-emitted: exit non-zero, persist last failing candidate + validator output under `.specify/plans/<change>/survey/staged/<source-key>.last-failure.json`. |

## Per-capability clustering briefs

The detailed clustering prompts live on the capability axis under `briefs/<capability>/cluster.md`:

- [`briefs/omnia/cluster.md`](briefs/omnia/cluster.md) — Omnia's source-local clustering refinements.
- [`briefs/vectis/cluster.md`](briefs/vectis/cluster.md) — Vectis's source-local clustering refinements.

The skill resolves the active capability via `specify capability resolve` and loads the relevant brief for capability-specific clustering signals. The global algorithm in [references/candidate-algorithm.md](references/candidate-algorithm.md) applies first; capability briefs refine within those bounds. These briefs are unchanged from earlier versions of the skill — they live on a separate axis from the per-language enumeration briefs that drive the agent producer.

## Reference Documentation

| Reference | Purpose |
|---|---|
| [references/candidate-algorithm.md](references/candidate-algorithm.md) | Full candidate algorithm: size check, surface candidates, minimal clustering, `unresolved` marking |
| [references/survey-md-shape.md](references/survey-md-shape.md) | Canonical `survey.md` shape with worked example |
| [references/sources-yaml-shape.md](references/sources-yaml-shape.md) | `--sources` YAML format and `--staged` pairing |
| [references/repair-loop.md](references/repair-loop.md) | Bounded retry contract: structured feedback, 3-retry budget, `surveyor-exhausted` exit |
| [`briefs/enumerate/`](briefs/enumerate/) | Per-language enumeration briefs (TypeScript / C# / Rust / COBOL) |
| [`briefs/`](briefs/) | Per-capability clustering briefs (`omnia/`, `vectis/`) |
| [`fixtures/`](fixtures/) | Acceptance fixtures: single-source / multi-source / unresolved / heading-handshake / greenfield / repair-loop and repair-loop-exhausted |
| [`../analyze/SKILL.md`](../analyze/SKILL.md) | Sibling skill for documentation-derived candidate blocks |
| [`rfcs/archive/rfc-20-survey.md`](../../../../rfcs/archive/rfc-20-survey.md) | Governing RFC |

## Guardrails

- **No LLM in the validator.** `specify change survey` is deterministic. Validation, canonicalization, and atomic writes are mechanical.
- **Bounded repair loop.** v1 retry budget is 3 per source. Exhaustion exits `surveyor-exhausted`.
- **Never write the `## Candidate inventory` heading.** The discovery brief owns it; survey only appends under it.
- **Never write `plan.yaml`.** Survey produces candidates; `propose` produces plan entries.
- **Never read `.specify/specs/` baselines.** Brownfield projects are opaque routing targets.
- **Byte-stable outputs.** No timestamps, absolute paths, or host-state in `surfaces.json`, `metadata.json`, `survey.md`, or `discovery.md`. Fixed field order, sorted lists.
- **Idempotent re-runs.** Skip candidate blocks whose heading already exists in `discovery.md`. Re-running on unchanged inputs produces byte-identical `survey.md`.
