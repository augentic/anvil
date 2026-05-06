# affects-orphan-warning — merged output walkthrough

Sister fixture to `extract-shared-validation/`. Pins the **step-4 warning path** of the specs brief's delta composition pass: the description references a change name that does not match any merged extract capability.

The plan entry's description references both `user-registration` (which matches a merged capability) and `stale-flag-never-used` (which no extracted capability produces), because the inferred scope (`src/common/validation/**`) contains no file that implements that capability.

## Scope inference

The description contains the path hint `src/common/validation/`. The brief infers `--include src/common/validation/**`.

## Per-source loop (one iteration)

```text
/spec:extract ./legacy/monolith <slice-dir>/.extract/monolith/ \
    --include 'src/common/validation/**'
```

## After `/spec:extract` returns

```text
<slice-dir>/.extract/monolith/specs/shared-validation/spec.md
<slice-dir>/.extract/monolith/specs/user-registration/spec.md
<slice-dir>/.extract/monolith/design.md
```

Note: no `stale-flag-never-used/` directory. The scope does not reach any file that would produce one.

## After the merge step (single source, no `## Source:` wrapper)

```text
<slice-dir>/specs/shared-validation/spec.md      ← from .extract/monolith/specs/
<slice-dir>/specs/user-registration/spec.md     ← from .extract/monolith/specs/
<slice-dir>/design.md                            ← from .extract/monolith/design.md
```

## After delta composition

Matching each inferred delta target against `<slice-dir>/specs/`:

```text
<slice-dir>/specs/user-registration/spec.md
  └─ DELTA against .specify/specs/user-registration/spec.md
     (matched — step 2)

<slice-dir>/specs/shared-validation/spec.md
  └─ NEW-CRATE spec for the extracted validation capability
     (no inferred target match — step 3)

<slice-dir>/specs/stale-flag-never-used/spec.md
  └─ ABSENT — no matching extract capability (step 4)
```

### Warnings

Step 4 fires exactly one brief-level warning, naming the orphan inferred target and suggesting the description may be inaccurate:

```text
warn: inferred delta target stale-flag-never-used had no matching
      extract capability in <slice-dir>/specs/ after the per-source
      merge. The description may reference a change that this slice
      does not actually modify — amend the description via
      `specify change plan amend extract-shared-validation \
          --description "..."`.
```

The warning is informational; the brief continues and returns normally.

## `.extract/monolith/` after merge

Retained by default for human review; operator cleans up manually with `rm -rf <slice-dir>/.extract/` once the change is merged. Never committed.
