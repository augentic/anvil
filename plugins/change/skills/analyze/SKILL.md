---
name: change-analyze
description: Extract plan-time candidate hints from documentation inputs and emit them into `discovery.md` — not full specs. Use when the plan-time discovery brief needs a capability-level inventory of a documentation source before `propose` slices it.
argument-hint: <input-path> <output-dir> [source-key]
---

# Analyze Skill

## Critical Path

1. **Validate invocation** — require a local `$INPUT_PATH` and writable `$OUTPUT_DIR`; fail before partial writes.
2. **Materialize remotes outside analyze** — if the source is remote, use the guarded clone snippet first and pass the resulting local path as `$INPUT_PATH`.
3. **Resolve capability prompt** — run capability resolution and load `plugins/change/skills/draft/briefs/<capability>/analyze.md`; never embed clustering heuristics in this SKILL.
4. **Emit candidate blocks only** — append fenced-YAML candidate blocks under the pre-existing `## Candidate inventory` heading in `<output-dir>/discovery.md`. Do not write the heading itself; the discovery brief owns it.
5. **Tag and deduplicate** — carry `<source-key>` markers when supplied, overwrite same-name candidates from this run, and preserve unrelated prior candidate blocks.
6. **Preserve idempotency** — keep field order fixed, sort lists, reject malformed brief output, and prevent timestamps, absolute paths, or host state from leaking into outputs.

`/change:analyze` is the sole plan-time documentation analysis skill. It reads one documentation input — architecture notes, API docs, runbooks, or other prose — and appends **candidate blocks** under the `## Candidate inventory` heading in `<output-dir>/discovery.md`. It does **not** write the heading itself; the discovery brief writes it exactly once before either analyze or survey runs. It does **not** produce full `specs/` + `design.md`; deep per-slice extraction remains `/spec:extract`'s job at define time.

`$INPUT_PATH` is a filesystem path to a documentation bundle. `$OUTPUT_DIR` is the plan working directory (`.specify/plans/<change-name>/` when called from the discovery brief); the skill appends candidate blocks to `discovery.md` under it. `$SOURCE_KEY` is optional; when supplied, the discovery brief uses it to tag this run for a specific top-level plan source.

### Cloning a source tree

`/change:analyze` only consumes local paths. When a `source <key>=<url>` (or any caller) needs to materialise a remote git URL into `$INPUT_PATH` first, clone it into a fresh temporary directory and pass that local path as `$INPUT_PATH`:

```bash
git clone "$url" "$dest"
```

Pass the resulting local path as `$INPUT_PATH` on the next `/change:analyze` invocation.

## Output contract

Each run appends candidate blocks under the pre-existing `## Candidate inventory` heading in `<output-dir>/discovery.md`. The heading is written once by the discovery brief before analyze runs; analyze never re-emits it.

Each candidate block is a fenced-YAML block following a Markdown sub-heading. Fields appear in fixed order so re-runs diff cleanly:

````markdown
### user-registration

```yaml
kind: candidate
sources: [architecture-notes]
surfaces:
  - architecture-notes:http-post-users
  - architecture-notes:message-pub-user-created
declared-at:
  - architecture-notes:design.md#user-registration
```
````

Fields, in fixed order:

- **`kind`** — always `candidate`. Consumers identify terminal leaves by `kind == "candidate"`.
- **`sources`** — list of source keys this candidate was inferred from. For documentation inputs, this is typically a single documentation source key.
- **`handler`** — optional. The handler or call site for the candidate's primary surface. Omit when no hint applies (common for documentation-derived blocks).
- **`touches`** — optional. Source files reached from the handler. Omit when no hint applies (common for documentation-derived blocks).
- **`surfaces`** — list of observable surfaces this candidate covers, namespaced `<source-key>:<surface-id>`.
- **`declared-at`** — list of paths (or `path:line` / `path#fragment` references) where the candidate is described or declared. Non-empty; sorted alphabetically. For documentation inputs, these are artifact paths with optional fragment references.
- **`unresolved`** — optional boolean. When `true`, the candidate cannot be split further and requires operator review during `propose`.

Doc-derived candidate blocks typically omit `handler` and `touches` because documentation inputs rarely carry handler or file-path hints. All other fields are required.

### `source-key` tagging

When `<source-key>` is supplied, the skill carries it into `discovery.md` as a top-of-block marker next to each candidate it produced on this invocation (e.g. an HTML comment `<!-- source-key: <k> -->` immediately before the `### <name>` heading).

## Idempotency

`/change:analyze` must produce byte-equivalent output on unchanged inputs. The rules:

- No timestamps, environment variables, absolute paths, or other host-state leaks into `discovery.md`.
- Candidates are sorted alphabetically by `name`.
- Inside each candidate's YAML block, fields appear in fixed order: `kind`, `sources`, `handler`, `touches`, `surfaces`, `declared-at`, `unresolved`.
- `sources`, `surfaces`, and `declared-at` are sorted alphabetically within their block.
- When appending to an existing `discovery.md`, the skill deduplicates by candidate `name` — later runs overwrite earlier entries with the same `name`. Candidates from an earlier run that are not present in this run's inputs are preserved; analyze only touches candidates it produced.

A byte-stable output lets the propose brief cache its slicing decisions and surfaces regressions via `git diff`.

## Per-capability prompts (planning-skill-owned)

The detailed extraction prompt lives under `plugins/change/skills/draft/briefs/<capability>/analyze.md` (planning briefs ship with the change-draft skill rather than the capability manifest):

- [`plugins/change/skills/draft/briefs/omnia/analyze.md`](../draft/briefs/omnia/analyze.md) — Omnia's documentation extraction prompt.
- Other capabilities ship their own variant alongside under `plugins/change/skills/draft/briefs/<capability>/`.

`/change:analyze` resolves the active capability via `specify capability resolve` and invokes the relevant brief internally. The skill does **not** embed clustering heuristics; those are capability-specific judgement calls.

## Error handling

- **Missing `$INPUT_PATH`** — hard exit; no placeholder entry.
- **Malformed brief output** (missing required field, invalid candidate block shape) — halt with a diagnostic that names the offending candidate and the brief path; do not write a partially-valid `discovery.md`.

## Fixtures

- [`fixtures/scaffold-example/`](fixtures/scaffold-example/) — an illustrative candidate block demonstrating the on-disk shape of `discovery.md` with the unified fenced-YAML candidate grammar.

## Guardrails

- Never emit full specs; analyze produces candidate blocks only. Deep extraction is `/spec:extract`'s job, run per-slice at define time.
- Never embed clustering heuristics in this SKILL; those live in the capability-owned brief (§*Per-capability prompts*).
- Never let timestamps, absolute paths, or run IDs leak into `discovery.md` — idempotency is a hard contract, not a nicety.
- Never mutate files outside `discovery.md`. The skill appends candidate blocks under the pre-existing `## Candidate inventory` heading; it does not write the heading and does not write any sidecar files.
