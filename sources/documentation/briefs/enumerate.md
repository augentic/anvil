# `documentation.enumerate`

Walk `$SOURCE_DIR` (a read-only preopen of the operator-bound docs path) and emit one `Candidate` per top-level concept the docs describe. The CLI persists the result; this brief returns the candidate-block payload only.

## Inputs

- `$SOURCE_DIR` — read-only directory holding the bound documentation set. Never write here.
- `<source-key>` — the plan-level binding key under `plan.yaml.sources.<key>`; the CLI passes it in and this brief embeds it in every `sources:` list.
- `$SCRATCH_DIR` — per-slice write-only scratch space; use only if intermediate state is unavoidable.

## What is a top-level concept

One discrete, slice-sized behaviour the docs describe. Two recognition rules, in order:

1. **One file, one concept.** When `$SOURCE_DIR` holds multiple markdown files, treat each file's top heading (the first `# ...` H1) as a candidate. Files without a top heading fall back to the kebab-cased filename stem.
2. **Monolithic file.** When `$SOURCE_DIR` holds a single markdown file with multiple top-level sections, treat each H1 (or each H2 when the file uses H1 as a title only) as a candidate.

Skip files that contain no behavioural content (e.g. tables of contents, license boilerplate, glossaries). When in doubt, emit the candidate — `propose` and the operator at Gate 1 reconcile false positives.

## Candidate id and summary

- `id`: kebab-case slug derived from the concept's heading. Lowercase, strip punctuation, replace whitespace with `-`. Example: `# Password reset` -> `password-reset`. Re-enumerating the same source replaces by `id`, so stability matters more than prettiness.
- `summary`: one line lifted (or lightly compressed) from the concept's opening paragraph — the first non-heading, non-list paragraph after the heading. Keep it under 200 characters. Do not invent content the docs do not state.

## Output

Return one block per candidate, in alphabetical `id` order. The CLI appends them under the existing `## Candidate inventory` heading in `discovery.md`; this brief never writes the heading itself.

```markdown
### password-reset

- id: password-reset
- sources: [<source-key>]
- summary: Account service that lets a registered user request a password reset link by email.
```

Field order is fixed (`id`, `sources`, `summary`). `sources:` always carries exactly the supplied `<source-key>` for this adapter; cross-source merging is `/spec:plan`'s `propose` sub-step, not this brief's job.

## Worked example

Bound directory layout (relative to `$SOURCE_DIR`):

```text
account.md          # top heading: "Account"
password-reset.md   # top heading: "Password reset"
```

Expected output (alphabetically by `id`):

```markdown
### account

- id: account
- sources: [product-notes]
- summary: Account service that stores per-user identity, credential, and notification preferences.

### password-reset

- id: password-reset
- sources: [product-notes]
- summary: Account service that lets a registered user request a password reset link by email.
```

A full input/output fixture for this example lives at [`tests/fixtures/sources/documentation/`](../../../tests/fixtures/sources/documentation/) in the repo.

## Determinism

- Emit candidates sorted alphabetically by `id`.
- Field order inside each block is fixed: `id`, `sources`, `summary`.
- No timestamps, host paths, or other run-state in the output — re-running against unchanged inputs produces byte-identical blocks.

## Guardrails

- `$SOURCE_DIR` is read-only. Reads outside it surface as `source-enumerate-path-denied`; never attempt to widen the preopen.
- Do not write or rewrite the `## Candidate inventory` heading — the CLI owns the section frame.
- Do not emit Evidence here. Per-claim extraction is `documentation.extract`'s job, run once per candidate at slice time.
- Do not invent a candidate the docs do not describe. Empty inventories (`$SOURCE_DIR` parseable but no behavioural concepts) are valid output.
