# extract-shared-validation — merged output walkthrough

Pins the canonical refactor case as it runs through the specs brief's source-driven branch — from description-driven scope inference through the post-merge delta composition pass.

## Scope inference

The plan entry's description says:

> Lift the shared validation helpers (src/common/validation/) out of user-registration and email-verification into their own slice; delta-target both prior baselines.

The brief infers `--include src/common/validation/**` from the path hint and logs the inference in the journal.

## Per-source loop (one iteration)

```text
/spec:extract ./legacy/monolith <slice-dir>/.extract/monolith/ \
    include 'src/common/validation/**'
```

## After `/spec:extract` returns

```text
<slice-dir>/.extract/monolith/specs/shared-validation/spec.md
<slice-dir>/.extract/monolith/specs/user-registration/spec.md
<slice-dir>/.extract/monolith/specs/email-verification/spec.md
<slice-dir>/.extract/monolith/design.md
```

## After the merge step (single source, no `## Source:` wrapper)

```text
<slice-dir>/specs/shared-validation/spec.md      ← from .extract/monolith/specs/
<slice-dir>/specs/user-registration/spec.md      ← from .extract/monolith/specs/
<slice-dir>/specs/email-verification/spec.md     ← from .extract/monolith/specs/
<slice-dir>/design.md                            ← from .extract/monolith/design.md
```

## After delta composition

The description references "user-registration" and "email-verification" as delta targets. The brief checks `.specify/specs/` for baselines and finds both. Each match is rewritten in delta form; unmatched extracted adapters stay as fresh new-crate specs.

```text
<slice-dir>/specs/user-registration/spec.md
  └─ DELTA against .specify/specs/user-registration/spec.md
     (shared validation rules moved out to shared-validation)

<slice-dir>/specs/email-verification/spec.md
  └─ DELTA against .specify/specs/email-verification/spec.md
     (shared validation rules moved out to shared-validation)

<slice-dir>/specs/shared-validation/spec.md
  └─ NEW-CRATE spec for the extracted validation adapter
     (no baseline; no inferred delta match)
```

### Warnings

Both inferred delta targets (`user-registration`, `email-verification`) matched an extracted adapter, so step 4 of the composition pass fires no warnings. The pinned behaviour here is **silent-when-matched** — a clean run logs nothing. The orphan-warning path is pinned by the sibling `affects-orphan-warning/` fixture.

## `.extract/monolith/` after merge

Retained by default for human review. Operator cleans up manually:

```text
rm -rf <slice-dir>/.extract/
```

once the change is merged. Never committed.
