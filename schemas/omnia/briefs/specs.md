---
id: specs
description: Create specification files that define WHAT the system should do
generates: specs/**/*.md
needs: [proposal]
---

## Baseline Contract Awareness

When `.specify/contracts/` exists and contains files, read its contents as **read-only context** before writing specs. This ensures behavioral requirements conform to existing interface shapes rather than inventing new ones:

- **Endpoint conformance:** When baseline contracts define HTTP endpoints (in `.specify/contracts/http/`), write spec scenarios that reference the existing endpoint paths, HTTP methods, and status codes. Do not invent new endpoint paths when the baseline already defines one for the same interaction.
- **Payload conformance:** When baseline contracts define JSON Schema types (in `.specify/contracts/schemas/`), write spec scenarios whose data references are consistent with the existing field names, types, and required/optional status. Do not describe payload fields that contradict the schema.
- **Message conformance:** When baseline contracts define messaging channels (in `.specify/contracts/messages/`), write spec scenarios that reference the existing channel names and message structures.
- **Error conformance:** When baseline contracts define error responses, write error condition sections that are consistent with the contract's error types and status codes.

This is a **context hint, not a hard constraint**. When the change requires interactions not covered by the baseline contracts, write the spec scenarios naturally and flag the gap in the spec. New or changed interface shapes belong in a separate `contracts@v1` change before implementation depends on them. The goal is consistency with existing contracts, not restriction to them.

When `.specify/contracts/` does not exist, this section has no effect — proceed with spec authoring as normal.

When the plan entry has a `context` field containing `contracts/` paths, read only those specific contract files as conformance context rather than scanning the entire `.specify/contracts/` directory.

---

Branch on the flag surface handed to `/spec:define`:

- **Source-driven** — at least one `--source <key>=<path>` flag is present. Run the per-source extract loop below, then merge.
- **Manual** — no `--source` flags are present. Author specs by hand using the manual templates at the bottom of this brief.

> **Migration note.** Proposals that used to name a single *Repository* URL or *Source-code* path are still valid; `/spec:execute` (or the operator on a direct invocation) translates those into a single `--source <key>=<path>` flag before reaching this brief. The proposal's Source section is a secondary signal for context only — the authoritative branching trigger is the `--source` flag set.

---

## Source-driven branch

For each `--source <key>=<path>` flag, processed in declaration order:

1. **Infer scope from the change description.**

   Read the plan entry's `description` and look for file-path hints (e.g. `src/common/validation/`, `src/auth/**`). Build the extract filter set:

   - When the description contains path-like references for this source, use each as an `--include` glob on `/spec:extract`. Treat bare directory names as recursive globs (e.g. `src/auth/` becomes `src/auth/**`).
   - When the description contains no path hints for this source, run extract on the full source tree (no filter flags).

   Log the inferred scope in the journal via `specify change journal append <name> define question --summary "Inferred scope for <key>: <filters>"`. This gives operators an audit trail of what was extracted.

2. **Invoke `/spec:extract`:**

   ```text
   /spec:extract <path> <change-dir>/.extract/<key>/ [inferred filters]
   ```

   `/spec:extract` resolves globs relative to `<path>`, applies the *sentinels always read* rule, and treats a zero-match filter as a hard error. See `plugins/spec/skills/extract/SKILL.md` for the full contract.

3. **Merge the per-source output into the change root.**

   ```text
   <change-dir>/.extract/<key>/specs/    →  <change-dir>/specs/
   <change-dir>/.extract/<key>/design.md →  <change-dir>/design.md
   ```

   Merge policy:

   - **`specs/`** — copy each extracted capability's directory to `<change-dir>/specs/<capability>/`. If two source keys both emit a spec for the same capability name, that is a **name collision**: halt with a brief-level error and surface both colliding paths. The propose brief is responsible for preventing this by forcing distinct capability names across sources or consolidating duplicates under one source. Do not attempt to auto-resolve.
   - **`design.md`** — concatenate per-source design sections in `--source` declaration order. When two or more sources contribute, wrap each section under a level-2 heading `## Source: <key>` so the merged artifact makes provenance obvious. When there is exactly one source, emit the section content without the wrapper — the small-legacy case stays clean.

After every `--source` has been processed and merged, the merged `<change-dir>/specs/` and `<change-dir>/design.md` are the specs-phase output.

### `.extract/<key>/` scratch directory

- `<change-dir>/.extract/<key>/` is per-source scratch. Each iteration writes its full extract output here (specs, design, per-module YAML, traceability dumps).
- **Default: keep after merge.** Retention helps human review — inspect the per-source output while the change is in-flight. Deletion is manual: `rm -rf <change-dir>/.extract/` once the change is merged.
- The scratch tree MUST NOT be committed with the change. Treat `.extract/` as operator-disciplined local-only state; `.gitignore` wiring is a downstream concern and not handled by this brief.

### Delta composition (affects inference)

After the per-source loop has merged its extracted specs into `<change-dir>/specs/`, determine whether this change modifies existing baselines by reading the plan entry's `description` for references to prior change names (e.g. "delta-target user-registration", "modifies email-verification", "refactors out of user-registration and email-verification").

For each referenced name, check whether a baseline exists at `.specify/specs/<name>/spec.md`. Collect the confirmed names into the **inferred affects set**. Log the inferred set in the journal via `specify change journal append <name> define question --summary "Inferred delta targets: <names>"`.

If the inferred affects set is non-empty, run the following four-step pass. If the description does not reference any existing baselines, skip this section entirely — all extracted specs remain in fresh new-crate form.

1. **Reuse the merged extract output — do not re-invoke extract.** The per-source loop above has already written one `<change-dir>/specs/<capability>/spec.md` for every capability the merged extract covers. That merged tree is the input to the matching step; no additional `/spec:extract` call is needed here. Extract remains baseline-unaware — it never reads `.specify/specs/`.

2. **Rewrite matched capabilities in DELTA form.** For each name in the inferred affects set, check for a merged spec at `<change-dir>/specs/<name>/spec.md`. When one exists:

   - Read the baseline at `.specify/specs/<name>/spec.md`.
   - Diff the extracted capability spec against the baseline and rewrite `<change-dir>/specs/<name>/spec.md` using the ADDED / MODIFIED / RENAMED / REMOVED delta structure documented under the define skill's [Spec format conventions → Delta-specific workflows](../../../plugins/spec/skills/define/SKILL.md#spec-format-conventions).
   - The delta form replaces the fresh-spec form at `<change-dir>/specs/<name>/spec.md`; do not keep both. The baseline at `.specify/specs/<name>/spec.md` stays untouched until the change merges.

3. **Leave unmatched capabilities as fresh specs.** Capabilities whose names do not match any inferred affects target keep the new-crate spec form already written by the per-source merge. No rewrite, no additional work.

4. **Warn on inferred targets with no extract match.** For each name in the inferred affects set with no corresponding `<change-dir>/specs/<name>/spec.md` after the merge, emit a brief-level warning naming the orphan target. Suggest that the description may be inaccurate — the operator can amend it via `specify plan amend <change> --description "..."`.

   The warning is informational; the brief continues.

After this pass, `<change-dir>/specs/<name>/spec.md` is in delta form for every matched inferred target, in fresh new-crate form for every unmatched extracted capability, and absent for every unmatched inferred target (with a warning logged).

---

## Manual branch

When no `--source` flag is present, create one spec file per crate listed in the proposal's Crates section.

**New Crates**: Use the exact kebab-case name from the proposal (`specs/<crate>/spec.md`). Follow this structure:

```markdown
# <Crate Name> Specification

## Purpose

<1-2 sentence description of what this crate does>

### Requirement: <Behavior Name>

ID: REQ-001

The system SHALL <behavioral description>.

#### Scenario: <Happy Path>

- **WHEN** <trigger or input>
- **THEN** <expected behavior>

#### Scenario: <Error Case>

- **WHEN** <invalid input or failing condition>
- **THEN** <expected error behavior>

## Error Conditions

- <error type>: <description and trigger conditions>

## Metrics

- `<metric_name>` — type: <counter|gauge|histogram>; emitted: <when>
```

Repeat `### Requirement:` blocks for each distinct behavior, incrementing `ID: REQ-XXX` for each new requirement.

**Modified Crates**: Use the existing spec folder name from `.specify/specs/<crate>/` when creating the delta spec at `specs/<crate>/spec.md`. Follow this structure:

```markdown
## ADDED Requirements

### Requirement: <!-- requirement name -->
ID: REQ-<!-- next available id -->
<!-- requirement text -->

#### Scenario: <!-- scenario name -->
- **WHEN** <!-- condition -->
- **THEN** <!-- expected outcome -->

## MODIFIED Requirements

### Requirement: <!-- existing requirement name -->
ID: REQ-<!-- existing id (must match baseline) -->
<!-- full updated requirement text -->

#### Scenario: <!-- scenario name -->
- **WHEN** <!-- condition -->
- **THEN** <!-- expected outcome -->

## REMOVED Requirements

### Requirement: <!-- existing requirement name -->
ID: REQ-<!-- existing id -->
**Reason**: <!-- why this requirement is being removed -->
**Migration**: <!-- how to handle the removal -->

## RENAMED Requirements

ID: REQ-<!-- existing id -->
TO: <!-- new requirement name -->
```

Follow the spec format conventions defined in the define skill for delta operations, format rules, and the MODIFIED/ADDED workflows.

---

## Fixtures

Worked walkthroughs live under `fixtures/specs/` next to this brief:

- `extract-shared-validation/` — single-source run with path hints in description and delta-target inference (the canonical refactor case). Pins the full extract → merge → delta-composition path.
- `affects-orphan-warning/` — sibling case pinning step 4 of the delta composition pass: one matched inferred target, one orphan that fires a warning.
- `single-source-no-scope/` — full-tree path: one source, description without path hints.
- `description-driven-multi-source/` — two sources with description- inferred scope; demonstrates the `## Source: <key>` design wrapper and the name-collision merge rule's trigger conditions.
