---
name: change-survey
description: "Decompose legacy-code sources into slice-sized candidates by mechanically scanning surfaces, sizing code footprints, and applying minimal same-source clustering. Use when `/change:draft` has recorded `legacy-code` sources and the pipeline needs a candidate inventory before `propose`, or when refreshing survey artifacts for an existing change."
argument-hint: <change-name>
---

# Survey skill

> **Mechanically decompose legacy-code sources into slice-sized candidates for `propose`.** `/change:survey` sits between workspace sync and propose in the `/change:draft` pipeline. It invokes `specify change survey` once per change, then reads the structural evidence, sizes candidates, applies minimal same-source clustering, and writes the candidate inventory consumed by `propose`.

## Critical Path

1. **Enumerate sources** — read the change's recorded `legacy-code` sources from `change.md` / `plan.yaml`.
2. **Write sources file** — write `.specify/plans/<change>/survey/sources.yaml` with `version: 1` and one row per source.
3. **Invoke CLI** — `specify change survey --sources <file> --out .specify/plans/<change>/survey/`. The CLI writes `<source-key>/surfaces.json` + `<source-key>/metadata.json` per row.
4. **Read sidecars** — load every `surfaces.json` and `metadata.json` written by the verb.
5. **Run candidate algorithm** — per source, apply sizing + surface candidates + minimal clustering. See [references/candidate-algorithm.md](references/candidate-algorithm.md).
6. **Write `survey.md`** — render `.specify/plans/<change>/survey.md` with the three required sections (`Summary`, `Source inventory`, `Candidate inventory`). Byte-stable.
7. **Append to discovery** — append candidate blocks under the `## Candidate inventory` heading in `discovery.md`. Skip blocks already present (idempotent re-runs).

## Inputs

The change's recorded `legacy-code` sources — each a local path or materialized clone. The discovery brief's `## Candidate inventory` heading must already exist in `discovery.md`; this skill appends under it but never writes it.

## Outputs

- `.specify/plans/<change>/survey/<source-key>/surfaces.json` — per-source surfaces (written by CLI).
- `.specify/plans/<change>/survey/<source-key>/metadata.json` — per-source metadata (written by CLI).
- `.specify/plans/<change>/survey/sources.yaml` — batch input file for CLI.
- `.specify/plans/<change>/survey.md` — combined candidate inventory.
- Appended candidate blocks under `## Candidate inventory` in `discovery.md`.

All outputs are byte-stable: fixed field order, sorted lists, no timestamps, no absolute paths.

## Candidate algorithm

See [references/candidate-algorithm.md](references/candidate-algorithm.md) for the full algorithm. Summary:

1. **Size check** — if a source's union-of-`touches` LOC < 1000, emit one source-level candidate covering every surface and stop for that source.
2. **Surface candidates** — otherwise, treat each surface as the default candidate; size it by its `touches` LOC.
3. **Minimal clustering** — merge same-source surfaces only when: (a) shared `touches` overlap >= 50%, OR (b) documentation in `discovery.md` explicitly groups them, OR (c) they share a handler / call site. Combined size must remain `acceptable` (< 1000 LOC).
4. **`too-large` post-cluster** — any candidate whose LOC >= 1000 after clustering is emitted with `unresolved: true`. Survey exits 0; `propose` is the gate.

## Sizing rules

Production LOC excludes tests, generated code, vendored deps, blank lines, and comment-only lines. v1 sizing uses a simple line count over touched files applying the same skip patterns as the CLI (`node_modules`, `vendor`, `target`, `.venv`, `*.gen.*`, `*.d.ts`, test directories). Per-file LOC from `metadata.json` is a deferred refinement.

| Size | Production LOC | Planning meaning |
|---|---|---|
| `acceptable` | `< 1000` | Slice-sized; emit as candidate. |
| `too-large` | `>= 1000` | Split or mark `unresolved: true`. |

## `declared-at` rendering

Single renderer: list sorted `<source-key>:<path>` or `<source-key>:<path>:<line>` entries from the underlying `surfaces[].declared-at` arrays. No prose, no detector-hand-written text. Entries are sorted alphabetically.

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

## Failure modes

`specify change survey` exit discriminants and skill response:

| Discriminant | Skill action |
|---|---|
| `no-detectors` | Refuse to compose `survey.md`; surface CLI error verbatim. |
| `detector-id-collision` | Refuse to compose `survey.md`; surface CLI error verbatim. |
| `source-path-missing` | Refuse to compose `survey.md`; surface CLI error verbatim. |
| `source-path-not-readable` | Refuse to compose `survey.md`; surface CLI error verbatim. |
| `detector-failure` | Refuse to compose `survey.md`; surface CLI error verbatim. |
| `sources-file-missing` | Refuse to compose `survey.md`; surface CLI error verbatim. |
| `sources-file-malformed` | Refuse to compose `survey.md`; surface CLI error verbatim. |
| `source-key-mismatch` | Refuse to compose `survey.md`; surface CLI error verbatim. |

On any non-zero exit, the skill does not write `survey.md` or append to `discovery.md`. The CLI error message is surfaced verbatim to the operator.

## Per-capability clustering briefs

The detailed clustering prompts live under `briefs/<capability>/cluster.md`:

- [`briefs/omnia/cluster.md`](briefs/omnia/cluster.md) — Omnia's source-local clustering refinements.
- [`briefs/vectis/cluster.md`](briefs/vectis/cluster.md) — Vectis's source-local clustering refinements.

The skill resolves the active capability via `specify capability resolve` and loads the relevant brief for capability-specific clustering signals. The global algorithm in [references/candidate-algorithm.md](references/candidate-algorithm.md) applies first; capability briefs refine within those bounds.

## Reference Documentation

| Reference | Purpose |
|---|---|
| [references/candidate-algorithm.md](references/candidate-algorithm.md) | Full candidate algorithm: size check, surface candidates, minimal clustering, `unresolved` marking |
| [references/survey-md-shape.md](references/survey-md-shape.md) | Canonical `survey.md` shape with worked example |
| [`briefs/`](briefs/) | Per-capability clustering briefs (`omnia/`, `vectis/`) |
| [`fixtures/`](fixtures/) | Heading-handshake fixture |
| [`../analyze/SKILL.md`](../analyze/SKILL.md) | Sibling skill for documentation-derived candidate blocks |
| [`rfcs/rfc-20-survey.md`](../../../../rfcs/rfc-20-survey.md) | Governing RFC |

## Guardrails

- **No LLM in the scanner.** `specify change survey` is mechanical only. The skill never calls an LLM for surface detection.
- **Never write the `## Candidate inventory` heading.** The discovery brief owns it; survey only appends under it.
- **Never write `plan.yaml`.** Survey produces candidates; `propose` produces plan entries.
- **Never read `.specify/specs/` baselines.** Brownfield projects are opaque routing targets.
- **Byte-stable outputs.** No timestamps, absolute paths, or host-state in `surfaces.json`, `metadata.json`, `survey.md`, or `discovery.md`. Fixed field order, sorted lists.
- **Idempotent re-runs.** Skip candidate blocks whose heading already exists in `discovery.md`. Re-running on unchanged inputs produces byte-identical `survey.md`.
