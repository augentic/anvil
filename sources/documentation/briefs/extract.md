# `documentation.extract`

For one `Candidate`, walk `$SOURCE_DIR` (read-only) and return a single `Evidence` document of structured claims. The CLI persists the result at `.specify/slices/<slice>/evidence/<source-key>.yaml`; this brief returns the YAML body only.

## Inputs

- `$SOURCE_DIR` — read-only preopen of the bound docs path.
- `<source-key>` — the plan-level binding key under `plan.yaml.sources.<key>`.
- `<candidate-id>` — the candidate from `discovery.md` this run is extracting Evidence for.
- `$SCRATCH_DIR` — per-slice write-only scratch space; use only for unavoidable intermediate state.

## Locate the candidate's docs

The candidate id was produced by `documentation.enumerate` from a top-heading (or a logical section in a monolithic file). Resolve it back:

1. Prefer the file whose top H1 slugs to `<candidate-id>`.
2. Fall back to the file whose kebab-cased stem equals `<candidate-id>`.
3. In a monolithic file, anchor to the section whose H1 (or top-most heading) slugs to `<candidate-id>`.

If no doc resolves, return Evidence with `claims: []` rather than fabricating content. The CLI treats empty `claims:` as valid; an unresolvable candidate becomes a `Status: unknown` requirement during synthesis.

## Claim kinds

Closed for this adapter:

| Kind | Required body field | When to emit |
|---|---|---|
| `requirement` | `statement` | A behavioural claim the docs state about the system (one sentence, present tense). |
| `criterion` | `criterion` | An acceptance criterion the docs list (often under "Acceptance:" or a bullet list under a requirement). |
| `decision` | `decision` | A design or product decision the docs record (often "Decision:" lines or paragraphs). |
| `section` | (free-form) | A bounded prose section worth carrying into synthesis verbatim when no finer-grained claim fits. |

`claim-id` is **required** on `requirement` and `criterion` kinds (deterministic fusion at synthesis time keys off it). `claim-id` is **optional** on `decision` and `section`. Other claim kinds in `schemas/evidence.schema.json` are out of scope for this adapter.

## `path` grammar

Every claim carries a `path` rooted relative to `$SOURCE_DIR`. The grammar matches GitHub-style anchors:

- `<path>` — whole-file claim.
- `<path>#L<n>` — single line.
- `<path>#L<start>-L<end>` — line range.

Line numbers are 1-indexed against the file at extract time. Choose the tightest anchor that bounds the cited text.

## Output

Return one Evidence document matching `schemas/evidence.schema.json`. Field order is fixed (`source`, `adapter`, `authority`, `candidate`, `claims`).

```yaml
source: <source-key>
adapter: documentation
authority: documentation
candidate: <candidate-id>
claims:
  - kind: requirement
    claim-id: <kebab-or-dotted-id>
    path: <relative-path>#L<n>
    statement: "..."
  - kind: criterion
    claim-id: <kebab-or-dotted-id>
    path: <relative-path>#L<n>
    criterion: "..."
  - kind: decision
    path: <relative-path>#L<n>
    decision: "..."
```

`adapter` is always the literal `documentation`. `authority` is always the literal `documentation` (operator-provided written product/technical intent — see the authority hierarchy `intent > documentation > behaviour`). `source` is the supplied `<source-key>`. `candidate` is the supplied `<candidate-id>`.

## Worked example

Input (`password-reset.md` in `$SOURCE_DIR`):

```markdown
# Password reset

The account service should let a registered user request a password reset link by email.

Acceptance:
- Unknown email addresses receive the same outward response as known users.
- Reset links expire after 30 minutes.

Decision: use the existing transactional email provider rather than introducing a new notification service.
```

Output (Evidence for `candidate: password-reset`, bound under `source-key: product-notes`):

```yaml
source: product-notes
adapter: documentation
authority: documentation
candidate: password-reset
claims:
  - kind: requirement
    claim-id: password-reset.request
    path: password-reset.md#L3
    statement: "The account service should let a registered user request a password reset link by email."
  - kind: criterion
    claim-id: password-reset.response-privacy
    path: password-reset.md#L6
    criterion: "Unknown email addresses receive the same outward response as known users."
  - kind: criterion
    claim-id: password-reset.expiry
    path: password-reset.md#L7
    criterion: "Reset links expire after 30 minutes."
  - kind: decision
    path: password-reset.md#L9
    decision: "Use the existing transactional email provider rather than introducing a new notification service."
```

A full input/output fixture for this example lives at [`tests/fixtures/sources/documentation/`](../../../tests/fixtures/sources/documentation/) in the repo.

## Determinism

- Emit claims in source order (top of file to bottom). Stable order keeps synthesis golden runs reproducible.
- Quote statements / criteria / decisions verbatim from the docs where possible. Light grammatical normalisation (capitalisation, terminal punctuation) is allowed; rephrasing is not.
- Do not invent `claim-id`s. Derive them from the candidate id plus a short noun phrase the docs use (`password-reset.expiry`, not `req-007`).

## Guardrails

- `$SOURCE_DIR` is read-only. Reads outside it surface as `source-extract-path-denied`; never attempt to widen the preopen.
- Never write Evidence to disk yourself — return the YAML body to the CLI, which persists it under `.specify/slices/<slice>/evidence/<source-key>.yaml`.
- Never emit closed-enum kinds outside `{requirement, criterion, decision, section}` from this adapter. Spatial kinds (`region`/`container`/`leaf`) belong to `screenshots`; behaviour kinds (`excerpt`/`type`/`call`) belong to code source adapters.
- Never omit `claim-id` on `requirement` or `criterion`. The CLI validates Evidence against `schemas/evidence.schema.json` before synthesis; a missing `claim-id` fails the slice in `refining`.
- Empty `claims: []` is valid output when a candidate cannot be resolved to any doc content. Do not pad with speculative claims.
