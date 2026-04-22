# extract-shared-validation — merged output walkthrough

Pins the RFC-3a canonical case (§*`--affects` composition with scope*) as
it runs through the specs brief's source-driven branch — from the scoped
`/spec:extract` invocation through the post-merge Affects composition
pass.

## Per-source loop (one iteration)

The flag set handed to the brief groups into a single-key bundle:

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
<change-dir>/.extract/monolith/specs/email-verification/spec.md
<change-dir>/.extract/monolith/design.md
```

## After the merge step (single source, no `## Source:` wrapper)

```text
<change-dir>/specs/shared-validation/spec.md      ← from .extract/monolith/specs/
<change-dir>/specs/user-registration/spec.md      ← from .extract/monolith/specs/
<change-dir>/specs/email-verification/spec.md     ← from .extract/monolith/specs/
<change-dir>/design.md                            ← from .extract/monolith/design.md
```

## After Affects composition

The `--affects user-registration --affects email-verification` flags
drive the post-merge composition pass. Each `--affects` name is matched
against the merged `<change-dir>/specs/` tree; matches are rewritten in
delta form against `.specify/specs/<name>/spec.md`, unmatched extracted
capabilities stay as fresh new-crate specs.

```text
<change-dir>/specs/user-registration/spec.md
  └─ DELTA against .specify/specs/user-registration/spec.md
     (shared validation rules moved out to shared-validation)

<change-dir>/specs/email-verification/spec.md
  └─ DELTA against .specify/specs/email-verification/spec.md
     (shared validation rules moved out to shared-validation)

<change-dir>/specs/shared-validation/spec.md
  └─ NEW-CRATE spec for the extracted validation capability
     (no baseline; no --affects match)
```

### Warnings

Both `--affects` names (`user-registration`, `email-verification`)
matched an extracted capability, so step 4 of the composition pass
fires no warnings. The pinned behaviour here is **silent-when-matched**
— a clean run logs nothing. The orphan-warning path is pinned by the
sibling `affects-orphan-warning/` fixture.

## `.extract/monolith/` after merge

Retained by default for human review. Operator cleans up manually:

```text
rm -rf <change-dir>/.extract/
```

once the change is merged. Never committed.
