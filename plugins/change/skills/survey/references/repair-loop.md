# Bounded repair loop

`/change:survey` enumerates surfaces through an LLM driven by a per-language brief. The LLM output is non-deterministic; the canonical `surfaces.json` schema is not. When the staged candidate fails the CLI's structured validator, the skill re-prompts the LLM with the validator output and tries again — up to a bounded budget. Exhaustion fails the run cleanly with the last failing candidate preserved for the operator.

This file pins the contract: the feedback grammar fed back to the LLM, the retry budget, the exhaustion shape, and the set of exit discriminants that are **never** retried.

## Structured feedback grammar

Each retry hands the LLM three things in a single prompt:

1. The original brief (the same `plugins/change/skills/survey/briefs/enumerate/<language>.md` the first attempt used).
2. The candidate `surfaces.json` the prior attempt produced (verbatim).
3. A small JSON envelope carrying the CLI's `Error::Diag` `code` and `detail`, plus a tight "fix the cited rule only" instruction.

Envelope shape:

```json
{
  "failure": {
    "code": "surfaces-touches-out-of-tree",
    "detail": "surfaces[2].touches[1]: ../escaped/path.ts"
  },
  "instruction": "Fix only the rule cited above. Re-emit the full surfaces.json with the offending entry corrected; do not alter unrelated surfaces."
}
```

`code` and `detail` come from the CLI verbatim — the skill never paraphrases. The instruction is constant prose; the LLM has the candidate, the brief, and the structured complaint and produces a fresh candidate.

## Retry budget

Each `legacy-code` source has its own budget. v1: **3 retries** per source. Each retry is one CLI invocation in `--validate-only` form:

```text
specify change survey \
  --sources <sources.yaml> --staged <staged-dir> --out <out-dir> \
  --validate-only
```

`--validate-only` short-circuits the metadata-and-write step, so the canonical sidecars in `<out>/<source-key>/` stay untouched until a candidate validates cleanly. After a successful retry the skill drops `--validate-only` and re-invokes the verb once to perform the canonical write.

The budget covers only the three repair-eligible discriminants:

- `surfaces-validation-failed`
- `surfaces-id-collision`
- `surfaces-touches-out-of-tree`

Any other non-zero exit halts immediately (see "Fail-closed rule").

## Exhaustion contract

After 3 failed retries on a source, the skill:

1. Exits non-zero with the skill-emitted discriminant `surveyor-exhausted`.
2. Prints the last failing candidate and the last validator output to the operator.
3. Persists both under `.specify/plans/<change>/survey/staged/<source-key>.last-failure.json`:

```json
{
  "source-key": "legacy-monolith",
  "attempts": 3,
  "last-candidate": { "version": 1, "source-key": "legacy-monolith", "...": "..." },
  "last-failure": {
    "code": "surfaces-touches-out-of-tree",
    "detail": "surfaces[2].touches[1]: ../escaped/path.ts"
  }
}
```

The operator hand-edits the candidate (or tightens the brief) and re-runs `/change:survey`. The skill never writes `survey.md` or appends to `discovery.md` when any source ends in `surveyor-exhausted`.

## Fail-closed rule

Exit discriminants that are **not** retried — these are skill or operator bugs, not LLM hallucinations:

- `staged-input-missing`
- `staged-input-malformed`
- `source-path-missing`
- `source-path-not-readable`
- `source-key-mismatch`
- `sources-file-missing`
- `sources-file-malformed`

The skill surfaces the CLI's error message verbatim and exits non-zero. Re-prompting the LLM would not help — the staged file is missing, the operator's path is wrong, or the skill itself wrote a malformed `--sources` file. Fix the upstream cause and re-run.
