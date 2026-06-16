# `documentation.survey`

Walk `$SOURCE_DIR` (a read-only preopen of the operator-bound docs path) and emit one `Lead` per top-level concept the docs describe. The CLI persists the result; this brief returns the lead-block payload only.

## Inputs

- `$SOURCE_DIR` — read-only directory holding the bound documentation set. Never write here.
- `<source>` — the plan-level binding key under `plan.yaml.sources.<key>`; the CLI passes it in for context and stamps each lead's `source` itself, so this brief does not emit it.
- `$SCRATCH_DIR` — per-slice write-only scratch space; use only if intermediate state is unavoidable.

## What is a top-level concept

One discrete, slice-sized behaviour the docs describe. Two recognition rules, in order:

1. **One file, one concept.** When `$SOURCE_DIR` holds multiple markdown files, treat each file's top heading (the first `# ...` H1) as a lead. Files without a top heading fall back to the kebab-cased filename stem.
2. **Monolithic file.** When `$SOURCE_DIR` holds a single markdown file with multiple top-level sections, treat each H1 (or each H2 when the file uses H1 as a title only) as a lead.

Skip files that contain no behavioural content (e.g. tables of contents, license boilerplate, glossaries). When in doubt, emit the lead — `propose` and the operator at Gate 1 reconcile false positives.

## Lead id and synopsis

- `lead`: kebab-case slug derived from the concept's heading. Lowercase, strip punctuation, replace whitespace with `-`. Example: `# Password reset` -> `password-reset`. Re-surveying the same source replaces by `(source, lead)`, so stability matters more than prettiness.
- `synopsis`: a content-bearing description lifted (or lightly compressed) from the concept's opening paragraph — the first non-heading, non-list paragraph after the heading. Name the concept's behaviour and its salient constraint so a same-slug lead from another source can be matched or distinguished on content, not just the shared slug. When a concept is cross-cutting guidance that applies across the other concepts (a conventions or approach document) rather than a discrete behaviour, say so explicitly (e.g. `cross-cutting: applies to all flows in this set`) so propose can recognise it and bind it into every slice it informs. Prefer one line and keep it tight (~200 characters); it MAY run to a few lines when one is too thin. Do not invent content the docs do not state, and never spill slice-time detail here — that is `documentation.extract`'s job.
- `topics` (optional): an inline list of kebab-case slugs naming the concept's domains, drawn from the heading and opening paragraph (e.g. `[identity, password]`). Author them only when the docs clearly support the classification; omit the bullet when unsure. They are extra grouping context for `propose` and the join key for the decision-contradiction warning — never a grouping the CLI computes. Keep slugs stable across re-surveys for the same concept.

## Output

Return one block per lead, in alphabetical `lead` order. The CLI appends them under the existing `## Lead inventory` heading in `discovery.md`; this brief never writes the heading itself.

```markdown
### password-reset

- lead: password-reset
- synopsis: Account service that lets a registered user request a password reset link by email.
- topics: [identity, password]
```

Field order is fixed (`lead`, `synopsis`, then optional `topics`). Do not emit `source`; the CLI stamps it from the survey binding. Cross-source merging is `/spec:plan`'s `propose` sub-step, not this brief's job — see [From sources to slices](../references/spec-runtime/reconciliation.md#plan-time-leads-become-slices) for how leads reconcile into slices.

## Worked example

Bound directory layout (relative to `$SOURCE_DIR`):

```text
account.md          # top heading: "Account"
password-reset.md   # top heading: "Password reset"
```

Expected output (alphabetically by `lead`):

```markdown
### account

- lead: account
- synopsis: Account service that stores per-user identity, credential, and notification preferences.

### password-reset

- lead: password-reset
- synopsis: Account service that lets a registered user request a password reset link by email.
```

A full input/output fixture for this example lives at [`evals/fixtures/sources/documentation/`](../../../../evals/fixtures/sources/documentation/) in the repo.

## Determinism

- Emit leads sorted alphabetically by `lead`.
- Field order inside each block is fixed: `lead`, `synopsis`, then optional `topics`.
- When emitted, `topics` slugs are ordered deterministically (e.g. as they appear, deduplicated) so re-running is byte-identical.
- No timestamps, host paths, or other run-state in the output — re-running against unchanged inputs produces byte-identical blocks.

## Guardrails

- `$SOURCE_DIR` is read-only. Reads outside it surface as `source-survey-path-denied`; never attempt to widen the preopen.
- Do not write or rewrite the `## Lead inventory` heading — the CLI owns the section frame.
- Do not emit Evidence here. Per-claim extraction is `documentation.extract`'s job, run once per lead at slice time.
- Do not invent a lead the docs do not describe. Empty inventories (`$SOURCE_DIR` parseable but no behavioural concepts) are valid output.
