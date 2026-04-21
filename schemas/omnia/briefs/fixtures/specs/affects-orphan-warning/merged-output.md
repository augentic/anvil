# affects-orphan-warning — merged output walkthrough

Sister fixture to `extract-shared-validation/`. Pins the **step-4
warning path** of the specs brief's Affects composition pass: an
`--affects <name>` flag whose `<name>` does not match any merged
extract capability.

The plan entry declares one extra `--affects` name
(`stale-flag-never-used`) that the scoped extract cannot possibly
touch, because `src/common/validation/**` contains no file that
implements that capability. The other `--affects` name
(`user-registration`) matches normally.

## Per-source loop (one iteration)

```text
key        include                      exclude   manifest
----------------------------------------------------------
monolith   src/common/validation/**     —         —
```

Translated to extract's native flags:

```text
/spec:extract ./legacy/monolith <change-dir>/.extract/monolith/ \
    --include 'src/common/validation/**'
```

## After `/spec:extract` returns

```text
<change-dir>/.extract/monolith/specs/shared-validation/spec.md
<change-dir>/.extract/monolith/specs/user-registration/spec.md
<change-dir>/.extract/monolith/design.md
```

Note: no `stale-flag-never-used/` directory. The scope does not reach
any file that would produce one.

## After the merge step (single source, no `## Source:` wrapper)

```text
<change-dir>/specs/shared-validation/spec.md      ← from .extract/monolith/specs/
<change-dir>/specs/user-registration/spec.md     ← from .extract/monolith/specs/
<change-dir>/design.md                            ← from .extract/monolith/design.md
```

## After Affects composition

Matching each `--affects <name>` against `<change-dir>/specs/`:

```text
<change-dir>/specs/user-registration/spec.md
  └─ DELTA against .specify/specs/user-registration/spec.md
     (matched — step 2)

<change-dir>/specs/shared-validation/spec.md
  └─ NEW-CRATE spec for the extracted validation capability
     (no --affects match — step 3)

<change-dir>/specs/stale-flag-never-used/spec.md
  └─ ABSENT — no matching extract capability (step 4)
```

### Warnings

Step 4 fires exactly one brief-level warning, naming the orphan flag
and surfacing both remediations:

```text
warn: --affects stale-flag-never-used had no matching extract
      capability in <change-dir>/specs/ after the per-source merge.
      Either:
        - the baseline is untouched by this slice — drop the flag via
          `specify initiative amend extract-shared-validation \
              --affects-rm stale-flag-never-used`; or
        - the scope is too narrow to see the file that would change
          the baseline's behaviour — widen `scope.monolith.include`
          (or switch to `scope.monolith.manifest`) via
          `specify initiative amend`.
```

The warning is informational; the brief continues and returns normally.

## `.extract/monolith/` after merge

Retained by default for human review; operator cleans up manually with
`rm -rf <change-dir>/.extract/` once the change is merged. Never
committed.
