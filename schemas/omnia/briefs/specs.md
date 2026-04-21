---
id: specs
description: Create specification files that define WHAT the system should do
generates: specs/**/*.md
needs: [proposal]
---

This brief implements RFC-3a §*How `scope` travels through the pipeline* for
the specs phase.

Branch on the flag surface handed to `/spec:define`:

- **Source-driven** — at least one `--source <key>=<path>` flag is present.
  Run the per-source extract loop below, then merge.
- **Manual** — no `--source` flags are present. Author specs by hand using
  the manual templates at the bottom of this brief.

> **Migration note.** Proposals that used to name a single *Repository* URL
> or *Source-code* path are still valid; `/spec:execute` (or the operator on
> a direct invocation) translates those into a single `--source <key>=<path>`
> flag before reaching this brief. The proposal's Source section is a
> secondary signal for context only — the authoritative branching trigger is
> the `--source` flag set.

---

## Source-driven branch

For each `--source <key>=<path>` flag, processed in declaration order:

1. **Collect the scope bundle for `<key>`.**

   ```text
   includes  = every --scope-include <key>=<glob>    (order-preserving)
   excludes  = every --scope-exclude <key>=<glob>    (order-preserving)
   manifest  = the --scope-manifest <key>=<path>, if any
   ```

   Bundle invariant: `manifest` is set XOR (`includes` OR `excludes`) is
   non-empty. `/spec:define` enforces this defensively; assert it here too
   so a hand-crafted invocation cannot slip through. If both are present,
   halt with a brief-level error naming the offending key.

   If the bundle is empty for this key (no `--scope-*` flags referenced
   `<key>`), skip the translation step — this is the back-compat /
   small-legacy path and simply hands the whole source tree to extract.

2. **Translate the bundle to extract's native flags.**

   ```text
   --include <glob>     for each entry in includes      (repeat)
   --exclude <glob>     for each entry in excludes      (repeat)
   --manifest <path>    when manifest is set
   ```

   Forward globs and manifest paths verbatim — no expansion, no stat.
   Path-shape and existence diagnostics are `/spec:extract`'s concern.

3. **Invoke `/spec:extract`:**

   ```text
   /spec:extract <path> <change-dir>/.extract/<key>/ [translated flags]
   ```

   `/spec:extract` resolves globs relative to `<path>`, reads the manifest
   (paths also relative to `<path>`), applies the *sentinels always read*
   rule, and treats a zero-match filter as a hard error. See
   `plugins/spec/skills/extract/SKILL.md` for the full contract.

4. **Merge the per-source output into the change root.**

   ```text
   <change-dir>/.extract/<key>/specs/    →  <change-dir>/specs/
   <change-dir>/.extract/<key>/design.md →  <change-dir>/design.md
   ```

   Merge policy:

   - **`specs/`** — copy each extracted capability's directory to
     `<change-dir>/specs/<capability>/`. If two source keys both emit a
     spec for the same capability name, that is a **name collision**:
     halt with a brief-level error and surface both colliding paths. The
     propose brief is responsible for preventing this by forcing distinct
     capability names across sources or consolidating duplicates under
     one source. Do not attempt to auto-resolve.
   - **`design.md`** — concatenate per-source design sections in
     `--source` declaration order. When two or more sources contribute,
     wrap each section under a level-2 heading `## Source: <key>` so the
     merged artifact makes provenance obvious. When there is exactly one
     source, emit the section content without the wrapper — the
     small-legacy case stays clean.

After every `--source` has been processed and merged, the merged
`<change-dir>/specs/` and `<change-dir>/design.md` are the specs-phase
output.

### `.extract/<key>/` scratch directory

- `<change-dir>/.extract/<key>/` is per-source scratch. Each iteration
  writes its full extract output here (specs, design, per-module YAML,
  traceability dumps).
- **Default: keep after merge.** Retention helps human review — inspect
  the per-source output while the change is in-flight. Deletion is
  manual: `rm -rf <change-dir>/.extract/` once the change is merged.
- The scratch tree MUST NOT be committed with the change. Treat `.extract/`
  as operator-disciplined local-only state; `.gitignore` wiring is a
  downstream concern and not handled by this brief.

### Affects composition

When the driver supplies `--affects <name>` flags alongside `--source`
(forwarded from the plan entry's `affects:` list), run the following
four-step pass **after** the per-source loop has merged its extracted
specs into `<change-dir>/specs/`. The RFC pins exactly four steps; keep
the numbering intact.

1. **Reuse the merged extract output — do not re-invoke extract.** The
   per-source loop above has already written one
   `<change-dir>/specs/<capability>/spec.md` for every capability the
   merged extract covers. That merged tree is the input to the matching
   step; no additional `/spec:extract` call is needed here. Extract
   remains baseline-unaware — it never reads `.specify/specs/`.

2. **Rewrite matched capabilities in DELTA form.** For each
   `--affects <name>`, check for a merged spec at
   `<change-dir>/specs/<name>/spec.md`. When one exists:

   - Read the baseline at `.specify/specs/<name>/spec.md`.
   - Diff the extracted capability spec against the baseline and
     rewrite `<change-dir>/specs/<name>/spec.md` using the
     ADDED / MODIFIED / RENAMED / REMOVED delta structure documented
     under the define skill's
     [Spec format conventions → Delta-specific workflows](../../../plugins/spec/skills/define/SKILL.md#spec-format-conventions).
   - The delta form replaces the fresh-spec form at
     `<change-dir>/specs/<name>/spec.md`; do not keep both. The
     baseline at `.specify/specs/<name>/spec.md` stays untouched until
     the change merges.

3. **Leave unmatched capabilities as fresh specs.** Capabilities whose
   names do not match any supplied `--affects <name>` keep the
   new-crate spec form already written by the per-source merge. No
   rewrite, no additional work.

4. **Warn on `--affects` names with no extract match.** For each
   `--affects <name>` flag with no corresponding
   `<change-dir>/specs/<name>/spec.md` after the merge, emit a
   brief-level warning that names the orphan flag and surfaces both
   remediations:

   - The baseline is untouched by this slice — drop the flag via
     `specify initiative amend <change> --affects-rm <name>`.
   - The scope is too narrow to see the file that would change the
     baseline's behaviour — widen `scope.<key>.include` (or switch to
     `scope.<key>.manifest`) via `specify initiative amend`.

   The warning is informational; the brief continues.

After this pass, `<change-dir>/specs/<name>/spec.md` is in delta form
for every matched `--affects <name>`, in fresh new-crate form for
every unmatched extracted capability, and absent for every
unmatched `--affects` name (with a warning logged).

---

## Manual branch

When no `--source` flag is present, create one spec file per crate listed
in the proposal's Crates section.

**New Crates**: Use the exact kebab-case name from the proposal
(`specs/<crate>/spec.md`). Follow this structure:

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

Repeat `### Requirement:` blocks for each distinct behavior, incrementing
`ID: REQ-XXX` for each new requirement.

**Modified Crates**: Use the existing spec folder name from
`.specify/specs/<crate>/` when creating the delta spec at
`specs/<crate>/spec.md`. Follow this structure:

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

Follow the spec format conventions defined in the define skill for delta
operations, format rules, and the MODIFIED/ADDED workflows.

---

## Fixtures

Worked walkthroughs live under `fixtures/specs/` next to this brief:

- `extract-shared-validation/` — single-source run with `--scope-include`
  and two `--affects` flags (the RFC-3a canonical case). Pins the full
  extract → merge → Affects-composition path, with both `--affects`
  names matched (silent-when-matched behaviour).
- `affects-orphan-warning/` — sibling case pinning step 4 of the
  Affects composition pass: one matched `--affects`, one orphan
  `--affects` that fires a warning.
- `single-source-no-scope/` — back-compat path: one source, no scope
  flags.
- `two-source-scope/` — two sources, one glob-filtered and one
  manifest-based; demonstrates the `## Source: <key>` design wrapper and
  the name-collision merge rule's trigger conditions.
