---
name: specify-define
description: Define a new Specify slice end-to-end — proposal, design, specs, and tasks — in a single pass. Use when starting a fresh slice from a user description; not when continuing or regenerating an existing slice.
argument-hint: "[description]"
---

# Define Skill

When ready to implement, run `/spec:build`.

When working plan-driven (a `plan.yaml` exists), the active entry is claimed before `/spec:define` starts. If this skill uncovers a neighbouring slice or dependency that should be tracked, mutate the plan only through the commands allowed by the shared [phase outcome contract](../../references/phase-outcome-contract.md).

Deterministic bookkeeping — name validation, `.metadata.yaml` writes, adapter resolution, pipeline topology, touched-specs scanning, overlap detection — is delegated to the `specify` CLI. This skill only drives the agent-side work: eliciting intent from the user, reading brief bodies, and writing the artifact files those briefs describe.

## Critical Path

### 1. Resolve intent and project config

If no clear input was provided, ask in normal chat:

> "What slice do you want to work on? Describe what you want to build or fix."

From the description, derive a kebab-case slice name (e.g., "add user authentication" → `add-user-auth`). Do **not** proceed without understanding what the user wants to build.

Read `.specify/project.yaml` for `adapter`, `domain`, and `rules`. If the file is missing, run `/spec:init` first and stop. `domain` and `rules` guide how you write artifacts — they are constraints for the agent, never content to copy into any artifact output. If `domain` is absent or a placeholder, fall back to any domain guidance carried by the active adapter's briefs and references. Adapter resolution and pipeline topology are handled by the CLI in later steps; no need to invoke `specify adapter resolve` here.

### 2. Handle regenerate mode (artifact ID supplied)

If the user supplied an artifact ID (e.g., `/spec:define my-change design`), follow [`references/define-regenerate.md`](references/define-regenerate.md) — it owns the eight-step regenerate procedure and the output template. Regenerate mode does not transition lifecycle status and stops once the single artifact is rewritten.

### 3. Create or resume the slice

Run `specify slice create <name> --if-exists continue --format json` and branch on the `created` field. The CLI handles kebab-case validation, directory creation (`.specify/slices/<name>/specs/`), and the initial `.metadata.yaml` write. When `created: false` (an existing slice with valid `.metadata.yaml` was reused), confirm with the user that they want to continue the in-flight slice before proceeding. To start fresh over an existing slice destructively, pass `--if-exists restart` instead. See `specify slice create --help` for the full flag semantics.

### 4. Check for overlapping changes

Run `specify slice overlap <name> --format json`. For each entry in the `overlaps` array, warn:

> "The adapter `<adapter>` is also being modified by change `<other-change>`. This may cause conflicts at merge time."

Informational only — do not block the proposal. The CLI only reports overlaps against the slice's current `touched-specs`; step 6 updates those after artifacts are created.

### 5. Resolve the define pipeline

Run `specify adapter pipeline define --change .specify/slices/<name> --format json`. The response lists every define brief in topological order with its absolute `path`, `needs` edges, `generates` target, and current `present` flag relative to this slice. Use this list — not `adapter.yaml` directly — to drive the generation loop in step 6.

### 6. Generate artifacts in dependency order

Use the **TodoWrite tool** to track progress through the briefs. For each brief returned by step 5 in order:

- Read any completed dependency files (each brief in `needs`'s `generates` path, joined to `.specify/slices/<name>/`).
- Read the brief file itself from its `path`.
- Resolve the output path from the brief's `generates` field, relative to `.specify/slices/<name>/`. Simple filenames (e.g., `proposal.md`) write to that exact path; glob patterns (e.g., `specs/**/*.md`) let the brief determine how many files to create within the pattern.
- Create the artifact file following the brief, applying `domain` and effective rules as constraints.
- **YAML output handling:** dispatch on the `generates` extension. Files ending in `.md` are written as-is. Files ending in `.yaml` (e.g., `composition.yaml`) must be valid YAML — validate output before writing. The brief's prose may direct the agent to read an existing artifact (e.g., the composition brief reads an existing `layout.yaml` or `composition.yaml` as the starting point); the agent reads the file system directly when the brief instructs it.
- Verify the file exists after writing before proceeding to the next brief.

Format conventions for spec, design, and task artifacts (delta workflows, ID assignment, design template, agent-completable task invariant, skill-directive comment syntax) live in `references/artifact-conventions.md` — read it before writing the spec, design, or tasks artifacts.

### 7. Stamp readiness through CLI

Scan the specs directory and record classifications, then transition to `defined`:

```bash
specify slice touched-specs <name> --scan --format json
specify slice transition <name> defined --format json
```

`touched-specs --scan` walks `.specify/slices/<name>/specs/*`, classifies each adapter as `new` (no baseline under `.specify/specs/`) or `modified` (baseline exists), and writes the list into `.metadata.yaml`. `slice transition` stamps `defined-at` and enforces the `defining → defined` edge.

Summarize:

- Slice name and location
- List of artifacts created with brief descriptions
- Any `touched-specs` classified as `modified` (these will surface in conflict checks at merge time)
- What's ready: "All artifacts created! Ready for implementation."
- Prompt: "Run `/spec:build` or ask me to implement to start working on the tasks."

## Driver-supplied arguments

When invoked by `/change:execute` from a plan entry, this skill accepts:

```
/spec:define <name> \
    [source <key>=<path-or-url>...]
```

- **`--source <key>=<path-or-url>`** — a resolved entry from the plan's top-level `sources` map. The key is the kebab-case identifier used in the plan entry's `sources` list; the value is either a local filesystem path or a git URL. `/change:execute` has already validated that the key exists in the plan's top-level `sources` map; this skill treats the `value` as opaque and forwards it to whichever define brief invokes `/spec:extract` (which inlines a guarded `git clone` snippet for URL values — see the *Cloning a source tree* subsection in `/change:analyze`). The driver never clones; that stays inside the brief pipeline.

The plan entry's `description` field provides the scoping and delta-targeting context that the specs brief uses to infer extract filters and baseline targets. See §Scope inference and §Delta-target inference below.

### Scope inference

The specs brief infers which files to extract from each source by reading the plan entry's `description` for file-path hints. This replaces the former `--scope-*` flag-forwarding pipeline; the define skill no longer receives scope flags from the driver.

- **Path hints present** — the description contains path-like references (e.g. `src/common/validation/`, `src/auth/**`). The brief uses these as `include` globs on `/spec:extract`, treating bare directory names as recursive globs.
- **No path hints** — the brief runs extract on the full source tree.

The brief logs the inferred scope in the journal so operators can audit what was extracted and amend the description if the inference was wrong.

### Delta-target inference

The specs brief infers which existing baselines this slice modifies by reading the plan entry's `description` for references to prior change names (e.g. "delta-target user-registration", "modifies email-verification"). For each referenced name, the brief checks whether a baseline exists at `.specify/specs/<name>/spec.md` and applies the DELTA composition pass on confirmed matches.

The brief logs the inferred delta targets in the journal. If the description does not reference any existing baselines, all extracted specs remain in fresh new-crate form.

The authoritative contract for how `/change:execute` builds these flag values lives in `/change:execute` §Argument resolution (`sources`). The downstream contract for how extract's native filter flags work lives in `/spec:extract` (§Scope filters, §Sentinels always read, §Manifest shape).

## Phase outcome contract

This skill is the **define** phase of the `/change:execute` driver loop. Apply the shared [phase outcome contract](../../references/phase-outcome-contract.md), including define's per-phase deltas, journal rules, plan-mutation allowlist, and verbatim-`summary` rule.

## Guardrails

- Create all artifacts for briefs returned by `specify adapter pipeline define` before declaring the slice ready.
- Always read dependency artifacts (from each brief's `needs`) before creating a new one.
- **All artifacts MUST be written under `.specify/slices/<name>/`**.
- If context is critically unclear, ask the user — but prefer making reasonable decisions to keep momentum.
- Route every write to `.metadata.yaml`, `plan.yaml`, and the `.specify/specs/` baseline through the CLI — see [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state). Status transitions and timestamp writes go through `specify slice transition`; `touched-specs` updates go through `specify slice touched-specs`; plan amendments only when the run uncovers neighbouring work and only via `specify plan add` / `specify plan amend` per the [phase outcome contract](../../references/phase-outcome-contract.md).
- Never implement code or flip task checkboxes here — implementation is `/spec:build`'s phase; define stops once artifacts are written and the slice is `defined`.
- Never extract specs from external source code directly — delegate to `/spec:extract` (invoked by define briefs in driver-supplied `<source>` mode).
- Never run plan-time adapter inference — that lives in `/change:analyze`, orchestrated by the `/change:draft` discovery brief.
- If a slice with that name already exists, use `specify slice status <name>` to decide how to proceed.
- Verify each artifact file exists after writing before proceeding to next.
- **IMPORTANT**: `domain` and effective rules (project config overrides) are constraints for YOU, not content for the file. Do NOT copy `<domain>`, `<rules>`, `<project_context>` blocks into any artifact.
