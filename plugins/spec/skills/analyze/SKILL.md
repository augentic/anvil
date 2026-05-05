---
name: specify-analyze
description: |
  Plan-time capability inference for both legacy code and documentation
  inputs. Emits capability summaries into discovery.md — not full specs.
  Branches internally on --kind; per-kind clustering / extraction prompts
  are schema-owned. Use when the plan-time discovery brief needs a
  capability-level inventory of a source before propose slices it.
argument-hint: "<input-path> <output-dir>"
---

# Analyze Skill

`/spec:analyze` is the sole plan-time discovery skill. It reads one input — a legacy code tree or a documentation bundle — and appends **capability summaries** to `$DISCOVERY`. It does **not** produce full `specs/` + `design.md`; deep per-slice extraction remains [`../extract/SKILL.md`](../extract/SKILL.md)'s job at define time.

The rationale for the two-skill split: analyze produces capability summaries at plan time; extract produces full specs + design at define time.

## Derived Arguments

```text
$INPUT_PATH  = $ARGUMENTS[0]
$OUTPUT_DIR  = $ARGUMENTS[1]
$KIND        = $ARGUMENTS --kind flag value   # closed: legacy-code | documentation
$SOURCE_KEY  = $ARGUMENTS --source-key flag value (optional)
$DISCOVERY   = $OUTPUT_DIR/discovery.md
```

`$INPUT_PATH` is either a filesystem path to a source tree (for `--kind legacy-code`) or to a documentation bundle (for `--kind documentation`). `$OUTPUT_DIR` is the plan working directory (`.specify/plans/<initiative>/` when called from the discovery brief); the skill writes to `$DISCOVERY` under it, and — for `--kind legacy-code` only — to the structural-metadata sidecar at `$OUTPUT_DIR/analyze/<$SOURCE_KEY>/metadata.json` (see §*Structural metadata*). `$SOURCE_KEY` is optional; when supplied, the discovery brief uses it to tag this run for a specific top-level plan source.

### Cloning a source tree

`/spec:analyze` only consumes local paths. When a `--source <key>=<url>` (or any caller) needs to materialise a remote git URL into `$INPUT_PATH` first, use the following guarded clone — the inlined replacement for the retired RT clone skill:

```bash
# Quote DEST and never run rm -rf without verifying the target.
git clone "$URL" "$DEST"
test -d "$DEST/.git" && rm -rf "$DEST/.git"   # only if --detach mode is required
```

Pass the resulting `$DEST` as `$INPUT_PATH` on the next `/spec:analyze` invocation.

## Input kinds (closed enum)

`--kind` must be exactly one of:

| kind            | Branch                                                                |
| --------------- | --------------------------------------------------------------------- |
| `legacy-code`   | Cluster code into capability summaries (schema-owned algorithm).      |
| `documentation` | Extract capability summaries from prose / PDFs / runbooks / API docs. |

Any other value is a hard error; the skill exits non-zero before writing anything to `$DISCOVERY`. See [`../plan/SKILL.md` §*Input kinds*](../plan/SKILL.md) for the normative enum definition — `/spec:analyze` is the enforcement site for unknown-kind errors, but the vocabulary itself is pinned by the plan skill. Do not extend it.

## Output contract

Each run appends one capability block per inferred capability to `$DISCOVERY`, creating the file if it does not yet exist. The on-disk shape of a single capability is:

````markdown
### user-registration

```yaml
summary: Create new user accounts with email verification.
sources:
  - src/auth/verify.ts
  - src/users/register.ts
  - src/users/validation.ts
depends-on: [email-verification, shared-validation]
hints:
  entry_points: [POST /users]
  external_deps: [postgres, sendgrid]
confidence: high
```
````

The markdown `### <name>` heading keeps `$DISCOVERY` human-scannable with existing tools; the fenced YAML block makes the fields mechanically parseable by the propose brief without markdown-specific heuristics. Fields inside the YAML block:

- **`summary`** — single-line imperative description.
- **`sources`** — list of file paths relative to `$INPUT_PATH`, sorted alphabetically. These are the **file hints** the propose brief maps directly onto `scope.<src>.include` on the plan entry.
- **`depends-on`** — list of capability names this capability references at the code / conceptual level, sorted alphabetically.
- **`hints`** — map with optional keys `entry_points` (e.g. `POST /users`) and `external_deps` (e.g. `postgres`, `sendgrid`). Both lists are sorted alphabetically. Either key may be omitted.
- **`confidence`** — one of `high`, `medium`, `low`.

Append semantics: each `/spec:analyze` invocation appends its capabilities to `$DISCOVERY` in alphabetical order by `name`. The discovery brief calls `/spec:analyze` once per input and the combined file is the union; analyze itself does not dedup across runs beyond the rule below.

The code branch additionally writes a structural-metadata sidecar — see §*Structural metadata* below.

### `--source-key` tagging

When `$SOURCE_KEY` is supplied, the skill carries it into `$DISCOVERY` as a top-of-block marker next to each capability it produced on this invocation (e.g. an HTML comment `<!-- source-key: <k> -->` immediately before the `### <name>` heading). For the scaffold branch the semantics stay thin: the flag is recorded, not used to rewrite `sources:` paths. Path-rewriting nuance is refined in the documentation branch and code branch once the per-kind prompts land.

## Structural metadata (per-source)

In addition to appending capability summaries to `$DISCOVERY`, the code branch (`--kind legacy-code`) writes a small JSON sidecar capturing source-tree structural facts. The documentation branch does **not** write this sidecar — it has no code structure to measure.

**Location.** `<plan-dir>/analyze/<$SOURCE_KEY>/metadata.json`, where `<plan-dir>` is `.specify/plans/<initiative-name>/` (i.e. `$OUTPUT_DIR` when the skill is invoked by the discovery brief). The `<$SOURCE_KEY>` segment matches the `--source-key` flag value; when the flag is omitted, analyze synthesises a key using the same rule as §*`--source-key` tagging*.

**Shape (version 1):**

````json
{
  "version": 1,
  "source_key": "monolith",
  "language": "typescript",
  "loc": 87312,
  "module_count": 42,
  "top_level_modules": [
    "src/auth",
    "src/ingest",
    "src/billing"
  ]
}
````

| field | type | notes |
| ---- | ---- | ---- |
| `version` | integer | `1` for v1. Do not bump this version. |
| `source_key` | string | Matches the directory segment; redundant but stable on its own. |
| `language` | string | Detected primary source language (kebab-case: `typescript`, `javascript`, `rust`, `go`, `python`, `java`, `kotlin`, `csharp`, …). Per-schema convention — schemas pin their own valid set. |
| `loc` | integer | Total source lines of code. Schema-owned convention (non-blank non-comment preferred; raw line count acceptable). Must be consistent across runs of the same schema. |
| `module_count` | integer | Total module count. Schema-owned definition (TS: files; Java: classes; Python: modules; …). |
| `top_level_modules` | array[string] | Immediate children of the source root, alphabetically sorted, relative path strings. May be empty. |

All fields are required. The detection algorithm that produces each field is owned by the schema-specific code branch prompt (`schemas/<schema>/briefs/plan/analyze.md`); this SKILL only pins the field names, types, and on-disk shape.

**Idempotency.** Same rules as §*Output contract*: no timestamps, no host state, byte-stable field order matching the shape above, and alphabetically-sorted `top_level_modules`. Re-running analyze on unchanged inputs emits byte-identical metadata. This lets `specify plan validate` diff the file across runs without drift.

**Consumers.** `specify plan validate` reads this metadata to emit the non-blocking `scope-missing-on-monolith` warning. No other consumer exists in v1; propose reads capability summaries from `$DISCOVERY`, not this sidecar.

## Idempotency

`/spec:analyze` must produce byte-equivalent output on unchanged inputs. The rules:

- No timestamps, environment variables, absolute paths, or other host-state leaks into `$DISCOVERY`.
- Capabilities are sorted alphabetically by `name`.
- Inside each capability's YAML block, fields appear in fixed order: `summary`, `sources`, `depends-on`, `hints`, `confidence`.
- `sources`, `depends-on`, `hints.entry_points`, and `hints.external_deps` are sorted alphabetically within their block.
- When appending to an existing `$DISCOVERY`, the skill deduplicates by capability `name` — later runs overwrite earlier entries with the same `name`. Capabilities from an earlier run that are not present in this run's inputs are preserved; analyze only touches capabilities it produced.

A byte-stable output lets the propose brief cache its slicing decisions and surfaces regressions via `git diff`.

## Per-kind prompts (schema-owned)

The detailed clustering / extraction prompt for each `--kind` value lives under `schemas/<schema>/briefs/plan/analyze.md`:

- `capabilities/omnia/briefs/plan/analyze.md` — Omnia's per-kind prompt (documentation branch and code branch).
- Other schemas ship their own.

`/spec:analyze` resolves the active schema via `specify schema resolve` and invokes the relevant brief internally. The skill does **not** embed clustering heuristics; those are schema-specific judgement calls (import-graph vs docstring vs endpoint-name weighting, confidence thresholds, etc.).

## Process

1. **Validate arguments.** Reject if `$KIND` is not in the closed enum, if `$INPUT_PATH` does not exist, or if `$OUTPUT_DIR` is not writable. Each failure is a hard exit with a clear diagnostic; no partial write to `$DISCOVERY` ever ships.
2. **Resolve schema and per-kind brief path.** Run `specify schema resolve` and load `schemas/<schema>/briefs/plan/analyze.md`.
3. **Invoke the brief against `$KIND`.** The brief owns clustering (for `legacy-code`) or extraction (for `documentation`) and emits capability summaries in the shape pinned above.
4. **Write outputs.**
   - **4a.** Write / append to `$DISCOVERY` with the idempotent ordering rules (both branches), optionally tagging each emitted capability with `$SOURCE_KEY`. Report the list of capability names written on stdout for the discovery brief to aggregate.
   - **4b.** For `$KIND = legacy-code` **only**, write `<plan-dir>/analyze/<$SOURCE_KEY>/metadata.json` per §*Structural metadata*. Create the directory if it does not exist; overwrite the file if present. The documentation branch MUST NOT write this sidecar.

## Error handling

- **Unknown `--kind`** — hard exit. The diagnostic names the closed enum and points at [`../plan/SKILL.md` §*Input kinds*](../plan/SKILL.md).
- **Missing `$INPUT_PATH`** — hard exit; no placeholder entry.
- **Malformed brief output** (missing required field, non-enum confidence, non-string summary) — halt with a diagnostic that names the offending capability and the brief path; do not write a partially-valid `$DISCOVERY`.
- **Metadata sidecar on the documentation branch** — hard guardrail, not a runtime error: `$KIND = documentation` MUST NOT write `<plan-dir>/analyze/<$SOURCE_KEY>/metadata.json`. The documentation branch has no code structure to measure, so the slot stays absent for doc inputs.

## Fixtures

- [`fixtures/scaffold-example/`](fixtures/scaffold-example/) — an illustrative capability-summary block plus a small structural- metadata sidecar demonstrating the on-disk shape of both `$DISCOVERY` and `<plan-dir>/analyze/<$SOURCE_KEY>/metadata.json`. Structural only; the per-kind fixtures with real clustering land in the documentation and code branches.

## Guardrails

- Never emit full specs; analyze produces capability summaries only. Deep extraction is [`../extract/SKILL.md`](../extract/SKILL.md)'s job, run per-slice at define time.
- Never embed clustering heuristics in this SKILL; those live in the schema-owned per-kind brief (§*Per-kind prompts*).
- Never let timestamps, absolute paths, or run IDs leak into `$DISCOVERY` or the structural-metadata sidecar — idempotency is a hard contract, not a nicety.
- Never mutate files outside `$DISCOVERY` and `<plan-dir>/analyze/<$SOURCE_KEY>/metadata.json`. The structural-metadata sidecar is written by the code branch only; the documentation branch must leave the slot untouched.
- NEVER add fields to `metadata.json` beyond the six pinned in §*Structural metadata*.
