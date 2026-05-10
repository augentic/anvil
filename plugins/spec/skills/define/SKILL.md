---
name: specify-define
description: Define a new Specify slice end-to-end — proposal, design, specs, and tasks — in a single pass. Use when an operator describes what they want to build and wants a complete, ready-to-implement slice proposal.
argument-hint: "[description]"
---

# Define Skill

## Critical Path (Quick Reference)

1. **Resolve intent and config** — derive or confirm the slice name, read `.specify/project.yaml`, and treat `domain` / `rules` as constraints only.
2. **Create or resume via CLI** — run `specify slice create <name> --if-exists continue --format json`, surface overlaps, and never hand-edit metadata.
3. **Handle regenerate mode narrowly** — when an artifact ID is provided, resolve that define brief, read its dependencies, rewrite only its `generates` output, and leave lifecycle state unchanged.
4. **Resolve the define pipeline** — call `specify capability pipeline define --change .specify/slices/<name> --format json` and follow the returned topological brief order.
5. **Generate artifacts in dependency order** — read each brief and its `needs`, write every output under `.specify/slices/<name>/`, validate YAML outputs, and follow spec/design/task conventions.
6. **Stamp readiness through CLI** — scan touched specs, transition to `defined`, and summarize artifacts, modified baselines, blockers, and the `/spec:build` handoff.
7. **Report phase outcome** — apply the shared [phase outcome contract](../../references/phase-outcome-contract.md) for `success`, `failure`, or `deferred`, including any plan-entry mutations discovered mid-run.

Define a new slice - create it and generate all artifacts in one step.

When ready to implement, run `/spec:build`.

When working plan-driven (a `plan.yaml` exists), the active entry is claimed before `/spec:define` starts. If this skill uncovers a neighbouring slice or dependency that should be tracked, mutate the plan only through the commands allowed by the shared [phase outcome contract](../../references/phase-outcome-contract.md).

Deterministic bookkeeping — name validation, `.metadata.yaml` writes, capability resolution, pipeline topology, touched-specs scanning, overlap detection — is delegated to the `specify` CLI. This skill only drives the agent-side work: eliciting intent from the user, reading brief bodies, and writing the artifact files those briefs describe.

---

## Driver-supplied arguments

When invoked by `/change:execute` from a plan entry, this skill accepts:

```
/spec:define <name> \
    [source <key>=<path-or-url>...]
```

- **`--source <key>=<path-or-url>`** — a resolved entry from the plan's top-level `sources` map. The key is the kebab-case identifier used in the plan entry's `sources` list; the value is either a local filesystem path or a git URL. `/change:execute` has already validated that the key exists in the plan's top-level `sources` map; this skill treats the `value` as opaque and forwards it to whichever define brief invokes `/spec:extract` (which inlines a guarded `git clone` snippet for URL values — see the *Cloning a source tree* subsection in [`../analyze/SKILL.md`](../analyze/SKILL.md)). The driver never clones; that stays inside the brief pipeline.

The plan entry's `description` field provides the scoping and delta- targeting context that the specs brief uses to infer extract filters and baseline targets. See §Scope inference and §Delta-target inference below.

### Scope inference

The specs brief infers which files to extract from each source by reading the plan entry's `description` for file-path hints. This replaces the former `--scope-*` flag-forwarding pipeline; the define skill no longer receives scope flags from the driver.

- **Path hints present** — the description contains path-like references (e.g. `src/common/validation/`, `src/auth/**`). The brief uses these as `include` globs on `/spec:extract`, treating bare directory names as recursive globs.
- **No path hints** — the brief runs extract on the full source tree.

The brief logs the inferred scope in the journal so operators can audit what was extracted and amend the description if the inference was wrong.

### Delta-target inference

The specs brief infers which existing baselines this slice modifies by reading the plan entry's `description` for references to prior change names (e.g. "delta-target user-registration", "modifies email-verification"). For each referenced name, the brief checks whether a baseline exists at `.specify/specs/<name>/spec.md` and applies the DELTA composition pass on confirmed matches.

The brief logs the inferred delta targets in the journal. If the description does not reference any existing baselines, all extracted specs remain in fresh new-crate form.

The authoritative contract for how `/change:execute` builds these flag values lives in [`../../../change/skills/execute/SKILL.md` → §Argument resolution (`sources`)](../../../change/skills/execute/SKILL.md). The downstream contract for how extract's native filter flags work lives in [`../extract/SKILL.md`](../extract/SKILL.md) (§Scope filters, §Sentinels always read, §Manifest shape).

---

## Phase outcome contract

This skill is the **define** phase of the `/change:execute` driver loop. Apply the shared [phase outcome contract](../../references/phase-outcome-contract.md), including define's per-phase deltas, journal rules, plan-mutation allowlist, and verbatim-`summary` rule.

---

## Input

The user's request should include a slice name (kebab-case) OR a description of what they want to build. Optionally, an artifact ID to regenerate a single artifact for an existing slice (e.g., `/spec:define my-change design`).

## Steps

1. **If no clear input provided, ask what they want to build**

   Ask the user in normal chat:
   > "What slice do you want to work on? Describe what you want to build or fix."

   From their description, derive a kebab-case name (e.g., "add user authentication" -> `add-user-auth`).

   **IMPORTANT**: Do NOT proceed without understanding what the user wants to build.

2. **Read project config**

   Read `.specify/project.yaml` (use the Read tool) for `capability`, `domain`, and `rules`:

   - `capability`: Capability identifier. If `.specify/project.yaml` is missing, run `/spec:init` first and stop.
   - `domain`: Project-level domain context. If absent or a placeholder, fall back to any domain guidance carried by the active capability's briefs and references (resolved under `<resolved_capability_dir>/`, where `<resolved_capability_dir>` comes from any brief `path` returned by step 6).
   - `rules`: Per-brief rule overrides. Optional; empty values mean "no rules apply."

   **IMPORTANT**: `domain` and `rules` guide how you write artifacts. Do NOT copy them into any artifact output.

   Capability resolution and pipeline topology are handled by the CLI in later steps — there is no need to invoke `specify capability resolve` explicitly here.

3. **Check for regenerate mode**

   If the user specified an artifact ID (e.g., `design`):

   a. Run `specify slice status <name> --format json` to confirm the slice exists and its `status` is `defined` or `building`. If the CLI errors with `not_found`, the slice is missing; if `status` is some other value, warn before proceeding. b. Run `specify capability pipeline define --change .specify/slices/<name> --format json` to resolve the brief for the target artifact ID. The returned `briefs[]` array lists every define brief in topological order with each brief's `path`, `needs`, and `generates`. c. For the brief matching the requested artifact ID, verify each entry in its `needs` is already present (the `present` field on the pipeline response). d. Read the required dependency artifacts for context (their paths come from each brief's `generates` joined to `.specify/slices/<name>/`). e. Read the brief file itself from the returned `path`. f. Regenerate ONLY the specified artifact following the brief, applying `domain` and effective rules as constraints. g. Do NOT change the `.metadata.yaml` status — there is no `specify slice transition` call in regenerate mode. h. Show output:

      ```markdown
      ## Artifact Regenerated

      **Change:** <name>
      **Artifact:** <generates> (regenerated)
      **Dependencies read:** <list of needs artifacts>

      The artifact has been updated. Other artifacts are unchanged.
      ```

   i. Stop — do not proceed to full define flow.

4. **Create the slice**

   Run:

   ```bash
   specify slice create <name> --if-exists continue --format json
   ```

   The CLI handles kebab-case validation, directory creation (`.specify/slices/<name>/specs/`), and the initial `.metadata.yaml` write (status `defining`, `created_at` timestamp). With `--if-exists continue`:

   - If the directory does not exist, it is created fresh (`created: true`).
   - If it exists with a valid `.metadata.yaml`, the CLI reuses it (`created: false`) — ask the user whether they want to continue the in-flight slice or pick a different name before proceeding.
   - If it exists without `.metadata.yaml`, the CLI errors — rename or remove the stray directory.

   To start fresh over an existing slice (destructive), pass `--if-exists restart` instead.

5. **Check for overlapping changes**

   Run:

   ```bash
   specify slice overlap <name> --format json
   ```

   For each entry in the `overlaps` array, warn:

   > "The capability `<capability>` is also being modified by change `<other-change>`. This may cause conflicts at merge time."

   This is informational only — do not block the proposal. The CLI only reports overlaps against the slice's current `touched-specs`; step 7 updates those after artifacts are created.

6. **Read the brief pipeline**

   Run:

   ```bash
   specify capability pipeline define --change .specify/slices/<name> --format json
   ```

   The response lists every define brief in topological order with its absolute `path`, `needs` edges, `generates` target, and current `present` flag relative to this slice. Use this list — not `capability.yaml` directly — to drive the generation loop.

7. **Create artifacts in dependency order**

   Use the **TodoWrite tool** to track progress through the briefs.

   For each brief from step 6 (in the order the CLI returned — topologically sorted):

   - Read any completed dependency files (each brief in `needs`'s `generates` path joined to `.specify/slices/<name>/`).
   - Read the brief file itself from its `path`.
   - Resolve the output path from the brief's `generates` field, relative to `.specify/slices/<name>/`:
     - Simple filename (e.g., `proposal.md`): write to `.specify/slices/<name>/<generates>`.
     - Glob pattern (e.g., `specs/**/*.md`): the brief determines how many files to create and where within the pattern.
   - Create the artifact file following the brief, applying `domain` and effective rules as constraints.
   - **YAML output handling:** Dispatch on the `generates` extension. Files ending in `.md` are written as-is. Files ending in `.yaml` (e.g., `composition.yaml`) must be valid YAML — validate the agent's output before writing. The brief's prose instructions may direct the agent to check for an existing artifact in the slice directory or baseline (e.g., the composition brief reads an existing `layout.yaml` or `composition.yaml` as the starting point); the agent reads the file system directly when the brief instructs it.
   - Verify the file exists after writing before proceeding to the next brief.

   ### Spec format conventions

   Follow the heading conventions in `references/spec-format.md` and the baseline/delta format in `references/specify.md` (Spec Files section). The instruction file provides the templates and workflow routing; these conventions govern the content written into those templates.

   **Delta-specific workflows (modified-crate specs):**

   MODIFIED requirements:
   1. Locate the existing requirement in `.specify/specs/<crate>/spec.md`
   2. Copy the ENTIRE requirement block (from `### Requirement:` through all scenarios), including the `ID:` line
   3. Paste under the MODIFIED heading and edit to reflect new behavior
   4. Preserve the original `ID:` value exactly

   ADDED requirements:
   1. Inspect `.specify/specs/<crate>/spec.md` for the highest existing requirement ID
   2. Assign the next sequential ID to the new requirement block
   3. Do not reuse IDs from removed requirements

   **Common pitfalls:**
   - Using MODIFIED with partial content loses detail at merge time
   - If adding new concerns without changing existing behavior, use ADDED instead

   ### Design writing guidance

   Follow the design format and decision criteria in `references/specify.md` (Design Document section, including "When To Create A Full Design"). The instruction file provides the output template.

   ### Task format conventions

   Follow the task format and guidelines in `references/specify.md` (Tasks Document section). The instruction file provides the available-skills table per capability. The build phase parses checkbox format to track progress.

   **Agent-completable task invariant:** Every generated task MUST be executable and verifiable by an agent using code, local tooling, mocks, fixtures, contract validators, build commands, or reviewer skills. Never generate tasks that depend on manual app testing, real-world API credentials, visual inspection, physical-device-only checks, app store review, or asking the user to verify behavior. If a requirement appears to call for human validation, encode the equivalent code-based test or scripted verification task instead. After writing `tasks.md`, complete the **Self-Review** step in the capability's `tasks` brief: re-read every checkbox in context and rewrite any task that fails the agent-completability check. For `tasks.md`, `specify slice validate` checks checkbox/grouping shape only — it does not inspect task intent, so agent-completability must be judged here at write-time (and is re-checked by `/spec:build` as a preflight).

   **Skill directives (optional):** Tasks may include an HTML comment tag that names a specialist skill to invoke during build. The build phase parses these tags and delegates the task to the referenced skill instead of following the default build instruction.

   Format: `- [ ] X.Y Task description <!-- skill: plugin:skill-name -->`

   Tasks without a skill tag are implemented via the default build instruction (mode detection, verification loop, etc.). Use skill tags when a task maps cleanly to a single specialist skill invocation. The instruction file lists available skills per capability.

8. **Finalize and show status**

   Scan the specs directory and record classifications, then transition to `defined`:

   ```bash
   specify slice touched-specs <name> --scan --format json
   specify slice transition <name> defined --format json
   ```

   `touched-specs --scan` walks `.specify/slices/<name>/specs/*`, classifies each capability as `new` (no baseline under `.specify/specs/`) or `modified` (baseline exists), and writes the list into `.metadata.yaml`. `change transition` stamps `defined-at` and enforces the `defining → defined` edge.

   Summarize:

   - Slice name and location
   - List of artifacts created with brief descriptions
   - Any `touched-specs` classified as `modified` (these will surface in conflict checks at merge time)
   - What's ready: "All artifacts created! Ready for implementation."
   - Prompt: "Run `/spec:build` or ask me to implement to start working on the tasks."

## Guardrails

- Create all artifacts for briefs returned by `specify capability pipeline define` before declaring the slice ready.
- Always read dependency artifacts (from each brief's `needs`) before creating a new one.
- **All artifacts MUST be written under `.specify/slices/<name>/`**.
- If context is critically unclear, ask the user -- but prefer making reasonable decisions to keep momentum.
- Never hand-edit `.metadata.yaml`. All status transitions and timestamp writes go through `specify slice transition`; all `touched-specs` updates go through `specify slice touched-specs`. The CLI enforces the legal set of lifecycle values — you do not need to track them yourself.
- If a slice with that name already exists, use `specify slice status <name>` to decide how to proceed.
- Verify each artifact file exists after writing before proceeding to next.
- **IMPORTANT**: `domain` and effective rules (project config overrides) are constraints for YOU, not content for the file. Do NOT copy `<domain>`, `<rules>`, `<project_context>` blocks into any artifact.
